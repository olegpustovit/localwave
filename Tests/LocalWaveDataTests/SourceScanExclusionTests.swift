import Foundation
import XCTest
import SQLite
import LocalWaveCore
import LocalWaveDomain
@testable import LocalWaveData

final class SourceScanExclusionTests: XCTestCase {
    func testGeneratedAbsoluteSubtreesAreExcludedButExternalMusicFoldersRemain() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let cache = root.appendingPathComponent("App/Documents/Music", isDirectory: true)
        let shared = root.appendingPathComponent("App/tmp/LocalWave Share", isDirectory: true)
        let external = root.appendingPathComponent("Collection/Music", isDirectory: true)
        for folder in [cache, shared, external] { try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true) }
        defer { try? FileManager.default.removeItem(at: root) }
        try Data([1]).write(to: cache.appendingPathComponent("generated.flac"))
        try Data([1]).write(to: shared.appendingPathComponent("shared.flac"))
        try Data([1]).write(to: external.appendingPathComponent("original.flac"))
        let excluded = [cache, shared, root.appendingPathComponent("App/Caches/waveforms.sqlite")]
        XCTAssertTrue(LocalWaveGeneratedPaths.contains(cache.appendingPathComponent("generated.flac"), excludedURLs: excluded))
        XCTAssertFalse(LocalWaveGeneratedPaths.contains(external, excludedURLs: excluded))
        XCTAssertFalse(LocalWaveGeneratedPaths.contains(root.appendingPathComponent("App/Documents/Music Original"), excludedURLs: excluded))
        let database = try Connection(.inMemory)
        let sources = try SQLiteSourceRepository(db: database)
        let paths = try SQLiteSourcePathRepository(db: database)
        let search = try SQLiteSourcePathSearchRepository(db: database)
        let access = SourceAccess(repository: sources, excludedURLs: excluded)
        let source = try await sources.create(source: Source(id: nil, dirPath: root.path, pathId: makeURLHash(root), userId: 1, type: .iCloud, totalPaths: nil, syncError: nil, isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil, bookmark: root.bookmarkData(options: sourceBookmarkCreationOptions)))
        let scanner = DefaultSourceSyncService(sourceRepository: sources, sourcePathSearchRepository: search, sourcePathRepository: paths, sourceAccess: access, excludedURLs: excluded)
        let id = try XCTUnwrap(source.id)
        let scanned = try await scanner.syncDir(sourceId: id, folderURL: root, onCurrentURL: nil, onSetLoading: nil)
        XCTAssertNil(scanned?.syncError)
        XCTAssertEqual(scanned?.totalPaths, 1)
        let indexed = try await paths.getByParentId(sourceId: id, parentPathId: nil)
        XCTAssertEqual(indexed.filter { !$0.isDirectory }.map(\.relativePath), ["Collection/Music/original.flac"])
        let cacheSource = try await sources.create(source: Source(id: nil, dirPath: cache.path, pathId: makeURLHash(cache), userId: 1, type: .iCloud, totalPaths: nil, syncError: nil, isCurrent: false, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil, bookmark: cache.bookmarkData(options: sourceBookmarkCreationOptions)))
        let rejected = try await scanner.syncDir(sourceId: XCTUnwrap(cacheSource.id), folderURL: cache, onCurrentURL: nil, onSetLoading: nil)
        XCTAssertNotNil(rejected?.syncError)
    }

    func testReadonlyScanAndGeneratedCacheWritesDoNotTriggerRescanStream() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let cache = root.appendingPathComponent("Music", isDirectory: true)
        try FileManager.default.createDirectory(at: cache, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let database = try Connection(.inMemory)
        let sources = try SQLiteSourceRepository(db: database)
        let source = try await sources.create(source: Source(id: nil, dirPath: root.path, pathId: makeURLHash(root), userId: 1, type: .iCloud, totalPaths: nil, syncError: nil, isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil, bookmark: root.bookmarkData(options: sourceBookmarkCreationOptions)))
        let access = SourceAccess(repository: sources, excludedURLs: [cache])
        let id = try XCTUnwrap(source.id)
        let stream = await access.changes()
        let unwanted = expectation(description: "No self-generated changes")
        unwanted.isInverted = true
        let observer = Task { for await _ in stream { unwanted.fulfill() } }
        defer { observer.cancel() }
        await access.beginScan(sourceId: id)
        _ = try await access.rootURL(sourceId: id)
        var coordinationError: NSError?
        var writeError: Error?
        NSFileCoordinator().coordinate(writingItemAt: cache.appendingPathComponent("generated.flac"), options: [], error: &coordinationError) { file in
            do { try Data([1, 2]).write(to: file) } catch { writeError = error }
        }
        XCTAssertNil(coordinationError)
        XCTAssertNil(writeError)
        await access.endScan(sourceId: id)
        await fulfillment(of: [unwanted], timeout: 1.2)
    }
}
