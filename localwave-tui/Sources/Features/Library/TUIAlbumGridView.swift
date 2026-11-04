//
//  TUIAlbumGridView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI view for displaying list of albums
///
/// Features:
/// - Scrollable list of albums with artist, year, and song counts
/// - Search functionality with '/' key
/// - Navigate to album detail on Enter
/// - Pagination with u/d keys
/// - Vim-style navigation (j/k/g/G)
/// - Sorting options (name, artist, year)
struct TUIAlbumGridView: View {
    let dependencies: TUIDependencyContainer
    @Binding var navigationState: NavigationState

    // State
    @State private var albums: [Album] = []
    @State private var albumSongCounts: [String: Int] = [:]
    @State private var albumYears: [String: Int] = [:]
    @State private var selectedIndex = 0
    @State private var isLoading = false
    @State private var errorMessage: String?

    // Pagination
    @State private var pageOffset = 0
    private let pageSize = 10

    // Search
    @State private var searchMode = false
    @State private var searchTerm = ""

    // Sorting
    @State private var sortMode: SortMode = .name

    enum SortMode: String {
        case name = "Name"
        case artist = "Artist"
        case year = "Year"
    }

    var filteredAlbums: [Album] {
        guard !searchTerm.isEmpty else { return sortedAlbums }
        return sortedAlbums.filter { album in
            album.name.localizedCaseInsensitiveContains(searchTerm) ||
            album.artist?.localizedCaseInsensitiveContains(searchTerm) ?? false
        }
    }

    var sortedAlbums: [Album] {
        switch sortMode {
        case .name:
            return albums.sorted { $0.name < $1.name }
        case .artist:
            return albums.sorted { ($0.artist ?? "") < ($1.artist ?? "") }
        case .year:
            return albums.sorted { (albumYears[$0.id] ?? 0) > (albumYears[$1.id] ?? 0) }
        }
    }

    var paginatedAlbums: ArraySlice<Album> {
        let start = pageOffset
        let end = min(start + pageSize, filteredAlbums.count)
        guard start < filteredAlbums.count else { return [] }
        return filteredAlbums[start..<end]
    }

    var body: some View {
        VStack(spacing: 1) {
            // Header
            HStack {
                Text("Albums (\(filteredAlbums.count))")
                Text(" - Sort: \(sortMode.rawValue)")
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
            } else if filteredAlbums.isEmpty {
                emptyView
            } else {
                albumListView
            }

            Spacer()
            helpBar
        }
        .onAppear {
            Task { await loadAlbums() }
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
        .onKeyPress("r") { if !searchMode { Task { await loadAlbums() } } }
        .onKeyPress("s") { if !searchMode { cycleSortMode() } }
        .onKeyPress("S") { if !searchMode { cycleSortMode() } }
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
                TextField(placeholder: "Type album or artist name...") { query in
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
            Text("Loading albums...")
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
                Text("No albums found")
                Text("")
                Text("Add a music source with audio files to populate albums")
            } else {
                Text("No albums match \"\(searchTerm)\"")
                Text("")
                Text("Press Esc to clear search")
            }
            Spacer()
        }
    }

    private var albumListView: some View {
        VStack(spacing: 0) {
            // Page indicator
            if filteredAlbums.count > pageSize {
                HStack {
                    Text("Showing \(pageOffset + 1)-\(min(pageOffset + pageSize, filteredAlbums.count)) of \(filteredAlbums.count)")
                    Text(" [u/d for pages]")
                    Spacer()
                }
                Text("")
            }

            // Table header
            HStack {
                Text(" Album                ")
                Text("Artist              ")
                Text("Year    ")
                Text("Songs")
                Spacer()
            }
            HStack {
                Text(TUITheme.divider(width: 80))
                Spacer()
            }

            ForEach(Array(paginatedAlbums.enumerated()), id: \.element.id) { index, album in
                let globalIndex = pageOffset + index
                let isSelected = globalIndex == selectedIndex
                albumRow(album: album, isSelected: isSelected)
            }
            Spacer()
        }
    }

    private func albumRow(album: Album, isSelected: Bool) -> some View {
        HStack {
            Text(isSelected ? " > " : "   ")
            Text(TUITheme.truncate(album.name, width: 20))
            Text(" ")
            Text(TUITheme.truncate(album.artist ?? "Unknown Artist", width: 20))
            Text(" ")
            if let year = albumYears[album.id] {
                Text(String(year))
            } else {
                Text("    ")
            }
            Text("    ")
            if let count = albumSongCounts[album.id] {
                Text(String(count))
            } else {
                Text(" ")
            }
            Spacer()
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
                    Text("[s] Sort")
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

    private func loadAlbums() async {
        isLoading = true
        errorMessage = nil

        do {
            let songRepo = await dependencies.songRepository
            albums = try await songRepo.getAllAlbums()

            // Load song counts and years for all albums
            if let sqliteRepo = songRepo as? SQLiteSongRepository {
                for album in albums {
                    // Get songs for this album to count and get year
                    let songs = try await sqliteRepo.getSongsByAlbum(album.name, artist: album.artist)
                    albumSongCounts[album.id] = songs.count

                    // Get year from first song with releaseYear
                    if let firstSongWithYear = songs.first(where: { $0.releaseYear != nil }),
                       let year = firstSongWithYear.releaseYear {
                        albumYears[album.id] = year
                    }
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
        guard selectedIndex < filteredAlbums.count else { return }
        let album = filteredAlbums[selectedIndex]
        navigationState.push(.albumDetail(album: album.name, artist: album.artist))
    }

    private func cycleSortMode() {
        switch sortMode {
        case .name:
            sortMode = .artist
        case .artist:
            sortMode = .year
        case .year:
            sortMode = .name
        }
        // Reset pagination after sorting
        selectedIndex = 0
        pageOffset = 0
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
            // Exit album list view
            navigationState.pop()
        }
    }

    // MARK: - Navigation Helpers

    private func selectNext() {
        guard !filteredAlbums.isEmpty else { return }
        if selectedIndex < filteredAlbums.count - 1 {
            selectedIndex += 1
            // Auto-scroll to next page if needed
            if selectedIndex >= pageOffset + pageSize {
                pageOffset = min(selectedIndex, filteredAlbums.count - pageSize)
            }
        }
    }

    private func selectPrevious() {
        guard !filteredAlbums.isEmpty else { return }
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
        guard !filteredAlbums.isEmpty else { return }
        selectedIndex = filteredAlbums.count - 1
        pageOffset = max(0, filteredAlbums.count - pageSize)
    }

    private func pageUp() {
        pageOffset = max(0, pageOffset - pageSize)
        selectedIndex = max(0, min(selectedIndex, pageOffset + pageSize - 1))
    }

    private func pageDown() {
        let maxOffset = max(0, filteredAlbums.count - pageSize)
        pageOffset = min(maxOffset, pageOffset + pageSize)
        selectedIndex = min(filteredAlbums.count - 1, pageOffset)
    }
}
