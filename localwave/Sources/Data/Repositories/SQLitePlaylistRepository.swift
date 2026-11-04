import Foundation
import LocalWaveDomain
import LocalWaveCore
import os
import SQLite

public actor SQLitePlaylistRepository: PlaylistRepository {
    private let db: Connection
    private let table = Table("playlists")
    private let colId: SQLite.Expression<Int64>
    private let colName: SQLite.Expression<String>
    private let colCreatedAt: SQLite.Expression<Date>
    private let colUpdatedAt: SQLite.Expression<Date?>
    private let logger = createLogger(subsystem: subsystem, category: "SQLitePlaylistRepository")

    public init(db: Connection) throws {
        self.db = db
        // Column definitions
        let colId = SQLite.Expression<Int64>("id")
        let colName = SQLite.Expression<String>("name")
        let colCreatedAt = SQLite.Expression<Date>("createdAt")
        let colUpdatedAt = SQLite.Expression<Date?>("updatedAt")

        try db.run(
            table.create(ifNotExists: true) { t in
                t.column(colId, primaryKey: .autoincrement)
                t.column(colName)
                t.column(colCreatedAt)
                t.column(colUpdatedAt)
            })

        self.colId = colId
        self.colName = colName
        self.colCreatedAt = colCreatedAt
        self.colUpdatedAt = colUpdatedAt
    }

    public func create(playlist: Playlist) async throws -> Playlist {
        let insert = table.insert(
            colName <- playlist.name,
            colCreatedAt <- playlist.createdAt,
            colUpdatedAt <- playlist.updatedAt
        )
        let rowId = try db.run(insert)
        return Playlist(
            id: rowId, name: playlist.name, createdAt: playlist.createdAt,
            updatedAt: playlist.updatedAt
        )
    }

    public func update(playlist: Playlist) async throws -> Playlist {
        guard let playlistId = playlist.id else {
            throw CustomError.genericError("Cannot update playlist without ID")
        }

        let query = table.filter(colId == playlistId)
        try db.run(
            query.update(
                colName <- playlist.name,
                colUpdatedAt <- Date()
            ))

        return Playlist(
            id: playlistId,
            name: playlist.name,
            createdAt: playlist.createdAt,
            updatedAt: Date()
        )
    }

    public func delete(playlistId: Int64) async throws {
        let query = table.filter(colId == playlistId)
        try db.run(query.delete())
    }

    public func getAll() async throws -> [Playlist] {
        try db.prepare(table).map { row in
            Playlist(
                id: row[colId],
                name: row[colName],
                createdAt: row[colCreatedAt],
                updatedAt: row[colUpdatedAt]
            )
        }
    }

    public func getOne(id: Int64) async throws -> Playlist? {
        let query = table.filter(colId == id)
        return try db.pluck(query).map { row in
            Playlist(
                id: row[colId],
                name: row[colName],
                createdAt: row[colCreatedAt],
                updatedAt: row[colUpdatedAt]
            )
        }
    }
}
