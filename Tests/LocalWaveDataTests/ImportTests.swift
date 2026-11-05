import XCTest
import Foundation
import AVFoundation
import ImageIO
import SQLite
import LocalWaveCore
import LocalWaveDomain
@testable import LocalWaveData

final class ImportTests: XCTestCase {
    func testFallbackTitlesDropExtensionAndLeadingTrackNumbers() {
        XCTAssertEqual(DefaultSongImportService.fallbackTitle(for: "01 – COUSINS.m4a"), "COUSINS")
        XCTAssertEqual(DefaultSongImportService.fallbackTitle(for: "07.artist-name.flac"), "artist-name")
        XCTAssertEqual(DefaultSongImportService.fallbackTitle(for: "1984.m4a"), "1984")
        XCTAssertEqual(DefaultSongImportService.fallbackTitle(for: "CJKタイトル.wav"), "CJKタイトル")
    }

    func testM4AReadsITunesTagsArtworkAndAudibleNativeAudio() async throws {
        // One-second generated three-tone AAC, with real M4A tags and PNG art.
        // Album artist differs deliberately so a fallback to artist cannot pass.
        let fixture = try XCTUnwrap(Bundle.module.url(forResource: "tagged-m4a", withExtension: "m4a", subdirectory: "Fixtures"))
        let db = try Connection(.inMemory)
        let importer = DefaultSongImportService(songRepo: try SQLiteSongRepository(db: db),
            sourcePathRepo: try SQLiteSourcePathRepository(db: db), sourceRepo: try SQLiteSourceRepository(db: db))
        let tags = await importer.readMetadataAVAsset(url: fixture)
        XCTAssertEqual(tags.title, "M4A Fixture")
        XCTAssertEqual(tags.artist, "LocalWave Tests")
        XCTAssertEqual(tags.album, "M4A Album")
        XCTAssertEqual(tags.albumArtist, "AAC Ensemble")
        XCTAssertEqual(tags.releaseYear, 2025)
        XCTAssertEqual(tags.trackNumber, 3)
        XCTAssertEqual(tags.discNumber, 2)

        let metadata = try await AVURLAsset(url: fixture).load(.metadata)
        let artwork = try XCTUnwrap(metadata.first { $0.commonKey == .commonKeyArtwork })
        let data = try await artwork.load(.dataValue)
        let imageData = try XCTUnwrap(data)
        let source = try XCTUnwrap(CGImageSourceCreateWithData(imageData as CFData, nil))
        XCTAssertNotNil(CGImageSourceCreateImageAtIndex(source, 0, nil))
        let audio = try AVAudioFile(forReading: fixture)
        let buffer = try XCTUnwrap(AVAudioPCMBuffer(pcmFormat: audio.processingFormat, frameCapacity: 4096))
        try audio.read(into: buffer, frameCount: 4096)
        XCTAssertGreaterThan(buffer.frameLength, 0)
        let samples = try XCTUnwrap(buffer.floatChannelData)
        let peak = (0..<Int(buffer.frameLength)).map { abs(samples[0][$0]) }.max() ?? 0
        XCTAssertGreaterThan(peak, 0.01, "The fixture must contain actual audible samples, not only metadata")
    }

    func testITunesTrackAndDiscIndicesRejectTruncatedAndUnspecifiedValues() {
        for bytes: [UInt8] in [[], [0], [0, 0], [0, 0, 1], [0, 0, 0, 0]] {
            XCTAssertNil(DefaultSongImportService.iTunesIndex(from: Data(bytes)))
        }
        XCTAssertEqual(DefaultSongImportService.iTunesIndex(from: Data([0, 0, 0, 3, 0, 8, 0, 0])), 3)
        XCTAssertEqual(DefaultSongImportService.iTunesIndex(from: Data([0, 0, 1, 44])), 300)
        XCTAssertEqual(DefaultSongImportService.iTunesIndex(from: Data([0, 0, 255, 255])), 65535)
        let sliced = Data([99, 0, 0, 0, 2, 0, 3, 0, 0]).dropFirst()
        XCTAssertEqual(DefaultSongImportService.iTunesIndex(from: sliced), 2)
    }

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
        UserDefaults.standard.set(try folder.bookmarkData(options: sourceBookmarkCreationOptions), forKey: bookmarkKey)
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
        let noteURL = nested.appendingPathComponent("cover.txt")
        try Data("not audio".utf8).write(to: noteURL)
        let note = try await paths.create(path: SourcePath(id: nil, sourceId: sourceID, pathId: 4, parentPathId: 2, name: "cover.txt", relativePath: "nested/cover.txt", isDirectory: false, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil, fileSize: 9))
        let importer = DefaultSongImportService(songRepo: songs, sourcePathRepo: paths, sourceRepo: sources)
        let progress = ImportProgressRecorder()
        try await importer.importPathsWithFileProgress(paths: [directory]) { event in await progress.append(event) }
        let events = await progress.events
        XCTAssertEqual(events.map(\.completedFiles), [0, 1])
        XCTAssertEqual(events.map(\.totalFiles), [1, 1], "Folders and non-audio paths must not inflate the denominator")
        XCTAssertEqual(events.map(\.percentage), [0, 100])

