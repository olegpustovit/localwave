import AVFoundation
import CryptoKit
import os
import LocalWaveDomain
import LocalWaveCore

public actor BackgroundFileService {
    private let songRepo: SongRepository
    private let logger = createLogger(subsystem: subsystem, category: "BackgroundFileService")
    private var isRunning = false
    private var processing = false
    private let documentsURL: URL
    private let preferences: UserDefaults
    private let sourceAccess: SourceAccess?

    public init(songRepo: SongRepository, documentsURL: URL? = nil, preferences: UserDefaults = LocalWavePreferences.current, sourceAccess: SourceAccess? = nil) {
        self.songRepo = songRepo
        self.documentsURL = documentsURL ?? FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        self.preferences = preferences
        self.sourceAccess = sourceAccess
        logger.debug("Initialised")
    }

    public func start() {
        logger.debug("attempting to start myself!")
        guard !isRunning else {
            logger.debug("service already running - aborting restart")
            return
        }
        isRunning = true
        logger.debug("service started successfully")

        Task {
            while isRunning {
                logger.debug("beginning processing cycle")
                await processQueue()
                logger.debug("processing cycle completed")
                do { try await Task.sleep(nanoseconds: 30 * 1_000_000_000) }
                catch { isRunning = false }
            }
        }
        Task {
            for await _ in NotificationCenter.default.notifications(named: Notification.Name("SongListRefresh")) {
                await processQueue()
            }
        }
    }

    private func processQueue() async {
        guard !processing else { return }
        processing = true
        defer { processing = false }
        let songs = await songRepo.getSongsNeedingCopy()
        logger.debug("Processing \(songs.count) files needing copy")

        for song in songs {
            if song.fileState == .failed,
               !Set(preferences.stringArray(forKey: "lwPinnedSongs") ?? []).contains(String(song.id ?? -1)) { continue }
            do {
                logger.debug("attempting cop-y for song id \(song.id ?? -1)")
                let path = try await attemptFileCopy(song: song)
                var stored = false
                if let id = song.id {
                    stored = try await songRepo.updateCachedFile(songId: id, localFilePath: path, state: .copied, expectedState: song.fileState, expectedPath: song.localFilePath)
                }
                NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
                if stored, let sourceID = song.sourceId {
                    NotificationCenter.default.post(name: Notification.Name("LWSourceMaterialized"), object: sourceID)
                }
                logger.debug("succesfully copied song id \(song.id ?? -1)")
            } catch {
                logger.error("Failed to copy file for song \(song.id ?? -1): \(error)")
                // Pinned failed songs retry every cycle; alert only on the transition to failed.
                if song.fileState != .failed {
                    NotificationCenter.default.post(name: Notification.Name("LWShowError"), object: "Couldn’t keep “\(song.title)” offline: \(error.localizedDescription)")
                }
                await markFailed(song: song)
            }
        }
        do { try await enforceCacheLimit() }
        catch { logger.error("Unable to trim offline cache: \(error)") }
    }

    private func attemptFileCopy(song: Song) async throws -> String {
        let sourceURL: URL
        let scopeURL: URL
        var scoped = false
        if let sourceAccess, let sourceId = song.sourceId, let relativePath = song.relativePath {
            let access = try await sourceAccess.fileURL(sourceId: sourceId, relativePath: relativePath)
            sourceURL = access.file
            scopeURL = access.root
            scoped = scopeURL.startAccessingSecurityScopedResource()
        } else {
            guard let bookmark = song.bookmark else { throw CustomError.genericError("No source folder or bookmark is available") }
            var stale = false
            sourceURL = try URL(resolvingBookmarkData: bookmark, options: sourceBookmarkResolutionOptions, bookmarkDataIsStale: &stale)
            scopeURL = sourceURL
            scoped = sourceURL.startAccessingSecurityScopedResource()
            guard scoped || FileManager.default.isReadableFile(atPath: sourceURL.path) else {
                throw CustomError.genericError("Couldn't access the audio file. Reconnect its source folder.")
            }
        }
        defer { if scoped { scopeURL.stopAccessingSecurityScopedResource() } }
        let finalFile = try Self.copyCoordinated(sourceURL, into: documentsURL.appendingPathComponent("Music", isDirectory: true), songID: song.id, preferences: preferences)
        return "Music/\(finalFile.lastPathComponent)"
    }

    public static func copyCoordinated(_ sourceURL: URL, into musicDir: URL, songID: Int64? = nil, preferences: UserDefaults = LocalWavePreferences.current) throws -> URL {
        try FileAvailabilityService.requireLocalForNetworkSimulation(sourceURL, preferences: preferences)
        var coordinationError: NSError?
        var result: Result<URL, Error>?
        NSFileCoordinator().coordinate(readingItemAt: sourceURL, options: [], error: &coordinationError) { readableURL in
            result = Result { try copyStream(readableURL, into: musicDir, sourceSongID: songID, versionSourceURL: sourceURL, preferences: preferences) }
        }
        if let coordinationError { throw coordinationError }
        guard let result else { throw CustomError.genericError("The file provider did not supply the audio file.") }
        return try result.get()
    }

    public static func copyStream(_ sourceURL: URL, into musicDir: URL, protectingSongID: Int64? = nil, sourceSongID: Int64? = nil, versionSourceURL: URL? = nil, preferences: UserDefaults = LocalWavePreferences.current) throws -> URL {
        let networkSource = versionSourceURL ?? sourceURL
        try FileAvailabilityService.requireLocalForNetworkSimulation(networkSource, preferences: preferences)
        var checkedSimulation = preferences.bool(forKey: "lwSimulateNoNetwork")
        let manager = FileManager.default
        try manager.createDirectory(at: musicDir, withIntermediateDirectories: true)
        let temporary = musicDir.appendingPathComponent(UUID().uuidString + ".partial")
        guard manager.createFile(atPath: temporary.path, contents: nil) else {
            throw CustomError.genericError("Unable to create the offline audio file.")
        }
        defer { try? manager.removeItem(at: temporary) }
        let input = try FileHandle(forReadingFrom: sourceURL)
        defer { try? input.close() }
        let output = try FileHandle(forWritingTo: temporary)
        defer { try? output.close() }
        var hash = SHA256()
        while let chunk = try input.read(upToCount: 256 * 1024), !chunk.isEmpty {
            try Task.checkCancellation()
            if !checkedSimulation && preferences.bool(forKey: "lwSimulateNoNetwork") {
                try FileAvailabilityService.requireLocalForNetworkSimulation(networkSource, preferences: preferences)
                checkedSimulation = true
            }
            hash.update(data: chunk)
            try output.write(contentsOf: chunk)
        }
        try output.synchronize()
        try output.close()
        let digest = hash.finalize().map { String(format: "%02x", $0) }.joined()
        let filename = sourceURL.pathExtension.isEmpty ? digest : digest + "." + sourceURL.pathExtension.lowercased()
        let finalFile = musicDir.appendingPathComponent(filename)
        let version = try? (versionSourceURL ?? sourceURL).resourceValues(forKeys: [.contentModificationDateKey, .fileSizeKey])
        return try withOfflineCacheLock {
            try Task.checkCancellation()
            if let id = protectingSongID, LocalWavePreferences.current.string(forKey: "lwActiveSongID") == String(id) {
                LocalWavePreferences.current.set(finalFile.resolvingSymlinksInPath().path, forKey: "lwActiveCachePath")
            }
            if !manager.fileExists(atPath: finalFile.path) {
                do { try manager.moveItem(at: temporary, to: finalFile) }
                catch {
                    guard manager.fileExists(atPath: finalFile.path), manager.contentsEqual(atPath: temporary.path, andPath: finalFile.path) else { throw error }
                }
            }
            if let id = sourceSongID ?? protectingSongID {
                var versions = preferences.dictionary(forKey: "lwCacheSourceVersions") as? [String: [String: Any]] ?? [:]
                var record: [String: Any] = ["cachePath": finalFile.resolvingSymlinksInPath().path]
                if let date = version?.contentModificationDate { record["modified"] = date.timeIntervalSince1970 }
                if let size = version?.fileSize { record["size"] = size }
                versions[String(id)] = record
                preferences.set(versions, forKey: "lwCacheSourceVersions")
            }
            return finalFile
        }
    }

    public static func cachedSourceVersionMatches(songID: Int64, sourceURL: URL, cacheURL: URL, preferences: UserDefaults = LocalWavePreferences.current) throws -> Bool? {
        let record = withOfflineCacheLock {
            (preferences.dictionary(forKey: "lwCacheSourceVersions") as? [String: [String: Any]])?[String(songID)]
        }
        guard let record, record["cachePath"] as? String == cacheURL.resolvingSymlinksInPath().path else { return nil }
        let current = try sourceURL.resourceValues(forKeys: [.contentModificationDateKey, .fileSizeKey])
        return record["modified"] as? Double == current.contentModificationDate?.timeIntervalSince1970 && record["size"] as? Int == current.fileSize
    }

    public func enforceCacheLimit() async throws {
        let stored = preferences.integer(forKey: "lwCacheGB")
        let gigabytes = [2, 4, 8, 16, 32].contains(stored) ? stored : 8
        try await enforceCacheLimit(maxBytes: Int64(gigabytes) * 1_073_741_824)
    }

    func enforceCacheLimit(maxBytes: Int64) async throws {
        let files = try Self.cacheFiles(in: documentsURL)
        var total = files.reduce(Int64(0)) { $0 + $1.bytes }
        guard total > maxBytes else { return }
        var songs: [Song] = []
        var offset = 0
        while true {
            let page = try await songRepo.searchSongsFTS(query: "", limit: 500, offset: offset)
            songs.append(contentsOf: page)
            if page.count < 500 { break }
            offset += page.count
        }
        let musicRoot = documentsURL.appendingPathComponent("Music", isDirectory: true).resolvingSymlinksInPath().standardizedFileURL
        var byFile: [URL: [Song]] = [:]
        for song in songs {
            guard let path = song.localFilePath else { continue }
            let file = documentsURL.appendingPathComponent(path).resolvingSymlinksInPath().standardizedFileURL
            guard file.deletingLastPathComponent() == musicRoot else { continue }
            byFile[file, default: []].append(song)
        }
        var changed = false
        for file in files.sorted(by: { $0.lastUse < $1.lastUse }) where total > maxBytes {
            let owners = byFile[file.url] ?? []
            let removed = try withOfflineCacheLock {
                let pinned = Set(preferences.stringArray(forKey: "lwPinnedSongs") ?? [])
                let active = preferences.string(forKey: "lwActiveSongID")
                let activePath = preferences.string(forKey: "lwActiveCachePath")
                let queuedPaths = Set(preferences.stringArray(forKey: "lwQueuedCachePaths") ?? [])
                guard activePath != file.url.path, !queuedPaths.contains(file.url.path),
                      !owners.contains(where: { pinned.contains(String($0.id ?? -1)) || active == String($0.id ?? -1) }) else { return false }
                guard FileManager.default.fileExists(atPath: file.url.path) else { return false }
                try FileManager.default.removeItem(at: file.url)
                return true
            }
            guard removed else { continue }
            total -= file.bytes
            for song in owners {
                if let id = song.id {
                    _ = try await songRepo.updateCachedFile(songId: id, localFilePath: nil, state: .bookmarkOnly, expectedState: song.fileState, expectedPath: song.localFilePath)
                }
            }
            changed = true
        }
        if changed { NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil) }
    }

    public static func cachedByteCount(documentsURL: URL? = nil) throws -> Int64 {
        let docs = documentsURL ?? FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        return try cacheFiles(in: docs).reduce(0) { $0 + $1.bytes }
    }

    private static func cacheFiles(in documentsURL: URL) throws -> [(url: URL, bytes: Int64, lastUse: Date)] {
        let directory = documentsURL.appendingPathComponent("Music", isDirectory: true)
        guard FileManager.default.fileExists(atPath: directory.path) else { return [] }
        let keys: Set<URLResourceKey> = [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey, .contentModificationDateKey]
        return try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: Array(keys)).compactMap { file in
            let values = try file.resourceValues(forKeys: keys)
            guard values.isRegularFile == true, values.isSymbolicLink != true, file.pathExtension != "partial" else { return nil }
            return (file.resolvingSymlinksInPath().standardizedFileURL, Int64(values.fileSize ?? 0), values.contentModificationDate ?? .distantPast)
        }
    }

    private func markFailed(song: Song) async {
        guard let id = song.id else { return }
        do { _ = try await songRepo.updateCachedFile(songId: id, localFilePath: song.localFilePath, state: .failed, expectedState: song.fileState, expectedPath: song.localFilePath) }
        catch { logger.error("Unable to record failed copy for song \(id): \(error)") }
    }
}
