import Foundation
import LocalWaveDomain
import LocalWaveCore
import os
import SQLite

public actor SQLiteSourceRepository: SourceRepository {
    private let db: Connection
    private let table = Table("sources")

    // Add type column
    private var colType: SQLite.Expression<String?>
    private var colId: SQLite.Expression<Int64>
    private var colDirPath: SQLite.Expression<String>
    private var colPathId: SQLite.Expression<Int64>
    private var colUserId: SQLite.Expression<Int64>
    private var colTotalPaths: SQLite.Expression<Int?>
    private var colSyncError: SQLite.Expression<String?>
    private var colIsCurrent: SQLite.Expression<Bool>
    private var colCreatedAt: SQLite.Expression<Date>
    private var colLastSyncedAt: SQLite.Expression<Date?>
    private var colUpdatedAt: SQLite.Expression<Date?>

    private let logger = createLogger(subsystem: subsystem, category: "SQLiteSourceRepository")

    public init(db: Connection) throws {
        // Existing columns
        let colId = SQLite.Expression<Int64>("id")
        let colDirPath = SQLite.Expression<String>("dirPath")
        let colPathId = SQLite.Expression<Int64>("pathId")
        let colUserId = SQLite.Expression<Int64>("userId")
        let colTotalPaths = SQLite.Expression<Int?>("totalPaths")
        let colSyncError = SQLite.Expression<String?>("syncError")
        let colIsCurrent = SQLite.Expression<Bool>("isCurrent")
        let colCreatedAt = SQLite.Expression<Date>("createdAt")
        let colLastSyncedAt = SQLite.Expression<Date?>("lastSyncedAt")
        let colUpdatedAt = SQLite.Expression<Date?>("updatedAt")
        let colType = SQLite.Expression<String?>("type")

        self.db = db

        // Create table with new column
        try db.run(
            table.create(ifNotExists: true) { t in
                t.column(colId, primaryKey: .autoincrement)
                t.column(colDirPath)
                t.column(colPathId)
                t.column(colUserId)
                t.column(colType)
                t.column(colTotalPaths)
                t.column(colSyncError)
                t.column(colIsCurrent)
                t.column(colCreatedAt)
                t.column(colLastSyncedAt)
                t.column(colUpdatedAt)
            })

        self.colId = colId
        self.colDirPath = colDirPath
        self.colPathId = colPathId
        self.colUserId = colUserId
        self.colTotalPaths = colTotalPaths
        self.colSyncError = colSyncError
        self.colIsCurrent = colIsCurrent
        self.colCreatedAt = colCreatedAt
        self.colLastSyncedAt = colLastSyncedAt
        self.colUpdatedAt = colUpdatedAt
        self.colType = colType
    }

    public func deleteSource(sourceId: Int64) async throws {
        let query = table.filter(colId == sourceId)
        try db.run(query.delete())
        logger.debug("Deleted source with ID: \(sourceId)")
    }

    public func getOne(id: Int64) async throws -> Source? {
        let query = table.filter(colId == id)
        if let row = try db.pluck(query) {
            return Source(
                id: row[colId],
                dirPath: row[colDirPath],
                pathId: row[colPathId],
                userId: row[colUserId],
                type: row[colType].flatMap(SourceType.init(rawValue:)), // Map from String?
                totalPaths: row[colTotalPaths],
                syncError: row[colSyncError],
                isCurrent: row[colIsCurrent],
                createdAt: row[colCreatedAt],
                lastSyncedAt: row[colLastSyncedAt],
                updatedAt: row[colUpdatedAt]
            )
        }
        return nil
    }

    public func create(source: Source) async throws -> Source {
        // Force explicit type setting (even if nil)
        let insert = table.insert(
            colDirPath <- source.dirPath,
            colPathId <- source.pathId,
            colUserId <- source.userId,
            colType <- source.type?.rawValue, // Explicit null if type is nil
            colTotalPaths <- source.totalPaths,
            colSyncError <- source.syncError,
            colIsCurrent <- source.isCurrent,
            colCreatedAt <- source.createdAt,
            colLastSyncedAt <- source.lastSyncedAt,
            colUpdatedAt <- source.updatedAt
        )

        let rowId = try db.run(insert)
        logger.debug("Inserted source with ID: \(rowId)")

        return Source(
            id: rowId,
            dirPath: source.dirPath,
            pathId: source.pathId,
            userId: source.userId,
            type: source.type, // Preserve original type
            totalPaths: source.totalPaths,
            syncError: source.syncError,
            isCurrent: source.isCurrent,
            createdAt: source.createdAt,
            lastSyncedAt: source.lastSyncedAt,
            updatedAt: source.updatedAt
        )
    }

    public func findOneByUserId(userId: Int64, path: String?) async throws -> [Source] {
        var predicate = colUserId == userId
        if let path = path {
            predicate = predicate && colDirPath == path
        }

        return try db.prepare(table.filter(predicate)).map { row in
            Source(
                id: row[colId],
                dirPath: row[colDirPath],
                pathId: row[colPathId],
                userId: row[colUserId],
                type: row[colType].flatMap(SourceType.init(rawValue:)),
                totalPaths: row[colTotalPaths],
                syncError: row[colSyncError],
                isCurrent: row[colIsCurrent],
                createdAt: row[colCreatedAt],
                lastSyncedAt: row[colLastSyncedAt],
                updatedAt: row[colUpdatedAt]
            )
        }
    }

    public func updateSource(source: Source) async throws -> Source {
        guard let sourceId = source.id else {
            throw NSError(domain: "Invalid source ID", code: 0, userInfo: nil)
        }

        let query = table.filter(colId == sourceId)
        try db.run(
            query.update(
                colDirPath <- source.dirPath,
                colPathId <- source.pathId,
                colType <- source.type?.rawValue,
                colTotalPaths <- source.totalPaths,
                colSyncError <- source.syncError,
                colIsCurrent <- source.isCurrent,
                colLastSyncedAt <- source.lastSyncedAt,
                colUpdatedAt <- source.updatedAt
            ))

        return source
    }

    public func setCurrentSource(userId: Int64, sourceId: Int64) async throws -> Source {
        try db.transaction {
            try db.run(table.filter(colUserId == userId).update(colIsCurrent <- false))
            try db.run(table.filter(colId == sourceId).update(colIsCurrent <- true))
        }

        guard let row = try db.pluck(table.filter(colId == sourceId)) else {
            throw NSError(domain: "Source not found", code: 0, userInfo: nil)
        }

        return Source(
            id: row[colId],
            dirPath: row[colDirPath],
            pathId: row[colPathId],
            userId: row[colUserId],
            type: row[colType].flatMap(SourceType.init(rawValue:)),
            totalPaths: row[colTotalPaths],
            syncError: row[colSyncError],
            isCurrent: row[colIsCurrent],
            createdAt: row[colCreatedAt],
            lastSyncedAt: row[colLastSyncedAt],
            updatedAt: row[colUpdatedAt]
        )
    }
}
