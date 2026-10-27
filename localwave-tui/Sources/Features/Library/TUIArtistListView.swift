//
//  TUIArtistListView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI view for displaying list of artists
///
/// Features:
/// - Scrollable list of artists with song counts
/// - Search functionality with '/' key
/// - Navigate to artist detail on Enter
/// - Pagination with u/d keys
/// - Vim-style navigation (j/k/g/G)
struct TUIArtistListView: View {
    let dependencies: TUIDependencyContainer
    @Binding var navigationState: NavigationState

    // State
    @State private var artists: [String] = []
    @State private var artistSongCounts: [String: Int] = [:]
    @State private var selectedIndex = 0
    @State private var isLoading = false
    @State private var errorMessage: String?

    // Pagination
    @State private var pageOffset = 0
    private let pageSize = 10

    // Search
    @State private var searchMode = false
    @State private var searchTerm = ""

    var filteredArtists: [String] {
        guard !searchTerm.isEmpty else { return artists }
        return artists.filter { $0.localizedCaseInsensitiveContains(searchTerm) }
    }

    var paginatedArtists: ArraySlice<String> {
        let start = pageOffset
        let end = min(start + pageSize, filteredArtists.count)
        guard start < filteredArtists.count else { return [] }
        return filteredArtists[start..<end]
    }

    var body: some View {
        VStack(spacing: 1) {
            // Header
            HStack {
                Text("Artists (\(filteredArtists.count))")
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
            } else if filteredArtists.isEmpty {
                emptyView
            } else {
                artistListView
            }

            Spacer()
            helpBar
        }
        .onAppear {
            Task { await loadArtists() }
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
        .onKeyPress("l") { if !searchMode { openSelected() } }
        .onKeyPress("\r") { if !searchMode { openSelected() } }
        .onKeyPress("r") { if !searchMode { Task { await loadArtists() } } }
        .onKeyPress("/") { if !searchMode { searchMode = true } }
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
                TextField(placeholder: "Type artist name...") { query in
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
            Text("Loading artists...")
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
                Text("No artists found")
                Text("")
                Text("Add a music source with audio files to populate artists")
            } else {
                Text("No artists match \"\(searchTerm)\"")
                Text("")
                Text("Press Esc to clear search")
            }
            Spacer()
        }
    }

    private var artistListView: some View {
        VStack(spacing: 0) {
            // Page indicator
            if filteredArtists.count > pageSize {
                HStack {
                    Text("Showing \(pageOffset + 1)-\(min(pageOffset + pageSize, filteredArtists.count)) of \(filteredArtists.count)")
                    Text(" [u/d for pages]")
                    Spacer()
                }
                Text("")
            }

            ForEach(Array(paginatedArtists.enumerated()), id: \.element) { index, artist in
                let globalIndex = pageOffset + index
                let isSelected = globalIndex == selectedIndex
                artistRow(artist: artist, isSelected: isSelected)
            }
            Spacer()
        }
    }

    private func artistRow(artist: String, isSelected: Bool) -> some View {
        HStack {
            Text(isSelected ? " > " : "   ")
            Text(TUITheme.Icons.artist)
            Text(" ")
            Text(TUITheme.truncate(artist, width: 40))
            Spacer()
            if let count = artistSongCounts[artist] {
                Text("(\(count) songs)")
            }
        }
    }

    private var helpBar: some View {
        VStack(spacing: 0) {
            Text(TUITheme.divider(width: 80))
            HStack {
                if searchMode {
                    Text("[Esc] Exit Search")
                } else {
                    Text("[j/k] Navigate")
                    Text("  ")
                    Text("[l/Enter] Open")
                    Text("  ")
                    Text("[/] Search")
                    Text("  ")
                    Text("[r] Refresh")
                }
                Spacer()
            }
        }
    }

    // MARK: - Actions

    private func loadArtists() async {
        isLoading = true
        errorMessage = nil

        do {
            let songRepo = dependencies.songRepository
            artists = try await songRepo.getAllArtists()

            // Load song counts for all artists (using concrete SQLiteSongRepository)
            if let sqliteRepo = songRepo as? SQLiteSongRepository {
                for artist in artists {
                    let songs = try await sqliteRepo.getSongsByArtist(artist)
                    artistSongCounts[artist] = songs.count
                }
            }

            selectedIndex = 0
            pageOffset = 0
        } catch {
            errorMessage = error.localizedDescription
        }

        isLoading = false
    }

    private func openSelected() {
        guard selectedIndex < filteredArtists.count else { return }
        let artist = filteredArtists[selectedIndex]
        navigationState.push(.artistDetail(artist: artist))
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
            // Exit artist list view
            navigationState.pop()
        }
    }

    // MARK: - Navigation Helpers

    private func selectNext() {
        guard !filteredArtists.isEmpty else { return }
        if selectedIndex < filteredArtists.count - 1 {
            selectedIndex += 1
            // Auto-scroll to next page if needed
            if selectedIndex >= pageOffset + pageSize {
                pageOffset = min(selectedIndex, filteredArtists.count - pageSize)
            }
        }
    }

    private func selectPrevious() {
        guard !filteredArtists.isEmpty else { return }
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
        guard !filteredArtists.isEmpty else { return }
        selectedIndex = filteredArtists.count - 1
        pageOffset = max(0, filteredArtists.count - pageSize)
    }

    private func pageUp() {
        pageOffset = max(0, pageOffset - pageSize)
        selectedIndex = max(0, min(selectedIndex, pageOffset + pageSize - 1))
    }

    private func pageDown() {
        let maxOffset = max(0, filteredArtists.count - pageSize)
        pageOffset = min(maxOffset, pageOffset + pageSize)
        selectedIndex = min(filteredArtists.count - 1, pageOffset)
    }
}
