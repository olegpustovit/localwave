//
//  PlayerViewModel.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import SwiftUI
import os
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

#if canImport(UIKit)
import SwiftUI
import UIKit
#elseif canImport(AppKit)
import AppKit
#endif

#if canImport(MediaPlayer)
import MediaPlayer
#endif


@MainActor
class PlayerViewModel: NSObject, ObservableObject {
    @Published var currentSong: Song?
    @Published private(set) var playbackContext: PlaybackContext = .library
    @Published var isPlaying = false
    @Published var playbackProgress: Double = 0
    @Published var currentTime: String = "0:00"
    @Published var duration: String = "0:00"

    @Published private(set) var isShuffleEnabled: Bool = false
    private var originalQueue: [Song] = []
    private var queueEntryIDs: [UUID] = []
    private var originalEntryIDs: [UUID] = []
    private var queueRevision = 0
    private var applyingRestoredVolume = false
    @Published private(set) var repeatMode: RepeatMode = .none
    @Published var playbackError: String?
    @Published private(set) var playbackNotice: String?
    @Published private(set) var isLoading = false
    @Published private(set) var playbackURL: URL?
    @Published private(set) var playbackFormat = "AUDIO"
    @Published private(set) var playbackFileSize = "—"
    private var loadTask: Task<PreparedAudio, Error>?
    private var loadGeneration = UUID()
    private var countedCurrentLoad = false
    private var measuredDuration: Double = 0
    private struct FileFailureTraversal {
        var visited: Set<UUID> = []
    }
    private var fileFailureTraversal = FileFailureTraversal()
    private var handledFailedItem: ObjectIdentifier?
    private var wasPlayingBeforeInterruption = false
    private let audioSession: AudioSessionController
    private var audioSessionEvents: Task<Void, Never>?
    private var activationTask: Task<Void, Never>?
    private var activationGeneration = UUID()
    private let downloadAheadPreferences: UserDefaults
    private var upcomingCopyTask: Task<Void, Never>?
    private var upcomingCopyGeneration = UUID()
    #if canImport(MediaPlayer)
    private var remoteCommandTargets: [(MPRemoteCommand, Any)] = []
    #endif

    private struct PreparedAudio: Sendable {
        let url: URL
        let renewedBookmark: Data?
        var format = "AUDIO"
        var bytes: Int64 = 0
    }

    private struct QueuedAudio {
        let entryID: UUID?
        let song: Song
        let audio: PreparedAudio
    }
    private var queuedAudio: [ObjectIdentifier: QueuedAudio] = [:]
    private var queuedPlayerItems: [ObjectIdentifier: AVPlayerItem] = [:]
    private var engineObservations: [NSKeyValueObservation] = []
    private var itemObservation: NSKeyValueObservation?
    private var timeObservation: Any?
    private var completionObservation: NSObjectProtocol?
    private var libraryObservation: NSObjectProtocol?
    private var wantsPlayback = false
    private var observedItem: AVPlayerItem?
    var playbackEngine: AVQueuePlayer? { player }


    var queue: [Song] {
        return songs
    }

    var queueEntryIdentifiers: [UUID] { queueEntryIDs }
    var downloadAheadEnabled: Bool { downloadAheadPreferences.object(forKey: "lwDownloadAhead") as? Bool ?? true }

    func setDownloadAhead(_ enabled: Bool) {
        downloadAheadPreferences.set(enabled, forKey: "lwDownloadAhead")
        cancelUpcomingCopies()
        rebuildUpcomingItems()
        scheduleUpcomingCopies()
    }

    var remainingTime: String {
        #if DEBUG && os(iOS)
        if LWReferenceFixture.enabled {
            let parts = currentTime.split(separator: ":").compactMap { Double($0) }
            let elapsed = parts.reduce(0) { $0 * 60 + $1 }
            return "-" + formatTime(max(0, (currentSong?.duration ?? 0) - elapsed))
        }
        #endif
        guard let player else { return "-0:00" }
        return "-" + formatTime(max(0, safeSeconds(player.currentItem?.duration ?? .zero) - safeSeconds(player.currentTime())))
    }

    private let songRepo: SongRepository?
    private let playlistRepo: PlaylistRepository
    private let playlistSongRepo: PlaylistSongRepository
    private let playerPersistenceService: PlayerPersistenceService?
    private let sourceAccessResolver: (@Sendable (Song) async throws -> (root: URL, file: URL)?)?

    func reorderQueue(from source: IndexSet, to destination: Int) {
        guard source.allSatisfy(songs.indices.contains), (0...songs.count).contains(destination) else { return }
        queueRevision += 1
        let currentID = queueEntryIDs.indices.contains(currentQueueIndex) ? queueEntryIDs[currentQueueIndex] : nil
        songs.move(fromOffsets: source, toOffset: destination)
        queueEntryIDs.move(fromOffsets: source, toOffset: destination)
        if let currentID { currentQueueIndex = queueEntryIDs.firstIndex(of: currentID) ?? 0 }
        rebuildUpcomingItems()
        persistQueue()
    }

    func removeFromQueue(at index: Int) {
        guard songs.indices.contains(index), index != currentQueueIndex else { return }
        queueRevision += 1
        songs.remove(at: index)
        let removedID = queueEntryIDs.remove(at: index)
        if let original = originalEntryIDs.firstIndex(of: removedID) {
            originalEntryIDs.remove(at: original)
            originalQueue.remove(at: original)
        }
        if index < currentQueueIndex { currentQueueIndex -= 1 }
        rebuildUpcomingItems()
        persistQueue()
    }

