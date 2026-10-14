import AVFoundation
import CryptoKit
import os
import UIKit

private enum ImageFormat {
    case png
    case jpeg

    var fileExtension: String {
        switch self {
        case .png: return "png"
        case .jpeg: return "jpg"
        }
    }
}

actor DefaultSongImportService: SongImportService {
    private var currentImportTask: Task<Void, Error>?
    private var isImporting = false

    func cancelImport() {
        currentImportTask?.cancel()
        releaseSecurityAccess()
    }

    private let logger = Logger(subsystem: subsystem, category: "SongImporter")
    private let songRepo: SongRepository
    private let sourcePathRepo: SourcePathRepository
    private let sourceRepo: SourceRepository
    private var activeRootURLs: [Int64: URL] = [:]

    init(
        songRepo: SongRepository,
        sourcePathRepo: SourcePathRepository,
        sourceRepo: SourceRepository
    ) {
        self.songRepo = songRepo
        self.sourcePathRepo = sourcePathRepo
        self.sourceRepo = sourceRepo
    }

    private func prepareSecurityAccess(for paths: [SourcePath]) async throws {
        let sourceIds = Set(paths.map { $0.sourceId })

        for sourceId in sourceIds {
            guard let rootURL = try await resolveRootURL(for: sourceId) else {
                throw CustomError.genericError("Failed to access source \(sourceId)")
            }
            activeRootURLs[sourceId] = rootURL
        }
    }

    private func resolveRootURL(for sourceId: Int64) async throws -> URL? {
        // Check cache first
        if let cached = activeRootURLs[sourceId] {
            return cached
        }

        // Fetch source from repository
        guard let source = try await sourceRepo.getOne(id: sourceId) else {
            logger.error("Source not found: \(sourceId)")
            return nil
        }

        // Resolve bookmark
        let bookmarkKey = makeBookmarkKey(URL(filePath: source.dirPath))
        guard let bookmarkData = UserDefaults.standard.data(forKey: bookmarkKey) else {
            logger.error("Missing bookmark for source \(sourceId)")
            return nil
        }

        var isStale = false
        let url = try URL(
            resolvingBookmarkData: bookmarkData,
            options: [],
            relativeTo: nil,
            bookmarkDataIsStale: &isStale
        )

        // Refresh stale bookmark
        if isStale {
            let newBookmark = try url.bookmarkData(options: [])
            UserDefaults.standard.set(newBookmark, forKey: bookmarkKey)
        }

        // Start security access
        guard url.startAccessingSecurityScopedResource() else {
            logger.error("Security scope access failed for \(sourceId)")
            return nil
        }

        // Cache the resolved URL
        activeRootURLs[sourceId] = url
        return url
    }

    private func releaseSecurityAccess() {
        activeRootURLs.forEach { $1.stopAccessingSecurityScopedResource() }
        activeRootURLs.removeAll()
    }

    func importPaths(
        paths: [SourcePath],
        onProgress: ((Double, URL) async -> Void)? = nil
    ) async throws {
        // Cancel any existing task
        if let currentTask = currentImportTask {
            currentTask.cancel()
            try await currentTask.value
        }

        currentImportTask = Task {
            defer {
                currentImportTask = nil
                isImporting = false
                releaseSecurityAccess()
            }

            isImporting = true
            try await importImplementation(paths: paths, onProgress: onProgress)
        }

        try await currentImportTask!.value
    }

    private func checkBookmarkValidity(song: Song) -> Bool {
        guard let bookmarkData = song.bookmark else { return false }

        var isStale = false
        do {
            _ = try URL(
                resolvingBookmarkData: bookmarkData,
                options: .withoutUI,
                relativeTo: nil,
                bookmarkDataIsStale: &isStale
            )
            return !isStale
        } catch {
            return false
        }
    }

    /// Import all paths, recursively grabbing files from directories.
    /// - parameter onProgress: Called with (percentage from 0..100, currentFileURL).
    func importImplementation(
        paths: [SourcePath],
        onProgress: ((Double, URL) async -> Void)? = nil
    ) async throws {
        logger.debug("Gathering files from \(paths.count) paths")
        let filePaths = try await gatherAllFiles(paths: paths)
        logger.debug("Total files: \(filePaths.count)")
        try await prepareSecurityAccess(for: filePaths)
        defer { releaseSecurityAccess() }

        for (index, filePath) in filePaths.enumerated() {
            guard let fileURL = await resolveFileURL(for: filePath) else {
                logger.error("Skipping file due to unresolved URL")
                continue
            }

            // Provide progress callback
            let percent = Double(index + 1) / Double(filePaths.count) * 100
            logger.debug("progress: \(percent)%; url: \(fileURL)")
            await onProgress?(percent, fileURL)

            // 1) Skip if already processed
            if filePath.fileHashSHA256 != nil {
                if let existingSong = await songRepo.getSongByURL(fileURL) {
                    // Verify bookmark validity
                    if checkBookmarkValidity(song: existingSong) {
                        logger.debug("Skipping valid existing file: \(fileURL.lastPathComponent)")
                        continue
                    }
                    logger.debug(
                        "Existing bookmark invalid, reprocessing: \(fileURL.lastPathComponent)")
                }
            }

            // 2) Compute new file hash & store in DB
            let fileData = try Data(contentsOf: fileURL)
            let fileHash = sha256(fileData)
            try await sourcePathRepo.updateFileHash(pathId: filePath.pathId, fileHash: fileHash)

            // 3) Read metadata
            let (artist, title, album, albumArtist, releaseYear, trackNumber, discNumber) =
                await readMetadataAVAsset(url: fileURL)
            let songKey = generateSongKey(artist: artist, title: title, album: album)

            // 4) Extract artwork
            let coverArtData = await extractCoverArtData(url: fileURL)
            let coverArtPath = try storeCoverArtOnDiskDedup(coverArtData)

            // 5) Upsert into DB
            let newSong = try Song(
                id: nil,
                songKey: songKey,
                artist: artist,
                title: title,
                album: album,
                albumArtist: albumArtist,
                releaseYear: releaseYear,
                discNumber: discNumber,
                trackNumber: trackNumber,
                coverArtPath: coverArtPath,
                bookmark: createBookmark(for: fileURL),
                pathHash: makeURLHash(fileURL),
                createdAt: Date(),
                updatedAt: nil,
                localFilePath: nil,
                fileState: .bookmarkOnly,
                isFavorite: 0
            )
            let inserted = try await songRepo.upsertSong(newSong)
            logger.debug("Upserted song: \(inserted.title)")
            try await songRepo.markSongForCopy(songId: inserted.id!)
        }
    }

    // MARK: - Recursively gather all file paths

    private func gatherAllFiles(paths: [SourcePath]) async throws -> [SourcePath] {
        var result = [SourcePath]()
        for p in paths {
            if p.isDirectory {
                let children = try await sourcePathRepo.getByParentId(
                    sourceId: p.sourceId, parentPathId: p.pathId
                )
                try result.append(contentsOf: await gatherAllFiles(paths: children))
            } else {
                result.append(p)
            }
        }
        return result
    }

    // MARK: - Resolve file URL with caching

    // MARK: - File URL Resolution

    private func resolveFileURL(for path: SourcePath) async -> URL? {
        guard let rootURL = activeRootURLs[path.sourceId] else {
            logger.error("No root URL found for source \(path.sourceId)")
            return nil
        }

        return rootURL.appendingPathComponent(path.relativePath, isDirectory: false)
    }

    private func resolveURLFromSource(_ source: Source, relativePath: String) throws -> URL? {
        let folderAbsoluteString = source.dirPath
        let bookmarkKey = makeBookmarkKey(URL(filePath: folderAbsoluteString))

        guard let bookmarkData = UserDefaults.standard.data(forKey: bookmarkKey) else {
            logger.error("No bookmark found for source \(source.id ?? -1)")
            return nil
        }

        var isStale = false
        let resolvedURL = try URL(
            resolvingBookmarkData: bookmarkData,
            options: [],
            relativeTo: nil,
            bookmarkDataIsStale: &isStale
        )

        if isStale {
            // Renew bookmark and update cache if needed
            let newBookmark = try resolvedURL.bookmarkData()
            UserDefaults.standard.set(newBookmark, forKey: bookmarkKey)
        }

        guard resolvedURL.startAccessingSecurityScopedResource() else {
            logger.error("Security scope access failed for \(source.id ?? -1)")
            return nil
        }
        defer { resolvedURL.stopAccessingSecurityScopedResource() }

        return resolvedURL.appendingPathComponent(relativePath)
    }

    // MARK: - Create a bookmark

    private func createBookmark(for fileURL: URL) throws -> Data {
        try fileURL.bookmarkData(
            options: [],
            includingResourceValuesForKeys: nil,
            relativeTo: nil
        )
    }

    // MARK: - Read metadata (iOS 16+)

    @available(iOS 16.0, *)
    func readMetadataAVAsset(url: URL) async -> (
        artist: String, title: String, album: String,
        albumArtist: String, releaseYear: Int?, trackNumber: Int?, discNumber: Int?
    ) {
        let asset = AVURLAsset(url: url)
        var defaultSongTitle = url.lastPathComponent
        if defaultSongTitle.isEmpty {
            defaultSongTitle = "Unknown Title"
        }
        do {
            var artist = ""
            var title = ""
            var album = ""
            var trackNumber: Int? = nil

            var albumArtist = ""
            var releaseYear: Int? = nil
            var discNumber: Int? = nil

            let formats = try await asset.load(.availableMetadataFormats)
            for format in formats {
                let metadata = try await asset.loadMetadata(for: format)
                for item in metadata {
                    // First check common keys
                    if let commonKey = item.commonKey {
                        switch commonKey {
                        case .commonKeyArtist where artist.isEmpty:
                            if let val = try? await item.load(.stringValue) { artist = val }
                        case .commonKeyTitle where title.isEmpty:
                            if let val = try? await item.load(.stringValue) { title = val }
                        case .commonKeyAlbumName where album.isEmpty:
                            if let val = try? await item.load(.stringValue) { album = val }
                        default:
                            break
                        }
                    }
                    // Then check ID3 or format-specific keys
                    if let keyStr = item.key as? String {
                        // Album Artist: check common ID3 keys
                        if (keyStr == "TPE2" || keyStr == "aART") && albumArtist.isEmpty {
                            if let val = try? await item.load(.stringValue) { albumArtist = val }
                        }
                        // Release Year
                        if keyStr == "TDRC" && releaseYear == nil {
                            if let val = try? await item.load(.stringValue),
                               let year = Int(val.prefix(4))
                            {
                                releaseYear = year
                            }
                        }
                        // Disc Number
                        if keyStr == "TPOS" && discNumber == nil {
                            if let val = try? await item.load(.stringValue) {
                                let parts = val.components(separatedBy: "/")
                                if let disc = Int(parts[0]) {
                                    discNumber = disc
                                }
                            }
                        }
                        // Track Number (ID3 TRCK key)
                        if keyStr == "TRCK" && trackNumber == nil {
                            if let val = try? await item.load(.stringValue) {
                                let cleanString = val.components(separatedBy: "/").first ?? val
                                if let number = Int(cleanString) { trackNumber = number }
                            }
                        }
                    }
                }
            }

            // Fallbacks if values are still empty
            let cleanedArtist = artist.isEmpty ? "Unknown Artist" : artist
            let cleanedTitle = title.isEmpty ? defaultSongTitle : title
            let cleanedAlbum = album.isEmpty ? "Unknown Album" : album
            let cleanedAlbumArtist = albumArtist.isEmpty ? cleanedArtist : albumArtist

            return (
                cleanedArtist, cleanedTitle, cleanedAlbum, cleanedAlbumArtist, releaseYear,
                trackNumber, discNumber
            )
        } catch {
            logger.error("Failed to read metadata: \(error)")
            return (
                "Unknown Artist", defaultSongTitle, "Unknown Album", "Unknown Artist", nil, nil, nil
            )
        }
    }

    // MARK: - Extract cover art data (improved)

    @available(iOS 16.0, *)
    private func extractCoverArtData(url: URL) async -> Data? {
        let asset = AVURLAsset(url: url)
        do {
            let metadata = try await asset.load(.metadata)

            // Check multiple possible metadata keys
            for item in metadata {
                guard item.commonKey != nil else { continue }

                // Handle both data values and external URLs
                if let dataValue = try? await item.load(.dataValue) {
                    if isValidImageData(dataValue) {
                        return dataValue
                    }
                }

                // Handle URL references
                if let stringValue = try? await item.load(.stringValue),
                   let url = URL(string: stringValue),
                   url.scheme == "http" || url.scheme == "https"
                {
                    logger.debug("Found remote artwork URL: \(url.absoluteString)")
                    // Consider downloading here if appropriate
                }
            }

            // Fallback: Check all metadata items regardless of key space
            for item in metadata {
                if let dataValue = try? await item.load(.dataValue),
                   isValidImageData(dataValue)
                {
                    return dataValue
                }
            }
        } catch {
            logger.error("extractCoverArtData error: \(error)")
        }
        return nil
    }

    // MARK: - Validate image data

    private func isValidImageData(_ data: Data) -> Bool {
        return UIImage(data: data) != nil
    }

    // MARK: - Store cover art with format detection

    private func storeCoverArtOnDiskDedup(_ data: Data?) throws -> String? {
        guard let data = data, !data.isEmpty else { return nil }

        // Validate image data
        guard isValidImageData(data), let image = UIImage(data: data) else {
            logger.error("Invalid image data, skipping cover art")
            return nil
        }

        // Detect image format
        let format: ImageFormat = image.pngData() != nil ? .png : .jpeg
        let hashString = sha256(data).map { String(format: "%02x", $0) }.joined()
        let filename = "cover-\(hashString).\(format.fileExtension)"

        guard
            let documentsDir = FileManager.default.urls(
                for: .documentDirectory, in: .userDomainMask
            ).first
        else {
            return nil
        }

        let coverArtDir = documentsDir.appendingPathComponent("CoverArt", isDirectory: true)
        if !FileManager.default.fileExists(atPath: coverArtDir.path) {
            try FileManager.default.createDirectory(
                at: coverArtDir, withIntermediateDirectories: true
            )
        }

        let fileURL = coverArtDir.appendingPathComponent(filename)

        if !FileManager.default.fileExists(atPath: fileURL.path) {
            switch format {
            case .png:
                try image.pngData()?.write(to: fileURL)
            case .jpeg:
                try image.jpegData(compressionQuality: 0.8)?.write(to: fileURL)
            }
        }

        return "CoverArt/\(filename)"
    }

    // MARK: - Utility: SHA256 for Data

    private func sha256(_ data: Data) -> Data {
        // If you're on iOS 13+, you can use CryptoKit. For brevity:
        var hasher = CryptoKit.SHA256()
        hasher.update(data: data)
        return Data(hasher.finalize())
    }
}
