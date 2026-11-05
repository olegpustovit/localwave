import Foundation
import LocalWaveCore
import LocalWaveDomain

/// Owns one live grant per source bookmark; songs keep only the source ID and a relative path.
public actor SourceAccess {
    private let repository: SourceRepository
    private let preferences: UserDefaults
    private let excludedURLs: [URL]
    private let sourcePathRepository: SourcePathRepository?
    private let songRepository: SongRepository?
    private struct Root {
        var bookmark: Data
        let originalBookmark: Data
        let url: URL
        let scoped: Bool
    }
    private var roots: [Int64: Root] = [:]
    private var presenters: [Int64: SourceFolderPresenter] = [:]
    private var fingerprints: [Int64: Int] = [:]
    private var scanBaselines: [Int64: Int] = [:]
    private var scanning = Set<Int64>()
    private var changeSubscribers: [UUID: AsyncStream<Int64>.Continuation] = [:]

    public init(repository: SourceRepository, preferences: UserDefaults = LocalWavePreferences.current, excludedURLs: [URL] = LocalWaveGeneratedPaths.urls, sourcePathRepository: SourcePathRepository? = nil, songRepository: SongRepository? = nil) {
        self.repository = repository
        self.preferences = preferences
        self.excludedURLs = excludedURLs
        self.sourcePathRepository = sourcePathRepository
        self.songRepository = songRepository
    }

    deinit {
        changeSubscribers.values.forEach { $0.finish() }
        presenters.values.forEach { $0.invalidate() }
        for root in roots.values where root.scoped { root.url.stopAccessingSecurityScopedResource() }
    }

    public func rootURL(sourceId: Int64) async throws -> URL {
        guard let source = try await repository.getOne(id: sourceId) else {
            releaseRoot(sourceId: sourceId)
            throw CustomError.genericError("This music source was removed.")
        }
        let legacyKey = makeBookmarkKey(makeURLFromString(source.dirPath))
        guard let bookmark = source.bookmark ?? preferences.data(forKey: legacyKey) else {
            throw CustomError.genericError("Folder access is missing. Reconnect its source folder.")
        }
        if let cached = roots[sourceId], cached.bookmark == bookmark || cached.originalBookmark == bookmark,
           FileManager.default.isReadableFile(atPath: cached.url.path) { return cached.url }
        var stale = false
        let url = try URL(resolvingBookmarkData: bookmark, options: sourceBookmarkResolutionOptions, bookmarkDataIsStale: &stale)
        guard !LocalWaveGeneratedPaths.contains(url, excludedURLs: excludedURLs) else {
            throw CustomError.genericError("Choose your original music folder. LocalWave’s generated cache cannot be used as a source.")
        }
        let scoped = url.startAccessingSecurityScopedResource()
        guard scoped || FileManager.default.isReadableFile(atPath: url.path) else {
            throw CustomError.genericError("Folder access has expired. Reconnect its source folder.")
        }
        let refreshed: Data
        do { refreshed = stale ? try url.bookmarkData(options: sourceBookmarkCreationOptions) : bookmark }
        catch { if scoped { url.stopAccessingSecurityScopedResource() }; throw error }
        releaseRoot(sourceId: sourceId)
        roots[sourceId] = Root(bookmark: refreshed, originalBookmark: bookmark, url: url, scoped: scoped)
        let moved = makeURLFromString(source.dirPath).resolvingSymlinksInPath().standardizedFileURL.path != url.resolvingSymlinksInPath().standardizedFileURL.path
        do {
            if moved {
                try await migrateLegacySongs(source: source, root: url)
                let saved = try await repository.updateResolvedSourceRoot(sourceId: sourceId, dirPath: url.path, pathId: makeURLHash(url), bookmark: refreshed, expectedBookmark: source.bookmark)
                if !saved {
                    if roots[sourceId]?.bookmark == refreshed { releaseRoot(sourceId: sourceId) }
                    return try await rootURL(sourceId: sourceId)
                }
            } else if source.bookmark == nil || stale {
                _ = try await repository.updateSourceBookmark(sourceId: sourceId, bookmark: refreshed, expectedBookmark: source.bookmark)
            }
        } catch {
            if roots[sourceId]?.bookmark == refreshed, roots[sourceId]?.url == url { releaseRoot(sourceId: sourceId) }
            throw error
        }
        if roots[sourceId]?.url == url, roots[sourceId]?.bookmark == refreshed {
            fingerprints[sourceId] = try? fingerprint(url)
            presenters[sourceId] = SourceFolderPresenter(url: url) { [weak self] movedRoot in
                Task { await self?.sourceChanged(sourceId: sourceId, watchedRoot: url, movedRoot: movedRoot) }
            }
        }
        return url
    }

    public func fileURL(sourceId: Int64, relativePath: String) async throws -> (root: URL, file: URL) {
        let root = try await rootURL(sourceId: sourceId)
        return (root, try Self.fileURL(root: root, relativePath: relativePath))
    }

    public func availability(for song: Song) async -> FileAvailability {
        #if DEBUG
        if ProcessInfo.processInfo.environment["LOCALWAVE_REFERENCE_FIXTURE"] == "1",
           UUID(uuidString: ProcessInfo.processInfo.environment["LOCALWAVE_UI_TEST_ID"] ?? "") != nil,
           let state = song.sourceAvailability { return state }
        #endif
        if let path = song.localFilePath {
            let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
            let file = path.hasPrefix("/") ? URL(fileURLWithPath: path) : documents.appendingPathComponent(path)
            if let state = try? FileAvailabilityService.inspect(file, knownLocalCopy: true), state == .onDevice { return state }
        }
        if let id = song.sourceId, let relativePath = song.relativePath,
           let access = try? await fileURL(sourceId: id, relativePath: relativePath) {
            return (try? FileAvailabilityService.inspect(access.file)) ?? .unknown
        }
        if let bookmark = song.bookmark {
            var stale = false
            if let file = try? URL(resolvingBookmarkData: bookmark, options: sourceBookmarkResolutionOptions, bookmarkDataIsStale: &stale) {
                let scoped = file.startAccessingSecurityScopedResource()
                defer { if scoped { file.stopAccessingSecurityScopedResource() } }
                return (try? FileAvailabilityService.inspect(file)) ?? .unknown
            }
        }
        return .unknown
    }

    public nonisolated static func fileURL(root: URL, relativePath: String) throws -> URL {
        guard !relativePath.isEmpty, !relativePath.hasPrefix("/"), !relativePath.split(separator: "/").contains("..") else {
            throw CustomError.genericError("The audio path is outside its source folder.")
        }
        let file = root.appendingPathComponent(relativePath).standardizedFileURL
        let canonicalRoot = root.resolvingSymlinksInPath().standardizedFileURL.path
        let canonicalFile = file.resolvingSymlinksInPath().path
        guard canonicalFile.hasPrefix(canonicalRoot.hasSuffix("/") ? canonicalRoot : canonicalRoot + "/") else {
            throw CustomError.genericError("The audio path is outside its source folder.")
        }
        return file
    }

    public func reconnect(sourceId: Int64, to url: URL) async throws -> Source {
        guard !LocalWaveGeneratedPaths.contains(url, excludedURLs: excludedURLs) else {
            throw CustomError.genericError("Choose your original music folder. LocalWave’s generated cache cannot be used as a source.")
        }
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        guard scoped || FileManager.default.isReadableFile(atPath: url.path) else {
            throw CustomError.genericError("Unable to access this folder. Choose it again in Files.")
        }
        try await migrateLegacySongs(sourceId: sourceId)
        let bookmark = try url.bookmarkData(options: sourceBookmarkCreationOptions)
        guard let updated = try await repository.reconnectSource(sourceId: sourceId, dirPath: url.path, pathId: makeURLHash(url), bookmark: bookmark) else {
            throw CustomError.genericError("This music source was removed.")
        }
        releaseRoot(sourceId: sourceId)
        _ = try await rootURL(sourceId: sourceId)
        return updated
    }

    public func remove(sourceId: Int64) async throws {
        try await migrateLegacySongs(sourceId: sourceId)
        try await repository.deleteSource(sourceId: sourceId)
        releaseRoot(sourceId: sourceId)
    }

    public func migrateLegacySongs(sourceId: Int64) async throws {
        guard let source = try await repository.getOne(id: sourceId) else { return }
        let root = (try? await rootURL(sourceId: sourceId)) ?? makeURLFromString(source.dirPath).resolvingSymlinksInPath()
        try await migrateLegacySongs(source: source, root: root)
    }

    private func migrateLegacySongs(source: Source, root: URL) async throws {
        guard let sourcePathRepository, let songRepository, let sourceId = source.id else { return }
        let paths = try await sourcePathRepository.getByParentId(sourceId: sourceId, parentPathId: nil)
        let registeredRoot = makeURLFromString(source.dirPath).resolvingSymlinksInPath()
        var relativePaths: [Int64: String] = [:]
        for path in paths where !path.isDirectory {
            relativePaths[path.pathId] = path.relativePath
            relativePaths[makeURLHash(root.appendingPathComponent(path.relativePath))] = path.relativePath
            relativePaths[makeURLHash(registeredRoot.appendingPathComponent(path.relativePath))] = path.relativePath
        }
        let songs = try await songRepository.searchSongsFTS(query: "", limit: Int.max, offset: 0)
        for song in songs where song.sourceId == nil {
            if let id = song.id, let path = relativePaths[song.pathHash] {
                try await songRepository.updateSourceLocation(songId: id, sourceId: sourceId, relativePath: path)
            }
        }
    }

    private func releaseRoot(sourceId: Int64) {
        presenters.removeValue(forKey: sourceId)?.invalidate()
        fingerprints.removeValue(forKey: sourceId)
        if let root = roots.removeValue(forKey: sourceId), root.scoped { root.url.stopAccessingSecurityScopedResource() }
    }

    public func changes() -> AsyncStream<Int64> {
        let id = UUID()
        return AsyncStream(bufferingPolicy: .bufferingNewest(32)) { continuation in
            changeSubscribers[id] = continuation
            continuation.onTermination = { [weak self] _ in Task { await self?.removeSubscriber(id) } }
        }
    }

    private func removeSubscriber(_ id: UUID) { changeSubscribers.removeValue(forKey: id) }

    public func beginScan(sourceId: Int64) async {
        scanning.insert(sourceId)
        if let root = try? await rootURL(sourceId: sourceId) { scanBaselines[sourceId] = try? fingerprint(root) }
    }

    public func endScan(sourceId: Int64) {
        scanning.remove(sourceId)
        let before = scanBaselines.removeValue(forKey: sourceId)
        guard let root = roots[sourceId]?.url else { return }
        let after = try? fingerprint(root)
        fingerprints[sourceId] = after
        if before != after { emitChange(sourceId) }
    }

    private func sourceChanged(sourceId: Int64, watchedRoot: URL, movedRoot: URL?) async {
        guard roots[sourceId]?.url == watchedRoot else { return }
        if let movedRoot {
            let scoped = movedRoot.startAccessingSecurityScopedResource()
            defer { if scoped { movedRoot.stopAccessingSecurityScopedResource() } }
            do {
                if scoped || FileManager.default.isReadableFile(atPath: movedRoot.path), let source = try await repository.getOne(id: sourceId) {
                    let bookmark = try movedRoot.bookmarkData(options: sourceBookmarkCreationOptions)
                    try await migrateLegacySongs(source: source, root: movedRoot)
                    let saved = try await repository.updateResolvedSourceRoot(sourceId: sourceId, dirPath: movedRoot.path, pathId: makeURLHash(movedRoot), bookmark: bookmark, expectedBookmark: source.bookmark)
                    if saved, roots[sourceId]?.url == watchedRoot { releaseRoot(sourceId: sourceId) }
                }
            } catch {
                // The queued rescan below retries the durable update and surfaces access or storage failures.
            }
            _ = try? await rootURL(sourceId: sourceId)
            emitChange(sourceId)
            return
        }
        guard !scanning.contains(sourceId) else { return }
        let current = try? fingerprint(watchedRoot)
        guard current != fingerprints[sourceId] else { return }
        fingerprints[sourceId] = current
        emitChange(sourceId)
    }

    private func emitChange(_ sourceId: Int64) {
        changeSubscribers.values.forEach { $0.yield(sourceId) }
    }

    private func fingerprint(_ root: URL) throws -> Int {
        let keys: Set<URLResourceKey> = [.isDirectoryKey, .contentModificationDateKey, .fileSizeKey, .isUbiquitousItemKey, .ubiquitousItemDownloadingStatusKey]
        var enumerationError: Error?
        guard let enumerator = FileManager.default.enumerator(at: root, includingPropertiesForKeys: Array(keys), options: [.skipsHiddenFiles, .skipsPackageDescendants], errorHandler: { _, error in enumerationError = error; return false }) else {
            throw CustomError.genericError("Unable to read the source folder.")
        }
        var entries: [String] = []
        for case let file as URL in enumerator {
            if LocalWaveGeneratedPaths.contains(file, excludedURLs: excludedURLs) { enumerator.skipDescendants(); continue }
            let values = try file.resourceValues(forKeys: keys)
            let path = FileHelper(fileURL: file).relativePath(from: root) ?? file.path
            if values.isDirectory == true { entries.append(path + "/") }
            else if values.isDirectory == false {
                entries.append("\(path)\u{001F}\(values.contentModificationDate?.timeIntervalSince1970 ?? 0)\u{001F}\(values.fileSize ?? 0)\u{001F}\(String(describing: values.ubiquitousItemDownloadingStatus))")
            }
        }
        if let enumerationError { throw enumerationError }
        var hasher = Hasher()
        entries.sorted().forEach { hasher.combine($0) }
        return hasher.finalize()
    }
}
