import Foundation
import XCTest
import SQLite
import LocalWaveCore
import LocalWaveDomain
@testable import LocalWaveData

final class SourceRemovalTests: XCTestCase {
    func testRemovalCleansLinkedAndLegacyEntriesButPreservesOtherOwnerAndOriginalFiles() async throws {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let folder = base.appendingPathComponent("Collection", isDirectory: true)
        let nested = folder.appendingPathComponent("Nested", isDirectory: true)
        let cache = base.appendingPathComponent("Cache", isDirectory: true)
        try FileManager.default.createDirectory(at: nested, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: cache, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: base) }
        let targetFile = folder.appendingPathComponent("first.flac")
        let otherFile = nested.appendingPathComponent("other.flac")
        let cachedFile = cache.appendingPathComponent("cached.flac")
        for file in [targetFile, otherFile, cachedFile] { try Data([1, 2, 3]).write(to: file) }
        let database = try Connection(.inMemory)
        let sources = try SQLiteSourceRepository(db: database)
        let songs = try SQLiteSongRepository(db: database)
        let paths = try SQLiteSourcePathRepository(db: database)
        let pathSearch = try SQLiteSourcePathSearchRepository(db: database)
        let lists = try SQLitePlaylistRepository(db: database)
        let links = try SQLitePlaylistSongRepository(db: database)
        func source(_ root: URL) throws -> Source {
            Source(id: nil, dirPath: root.path, pathId: makeURLHash(root), userId: 1, type: .iCloud, totalPaths: nil, syncError: nil, isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil, bookmark: try root.bookmarkData(options: sourceBookmarkCreationOptions))
        }
        let targetSource = try await sources.create(source: source(folder))
        let otherSource = try await sources.create(source: source(nested))
        let targetID = try XCTUnwrap(targetSource.id)
        let otherID = try XCTUnwrap(otherSource.id)
        func song(_ key: Int64, title: String, file: URL, source: Int64?, relative: String?) throws -> Song {
            Song(id: nil, songKey: key, artist: "Artist", title: title, album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: try file.bookmarkData(options: sourceBookmarkCreationOptions), pathHash: makeURLHash(file.resolvingSymlinksInPath()), createdAt: Date(), updatedAt: nil, localFilePath: cachedFile.path, fileState: .copied, sourceId: source, relativePath: relative)
        }
        let target = try await songs.upsertSong(song(1, title: "Target", file: targetFile, source: targetID, relative: "first.flac"))
        let legacy = try await songs.upsertSong(song(2, title: "Legacy", file: targetFile, source: nil, relative: nil))
        let other = try await songs.upsertSong(song(3, title: "Other", file: otherFile, source: otherID, relative: "other.flac"))
        let listed = try await lists.create(playlist: Playlist(id: nil, name: "Mixed", createdAt: Date(), updatedAt: nil))
        let playlistID = try XCTUnwrap(listed.id)
        for item in [target, legacy, other] { try await links.addSong(playlistId: playlistID, songId: XCTUnwrap(item.id)) }
        let sourcePaths = [
            SourcePath(id: nil, sourceId: targetID, pathId: makeURLHash(targetFile), parentPathId: targetSource.pathId, name: "first.flac", relativePath: "first.flac", isDirectory: false, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil),
            SourcePath(id: nil, sourceId: targetID, pathId: makeURLHash(otherFile), parentPathId: targetSource.pathId, name: "other.flac", relativePath: "Nested/other.flac", isDirectory: false, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil)
        ]
        try await paths.batchUpsert(paths: sourcePaths)
        try await pathSearch.batchUpsertIntoFTS(paths: sourcePaths)
        let access = SourceAccess(repository: sources, sourcePathRepository: paths, songRepository: songs)
        _ = try await access.rootURL(sourceId: targetID)
        try await access.remove(sourceId: targetID)
        let retained = try await songs.searchSongsFTS(query: "", limit: 10, offset: 0)
        XCTAssertEqual(retained.map(\.id), [other.id])
        XCTAssertEqual(retained.first?.sourceId, otherID)
        let playlist = try await links.getSongs(playlistId: playlistID)
        XCTAssertEqual(playlist.map(\.id), [other.id])
        let removedFTS = try await songs.searchSongsFTS(query: "Target", limit: 10, offset: 0)
        XCTAssertTrue(removedFTS.isEmpty)
        let removedPaths = try await paths.getByParentId(sourceId: targetID, parentPathId: nil)
        let removedPathSearch = try await pathSearch.search(sourceId: targetID, query: "first", limit: 10)
        XCTAssertTrue(removedPaths.isEmpty)
        XCTAssertTrue(removedPathSearch.isEmpty)
        for file in [targetFile, otherFile, cachedFile] { XCTAssertEqual(try Data(contentsOf: file), Data([1, 2, 3])) }
        do { _ = try await songs.upsertSong(target); XCTFail("Late import recreated a removed source's song") } catch { }
        do { try await paths.batchUpsert(paths: sourcePaths); XCTFail("Late scan recreated removed paths") } catch { }
        do { try await pathSearch.batchUpsertIntoFTS(paths: sourcePaths); XCTFail("Late scan recreated removed search rows") } catch { }
        do { _ = try await access.rootURL(sourceId: targetID); XCTFail("Removed source kept an accessible cached root") } catch { }
    }
}
