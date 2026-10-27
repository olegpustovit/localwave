import Foundation
import LocalWaveDomain
import LocalWaveCore
import os
import SQLite

public actor SQLiteSourcePathSearchRepository: SourcePathSearchRepository {
    public func search(sourceId: Int64, query: String, limit: Int) async throws -> [PathSearchResult] {
        let processedQuery = preprocessFTSQuery(query)

        let sql = """
        SELECT pathId, bm25(source_paths_fts) AS rank
        FROM source_paths_fts
        WHERE source_paths_fts MATCH ?
            AND sourceId = ?
        ORDER BY rank
        LIMIT ?;
        """

        var results: [PathSearchResult] = []
        for row in try db.prepare(sql, processedQuery, sourceId, limit) {
            let pathId = row[0] as? Int64 ?? 0
            let rank = row[1] as? Double ?? 0.0
            results.append(PathSearchResult(pathId: pathId, rank: rank))
        }
        return results
    }

    public func deleteAllFTS(sourceId: Int64) async throws {
        let query = ftsTable.filter(colFtsSourceId == sourceId)
        try db.run(query.delete())
        logger.debug("Deleted all FTS entries for source: \(sourceId)")
    }

    private let logger = createLogger(subsystem: subsystem, category: "SourcePathSearchRepository")

    // MARK: - Batch Delete by sourceId, excluding runId

    public func batchDeleteFTS(sourceId: Int64, excludingRunId: Int64) async throws {
        // Delete all rows with this sourceId where runId != excludingRunId
        let query = ftsTable.filter(
            colFtsSourceId == sourceId && colFtsRunId != excludingRunId
        )
        try db.transaction {
            try db.run(query.delete())
        }
    }

    // MARK: - Batch Upsert

    /// If `(sourceId, pathId)` already exists, we update `runId`, `fullPath`, `fileName`.
    /// Otherwise, we insert a new row.
    public func batchUpsertIntoFTS(paths: [SourcePath]) async throws {
        guard !paths.isEmpty else { return }

        try db.transaction {
            for path in paths {
                let existingQuery = self.ftsTable.filter(
                    self.colFtsPathId == path.pathId && self.colFtsSourceId == path.sourceId
                )
                if try db.pluck(existingQuery) != nil {
                    // Update
                    try db.run(
                        existingQuery.update(
                            self.colFtsRunId <- path.runId,
                            self.colFtsFullPath <- path.relativePath,
                            self.colFtsFileName <- path.name
                        )
                    )
                } else {
                    // Insert
                    try db.run(
                        self.ftsTable.insert(
                            self.colFtsPathId <- path.pathId,
                            self.colFtsSourceId <- path.sourceId,
                            self.colFtsRunId <- path.runId,
                            self.colFtsFullPath <- path.relativePath,
                            self.colFtsFileName <- path.name
                        )
                    )
                }
            }
        }
    }

    private let db: Connection
    private let ftsTable = Table("source_paths_fts")
    private let colFtsPathId = SQLite.Expression<Int64>("pathId")
    private let colFtsSourceId = SQLite.Expression<Int64>("sourceId")
    private let colFtsRunId = SQLite.Expression<Int64>("runId")
    private let colFtsFullPath = SQLite.Expression<String>("fullPath")
    private let colFtsFileName = SQLite.Expression<String>("fileName")

    public init(db: Connection) throws {
        self.db = db
        try db.execute(
            """
            CREATE VIRTUAL TABLE IF NOT EXISTS source_paths_fts
            USING fts5(
                pathId UNINDEXED,
                sourceId UNINDEXED,
                runId UNINDEXED,
                fullPath,
                fileName,
                tokenize='unicode61'
            );
            """)
    }
}
