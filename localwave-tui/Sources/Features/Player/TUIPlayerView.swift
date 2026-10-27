//
//  TUIPlayerView.swift
//  localwave-tui
//
//  Created by Claude Code on 05.10.2025.
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData
import LocalWavePlayer

/// TUI full player view (Tab 6)
///
/// Features:
/// - Large display of current song info
/// - Interactive playback controls (◀◀ ⏸/▶ ▶▶)
/// - Progress bar with seek controls
/// - Volume control
/// - Shuffle and Repeat toggles
/// - Queue preview (next 3 songs)
/// - Keyboard controls: Space (play/pause), ←/→ (prev/next), [/] (seek), +/- (volume)
@MainActor
struct TUIPlayerView: View {
    @ObservedObject var playerViewModel: BasePlayerViewModel
    let dependencies: TUIDependencyContainer
    @Binding var navigationState: NavigationState

    var body: some View {
        VStack(spacing: 1) {
            headerView
            nowPlayingView
            progressView
            controlsView
            queuePreviewView
            Spacer()
            helpBar
        }
        // Playback controls
        .onKeyPress(" ") { togglePlayPause() }
        .onKeyPress("p") { playerViewModel.previousSong() }
        .onKeyPress("P") { playerViewModel.previousSong() }
        .onKeyPress("n") { playerViewModel.nextSong() }
        .onKeyPress("N") { playerViewModel.nextSong() }
        // Seek controls
        .onKeyPress("[") { seekBackward() }
        .onKeyPress("]") { seekForward() }
        // Volume controls
        .onKeyPress("+") { volumeUp() }
        .onKeyPress("=") { volumeUp() } // + key without shift
        .onKeyPress("-") { volumeDown() }
        // Toggle controls
        .onKeyPress("s") { toggleShuffle() }
        .onKeyPress("S") { toggleShuffle() }
        .onKeyPress("r") { toggleRepeat() }
        .onKeyPress("R") { toggleRepeat() }
        // Queue view
        .onKeyPress("q") { openQueue() }
        .onKeyPress("Q") { openQueue() }
    }

    // MARK: - View Components

    private var headerView: some View {
        VStack(spacing: 1) {
            Text("")
            HStack {
                Text(TUIColors.Indicators.info("♫ Now Playing"))
                Spacer()
            }
            Text("")
        }
    }

    private var nowPlayingView: some View {
        let song = playerViewModel.currentSong
        return VStack(spacing: 1) {
            if let song = song {
                HStack {
                    Text(TUIColors.Indicators.success("Title:  "))
                    Text(song.title)
                    Spacer()
                }
                HStack {
                    Text(TUIColors.Indicators.success("Artist: "))
                    Text(song.artist)
                    Spacer()
                }
                HStack {
                    Text(TUIColors.Indicators.success("Album:  "))
                    Text(song.album)
                    if let year = song.releaseYear {
                        Text(" (\(year))")
                    }
                    Spacer()
                }
            } else {
                HStack {
                    Text(TUIColors.Indicators.warning("No song playing"))
                    Spacer()
                }
            }
            Text("")
        }
    }

    private var progressView: some View {
        let currentTime = playerViewModel.currentTime
        let duration = playerViewModel.duration
        let progress = playerViewModel.playbackProgress
        return VStack(spacing: 1) {
            HStack {
                Text(TUITheme.divider(width: 80))
                Spacer()
            }
            HStack {
                Text(" ")
                Text(currentTime)
                Text("  ")
                TUIProgressBar(
                    value: progress,
                    width: 60,
                    label: nil,
                    showPercentage: false
                )
                Text("  ")
                Text(duration)
                Spacer()
            }
            HStack {
                Text(TUITheme.divider(width: 80))
                Spacer()
            }
        }
    }

    private var controlsView: some View {
        let isPlaying = playerViewModel.isPlaying
        let volume = playerViewModel.volume
        let volumePercent = Int(volume * 100)
        let isShuffleEnabled = playerViewModel.isShuffleEnabled
        let repeatMode = playerViewModel.repeatMode
        return VStack(spacing: 1) {
            // Playback controls
            HStack {
                Text("   ")
                Text("◀◀")
                Text("  ")
                Text(isPlaying ? "⏸" : "▶")
                Text("  ")
                Text("▶▶")
                Spacer()
            }

            Text("")

            // Volume control
            HStack {
                Text(TUIColors.Indicators.success("Volume: "))
                TUIProgressBar(
                    value: Double(volume),
                    width: 30,
                    label: "\(volumePercent)%",
                    showPercentage: false
                )
                Spacer()
            }

            Text("")

            // Shuffle and Repeat
            HStack {
                Text(TUIColors.Indicators.success("Shuffle: "))
                Text(isShuffleEnabled ? "🔀 ON " : "OFF")
                Text("   ")
                Text(TUIColors.Indicators.success("Repeat: "))
                Text("\(repeatMode.symbol) \(repeatMode.displayName)")
                Spacer()
            }
            Text("")
        }
    }

    private var queuePreviewView: some View {
        let queue = playerViewModel.queue
        return VStack(spacing: 1) {
            HStack {
                Text(TUITheme.divider(width: 80))
                Spacer()
            }
            HStack {
                Text(TUIColors.Indicators.info("Next in Queue"))
                Spacer()
            }

            if queue.isEmpty {
                HStack {
                    Text("  Queue is empty")
                    Spacer()
                }
            } else {
                // Show next 3 songs
                ForEach(Array(queue.prefix(3).enumerated()), id: \.element.id) { index, song in
                    HStack {
                        Text("  \(index + 1). ")
                        Text(TUITheme.truncate("\(song.artist) - \(song.title)", width: 60))
                        Spacer()
                    }
                }
                if queue.count > 3 {
                    HStack {
                        Text("  ... and \(queue.count - 3) more")
                        Spacer()
                    }
                }
            }
        }
    }

    private var helpBar: some View {
        VStack(spacing: 0) {
            Text(TUITheme.divider(width: 80))
            HStack {
                Text("[Space] Play/Pause")
                Text("  ")
                Text("[P/N] Prev/Next")
                Text("  ")
                Text("[[ / ]] Seek")
                Text("  ")
                Text("[Q] Queue")
                Spacer()
            }
            HStack {
                Text("[+ / -] Volume")
                Text("  ")
                Text("[S] Shuffle")
                Text("  ")
                Text("[R] Repeat")
                Spacer()
            }
        }
    }

    // MARK: - Actions

    @MainActor
    private func togglePlayPause() {
        if playerViewModel.isPlaying {
            playerViewModel.pause()
        } else {
            playerViewModel.play()
        }
    }

    @MainActor
    private func seekBackward() {
        playerViewModel.seekBySeconds(-10.0)
    }

    @MainActor
    private func seekForward() {
        playerViewModel.seekBySeconds(10.0)
    }

    @MainActor
    private func volumeUp() {
        playerViewModel.volume = min(1.0, playerViewModel.volume + 0.1)
    }

    @MainActor
    private func volumeDown() {
        playerViewModel.volume = max(0.0, playerViewModel.volume - 0.1)
    }

    @MainActor
    private func toggleShuffle() {
        playerViewModel.toggleShuffle()
    }

    @MainActor
    private func toggleRepeat() {
        playerViewModel.toggleRepeat()
    }

    @MainActor
    private func openQueue() {
        navigationState.push(.queue)
    }
}
