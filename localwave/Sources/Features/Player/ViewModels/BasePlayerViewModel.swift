//
//  BasePlayerViewModel.swift
//  localwave
//
//  Created by Claude Code on 04.10.2025.
//

import Combine
import Foundation
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// Platform-agnostic base player view model
/// Contains all business logic that can be shared between iOS and TUI
@MainActor
public class BasePlayerViewModel: ObservableObject, AudioPlayerDelegate {
    // MARK: - Published Properties

    @Published public var currentSong: Song?
    @Published public var isPlaying = false
    @Published public var playbackProgress: Double = 0
    @Published public var currentTime: String = "0:00"
    @Published public var duration: String = "0:00"
    @Published public private(set) var isLoading = false
    @Published public private(set) var playbackError: String?

    @Published public var volume: Float = 1.0 {
        didSet {
            // Apply logarithmic curve for better human perception
            let actualVolume = volume * volume
            player.volume = actualVolume
            guard !applyingRestoredVolume else { return }
            Task {
                await playerPersistenceService?.savePlaybackState(
                    volume: volume, currentIndex: currentIndex, songs: songs
                )
            }
        }
    }

    // MARK: - Internal State

    public var isShuffleEnabled: Bool = false
    internal var originalQueue: [Song] = []
    public var repeatMode: RepeatMode = .none

    public var queue: [Song] {
        return songs
    }

    // MARK: - Dependencies

    internal let songRepo: SongRepository?
    internal let playlistRepo: PlaylistRepository
    internal let playlistSongRepo: PlaylistSongRepository
    internal let playerPersistenceService: PlayerPersistenceService?
    internal let player: AudioPlayerProtocol

    internal var timer: Timer?
    internal var dispatchTimer: DispatchSourceTimer?
    internal var songs: [Song] = []
    internal var currentIndex: Int = 0
    internal var cacheDirectory: URL?

    internal let logger: AppLogger
    private let sourceAccessResolver: PlaybackFileAccess.SourceResolver?
    private var loadTask: Task<Void, Never>?
    private var restorationTask: Task<Void, Never>?
    private var loadGeneration = UUID()
    private var wantsPlayback = false
    private var playbackReady = false
    private var applyingRestoredVolume = false

    // MARK: - Initialization

    public init(
        player: AudioPlayerProtocol,
        playerPersistenceService: PlayerPersistenceService? = nil,
        songRepo: SongRepository? = nil,
        playlistRepo: PlaylistRepository,
        playlistSongRepo: PlaylistSongRepository,
        logger: AppLogger? = nil,
        sourceAccessResolver: PlaybackFileAccess.SourceResolver? = nil
    ) {
        self.player = player
        self.playerPersistenceService = playerPersistenceService
        self.songRepo = songRepo
        self.playlistRepo = playlistRepo
        self.playlistSongRepo = playlistSongRepo
        self.sourceAccessResolver = sourceAccessResolver

        // Use provided logger or create platform-appropriate logger
        self.logger = logger ?? createLogger(subsystem: subsystem, category: "BasePlayerViewModel")

        self.player.delegate = self

        let startupGeneration = loadGeneration
        restorationTask = Task {
            // Restore volume
            if let storedVolume = await self.playerPersistenceService?.getVolume() {
                guard loadGeneration == startupGeneration, !Task.isCancelled else { return }
                applyingRestoredVolume = true
                self.volume = storedVolume
                applyingRestoredVolume = false
            }

            // Restore queue and current song
            if let (songs, currentIndex, currentSong) = await self.playerPersistenceService?.restore() {
                guard loadGeneration == startupGeneration, !Task.isCancelled else { return }
                self.songs = songs
                self.currentIndex = currentIndex
                self.currentSong = currentSong

                restorationTask = nil
                if let currentSong = currentSong {
                    stopAndPreloadSong(currentSong)
                }
            }
        }
    }

    // MARK: - AudioPlayerDelegate

    public func audioPlayerDidFinishPlaying(_ player: AudioPlayerProtocol, successfully: Bool) {
        nextSong()
    }

    // MARK: - Queue Management

    public func addToQueue(_ song: Song) {
        songs.append(song)
        Task {
            await playerPersistenceService?.savePlaybackState(
                volume: volume,
                currentIndex: currentIndex,
                songs: songs
            )
        }
    }

    public func removeFromQueue(at index: Int) {
        guard index >= 0 && index < songs.count else { return }

        // Don't allow removing the currently playing song
        guard index != currentIndex else { return }

        songs.remove(at: index)

        // Update current index if needed
        if index < currentIndex {
            currentIndex -= 1
        }

        Task {
            await playerPersistenceService?.savePlaybackState(
                volume: volume, currentIndex: currentIndex, songs: songs
            )
        }
    }

    public func clearQueue() {
        stop()
        songs.removeAll()
        currentIndex = 0
        currentSong = nil
        Task {
            await playerPersistenceService?.savePlaybackState(
                volume: volume, currentIndex: currentIndex, songs: songs
            )
        }
    }

