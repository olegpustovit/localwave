import Foundation
import LocalWaveDomain
import LocalWaveCore
import os

public actor DefaultSourceSyncService: SourceSyncService {
    let logger = createLogger(subsystem: subsystem, category: "SourceSyncService")

    let sourceRepository: SourceRepository
    let sourcePathRepository: SourcePathRepository
    let sourcePathSearchRepository: SourcePathSearchRepository

    public init(
        sourceRepository: SourceRepository,
        sourcePathSearchRepository: SourcePathSearchRepository,
        sourcePathRepository: SourcePathRepository
    ) {
        self.sourceRepository = sourceRepository
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
            let result = try await syncDirInner(
                folderURL: folderURL, onCurrentURL: onCurrentURL, onSetLoading: onSetLoading
            )
            let runId = Int64(Date().timeIntervalSince1970 * 1000)

            let itemsToCreate = result?.allItems.map { x in
                var parentPathId: Int64? = nil
                if let parentURL = x.parentURL {
                    parentPathId = makeURLHash(parentURL)
                    logger.debug(
                        "\(x.url) is creating parent path [\(parentPathId ?? -1)]: \(parentURL.absoluteString)"
                    )
                }
                let pathId = makeURLHash(x.url)
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
                    updatedAt: nil
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
                    return try await sourceRepository.updateSource(source: lib)
                } else {
                    logger.error("for some reason source \(sourceId) wasn't found")
                }
            }

            if var lib = try await sourceRepository.getOne(id: sourceId) {
                lib.lastSyncedAt = Date()
                lib.updatedAt = Date()
                lib.totalPaths = result?.totalAudioFiles ?? 0 // Ensure totalPaths is set
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
        onSetLoading _: ((_ loading: Bool) -> Void)?
    ) async throws
        -> SourceSyncResult?
    {
        var audioURLs: [SourceSyncResultItem] = []
        var result = [String: SourceSyncResultItem]()
        let bookmarkKey = makeBookmarkKey(folderURL)
        let audioExtensions = ["mp3", "wav", "m4a", "flac", "aac", "aiff", "aif"]
        guard let bookmarkData = UserDefaults.standard.data(forKey: bookmarkKey) else {
            throw CustomError.genericError("no bookmark found, pick folder")
        }

        var isStale = false
        do {
            let folderURL = try URL(
                resolvingBookmarkData: bookmarkData,
                options: [],
                relativeTo: nil,
                bookmarkDataIsStale: &isStale
            )

            if isStale {
                // The bookmark is stale, so we need a new one
                throw CustomError.genericError(
                    "Bookmark is stale, user needs to pick folder again.")
            }

            // Start accessing security-scoped resource
            guard folderURL.startAccessingSecurityScopedResource() else {
                throw CustomError.genericError("Couldn't start accessing security scoped resource.")
            }
            defer { folderURL.stopAccessingSecurityScopedResource() }

            // Now we can scan the folder
            if let enumerator = FileManager.default.enumerator(
                at: folderURL, includingPropertiesForKeys: [.isDirectoryKey],
                options: [.skipsHiddenFiles, .skipsPackageDescendants]
            ) {
                for case let file as URL in enumerator {
                    do {
                        onCurrentURL?(file)
                        let resourceValues = try file.resourceValues(forKeys: [.isDirectoryKey])
                        if resourceValues.isDirectory == false, // file.pathExtension.lowercased() == "mp3"
                           audioExtensions.contains(file.pathExtension.lowercased())
                        {
                            let resultItem = SourceSyncResultItem(
                                rootURL: folderURL, current: file, isDirectory: false
                            )
                            audioURLs.append(resultItem)
                            result[FileHelper(fileURL: file).toString()] = resultItem
                        } else {
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
