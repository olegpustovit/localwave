import Foundation
import LocalWaveDomain
import LocalWaveCore
import os
import SQLite

public actor SQLitePlaylistSongRepository: PlaylistSongRepository {
    private let db: Connection
    private let table = Table("playlist_songs")
    private let colId: SQLite.Expression<Int64>
    private let colPlaylistId: SQLite.Expression<Int64>
    private let colSongId: SQLite.Expression<Int64>
    private let colPosition: SQLite.Expression<Int>
    private let logger = createLogger(subsystem: subsystem, category: "SQLitePlaylistSongRepository")

    public init(db: Connection) throws {
        self.db = db
        // Column definitions
        let colId = SQLite.Expression<Int64>("id")
        let colPlaylistId = SQLite.Expression<Int64>("playlistId")
        let colSongId = SQLite.Expression<Int64>("songId")
        let colPosition = SQLite.Expression<Int>("position")

        try db.run(
            table.create(ifNotExists: true) { t in
                t.column(colId, primaryKey: .autoincrement)
                t.column(colPlaylistId)
                t.column(colSongId)
                t.column(colPosition)
                t.unique(colPlaylistId, colSongId) // Prevent duplicates
            }
        )

        self.colId = colId
        self.colPlaylistId = colPlaylistId
        self.colSongId = colSongId
        self.colPosition = colPosition
    }

    public func addSong(playlistId: Int64, songId: Int64) async throws {
        // Get current max position
        let maxPosition =
            try db.scalar(
                table.filter(colPlaylistId == playlistId)
                    .select(colPosition.max)
            ) ?? -1

        let insert = table.insert(
            colPlaylistId <- playlistId,
            colSongId <- songId,
            colPosition <- maxPosition + 1
        )
        try db.run(insert)
    }

    public func removeSong(playlistId: Int64, songId: Int64) async throws {
        let query = table.filter(colPlaylistId == playlistId && colSongId == songId)
        try db.run(query.delete())
    }

    public func getSongs(playlistId: Int64) async throws -> [Song] {
        let songsTable = Table("songs")
        let songIdCol = SQLite.Expression<Int64>("id")

        return try db.prepare(
            table
                .join(songsTable, on: songsTable[songIdCol] == table[colSongId])
                .filter(colPlaylistId == playlistId)
                .order(colPosition.asc)
        ).map { row in
            Song(
                id: row[songsTable[songIdCol]],
                songKey: row[songsTable[Expression<Int64>("songKey")]],
                artist: row[songsTable[Expression<String>("artist")]],
                title: row[songsTable[Expression<String>("title")]],
                album: row[songsTable[Expression<String>("album")]],
                albumArtist: row[songsTable[Expression<String>("albumArtist")]],
                releaseYear: row[songsTable[Expression<Int?>("releaseYear")]],
                discNumber: row[songsTable[Expression<Int?>("discNumber")]],
                trackNumber: row[songsTable[Expression<Int?>("trackNumber")]],
                coverArtPath: row[songsTable[Expression<String?>("coverArtPath")]],
                bookmark: (row[songsTable[SQLite.Expression<Blob?>("bookmark")]]?.bytes).map {
                    Data($0)
                },

                pathHash: row[songsTable[Expression<Int64>("pathHash")]],
                createdAt: Date(
                    timeIntervalSince1970: row[songsTable[Expression<Double>("createdAt")]]),
                updatedAt: row[songsTable[Expression<Double?>("updatedAt")]].map(
                    Date.init(timeIntervalSince1970:)),
                localFilePath: row[songsTable[Expression<String?>("localFilePath")]],
                fileState: FileState(rawValue: row[songsTable[Expression<Int>("fileState")]])
                    ?? .bookmarkOnly
            )
        }
    }

    public func reorderSongs(playlistId: Int64, newOrder: [Int64]) async throws {
        try db.transaction {
            // Clear existing positions
            try db.run(table.filter(colPlaylistId == playlistId).update(colPosition <- -1))

            // Update with new positions
            for (index, songId) in newOrder.enumerated() {
                let query = table.filter(colPlaylistId == playlistId && colSongId == songId)
                try db.run(query.update(colPosition <- index))
            }

            // Cleanup any invalid entries (shouldn't be necessary)
            try db.run(table.filter(colPlaylistId == playlistId && colPosition == -1).delete())
        }
    }
}
