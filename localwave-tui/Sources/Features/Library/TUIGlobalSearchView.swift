//
//  TUIGlobalSearchView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI global search view with real-time FTS5 search
///
/// Features:
/// - Real-time search as user types
/// - FTS5 full-text search across artist/title/album
/// - Display results with match count
/// - Navigate to song details or play
/// - Vim-style navigation (j/k/g/G)
struct TUIGlobalSearchView: View {
    let dependencies: TUIDependencyContainer
    @Binding var navigationState: NavigationState

    // State
    @State private var searchQuery = ""
    @State private var results: [Song] = []
    @State private var selectedIndex = 0
    @State private var isSearching = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(spacing: 1) {
            // Search box
            searchBoxView

            Text("")

            // Results
            if isSearching {
                loadingView
            } else if let error = errorMessage {
                errorView(error)
            } else if searchQuery.isEmpty {
                promptView
            } else if results.isEmpty {
                noResultsView
            } else {
                resultsListView
            }

            Spacer()
            helpBar
        }
        // Navigation
        .onKeyPress("j") { selectNext() }
        .onKeyPress("k") { selectPrevious() }
        .onKeyPress("g") { selectFirst() }
        .onKeyPress("G") { selectLast() }
        // Actions
        .onKeyPress("\r") { openSelected() }
        .onKeyPress(" ") { Task { @MainActor in playSong() } }
        .onKeyPress("q") { Task { @MainActor in queueSong() } }
        .onKeyPress("Q") { Task { @MainActor in queueSong() } }
        .onKeyPress("\u{1B}") { navigationState.pop() }
    }

    // MARK: - View Components

    private var searchBoxView: some View {
        VStack(spacing: 0) {
            HStack {
                Text(TUITheme.divider(width: 80))
                Spacer()
            }
            HStack {
                Text(" Search: ")
                TextField(placeholder: "Type to search artist, title, or album...") { query in
                    searchQuery = query
                    Task { await performSearch() }
                }
                Spacer()
            }
            HStack {
                Text(TUITheme.divider(width: 80))
                Spacer()
            }
        }
    }

    private var promptView: some View {
        VStack {
            Text("")
            Text("Type to search across all songs")
            Text("")
            Text("Search will match artist, title, and album")
            Spacer()
        }
    }

    private var loadingView: some View {
        VStack {
            Text("Searching...")
            Spacer()
        }
    }

    private func errorView(_ message: String) -> some View {
        VStack {
            HStack {
                Text(TUIColors.Indicators.error("Error: \(message)"))
                Spacer()
            }
            Spacer()
        }
    }

    private var noResultsView: some View {
        VStack {
            Text("")
            Text("No results found for \"\(searchQuery)\"")
            Text("")
            Text("Try different search terms")
            Spacer()
        }
    }

    private var resultsListView: some View {
        VStack(spacing: 0) {
            HStack {
                Text("\(results.count) results found:")
                Spacer()
            }
            Text("")

            ForEach(Array(results.prefix(10).enumerated()), id: \.element.id) { index, song in
                let isSelected = index == selectedIndex
                resultRow(song: song, isSelected: isSelected)
            }

            if results.count > 10 {
                Text("")
                HStack {
                    Text("(showing first 10 results)")
                    Spacer()
                }
            }

            Spacer()
        }
    }

    private func resultRow(song: Song, isSelected: Bool) -> some View {
        HStack {
            Text(isSelected ? " > " : "   ")
            Text(TUITheme.truncate(song.title, width: 25))
            Text(" - ")
            Text(TUITheme.truncate(song.artist, width: 20))
            Text(" - ")
            Text(TUITheme.truncate(song.album, width: 20))
            Spacer()
        }
    }

    private var helpBar: some View {
        VStack(spacing: 0) {
            Text(TUITheme.divider(width: 80))
            HStack {
                Text("[j/k] Navigate")
                Text("  ")
                Text("[Enter] Open")
                Text("  ")
                Text("[Space] Play")
                Text("  ")
                Text("[q] Queue")
                Text("  ")
                Text("[Esc] Close")
                Spacer()
            }
        }
    }

    // MARK: - Actions

    private func performSearch() async {
        guard !searchQuery.isEmpty else {
            results = []
            return
        }

        isSearching = true
        errorMessage = nil

        do {
            let songRepo = await dependencies.songRepository
            results = try await songRepo.searchSongsFTS(
                query: searchQuery,
                limit: 50,  // Get up to 50 results
                offset: 0
            )
            selectedIndex = 0
        } catch {
            errorMessage = error.localizedDescription
            results = []
        }

        isSearching = false
    }

    private func openSelected() {
        guard selectedIndex < results.count else { return }
        let song = results[selectedIndex]
        // Navigate to artist or album detail
        navigationState.push(.artistDetail(artist: song.artist))
    }

    @MainActor
    private func playSong() {
        guard selectedIndex < results.count else { return }
        let song = results[selectedIndex]

        // Configure queue with search results, starting from selected
        let playerVM = dependencies.playerViewModel
        playerVM.configureQueue(songs: results, startIndex: selectedIndex)
        playerVM.playSong(song)
    }

    @MainActor
    private func queueSong() {
        guard selectedIndex < results.count else { return }
        let song = results[selectedIndex]

        // Add song to the end of current queue
        let playerVM = dependencies.playerViewModel
        playerVM.addToQueue(song)
    }

    // MARK: - Navigation Helpers

    private func selectNext() {
        guard !results.isEmpty else { return }
        selectedIndex = min(selectedIndex + 1, min(9, results.count - 1))
    }

    private func selectPrevious() {
        guard !results.isEmpty else { return }
        selectedIndex = max(selectedIndex - 1, 0)
    }

    private func selectFirst() {
        selectedIndex = 0
    }

    private func selectLast() {
        guard !results.isEmpty else { return }
        selectedIndex = min(9, results.count - 1)
    }
}
