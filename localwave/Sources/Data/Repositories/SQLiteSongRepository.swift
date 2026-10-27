import Foundation
import SQLite
import LocalWaveDomain
import LocalWaveCore

public actor SQLiteSongRepository: SongRepository {
    public func updateBookmark(songId: Int64, bookmark: Data) async throws {
        let query = songsTable.filter(colId == songId)
        try db.run(query.update(colBookmark <- Blob(bytes: [UInt8](bookmark))))
    }

    private let db: Connection

    // MARK: - Main "songs" table

    private let songsTable = Table("songs")

    // Typed columns
    private let colId: SQLite.Expression<Int64>
    private let colSongKey: SQLite.Expression<Int64>
    private let colArtist: SQLite.Expression<String>
    private let colTitle: SQLite.Expression<String>
    private let colAlbum: SQLite.Expression<String>
    private let colAlbumArtist: SQLite.Expression<String>
    private let colReleaseYear: SQLite.Expression<Int?>
    private let colDiscNumber: SQLite.Expression<Int?>
    private let colTrackNumber: SQLite.Expression<Int?>
    private let colCoverArtPath: SQLite.Expression<String?>
    private let colBookmark: SQLite.Expression<Blob?>
    private let colPathHash: SQLite.Expression<Int64>
    private let colCreatedAt: SQLite.Expression<Double>
    private let colUpdatedAt: SQLite.Expression<Double?>
    // NEW: new fields for localFilePath and fileState
    private let colLocalFilePath: SQLite.Expression<String?> // NEW
    private let colFileState: SQLite.Expression<Int> // NEW

    // MARK: - FTS table

    private let ftsSongsTable = Table("songs_fts")
    private let colFtsSongId = SQLite.Expression<Int64>("songId")
    private let colFtsArtist = SQLite.Expression<String>("artist")
    private let colFtsTitle = SQLite.Expression<String>("title")
    private let colFtsAlbum = SQLite.Expression<String>("album")
    private let colFtsAlbumArtist = SQLite.Expression<String>("albumArtist")

    // MARK: - Init

    public init(db: Connection) throws {
        self.db = db

        // Initialize typed column expressions
        let colId = SQLite.Expression<Int64>("id")
        let colSongKey = SQLite.Expression<Int64>("songKey")
        let colArtist = SQLite.Expression<String>("artist")
        let colTitle = SQLite.Expression<String>("title")
        let colAlbum = SQLite.Expression<String>("album")
        let colAlbumArtist = SQLite.Expression<String>("albumArtist")
        let colReleaseYear = SQLite.Expression<Int?>("releaseYear")
        let colDiscNumber = SQLite.Expression<Int?>("discNumber")
        let colTrackNumber = SQLite.Expression<Int?>("trackNumber")
        let colCoverArtPath = SQLite.Expression<String?>("coverArtPath")
        let colBookmark = SQLite.Expression<Blob?>("bookmark")
        let colPathHash = SQLite.Expression<Int64>("pathHash")
        let colCreatedAt = SQLite.Expression<Double>("createdAt")
        let colUpdatedAt = SQLite.Expression<Double?>("updatedAt")
        // NEW: new expressions
        let colLocalFilePath = SQLite.Expression<String?>("localFilePath") // NEW
        let colFileState = SQLite.Expression<Int>("fileState") // NEW

        self.colId = colId
        self.colSongKey = colSongKey
        self.colArtist = colArtist
        self.colTitle = colTitle
        self.colAlbum = colAlbum
        self.colAlbumArtist = colAlbumArtist
        self.colReleaseYear = colReleaseYear
        self.colDiscNumber = colDiscNumber
        self.colTrackNumber = colTrackNumber
        self.colCoverArtPath = colCoverArtPath
        self.colBookmark = colBookmark
        self.colPathHash = colPathHash
        self.colCreatedAt = colCreatedAt
        self.colUpdatedAt = colUpdatedAt
        // NEW: assign new columns
        self.colLocalFilePath = colLocalFilePath // NEW
        self.colFileState = colFileState // NEW

        // Create main table if needed
        try db.run(
            songsTable.create(ifNotExists: true) { t in
                t.column(colId, primaryKey: .autoincrement)
                t.column(colSongKey)
                t.column(colArtist)
                t.column(colTitle)
                t.column(colAlbum)
                t.column(colAlbumArtist)
                t.column(colReleaseYear)
                t.column(colDiscNumber)
                t.column(colTrackNumber)
                t.column(colCoverArtPath)
                t.column(colBookmark)
                t.column(colPathHash)
                t.column(colCreatedAt)
                t.column(colUpdatedAt)
                // NEW: add new columns
                t.column(colLocalFilePath) // NEW
                t.column(colFileState) // NEW
            }
        )

        // Create FTS table with albumArtist column included
        try db.execute(
            """
            CREATE VIRTUAL TABLE IF NOT EXISTS songs_fts
            USING fts5(
                songId UNINDEXED,
                artist,
                title,
                album,
                albumArtist,  -- NEW
                tokenize='unicode61'
            );
            """
        )
    }

    public func getSongs(ids: [Int64]) async -> [Song] {
        let query = songsTable.filter(ids.contains(colId))
        let songs = try! db.prepare(query).map { row in
            Song(
                id: row[colId],
                songKey: row[colSongKey],
                artist: row[colArtist],
                title: row[colTitle],
                album: row[colAlbum],
                albumArtist: row[colAlbumArtist],
                releaseYear: row[colReleaseYear],
                discNumber: row[colDiscNumber],
                trackNumber: row[colTrackNumber],
                coverArtPath: row[colCoverArtPath],
                bookmark: (row[colBookmark]?.bytes).map { Data($0) },
                pathHash: row[colPathHash],
                createdAt: Date(timeIntervalSince1970: row[colCreatedAt]),
                updatedAt: row[colUpdatedAt].map(Date.init(timeIntervalSince1970:)),
                // NEW: add new fields
                localFilePath: row[colLocalFilePath], // NEW
                fileState: FileState(rawValue: row[colFileState]) ?? .bookmarkOnly // NEW
            )
        }
        
        // Create a dictionary for O(1) lookup
        let songDict: [Int64: Song] = Dictionary(uniqueKeysWithValues: songs.compactMap { song in
            guard let id = song.id else { return nil }
            return (id, song)
        })
        
        // Return songs in the same order as the input IDs
        return ids.compactMap { songDict[$0] }
    }

    public func getSongByURL(_ url: URL) async -> Song? {
        let pathHash = makeURLHash(url)
        let query = songsTable.filter(colPathHash == pathHash)
        return try? db.pluck(query).map { row in
            Song(
                id: row[colId],
                songKey: row[colSongKey],
                artist: row[colArtist],
                title: row[colTitle],
                album: row[colAlbum],
                albumArtist: row[colAlbumArtist],
                releaseYear: row[colReleaseYear],
                discNumber: row[colDiscNumber],
                trackNumber: row[colTrackNumber],
                coverArtPath: row[colCoverArtPath],
                bookmark: (row[colBookmark]?.bytes).map { Data($0) },
                pathHash: row[colPathHash],
                createdAt: Date(timeIntervalSince1970: row[colCreatedAt]),
                updatedAt: row[colUpdatedAt].map(Date.init(timeIntervalSince1970:)),
                // NEW: add new fields
                localFilePath: row[colLocalFilePath], // NEW
                fileState: FileState(rawValue: row[colFileState]) ?? .bookmarkOnly // NEW
            )
        }
    }

    public func totalSongCount(query: String) async throws -> Int {
        if query.isEmpty {
            let countQuery = "SELECT COUNT(*) FROM songs;"
            var count = 0
            for row in try db.prepare(countQuery) {
                if let c = row[0] as? Int64 {
                    count = Int(c)
                    break
                }
            }
            return count
        } else {
            let processedQuery = preprocessFTSQuery(query)
            let countQuery = """
            SELECT COUNT(*) FROM songs s
            JOIN songs_fts fts ON s.id = fts.songId
            WHERE songs_fts MATCH ?;
            """
            var count = 0
            for row in try db.prepare(countQuery, processedQuery) {
                if let c = row[0] as? Int64 {
                    count = Int(c)
                    break
                }
            }
            return count
        }
    }

    // MARK: - Upsert

    public func upsertSong(_ song: Song) async throws -> Song {
        let existingRow = try db.pluck(songsTable.filter(colSongKey == song.songKey))
        let now = Date().timeIntervalSince1970
        if let row = existingRow {
            let songId = row[colId]
            try db.run(
                songsTable
                    .filter(colId == songId)
                    .update(
                        colArtist <- song.artist,
                        colTitle <- song.title,
                        colAlbum <- song.album,
                        colAlbumArtist <- song.albumArtist,
                        colReleaseYear <- song.releaseYear,
                        colDiscNumber <- song.discNumber,
                        colTrackNumber <- song.trackNumber,
                        colCoverArtPath <- song.coverArtPath,
                        colBookmark <- song.bookmark.map { data in Blob(bytes: [UInt8](data)) },
                        colPathHash <- song.pathHash,
                        colUpdatedAt <- now,
                        // NEW: update new fields
                        colLocalFilePath <- song.localFilePath, // NEW
                        colFileState <- song.fileState.rawValue // NEW
                    )
            )
            try db.run(
                ftsSongsTable
                    .filter(colFtsSongId == songId)
                    .update(
                        colFtsArtist <- song.artist,
                        colFtsTitle <- song.title,
                        colFtsAlbum <- song.album,
                        colFtsAlbumArtist <- song.albumArtist
                    )
            )
            return song.copyWith(id: songId)
        } else {
            let rowId = try db.run(
                songsTable.insert(
                    colSongKey <- song.songKey,
                    colArtist <- song.artist,
                    colTitle <- song.title,
                    colAlbum <- song.album,
                    colAlbumArtist <- song.albumArtist,
                    colReleaseYear <- song.releaseYear,
                    colDiscNumber <- song.discNumber,
                    colTrackNumber <- song.trackNumber,
                    colCoverArtPath <- song.coverArtPath,
                    colBookmark <- song.bookmark.map { Blob(bytes: [UInt8]($0)) },
                    colPathHash <- song.pathHash,
                    colCreatedAt <- song.createdAt.timeIntervalSince1970,
                    colUpdatedAt <- song.updatedAt?.timeIntervalSince1970,
                    // NEW: insert new fields
                    colLocalFilePath <- song.localFilePath, // NEW
                    colFileState <- song.fileState.rawValue // NEW
                )
            )
            try db.run(
                ftsSongsTable.insert(
                    colFtsSongId <- rowId,
                    colFtsArtist <- song.artist,
                    colFtsTitle <- song.title,
                    colFtsAlbum <- song.album,
                    colFtsAlbumArtist <- song.albumArtist
                )
            )
            return song.copyWith(id: rowId)
        }
    }

    public func deleteSong(songId: Int64) async throws {
        let query = songsTable.filter(colId == songId)
        try db.run(query.delete())
        try db.run(ftsSongsTable.filter(colFtsSongId == songId).delete())
    }

    public func deleteAlbum(album: String, artist: String?) async throws {
        var query = songsTable.filter(colAlbum == album)
        if let artist = artist {
            query = query.filter(colArtist == artist)
        }
        // Retrieve song IDs for FTS cleanup
        let songIds = try db.prepare(query).map { row in row[colId] }
        try db.run(query.delete())
        for songId in songIds {
            try db.run(ftsSongsTable.filter(colFtsSongId == songId).delete())
        }
    }

    // MARK: - FTS Searching

    public func searchSongsFTS(query: String, limit: Int, offset: Int) async throws -> [Song] {
        var results = [Song]()
        let statement: Statement
        let sql: String
        let bindings: [Binding?]

        if query.isEmpty {
            sql = """
            SELECT id, songKey, artist, title, album, trackNumber, coverArtPath, bookmark, pathHash, createdAt, updatedAt, localFilePath, fileState
              FROM songs
             ORDER BY createdAt DESC
             LIMIT ? OFFSET ?;
            """ // NEW: added localFilePath and fileState
            bindings = [limit, offset]
        } else {
            let processedQuery = preprocessFTSQuery(query)
            sql = """
            SELECT s.id, s.songKey, s.artist, s.title, s.album, s.trackNumber,
                   s.coverArtPath, s.bookmark, s.pathHash, s.createdAt, s.updatedAt, s.localFilePath, s.fileState
              FROM songs s
              JOIN songs_fts fts ON s.id = fts.songId
             WHERE songs_fts MATCH ?
             ORDER BY bm25(songs_fts)
             LIMIT ? OFFSET ?;
            """ // NEW: added localFilePath and fileState
            bindings = [processedQuery, limit, offset]
        }

        statement = try db.prepare(sql, bindings)
        for row in statement {
            guard let id = row[0] as? Int64 else { continue }
            let songKey = (row[1] as? Int64) ?? 0
            let artist = (row[2] as? String) ?? ""
            let title = (row[3] as? String) ?? ""
            let album = (row[4] as? String) ?? ""
            var trackNumber: Int? = nil
            if let t = (row[5] as? Int64) { trackNumber = Int(t) }
            let coverArtPath = row[6] as? String
            let bookmarkBlob = row[7] as? Blob
            let bookmarkData = bookmarkBlob.map { Data($0.bytes) }
            let pathHash = row[8] as? Int64 ?? -1
            let createdDouble = row[9] as? Double ?? 0
            let createdAt = Date(timeIntervalSince1970: createdDouble)
            let updatedDouble = row[10] as? Double
            let updatedAt = updatedDouble.map { Date(timeIntervalSince1970: $0) }
            // NEW: get new fields
            let localFilePath = row[11] as? String // NEW
            let fileStateRaw = row[12] as? Int ?? FileState.bookmarkOnly.rawValue // NEW
            let fileState = FileState(rawValue: fileStateRaw) ?? .bookmarkOnly // NEW

            // NOTE: FTS search doesn't return albumArtist, releaseYear, or discNumber.
            let song = Song(
                id: id,
                songKey: songKey,
                artist: artist,
                title: title,
                album: album,
                albumArtist: "", // Not available from FTS result.
                releaseYear: nil, // Not available from FTS result.
                discNumber: nil, // Not available from FTS result.
                trackNumber: trackNumber,
                coverArtPath: coverArtPath,
                bookmark: bookmarkData,
                pathHash: pathHash,
                createdAt: createdAt,
                updatedAt: updatedAt,
                // NEW: new fields
                localFilePath: localFilePath, // NEW
                fileState: fileState // NEW
            )
            results.append(song)
        }
        return results
    }

    public func getAllArtists() async throws -> [String] {
        let query = songsTable.select(colArtist)
            .filter(colArtist != "")
            .group(colArtist)
        return try db.prepare(query).compactMap { $0[colArtist] }
    }

    public func getAllAlbums() async throws -> [Album] {
        let query = """
            SELECT album, artist, coverArtPath
            FROM songs AS s1
            WHERE album != ''
              AND LENGTH(artist) = (
                SELECT MIN(LENGTH(artist))
                FROM songs AS s2
                WHERE s2.album = s1.album
            )
            GROUP BY album
            ORDER BY album;
        """
        var albums = [Album]()
        for row in try db.prepare(query) {
            let albumName = row[0] as? String ?? ""
            let artist = row[1] as? String ?? ""
            let coverPath = row[2] as? String
            albums.append(Album(name: albumName, artist: artist, coverArtPath: coverPath))
        }
        return albums
    }

    public func getSongsByArtist(_ artist: String) async throws -> [Song] {
        let query = songsTable.filter(colArtist == artist)
        return try parseSongsFromRows(db.prepare(query))
    }

    public func getSongsByAlbum(_ album: String, artist: String?) async throws -> [Song] {
        var query = songsTable.filter(colAlbum == album)
        if let artist = artist { query = query.filter(colArtist == artist) }
        return try parseSongsFromRows(db.prepare(query))
    }

    private func parseSongsFromRows(_ rows: AnySequence<Row>) throws -> [Song] {
        return rows.map { row in
            let bookmarkBlob = row[colBookmark]
            let bookmarkData = bookmarkBlob.map { Data($0.bytes) }
            return Song(
                id: row[colId],
                songKey: row[colSongKey],
                artist: row[colArtist],
                title: row[colTitle],
                album: row[colAlbum],
                albumArtist: row[colAlbumArtist],
                releaseYear: row[colReleaseYear],
                discNumber: row[colDiscNumber],
                trackNumber: row[colTrackNumber],
                coverArtPath: row[colCoverArtPath],
                bookmark: bookmarkData,
                pathHash: row[colPathHash],
                createdAt: Date(timeIntervalSince1970: row[colCreatedAt]),
                updatedAt: row[colUpdatedAt].map(Date.init(timeIntervalSince1970:)),
                // NEW: add new fields
                localFilePath: row[colLocalFilePath], // NEW
                fileState: FileState(rawValue: row[colFileState]) ?? .bookmarkOnly // NEW
            )
        }
    }

    // NEW: getSongsNeedingCopy - returns songs with fileState of bookmarkOnly or failed  // NEW
    public func getSongsNeedingCopy() async -> [Song] {
        let query = songsTable.filter(
            colFileState == FileState.bookmarkOnly.rawValue
                || colFileState == FileState.failed.rawValue) // NEW
        return try! db.prepare(query).map { row in // NEW
            Song(
                id: row[colId],
                songKey: row[colSongKey],
                artist: row[colArtist],
                title: row[colTitle],
                album: row[colAlbum],
                albumArtist: row[colAlbumArtist],
                releaseYear: row[colReleaseYear],
                discNumber: row[colDiscNumber],
                trackNumber: row[colTrackNumber],
                coverArtPath: row[colCoverArtPath],
                bookmark: (row[colBookmark]?.bytes).map { Data($0) },
                pathHash: row[colPathHash],
                createdAt: Date(timeIntervalSince1970: row[colCreatedAt]),
                updatedAt: row[colUpdatedAt].map(Date.init(timeIntervalSince1970:)),
                localFilePath: row[colLocalFilePath],
                fileState: FileState(rawValue: row[colFileState]) ?? .bookmarkOnly
            )
        }
    }

    // NEW: markSongForCopy - update the song's fileState to copyPending  // NEW
    public func markSongForCopy(songId: Int64) async throws {
        try db.run(
            songsTable.filter(colId == songId).update(
                colFileState <- FileState.copyPending.rawValue)) // NEW
    }
}
