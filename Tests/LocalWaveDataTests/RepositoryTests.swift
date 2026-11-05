import XCTest
import Foundation
import SQLite
import LocalWaveDomain
@testable import LocalWaveData

final class RepositoryTests: XCTestCase {
    func testRepositoryArtistAndAlbumLookupsWorkThroughProtocol() async throws {
        let db = try Connection(.inMemory)
        let repository: any SongRepository = try SQLiteSongRepository(db: db)
        for (key, artist, album) in [(1, "North", "First"), (2, "North", "Second"), (3, "South", "First")] {
            _ = try await repository.upsertSong(Song(id: nil, songKey: Int64(key), artist: artist, title: "Track \(key)", album: album, albumArtist: artist, releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: Int64(key), createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly))
        }
        let north = try await repository.getSongsByArtist("North")
        let southAlbum = try await repository.getSongsByAlbum("First", artist: "South")
        XCTAssertEqual(north.count, 2)
        XCTAssertEqual(southAlbum.map(\.artist), ["South"])
    }

    func testSourceRelativeIdentityAndMeasurementsSurviveEditsAndMetadataChanges() async throws {
        let database = try Connection(.inMemory)
        let repository = try SQLiteSongRepository(db: database)
        let playlists = try SQLitePlaylistRepository(db: database)
        let links = try SQLitePlaylistSongRepository(db: database)
        let imported = Song(id: nil, songKey: 123, artist: "Artist", title: "Title", album: "Album", albumArtist: "Album Artist", releaseYear: 2026, discNumber: 2, trackNumber: 3, coverArtPath: nil, bookmark: nil, pathHash: 123, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly, sourceId: 7, relativePath: "Album/track.flac", duration: 90.5, fileSize: 1000, fileExtension: "flac", sampleRate: 48000, bitDepth: 24)
        let first = try await repository.upsertSong(imported)
        _ = try await repository.upsertSong(makeSong())
        let editedRows = await repository.getSongs(ids: [try XCTUnwrap(first.id)])
        let edited = try XCTUnwrap(editedRows.first)
        XCTAssertEqual(edited.sourceId, 7)
        XCTAssertEqual(edited.relativePath, "Album/track.flac")
        XCTAssertEqual(edited.duration, 90.5)
        XCTAssertEqual(edited.fileSize, 1000)
        XCTAssertEqual(edited.fileExtension, "flac")
        XCTAssertEqual(edited.sampleRate, 48000)
        XCTAssertEqual(edited.bitDepth, 24)
        XCTAssertEqual(edited.copyWith("Music/cache.flac", .copied).duration, 90.5)
        let firstID = try XCTUnwrap(first.id)
        try await repository.toggleSongAsFavorite(songId: firstID, state: true)
        let list = try await playlists.create(playlist: Playlist(id: nil, name: "Keeps membership", createdAt: Date(), updatedAt: nil))
        let listID = try XCTUnwrap(list.id)
        try await links.addSong(playlistId: listID, songId: firstID)
        let collision = Song(id: nil, songKey: 999, artist: "Artist", title: "New tags", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 888, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly, sourceId: 8, relativePath: "Other/track.flac")
        let other = try await repository.upsertSong(collision)
        let renamed = Song(id: nil, songKey: 999, artist: "Artist", title: "New tags", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 999, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly, sourceId: 7, relativePath: "Album/track.flac")
        let updated = try await repository.upsertSong(renamed)
        XCTAssertEqual(updated.id, first.id)
        XCTAssertEqual(updated.isFavorite, 1)
        let membership = try await links.getSongs(playlistId: listID)
        XCTAssertEqual(membership.map(\.id), [firstID])
        XCTAssertEqual(membership.first?.title, "New tags")
        let otherRows = await repository.getSongs(ids: [try XCTUnwrap(other.id)])
        XCTAssertEqual(otherRows.first?.sourceId, 8)
        let count = try await repository.totalSongCount(query: "")
        XCTAssertEqual(count, 2)
    }
    func testCacheUpdatesPreserveConcurrentMetadataAndCannotReinsertDeletedSong() async throws {
        let database = try Connection(.inMemory)
        let repository = try SQLiteSongRepository(db: database)
        let song = try await repository.upsertSong(makeSong())
        let id = try XCTUnwrap(song.id)
        try database.execute("UPDATE songs SET title = 'User edit' WHERE id = \(id)")
        try await repository.updateBookmark(songId: id, bookmark: Data([9]))
        let copied = try await repository.updateCachedFile(songId: id, localFilePath: "Music/cached.flac", state: .copied, expectedState: .bookmarkOnly, expectedPath: nil)
        XCTAssertTrue(copied)
        let latest = await repository.getSongs(ids: [id]).first
        XCTAssertEqual(latest?.title, "User edit")
        XCTAssertEqual(latest?.bookmark, Data([9]))
        let staleFailure = try await repository.updateCachedFile(songId: id, localFilePath: nil, state: .failed, expectedState: .bookmarkOnly, expectedPath: nil)
        XCTAssertFalse(staleFailure)
        try await repository.deleteSong(songId: id)
        let deletedCopy = try await repository.updateCachedFile(songId: id, localFilePath: "Music/cached.flac", state: .copied, expectedState: .copied, expectedPath: "Music/cached.flac")
        XCTAssertFalse(deletedCopy)
        let remaining = await repository.getSongs(ids: [id])
        XCTAssertTrue(remaining.isEmpty)
    }
    func testUpgradeAndFavoriteSurviveReimportAndSearch() async throws {
        let db = try Connection(.inMemory)
        // Simulate the schema before local-copy and Favorites columns existed.
        try db.execute("""
            CREATE TABLE songs (
                id INTEGER PRIMARY KEY AUTOINCREMENT, songKey INTEGER NOT NULL,
                artist TEXT NOT NULL, title TEXT NOT NULL, album TEXT NOT NULL,
                albumArtist TEXT NOT NULL, releaseYear INTEGER, discNumber INTEGER,
                trackNumber INTEGER, coverArtPath TEXT, bookmark BLOB,
                pathHash INTEGER NOT NULL, createdAt REAL NOT NULL, updatedAt REAL
            );
            INSERT INTO songs (songKey, artist, title, album, albumArtist, pathHash, createdAt)
            VALUES (99, 'Existing', 'Keep me', 'Old library', 'Existing', 99, 0);
            """)
        let repo = try SQLiteSongRepository(db: db)
        let old = await repo.getSongs(ids: [1])
        XCTAssertEqual(old.count, 1)
        XCTAssertEqual(old.first?.isFavorite, 0)
        XCTAssertEqual(old.first?.fileState, .bookmarkOnly)
        // Running startup migrations twice must also be safe.
        _ = try SQLiteSongRepository(db: db)
        let song = makeSong()
        let inserted = try await repo.upsertSong(song)
        let id = try XCTUnwrap(inserted.id)
        try await repo.toggleSongAsFavorite(songId: id, state: true)
        let reimported = try await repo.upsertSong(song)
        XCTAssertEqual(reimported.isFavorite, 1)
        let results = try await repo.searchSongsFTS(query: "Title", limit: 20, offset: 0)
        XCTAssertEqual(results.count, 1)
        XCTAssertEqual(results.first?.isFavorite, 1)
        XCTAssertEqual(results.first?.albumArtist, "Album Artist")
        XCTAssertEqual(results.first?.releaseYear, 2026)
        XCTAssertEqual(results.first?.discNumber, 2)
        let ordered = await repo.getSongs(ids: [id, 1, id])
        XCTAssertEqual(ordered.compactMap(\.id), [id, 1, id])
        try await repo.toggleSongAsFavorite(songId: id, state: false)
        let restored = await repo.getSongs(ids: [id])
        XCTAssertEqual(restored.first?.isFavorite, 0)
    }

