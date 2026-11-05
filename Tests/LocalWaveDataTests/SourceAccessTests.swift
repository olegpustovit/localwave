import Foundation
import XCTest
import SQLite
import LocalWaveCore
import LocalWaveDomain
@testable import LocalWaveData

final class SourceAccessTests: XCTestCase {
    func testPresentedRootRenameUpdatesDurableDisplayPathAndRelativeAccess() async throws {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let original = base.appendingPathComponent("Original", isDirectory: true)
        let moved = base.appendingPathComponent("Renamed", isDirectory: true)
        try FileManager.default.createDirectory(at: original, withIntermediateDirectories: true)
        try Data([1, 2]).write(to: original.appendingPathComponent("track.flac"))
        defer { try? FileManager.default.removeItem(at: base) }
        let repository = try SQLiteSourceRepository(db: Connection(.inMemory))
        let source = try await repository.create(source: source(original, bookmark: original.bookmarkData(options: sourceBookmarkCreationOptions)))
        let id = try XCTUnwrap(source.id)
        let access = SourceAccess(repository: repository)
        let stream = await access.changes()
        let received = expectation(description: "Root move reached source access")
        let observer = Task { for await changed in stream { if changed == id { received.fulfill(); return } } }
        defer { observer.cancel() }
        _ = try await access.rootURL(sourceId: id)
        try await Task.detached {
            let coordinator = NSFileCoordinator()
            var coordinationError: NSError?
            var moveError: Error?
            coordinator.coordinate(writingItemAt: original, options: .forMoving, writingItemAt: moved, options: .forReplacing, error: &coordinationError) { oldURL, newURL in
                do {
                    coordinator.item(at: oldURL, willMoveTo: newURL)
                    try FileManager.default.moveItem(at: oldURL, to: newURL)
                    coordinator.item(at: oldURL, didMoveTo: newURL)
                } catch { moveError = error }
            }
            if let coordinationError { throw coordinationError }
            if let moveError { throw moveError }
        }.value
        await fulfillment(of: [received], timeout: 5)
        let stored = try await repository.getOne(id: id)
        XCTAssertEqual(URL(fileURLWithPath: try XCTUnwrap(stored?.dirPath)).lastPathComponent, "Renamed")
        XCTAssertEqual(stored?.id, id)
        let file = try await access.fileURL(sourceId: id, relativePath: "track.flac")
        XCTAssertEqual(try Data(contentsOf: file.file), Data([1, 2]))
    }
    func testNonAudioMetadataChangeReachesSourceWatcher() async throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let repository = try SQLiteSourceRepository(db: Connection(.inMemory))
        let saved = try await repository.create(source: source(folder, bookmark: folder.bookmarkData(options: sourceBookmarkCreationOptions)))
        let id = try XCTUnwrap(saved.id)
        let access = SourceAccess(repository: repository)
        let stream = await access.changes()
        _ = try await access.rootURL(sourceId: id)
        let changed = expectation(description: "Non-audio child metadata change is watched")
        let observer = Task { for await sourceID in stream { if sourceID == id { changed.fulfill(); return } } }
        defer { observer.cancel() }
        let cover = folder.appendingPathComponent("cover.jpg")
        try await Task.detached {
            var coordinationError: NSError?
            var writeError: Error?
            NSFileCoordinator().coordinate(writingItemAt: cover, options: .forReplacing, error: &coordinationError) { url in
                do { try Data([1, 2, 3]).write(to: url) } catch { writeError = error }
            }
            if let coordinationError { throw coordinationError }
            if let writeError { throw writeError }
        }.value
        await fulfillment(of: [changed], timeout: 5)
    }

    private func source(_ folder: URL, bookmark: Data? = nil) -> Source {
        Source(id: nil, dirPath: folder.path, pathId: makeURLHash(folder), userId: 1, type: .iCloud, totalPaths: 7, syncError: nil, isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil, bookmark: bookmark)
    }

    func testOldSourceSchemaAndLegacyBookmarkMigrateWithoutLosingData() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let database = try Connection(.inMemory)
        try database.execute("""
            CREATE TABLE sources (id INTEGER PRIMARY KEY AUTOINCREMENT, dirPath TEXT NOT NULL,
                pathId INTEGER NOT NULL, userId INTEGER NOT NULL, type TEXT, totalPaths INTEGER,
                syncError TEXT, isCurrent INTEGER NOT NULL, createdAt TEXT NOT NULL,
                lastSyncedAt TEXT, updatedAt TEXT);
            """)
        try database.run(Table("sources").insert(
            Expression<String>("dirPath") <- directory.path,
            Expression<Int64>("pathId") <- makeURLHash(directory),
            Expression<Int64>("userId") <- 1,
            Expression<String?>("type") <- "iCloud",
            Expression<Int?>("totalPaths") <- 7,
            Expression<Bool>("isCurrent") <- true,
            Expression<Date>("createdAt") <- Date()
        ))
        let repository = try SQLiteSourceRepository(db: database)
        _ = try SQLiteSourceRepository(db: database)
        let existing = try await repository.getOne(id: 1)
        XCTAssertEqual(existing?.totalPaths, 7)
        XCTAssertEqual(existing?.dirPath, directory.path)
        XCTAssertNil(existing?.bookmark)
        let suite = "SourceAccessTests-" + UUID().uuidString
        let preferences = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { preferences.removePersistentDomain(forName: suite) }
        preferences.set(try directory.bookmarkData(options: sourceBookmarkCreationOptions), forKey: makeBookmarkKey(directory))
        let access = SourceAccess(repository: repository, preferences: preferences)
        let first = try await access.rootURL(sourceId: 1)
        let migrated = try await repository.getOne(id: 1)
        XCTAssertNotNil(migrated?.bookmark)
        preferences.removeObject(forKey: makeBookmarkKey(directory))
        let relaunched = SourceAccess(repository: repository, preferences: preferences)
        let second = try await relaunched.rootURL(sourceId: 1)
        XCTAssertEqual(first.resolvingSymlinksInPath(), second.resolvingSymlinksInPath())
        let preserved = try await repository.getOne(id: 1)
        XCTAssertEqual(preserved?.totalPaths, 7)
    }

    func testSourceRelativeSongSurvivesMovedRootAndReconnectWithoutChildBookmark() async throws {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let original = base.appendingPathComponent("Original", isDirectory: true)
        let nested = original.appendingPathComponent("Album", isDirectory: true)
        try FileManager.default.createDirectory(at: nested, withIntermediateDirectories: true)
        let bytes = Data([1, 2, 3, 4])
        try bytes.write(to: nested.appendingPathComponent("track.flac"))
        defer { try? FileManager.default.removeItem(at: base) }
        let database = try Connection(.inMemory)
        let sources = try SQLiteSourceRepository(db: database)
        let songs = try SQLiteSongRepository(db: database)
        let registered = try await sources.create(source: source(original, bookmark: original.bookmarkData(options: sourceBookmarkCreationOptions)))
        let sourceID = try XCTUnwrap(registered.id)
        let inserted = try await songs.upsertSong(Song(id: nil, songKey: 1, artist: "Artist", title: "Track", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 1, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly, sourceId: sourceID, relativePath: "Album/track.flac"))
        let access = SourceAccess(repository: sources)
        let before = try await access.fileURL(sourceId: sourceID, relativePath: "Album/track.flac")
        XCTAssertEqual(try Data(contentsOf: before.file), bytes)
        let moved = base.appendingPathComponent("Moved", isDirectory: true)
        try FileManager.default.moveItem(at: original, to: moved)
        let reconnected = try await access.reconnect(sourceId: sourceID, to: moved)
        XCTAssertEqual(reconnected.id, sourceID)
        XCTAssertEqual(reconnected.totalPaths, 7)
        let rows = await songs.getSongs(ids: [try XCTUnwrap(inserted.id)])
        let restored = try XCTUnwrap(rows.first)
        XCTAssertNil(restored.bookmark)
        XCTAssertEqual(restored.relativePath, "Album/track.flac")
        let playlists = try SQLitePlaylistRepository(db: database)
        let links = try SQLitePlaylistSongRepository(db: database)
        let playlist = try await playlists.create(playlist: Playlist(id: nil, name: "Keeps its source", createdAt: Date(), updatedAt: nil))
        try await links.addSong(playlistId: XCTUnwrap(playlist.id), songId: XCTUnwrap(restored.id))
        let queued = try await links.getSongs(playlistId: XCTUnwrap(playlist.id))
        XCTAssertEqual(queued.first?.sourceId, sourceID)
        XCTAssertEqual(queued.first?.relativePath, "Album/track.flac")
        let after = try await access.fileURL(sourceId: try XCTUnwrap(restored.sourceId), relativePath: try XCTUnwrap(restored.relativePath))
        XCTAssertEqual(try Data(contentsOf: after.file), bytes)
        XCTAssertTrue(after.file.path.contains("Moved/Album/track.flac"))
    }

    func testRelativePathValidationRejectsTraversalAbsolutePathsAndEscapingSymlinks() throws {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let root = base.appendingPathComponent("Root", isDirectory: true)
        let outside = base.appendingPathComponent("outside.flac")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        try Data([1]).write(to: outside)
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("escape.flac"), withDestinationURL: outside)
        defer { try? FileManager.default.removeItem(at: base) }
        XCTAssertThrowsError(try SourceAccess.fileURL(root: root, relativePath: "../outside.flac"))
        XCTAssertThrowsError(try SourceAccess.fileURL(root: root, relativePath: outside.path))
        XCTAssertThrowsError(try SourceAccess.fileURL(root: root, relativePath: "escape.flac"))
        let valid = try SourceAccess.fileURL(root: root, relativePath: "Album/a.flac")
        XCTAssertEqual(valid.lastPathComponent, "a.flac")
    }
}
