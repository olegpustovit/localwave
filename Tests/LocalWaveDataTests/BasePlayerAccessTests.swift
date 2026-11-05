import Foundation
import AVFoundation
import XCTest
import SQLite
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData
@testable import LocalWavePlayer

@MainActor
final class BasePlayerAccessTests: XCTestCase {
    private func audioFolder() throws -> URL {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let fixture = try XCTUnwrap(Bundle.module.url(forResource: "tagged", withExtension: "flac", subdirectory: "Fixtures"))
        try FileManager.default.copyItem(at: fixture, to: root.appendingPathComponent("track.flac"))
        return root
    }

    private func song(_ id: Int64, sourceId: Int64? = nil) -> Song {
        Song(id: id, songKey: id, artist: "Artist", title: "Track \(id)", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: id, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly, sourceId: sourceId, relativePath: "track.flac")
    }

    private func player(root: URL, resolver: @escaping PlaybackFileAccess.SourceResolver) throws -> (BasePlayerViewModel, RecordingPlayer) {
        let database = try Connection(.inMemory)
        let audio = RecordingPlayer()
        let model = BasePlayerViewModel(player: audio, playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database), sourceAccessResolver: resolver)
        model.cacheDirectory = root.appendingPathComponent("Cache")
        return (model, audio)
    }

    private func waitForLoad(_ model: BasePlayerViewModel) async throws {
        for _ in 0..<300 {
            if !model.isLoading { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("Audio preload did not finish")
    }

    func testRootOnlySongPlaysFromCoordinatedCacheWithoutChildBookmark() async throws {
        let root = try audioFolder()
        defer { try? FileManager.default.removeItem(at: root) }
        let sources = try SQLiteSourceRepository(db: Connection(.inMemory))
        let source = try await sources.create(source: Source(id: nil, dirPath: root.path, pathId: makeURLHash(root), userId: 1, type: .iCloud, totalPaths: 1, syncError: nil, isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil, bookmark: root.bookmarkData(options: sourceBookmarkCreationOptions)))
        let access = SourceAccess(repository: sources)
        let track = song(1, sourceId: try XCTUnwrap(source.id))
        let (model, audio) = try player(root: root) { song in
            try await access.fileURL(sourceId: XCTUnwrap(song.sourceId), relativePath: XCTUnwrap(song.relativePath))
        }
        defer { model.stop() }
        model.playSong(track)
        try await waitForLoad(model)
        XCTAssertNil(model.playbackError)
        XCTAssertTrue(model.isPlaying)
        XCTAssertEqual(audio.prepareCount, 1)
        XCTAssertEqual(audio.url?.deletingLastPathComponent().lastPathComponent, "Cache")
        XCTAssertGreaterThan(audio.duration, 0)
    }

    func testPauseWhileRootResolutionIsPendingPreventsAutoplay() async throws {
        let root = try audioFolder()
        defer { try? FileManager.default.removeItem(at: root) }
        let gate = LoadGate()
        let (model, audio) = try player(root: root) { _ in
            await gate.wait()
            return (root, root.appendingPathComponent("track.flac"))
        }
        defer { model.stop() }
        model.playSong(song(1))
        XCTAssertTrue(model.isLoading)
        model.pause()
        await gate.release()
        try await waitForLoad(model)
        XCTAssertNil(model.playbackError)
        XCTAssertEqual(audio.playCount, 0)
        XCTAssertFalse(model.isPlaying)
        model.play()
        XCTAssertTrue(model.isPlaying)
    }

    func testReplacedPendingLoadCannotPrepareOrPlayOverNewSong() async throws {
        let root = try audioFolder()
        defer { try? FileManager.default.removeItem(at: root) }
        let gate = LoadGate()
        let (model, audio) = try player(root: root) { song in
            if song.id == 1 { await gate.wait() }
            return (root, root.appendingPathComponent("track.flac"))
        }
        defer { model.stop() }
        model.playSong(song(1))
        await Task.yield()
        model.playSong(song(2))
        try await waitForLoad(model)
        await gate.release()
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(model.currentSong?.id, 2)
        XCTAssertEqual(audio.prepareCount, 1)
        XCTAssertEqual(audio.playCount, 1)
        XCTAssertTrue(model.isPlaying)
    }
}

private actor LoadGate {
    private var released = false
    private var continuations: [CheckedContinuation<Void, Never>] = []
    func wait() async {
        if released { return }
        await withCheckedContinuation { continuations.append($0) }
    }
    func release() {
        released = true
        let pending = continuations
        continuations.removeAll()
        pending.forEach { $0.resume() }
    }
}

@MainActor
private final class RecordingPlayer: AudioPlayerProtocol {
    var isPlaying = false
    var currentTime: TimeInterval = 0
    var duration: TimeInterval = 0
    var volume: Float = 1
    let rate: Float = 1
    var url: URL?
    weak var delegate: AudioPlayerDelegate?
    var prepareCount = 0
    var playCount = 0
    func prepareToPlay(url: URL) throws {
        let decoded = try AVAudioFile(forReading: url)
        duration = Double(decoded.length) / decoded.processingFormat.sampleRate
        self.url = url
        prepareCount += 1
    }
    func play() { if url != nil { isPlaying = true; playCount += 1 } }
    func pause() { isPlaying = false }
    func stop() { isPlaying = false; currentTime = 0 }
}
