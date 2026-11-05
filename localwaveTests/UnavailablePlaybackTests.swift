import AVFoundation
import Foundation
import LocalWaveDomain
import LocalWaveData
import SQLite
import Testing
@testable import LocalWaveCore
@testable import localwave

struct UnavailablePlaybackTests {
    @MainActor @Test(arguments: [false, true])
    func missingOrCorruptAudioSkipsToActualPlayableCAF(corrupt: Bool) async throws {
        let folder = try makeFolder()
        defer { try? FileManager.default.removeItem(at: folder) }
        let badURL = folder.appendingPathComponent("unavailable.caf")
        if corrupt { try Data("This is not audio".utf8).write(to: badURL) }
        let goodURL = try makeCAF(in: folder)
        let player = try makePlayer()
        defer { player.stop() }
        let bad = song(401, path: badURL.path), good = song(402, path: goodURL.path)
        player.configureQueue(songs: [bad, good], startIndex: 0, context: .album(name: "Original context", artist: "Test"))
        let entries = player.queueEntryIdentifiers
        player.playQueueEntry(at: 0)
        try await waitUntil { player.currentQueueIndex == 1 && player.isPlaying }
        #expect(player.currentSong?.id == good.id)
        #expect(player.playbackURL == goodURL)
        #expect(player.queueEntryIdentifiers == entries)
        #expect(player.playbackContext == .album(name: "Original context", artist: "Test"))
        #expect(player.playbackError == nil)
        #expect(player.playbackNotice?.contains(bad.title) == true)
    }

    @MainActor @Test func duplicateUnavailableIDsAreDistinctBoundedEntries() async throws {
        let folder = try makeFolder()
        defer { try? FileManager.default.removeItem(at: folder) }
        let goodURL = try makeCAF(in: folder)
        let reads = UnavailableReadCounter()
        let player = try makePlayer(resolver: { song in await reads.record(song.id); return nil })
        defer { player.stop() }
        let missing = song(410, path: folder.appendingPathComponent("missing.caf").path)
        player.configureQueue(songs: [missing, missing, song(411, path: goodURL.path)], startIndex: 0)
        let entries = player.queueEntryIdentifiers
        #expect(Set(entries).count == 3)
        player.playQueueEntry(at: 0)
        try await waitUntil { player.currentQueueIndex == 2 && player.isPlaying }
        #expect(await reads.ids == [410, 410])
        #expect(player.queueEntryIdentifiers == entries)
        #expect(player.playbackError == nil)
    }

    @MainActor @Test func prequeuedCorruptDecoderAdvancesToPlayableAudio() async throws {
        let folder = try makeFolder()
        defer { try? FileManager.default.removeItem(at: folder) }
        let firstURL = try makeCAF(in: folder, name: "first.caf", seconds: 0.5)
        let finalURL = try makeCAF(in: folder)
        let corruptURL = folder.appendingPathComponent("corrupt.caf")
        try Data("Invalid queued audio".utf8).write(to: corruptURL)
        let player = try makePlayer()
        defer { player.stop() }
        player.configureQueue(songs: [song(450, path: firstURL.path), song(451, path: corruptURL.path), song(452, path: finalURL.path)], startIndex: 0)
        player.playQueueEntry(at: 0)
        try await waitUntil { player.currentQueueIndex == 2 && player.isPlaying }
        #expect(player.currentSong?.id == 452)
        #expect(player.playbackURL == finalURL)
        #expect(player.playbackError == nil)
        #expect(player.playbackNotice?.contains("Track 451") == true)
    }

