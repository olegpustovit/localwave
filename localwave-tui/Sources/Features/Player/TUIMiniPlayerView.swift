//
//  TUIMiniPlayerView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData
import LocalWavePlayer

/// TUI mini player status bar (always visible at bottom)
///
/// Features:
/// - Line 1: Now playing info (artist, title, shuffle/repeat icons)
/// - Line 2: Progress bar and time
/// - Updates in real-time using BasePlayerViewModel
/// - Compact 2-line display
@MainActor
struct TUIMiniPlayerView: View {
    @ObservedObject var playerViewModel: BasePlayerViewModel

    var body: some View {
        let song = playerViewModel.currentSong
        let isPlaying = playerViewModel.isPlaying
        let isShuffleEnabled = playerViewModel.isShuffleEnabled
        let repeatMode = playerViewModel.repeatMode
        let playbackProgress = playerViewModel.playbackProgress
        let currentTime = playerViewModel.currentTime
        let duration = playerViewModel.duration

        return VStack(spacing: 0) {
            Text(TUITheme.divider(width: 80))

            // Line 1: Now playing info
            HStack {
                if let song = song {
                    Text(isPlaying ? " ▶ " : " ⏸ ")
                    Text(TUITheme.truncate("\(song.artist) - \(song.title)", width: 50))
                    Spacer()
                    if isShuffleEnabled {
                        Text("🔀 ")
                    }
                    Text(repeatMode.symbol)
                } else {
                    Text(" No song playing")
                    Spacer()
                }
            }

            // Line 2: Progress bar and time
            HStack {
                Text(" ")
                TUIProgressBar(
                    value: playbackProgress,
                    width: 60,
                    label: nil,
                    showPercentage: false
                )
                Text(" ")
                Text(currentTime)
                Text(" / ")
                Text(duration)
                Spacer()
            }
        }
    }
}
