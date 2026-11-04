import XCTest
import Foundation
import SQLite
import LocalWaveDomain
@testable import LocalWaveData

final class RepositoryTests: XCTestCase {
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
        _ = try await repo.upsertSong(song)
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

    private func makeSong() -> Song {
        Song(id: nil, songKey: 123, artist: "Artist", title: "Title", album: "Album",
             albumArtist: "Album Artist", releaseYear: 2026, discNumber: 2, trackNumber: 3,
             coverArtPath: nil, bookmark: nil, pathHash: 123, createdAt: Date(), updatedAt: nil,
             localFilePath: nil, fileState: .bookmarkOnly)
    }
}