    func clearUpcomingQueue() {
        queueRevision += 1
        songs = Array(songs.prefix(currentQueueIndex + 1))
        queueEntryIDs = Array(queueEntryIDs.prefix(currentQueueIndex + 1))
        let retained = Set(queueEntryIDs)
        let originalIndices = originalEntryIDs.indices.filter { retained.contains(originalEntryIDs[$0]) }
        originalQueue = originalIndices.map { originalQueue[$0] }
        originalEntryIDs = originalIndices.map { originalEntryIDs[$0] }
        rebuildUpcomingItems()
        persistQueue()
    }

    func restoreQueue(_ queue: [Song]) {
        queueRevision += 1
        let currentID = songs.indices.contains(currentQueueIndex) && songs[currentQueueIndex].songKey == currentSong?.songKey && queueEntryIDs.indices.contains(currentQueueIndex) ? queueEntryIDs[currentQueueIndex] : nil
        let index = queue.indices.contains(currentQueueIndex) && queue[currentQueueIndex].songKey == currentSong?.songKey
            ? currentQueueIndex : queue.firstIndex { $0.songKey == currentSong?.songKey } ?? 0
        songs = queue
        queueEntryIDs = queue.map { _ in UUID() }
        if let currentID, queue.indices.contains(index), queue[index].songKey == currentSong?.songKey {
            queueEntryIDs[index] = currentID
        }
        currentQueueIndex = index
        if isShuffleEnabled { originalQueue = queue; originalEntryIDs = queueEntryIDs }
        if let item = player?.currentItem, let old = queuedAudio[ObjectIdentifier(item)], queueEntryIDs.indices.contains(currentQueueIndex) {
            queuedAudio[ObjectIdentifier(item)] = QueuedAudio(entryID: queueEntryIDs[currentQueueIndex], song: old.song, audio: old.audio)
        }
        rebuildUpcomingItems()
        persistQueue()
    }

    private func persistQueue() {
        let savedVolume = volume, savedIndex = currentQueueIndex, savedSongs = songs
        Task {
            await playerPersistenceService?.savePlaybackState(volume: savedVolume, currentIndex: savedIndex, songs: savedSongs)
        }
    }

    func toggleCurrentFavorite() async throws {
        guard let id = currentSong?.id, let songRepo else { return }
        guard let latest = await songRepo.getSongs(ids: [id]).first else { return }
        try await songRepo.toggleSongAsFavorite(songId: id, state: latest.isFavorite != 1)
        guard let refreshed = await songRepo.getSongs(ids: [id]).first else { return }
        if currentSong?.id == id { currentSong = refreshed }
        for index in songs.indices where songs[index].id == id { songs[index] = refreshed }
        for index in originalQueue.indices where originalQueue[index].id == id { originalQueue[index] = refreshed }
        NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
    }

    func createPlaylist(name: String) async throws {
        let newPlaylist = Playlist(id: nil as Int64?, name: name, createdAt: Date(), updatedAt: nil as Date?)
        let createdPlaylist = try await playlistRepo.create(playlist: newPlaylist)
        guard let playlistId = createdPlaylist.id else { return }

        var seen = Set<Int64>()
        for song in songs {
            guard let songId = song.id, seen.insert(songId).inserted else { continue }
            try await playlistSongRepo.addSong(playlistId: playlistId, songId: songId)
        }
        NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
    }

    @Published var volume: Float = 1.0 {
        didSet {
            #if os(macOS)
            player?.volume = volume
            #else
            player?.volume = 1.0
            #endif
            if !applyingRestoredVolume { persistQueue() }
        }
    }

    init(
        playerPersistenceService: PlayerPersistenceService? = nil,
        songRepo: SongRepository? = nil,
        playlistRepo: PlaylistRepository,
        playlistSongRepo: PlaylistSongRepository,
        sourceAccessResolver: (@Sendable (Song) async throws -> (root: URL, file: URL)?)? = nil,
        audioSession: AudioSessionController = .shared,
        downloadAheadPreferences: UserDefaults = LocalWavePreferences.current
    ) {
        self.audioSession = audioSession
        self.downloadAheadPreferences = downloadAheadPreferences
        self.playerPersistenceService = playerPersistenceService
        self.songRepo = songRepo
        self.sourceAccessResolver = sourceAccessResolver
        self.playlistRepo = playlistRepo
        self.playlistSongRepo = playlistSongRepo
        super.init()
        setupRemoteCommands()
        audioSessionEvents = Task { [weak self, audioSession] in
            let stream = await audioSession.events()
            for await event in stream {
                guard let self else { break }
                self.handleAudioSessionEvent(event)
            }
        }
        libraryObservation = NotificationCenter.default.addObserver(forName: Notification.Name("LibraryRefresh"), object: nil, queue: .main) { [weak self] _ in
            Task { @MainActor [weak self] in await self?.refreshUpcomingFromRepository() }
        }

        let restorationRevision = queueRevision
        Task {
            let storedVolume = await self.playerPersistenceService?.getVolume()
            let restored = await self.playerPersistenceService?.restore()
            guard queueRevision == restorationRevision else { return }
            if let storedVolume {
                applyingRestoredVolume = true
                volume = storedVolume
                applyingRestoredVolume = false
            }
            if let (restoredSongs, restoredIndex, restoredSong) = restored {
                if let data = LocalWavePreferences.current.data(forKey: "lwPlaybackContext"), let context = try? JSONDecoder().decode(PlaybackContext.self, from: data) { playbackContext = context }
                songs = restoredSongs
                queueEntryIDs = restoredSongs.map { _ in UUID() }
                currentQueueIndex = songs.indices.contains(restoredIndex) ? restoredIndex : 0
                currentSong = songs.indices.contains(currentQueueIndex) ? songs[currentQueueIndex] : restoredSong
                if let currentSong, currentSong.localFilePath != nil { beginLoad(currentSong, autoplay: false, queueIndex: currentQueueIndex) }
                updateNowPlayingInfo()
            }
        }
    }

