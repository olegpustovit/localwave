import AVFoundation
import Foundation
import LocalWaveDomain
import LocalWaveData
import SQLite
import Testing
@testable import localwave

struct WaveformTests {
    @Test func waveformUsesDecodedAudioAndPreservesSilence() async throws {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).appendingPathExtension("caf")
        defer { try? FileManager.default.removeItem(at: url) }
        let format = try #require(AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 1))
        let buffer = try #require(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 48000))
        buffer.frameLength = 48000
        let samples = try #require(buffer.floatChannelData?[0])
        for index in 0..<48000 { samples[index] = index < 24000 ? 0 : 0.5 }
        do {
            let file = try AVAudioFile(forWriting: url, settings: format.settings)
            try file.write(from: buffer)
        }
        let song = Song(id: 1, songKey: 1, artist: "Test", title: "Signal", album: "Test", albumArtist: "Test", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 1, createdAt: Date(), updatedAt: nil, localFilePath: url.path, fileState: .copied)
        let peaks = try await WaveformCache.decode(url: url)
        #expect(peaks.count == 512)
        #expect(peaks.prefix(256).allSatisfy { $0 == 0 })
        #expect(peaks.suffix(256).allSatisfy { $0 == 1 })
    }

    @MainActor @Test func queueSelectionSurvivesShuffleEditsAfterUnavailableQueueStops() async throws {
        let database = try Connection(.inMemory)
        let player = PlayerViewModel(playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database))
        let songs = (1...3).map { index in
            Song(id: Int64(index), songKey: Int64(index), artist: "Test", title: "Track \(index)", album: "Test", albumArtist: "Test", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: Int64(index), createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly)
        }
        player.configureQueue(songs: songs, startIndex: 0)
        player.playSong(songs[1])
        for _ in 0..<100 {
            if !player.isLoading { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(player.currentSong?.id == 3)
        #expect(!player.isPlaying)
        #expect(player.playbackError != nil)
        player.removeFromQueue(at: 0)
        #expect(player.queue.map(\.id) == [2, 3])
        player.setShuffle(true)
        player.removeFromQueue(at: 1)
        player.setShuffle(false)
        #expect(player.queue.map(\.id) == [3])
        player.clearUpcomingQueue()
        #expect(player.queue.map(\.id) == [3])
    }

    @MainActor @Test func duplicateQueueEntriesAdvanceAndNewQueueResetsShuffle() throws {
        let database = try Connection(.inMemory)
        let player = PlayerViewModel(playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database))
        defer { player.stop() }
        let a = queueSong(1), b = queueSong(2), c = queueSong(3)
        player.configureQueue(songs: [a, a, b], startIndex: 0)
        player.playQueueEntry(at: 1)
        #expect(player.currentQueueIndex == 1)
        player.nextSong()
        #expect(player.currentQueueIndex == 2)
        #expect(player.currentSong?.id == 2)
        player.previousSong()
        #expect(player.currentQueueIndex == 1)
        player.reorderQueue(from: IndexSet(integer: 0), to: 3)
        #expect(player.currentQueueIndex == 0)
        player.removeFromQueue(at: 2)
        #expect(player.queue.map(\.id) == [1, 2])
        player.setShuffle(true)
        #expect(player.queue.count == 2)
        player.configureQueue(songs: [c], startIndex: 0)
        player.setShuffle(false)
        #expect(!player.isShuffleEnabled)
        #expect(player.queue.map(\.id) == [3])
        #expect(player.currentSong?.id == 3)
    }

    @MainActor @Test func restorationDoesNotWriteEmptyQueueOrOverrideAnEarlySelection() async throws {
        let database = try Connection(.inMemory)
        let playlists = try SQLitePlaylistRepository(db: database)
        let entries = try SQLitePlaylistSongRepository(db: database)
        let stored = queueSong(1), chosen = queueSong(2)
        let service = RestorationProbe(songs: [stored])
        let restored = PlayerViewModel(playerPersistenceService: service, playlistRepo: playlists, playlistSongRepo: entries)
        try await Task.sleep(for: .milliseconds(80))
        #expect(restored.queue.map(\.id) == [1])
        #expect(restored.volume == 0.4)
        #expect(await service.emptyWrites == 0)
        let delayed = RestorationProbe(songs: [stored])
        let selected = PlayerViewModel(playerPersistenceService: delayed, playlistRepo: playlists, playlistSongRepo: entries)
        selected.configureQueue(songs: [chosen], startIndex: 0)
        try await Task.sleep(for: .milliseconds(80))
        #expect(selected.queue.map(\.id) == [2])
    }

    @MainActor @Test func queuedAudioHandsOffToDuplicateEntryWithoutReplacingEngineOrItem() async throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let format = try #require(AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 1))
        let buffer = try #require(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 57600))
        buffer.frameLength = 57600
        let data = try #require(buffer.floatChannelData?[0])
        for index in 0..<57600 { data[index] = sin(Float(index) * 2 * .pi * 220 / 48000) * 0.05 }
        let audioURL = folder.appendingPathComponent("handoff.caf")
        do {
            let file = try AVAudioFile(forWriting: audioURL, settings: format.settings)
            try file.write(from: buffer)
        }
        let a = queueSong(101).copyWith(audioURL.path, .copied)
        let b = queueSong(102).copyWith(audioURL.path, .copied)
        let database = try Connection(.inMemory)
        let player = PlayerViewModel(playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database))
        defer { player.stop() }
        player.configureQueue(songs: [a, a, b], startIndex: 0)
        player.playQueueEntry(at: 0)
        for _ in 0..<400 {
            if player.playbackEngine?.items().count == 3, player.isPlaying { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        let engine = try #require(player.playbackEngine)
        #expect(engine.items().count == 3)
        let secondItem = try #require(engine.items().dropFirst().first)
        for _ in 0..<500 {
            if player.currentQueueIndex == 1 { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(player.currentQueueIndex == 1)
        #expect(player.currentSong?.id == 101)
        #expect(player.playbackEngine === engine)
        #expect(engine.currentItem === secondItem)
        #expect(engine.currentItem?.status == .readyToPlay)
        #expect(player.isPlaying == (engine.timeControlStatus == .playing))
    }

    @MainActor @Test func addingToEmptyQueueSeedsAPausedCurrentSong() throws {
        let database = try Connection(.inMemory)
        let player = PlayerViewModel(playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database))
        defer { player.stop() }
        player.addToQueue(queueSong(1))
        player.addToQueue(queueSong(2))
        #expect(player.currentSong?.id == 1)
        #expect(player.currentQueueIndex == 0)
        #expect(player.queue.map(\.id) == [1, 2])
        #expect(!player.isPlaying)
    }

    @MainActor @Test func disablingCloudLookaheadPreservesLocalGaplessHandoffAndCurrentItem() async throws {
        let suite = "localwave-lookahead-" + UUID().uuidString
        let preferences = try #require(UserDefaults(suiteName: suite))
        defer { preferences.removePersistentDomain(forName: suite) }
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let format = try #require(AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 1))
        let buffer = try #require(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 96000))
        buffer.frameLength = 96000
        let data = try #require(buffer.floatChannelData?[0])
        for index in 0..<96000 { data[index] = sin(Float(index) * 2 * .pi * 220 / 48000) * 0.05 }
        let url = folder.appendingPathComponent("local.caf")
        do { let file = try AVAudioFile(forWriting: url, settings: format.settings); try file.write(from: buffer) }
        let local = queueSong(201).copyWith(url.path, .copied)
        let cloud = queueSong(202), laterCloud = queueSong(203)
        let knownLocal = queueSong(204, availability: .onDevice)
        let repository = MockOrderedSongRepository(songs: [local, cloud, laterCloud, knownLocal])
        let database = try Connection(.inMemory)
        let player = PlayerViewModel(songRepo: repository, playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database), downloadAheadPreferences: preferences)
        defer { player.stop() }
        #expect(preferences.object(forKey: "lwDownloadAhead") == nil)
        #expect(player.downloadAheadEnabled)
        player.setDownloadAhead(false)
        player.configureQueue(songs: [local, local, cloud, laterCloud], startIndex: 0)
        player.playQueueEntry(at: 0)
        for _ in 0..<400 {
            if player.playbackEngine?.items().count == 2, player.isPlaying { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        let engine = try #require(player.playbackEngine)
        let current = try #require(engine.currentItem)
        let next = try #require(engine.items().dropFirst().first)
        #expect(await repository.copyRequests.isEmpty)
        player.setDownloadAhead(true)
        for _ in 0..<100 {
            if !(await repository.copyRequests.isEmpty) { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(await repository.copyRequests == [202])
        player.setDownloadAhead(false)
        #expect(player.playbackEngine === engine)
        #expect(engine.currentItem === current)
        #expect(engine.items().dropFirst().first === next)
        for _ in 0..<500 {
            if player.currentQueueIndex == 1 { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(player.currentQueueIndex == 1)
        #expect(engine.currentItem === next)
        #expect(await repository.copyRequests == [202])
        #expect(!player.downloadAheadEnabled)
        player.addToQueue(knownLocal)
        player.reorderQueue(from: IndexSet(integer: 4), to: 2)
        player.setDownloadAhead(false)
        for _ in 0..<100 {
            if await repository.copyRequests.contains(204) { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(Set(await repository.copyRequests) == Set([202, 204]))
        #expect(engine.currentItem === next)
    }

    private func queueSong(_ id: Int64, availability: FileAvailability? = nil) -> Song {
        Song(id: id, songKey: id, artist: "Test", title: "Track \(id)", album: "Test", albumArtist: "Test", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: id, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly, sourceAvailability: availability)
    }

}


private actor RestorationProbe: PlayerPersistenceService {
    var songs: [Song]
    private(set) var emptyWrites = 0
    init(songs: [Song]) { self.songs = songs }
    func getVolume() async -> Float? {
        try? await Task.sleep(for: .milliseconds(20))
        return 0.4
    }
    func restore() async -> ([Song], Int, Song?)? { (songs, 0, songs.first) }
    func savePlaybackState(volume: Float, currentIndex: Int, songs: [Song]) async {
        if songs.isEmpty { emptyWrites += 1 }
        self.songs = songs
    }
}