    @MainActor @Test func finalPrequeuedCorruptFilesRetainATerminalError() async throws {
        let folder = try makeFolder()
        defer { try? FileManager.default.removeItem(at: folder) }
        let firstURL = try makeCAF(in: folder, name: "short.caf", seconds: 0.5)
        let badURLs = [folder.appendingPathComponent("bad-one.caf"), folder.appendingPathComponent("bad-two.caf")]
        for url in badURLs { try Data("Invalid queued audio".utf8).write(to: url) }
        let player = try makePlayer()
        defer { player.stop() }
        player.configureQueue(songs: [song(470, path: firstURL.path), song(471, path: badURLs[0].path), song(472, path: badURLs[1].path)], startIndex: 0)
        player.playQueueEntry(at: 0)
        try await waitUntil { player.playbackError != nil && !player.isLoading }
        #expect(player.currentQueueIndex == 2)
        #expect(player.currentSong?.id == 472)
        #expect(!player.isPlaying)
        #expect(player.playbackError?.contains("Track 472") == true)
        let error = player.playbackError
        try await Task.sleep(for: .milliseconds(100))
        #expect(player.playbackError == error)
    }

    @MainActor @Test func repeatAllUnavailableQueueStopsAfterEachUUIDOnce() async throws {
        let reads = UnavailableReadCounter()
        let player = try makePlayer(resolver: { song in await reads.record(song.id); return nil })
        defer { player.stop() }
        let missing = song(420, path: "/unavailable/\(UUID().uuidString).caf")
        player.configureQueue(songs: [missing, missing], startIndex: 0)
        player.setRepeat(.all)
        player.playQueueEntry(at: 0)
        try await waitUntil { player.playbackError != nil && !player.isLoading }
        #expect(await reads.ids == [420, 420])
        #expect(player.currentQueueIndex == 1)
        #expect(!player.isPlaying)
        let error = player.playbackError
        try await Task.sleep(for: .milliseconds(150))
        #expect(await reads.ids == [420, 420])
        #expect(player.playbackError == error)
    }

