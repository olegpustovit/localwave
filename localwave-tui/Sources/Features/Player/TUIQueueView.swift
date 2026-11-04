//
//  TUIQueueView.swift
//  localwave-tui
//
//  Created by Claude Code on 05.10.2025.
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData
import LocalWavePlayer

/// TUI queue management view
///
/// Features:
/// - Display current playback queue
/// - Highlight currently playing song
/// - Navigate with j/k
/// - Remove songs with Delete/Backspace
/// - Clear entire queue with 'c'
/// - Move songs up/down with Ctrl+k/Ctrl+j
@MainActor
struct TUIQueueView: View {
    @ObservedObject var playerViewModel: BasePlayerViewModel
    @Binding var navigationState: NavigationState
    let dependencies: TUIDependencyContainer

    @State private var listState = TUIListState()

    var body: some View {
        let queue = playerViewModel.queue
        let currentSong = playerViewModel.currentSong
        let currentIndex = getCurrentSongIndex(queue: queue, currentSong: currentSong)

        return TerminalAwareList(
            state: $listState,
            sizeTracker: dependencies.terminalSizeTracker,
            reservedLines: TerminalSizeHelper.ReservedLines.playerQueue
        ) {
            VStack(spacing: 1) {
                headerView(queueCount: queue.count, totalDuration: calculateTotalDuration(queue: queue))
                Text("")

                if queue.isEmpty {
                    emptyQueueView
                } else {
                    queueListView(queue: queue, currentIndex: currentIndex)
                }

                Spacer()
                helpBar
            }
        }
        .onAppear {
            // Center on currently playing song
            if let currentIndex = currentIndex {
                listState.selectedIndex = currentIndex
                centerOnCurrentSong(queueSize: queue.count, currentIndex: currentIndex)
            }
        }
        // Navigation
        .onKeyPress("j") { listState.selectNext(itemCount: queue.count) }
        .onKeyPress("k") { listState.selectPrevious() }
        .onKeyPress("g") { listState.selectFirst() }
        .onKeyPress("G") { listState.selectLast(itemCount: queue.count) }
        // Actions
        .onKeyPress("\u{7F}") { removeSelected() } // Backspace
        .onKeyPress("x") { removeSelected() }
        .onKeyPress("c") { clearQueue() }
        .onKeyPress("C") { clearQueue() }
        // Move
        .onKeyPress("K") { moveUp() } // Shift+K
        .onKeyPress("J") { moveDown(queueSize: queue.count) } // Shift+J
        // Back
        .onKeyPress("\u{1B}") { navigationState.pop() } // Esc
    }

    // MARK: - View Components

    private func headerView(queueCount: Int, totalDuration: String) -> some View {
        VStack(spacing: 1) {
            HStack {
                Text(TUIColors.Indicators.info("♫ Queue"))
                Text(" (\(queueCount) songs - \(totalDuration))")
                Spacer()
            }
            HStack {
                Text(TUITheme.divider(width: 80))
                Spacer()
            }
        }
    }

    private var emptyQueueView: some View {
        VStack {
            Text("")
            HStack {
                Text("Queue is empty")
                Spacer()
            }
            Text("")
            HStack {
                Text("Add songs to the queue from the library")
                Spacer()
            }
        }
    }

    private func queueListView(queue: [Song], currentIndex: Int?) -> some View {
        let visibleStart = min(listState.scrollOffset, queue.count)
        let visibleEnd = min(listState.scrollOffset + listState.effectiveVisibleHeight, queue.count)

        return VStack(spacing: 0) {
            // Scroll indicator
            if queue.count > listState.effectiveVisibleHeight {
                HStack {
                    Text("Showing \(visibleStart + 1)-\(visibleEnd) of \(queue.count)")
                    Spacer()
                }
                Text("")
            }

            // Queue items - only render if we have a valid range
            // ULTRA-DEFENSIVE: Verify all bounds before creating ForEach Range
            if visibleStart >= 0 &&
               visibleStart < queue.count &&
               visibleEnd > 0 &&
               visibleEnd <= queue.count &&
               visibleStart < visibleEnd {
                ForEach(visibleStart..<visibleEnd, id: \.self) { globalIndex in
                    let song = queue[globalIndex]
                    let isSelected = globalIndex == listState.selectedIndex
                    let isPlaying = currentIndex == globalIndex
                    queueRow(song: song, index: globalIndex + 1, isSelected: isSelected, isPlaying: isPlaying)
                }
            }
        }
    }

