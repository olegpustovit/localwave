//
//  TUISongListView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI view for displaying all songs in table format
///
/// Features:
/// - Show all songs with title, artist, album columns
/// - Scrollable song list with pagination
/// - Search functionality with '/' key
/// - Currently playing indicator (♫)
/// - Play, queue, and playlist actions
/// - Vim-style navigation (j/k/g/G)
/// - Pagination with u/d keys
struct TUISongListView: View {
    let dependencies: TUIDependencyContainer
    @Binding var navigationState: NavigationState

    // State
    @State private var songs: [Song] = []
    @State private var selectedIndex = 0
    @State private var isLoading = false
    @State private var errorMessage: String?

    // Pagination
    @State private var pageOffset = 0
    private let pageSize = 10

    // Search
    @State private var searchMode = false
    @State private var searchTerm = ""

    // Currently playing (placeholder - would come from player state)
    @State private var currentlyPlayingSongId: Int64? = nil

    // Editor state
    @State private var showEditor = false
    @State private var songToEdit: Song? = nil

    var filteredSongs: [Song] {
        guard !searchTerm.isEmpty else { return songs }
        return songs.filter { song in
            song.title.localizedCaseInsensitiveContains(searchTerm) ||
            song.artist.localizedCaseInsensitiveContains(searchTerm) ||
            song.album.localizedCaseInsensitiveContains(searchTerm)
        }
    }

    var paginatedSongs: ArraySlice<Song> {
        let start = pageOffset
        let end = min(start + pageSize, filteredSongs.count)
        guard start < filteredSongs.count else { return [] }
        return filteredSongs[start..<end]
    }

    var body: some View {
        ZStack {
            mainContent
            if showEditor, let song = songToEdit {
                TUISongEditorView(
                    dependencies: dependencies,
                    song: song,
                    onSave: {
                        showEditor = false
                        songToEdit = nil
                        await loadSongs()
                    },
                    onCancel: {
                        showEditor = false
                        songToEdit = nil
                    }
                )
            }
        }
    }