    public func reorderQueue(from source: IndexSet, to destination: Int) {
        songs.move(fromOffsets: source, toOffset: destination)
        if let currentSong = currentSong {
            currentIndex = songs.firstIndex { $0.id == currentSong.id } ?? 0
        }
        Task {
            await playerPersistenceService?.savePlaybackState(
                volume: volume, currentIndex: currentIndex, songs: songs
            )
        }
    }

    public func moveSong(from sourceIndex: Int, to destinationIndex: Int) {
        guard sourceIndex >= 0 && sourceIndex < songs.count else { return }
        guard destinationIndex >= 0 && destinationIndex < songs.count else { return }
        guard sourceIndex != destinationIndex else { return }

        let song = songs.remove(at: sourceIndex)
        songs.insert(song, at: destinationIndex)

        // Update current index to track the currently playing song
        if let currentSong = currentSong {
            currentIndex = songs.firstIndex { $0.id == currentSong.id } ?? 0
        }

        Task {
            await playerPersistenceService?.savePlaybackState(
                volume: volume, currentIndex: currentIndex, songs: songs
            )
        }
    }

    public func configureQueue(songs: [Song], startIndex: Int) {
        loadGeneration = UUID()
        restorationTask?.cancel()
        loadTask?.cancel()
        isLoading = false
        self.songs = songs
        currentIndex = startIndex
        currentSong = songs[safe: startIndex]
        Task {
            await playerPersistenceService?.savePlaybackState(
                volume: volume, currentIndex: currentIndex, songs: songs
            )
        }
    }

    // MARK: - Playback Control

    public func playSong(_ song: Song) {
        beginLoad(song, autoplay: true)
    }

    public func playPause() {
        if wantsPlayback { pause() } else { play() }
    }

    public func play() {
        wantsPlayback = true
        guard !isLoading else { return }
        guard playbackReady else {
            if let song = currentSong { beginLoad(song, autoplay: true) }
            return
        }
        player.play()
        isPlaying = player.isPlaying
        startTimer()
    }

    public func pause() {
        wantsPlayback = false
        player.pause()
        isPlaying = false
        stopTimer()
    }

    public func stop() {
        loadGeneration = UUID()
        restorationTask?.cancel()
        loadTask?.cancel()
        loadTask = nil
        wantsPlayback = false
        playbackReady = false
        isLoading = false
        player.stop()
        isPlaying = false
        playbackProgress = 0
        stopTimer()
        withOfflineCacheLock {
            LocalWavePreferences.current.removeObject(forKey: "lwActiveSongID")
            LocalWavePreferences.current.removeObject(forKey: "lwActiveCachePath")
        }
    }

    public func previousSong() {
        guard !songs.isEmpty else { return }
        currentIndex = (currentIndex - 1 + songs.count) % songs.count
        playSong(songs[currentIndex])
        Task {
            await playerPersistenceService?.savePlaybackState(
                volume: volume, currentIndex: currentIndex, songs: songs
            )
        }
    }

    public func nextSong() {
        guard !songs.isEmpty else { return }
        let isPlayingLastSong = currentIndex == songs.count - 1

        switch repeatMode {
        case .none:
            if isPlayingLastSong {
                stop()
            } else {
                currentIndex += 1
                playSong(songs[currentIndex])
            }
        case .all:
            currentIndex = isPlayingLastSong ? 0 : currentIndex + 1
            playSong(songs[currentIndex])
        case .one:
            playSong(songs[currentIndex])
        }

        Task {
            await playerPersistenceService?.savePlaybackState(
                volume: volume, currentIndex: currentIndex, songs: songs
            )
        }
    }

    public func seek(to progress: Double) {
        player.currentTime = progress
        updateTimeDisplay()
    }

    public func seekByFraction(_ fraction: Double) {
        player.currentTime = fraction * player.duration
        updateTimeDisplay()
    }

    public func seekBySeconds(_ seconds: Double) {
        let newTime = max(0, min(player.currentTime + seconds, player.duration))
        player.currentTime = newTime
        updateTimeDisplay()
    }

    // MARK: - Shuffle & Repeat

    public func setShuffle(_ enabled: Bool) {
        if enabled {
            if !isShuffleEnabled {
                originalQueue = songs
                if let current = currentSong {
                    var remainingSongs = songs.filter { $0.id != current.id }
                    remainingSongs.shuffle()
                    songs = [current] + remainingSongs
                    currentIndex = 0
                } else {
                    songs.shuffle()
                    currentIndex = 0
                }
            }
        } else {
            if isShuffleEnabled {
                if let current = currentSong, !originalQueue.isEmpty {
                    songs = originalQueue
                    if let index = songs.firstIndex(where: { $0.id == current.id }) {
                        currentIndex = index
                    } else {
                        currentIndex = 0
                        currentSong = songs.first
                    }
                }
                originalQueue = []
            }
        }
        isShuffleEnabled = enabled
    }

