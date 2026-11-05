import Foundation
import XCTest
import SQLite
import LocalWaveDomain
@testable import LocalWaveData

final class OfflineCacheTests: XCTestCase {
    func testLRUEvictsOnlyOldUnpinnedManagedCopiesAndPreservesOriginals() async throws {
        let docs = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let music = docs.appendingPathComponent("Music")
        let added = docs.appendingPathComponent("Added Files")
        try FileManager.default.createDirectory(at: music, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: added, withIntermediateDirectories: true)
        let suite = "LocalWaveCacheTests-" + UUID().uuidString
        let preferences = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer {
            preferences.removePersistentDomain(forName: suite)
            try? FileManager.default.removeItem(at: docs)
        }
        let repository = try SQLiteSongRepository(db: Connection(.inMemory))
        var records: [Song] = []
        for (index, name) in ["old", "recent", "pinned", "active"].enumerated() {
            let file = music.appendingPathComponent(name + ".flac")
            try Data(repeating: UInt8(index), count: 4).write(to: file)
            try FileManager.default.setAttributes([.modificationDate: Date(timeIntervalSince1970: Double(index))], ofItemAtPath: file.path)
            records.append(try await repository.upsertSong(Song(id: nil, songKey: Int64(index + 1), artist: "Artist", title: name, album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: Data([42]), pathHash: Int64(index + 1), createdAt: Date(), updatedAt: nil, localFilePath: "Music/" + name + ".flac", fileState: .copied)))
        }
        let original = added.appendingPathComponent("original.flac")
        try Data(repeating: 7, count: 100).write(to: original)
        preferences.set([String(try XCTUnwrap(records[2].id))], forKey: "lwPinnedSongs")
        preferences.set(String(try XCTUnwrap(records[3].id)), forKey: "lwActiveSongID")
        let cache = BackgroundFileService(songRepo: repository, documentsURL: docs, preferences: preferences)
        XCTAssertEqual(try BackgroundFileService.cachedByteCount(documentsURL: docs), 16)
        try await cache.enforceCacheLimit(maxBytes: 12)
        XCTAssertFalse(FileManager.default.fileExists(atPath: music.appendingPathComponent("old.flac").path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: music.appendingPathComponent("recent.flac").path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: original.path))
        let evicted = await repository.getSongs(ids: [try XCTUnwrap(records[0].id)]).first
        XCTAssertNil(evicted?.localFilePath)
        XCTAssertEqual(evicted?.fileState, .bookmarkOnly)
        XCTAssertEqual(evicted?.bookmark, Data([42]))
        let materializing = music.appendingPathComponent("not-yet-in-database.flac")
        try Data([8, 8, 8]).write(to: materializing)
        preferences.set(materializing.resolvingSymlinksInPath().path, forKey: "lwActiveCachePath")
        try await cache.enforceCacheLimit(maxBytes: 0)
        XCTAssertTrue(FileManager.default.fileExists(atPath: music.appendingPathComponent("pinned.flac").path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: music.appendingPathComponent("active.flac").path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: original.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: materializing.path))
    }

    func testOnlyExplicitPendingCopiesAreRequested() async throws {
        let repository = try SQLiteSongRepository(db: Connection(.inMemory))
        let song = try await repository.upsertSong(Song(id: nil, songKey: 1, artist: "Artist", title: "Song", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 1, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly))
        let before = await repository.getSongsNeedingCopy()
        XCTAssertTrue(before.isEmpty)
        try await repository.markSongForCopy(songId: try XCTUnwrap(song.id))
        let after = await repository.getSongsNeedingCopy()
        XCTAssertEqual(after.map(\.id), [song.id])
    }
}
