import Foundation
import LocalWaveDomain
import LocalWaveCore
import os
import SQLite

func requireExistingSources(_ ids: Set<Int64>, in db: Connection) throws {
    guard try db.scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sources'") as? Int64 == 1 else { return }
    for id in ids {
        guard try db.pluck(Table("sources").filter(Expression<Int64>("id") == id)) != nil else {
            throw CustomError.genericError("This music source was removed.")
        }
    }
}

public actor SQLiteSourceRepository: SourceRepository {
    private let db: Connection
    private let table = Table("sources")

    // Add type column
    private let colBookmark = SQLite.Expression<Blob?>("bookmark")
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
        let colBookmark = SQLite.Expression<Blob?>("bookmark")
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
                t.column(colBookmark)
                t.column(colTotalPaths)
                t.column(colSyncError)
                t.column(colIsCurrent)
                t.column(colCreatedAt)
                t.column(colLastSyncedAt)
                t.column(colUpdatedAt)
            })

        try db.transaction {
            let columns = try Set(db.prepare("PRAGMA table_info(sources)").compactMap { $0[1] as? String })
            if !columns.contains("bookmark") { try db.execute("ALTER TABLE sources ADD COLUMN bookmark BLOB") }
        }

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
        try db.transaction {
            let tables = try Set(db.prepare("SELECT name FROM sqlite_master WHERE type IN ('table', 'view')").compactMap { $0[0] as? String })
            if tables.contains("songs") {
                let columns = try Set(db.prepare("PRAGMA table_info(songs)").compactMap { $0[1] as? String })
                if columns.contains("sourceId") {
                    if tables.contains("playlist_songs") {
                        try db.run("DELETE FROM playlist_songs WHERE songId IN (SELECT id FROM songs WHERE sourceId = ?)", sourceId)
                    }
                    if tables.contains("songs_fts") {
                        try db.run("DELETE FROM songs_fts WHERE songId IN (SELECT id FROM songs WHERE sourceId = ?)", sourceId)
                    }
                    try db.run("DELETE FROM songs WHERE sourceId = ?", sourceId)
                }
            }
            if tables.contains("source_paths_fts") { try db.run("DELETE FROM source_paths_fts WHERE sourceId = ?", sourceId) }
            if tables.contains("source_paths") { try db.run("DELETE FROM source_paths WHERE sourceId = ?", sourceId) }
            try db.run(table.filter(colId == sourceId).delete())
        }
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
                updatedAt: row[colUpdatedAt],
                bookmark: row[colBookmark].map { Data($0.bytes) }
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
            colBookmark <- source.bookmark.map { Blob(bytes: [UInt8]($0)) },
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
            updatedAt: source.updatedAt,
            bookmark: source.bookmark
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
                updatedAt: row[colUpdatedAt],
                bookmark: row[colBookmark].map { Data($0.bytes) }
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

    public func updateSourceBookmark(sourceId: Int64, bookmark: Data, expectedBookmark: Data?) async throws -> Bool {
        let expected = expectedBookmark.map { Blob(bytes: [UInt8]($0)) }
        let query = table.filter(colId == sourceId && colBookmark == expected)
        return try db.run(query.update(colBookmark <- Blob(bytes: [UInt8](bookmark)))) > 0
    }

    public func reconnectSource(sourceId: Int64, dirPath: String, pathId: Int64, bookmark: Data) async throws -> Source? {
        try db.run(table.filter(colId == sourceId).update(
            colDirPath <- dirPath, colPathId <- pathId, colBookmark <- Blob(bytes: [UInt8](bookmark)),
            colSyncError <- nil, colUpdatedAt <- Date()
        ))
        return try await getOne(id: sourceId)
    }

    public func updateResolvedSourceRoot(sourceId: Int64, dirPath: String, pathId: Int64, bookmark: Data, expectedBookmark: Data?) async throws -> Bool {
        let expected = expectedBookmark.map { Blob(bytes: [UInt8]($0)) }
        return try db.run(table.filter(colId == sourceId && colBookmark == expected).update(
            colDirPath <- dirPath, colPathId <- pathId, colBookmark <- Blob(bytes: [UInt8](bookmark)), colUpdatedAt <- Date()
        )) > 0
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
            updatedAt: row[colUpdatedAt],
                bookmark: row[colBookmark].map { Data($0.bytes) }
        )
    }
}