    private var player: AVQueuePlayer?
    @Published private var songs: [Song] = []
    @Published private(set) var currentQueueIndex: Int = 0

    func addToQueue(_ song: Song) {
        queueRevision += 1
        let entryID = UUID()
        songs.append(song)
        queueEntryIDs.append(entryID)
        if isShuffleEnabled { originalQueue.append(song); originalEntryIDs.append(entryID) }
        if currentSong == nil {
            currentQueueIndex = 0
            currentSong = songs.first
            if let currentSong, currentSong.localFilePath != nil { beginLoad(currentSong, autoplay: false, queueIndex: 0) }
        }
        rebuildUpcomingItems()
        persistQueue()
    }

    func configureQueue(songs: [Song], startIndex: Int, context: PlaybackContext = .library) {
        playbackContext = context
        if let data = try? JSONEncoder().encode(context) { LocalWavePreferences.current.set(data, forKey: "lwPlaybackContext") }
        queueRevision += 1
        stop()
        self.songs = songs
        queueEntryIDs = songs.map { _ in UUID() }
        originalQueue = []
        originalEntryIDs = []
        isShuffleEnabled = false
        currentQueueIndex = songs.indices.contains(startIndex) ? startIndex : 0
        currentSong = songs.indices.contains(currentQueueIndex) ? songs[currentQueueIndex] : nil
        rebuildUpcomingItems()
        persistQueue()
    }

    func playQueueEntry(at index: Int) {
        guard songs.indices.contains(index) else { return }
        fileFailureTraversal = FileFailureTraversal()
        playbackNotice = nil
        queueRevision += 1
        if let player, let next = player.items().dropFirst().first,
           queuedAudio[ObjectIdentifier(next)]?.entryID == queueEntryIDs[index] {
            wantsPlayback = true
            player.advanceToNextItem()
            adoptCurrentItem()
            play()
        } else { beginLoad(songs[index], autoplay: true, queueIndex: index) }
    }

    var logger = Logger(subsystem: subsystem, category: "PlayerViewModel")

    private func setupRemoteCommands() {
        #if canImport(MediaPlayer)
        let commandCenter = MPRemoteCommandCenter.shared()
        registerRemoteCommand(commandCenter.playCommand) { [weak self] _ in
            self?.play()
            return .success
        }
        registerRemoteCommand(commandCenter.pauseCommand) { [weak self] _ in
            self?.pause()
            return .success
        }

        registerRemoteCommand(commandCenter.togglePlayPauseCommand) { [weak self] _ in
            self?.playPause()
            return .success
        }

        registerRemoteCommand(commandCenter.nextTrackCommand) { [weak self] _ in
            self?.nextSong()
            return .success
        }

        registerRemoteCommand(commandCenter.previousTrackCommand) { [weak self] _ in
            self?.previousSong()
            return .success
        }
        registerRemoteCommand(commandCenter.changePlaybackPositionCommand) { [weak self] event in
            if let event = event as? MPChangePlaybackPositionCommandEvent {
                self?.seek(to: event.positionTime)
            }
            return .success
        }

        // Enable commands
        commandCenter.playCommand.isEnabled = true
        commandCenter.pauseCommand.isEnabled = true
        commandCenter.nextTrackCommand.isEnabled = true
        commandCenter.togglePlayPauseCommand.isEnabled = true
        commandCenter.previousTrackCommand.isEnabled = true
        commandCenter.changePlaybackPositionCommand.isEnabled = true
        #endif
    }

    #if canImport(MediaPlayer)
    private func registerRemoteCommand(_ command: MPRemoteCommand, handler: @escaping (MPRemoteCommandEvent) -> MPRemoteCommandHandlerStatus) {
        let token = command.addTarget(handler: handler)
        remoteCommandTargets.append((command, token))
    }
    #endif

    func updateNowPlayingInfo() {
        #if canImport(MediaPlayer)
        guard let song = currentSong, let player = player else { return }

        var nowPlayingInfo = [String: Any]()
        nowPlayingInfo[MPMediaItemPropertyTitle] = song.title
        nowPlayingInfo[MPMediaItemPropertyArtist] = song.artist
        nowPlayingInfo[MPMediaItemPropertyAlbumTitle] = song.album
        nowPlayingInfo[MPNowPlayingInfoPropertyElapsedPlaybackTime] = safeSeconds(player.currentTime())
        nowPlayingInfo[MPMediaItemPropertyPlaybackDuration] = safeSeconds(player.currentItem?.duration ?? .zero)
        nowPlayingInfo[MPNowPlayingInfoPropertyPlaybackRate] = isPlaying ? player.rate : 0.0

        if let artwork = loadCoverArt(for: song) {
            let mpArtwork = MPMediaItemArtwork(boundsSize: artwork.size) { _ in artwork }
            nowPlayingInfo[MPMediaItemPropertyArtwork] = mpArtwork
        }

        MPNowPlayingInfoCenter.default().nowPlayingInfo = nowPlayingInfo
        #endif
    }

