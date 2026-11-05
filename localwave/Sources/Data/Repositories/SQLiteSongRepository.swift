import Foundation
import SQLite
import LocalWaveDomain
import LocalWaveCore

public actor SQLiteSongRepository: SongRepository {
    public func updateBookmark(songId: Int64, bookmark: Data) async throws {

        let query = songsTable.filter(colId == songId)
        try db.run(query.update(colBookmark <- Blob(bytes: [UInt8](bookmark))))
    }

    public func updateSourceAvailability(songId: Int64, availability: FileAvailability) async throws {
        try db.run(songsTable.filter(colId == songId).update(colSourceAvailability <- availability.rawValue))
    }

    public func updateSourceLocation(songId: Int64, sourceId: Int64, relativePath: String) async throws {
        let unlinked = colSourceId == nil || (colSourceId == sourceId && colRelativePath == nil)
        try db.run(songsTable.filter(colId == songId && unlinked).update(colSourceId <- sourceId, colRelativePath <- relativePath))
    }

    private let db: Connection

    // MARK: - Main "songs" table

    private let songsTable = Table("songs")

    // Typed columns
    private let colSourceAvailability = SQLite.Expression<String?>("sourceAvailability")
    private let colDuration = SQLite.Expression<Double?>("duration")
    private let colFileSize = SQLite.Expression<Int64?>("fileSize")
    private let colFileExtension = SQLite.Expression<String?>("fileExtension")
    private let colSampleRate = SQLite.Expression<Double?>("sampleRate")
    private let colBitDepth = SQLite.Expression<Int?>("bitDepth")
    private let colSourceId = SQLite.Expression<Int64?>("sourceId")
    private let colRelativePath = SQLite.Expression<String?>("relativePath")
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
    
    // NEWER: new field for marking song as a favorite
    private let colIsFavorite: SQLite.Expression<Int64>

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
        
        // NEWER: newer expressions
        let colIsFavorite = SQLite.Expression<Int64>("isFavorite") // NEWER
        
        
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
        self.colIsFavorite = colIsFavorite // NEWER

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
                t.column(colIsFavorite) // NEWER
                t.column(colSourceId)
                t.column(colRelativePath)
                t.column(colSourceAvailability)
                t.column(colDuration)
                t.column(colFileSize)
                t.column(colFileExtension)
                t.column(colSampleRate)
                t.column(colBitDepth)
            }
        )

        // CREATE TABLE does not add columns to an existing installation.
        // SQLite has no ADD COLUMN IF NOT EXISTS; inspect first inside a transaction.
        try db.transaction {
            let columns = try Set(db.prepare("PRAGMA table_info(songs)").compactMap { $0[1] as? String })
            for (name, definition) in [
                ("localFilePath", "TEXT"),
                ("fileState", "INTEGER NOT NULL DEFAULT 0"),
                ("isFavorite", "INTEGER NOT NULL DEFAULT 0"),
                ("sourceId", "INTEGER"),
                ("relativePath", "TEXT"),
                ("duration", "REAL"),
                ("fileSize", "INTEGER"),
                ("fileExtension", "TEXT"),
                ("sampleRate", "REAL"),
                ("bitDepth", "INTEGER"),
                ("sourceAvailability", "TEXT")
            ] where !columns.contains(name) {
                try db.execute("ALTER TABLE songs ADD COLUMN \(name) \(definition)")
            }
        }

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
                fileState: FileState(rawValue: row[colFileState]) ?? .bookmarkOnly, // NEW
                isFavorite: row[colIsFavorite],
                sourceId: row[colSourceId],
                relativePath: row[colRelativePath],
                duration: row[colDuration],
                fileSize: row[colFileSize],
                fileExtension: row[colFileExtension],
                sampleRate: row[colSampleRate],
                bitDepth: row[colBitDepth],
                sourceAvailability: row[colSourceAvailability].flatMap(FileAvailability.init(rawValue:)) // NEWER
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
                fileState: FileState(rawValue: row[colFileState]) ?? .bookmarkOnly, // NEW
                isFavorite: row[colIsFavorite],
                sourceId: row[colSourceId],
                relativePath: row[colRelativePath],
                duration: row[colDuration],
                fileSize: row[colFileSize],
                fileExtension: row[colFileExtension],
                sampleRate: row[colSampleRate],
                bitDepth: row[colBitDepth],
                sourceAvailability: row[colSourceAvailability].flatMap(FileAvailability.init(rawValue:)) // NEWER
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
        var persisted = song
        try db.transaction {
            if let sourceId = song.sourceId { try requireExistingSources([sourceId], in: db) }
            persisted = try writeSong(song)
            if let id = persisted.id {
                persisted = try parseSongsFromRows(db.prepare(songsTable.filter(colId == id))).first ?? persisted
            }
        }
        return persisted
    }

    private func writeSong(_ song: Song) throws -> Song {
        let sourceRow: Row?
        if let sourceId = song.sourceId, let relativePath = song.relativePath {
            sourceRow = try db.pluck(songsTable.filter(colSourceId == sourceId && colRelativePath == relativePath))
        } else { sourceRow = nil }
        let existingRow = try sourceRow ?? db.pluck(songsTable.filter(colSongKey == song.songKey))
        let now = Date().timeIntervalSince1970
        if let row = existingRow {
            let songId = row[colId]
            try db.run(
                songsTable
                    .filter(colId == songId)
                    .update(
                        colSongKey <- song.songKey,
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
                        colSourceId <- song.sourceId ?? row[colSourceId],
                        colRelativePath <- song.relativePath ?? row[colRelativePath],
                        colDuration <- song.duration ?? row[colDuration],
                        colFileSize <- song.fileSize ?? row[colFileSize],
                        colFileExtension <- song.fileExtension ?? row[colFileExtension],
                        colSampleRate <- song.sampleRate ?? row[colSampleRate],
                        colBitDepth <- song.bitDepth ?? row[colBitDepth],
                        colSourceAvailability <- song.sourceAvailability?.rawValue ?? row[colSourceAvailability],
                        colUpdatedAt <- now,
                        // NEW: update new fields
                        colLocalFilePath <- song.localFilePath, // NEW
                        colFileState <- song.fileState.rawValue // Preserve the user favorite on reimport.
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
                    colDuration <- song.duration,
                    colFileSize <- song.fileSize,
                    colFileExtension <- song.fileExtension,
                    colSampleRate <- song.sampleRate,
                    colBitDepth <- song.bitDepth,
                    colSourceAvailability <- song.sourceAvailability?.rawValue,
                    colCreatedAt <- song.createdAt.timeIntervalSince1970,
                    colSourceId <- song.sourceId,
                    colRelativePath <- song.relativePath,
                    colUpdatedAt <- song.updatedAt?.timeIntervalSince1970,
                    // NEW: insert new fields
                    colLocalFilePath <- song.localFilePath, // NEW
                    colFileState <- song.fileState.rawValue, // NEW
                    colIsFavorite <- song.isFavorite // NEWER
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
        let sql: String
        let bindings: [Binding?]
        if query.isEmpty {
            sql = "SELECT id FROM songs ORDER BY createdAt DESC LIMIT ? OFFSET ?"
            bindings = [limit, offset]
        } else {
            sql = """
                SELECT s.id FROM songs s JOIN songs_fts fts ON s.id = fts.songId
                WHERE songs_fts MATCH ? ORDER BY bm25(songs_fts) LIMIT ? OFFSET ?
                """
            bindings = [preprocessFTSQuery(query), limit, offset]
        }
        let ids = try db.prepare(sql, bindings).compactMap { $0[0] as? Int64 }
        // Hydrate complete rows so search and library views preserve all metadata.
        return await getSongs(ids: ids)
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
                fileState: FileState(rawValue: row[colFileState]) ?? .bookmarkOnly, // NEW
                isFavorite: row[colIsFavorite],
                sourceId: row[colSourceId],
                relativePath: row[colRelativePath],
                duration: row[colDuration],
                fileSize: row[colFileSize],
                fileExtension: row[colFileExtension],
                sampleRate: row[colSampleRate],
                bitDepth: row[colBitDepth],
                sourceAvailability: row[colSourceAvailability].flatMap(FileAvailability.init(rawValue:)) // NEWER
            )
        }
    }

    // NEW: getSongsNeedingCopy - returns songs with fileState of bookmarkOnly or failed  // NEW
    public func getSongsNeedingCopy() async -> [Song] {
        let query = songsTable.filter(
            colFileState == FileState.copyPending.rawValue
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
                fileState: FileState(rawValue: row[colFileState]) ?? .bookmarkOnly,
                isFavorite: row[colIsFavorite],
                sourceId: row[colSourceId],
                relativePath: row[colRelativePath],
                duration: row[colDuration],
                fileSize: row[colFileSize],
                fileExtension: row[colFileExtension],
                sampleRate: row[colSampleRate],
                bitDepth: row[colBitDepth],
                sourceAvailability: row[colSourceAvailability].flatMap(FileAvailability.init(rawValue:))
            )
        }
    }

    // NEW: markSongForCopy - update the song's fileState to copyPending  // NEW
    public func markSongForCopy(songId: Int64) async throws {
        try db.run(
            songsTable.filter(colId == songId).update(
                colFileState <- FileState.copyPending.rawValue)) // NEW
    }

    public func updateCachedFile(songId: Int64, localFilePath: String?, state: FileState, expectedState: FileState, expectedPath: String?) async throws -> Bool {
        let query = songsTable.filter(colId == songId && colFileState == expectedState.rawValue && colLocalFilePath == expectedPath)
        return try db.run(query.update(colLocalFilePath <- localFilePath, colFileState <- state.rawValue)) > 0
    }

    public func getSong(sourceId: Int64, relativePath: String) async -> Song? {
        guard let row = try? db.pluck(songsTable.filter(colSourceId == sourceId && colRelativePath == relativePath)) else { return nil }
        return await getSongs(ids: [row[colId]]).first
    }
    
    // NEWER: toggleSongAsFavorite - for applying Favorites filter // NEWER
    public func toggleSongAsFavorite(songId: Int64, state: Bool) async throws {
        if (state) {
            try db.run(
                songsTable.filter(colId == songId).update(
                    colIsFavorite <- 1
                ))
        }
        else {
            try db.run(
                songsTable.filter(colId == songId).update(
                    colIsFavorite <- 0
                ))
        }
    }
}
