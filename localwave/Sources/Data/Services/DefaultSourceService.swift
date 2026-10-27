import AVFoundation
import os
import LocalWaveDomain
import LocalWaveCore

public class DefaultSourceService: SourceService {
    public func importService() -> any SourceImportService {
        return sourceImportService
    }

    let logger = createLogger(subsystem: subsystem, category: "SourceService")

    public func repository() -> SourceRepository {
        return sourceRepo
    }

    public func getCurrentSource(userId: Int64) async throws -> Source? {
        let sources = try await sourceRepo.findOneByUserId(userId: userId, path: nil)
        if sources.count == 0 {
            return nil
        } else if let lib = sources.first(where: { $0.isCurrent }) {
            return lib
        } else {
            let lib = sources[0]
            return try await sourceRepo.setCurrentSource(userId: userId, sourceId: lib.id!)
        }
    }

    public func registerSourcePath(userId: Int64, path: String, type: SourceType) async throws -> Source {
        let source = try await sourceRepo.findOneByUserId(userId: userId, path: path)
        if source.count == 0 {
            logger.debug("no source found, creating new one")
            let pathId = makeURLHash(makeURLFromString(path))
            let src = Source(
                id: nil, dirPath: path, pathId: pathId, userId: userId, type: type, totalPaths: nil,
                syncError: nil,
                isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil
            )
            let source = try await sourceRepo.create(source: src)
            logger.debug("updating current switch")
            let lib2 = try await sourceRepo.setCurrentSource(
                userId: userId, sourceId: source.id!
            )
            return lib2
        } else if !source[0].isCurrent {
            logger.debug("source is found, but it's not current")
            let lib2 = try await sourceRepo.setCurrentSource(
                userId: userId, sourceId: source[0].id!
            )
            return lib2
        } else {
            return source[0]
        }
    }

    public func syncService() -> SourceSyncService {
        return sourceSyncService
    }

    private var sourceRepo: SourceRepository
    private var sourceImportService: SourceImportService
    private var sourceSyncService: SourceSyncService

    public init(
        sourceRepo: SourceRepository, sourceSyncService: SourceSyncService,
        sourceImportService: SourceImportService
    ) {
        self.sourceRepo = sourceRepo
        self.sourceSyncService = sourceSyncService
        self.sourceImportService = sourceImportService
    }
}
