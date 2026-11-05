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
        } catch {
            logger.error("sync dir error: \(error)")
            if var lib = try await sourceRepository.getOne(id: sourceId) {
                lib.lastSyncedAt = Date()
                lib.updatedAt = Date()
                lib.syncError = "\(error)"
                return try await sourceRepository.updateSource(source: lib)
            }
            return nil
        }
    }

    public func syncDirInner(
        folderURL: URL,
        onCurrentURL: ((_ url: URL) -> Void)?,
        onSetLoading _: ((_ loading: Bool) -> Void)?,
        resolvedRootURL: URL? = nil
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
            let folderURL = scanRoot
            guard !LocalWaveGeneratedPaths.contains(folderURL, excludedURLs: excludedURLs) else {
                throw CustomError.genericError("Choose your original music folder. LocalWave’s generated cache cannot be used as a source.")
            }

            // Now we can scan the folder
            if let enumerator = FileManager.default.enumerator(
                at: folderURL, includingPropertiesForKeys: [.isDirectoryKey, .fileSizeKey],
                options: [.skipsHiddenFiles, .skipsPackageDescendants]
            ) {
                for case let file as URL in enumerator {
                    if LocalWaveGeneratedPaths.contains(file, excludedURLs: excludedURLs) {
                        enumerator.skipDescendants()
                        continue
                    }
                    do {
                        onCurrentURL?(file)
                        let resourceValues = try file.resourceValues(forKeys: [.isDirectoryKey, .fileSizeKey])
                        if resourceValues.isDirectory == false {
                            let resultItem = SourceSyncResultItem(
                                rootURL: folderURL, current: file, isDirectory: false, fileSize: resourceValues.fileSize.map(Int64.init)
                            )
                            if audioExtensions.contains(file.pathExtension.lowercased()) { audioURLs.append(resultItem) }
                            result[FileHelper(fileURL: file).toString()] = resultItem
                        } else if resourceValues.isDirectory == true {
                            let resultItem = SourceSyncResultItem(
                                rootURL: folderURL, current: file, isDirectory: true
                            )
                            result[FileHelper(fileURL: file).toString()] = resultItem
                        }
                    } catch {
                        logger.error("Error reading resource values: \(error)")
                    }
                }
                logger.debug("Total audio files found: \(audioURLs.count)")
                return SourceSyncResult(
                    allItems: Array(result.values), audioFiles: audioURLs,
                    totalAudioFiles: audioURLs.count
                )
            } else {
                throw CustomError.genericError("failed to get enumerator")
            }
        } catch {
            throw CustomError.genericError("error resolving bookmark: \(error)")
        }
    }
}
