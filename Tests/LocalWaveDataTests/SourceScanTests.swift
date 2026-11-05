import XCTest
import Foundation
import SQLite
import LocalWaveDomain
import LocalWaveCore
@testable import LocalWaveData

final class SourceScanTests: XCTestCase {
    func testRescanIndexesFileMetadataButCountsOnlyAudio() async throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        let nested = folder.appendingPathComponent("Album", isDirectory: true)
        try FileManager.default.createDirectory(at: nested, withIntermediateDirectories: true)
        try Data("cover notes".utf8).write(to: folder.appendingPathComponent("notes.txt"))
        try Data("private".utf8).write(to: folder.appendingPathComponent(".hidden.txt"))
        try Data([0]).write(to: nested.appendingPathComponent("track.flac"))
        let bookmark = makeBookmarkKey(folder)
        UserDefaults.standard.set(try folder.bookmarkData(options: sourceBookmarkCreationOptions), forKey: bookmark)
        defer {
            UserDefaults.standard.removeObject(forKey: bookmark)
            try? FileManager.default.removeItem(at: folder)
        }
        let db = try Connection(.inMemory)
        let sources = try SQLiteSourceRepository(db: db)
        let paths = try SQLiteSourcePathRepository(db: db)
        let search = try SQLiteSourcePathSearchRepository(db: db)
        let source = try await sources.create(source: Source(id: nil, dirPath: folder.path, pathId: makeURLHash(folder), userId: 1, type: .iCloud, totalPaths: nil, syncError: "Old access error", isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil))
        let scanner = DefaultSourceSyncService(sourceRepository: sources, sourcePathSearchRepository: search, sourcePathRepository: paths)
        let sourceID = try XCTUnwrap(source.id)
        let updated = try await scanner.syncDir(sourceId: sourceID, folderURL: folder, onCurrentURL: nil, onSetLoading: nil)
        XCTAssertNil(updated?.syncError)
        XCTAssertEqual(updated?.totalPaths, 1)
        let children = try await paths.getByParentId(sourceId: sourceID, parentPathId: source.pathId)
        XCTAssertEqual(Set(children.map(\.name)), ["Album", "notes.txt"])
        XCTAssertTrue(try XCTUnwrap(children.first { $0.name == "Album" }).isDirectory)
        let note = try XCTUnwrap(children.first { $0.name == "notes.txt" })
        XCTAssertFalse(note.isAudio)
        XCTAssertEqual(note.fileSize, 11)
        XCTAssertNil(note.fileHashSHA256)
        let trackHash = makeURLHash(folder.appendingPathComponent("Album/track.flac"))
        try await paths.updateFileHash(pathId: trackHash, fileHash: Data([1, 2, 3]))
        _ = try await scanner.syncDir(sourceId: sourceID, folderURL: folder, onCurrentURL: nil, onSetLoading: nil)
        let rescanned = try await paths.getByPathId(sourceId: sourceID, pathId: trackHash)
        XCTAssertEqual(rescanned?.fileHashSHA256, Data([1, 2, 3]))
    }
}