    @MainActor @Test func replacingQueueCancelsAnUnavailableContinuation() async throws {
        let folder = try makeFolder()
        defer { try? FileManager.default.removeItem(at: folder) }
        let goodURL = try makeCAF(in: folder)
        let gate = UnavailableReadGate()
        let player = try makePlayer(resolver: { song in
            if song.id == 430 { await gate.wait() }
            return nil
        })
        defer { player.stop() }
        player.configureQueue(songs: [song(430, path: "/missing/old.caf"), song(431, path: goodURL.path)], startIndex: 0)
        player.playQueueEntry(at: 0)
        for _ in 0..<200 {
            if await gate.entered { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(await gate.entered)
        player.configureQueue(songs: [song(432, path: goodURL.path)], startIndex: 0)
        player.playQueueEntry(at: 0)
        await gate.release()
        try await waitUntil { player.currentSong?.id == 432 && player.isPlaying }
        #expect(player.queue.map(\.id) == [432])
        #expect(player.currentQueueIndex == 0)
        #expect(player.playbackNotice == nil)
        #expect(player.playbackError == nil)
    }

    @MainActor @Test func activationFailureDoesNotSkipValidAudio() async throws {
        let folder = try makeFolder()
        defer { try? FileManager.default.removeItem(at: folder) }
        let goodURL = try makeCAF(in: folder)
        let session = AudioSessionController(configure: {}, activate: { throw NSError(domain: "Activation failed", code: 1) })
        let player = try makePlayer(session: session)
        defer { player.stop() }
        player.configureQueue(songs: [song(440, path: goodURL.path), song(441, path: goodURL.path)], startIndex: 0)
        player.playQueueEntry(at: 0)
        try await waitUntil { player.playbackError != nil && !player.isLoading }
        #expect(player.currentSong?.id == 440)
        #expect(player.currentQueueIndex == 0)
        #expect(player.playbackURL == goodURL)
        #expect(player.playbackEngine?.currentItem != nil)
        #expect(player.playbackNotice == nil)
        #expect(!player.isPlaying)
    }

    @MainActor @Test func undoQueueClearPreservesTheLoadingEntryIdentity() async throws {
        let folder = try makeFolder()
        defer { try? FileManager.default.removeItem(at: folder) }
        let goodURL = try makeCAF(in: folder)
        let gate = UnavailableReadGate()
        let player = try makePlayer(resolver: { song in
            if song.id == 460 { await gate.wait() }
            return nil
        })
        defer { player.stop() }
        let queue = [song(460, path: "/missing/loading.caf"), song(461, path: goodURL.path)]
        player.configureQueue(songs: queue, startIndex: 0)
        let loadingID = try #require(player.queueEntryIdentifiers.first)
        player.playQueueEntry(at: 0)
        for _ in 0..<200 {
            if await gate.entered { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(await gate.entered)
        player.clearUpcomingQueue()
        player.restoreQueue(queue)
        #expect(player.queueEntryIdentifiers.first == loadingID)
        await gate.release()
        try await waitUntil { player.currentSong?.id == 461 && player.isPlaying }
        #expect(player.playbackError == nil)
        #expect(player.playbackNotice?.contains("Track 460") == true)
    }

    @MainActor @Test func retryAfterFileRepairReopensTheSameDuplicatePlaylistEntry() async throws {
        let folder = try makeFolder()
        defer { try? FileManager.default.removeItem(at: folder) }
        let repairedURL = folder.appendingPathComponent("repaired.caf")
        let missing = song(480, path: repairedURL.path)
        let player = try makePlayer()
        defer { player.stop() }
        let context = PlaybackContext.playlist(id: 17, name: "Retry list")
        player.configureQueue(songs: [missing, missing], startIndex: 1, context: context)
        let entries = player.queueEntryIdentifiers
        player.playQueueEntry(at: 1)
        try await waitUntil { player.playbackError != nil && !player.isLoading }
        #expect(player.currentQueueIndex == 1)

        let readable = try makeCAF(in: folder)
        try FileManager.default.moveItem(at: readable, to: repairedURL)
        player.playQueueEntry(at: player.currentQueueIndex)
        try await waitUntil { player.isPlaying }

        #expect(player.currentQueueIndex == 1)
        #expect(player.queueEntryIdentifiers == entries)
        #expect(player.playbackContext == context)
        #expect(player.playbackURL == repairedURL)
        #expect(player.playbackError == nil)
    }

    @MainActor private func makePlayer(resolver: PlaybackFileAccess.SourceResolver? = nil, session: AudioSessionController = .shared) throws -> PlayerViewModel {
        let db = try Connection(.inMemory)
        return PlayerViewModel(playlistRepo: try SQLitePlaylistRepository(db: db), playlistSongRepo: try SQLitePlaylistSongRepository(db: db), sourceAccessResolver: resolver, audioSession: session)
    }

    @MainActor private func waitUntil(_ condition: () -> Bool) async throws {
        for _ in 0..<400 {
            if condition() { return }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(condition(), "Playback did not reach the expected terminal state")
    }

    private func makeFolder() throws -> URL {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        return folder
    }

    private func makeCAF(in folder: URL, name: String = "playable.caf", seconds: Double = 3) throws -> URL {
        let url = folder.appendingPathComponent(name)
        let frames = AVAudioFrameCount(48000 * seconds)
        let format = try #require(AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 1))
        let buffer = try #require(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: frames))
        buffer.frameLength = frames
        let samples = try #require(buffer.floatChannelData?[0])
        for index in 0..<Int(frames) { samples[index] = sin(Float(index) * 2 * .pi * 220 / 48000) * 0.01 }
        let file = try AVAudioFile(forWriting: url, settings: format.settings)
        try file.write(from: buffer)
        return url
    }

    private func song(_ id: Int64, path: String) -> Song {
        Song(id: id, songKey: id, artist: "Test", title: "Track \(id)", album: "Test", albumArtist: "Test", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: id, createdAt: Date(), updatedAt: nil, localFilePath: path, fileState: .copied)
    }
}

private actor UnavailableReadCounter {
    private(set) var ids: [Int64] = []
    func record(_ id: Int64?) { if let id { ids.append(id) } }
}

private actor UnavailableReadGate {
    private(set) var entered = false
    private var continuation: CheckedContinuation<Void, Never>?
    func wait() async {
        entered = true
        await withCheckedContinuation { continuation = $0 }
    }
    func release() { continuation?.resume(); continuation = nil }
}
