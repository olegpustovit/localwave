import Foundation
import LocalWaveDomain
import LocalWaveCore
import os

public actor DefaultSourceSyncService: SourceSyncService {
    let logger = createLogger(subsystem: subsystem, category: "SourceSyncService")

    let sourceRepository: SourceRepository
    private let sourceAccess: SourceAccess
    private let excludedURLs: [URL]
    let sourcePathRepository: SourcePathRepository
    let sourcePathSearchRepository: SourcePathSearchRepository

    public init(
        sourceRepository: SourceRepository,
        sourcePathSearchRepository: SourcePathSearchRepository,
        sourcePathRepository: SourcePathRepository,
        sourceAccess: SourceAccess? = nil,
        excludedURLs: [URL] = LocalWaveGeneratedPaths.urls
    ) {
        self.sourceRepository = sourceRepository
        self.sourceAccess = sourceAccess ?? SourceAccess(repository: sourceRepository)
        self.excludedURLs = excludedURLs
        self.sourcePathRepository = sourcePathRepository
        self.sourcePathSearchRepository = sourcePathSearchRepository
    }

    public func syncPaths(sourceId: Int64, relativePaths: Set<String>) async throws -> (source: Source?, audioPaths: [SourcePath]) {
        guard !relativePaths.isEmpty else { return (try await sourceRepository.getOne(id: sourceId), []) }
        let root = try await sourceAccess.rootURL(sourceId: sourceId)
        let scoped = root.startAccessingSecurityScopedResource()
        defer { if scoped { root.stopAccessingSecurityScopedResource() } }
        guard let source = try await sourceRepository.getOne(id: sourceId) else {
            throw CustomError.genericError("This music source was removed.")
        }
        // A directory notification subsumes any child notifications in the same batch.
        let scopes = relativePaths.sorted().filter { path in
            !relativePaths.contains { $0 != path && path.hasPrefix($0 + "/") }
        }
        var before: [SourcePath] = []
        for path in scopes {
            _ = try SourceAccess.fileURL(root: root, relativePath: path)
            if let existing = try await sourcePathRepository.getByPath(relativePath: path, sourceId: sourceId) {
                var pending = [existing]
                while let item = pending.popLast() {
                    before.append(item)
                    if item.isDirectory {
                        pending += try await sourcePathRepository.getByParentId(sourceId: sourceId, parentPathId: item.pathId)
                    }
                }
            }
        }
        guard let result = try await syncDirInner(folderURL: root, onCurrentURL: nil, onSetLoading: nil, resolvedRootURL: root, relativePaths: Set(scopes)) else {
            throw CustomError.genericError("Unable to read the changed paths.")
        }
        let registeredRoot = makeURLFromString(source.dirPath)
        let runId = Int64(Date().timeIntervalSince1970 * 1000)
        var items = result.allItems.map { item -> SourcePath in
            let url = registeredRoot.appendingPathComponent(item.relativePath, isDirectory: item.isDirectory)
            return SourcePath(id: nil, sourceId: sourceId, pathId: makeURLHash(url), parentPathId: makeURLHash(url.deletingLastPathComponent()),
                              name: item.name, relativePath: item.relativePath, isDirectory: item.isDirectory, fileHashSHA256: nil,
                              runId: runId, createdAt: Date(), updatedAt: nil, fileSize: item.fileSize)
        }
        let afterAudioCount = items.filter(\.isAudio).count
        // Providers may report only the new file, without separate appearances for its parent folders.
        var ancestors = Set<String>()
        for item in items {
            var parent = (item.relativePath as NSString).deletingLastPathComponent
            while !parent.isEmpty {
                ancestors.insert(parent)
                parent = (parent as NSString).deletingLastPathComponent
            }
        }
        let known = Set(items.map(\.relativePath))
        for parent in ancestors.sorted() where !known.contains(parent) {
            if try await sourcePathRepository.getByPath(relativePath: parent, sourceId: sourceId) != nil { continue }
            let url = try SourceAccess.fileURL(root: root, relativePath: parent)
            guard try url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory == true else {
                throw CustomError.genericError("A parent folder is no longer available.")
            }
            let registered = registeredRoot.appendingPathComponent(parent, isDirectory: true)
            items.append(SourcePath(id: nil, sourceId: sourceId, pathId: makeURLHash(registered), parentPathId: makeURLHash(registered.deletingLastPathComponent()),
                                    name: registered.lastPathComponent, relativePath: parent, isDirectory: true, fileHashSHA256: nil,
                                    runId: runId, createdAt: Date(), updatedAt: nil))
        }
        let previous = Dictionary(uniqueKeysWithValues: before.map { ($0.relativePath, $0) })
        let changed = items.filter { item in
            guard let old = previous[item.relativePath] else { return true }
            return old.pathId != item.pathId || old.parentPathId != item.parentPathId || old.name != item.name || old.isDirectory != item.isDirectory || old.fileSize != item.fileSize
        }
        let remaining = Set(items.map(\.relativePath))
        let deleted = before.filter { !remaining.contains($0.relativePath) }.map(\.pathId)
        try Task.checkCancellation()
        try await sourcePathRepository.batchUpsert(paths: changed)
        try await sourcePathSearchRepository.batchUpsertIntoFTS(paths: changed)
        try await sourcePathSearchRepository.deletePaths(sourceId: sourceId, pathIds: deleted)
        try await sourcePathRepository.deletePaths(sourceId: sourceId, pathIds: deleted)
        guard var updated = try await sourceRepository.getOne(id: sourceId) else { return (nil, []) }
        updated.totalPaths = max(0, (updated.totalPaths ?? 0) - before.filter(\.isAudio).count + afterAudioCount)
        updated.lastSyncedAt = Date()
        updated.updatedAt = Date()
        updated.syncError = nil
        return (try await sourceRepository.updateSource(source: updated), items.filter(\.isAudio))
    }

    public func syncDir(
        sourceId: Int64, folderURL: URL, onCurrentURL: ((_ url: URL?) -> Void)?,
        onSetLoading: ((_ loading: Bool) -> Void)?
    ) async throws
        -> Source?
    {
        logger.debug("starting to collect items")
        do {
            onSetLoading?(true)
            defer { onSetLoading?(false) }
            let root = try await sourceAccess.rootURL(sourceId: sourceId)
            let liveSource = try await sourceRepository.getOne(id: sourceId)
            let registeredRoot = liveSource.map { makeURLFromString($0.dirPath) } ?? folderURL
            let result = try await syncDirInner(
                folderURL: folderURL, onCurrentURL: onCurrentURL, onSetLoading: onSetLoading, resolvedRootURL: root
            )
            let runId = Int64(Date().timeIntervalSince1970 * 1000)

            let itemsToCreate = result?.allItems.map { x in
                // Bookmark resolution can change /var to /private/var; keep IDs relative to the registered root.
                let registeredURL = registeredRoot.appendingPathComponent(x.relativePath, isDirectory: x.isDirectory)
                let parentPathId = makeURLHash(registeredURL.deletingLastPathComponent())
                let pathId = makeURLHash(registeredURL)
                return SourcePath(
                    id: nil,
                    sourceId: sourceId,
                    pathId: pathId,
                    parentPathId: parentPathId,
                    name: x.name,
                    relativePath: x.relativePath,
                    isDirectory: x.isDirectory,
                    fileHashSHA256: nil,
                    runId: runId,
                    createdAt: Date(),
                    updatedAt: nil,
                    fileSize: x.fileSize
                )
            }

            let items = itemsToCreate ?? []
            let numberOfItemsToUpsert = items.count

            logger.debug("upserting \(numberOfItemsToUpsert) items")
            try Task.checkCancellation()
            try await sourcePathRepository.batchUpsert(paths: items)
            try await sourcePathSearchRepository.batchUpsertIntoFTS(paths: items)

            logger.debug("removing stale paths...")
            let deletedCount = try await sourcePathRepository.deleteMany(
                sourceId: sourceId, excludingRunId: runId
            )
            try await sourcePathSearchRepository.batchDeleteFTS(
                sourceId: sourceId, excludingRunId: runId
            )
            logger.debug("removed \(deletedCount) stale paths")

            if let result = result {
                let totalAudioFiles = result.totalAudioFiles
                logger.debug("updating source \(sourceId)")
                if var lib = try await sourceRepository.getOne(id: sourceId) {
                    lib.lastSyncedAt = Date()
                    lib.updatedAt = Date()
                    lib.totalPaths = totalAudioFiles
                    lib.syncError = nil
                    return try await sourceRepository.updateSource(source: lib)
                } else {
                    logger.error("for some reason source \(sourceId) wasn't found")
                }
            }

            if var lib = try await sourceRepository.getOne(id: sourceId) {
                lib.lastSyncedAt = Date()
                lib.updatedAt = Date()
                lib.totalPaths = result?.totalAudioFiles ?? 0 // Ensure totalPaths is set
                lib.syncError = nil
                return try await sourceRepository.updateSource(source: lib)
            }
            return nil
        } catch is CancellationError {
            throw CancellationError()
        } catch {
            logger.error("sync dir error: \(error)")
            if var lib = try await sourceRepository.getOne(id: sourceId) {
                lib.lastSyncedAt = Date()
                lib.updatedAt = Date()
                lib.syncError = error.localizedDescription
                return try await sourceRepository.updateSource(source: lib)
            }
            return nil
        }
    }

    public func syncDirInner(
        folderURL: URL,
        onCurrentURL: ((_ url: URL) -> Void)?,
        onSetLoading _: ((_ loading: Bool) -> Void)?,
        resolvedRootURL: URL? = nil,
        relativePaths: Set<String>? = nil
    ) async throws
        -> SourceSyncResult?
    {
        var audioURLs: [SourceSyncResultItem] = []
        var result = [String: SourceSyncResultItem]()
        let audioExtensions = AudioFileSupport.extensions
        do {
            let scanRoot: URL
            var scoped = false
            if let resolvedRootURL {
                scanRoot = resolvedRootURL
                scoped = scanRoot.startAccessingSecurityScopedResource()
                guard scoped || FileManager.default.isReadableFile(atPath: scanRoot.path) else {
                    throw CustomError.genericError("Folder access has expired. Reconnect its source folder.")
                }
            }
            else {
                let key = makeBookmarkKey(folderURL)
                guard let bookmark = LocalWavePreferences.current.data(forKey: key) else {
                    throw CustomError.genericError("No bookmark found. Choose the source folder again.")
                }
                var stale = false
                scanRoot = try URL(resolvingBookmarkData: bookmark, options: sourceBookmarkResolutionOptions, bookmarkDataIsStale: &stale)
                scoped = scanRoot.startAccessingSecurityScopedResource()
                guard scoped || FileManager.default.isReadableFile(atPath: scanRoot.path) else {
                    throw CustomError.genericError("Folder access has expired. Reconnect its source folder.")
                }
            }
            defer { if scoped { scanRoot.stopAccessingSecurityScopedResource() } }
            let folderURL = scanRoot.resolvingSymlinksInPath()
            guard !LocalWaveGeneratedPaths.contains(folderURL, excludedURLs: excludedURLs) else {
                throw CustomError.genericError("Choose your original music folder. LocalWave’s generated cache cannot be used as a source.")
            }

            let keys: Set<URLResourceKey> = [.isDirectoryKey, .fileSizeKey]
            func collect(_ original: URL) throws -> Bool {
                let file = original.resolvingSymlinksInPath()
                try Task.checkCancellation()
                if LocalWaveGeneratedPaths.contains(file, excludedURLs: excludedURLs) { return false }
                let relative = FileHelper(fileURL: file).relativePath(from: folderURL) ?? ""
                _ = try SourceAccess.fileURL(root: folderURL, relativePath: relative)
                let values = try file.resourceValues(forKeys: keys)
                let item = SourceSyncResultItem(rootURL: folderURL, current: file, isDirectory: values.isDirectory == true, fileSize: values.fileSize.map(Int64.init))
                onCurrentURL?(file)
                result[relative] = item
                if !item.isDirectory && audioExtensions.contains(file.pathExtension.lowercased()) { audioURLs.append(item) }
                return item.isDirectory
            }
            let starts = try relativePaths?.sorted().map { try SourceAccess.fileURL(root: folderURL, relativePath: $0) } ?? [folderURL]
            for start in starts {
                if start != folderURL {
                    do { if try !collect(start) { continue } }
                    catch let error as NSError where error.domain == NSCocoaErrorDomain && [NSFileNoSuchFileError, NSFileReadNoSuchFileError].contains(error.code) {
                        continue
                    }
                }
                var enumerationError: Error?
                guard let enumerator = FileManager.default.enumerator(
                    at: start, includingPropertiesForKeys: Array(keys), options: [.skipsHiddenFiles, .skipsPackageDescendants],
                    errorHandler: { _, error in enumerationError = error; return false }
                ) else { throw CustomError.genericError("Unable to read the source folder.") }
                for case let file as URL in enumerator {
                    if LocalWaveGeneratedPaths.contains(file, excludedURLs: excludedURLs) { enumerator.skipDescendants(); continue }
                    _ = try collect(file)
                }
                if let enumerationError { throw enumerationError }
            }
            return SourceSyncResult(allItems: Array(result.values), audioFiles: audioURLs, totalAudioFiles: audioURLs.count)
        } catch is CancellationError {
            throw CancellationError()
        } catch {
            throw CustomError.genericError("error resolving bookmark: \(error)")
        }
    }
}
