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

    @Published public var volume: Float = 1.0 {
        didSet {
            // Apply logarithmic curve for better human perception
            let actualVolume = volume * volume
            player.volume = actualVolume
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
    internal var activeSecurityScopedURLs = [URL]()

    internal let logger: AppLogger

    // MARK: - Initialization

    public init(
        player: AudioPlayerProtocol,
        playerPersistenceService: PlayerPersistenceService? = nil,
        songRepo: SongRepository? = nil,
        playlistRepo: PlaylistRepository,
        playlistSongRepo: PlaylistSongRepository,
        logger: AppLogger? = nil
    ) {
        self.player = player
        self.playerPersistenceService = playerPersistenceService
        self.songRepo = songRepo
        self.playlistRepo = playlistRepo
        self.playlistSongRepo = playlistSongRepo

        // Use provided logger or create platform-appropriate logger
        self.logger = logger ?? createLogger(subsystem: subsystem, category: "BasePlayerViewModel")

        self.player.delegate = self

        Task {
            // Restore volume
            if let storedVolume = await self.playerPersistenceService?.getVolume() {
                self.volume = storedVolume
            }

            // Restore queue and current song
            if let (songs, currentIndex, currentSong) = await self.playerPersistenceService?.restore() {
                self.songs = songs
                self.currentIndex = currentIndex
                self.currentSong = currentSong

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
        stopAndPreloadSong(song)
        play()
    }

    public func playPause() {
        if isPlaying {
            pause()
        } else {
            play()
        }
    }

    public func play() {
        logger.debug("BasePlayerViewModel.play() called, current isPlaying: \(self.isPlaying)")
        player.play()
        isPlaying = true
        startTimer()
    }

    public func pause() {
        logger.debug("BasePlayerViewModel.pause() called, current isPlaying: \(self.isPlaying)")
        player.pause()
        isPlaying = false
        stopTimer()
    }

    public func stop() {
        player.stop()
        isPlaying = false
        playbackProgress = 0
        stopTimer()

        if let url = player.url {
            url.stopAccessingSecurityScopedResource()
        }

        activeSecurityScopedURLs.forEach { $0.stopAccessingSecurityScopedResource() }
        activeSecurityScopedURLs.removeAll()
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
        stop()

        guard let url = resolveSongURL(song) else {
            logger.error("Can't load song URL.")
            return
        }

        guard url.startAccessingSecurityScopedResource() else {
            logger.error("Failed to start accessing security scoped resource for song: \(song.title)")
            do {
                let newBookmark = try url.bookmarkData(options: [])
                logger.warning("Renewed bookmark for song: \(song.title)")
                var updatedSong = song
                updatedSong.bookmark = newBookmark
                Task {
                    _ = try await songRepo?.upsertSong(updatedSong)
                }
            } catch {
                logger.error("Failed to renew bookmark: \(error)")
            }
            return
        }
        activeSecurityScopedURLs.append(url)

        do {
            try player.prepareToPlay(url: url)
            player.volume = volume * volume
            currentSong = song
            updateTimeDisplay()
        } catch {
            logger.error("Player init error: \(error)")
            url.stopAccessingSecurityScopedResource()
            activeSecurityScopedURLs.removeAll { $0 == url }
        }
    }

    internal func resolveSongURL(_ song: Song) -> URL? {
        guard let bookmarkData = song.bookmark else { return nil }

        var isStale = false
        do {
            let url = try URL(
                resolvingBookmarkData: bookmarkData,
                options: .withoutUI,
                relativeTo: nil,
                bookmarkDataIsStale: &isStale
            )

            if isStale {
                let newBookmark = try url.bookmarkData(options: [])
                logger.warning("Bookmark was stale - consider reimporting this file")

                var updatedSong = song
                updatedSong.bookmark = newBookmark
                Task {
                    _ = try await songRepo?.upsertSong(updatedSong)
                }
            }

            return url
        } catch {
            logger.error("Bookmark error: \(error)")
            return nil
        }
    }

    internal func startTimer() {
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