        try await importer.importPaths(paths: [note], onProgress: nil)
        let unprocessedNote = try await paths.getByPathId(sourceId: sourceID, pathId: 4)
        XCTAssertNil(unprocessedNote?.fileHashSHA256)
        XCTAssertEqual(unprocessedNote?.fileSize, 9)
        let results = try await songs.searchSongsFTS(query: "", limit: 10, offset: 0)
        let song = try XCTUnwrap(results.first)
        XCTAssertEqual(results.count, 1)
        XCTAssertEqual(song.title, "FLAC Fixture")
        XCTAssertEqual(song.artist, "LocalWave Tests")
        XCTAssertEqual(song.albumArtist, "Test Ensemble")
        XCTAssertEqual(song.releaseYear, 2026)
        XCTAssertEqual(song.trackNumber, 3)
        XCTAssertEqual(song.discNumber, 2)
        XCTAssertEqual(song.fileState, .bookmarkOnly)
        XCTAssertEqual(song.sourceId, sourceID)
        XCTAssertEqual(song.relativePath, "nested/tagged.flac")
        XCTAssertNil(song.bookmark)
        XCTAssertEqual(try XCTUnwrap(song.duration), Double(audio.length) / audio.processingFormat.sampleRate, accuracy: 0.02)
        XCTAssertEqual(song.fileExtension, "flac")
        XCTAssertEqual(song.fileSize, Int64(try target.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0))
        XCTAssertEqual(song.sampleRate, audio.fileFormat.sampleRate)
        let durableSource = try await sources.getOne(id: sourceID)
        XCTAssertNotNil(durableSource?.bookmark)
        let saved = try await paths.getByPathId(sourceId: sourceID, pathId: 3)
        XCTAssertNotNil(saved?.fileHashSHA256)
        _ = try await songs.upsertSong(song.copyWith("Music/already-cached.flac", .copied))
        try await importer.importPaths(paths: [directory], onProgress: nil)
        let unchanged = await songs.getSongs(ids: [try XCTUnwrap(song.id)]).first
        XCTAssertEqual(unchanged?.localFilePath, "Music/already-cached.flac")
        XCTAssertEqual(unchanged?.fileState, .copied)
        try FileManager.default.setAttributes([.modificationDate: Date(timeIntervalSinceNow: 5)], ofItemAtPath: target.path)
        try await importer.importPaths(paths: [directory], onProgress: nil)
        let changed = await songs.getSongs(ids: [try XCTUnwrap(song.id)]).first
        XCTAssertEqual(changed?.fileState, .copied)
        XCTAssertEqual(changed?.localFilePath, "Music/already-cached.flac")
        let changedFile = try FileHandle(forWritingTo: target)
        try changedFile.seekToEnd()
        try changedFile.write(contentsOf: Data([0]))
        try changedFile.close()
        try await importer.importPaths(paths: [directory], onProgress: nil)
        let contentChanged = await songs.getSongs(ids: [try XCTUnwrap(song.id)]).first
        XCTAssertEqual(contentChanged?.fileState, .bookmarkOnly)
        XCTAssertNil(contentChanged?.localFilePath)
    }
}

private actor ImportProgressRecorder {
    var events: [SongImportProgress] = []
    func append(_ event: SongImportProgress) { events.append(event) }
}
