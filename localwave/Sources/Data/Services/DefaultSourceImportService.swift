
import os
import LocalWaveDomain
import LocalWaveCore

public actor DefaultSourceImportService: SourceImportService {
    let logger = createLogger(subsystem: subsystem, category: "SourceImportService")

    private let sourceRepository: SourceRepository
    private let sourcePathRepository: SourcePathRepository
    private let sourcePathSearchRepository: SourcePathSearchRepository

    public init(
        sourceRepository: SourceRepository,
        sourcePathRepository: SourcePathRepository,
        sourcePathSearchRepository: SourcePathSearchRepository
    ) {
        self.sourceRepository = sourceRepository
        self.sourcePathRepository = sourcePathRepository
        self.sourcePathSearchRepository = sourcePathSearchRepository
    }

    public func deleteOne(sourceId: Int64) async throws {
        try await sourceRepository.deleteSource(sourceId: sourceId)
        try await sourcePathRepository.deleteAllPaths(sourceId: sourceId)
        try await sourcePathSearchRepository.deleteAllFTS(sourceId: sourceId)
    }

    public func listItems(sourceId: Int64, parentPathId: Int64?) async throws -> [SourcePath] {
        logger.debug("attempting to list for libID : \(sourceId), parent: \(parentPathId ?? -1)")
        let all = try await sourcePathRepository.getByParentId(
            sourceId: sourceId, parentPathId: parentPathId
        )

        logger.debug("got : \(all.count) items")

        return all
    }

    public func search(sourceId: Int64, query: String) async throws -> [SourcePath] {
        let results = try await sourcePathSearchRepository.search(
            sourceId: sourceId,
            query: query,
            limit: 100 // or whatever you like
        )

        // Retrieve the actual SourcePath records for each matching pathId
        var paths = [SourcePath]()
        for r in results {
            if let p = try await sourcePathRepository.getByPathId(
                sourceId: sourceId, pathId: r.pathId
            ) {
                paths.append(p)
            }
        }
        return paths
    }
}