    private func queueRow(song: Song, index: Int, isSelected: Bool, isPlaying: Bool) -> some View {
        HStack {
            if isPlaying {
                Text(" ♫ ")
            } else if isSelected {
                Text(" > ")
            } else {
                Text("   ")
            }
            Text(String(format: "%2d.", index))
            Text(" ")
            Text(TUITheme.truncate("\(song.artist) - \(song.title)", width: 55))
            Spacer()
            // Show duration if available (placeholder for now)
            Text("--:--")
        }
    }

    private var helpBar: some View {
        VStack(spacing: 0) {
            Text(TUITheme.divider(width: 80))
            HStack {
                Text("[j/k] Navigate")
                Text("  ")
                Text("[K/J] Move Up/Down")
                Text("  ")
                Text("[x] Remove")
                Text("  ")
                Text("[c] Clear")
                Spacer()
            }
            HStack {
                Text("[Esc] Back")
                Spacer()
            }
        }
    }

    // MARK: - Helper Methods

    private func getCurrentSongIndex(queue: [Song], currentSong: Song?) -> Int? {
        guard let currentSong = currentSong else { return nil }
        return queue.firstIndex { $0.id == currentSong.id }
    }

    private func calculateTotalDuration(queue: [Song]) -> String {
        // Placeholder - would need duration from song metadata
        // For now, estimate 3 minutes per song
        let totalMinutes = queue.count * 3
        let hours = totalMinutes / 60
        let minutes = totalMinutes % 60
        if hours > 0 {
            return "\(hours):\(String(format: "%02d", minutes)):00"
        } else {
            return "\(minutes):00"
        }
    }

    private func centerOnCurrentSong(queueSize: Int, currentIndex: Int) {
        // Center the view on the current song
        let halfHeight = listState.effectiveVisibleHeight / 2
        listState.scrollOffset = max(0, currentIndex - halfHeight)
        listState.scrollOffset = min(listState.scrollOffset, max(0, queueSize - listState.effectiveVisibleHeight))
    }

    // MARK: - Queue Actions

    private func removeSelected() {
        let queue = playerViewModel.queue
        guard listState.selectedIndex >= 0 && listState.selectedIndex < queue.count else { return }

        // Can't remove currently playing song
        let currentSong = playerViewModel.currentSong
        if let currentIndex = getCurrentSongIndex(queue: queue, currentSong: currentSong),
           listState.selectedIndex == currentIndex {
            return
        }

        playerViewModel.removeFromQueue(at: listState.selectedIndex)

        // Adjust selection
        let newQueueSize = queue.count - 1
        if listState.selectedIndex >= newQueueSize && newQueueSize > 0 {
            listState.selectedIndex = newQueueSize - 1
        }
    }

    private func clearQueue() {
        playerViewModel.clearQueue()
        listState.reset()
    }

    private func moveUp() {
        guard listState.selectedIndex > 0 else { return }
        let queue = playerViewModel.queue
        guard listState.selectedIndex < queue.count else { return }

        // Can't move currently playing song
        let currentSong = playerViewModel.currentSong
        if let currentIndex = getCurrentSongIndex(queue: queue, currentSong: currentSong),
           listState.selectedIndex == currentIndex {
            return
        }

        // Move song up in queue
        playerViewModel.moveSong(from: listState.selectedIndex, to: listState.selectedIndex - 1)

        // Update selection to follow the moved song
        listState.selectPrevious()
    }

    private func moveDown(queueSize: Int) {
        guard listState.selectedIndex < queueSize - 1 else { return }
        let queue = playerViewModel.queue
        guard listState.selectedIndex < queue.count else { return }

        // Can't move currently playing song
        let currentSong = playerViewModel.currentSong
        if let currentIndex = getCurrentSongIndex(queue: queue, currentSong: currentSong),
           listState.selectedIndex == currentIndex {
            return
        }

        // Move song down in queue
        playerViewModel.moveSong(from: listState.selectedIndex, to: listState.selectedIndex + 1)

        // Update selection to follow the moved song
        listState.selectNext(itemCount: queueSize)
    }
}
