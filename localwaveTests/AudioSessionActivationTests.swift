import AVFoundation
import Foundation
import LocalWaveData
import LocalWaveDomain
import SQLite
import Testing
@testable import LocalWaveCore
@testable import localwave

struct AudioSessionActivationTests {
    @MainActor @Test func stopDuringSessionActivationCannotStartTheRetainedQueuePlayer() async throws {
        let url = try makeAudio()
        defer { try? FileManager.default.removeItem(at: url) }
        let gate = BlockingSessionActivation()
        defer { gate.release() }
        let session = AudioSessionController(configure: {}, activate: { gate.activate() })
        let database = try Connection(.inMemory)
        let player = PlayerViewModel(playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database), audioSession: session)
        defer { player.stop() }
        let song = Song(id: 903, songKey: 903, artist: "Test", title: "Activation", album: "Test", albumArtist: "Test", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 903, createdAt: Date(), updatedAt: nil, localFilePath: url.path, fileState: .copied)
        player.configureQueue(songs: [song], startIndex: 0)
        player.playQueueEntry(at: 0)
        for _ in 0..<500 {
            if gate.started { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(gate.started)
        let engine = try #require(player.playbackEngine)
        player.stop()
        gate.release()
        try await Task.sleep(for: .milliseconds(100))
        #expect(player.playbackEngine == nil)
        #expect(!player.isPlaying)
        #expect(!player.isLoading)
        #expect(engine.rate == 0)
        #expect(engine.items().isEmpty)
    }

    @MainActor @Test func adapterPauseDuringActivationCannotStartPlaybackLater() async throws {
        let url = try makeAudio()
        defer { try? FileManager.default.removeItem(at: url) }
        let gate = BlockingSessionActivation()
        defer { gate.release() }
        let session = AudioSessionController(configure: {}, activate: { gate.activate() })
        let adapter = AVAudioPlayerAdapter(audioSession: session)
        try adapter.prepareToPlay(url: url)
        adapter.play()
        for _ in 0..<300 {
            if gate.started { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(gate.started)
        adapter.pause()
        gate.release()
        try await Task.sleep(for: .milliseconds(100))
        #expect(!adapter.isPlaying)
        #expect(adapter.currentTime == 0)
    }

    @MainActor @Test(arguments: ["resume", "unplug", "stop"]) func queueInterruptionRespectsResumeAndCancellation(action: String) async throws {
        let url = try makeAudio()
        defer { try? FileManager.default.removeItem(at: url) }
        let session = AudioSessionController(configure: {}, activate: {})
        let database = try Connection(.inMemory)
        let player = PlayerViewModel(playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database), audioSession: session)
        defer { player.stop() }
        let song = Song(id: 904, songKey: 904, artist: "Test", title: "Interruption", album: "Test", albumArtist: "Test", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 904, createdAt: Date(), updatedAt: nil, localFilePath: url.path, fileState: .copied)
        player.configureQueue(songs: [song], startIndex: 0)
        player.playQueueEntry(at: 0)
        for _ in 0..<500 {
            if player.isPlaying { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        try #require(player.isPlaying)
        await session.receive(.interruptionBegan)
        for _ in 0..<100 {
            if !player.isPlaying { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        try #require(!player.isPlaying)
        if action == "unplug" { await session.receive(.oldOutputUnavailable) }
        else if action == "stop" { player.stop() }
        await session.receive(.interruptionEnded(shouldResume: true))
        try await Task.sleep(for: .milliseconds(100))
        #expect(player.isPlaying == (action == "resume"))
        if action != "resume" { #expect((player.playbackEngine?.rate ?? 0) == 0) }
        if action == "stop" { #expect(player.playbackEngine == nil) }
    }

    @MainActor @Test(arguments: ["resume", "unplug", "stop", "pause"]) func adapterInterruptionRespectsResumeAndCancellation(action: String) async throws {
        let url = try makeAudio()
        defer { try? FileManager.default.removeItem(at: url) }
        let session = AudioSessionController(configure: {}, activate: {})
        let adapter = AVAudioPlayerAdapter(audioSession: session)
        defer { adapter.stop() }
        try adapter.prepareToPlay(url: url)
        adapter.play()
        for _ in 0..<300 {
            if adapter.isPlaying { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        try #require(adapter.isPlaying)
        await session.receive(.interruptionBegan)
        for _ in 0..<100 {
            if !adapter.isPlaying { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        try #require(!adapter.isPlaying)
        switch action {
        case "unplug": await session.receive(.oldOutputUnavailable)
        case "stop": adapter.stop()
        case "pause": adapter.pause()
        default: break
        }
        await session.receive(.interruptionEnded(shouldResume: true))
        try await Task.sleep(for: .milliseconds(100))
        #expect(adapter.isPlaying == (action == "resume"))
    }

    private func makeAudio() throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).appendingPathExtension("caf")
        let format = try #require(AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 1))
        let buffer = try #require(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 48000))
        buffer.frameLength = 48000
        let samples = try #require(buffer.floatChannelData?[0])
        for index in 0..<48000 { samples[index] = 0 }
        let file = try AVAudioFile(forWriting: url, settings: format.settings)
        try file.write(from: buffer)
        return url
    }
}

private final class BlockingSessionActivation: @unchecked Sendable {
    private let condition = NSCondition()
    private var began = false
    private var released = false
    var started: Bool {
        condition.lock()
        defer { condition.unlock() }
        return began
    }
    func activate() {
        condition.lock()
        began = true
        while !released { condition.wait() }
        condition.unlock()
    }
    func release() {
        condition.lock()
        released = true
        condition.broadcast()
        condition.unlock()
    }
}
