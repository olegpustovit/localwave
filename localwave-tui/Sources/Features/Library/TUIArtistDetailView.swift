//
//  TUIArtistDetailView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI view for displaying songs by a specific artist
///
/// Features:
/// - Show all songs by selected artist
/// - Display artist name header with song count and total duration
/// - Scrollable list of songs with pagination
/// - Play, queue, and playlist actions
/// - Vim-style navigation (j/k/g/G)
/// - Pagination with u/d keys
struct TUIArtistDetailView: View {
    let dependencies: TUIDependencyContainer
    let artist: String
    @Binding var navigationState: NavigationState

    // State
    @State private var songs: [Song] = []
    @State private var selectedIndex = 0
    @State private var isLoading = false
    @State private var errorMessage: String?

    // Pagination
    @State private var pageOffset = 0
    private let pageSize = 10

    var paginatedSongs: ArraySlice<Song> {
        let start = pageOffset
        let end = min(start + pageSize, songs.count)
        guard start < songs.count else { return [] }
        return songs[start..<end]
    }

    var totalDuration: String {
        // Note: Song model doesn't have duration property yet
        // For now, we'll skip this or return "N/A"
        return "N/A"
    }

    var body: some View {
        VStack(spacing: 1) {
            // Header with artist info
            headerView

            Text("")

            if isLoading {
                loadingView
            } else if let error = errorMessage {
                errorView(error)
            } else if songs.isEmpty {
                emptyView
            } else {
                songListView
            }

            Spacer()
            helpBar
        }
        .onAppear {
            Task { await loadSongs() }
        }
        // Navigation
        .onKeyPress("j") { selectNext() }
        .onKeyPress("k") { selectPrevious() }
        .onKeyPress("g") { selectFirst() }
        .onKeyPress("G") { selectLast() }
        // Pagination
        .onKeyPress("u") { pageUp() }
        .onKeyPress("d") { pageDown() }
        // Actions
        .onKeyPress(" ") { Task { @MainActor in playSong() } }
        .onKeyPress("q") { Task { @MainActor in queueSong() } }
        .onKeyPress("Q") { Task { @MainActor in queueSong() } }
        .onKeyPress("p") { addToPlaylist() }
        .onKeyPress("P") { addToPlaylist() }
        .onKeyPress("h") { navigationState.pop() }
        .onKeyPress("\u{1B}") { navigationState.pop() }
    }

    // MARK: - View Components

    private var headerView: some View {
        VStack(spacing: 0) {
            HStack {
                Text("LocalWave > Artists > \(artist)")
                Spacer()
            }
            Text("")
            HStack {
                Text(TUIColors.Indicators.info("\(artist) - \(songs.count) songs - \(totalDuration)"))
                Spacer()
            }
        }
    }

    private var loadingView: some View {
        VStack {
            Text("Loading songs...")
            Spacer()
        }
    }

    private func errorView(_ message: String) -> some View {
        VStack {
            HStack {
                Text(TUIColors.Indicators.error("Error: \(message)"))
                Spacer()
            }
            Text("")
            HStack {
                Text("Press 'h' or Esc to go back")
                Spacer()
            }
            Spacer()
        }
    }

    private var emptyView: some View {
        VStack {
            Text("No songs found for \(artist)")
            Text("")
            Text("Press 'h' or Esc to go back")
            Spacer()
        }
    }

    private var songListView: some View {
        VStack(spacing: 0) {
            // Page indicator
            if songs.count > pageSize {
                HStack {
                    Text("Showing \(pageOffset + 1)-\(min(pageOffset + pageSize, songs.count)) of \(songs.count)")
                    Text(" [u/d for pages]")
                    Spacer()
                }
                Text("")
            }

            ForEach(Array(paginatedSongs.enumerated()), id: \.element.id) { index, song in
                let globalIndex = pageOffset + index
                let isSelected = globalIndex == selectedIndex
                songRow(song: song, isSelected: isSelected)
            }
            Spacer()
        }
    }

    private func songRow(song: Song, isSelected: Bool) -> some View {
        HStack {
            Text(isSelected ? " > " : "   ")
            Text(TUITheme.Icons.music)
            Text(" ")
            Text(TUITheme.truncate(song.title, width: 30))
            Spacer()
            Text(TUITheme.truncate(song.album, width: 20))
            Text("  ")
            // Duration would go here if available
            // Text("3:45")
        }
    }

    private var helpBar: some View {
        VStack(spacing: 0) {
            Text(TUITheme.divider(width: 80))
            HStack {
                Text("[j/k] Navigate")
                Text("  ")
                Text("[Space] Play")
                Text("  ")
                Text("[q] Queue")
                Text("  ")
                Text("[p] Playlist")
                Text("  ")
                Text("[h/Esc] Back")
                Spacer()
            }
        }
    }

    // MARK: - Actions

    private func loadSongs() async {
        isLoading = true
        errorMessage = nil

        do {
            let songRepo = await dependencies.songRepository

            // Use SQLiteSongRepository to get songs by artist
            if let sqliteRepo = songRepo as? SQLiteSongRepository {
                songs = try await sqliteRepo.getSongsByArtist(artist)
            } else {
                // Fallback: use FTS search with artist filter
                songs = try await songRepo.searchSongsFTS(
                    query: "artist:\"\(artist)\"",
                    limit: 1000,  // Get all songs for this artist
                    offset: 0
                )
            }

            selectedIndex = 0
            pageOffset = 0
        } catch {
            errorMessage = error.localizedDescription
        }

        isLoading = false
    }

    @MainActor
    private func playSong() {
        guard selectedIndex < songs.count else { return }
        let song = songs[selectedIndex]

        // Configure queue with all artist songs, starting from selected
        let playerVM = dependencies.playerViewModel
        playerVM.configureQueue(songs: songs, startIndex: selectedIndex)
        playerVM.playSong(song)
    }

    @MainActor
    private func queueSong() {
        guard selectedIndex < songs.count else { return }
        let song = songs[selectedIndex]

        // Add song to the end of current queue
        let playerVM = dependencies.playerViewModel
        playerVM.addToQueue(song)
    }

    private func addToPlaylist() {
        guard selectedIndex < songs.count else { return }
        let song = songs[selectedIndex]
        // TODO: Implement playlist integration
        errorMessage = "Playlist functionality coming soon for: \(song.title)"
    }

    // MARK: - Navigation Helpers

    private func selectNext() {
        guard !songs.isEmpty else { return }
        if selectedIndex < songs.count - 1 {
            selectedIndex += 1
            // Auto-scroll to next page if needed
            if selectedIndex >= pageOffset + pageSize {
                pageOffset = min(selectedIndex, songs.count - pageSize)
            }
        }
    }

    private func selectPrevious() {
        guard !songs.isEmpty else { return }
        if selectedIndex > 0 {
            selectedIndex -= 1
            // Auto-scroll to previous page if needed
            if selectedIndex < pageOffset {
                pageOffset = max(0, selectedIndex)
            }
        }
    }

    private func selectFirst() {
        selectedIndex = 0
        pageOffset = 0
    }

    private func selectLast() {
        guard !songs.isEmpty else { return }
        selectedIndex = songs.count - 1
        pageOffset = max(0, songs.count - pageSize)
    }

    private func pageUp() {
        pageOffset = max(0, pageOffset - pageSize)
        selectedIndex = max(0, min(selectedIndex, pageOffset + pageSize - 1))
    }

    private func pageDown() {
        let maxOffset = max(0, songs.count - pageSize)
        pageOffset = min(maxOffset, pageOffset + pageSize)
        selectedIndex = min(songs.count - 1, pageOffset)
    }
}
