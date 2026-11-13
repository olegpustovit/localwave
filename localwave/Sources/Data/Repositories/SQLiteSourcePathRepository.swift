import Foundation
import LocalWaveDomain
import LocalWaveCore
import os
import SQLite

public actor SQLiteSourcePathRepository: SourcePathRepository {
    private let db: Connection
    private let table = Table("source_paths")

    private let colId: SQLite.Expression<Int64>
    private let colSourceId: SQLite.Expression<Int64>
    private let colPathId: SQLite.Expression<Int64>
    private let colParentPathId: SQLite.Expression<Int64?>
    private let colName: SQLite.Expression<String>
    private let colRelativePath: SQLite.Expression<String>
    private let colIsDirectory: SQLite.Expression<Bool>
    private let colFileSize: SQLite.Expression<Int64?>
    private let colFileHashSHA256: SQLite.Expression<Data?>
    private let colRunId: SQLite.Expression<Int64>
    private let colCreatedAt: SQLite.Expression<Date>
    private let colUpdatedAt: SQLite.Expression<Date?>

    private let logger = createLogger(subsystem: subsystem, category: "SQLiteSourcePathRepository")

    public func deletePaths(sourceId: Int64, pathIds: [Int64]) async throws {
        guard !pathIds.isEmpty else { return }
        try db.transaction {
            for id in pathIds { try db.run(table.filter(colSourceId == sourceId && colPathId == id).delete()) }
        }
    }

    public func getByPathId(sourceId: Int64, pathId: Int64) async throws -> SourcePath? {
        let query = table.filter(colSourceId == sourceId && colPathId == pathId)
        if let row = try db.pluck(query) {
            return SourcePath(
                id: row[colId],
                sourceId: row[colSourceId],
                pathId: row[colPathId],
                parentPathId: row[colParentPathId],
                name: row[colName],
                relativePath: row[colRelativePath],
                isDirectory: row[colIsDirectory],
                fileHashSHA256: row[colFileHashSHA256],
                runId: row[colRunId],
                createdAt: row[colCreatedAt],
                updatedAt: row[colUpdatedAt],
                fileSize: row[colFileSize]
            )
        }
        return nil
    }

    public func deleteAllPaths(sourceId: Int64) async throws {
        let query = table.filter(colSourceId == sourceId)
        try db.run(query.delete())
        logger.debug("Deleted all paths for source: \(sourceId)")
    }

    public func getByParentId(sourceId: Int64, parentPathId: Int64?) async throws -> [SourcePath] {
        let rows: AnySequence<Row>
        if let parentId = parentPathId {
            let query = table.filter(colSourceId == sourceId && colParentPathId == parentId)
            rows = try db.prepare(query)
        } else {
            let query = table.filter(colSourceId == sourceId)
            rows = try db.prepare(query)
        }
        return rows.map { row in
            SourcePath(
                id: row[colId],
                sourceId: row[colSourceId],
                pathId: row[colPathId],
                parentPathId: row[colParentPathId],
                name: row[colName],
                relativePath: row[colRelativePath],
                isDirectory: row[colIsDirectory],
                fileHashSHA256: row[colFileHashSHA256],
                runId: row[colRunId],
                createdAt: row[colCreatedAt],
                updatedAt: row[colUpdatedAt],
                fileSize: row[colFileSize]
            )
        }
    }

    // MARK: - Initializer

    public init(db: Connection) throws {
        let colId = SQLite.Expression<Int64>("id")
        let colSourceId = SQLite.Expression<Int64>("sourceId")
        let colPathId = SQLite.Expression<Int64>("pathId")
        let colParentPathId = SQLite.Expression<Int64?>("parentPathId")
        let colName = SQLite.Expression<String>("name")
        let colRelativePath = SQLite.Expression<String>("relativePath")
        let colIsDirectory = SQLite.Expression<Bool>("isDirectory")
        let colFileSize = SQLite.Expression<Int64?>("fileSize")
        let colFileHashSHA256 = SQLite.Expression<Data?>("fileHashSHA256")
        let colRunId = SQLite.Expression<Int64>("runId")
        let colCreatedAt = SQLite.Expression<Date>("createdAt")
        let colUpdatedAt = SQLite.Expression<Date?>("updatedAt")

        self.db = db

        try db.run(
            table.create(ifNotExists: true) { t in
                t.column(colId, primaryKey: .autoincrement)
                t.column(colSourceId)
                t.column(colPathId)
                t.column(colParentPathId)
                t.column(colName)
                t.column(colRelativePath)
                t.column(colIsDirectory)
                t.column(colFileSize)
                t.column(colFileHashSHA256)
                t.column(colRunId)
                t.column(colCreatedAt)
                t.column(colUpdatedAt)
            }
        )
        let columns = try db.prepare("PRAGMA table_info(source_paths)").compactMap { $0[1] as? String }
        if !columns.contains("fileSize") { try db.run("ALTER TABLE source_paths ADD COLUMN fileSize INTEGER") }
        logger.debug("Created table: source_paths")

        self.colId = colId
        self.colSourceId = colSourceId
        self.colPathId = colPathId
        self.colParentPathId = colParentPathId
        self.colName = colName
        self.colRelativePath = colRelativePath
        self.colIsDirectory = colIsDirectory
        self.colFileSize = colFileSize
        self.colFileHashSHA256 = colFileHashSHA256
        self.colRunId = colRunId
        self.colCreatedAt = colCreatedAt
        self.colUpdatedAt = colUpdatedAt
    }

    public func deleteMany(sourceId: Int64, excludingRunId: Int64) async throws -> Int {
        let query = table.filter(colSourceId == sourceId && colRunId != excludingRunId)
        let count = try db.run(query.delete())
        logger.debug(
            "Deleted \(count) source paths for sourceId: \(sourceId) excluding runId: \(excludingRunId)"
        )
        return count
    }

    // MARK: - Create

    public func create(path: SourcePath) async throws -> SourcePath {
        let insert = table.insert(
            colSourceId <- path.sourceId,
            colPathId <- path.pathId,
            colParentPathId <- path.parentPathId,
            colName <- path.name,
            colRelativePath <- path.relativePath,
            colIsDirectory <- path.isDirectory,
            colFileSize <- path.fileSize,
            colFileHashSHA256 <- path.fileHashSHA256,
            colRunId <- path.runId,
            colCreatedAt <- path.createdAt,
            colUpdatedAt <- path.updatedAt
        )
        let rowId = try db.run(insert)
        logger.debug("Inserted source path with ID: \(rowId)")
        return path.copyWith(id: rowId)
    }

    // MARK: - Update File Hash

    public func updateFileHash(pathId: Int64, fileHash: Data?) async throws {
        let query = table.filter(colPathId == pathId)
        try db.run(query.update(colFileHashSHA256 <- fileHash))
        logger.debug("Updated file hash for path ID: \(pathId)")
    }

    // MARK: - Delete Many

    public func deleteMany(sourceId: Int64) async throws {
        let query = table.filter(colSourceId == sourceId)
        let count = try db.run(query.delete())
        logger.debug("Deleted \(count) source paths for source ID: \(sourceId)")
    }

    // MARK: - Get By Parent ID

    public func getByParentId(parentId: Int64) async throws -> [SourcePath] {
        try db.prepare(table.filter(colParentPathId == parentId)).map { row in
            SourcePath(
                id: row[colId],
                sourceId: row[colSourceId],
                pathId: row[colPathId],
                parentPathId: row[colParentPathId],
                name: row[colName],
                relativePath: row[colRelativePath],
                isDirectory: row[colIsDirectory],
                fileHashSHA256: row[colFileHashSHA256],
                runId: row[colRunId],
                createdAt: row[colCreatedAt],
                updatedAt: row[colUpdatedAt],
                fileSize: row[colFileSize]
            )
        }
    }

    // MARK: - Get By Path

    public func getByPath(relativePath: String, sourceId: Int64) async throws -> SourcePath? {
        let query = table.filter(colRelativePath == relativePath && colSourceId == sourceId)
        if let row = try db.pluck(query) {
            return SourcePath(
                id: row[colId],
                sourceId: row[colSourceId],
                pathId: row[colPathId],
                parentPathId: row[colParentPathId],
                name: row[colName],
                relativePath: row[colRelativePath],
                isDirectory: row[colIsDirectory],
                fileHashSHA256: row[colFileHashSHA256],
                runId: row[colRunId],
                createdAt: row[colCreatedAt],
                updatedAt: row[colUpdatedAt],
                fileSize: row[colFileSize]
            )
        }
        return nil
    }

    public func batchUpsert(paths: [SourcePath]) async throws {
        if paths.count == 0 {
            return
        }
        try db.transaction {
            try requireExistingSources(Set(paths.map(\.sourceId)), in: db)
            for path in paths {
                let query = table.filter(colSourceId == path.sourceId && colRelativePath == path.relativePath)
                if let existing = try db.pluck(query) {
                    // Update existing record
                    try db.run(
                        query.update(
                            colPathId <- path.pathId,
                            colParentPathId <- path.parentPathId,
                            colName <- path.name,
                            colRelativePath <- path.relativePath,
                            colIsDirectory <- path.isDirectory,
                            colFileSize <- path.fileSize ?? existing[colFileSize],
                            colFileHashSHA256 <- path.fileHashSHA256 ?? existing[colFileHashSHA256],
                            colRunId <- path.runId,
                            colUpdatedAt <- path.updatedAt
                        ))
                    logger.debug(
                        "Updated source path with sourceId: \(path.sourceId), pathId: \(path.pathId)"
                    )
                } else {
                    // Insert new record
                    try db.run(
                        table.insert(
                            colSourceId <- path.sourceId,
                            colPathId <- path.pathId,
                            colParentPathId <- path.parentPathId,
                            colName <- path.name,
                            colRelativePath <- path.relativePath,
                            colIsDirectory <- path.isDirectory,
                            colFileSize <- path.fileSize,
                            colFileHashSHA256 <- path.fileHashSHA256,
                            colRunId <- path.runId,
                            colCreatedAt <- path.createdAt,
                            colUpdatedAt <- path.updatedAt
                        ))
                    logger.debug(
                        "Inserted new source path with sourceId: \(path.sourceId), pathId: \(path.pathId)"
                    )
                }
            }
        }
    }
}
