import AVFoundation
import CryptoKit
import os
import LocalWaveDomain
import LocalWaveCore
#if canImport(UIKit)
import UIKit
#endif

private enum ImageFormat {
    case png
    case jpeg

    public var fileExtension: String {
        switch self {
        case .png: return "png"
        case .jpeg: return "jpg"
        }
    }
}

public actor DefaultSongImportService: SongImportService {
    private var currentImportTask: Task<Void, Error>?
    private var isImporting = false

    public func cancelImport() {
        currentImportTask?.cancel()
        // The task releases its scope after it has stopped reading files.
    }

    private let logger = createLogger(subsystem: subsystem, category: "SongImporter")
    private let songRepo: SongRepository
    private let sourcePathRepo: SourcePathRepository
    private let sourceRepo: SourceRepository
    private let sourceAccess: SourceAccess
    private let availabilityInspector: @Sendable (URL) throws -> FileAvailability
    private var activeRootURLs: [Int64: URL] = [:]
    private var scopedRootIds = Set<Int64>()

    public init(
        songRepo: SongRepository,
        sourcePathRepo: SourcePathRepository,
        sourceRepo: SourceRepository,
        sourceAccess: SourceAccess? = nil,
        availabilityInspector: @escaping @Sendable (URL) throws -> FileAvailability = { try FileAvailabilityService.inspect($0) }
    ) {
        self.songRepo = songRepo
        self.sourcePathRepo = sourcePathRepo
        self.sourceRepo = sourceRepo
        self.sourceAccess = sourceAccess ?? SourceAccess(repository: sourceRepo)
        self.availabilityInspector = availabilityInspector
    }

    private func prepareSecurityAccess(for paths: [SourcePath]) async throws {
        let sourceIds = Set(paths.map { $0.sourceId })
        do {
        for sourceId in sourceIds {
            guard let rootURL = try await resolveRootURL(for: sourceId) else {
                throw CustomError.genericError("Failed to access source \(sourceId)")
            }
            let scoped = rootURL.startAccessingSecurityScopedResource()
            guard scoped || FileManager.default.isReadableFile(atPath: rootURL.path) else {
                throw CustomError.genericError("Folder access has expired. Reconnect its source folder.")
            }
            if scoped { scopedRootIds.insert(sourceId) }
            activeRootURLs[sourceId] = rootURL
        }
        } catch { releaseSecurityAccess(); throw error }
    }

    private func resolveRootURL(for sourceId: Int64) async throws -> URL? {
        try await sourceAccess.rootURL(sourceId: sourceId)
    }

    private func releaseSecurityAccess() {
        for id in scopedRootIds { activeRootURLs[id]?.stopAccessingSecurityScopedResource() }
        scopedRootIds.removeAll()
        activeRootURLs.removeAll()
    }

    public func importPaths(
        paths: [SourcePath],
        onProgress: ((Double, URL) async -> Void)? = nil
    ) async throws {
        try await startImport(paths: paths, onProgress: onProgress, onFileProgress: nil)
    }

    public func importPathsWithFileProgress(paths: [SourcePath], onProgress: ((SongImportProgress) async -> Void)?) async throws {
        try await startImport(paths: paths, onProgress: nil, onFileProgress: onProgress)
    }

    /// True while an import (user scan or enrichment) runs; enrichment yields to user imports.
    public var isBusy: Bool { isImporting }

    /// Downloads cloud-only files in a prefetch window, reads their tags, then evicts what it downloaded.
    public func enrichCloudPaths(paths: [SourcePath], onProgress: ((SongImportProgress) async -> Void)?) async throws {
        // Never pre-empt a user import: refuse atomically (actor-isolated) instead of cancelling it.
        guard !isImporting else { throw CancellationError() }
        try await startImport(paths: paths, onProgress: nil, onFileProgress: onProgress, materializeCloud: true)
    }

    private func startImport(paths: [SourcePath], onProgress: ((Double, URL) async -> Void)?, onFileProgress: ((SongImportProgress) async -> Void)?, materializeCloud: Bool = false) async throws {
        // A newer import pre-empts the running one; the cancelled task's CancellationError is expected here.
        if let currentTask = currentImportTask {
            currentTask.cancel()
            _ = try? await currentTask.value
        }

        currentImportTask = Task {
            defer {
                currentImportTask = nil
                isImporting = false
                releaseSecurityAccess()
            }

            isImporting = true
            try await importImplementation(paths: paths, onProgress: onProgress, onFileProgress: onFileProgress, materializeCloud: materializeCloud)
        }

        try await currentImportTask!.value
    }

    private func checkBookmarkValidity(song: Song) -> Bool {
        guard let bookmarkData = song.bookmark else { return false }

        var isStale = false
        do {
            _ = try URL(
                resolvingBookmarkData: bookmarkData,
                options: sourceBookmarkResolutionOptions.union(.withoutUI),
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
    public func importImplementation(
        paths: [SourcePath],
        onProgress: ((Double, URL) async -> Void)? = nil,
        onFileProgress: ((SongImportProgress) async -> Void)? = nil,
        materializeCloud: Bool = false
    ) async throws {
        logger.debug("Gathering files from \(paths.count) paths")
        let filePaths = try await gatherAllFiles(paths: paths)
        logger.debug("Total files: \(filePaths.count)")
        try await prepareSecurityAccess(for: filePaths)
        defer { releaseSecurityAccess() }

        var completedFiles = 0
        func reportCompleted(_ url: URL) async {
            completedFiles += 1
            await onFileProgress?(SongImportProgress(percentage: Double(completedFiles) / Double(filePaths.count) * 100, completedFiles: completedFiles, totalFiles: filePaths.count, url: url))
        }

        var skipped = 0
        defer {
            if skipped > 0 {
                NotificationCenter.default.post(name: Notification.Name("LWShowError"), object: "\(skipped) file\(skipped == 1 ? "" : "s") could not be opened and were skipped.")
            }
        }
        // Enrichment: iCloud has no range reads, so start downloads a few files ahead (parallel transfer, serial parse)
        // and remember which ones were cloud-only so only those are evicted afterwards.
        var prefetched = -1
        var evictable = Set<URL>()
        for (index, filePath) in filePaths.enumerated() {
            try Task.checkCancellation()
            while materializeCloud, prefetched < min(index + 8, filePaths.count - 1) {
                prefetched += 1
                guard let ahead = await resolveFileURL(for: filePaths[prefetched]) else { continue }
                if (try? availabilityInspector(ahead)) == .inCloud {
                    evictable.insert(ahead)
                    // A real read is what makes fileproviderd fetch the whole file (startDownloadingUbiquitousItem was a no-op here); parallel via GCD.
                    DispatchQueue.global(qos: .utility).async {
                        if let handle = try? FileHandle(forReadingFrom: ahead) { _ = try? handle.read(upToCount: 4096); try? handle.close() }
                    }
                }
            }
            guard let fileURL = await resolveFileURL(for: filePath) else {
                logger.error("Skipping file due to unresolved URL")
                skipped += 1
                continue
            }

            // Provide progress callback
            let percent = Double(index + 1) / Double(filePaths.count) * 100
            logger.debug("progress: \(percent)%; url: \(fileURL)")
            await onFileProgress?(SongImportProgress(percentage: Double(completedFiles) / Double(filePaths.count) * 100, completedFiles: completedFiles, totalFiles: filePaths.count, url: fileURL))
            await onProgress?(percent, fileURL)

            // 1) Skip if already processed
            var existingSong = await songRepo.getSong(sourceId: filePath.sourceId, relativePath: filePath.relativePath)
            if existingSong == nil { existingSong = await songRepo.getSongByURL(fileURL) }
            let availability = (try? availabilityInspector(fileURL)) ?? .unknown
            if let existingSong, let id = existingSong.id, existingSong.sourceAvailability != availability {
                try await songRepo.updateSourceAvailability(songId: id, availability: availability)
            }
            let deferred = !materializeCloud && (availability == .inCloud || (availability == .unknown && FileAvailabilityService.isProviderLocation(fileURL)))
            var metadataURL = fileURL
            if deferred, let existingSong, let id = existingSong.id, let cache = verifiedCacheURL(existingSong) {
                var version = try? BackgroundFileService.cachedSourceVersionMatches(songID: id, sourceURL: fileURL, cacheURL: cache)
                if version == nil, existingSong.duration != nil {
                    let values = try? fileURL.resourceValues(forKeys: [.contentModificationDateKey, .fileSizeKey])
                    if let modified = values?.contentModificationDate, modified > (existingSong.updatedAt ?? existingSong.createdAt) { version = false }
                    if let size = values?.fileSize, let previous = existingSong.fileSize, Int64(size) != previous { version = false }
                }
                if version == false {
                    let cleared = try await songRepo.updateCachedFile(songId: id, localFilePath: nil, state: .bookmarkOnly, expectedState: existingSong.fileState, expectedPath: existingSong.localFilePath)
                    if cleared, LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs")?.contains(String(id)) == true { try await songRepo.markSongForCopy(songId: id) }
                } else { metadataURL = cache }
            }
            if deferred && metadataURL == fileURL {
                if let existingSong, let id = existingSong.id {
                    if existingSong.sourceId == nil { try await songRepo.updateSourceLocation(songId: id, sourceId: filePath.sourceId, relativePath: filePath.relativePath) }
                } else {
                    let size = try fileURL.resourceValues(forKeys: [.fileSizeKey]).fileSize.map(Int64.init)
                    let guess = Self.provisionalMetadata(relativePath: filePath.relativePath)
                    let placeholder = Song(id: nil, songKey: hashStringToInt64("source:\(filePath.sourceId)/\(filePath.relativePath)"),
                        artist: guess.artist ?? "Unknown Artist", title: Self.fallbackTitle(for: fileURL.lastPathComponent), album: guess.album ?? "Unknown Album", albumArtist: guess.artist ?? "Unknown Artist",
                        releaseYear: guess.year, discNumber: nil, trackNumber: guess.track, coverArtPath: nil, bookmark: nil,
                        pathHash: makeURLHash(fileURL), createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly,
                        sourceId: filePath.sourceId, relativePath: filePath.relativePath, fileSize: size,
                        fileExtension: fileURL.pathExtension.lowercased(), sourceAvailability: availability)
                    _ = try await songRepo.upsertSong(placeholder)
                }
                await reportCompleted(fileURL)
                continue
            }
            if filePath.fileHashSHA256 != nil {
                if let existingSong {
                    // Verify bookmark validity
                    let modified = try fileURL.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate
                    let cachedDigest = metadataURL.deletingPathExtension().lastPathComponent
                    let previousDigest = filePath.fileHashSHA256?.map { String(format: "%02x", $0) }.joined()
                    let unchangedCache = metadataURL != fileURL && cachedDigest == previousDigest
                    if (existingSong.sourceId == filePath.sourceId && existingSong.relativePath == filePath.relativePath || checkBookmarkValidity(song: existingSong)), let modified,
                       existingSong.duration != nil, existingSong.fileSize != nil, existingSong.fileExtension != nil,
                       (metadataURL == fileURL && modified <= (existingSong.updatedAt ?? existingSong.createdAt) || unchangedCache) {
                        if existingSong.sourceId == nil, let id = existingSong.id {
                            try await songRepo.updateSourceLocation(songId: id, sourceId: filePath.sourceId, relativePath: filePath.relativePath)
                        }
                        logger.debug("Skipping valid existing file: \(fileURL.lastPathComponent)")
                        await reportCompleted(fileURL)
                        continue
                    }
                    logger.debug(
                        "Existing bookmark invalid, reprocessing: \(fileURL.lastPathComponent)")
                }
            }

            // 2) Compute new file hash & store in DB
            let fileHash = try hashFile(metadataURL)
            let cacheUnchanged = filePath.fileHashSHA256 == fileHash
            try await sourcePathRepo.updateFileHash(pathId: filePath.pathId, fileHash: fileHash)

            // 3) Read metadata
            let (artist, title, album, albumArtist, releaseYear, trackNumber, discNumber) =
                await readMetadataAVAsset(url: metadataURL, fallbackTitle: Self.fallbackTitle(for: filePath.name))
            let songKey = generateSongKey(artist: artist, title: title, album: album)

            // 4) Extract artwork
            let coverArtData = await extractCoverArtData(url: metadataURL)
            let coverArtPath = try storeCoverArtOnDiskDedup(coverArtData)
            let measuredTime = try? await AVURLAsset(url: metadataURL).load(.duration)
            let measuredDuration = measuredTime?.seconds
            let duration = measuredDuration.flatMap { $0.isFinite && $0 >= 0 ? $0 : nil }
            let logicalSize = (try? fileURL.resourceValues(forKeys: [.fileSizeKey]))?.fileSize
            let materializedSize = (try? metadataURL.resourceValues(forKeys: [.fileSizeKey]))?.fileSize
            let fileSize = (logicalSize ?? materializedSize).map(Int64.init)
            let audioFormat = try? AVAudioFile(forReading: metadataURL).fileFormat
            let rawBitDepth = audioFormat?.streamDescription.pointee.mBitsPerChannel ?? 0

            // 5) Upsert into DB
            let latest = await songRepo.getSong(sourceId: filePath.sourceId, relativePath: filePath.relativePath) ?? existingSong
            let digest = fileHash.map { String(format: "%02x", $0) }.joined()
            let verifiedDigest = latest.flatMap(verifiedCacheURL)?.deletingPathExtension().lastPathComponent
            let changedCacheWhileReading = latest?.localFilePath != existingSong?.localFilePath
            let cachedPath = cacheUnchanged || verifiedDigest == digest || changedCacheWhileReading ? latest?.localFilePath : nil
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
                bookmark: nil,
                pathHash: makeURLHash(fileURL),
                createdAt: Date(),
                updatedAt: nil,
                localFilePath: cachedPath,
                fileState: cachedPath == nil ? .bookmarkOnly : (latest?.fileState ?? .bookmarkOnly),
                isFavorite: 0,
                sourceId: filePath.sourceId,
                relativePath: filePath.relativePath,
                duration: duration,
                fileSize: fileSize,
                fileExtension: fileURL.pathExtension.lowercased(),
                sampleRate: audioFormat?.sampleRate,
                bitDepth: rawBitDepth > 0 ? Int(rawBitDepth) : nil,
                sourceAvailability: availability
            )
            try Task.checkCancellation()
            let inserted = try await songRepo.upsertSong(newSong)
            logger.debug("Upserted song: \(inserted.title)")
            let pinned = inserted.id.map { LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs")?.contains(String($0)) == true } ?? false
            if pinned, let id = inserted.id {
                try await songRepo.markSongForCopy(songId: id)
            } else if evictable.contains(fileURL), cachedPath == nil, let id = inserted.id {
                // Tags are saved; give the disk space back and record that the file is cloud-only again.
                if (try? FileManager.default.evictUbiquitousItem(at: fileURL)) != nil {
                    try await songRepo.updateSourceAvailability(songId: id, availability: .inCloud)
                } else { logger.error("Could not evict \(fileURL.lastPathComponent) after reading tags") }
            }
            await reportCompleted(fileURL)
        }
    }

    // MARK: - Recursively gather all file paths

    private func verifiedCacheURL(_ song: Song) -> URL? {
        guard let path = song.localFilePath else { return nil }
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let url = path.hasPrefix("/") ? URL(fileURLWithPath: path) : documents.appendingPathComponent(path)
        return (try? FileAvailabilityService.inspect(url, knownLocalCopy: true)) == .onDevice ? url : nil
    }

    private func hashFile(_ url: URL) throws -> Data {
        var result: Result<Data, Error>?
        var coordinationError: NSError?
        NSFileCoordinator().coordinate(readingItemAt: url, options: [], error: &coordinationError) { readable in
            result = Result {
                let handle = try FileHandle(forReadingFrom: readable)
                defer { try? handle.close() }
                var hash = SHA256()
                while let chunk = try handle.read(upToCount: 256 * 1024), !chunk.isEmpty {
                    try Task.checkCancellation()
                    hash.update(data: chunk)
                }
                return Data(hash.finalize())
            }
        }
        if let coordinationError { throw coordinationError }
        guard let result else { throw CustomError.genericError("The file provider did not supply the audio file.") }
        return try result.get()
    }

    private func gatherAllFiles(paths: [SourcePath]) async throws -> [SourcePath] {
        var result = [SourcePath]()
        for p in paths {
            try Task.checkCancellation()
            if p.isDirectory {
                let children = try await sourcePathRepo.getByParentId(
                    sourceId: p.sourceId, parentPathId: p.pathId
                )
                try result.append(contentsOf: await gatherAllFiles(paths: children))
            } else if p.isAudio {
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

        return try? SourceAccess.fileURL(root: rootURL, relativePath: path.relativePath)
    }

    // MARK: - Read metadata (iOS 16+)

    @available(iOS 16.0, *)
    public func readMetadataAVAsset(url: URL, fallbackTitle: String? = nil) async -> (
        artist: String, title: String, album: String,
        albumArtist: String, releaseYear: Int?, trackNumber: Int?, discNumber: Int?
    ) {
        if url.pathExtension.lowercased() == "flac",
           let metadata = try? FLACMetadata.read(url: url) {
            let tags = metadata.comments
            let artist = tags["ARTIST"] ?? "Unknown Artist"
            func number(_ key: String) -> Int? {
                tags[key].flatMap { Int($0.split(separator: "/").first.map(String.init) ?? "") }
            }
            return (artist, tags["TITLE"] ?? fallbackTitle.map(Self.fallbackTitle(for:)) ?? Self.fallbackTitle(for: url.lastPathComponent),
                    tags["ALBUM"] ?? "Unknown Album",
                    tags["ALBUMARTIST"] ?? tags["ALBUM ARTIST"] ?? artist,
                    tags["DATE"].flatMap { Int($0.prefix(4)) } ?? number("YEAR"),
                    number("TRACKNUMBER"), number("DISCNUMBER"))
        }
        let asset = AVURLAsset(url: url)
        var defaultSongTitle = Self.fallbackTitle(for: fallbackTitle ?? url.lastPathComponent)
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
                    // M4A atom keys are NSNumber four-character codes, not
                    // Strings. AVFoundation's identifiers work across key types.
                    if let identifier = item.identifier {
                        switch identifier {
                        case .iTunesMetadataAlbumArtist where albumArtist.isEmpty:
                            if let value = try? await item.load(.stringValue) { albumArtist = value }
                        case .iTunesMetadataReleaseDate where releaseYear == nil:
                            if let value = try? await item.load(.stringValue) { releaseYear = Int(value.prefix(4)) }
                        case .iTunesMetadataTrackNumber where trackNumber == nil:
                            if let data = try? await item.load(.dataValue) { trackNumber = Self.iTunesIndex(from: data) }
                        case .iTunesMetadataDiscNumber where discNumber == nil:
                            if let data = try? await item.load(.dataValue) { discNumber = Self.iTunesIndex(from: data) }
                        default: break
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

    /// Best-effort tags from "Artist - Album (Year)/NN - Title.ext" paths so cloud-only songs look right before real tags are read.
    /// Only strict matches are used; real tags overwrite these.
    nonisolated static func provisionalMetadata(relativePath: String) -> (artist: String?, album: String?, year: Int?, track: Int?) {
        let parts = relativePath.split(separator: "/").map(String.init)
        func groups(_ pattern: String, in text: String) -> [String?]? {
            guard let regex = try? NSRegularExpression(pattern: pattern),
                  let match = regex.firstMatch(in: text, range: NSRange(text.startIndex..., in: text)) else { return nil }
            return (1..<match.numberOfRanges).map { Range(match.range(at: $0), in: text).map { String(text[$0]) } }
        }
        var artist: String?, album: String?, year: Int?, track: Int?
        if parts.count >= 2, let g = groups(#"^(.+?)\s+[-–—]\s+(.+?)(?:\s*\((\d{4})\))?$"#, in: parts[parts.count - 2]) {
            artist = g[0]; album = g[1]; year = g[2].flatMap { Int($0) }
        }
        if let last = parts.last, let g = groups(#"^\s*(\d{1,3})\s*[-–—._)]"#, in: last) { track = g[0].flatMap { Int($0) } }
        return (artist, album, year, track)
    }

    nonisolated static func fallbackTitle(for fileName: String) -> String {
        let stem = URL(fileURLWithPath: fileName).deletingPathExtension().lastPathComponent
        let title = stem.replacingOccurrences(of: #"^\s*\d{1,3}\s*[-–—._)]\s*"#, with: "", options: .regularExpression)
        return title.isEmpty ? stem : title
    }

    /// iTunes trkn/disk payloads begin with two reserved bytes, then the
    /// unsigned big-endian16-bit index. Zero denotes an unspecified index.
    nonisolated static func iTunesIndex(from data: Data) -> Int? {
        let bytes = Array(data.prefix(4))
        guard bytes.count == 4 else { return nil }
        let index = (Int(bytes[2]) << 8) | Int(bytes[3])
        return index > 0 ? index : nil
    }

    // MARK: - Extract cover art data (improved)

    @available(iOS 16.0, *)
    private func extractCoverArtData(url: URL) async -> Data? {
        if url.pathExtension.lowercased() == "flac",
           let metadata = try? FLACMetadata.read(url: url), let artwork = metadata.artwork {
            return artwork
        }
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
        #if canImport(UIKit)
        return UIImage(data: data) != nil
        #else
        // Simplified validation for non-iOS platforms
        // Check if data starts with common image format signatures
        guard data.count >= 4 else { return false }
        let prefix = data.prefix(4)
        // PNG signature
        if prefix.starts(with: [0x89, 0x50, 0x4E, 0x47]) { return true }
        // JPEG signature
        if prefix.starts(with: [0xFF, 0xD8, 0xFF]) { return true }
        return false
        #endif
    }

    // MARK: - Store cover art with format detection

    private func storeCoverArtOnDiskDedup(_ data: Data?) throws -> String? {
        guard let data = data, !data.isEmpty else { return nil }

        // Validate image data
        guard isValidImageData(data) else {
            logger.error("Invalid image data, skipping cover art")
            return nil
        }

        #if canImport(UIKit)
        guard let image = UIImage(data: data) else {
            logger.error("Failed to create UIImage from data")
            return nil
        }
        // Detect image format using UIKit
        let format: ImageFormat = image.pngData() != nil ? .png : .jpeg
        #else
        // Detect image format from magic bytes on non-iOS platforms
        let format: ImageFormat = data.starts(with: [0x89, 0x50, 0x4E, 0x47]) ? .png : .jpeg
        #endif

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
            #if canImport(UIKit)
            // iOS: Use UIImage to potentially re-encode the image
            switch format {
            case .png:
                try image.pngData()?.write(to: fileURL)
            case .jpeg:
                try image.jpegData(compressionQuality: 0.8)?.write(to: fileURL)
            }
            #else
            // Non-iOS: Write raw data directly (already validated)
            try data.write(to: fileURL)
            #endif
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