    func testPlaylistReadsFavoriteAndMetadata() async throws {
        let db = try Connection(.inMemory)
        let songs = try SQLiteSongRepository(db: db)
        let playlists = try SQLitePlaylistRepository(db: db)
        let links = try SQLitePlaylistSongRepository(db: db)
        let song = try await songs.upsertSong(makeSong())
        let playlist = try await playlists.create(playlist: Playlist(id: nil, name: "Favorites", createdAt: Date(), updatedAt: nil))
        let songID = try XCTUnwrap(song.id)
        let playlistID = try XCTUnwrap(playlist.id)
        try await songs.toggleSongAsFavorite(songId: songID, state: true)
        try await links.addSong(playlistId: playlistID, songId: songID)
        let rows = try await links.getSongs(playlistId: playlistID)
        XCTAssertEqual(rows.first?.isFavorite, 1)
        XCTAssertEqual(rows.first?.title, "Title")
    }

    func testPlaylistDuplicateAddsPreserveReorderedMembershipAndAppendNewSongs() async throws {
        let db = try Connection(.inMemory)
        let songs = try SQLiteSongRepository(db: db)
        let playlists = try SQLitePlaylistRepository(db: db)
        let links = try SQLitePlaylistSongRepository(db: db)
        let list = try await playlists.create(playlist: Playlist(id: nil, name: "Ordered", createdAt: Date(), updatedAt: nil))
        let listID = try XCTUnwrap(list.id)
        var ids: [Int64] = []
        for key: Int64 in 1...5 {
            let saved = try await songs.upsertSong(Song(id: nil, songKey: key, artist: "Artist", title: "Track \(key)", album: "Album",
                albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil,
                pathHash: key, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly))
            ids.append(try XCTUnwrap(saved.id))
        }
        for id in ids.prefix(3) { try await links.addSong(playlistId: listID, songId: id) }
        try await links.reorderSongs(playlistId: listID, newOrder: [ids[2], ids[0], ids[1]])
        try await links.addSong(playlistId: listID, songId: ids[0])
        var ordered = try await links.getSongs(playlistId: listID)
        XCTAssertEqual(ordered.compactMap(\.id), [ids[2], ids[0], ids[1]])
        for id in [ids[1], ids[3], ids[2], ids[3], ids[4], ids[0]] {
            try await links.addSong(playlistId: listID, songId: id)
        }
        ordered = try await links.getSongs(playlistId: listID)
        let expected = [ids[2], ids[0], ids[1], ids[3], ids[4]]
        XCTAssertEqual(ordered.compactMap(\.id), expected)
        let raw = try db.prepare("SELECT songId, position FROM playlist_songs WHERE playlistId = ? ORDER BY position", listID)
        let positions = raw.map { ($0[0] as? Int64, $0[1] as? Int64) }
        XCTAssertEqual(positions.compactMap { $0.0 }, expected)
        XCTAssertEqual(positions.compactMap { $0.1 }, [0, 1, 2, 3, 4], "Ignored duplicates must not move old rows or leave append gaps")
        // Conflict handling must not turn genuine SQLite write failures into success.
        try db.execute("PRAGMA query_only = ON")
        do {
            try await links.addSong(playlistId: listID, songId: ids[0])
            XCTFail("A read-only database must still report its SQL write error")
        } catch { XCTAssertTrue(String(describing: error).contains("readonly")) }
    }

    private func makeSong() -> Song {
        Song(id: nil, songKey: 123, artist: "Artist", title: "Title", album: "Album",
             albumArtist: "Album Artist", releaseYear: 2026, discNumber: 2, trackNumber: 3,
             coverArtPath: nil, bookmark: nil, pathHash: 123, createdAt: Date(), updatedAt: nil,
             localFilePath: nil, fileState: .bookmarkOnly)
    }
}