    private func handleAudioSessionEvent(_ event: AudioSessionController.Event) {
        switch event {
        case .interruptionBegan:
            let shouldResume = isPlaying || wantsPlayback
            if isLoading { cancelLoading() }
            pause()
            wasPlayingBeforeInterruption = shouldResume
        case let .interruptionEnded(shouldResume):
            let resume = shouldResume && wasPlayingBeforeInterruption
            wasPlayingBeforeInterruption = false
            if resume { play() }
        case .oldOutputUnavailable:
            if isLoading { cancelLoading() }
            pause()
        }
    }

    private func beginLoad(_ song: Song, autoplay: Bool, queueIndex: Int? = nil, failures: FileFailureTraversal? = nil) {
        stop()
        fileFailureTraversal = failures ?? FileFailureTraversal()
        if failures == nil { playbackNotice = nil }
        currentTime = "0:00"
        duration = "0:00"
        measuredDuration = 0
        playbackError = nil
        currentSong = song
        if let id = song.id {
            withOfflineCacheLock { LocalWavePreferences.current.set(String(id), forKey: "lwActiveSongID") }
        }
        countedCurrentLoad = false
        if let queueIndex { currentQueueIndex = queueIndex }
        else if !songs.indices.contains(currentQueueIndex) || songs[currentQueueIndex].songKey != song.songKey {
            currentQueueIndex = songs.firstIndex { $0.songKey == song.songKey } ?? 0
        }
        isLoading = true
        let generation = loadGeneration
        let entryID = queueEntryIDs.indices.contains(currentQueueIndex) && songs[currentQueueIndex].songKey == song.songKey ? queueEntryIDs[currentQueueIndex] : nil
        let resolver = sourceAccessResolver
        let worker = Task.detached(priority: .userInitiated) {
            let source = try await PlaybackFileAccess.prepare(song, resolver: resolver)
            var prepared = PreparedAudio(url: source.url, renewedBookmark: source.renewedBookmark)
            let playable = try await AVURLAsset(url: prepared.url).load(.isPlayable)
            guard playable else { throw CustomError.genericError("The audio file could not be decoded.") }
            if let audio = try? AVAudioFile(forReading: prepared.url) {
                prepared.format = prepared.url.pathExtension.uppercased() + " · " + String(format: "%g kHz", audio.processingFormat.sampleRate / 1000)
            } else { prepared.format = prepared.url.pathExtension.uppercased() }
            prepared.bytes = Int64((try? prepared.url.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? 0)
            return prepared
        }
        loadTask = worker
        Task {
            do {
                let prepared = try await worker.value
                guard loadGeneration == generation else { return }
                let item = AVPlayerItem(url: prepared.url)
                queuedAudio[ObjectIdentifier(item)] = QueuedAudio(entryID: entryID, song: song, audio: prepared)
                queuedPlayerItems[ObjectIdentifier(item)] = item
                wantsPlayback = autoplay
                installEngine(firstItem: item)
                loadTask = nil
                adoptCurrentItem()
                guard loadGeneration == generation else { return }
                if let bookmark = prepared.renewedBookmark, let id = song.id {
                    Task {
                        do { try await songRepo?.updateBookmark(songId: id, bookmark: bookmark) }
                        catch { logger.error("Could not save renewed bookmark: \(error)") }
                    }
                }
                if autoplay { play() }
                persistQueue()
                let musicDirectory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("Music", isDirectory: true)
                if prepared.url.deletingLastPathComponent().standardizedFileURL == musicDirectory.standardizedFileURL {
                    Task {
                        do {
                            guard let id = song.id, let songRepo,
                                  let latest = await songRepo.getSongs(ids: [id]).first else { return }
                            let changed = try await songRepo.updateCachedFile(songId: id, localFilePath: "Music/" + prepared.url.lastPathComponent, state: .copied, expectedState: latest.fileState, expectedPath: latest.localFilePath)
                            if changed, let updated = await songRepo.getSongs(ids: [id]).first {
                                if loadGeneration == generation, currentSong?.id == id, playbackURL == prepared.url { currentSong = updated }
                                for index in songs.indices where songs[index].id == id { songs[index] = updated }
                                for index in originalQueue.indices where originalQueue[index].id == id { originalQueue[index] = updated }
                                NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
                                if let sourceID = updated.sourceId {
                                    NotificationCenter.default.post(name: Notification.Name("LWSourceMaterialized"), object: sourceID)
                                }
                            }
                        } catch { logger.error("Could not save playback cache: \(error)") }
                    }
                }
            } catch {
                guard loadGeneration == generation else { return }
                isLoading = false
                loadTask = nil
                logger.error("Playback load failed: \(error)")
                handleFileFailure(song: song, entryID: entryID, message: error.localizedDescription, autoplay: autoplay)
            }
        }
    }

    func dismissPlaybackNotice() { playbackNotice = nil }

    private func handleFileFailure(song: Song, entryID: UUID?, message: String, autoplay: Bool) {
        markFileFailed(song)
        let detail = "Could not open “\(song.title)”. \(message) Reconnect its source in Folders if access was removed."
        guard autoplay, let entryID, let failedIndex = queueEntryIDs.firstIndex(of: entryID) else {
            playbackError = detail
            return
        }
        fileFailureTraversal.visited.insert(entryID)
        let following = Array(songs.indices.dropFirst(failedIndex + 1))
        let candidates = repeatMode == .none ? following : following + Array(songs.indices.prefix(failedIndex))
        guard let next = candidates.first(where: { !fileFailureTraversal.visited.contains(queueEntryIDs[$0]) }) else {
            wantsPlayback = false
            isPlaying = false
            isLoading = false
            player?.pause()
            playbackError = detail
            persistQueue()
            return
        }
        let traversal = fileFailureTraversal
        beginLoad(songs[next], autoplay: true, queueIndex: next, failures: traversal)
        playbackNotice = "Skipped “\(song.title)”: its audio file could not be opened."
    }

    private func markFileFailed(_ song: Song) {
        Task {
            guard let id = song.id, let songRepo,
                  let latest = await songRepo.getSongs(ids: [id]).first else { return }
            do {
                _ = try await songRepo.updateCachedFile(songId: id, localFilePath: latest.localFilePath, state: .failed, expectedState: latest.fileState, expectedPath: latest.localFilePath)
                NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
            } catch { logger.error("Could not save file failure state: \(error)") }
        }
    }

    func cancelLoading() { stop() }

    private func cancelPendingLoad() {
        loadGeneration = UUID()
        loadTask?.cancel()
        loadTask = nil
        isLoading = false
    }

    func playSong(_ song: Song) {
        queueRevision += 1
        beginLoad(song, autoplay: true)
    }

    func playPause() {
        if isLoading { cancelLoading(); return }
        if isPlaying {
            pause()
        } else {
            play()
        }
    }

    private func play() {
        guard let player, player.currentItem != nil else {
            if let currentSong { beginLoad(currentSong, autoplay: true) }
            return
        }
        if player.currentItem?.status == .failed {
            if let currentSong { beginLoad(currentSong, autoplay: true) }
            return
        }
        activationTask?.cancel()
        activationGeneration = UUID()
        let activation = activationGeneration
        let generation = loadGeneration
        playbackError = nil
        wantsPlayback = true
        activationTask = Task { [weak self, audioSession] in
            do {
                try await audioSession.activate()
                guard let self, !Task.isCancelled, self.activationGeneration == activation,
                      self.loadGeneration == generation, self.wantsPlayback, self.player === player else { return }
                self.activationTask = nil
                player.play()
                self.syncEngineState()
            } catch is CancellationError { }
            catch {
                guard let self, self.activationGeneration == activation else { return }
                self.activationTask = nil
                self.wantsPlayback = false
                self.playbackError = error.localizedDescription
                self.syncEngineState()
            }
        }
        syncEngineState()
    }

    private func cancelActivation() {
        activationGeneration = UUID()
        activationTask?.cancel()
        activationTask = nil
    }

    private func recordPlaybackStart() {
        fileFailureTraversal = FileFailureTraversal()
        guard !countedCurrentLoad, let id = currentSong?.id else { return }
        countedCurrentLoad = true
        if let url = playbackURL {
            let musicDirectory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("Music", isDirectory: true)
            if url.deletingLastPathComponent().standardizedFileURL == musicDirectory.standardizedFileURL {
                try? FileManager.default.setAttributes([.modificationDate: Date()], ofItemAtPath: url.path)
            }
        }
        let key = String(id)
        var counts = LocalWavePreferences.current.dictionary(forKey: "lwPlayCounts") as? [String: Int] ?? [:]
        counts[key, default: 0] += 1
        LocalWavePreferences.current.set(counts, forKey: "lwPlayCounts")
        var dates = LocalWavePreferences.current.dictionary(forKey: "lwLastPlayed") as? [String: Double] ?? [:]
        dates[key] = Date().timeIntervalSince1970
        LocalWavePreferences.current.set(dates, forKey: "lwLastPlayed")
        scheduleUpcomingCopies()
        NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
    }

    private func cancelUpcomingCopies() {
        upcomingCopyGeneration = UUID()
        upcomingCopyTask?.cancel()
        upcomingCopyTask = nil
    }

    private func scheduleUpcomingCopies() {
        cancelUpcomingCopies()
        guard player?.currentItem != nil, let songRepo else { return }
        let upcoming = upcomingIndices().map { songs[$0] }
        let generation = upcomingCopyGeneration
        let revision = queueRevision
        let logger = logger
        upcomingCopyTask = Task { [weak self] in
            for next in upcoming where next.fileState != .copied {
                guard !Task.isCancelled,
                      self?.upcomingCopyGeneration == generation, self?.queueRevision == revision else { return }
                guard self?.downloadAheadEnabled == true || next.sourceAvailability == .onDevice else { continue }
                guard let nextID = next.id else { continue }
                do {
                    try await songRepo.markSongForCopy(songId: nextID)
                    NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
                } catch { logger.error("Could not queue prefetch: \(error)") }
            }
        }
    }

    private func pause() {
        if loadTask != nil { stop(); return }
        fileFailureTraversal = FileFailureTraversal()
        wasPlayingBeforeInterruption = false
        cancelActivation()
        wantsPlayback = false
        player?.pause()
        syncEngineState()
    }

    func stop() {
        fileFailureTraversal = FileFailureTraversal()
        handledFailedItem = nil
        cancelUpcomingCopies()
        wasPlayingBeforeInterruption = false
        cancelActivation()
        cancelPendingLoad()
        wantsPlayback = false
        engineObservations = []
        itemObservation = nil
        observedItem = nil
        if let timeObservation { player?.removeTimeObserver(timeObservation) }
        timeObservation = nil
        if let completionObservation { NotificationCenter.default.removeObserver(completionObservation) }
        completionObservation = nil
        player?.pause()
        player?.removeAllItems()
        player = nil
        queuedAudio = [:]
        queuedPlayerItems = [:]
        playbackURL = nil
        playbackFormat = "AUDIO"
        playbackFileSize = "—"
        withOfflineCacheLock {
            LocalWavePreferences.current.removeObject(forKey: "lwActiveSongID")
            LocalWavePreferences.current.removeObject(forKey: "lwActiveCachePath")
            LocalWavePreferences.current.removeObject(forKey: "lwQueuedCachePaths")
        }
        isPlaying = false
        isLoading = false
        playbackProgress = 0
        currentTime = "0:00"
        measuredDuration = 0
        #if canImport(MediaPlayer)
        MPNowPlayingInfoCenter.default().nowPlayingInfo = nil
        #endif
    }

    private func installEngine(firstItem: AVPlayerItem) {
        let engine = AVQueuePlayer(items: [firstItem])
        engine.actionAtItemEnd = .advance
        #if os(macOS)
        engine.volume = volume
        #else
        engine.volume = 1
        #endif
        player = engine
        engineObservations = [
            engine.observe(\.currentItem, options: [.initial, .new]) { [weak self, weak engine] _, _ in
                Task { @MainActor [weak self, weak engine] in
                    guard let self, let engine, self.player === engine else { return }
                    self.adoptCurrentItem()
                }
            },
            engine.observe(\.timeControlStatus, options: [.initial, .new]) { [weak self, weak engine] _, _ in
                Task { @MainActor [weak self, weak engine] in
                    guard let self, let engine, self.player === engine else { return }
                    self.syncEngineState()
                }
            }
        ]
        timeObservation = engine.addPeriodicTimeObserver(forInterval: CMTime(seconds: 0.25, preferredTimescale: 600), queue: .main) { [weak self] _ in
            Task { @MainActor [weak self] in self?.updateTimeDisplay() }
        }
        completionObservation = NotificationCenter.default.addObserver(forName: .AVPlayerItemDidPlayToEndTime, object: nil, queue: .main) { [weak self] notification in
            guard let item = notification.object as? AVPlayerItem else { return }
            Task { @MainActor [weak self] in
                guard let self, self.queuedAudio[ObjectIdentifier(item)] != nil else { return }
                if self.player?.currentItem == nil, self.wantsPlayback {
                    if self.repeatMode == .none, self.currentQueueIndex >= self.songs.count - 1 {
                        self.finishCurrentTrack(item)
                    } else { self.nextSong() }
                } else { self.adoptCurrentItem() }
                self.queuedAudio.removeValue(forKey: ObjectIdentifier(item))
                self.queuedPlayerItems.removeValue(forKey: ObjectIdentifier(item))
            }
        }
    }

    private func finishCurrentTrack(_ item: AVPlayerItem) {
        let itemDuration = safeSeconds(item.duration)
        let total = itemDuration > 0 ? itemDuration : measuredDuration > 0 ? measuredDuration : storedDuration(currentSong)
        let url = playbackURL
        let format = playbackFormat
        let size = playbackFileSize
        stop()
        playbackURL = url
        playbackFormat = format
        playbackFileSize = size != "—" ? size : currentSong?.fileSize.map { ByteCountFormatter.string(fromByteCount: $0, countStyle: .file) } ?? "—"
        measuredDuration = total
        duration = formatTime(total)
        currentTime = duration
        playbackProgress = total > 0 ? 1 : 0
    }

    private func storedDuration(_ song: Song?) -> Double {
        guard let value = song?.duration, value.isFinite, value > 0 else { return 0 }
        return value
    }

    private func adoptCurrentItem() {
        if let engine = player { reportRetiredQueueFailures(in: engine) }
        guard let item = player?.currentItem,
              let entry = queuedAudio[ObjectIdentifier(item)] else { syncEngineState(); return }
        if observedItem !== item {
            observedItem = item
            handledFailedItem = nil
            countedCurrentLoad = false
            measuredDuration = storedDuration(entry.song)
            if let id = entry.entryID, let index = queueEntryIDs.firstIndex(of: id) {
                currentQueueIndex = index
                currentSong = songs[index]
            } else { currentSong = entry.song }
            playbackURL = entry.audio.url
            playbackFormat = entry.audio.format
            let bytes = entry.audio.bytes > 0 ? entry.audio.bytes : currentSong?.fileSize ?? 0
            playbackFileSize = bytes > 0 ? ByteCountFormatter.string(fromByteCount: bytes, countStyle: .file) : "—"
            withOfflineCacheLock {
                if let id = currentSong?.id { LocalWavePreferences.current.set(String(id), forKey: "lwActiveSongID") }
                LocalWavePreferences.current.set(entry.audio.url.resolvingSymlinksInPath().standardizedFileURL.path, forKey: "lwActiveCachePath")
            }
            itemObservation = item.observe(\.status, options: [.initial, .new]) { [weak self, weak item] _, _ in
                Task { @MainActor [weak self, weak item] in
                    guard let self, let item, self.player?.currentItem === item else { return }
                    self.syncEngineState()
                }
            }
            rebuildUpcomingItems()
            persistQueue()
        }
        updateTimeDisplay()
        syncEngineState()
    }

    private func reportRetiredQueueFailures(in engine: AVQueuePlayer) {
        // AVQueuePlayer can retire a failed upcoming item before current-item KVO observes it.
        // Retain the bounded native queue's items until adoption so that its real status survives.
        let active = Set(engine.items().map(ObjectIdentifier.init))
        let hasCurrent = engine.currentItem != nil
        for (identity, item) in Array(queuedPlayerItems) where !active.contains(identity) {
            let failed = item.status == .failed
            // Preserve the final completed item until its end notification snapshots its metadata.
            guard failed || hasCurrent else { continue }
            let entry = queuedAudio.removeValue(forKey: identity)
            queuedPlayerItems.removeValue(forKey: identity)
            guard failed, let entry, let id = entry.entryID, queueEntryIDs.contains(id) else { continue }
            markFileFailed(entry.song)
            playbackNotice = "Skipped “\(entry.song.title)”: its audio file could not be opened."
        }
    }

    private func syncEngineState() {
        guard let engine = player, let item = engine.currentItem else {
            isPlaying = false
            isLoading = loadTask != nil
            return
        }
        isPlaying = item.status == .readyToPlay && engine.timeControlStatus == .playing
        isLoading = loadTask != nil || activationTask != nil || item.status == .unknown || (wantsPlayback && engine.timeControlStatus == .waitingToPlayAtSpecifiedRate)
        if item.status == .failed {
            isLoading = false
            isPlaying = false
            let autoplay = wantsPlayback
            wantsPlayback = false
            engine.pause()
            guard handledFailedItem != ObjectIdentifier(item) else { return }
            handledFailedItem = ObjectIdentifier(item)
            if let entry = queuedAudio[ObjectIdentifier(item)] {
                handleFileFailure(song: entry.song, entryID: entry.entryID, message: item.error?.localizedDescription ?? "The audio file could not be decoded.", autoplay: autoplay)
            }
            return
        }
        if isPlaying { recordPlaybackStart() }
        updateTimeDisplay()
    }

    private func upcomingIndices() -> [Int] {
        guard songs.indices.contains(currentQueueIndex) else { return [] }
        switch repeatMode {
        case .one: return [currentQueueIndex, currentQueueIndex]
        case .all: return (1...2).map { (currentQueueIndex + $0) % songs.count }
        case .none: return Array(songs.indices.dropFirst(currentQueueIndex + 1).prefix(2))
        }
    }

    private func rebuildUpcomingItems() {
        guard let engine = player, let current = engine.currentItem else { return }
        var desired: [QueuedAudio] = []
        withOfflineCacheLock {
            for index in upcomingIndices() {
                let song = songs[index]
                guard let path = song.localFilePath else { break }
                let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
                let url = path.hasPrefix("/") ? URL(fileURLWithPath: path) : docs.appendingPathComponent(path)
                guard FileManager.default.isReadableFile(atPath: url.path) else { break }
                let bytes = Int64((try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? 0)
                let audio = PreparedAudio(url: url, renewedBookmark: nil, format: url.pathExtension.uppercased(), bytes: bytes)
                desired.append(QueuedAudio(entryID: queueEntryIDs[index], song: song, audio: audio))
            }
            let paths = ([playbackURL].compactMap { $0 } + desired.map { $0.audio.url }).map { $0.resolvingSymlinksInPath().standardizedFileURL.path }
            LocalWavePreferences.current.set(paths, forKey: "lwQueuedCachePaths")
        }
        let existing = Array(engine.items().dropFirst())
        let unchanged = existing.count == desired.count && zip(existing, desired).allSatisfy { item, wanted in
            guard let actual = queuedAudio[ObjectIdentifier(item)] else { return false }
            return actual.entryID == wanted.entryID && actual.audio.url == wanted.audio.url
        }
        if !unchanged {
            for item in existing {
                engine.remove(item)
                queuedAudio.removeValue(forKey: ObjectIdentifier(item))
                queuedPlayerItems.removeValue(forKey: ObjectIdentifier(item))
            }
            var previous = current
            for entry in desired {
                let item = AVPlayerItem(url: entry.audio.url)
                guard engine.canInsert(item, after: previous) else { break }
                queuedAudio[ObjectIdentifier(item)] = entry
                queuedPlayerItems[ObjectIdentifier(item)] = item
                engine.insert(item, after: previous)
                previous = item
            }
        }
        let paths = engine.items().compactMap { queuedAudio[ObjectIdentifier($0)]?.audio.url.resolvingSymlinksInPath().standardizedFileURL.path }
        withOfflineCacheLock { LocalWavePreferences.current.set(paths, forKey: "lwQueuedCachePaths") }
    }

    private func refreshUpcomingFromRepository() async {
        let revision = queueRevision
        if case let .playlist(id, _) = playbackContext,
           let list = try? await playlistRepo.getOne(id: id), queueRevision == revision {
            playbackContext = .playlist(id: id, name: list.name)
        }
        guard let songRepo, queueRevision == revision else { return }
        let ids = Array(Set(upcomingIndices().compactMap { songs[$0].id } + [currentSong?.id].compactMap { $0 }))
        let refreshed = await songRepo.getSongs(ids: ids)
        guard queueRevision == revision else { return }
        for song in refreshed {
            for index in songs.indices where songs[index].id == song.id { songs[index] = song }
            for index in originalQueue.indices where originalQueue[index].id == song.id { originalQueue[index] = song }
            if currentSong?.id == song.id { currentSong = song }
        }
        let byID = Dictionary(refreshed.compactMap { song in song.id.map { ($0, song) } }, uniquingKeysWith: { first, _ in first })
        queuedAudio = queuedAudio.mapValues { entry in
            guard let id = entry.song.id, let updated = byID[id] else { return entry }
            return QueuedAudio(entryID: entry.entryID, song: updated, audio: entry.audio)
        }
        rebuildUpcomingItems()
        updateNowPlayingInfo()
    }

    func previousSong() {
        guard !songs.isEmpty else { return }
        currentQueueIndex = (currentQueueIndex - 1 + songs.count) % songs.count
        playQueueEntry(at: currentQueueIndex)
        Task {
            await self.playerPersistenceService?.savePlaybackState(
                volume: self.volume, currentIndex: self.currentQueueIndex, songs: self.songs
            )
        }
    }

    func nextSong() {
        guard !songs.isEmpty else { return }
        let isPlayingLastSong = currentQueueIndex == songs.count - 1
        switch repeatMode {
        case .none:
            if (isPlayingLastSong) {
                stop()
            } else {
                currentQueueIndex += 1
                playQueueEntry(at: currentQueueIndex)
            }
        case .all:
            currentQueueIndex = isPlayingLastSong ? 0 : currentQueueIndex + 1
            playQueueEntry(at: currentQueueIndex)
        case .one:
            playQueueEntry(at: currentQueueIndex)
        }
        Task {
            await self.playerPersistenceService?.savePlaybackState(
                volume: self.volume, currentIndex: self.currentQueueIndex, songs: self.songs
            )
        }
    }

    func seek(to progress: Double) {
        guard let player = player else { return }

        let value = max(0, min(safeSeconds(player.currentItem?.duration ?? .zero), progress))
        player.seek(to: CMTime(seconds: value, preferredTimescale: 600), toleranceBefore: .zero, toleranceAfter: .zero) { [weak self] _ in
            Task { @MainActor [weak self] in self?.updateTimeDisplay() }
        }
    }

    func setShuffle(_ enabled: Bool) {
        guard enabled != isShuffleEnabled else { return }
        queueRevision += 1
        let currentID = queueEntryIDs.indices.contains(currentQueueIndex) ? queueEntryIDs[currentQueueIndex] : nil
        if enabled {
            originalQueue = songs
            originalEntryIDs = queueEntryIDs
            var order = songs.indices.filter { $0 != currentQueueIndex }.shuffled()
            if songs.indices.contains(currentQueueIndex) { order.insert(currentQueueIndex, at: 0) }
            songs = order.map { songs[$0] }
            queueEntryIDs = order.map { queueEntryIDs[$0] }
        } else {
            songs = originalQueue
            queueEntryIDs = originalEntryIDs
            originalQueue = []
            originalEntryIDs = []
        }
        currentQueueIndex = currentID.flatMap { queueEntryIDs.firstIndex(of: $0) } ?? 0
        isShuffleEnabled = enabled
        updateNowPlayingInfo()
        rebuildUpcomingItems()
        persistQueue()
    }

    func setRepeat(_ enabled: RepeatMode) {
        repeatMode = enabled
        rebuildUpcomingItems()
    }

    func seekBySeconds(_ seconds: Double) {
        guard let player else { return }
        seek(to: safeSeconds(player.currentTime()) + seconds)
    }

    func seekByFraction(_ fraction: Double) {
        guard let player else { return }
        seek(to: max(0, min(1, fraction)) * safeSeconds(player.currentItem?.duration ?? .zero))
    }

    private func updateTimeDisplay() {
        guard let player, let item = player.currentItem else { return }
        let elapsed = safeSeconds(player.currentTime())
        let itemDuration = safeSeconds(item.duration)
        let total = itemDuration > 0 ? itemDuration : measuredDuration > 0 ? measuredDuration : storedDuration(currentSong)
        measuredDuration = total
        playbackProgress = total > 0 ? max(0, min(1, elapsed / total)) : 0
        currentTime = formatTime(elapsed)
        duration = formatTime(total)
        updateNowPlayingInfo()
    }

    private func safeSeconds(_ time: CMTime) -> Double {
        let seconds = time.seconds
        return seconds.isFinite && seconds >= 0 ? seconds : 0
    }

    private func formatTime(_ time: TimeInterval) -> String {
        let minutes = Int(time) / 60
        let seconds = Int(time) % 60
        return String(format: "%d:%02d", minutes, seconds)
    }

    deinit {
        upcomingCopyTask?.cancel()
        activationTask?.cancel()
        audioSessionEvents?.cancel()
        if let libraryObservation { NotificationCenter.default.removeObserver(libraryObservation) }
        #if canImport(MediaPlayer)
        for (command, token) in remoteCommandTargets { command.removeTarget(token) }
        #endif
    }

    #if canImport(MediaPlayer)
    // Helper function to load cover art for a song (iOS-specific for Now Playing)
    private func loadCoverArt(for song: Song) -> PlatformImage? {
        guard let path = song.coverArtPath else { return nil }
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
        let url = docs.appendingPathComponent(path)
        return PlatformImage(contentsOfFile: url.path)
    }
    #endif
}

#if DEBUG
extension PlayerViewModel {
    func applyReferenceSnapshot(queue: [Song], index: Int, elapsed: Double, volume: Double) {
        stop()
        configureQueue(songs: queue, startIndex: index, context: .album(name: queue[index].album, artist: queue[index].artist))
        currentSong = queue[index]
        currentTime = formatTime(elapsed)
        duration = formatTime(queue[index].duration ?? 0)
        playbackProgress = elapsed / max(1, queue[index].duration ?? 1)
        playbackFormat = queue[index].fileExtension?.uppercased() ?? "FLAC"
        playbackFileSize = ByteCountFormatter.string(fromByteCount: queue[index].fileSize ?? 0, countStyle: .file)
        self.volume = Float(volume)
        isPlaying = false
        isLoading = false
    }
}
#endif
