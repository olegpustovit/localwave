import XCTest
import Foundation
import AVFoundation
import SQLite
import LocalWaveCore
import LocalWaveDomain
@testable import LocalWaveData

final class ImportTests: XCTestCase {
    func testRecursiveFolderImportReadsFLACTagsAndNativeDecoder() async throws {
        let fixture = try XCTUnwrap(Bundle.module.url(forResource: "tagged", withExtension: "flac", subdirectory: "Fixtures"))
        let audio = try AVAudioFile(forReading: fixture)
        XCTAssertGreaterThan(audio.length, 0)
        let buffer = try XCTUnwrap(AVAudioPCMBuffer(pcmFormat: audio.processingFormat, frameCapacity: AVAudioFrameCount(audio.length)))
        try audio.read(into: buffer)
        XCTAssertGreaterThan(buffer.frameLength, 0)

        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let nested = folder.appendingPathComponent("nested")
        try FileManager.default.createDirectory(at: nested, withIntermediateDirectories: true)
        let target = nested.appendingPathComponent("tagged.flac")
        try FileManager.default.copyItem(at: fixture, to: target)
        let bookmarkKey = makeBookmarkKey(folder)
        UserDefaults.standard.set(try folder.bookmarkData(), forKey: bookmarkKey)
        defer {
            UserDefaults.standard.removeObject(forKey: bookmarkKey)
            try? FileManager.default.removeItem(at: folder)
        }
        let db = try Connection(.inMemory)
        let songs = try SQLiteSongRepository(db: db)
        let sources = try SQLiteSourceRepository(db: db)
        let paths = try SQLiteSourcePathRepository(db: db)
        let source = try await sources.create(source: Source(id: nil, dirPath: folder.path, pathId: 1, userId: 1, type: .iCloud, totalPaths: nil, syncError: nil, isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil))
        let sourceID = try XCTUnwrap(source.id)
        let directory = try await paths.create(path: SourcePath(id: nil, sourceId: sourceID, pathId: 2, parentPathId: 1, name: "nested", relativePath: "nested", isDirectory: true, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil))
        _ = try await paths.create(path: SourcePath(id: nil, sourceId: sourceID, pathId: 3, parentPathId: 2, name: "tagged.flac", relativePath: "nested/tagged.flac", isDirectory: false, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil))
        let importer = DefaultSongImportService(songRepo: songs, sourcePathRepo: paths, sourceRepo: sources)
        try await importer.importPaths(paths: [directory], onProgress: nil)
        let results = try await songs.searchSongsFTS(query: "", limit: 10, offset: 0)
        let song = try XCTUnwrap(results.first)
        XCTAssertEqual(results.count, 1)
        XCTAssertEqual(song.title, "FLAC Fixture")
        XCTAssertEqual(song.artist, "LocalWave Tests")
        XCTAssertEqual(song.albumArtist, "Test Ensemble")
        XCTAssertEqual(song.releaseYear, 2026)
        XCTAssertEqual(song.trackNumber, 3)
        XCTAssertEqual(song.discNumber, 2)
        XCTAssertEqual(song.fileState, .copyPending)
        let saved = try await paths.getByPathId(sourceId: sourceID, pathId: 3)
        XCTAssertNotNil(saved?.fileHashSHA256)
    }
}