    public func setRepeat(_ enabled: RepeatMode) {
        repeatMode = enabled
    }

    public func toggleShuffle() {
        setShuffle(!isShuffleEnabled)
    }

    public func toggleRepeat() {
        var mode = repeatMode
        mode.toggle()
        setRepeat(mode)
    }

    // MARK: - Playlist Creation

    public func createPlaylist(name: String) async throws {
        let newPlaylist = Playlist(id: nil as Int64?, name: name, createdAt: Date(), updatedAt: nil as Date?)
        let createdPlaylist = try await playlistRepo.create(playlist: newPlaylist)
        guard let playlistId = createdPlaylist.id else { return }

        for song in songs {
            guard let songId = song.id else { continue }
            try await playlistSongRepo.addSong(playlistId: playlistId, songId: songId)
        }
    }

    // MARK: - Private Helpers

    internal func stopAndPreloadSong(_ song: Song) {
        beginLoad(song, autoplay: false)
    }

    private func beginLoad(_ song: Song, autoplay: Bool) {
        stop()
        let generation = loadGeneration
        wantsPlayback = autoplay
        currentSong = song
        playbackError = nil
        isLoading = true
        if let id = song.id { withOfflineCacheLock { LocalWavePreferences.current.set(String(id), forKey: "lwActiveSongID") } }
        let resolver = sourceAccessResolver
        let cacheDirectory = cacheDirectory
        loadTask = Task {
            do {
                let preparation = Task.detached(priority: .userInitiated) { try await PlaybackFileAccess.prepare(song, resolver: resolver, cacheDirectory: cacheDirectory) }
                let prepared = try await withTaskCancellationHandler {
                    try await preparation.value
                } onCancel: { preparation.cancel() }
                guard !Task.isCancelled, loadGeneration == generation else { return }
                try player.prepareToPlay(url: prepared.url)
                player.volume = volume * volume
                playbackReady = true
                if let id = song.id, let songRepo, let latest = await songRepo.getSongs(ids: [id]).first {
                    let music = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("Music", isDirectory: true).resolvingSymlinksInPath()
                    if prepared.url.resolvingSymlinksInPath().deletingLastPathComponent() == music {
                        let stored = try await songRepo.updateCachedFile(songId: id, localFilePath: "Music/" + prepared.url.lastPathComponent, state: .copied, expectedState: latest.fileState, expectedPath: latest.localFilePath)
                        if stored, let sourceID = song.sourceId { NotificationCenter.default.post(name: Notification.Name("LWSourceMaterialized"), object: sourceID) }
                    }
                    if let bookmark = prepared.renewedBookmark, latest.bookmark == song.bookmark {
                        try await songRepo.updateBookmark(songId: id, bookmark: bookmark)
                    }
                }
                guard !Task.isCancelled, loadGeneration == generation else { return }
                isLoading = false
                updateTimeDisplay()
                if wantsPlayback { play() }
            } catch is CancellationError {
                if loadGeneration == generation { isLoading = false }
            } catch {
                guard loadGeneration == generation else { return }
                isLoading = false
                wantsPlayback = false
                playbackReady = false
                playbackError = error.localizedDescription
                logger.error("Unable to load audio: \(error)")
            }
        }
    }

    internal func startTimer() {
        guard dispatchTimer == nil else { return }
        // Use DispatchSourceTimer instead of RunLoop Timer for TUI compatibility
        let timer = DispatchSource.makeTimerSource(queue: DispatchQueue.main)
        timer.schedule(deadline: .now(), repeating: 1.0)
        timer.setEventHandler { [weak self] in
            // Already on main queue, just need @MainActor context
            guard let self = self else { return }
            Task { @MainActor in
                await self.updateTimeDisplayAsync()
            }
        }
        timer.resume()
        dispatchTimer = timer

        logger.debug("DispatchTimer started for time display updates")
    }

    internal func stopTimer() {
        dispatchTimer?.cancel()
        dispatchTimer = nil
        timer?.invalidate()
        timer = nil
    }

    internal func updateTimeDisplayAsync() async {
        if !isLoading { isPlaying = player.isPlaying }
        let currentTimeValue = player.currentTime
        let durationValue = player.duration

        playbackProgress = durationValue > 0 ? currentTimeValue / durationValue : 0
        currentTime = formatTime(currentTimeValue)
        duration = formatTime(durationValue)
    }

    internal func updateTimeDisplay() {
        let currentTimeValue = player.currentTime
        let durationValue = player.duration

        playbackProgress = durationValue > 0 ? currentTimeValue / durationValue : 0
        currentTime = formatTime(currentTimeValue)
        duration = formatTime(durationValue)
    }

    internal func formatTime(_ time: TimeInterval) -> String {
        let minutes = Int(time) / 60
        let seconds = Int(time) % 60
        return String(format: "%d:%02d", minutes, seconds)
    }
}
