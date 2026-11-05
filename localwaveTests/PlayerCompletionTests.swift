import AVFoundation
import Foundation
import LocalWaveData
import LocalWaveDomain
import SQLite
import Testing
@testable import localwave

struct PlayerCompletionTests {
    // GitHub runner VMs have no audio hardware: the item never reaches its end there (FigFilePlayer -12864).
    @MainActor @Test(.disabled(if: ProcessInfo.processInfo.environment["LOCALWAVE_NO_AUDIO_DEVICE"] != nil, "Needs a real audio clock"))
    func naturalCompletionRetainsMeasuredMetadataAndReplayStartsAtZero() async throws {
        let url = try makeAudio(seconds: 1.25)
        defer { try? FileManager.default.removeItem(at: url) }
        let database = try Connection(.inMemory)
        let player = PlayerViewModel(playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database))
        defer { player.stop() }
        let track = song(url: url)
        #expect(track.duration == nil)
        #expect(track.fileSize == nil)
        player.configureQueue(songs: [track], startIndex: 0)
        player.playQueueEntry(at: 0)
        for _ in 0..<500 {
            if player.isPlaying && player.duration == "0:01" { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        try #require(player.isPlaying)
        let size = player.playbackFileSize
        let format = player.playbackFormat
        #expect(size != "—")
        for _ in 0..<1500 { // 15 s ceiling for a 1.25 s clip; polling exits as soon as playback ends
            if player.playbackEngine == nil { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        try #require(player.playbackEngine == nil)
        #expect(player.currentSong?.id == track.id)
        #expect(player.currentTime == "0:01")
        #expect(player.duration == "0:01")
        #expect(player.playbackProgress == 1)
        #expect(player.remainingTime == "-0:00")
        #expect(player.playbackFileSize == size)
        #expect(player.playbackFormat == format)
        #expect(player.playbackURL == url)
        player.playPause()
        for _ in 0..<500 {
            if player.isPlaying { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(player.isPlaying)
        #expect(player.playbackProgress < 0.5)
        #expect(player.playbackFileSize == size)
    }

    @MainActor @Test func libraryRefreshHydratesCurrentTagsWithoutReplacingPlaybackItem() async throws {
        let url = try makeAudio(seconds: 8)
        defer { try? FileManager.default.removeItem(at: url) }
        let database = try Connection(.inMemory)
        let repository = try SQLiteSongRepository(db: database)
        let initial = try await repository.upsertSong(song(url: url))
        let player = PlayerViewModel(songRepo: repository, playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database))
        defer { player.stop() }
        player.configureQueue(songs: [initial], startIndex: 0)
        player.playQueueEntry(at: 0)
        for _ in 0..<500 {
            if player.isPlaying { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        try #require(player.isPlaying)
        let engine = try #require(player.playbackEngine)
        let item = try #require(engine.currentItem)
        player.setShuffle(true)
        _ = try await repository.upsertSong(song(url: url, title: "Decoded title"))
        NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        for _ in 0..<100 {
            if player.currentSong?.title == "Decoded title" { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(player.currentSong?.title == "Decoded title")
        #expect(player.queue.first?.title == "Decoded title")
        player.setShuffle(false)
        #expect(player.queue.first?.title == "Decoded title")
        #expect(player.playbackEngine === engine)
        #expect(engine.currentItem === item)
        #expect(player.isPlaying)
    }

    private func song(url: URL, title: String = "Placeholder") -> Song {
        Song(id: 905, songKey: 905, artist: "Test", title: title, album: "Test", albumArtist: "Test", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 905, createdAt: Date(), updatedAt: nil, localFilePath: url.path, fileState: .copied)
    }

    private func makeAudio(seconds: Double) throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).appendingPathExtension("caf")
        let format = try #require(AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 1))
        let count = AVAudioFrameCount(48000 * seconds)
        let buffer = try #require(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: count))
        buffer.frameLength = count
        let samples = try #require(buffer.floatChannelData?[0])
        for index in 0..<Int(count) { samples[index] = 0 }
        let file = try AVAudioFile(forWriting: url, settings: format.settings)
        try file.write(from: buffer)
        return url
    }
}