    private var mainContent: some View {
        VStack(spacing: 1) {
            // Header
            HStack {
                Text("Songs (\(filteredSongs.count))")
                Spacer()
            }
            Text("")

            // Search bar or search results header
            if searchMode {
                searchBarView
            } else if !searchTerm.isEmpty {
                searchResultsHeader
            }

            Text("")

            if isLoading {
                loadingView
            } else if let error = errorMessage {
                errorView(error)
            } else if filteredSongs.isEmpty {
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
        // Navigation (only when not in search mode)
        .onKeyPress("j") { if !searchMode { selectNext() } }
        .onKeyPress("k") { if !searchMode { selectPrevious() } }
        .onKeyPress("g") { if !searchMode { selectFirst() } }
        .onKeyPress("G") { if !searchMode { selectLast() } }
        // Pagination (only when not in search mode)
        .onKeyPress("u") { if !searchMode { pageUp() } }
        .onKeyPress("d") { if !searchMode { pageDown() } }
        // Actions (only when not in search mode)
        .onKeyPress(" ") { if !searchMode { Task { @MainActor in playSong() } } }
        .onKeyPress("q") { if !searchMode { Task { @MainActor in queueSong() } } }
        .onKeyPress("Q") { if !searchMode { Task { @MainActor in queueSong() } } }
        .onKeyPress("p") { if !searchMode && !showEditor { addToPlaylist() } }
        .onKeyPress("P") { if !searchMode && !showEditor { addToPlaylist() } }
        .onKeyPress("e") { if !searchMode && !showEditor { editSong() } }
        .onKeyPress("E") { if !searchMode && !showEditor { editSong() } }
        .onKeyPress("r") { if !searchMode && !showEditor { Task { await loadSongs() } } }
        .onKeyPress("/") { if !searchMode && !showEditor { searchMode = true } }
        .onKeyPress("\u{1B}") { handleEscape() }
    }

    // MARK: - View Components

    private var searchBarView: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Search mode - type and press Enter to search, Esc to exit")
                Spacer()
            }
            HStack {
                Text("Query: ")
                TextField(placeholder: "Type title, artist, or album...") { query in
                    searchTerm = query
                    searchMode = false  // Exit search mode after submitting
                    selectedIndex = 0
                    pageOffset = 0
                }
                Spacer()
            }
        }
    }

    private var searchResultsHeader: some View {
        HStack {
            Text("Search results for: \"\(searchTerm)\"")
            Text(" - Press '/' to search again, Esc to clear")
            Spacer()
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
                Text("Press 'r' to retry")
                Spacer()
            }
            Spacer()
        }
    }

    private var emptyView: some View {
        VStack {
            if searchTerm.isEmpty {
                Text("No songs found")
                Text("")
                Text("Add a music source with audio files to populate songs")
            } else {
                Text("No songs match \"\(searchTerm)\"")
                Text("")
                Text("Press Esc to clear search")
            }
            Spacer()
        }
    }

    private var songListView: some View {
        VStack(spacing: 0) {
            // Page indicator
            if filteredSongs.count > pageSize {
                HStack {
                    Text("Showing \(pageOffset + 1)-\(min(pageOffset + pageSize, filteredSongs.count)) of \(filteredSongs.count)")
                    Text(" [u/d for pages]")
                    Spacer()
                }
                Text("")
            }

            // Table header
            HStack {
                Text("   Title              ")
                Text("Artist           ")
                Text("Album            ")
                Spacer()
            }
            HStack {
                Text(TUITheme.divider(width: 80))
                Spacer()
            }

            ForEach(Array(paginatedSongs.enumerated()), id: \.element.id) { index, song in
                let globalIndex = pageOffset + index
                let isSelected = globalIndex == selectedIndex
                let isPlaying = song.id == currentlyPlayingSongId
                songRow(song: song, isSelected: isSelected, isPlaying: isPlaying)
            }
            Spacer()
        }
    }

    private func songRow(song: Song, isSelected: Bool, isPlaying: Bool) -> some View {
        HStack {
            if isPlaying {
                Text(" ♫ ")
            } else if isSelected {
                Text(" > ")
            } else {
                Text("   ")
            }
            Text(TUITheme.truncate(song.title, width: 20))
            Text(" ")
            Text(TUITheme.truncate(song.artist, width: 17))
            Text(" ")
            Text(TUITheme.truncate(song.album, width: 17))
            Spacer()
            // Duration would go here if available
            // Text("3:45")
        }
    }

    private var helpBar: some View {
        VStack(spacing: 0) {
            Text(TUITheme.divider(width: 80))
            if searchMode {
                HStack {
                    Text("[Esc] Exit Search")
                    Spacer()
                }
            } else {
                HStack {
                    Text("[j/k] Navigate")
                    Text("  ")
                    Text("[Space] Play")
                    Text("  ")
                    Text("[q] Queue")
                    Text("  ")
                    Text("[p] Playlist")
                    Text("  ")
                    Text("[e] Edit")
                    Spacer()
                }
                HStack {
                    Text("[/] Search")
                    Text("  ")
                    Text("[r] Refresh")
                    Spacer()
                }
            }
        }
    }

    // MARK: - Actions

    private func loadSongs() async {
        isLoading = true
        errorMessage = nil

        do {
            let songRepo = await dependencies.songRepository

            // Load all songs using FTS search with empty query
            songs = try await songRepo.searchSongsFTS(
                query: "",
                limit: 1000,  // Load up to 1000 songs
                offset: 0
            )

            selectedIndex = 0
            pageOffset = 0
        } catch {
            errorMessage = error.localizedDescription
        }

        isLoading = false
    }

    @MainActor
    private func playSong() {
        guard selectedIndex < filteredSongs.count else { return }
        let song = filteredSongs[selectedIndex]

        // Configure queue with all filtered songs, starting from selected
        let playerVM = dependencies.playerViewModel
        playerVM.configureQueue(songs: filteredSongs, startIndex: selectedIndex)
        playerVM.playSong(song)
    }

    @MainActor
    private func queueSong() {
        guard selectedIndex < filteredSongs.count else { return }
        let song = filteredSongs[selectedIndex]

        // Add song to the end of current queue
        let playerVM = dependencies.playerViewModel
        playerVM.addToQueue(song)
    }

    private func addToPlaylist() {
        guard selectedIndex < filteredSongs.count else { return }
        let song = filteredSongs[selectedIndex]
        // TODO: Implement playlist integration
        errorMessage = "Playlist functionality coming soon for: \(song.title)"
    }

    private func editSong() {
        guard selectedIndex < filteredSongs.count else { return }
        songToEdit = filteredSongs[selectedIndex]
        showEditor = true
    }

    private func handleEscape() {
        if searchMode {
            // Cancel search input mode
            searchMode = false
            searchTerm = ""
        } else if !searchTerm.isEmpty {
            // Clear search results
            searchTerm = ""
            selectedIndex = 0
            pageOffset = 0
        } else {
            // Exit song list view
            navigationState.pop()
        }
    }

    // MARK: - Navigation Helpers

    private func selectNext() {
        guard !filteredSongs.isEmpty else { return }
        if selectedIndex < filteredSongs.count - 1 {
            selectedIndex += 1
            // Auto-scroll to next page if needed
            if selectedIndex >= pageOffset + pageSize {
                pageOffset = min(selectedIndex, filteredSongs.count - pageSize)
            }
        }
    }

    private func selectPrevious() {
        guard !filteredSongs.isEmpty else { return }
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
        guard !filteredSongs.isEmpty else { return }
        selectedIndex = filteredSongs.count - 1
        pageOffset = max(0, filteredSongs.count - pageSize)
    }

    private func pageUp() {
        pageOffset = max(0, pageOffset - pageSize)
        selectedIndex = max(0, min(selectedIndex, pageOffset + pageSize - 1))
    }

    private func pageDown() {
        let maxOffset = max(0, filteredSongs.count - pageSize)
        pageOffset = min(maxOffset, pageOffset + pageSize)
        selectedIndex = min(filteredSongs.count - 1, pageOffset)
    }
}
