import Foundation
import AVFoundation
import Combine
import XCTest
import SQLite
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData
@testable import localwave

@MainActor
final class SourceWatchingTests: XCTestCase {
    func testFailedScanRefreshIncludesCommittedReconnectError() async throws {
        let missing = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let database = try Connection(.inMemory)
        let sources = try SQLiteSourceRepository(db: database)
        let paths = try SQLiteSourcePathRepository(db: database)
        let search = try SQLiteSourcePathSearchRepository(db: database)
        let access = SourceAccess(repository: sources)
        let scan = DefaultSourceSyncService(sourceRepository: sources, sourcePathSearchRepository: search, sourcePathRepository: paths, sourceAccess: access)
        let browse = DefaultSourceImportService(sourceRepository: sources, sourcePathRepository: paths, sourcePathSearchRepository: search)
        let service = DefaultSourceService(sourceRepo: sources, sourceSyncService: scan, sourceImportService: browse)
        let source = try await sources.create(source: Source(id: nil, dirPath: missing.path, pathId: makeURLHash(missing), userId: 1, type: .iCloud, totalPaths: nil, syncError: nil, isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil))
        let sourceID = try XCTUnwrap(source.id)
        let model = SyncViewModel(userCloudService: WatchTestUser(), icloudProvider: nil, sourceService: service, songImportService: nil, sourceAccess: access)
        model.sources = [source]
        var snapshots: [String?] = []
        // Read at notification delivery, rather than after another asynchronous task can commit.
        let observer = NotificationCenter.default.publisher(for: Notification.Name("LibraryRefresh"))
            .sink { _ in snapshots.append((try? database.scalar("SELECT syncError FROM sources WHERE id = ?", sourceID)) as? String) }
        defer { observer.cancel() }
        model.resyncSource(source)
        await model.awaitPendingWork()
        let stored = try await sources.getOne(id: sourceID)
        let failure = try XCTUnwrap(stored?.syncError)
        XCTAssertFalse(failure.isEmpty)
        XCTAssertEqual(model.sources.first?.syncError, failure)
        XCTAssertGreaterThanOrEqual(snapshots.count, 2)
        XCTAssertEqual(snapshots.last ?? nil, failure)
    }

    func testCoordinatedNewAudioAutomaticallyReachesLibrary() async throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let database = try Connection(.inMemory)
        let sources = try SQLiteSourceRepository(db: database)
        let paths = try SQLiteSourcePathRepository(db: database)
        let pathSearch = try SQLiteSourcePathSearchRepository(db: database)
        let songs = try SQLiteSongRepository(db: database)
        let access = SourceAccess(repository: sources)
        let scan = DefaultSourceSyncService(sourceRepository: sources, sourcePathSearchRepository: pathSearch, sourcePathRepository: paths, sourceAccess: access)
        let browse = DefaultSourceImportService(sourceRepository: sources, sourcePathRepository: paths, sourcePathSearchRepository: pathSearch)
        let service = DefaultSourceService(sourceRepo: sources, sourceSyncService: scan, sourceImportService: browse)
        let importer = DefaultSongImportService(songRepo: songs, sourcePathRepo: paths, sourceRepo: sources, sourceAccess: access)
        let source = try await sources.create(source: Source(id: nil, dirPath: folder.path, pathId: makeURLHash(folder), userId: 1, type: .iCloud, totalPaths: nil, syncError: nil, isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil, bookmark: folder.bookmarkData(options: sourceBookmarkCreationOptions)))
        let sourceID = try XCTUnwrap(source.id)
        let model = SyncViewModel(userCloudService: WatchTestUser(), icloudProvider: nil, sourceService: service, songImportService: importer, sourceAccess: access)
        model.refreshSources()
        var initialized = false
        for _ in 0..<300 {
            if model.sources.first?.lastSyncedAt != nil, model.currentSyncSourceId == nil { initialized = true; break }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTAssertTrue(initialized)
        let added = folder.appendingPathComponent("added.wav")
        let format = try XCTUnwrap(AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 1))
        let buffer = try XCTUnwrap(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 4800))
        buffer.frameLength = 4800
        let samples = try XCTUnwrap(buffer.floatChannelData?[0])
        let phaseStep: Double = 2.0 * Double.pi * 220.0 / 48000.0
        for frame in 0..<4800 { samples[frame] = Float(sin(Double(frame) * phaseStep)) * Float(0.1) }
        var coordinationError: NSError?
        var writeError: Error?
        NSFileCoordinator().coordinate(writingItemAt: added, options: [], error: &coordinationError) { url in
            do {
                let file = try AVAudioFile(forWriting: url, settings: format.settings)
                try file.write(from: buffer)
            } catch { writeError = error }
        }
        XCTAssertNil(coordinationError)
        XCTAssertNil(writeError)
        for _ in 0..<500 {
            if let imported = await songs.getSong(sourceId: sourceID, relativePath: "added.wav"), model.currentSyncSourceId == nil {
                XCTAssertNil(imported.bookmark)
                XCTAssertEqual(try XCTUnwrap(imported.duration), 0.1, accuracy: 0.01)
                XCTAssertNil(model.sources.first?.syncError)
                return
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("The watched source did not import its new audio file")
    }
}

private struct WatchTestUser: UserCloudService {
    func resolveCurrentICloudUser() async throws -> User? { User(id: 1, icloudId: 1) }
}
