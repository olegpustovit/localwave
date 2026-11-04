//
//  TUIAlbumDetailView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI view for displaying songs in a specific album
///
/// Features:
/// - Show album info (name, artist, year) with song count
/// - List all songs with track numbers
/// - Scrollable song list with pagination
/// - Play album, queue, and playlist actions
/// - Vim-style navigation (j/k/g/G)
/// - Pagination with u/d keys
struct TUIAlbumDetailView: View {
    let dependencies: TUIDependencyContainer
    let albumName: String
    let artist: String?
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

    var albumYear: Int? {
        songs.first(where: { $0.releaseYear != nil })?.releaseYear
    }

    var totalDuration: String {
        // Note: Song model doesn't have duration property yet
        return "N/A"
    }

    var body: some View {
        VStack(spacing: 1) {
            // Header with album info
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
        .onKeyPress("a") { Task { @MainActor in playAlbum() } }
        .onKeyPress("A") { Task { @MainActor in queueAlbum() } }
        .onKeyPress("p") { addToPlaylist() }
        .onKeyPress("P") { addToPlaylist() }
        .onKeyPress("h") { navigationState.pop() }
        .onKeyPress("\u{1B}") { navigationState.pop() }
    }

    // MARK: - View Components

    private var headerView: some View {
        let headerText = buildHeaderText()
        return VStack(spacing: 0) {
            HStack {
                Text("LocalWave > Albums > \(albumName)")
                Spacer()
            }
            Text("")
            HStack {
                Text(TUIColors.Indicators.info(headerText))
                Spacer()
            }
        }
    }

    private func buildHeaderText() -> String {
        var header = "\(albumName)"
        if let artist = artist {
            header += " - \(artist)"
        }
        if let year = albumYear {
            header += " (\(year))"
        }
        header += " - \(songs.count) songs"
        if totalDuration != "N/A" {
            header += " - \(totalDuration)"
        }
        return header
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
            Text("No songs found in album \(albumName)")
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
            // Show track number if available
            if let trackNum = song.trackNumber {
                Text(String(format: "%2d.", trackNum))
            } else {
                Text("  .")
            }
            Text(" ")
            Text(TUITheme.truncate(song.title, width: 45))
            Spacer()
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
                Text("[a] Play Album")
                Text("  ")
                Text("[q] Queue")
                Text("  ")
                Text("[A] Queue Album")
                Spacer()
            }
            HStack {
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

            // Use SQLiteSongRepository to get songs by album
            if let sqliteRepo = songRepo as? SQLiteSongRepository {
                songs = try await sqliteRepo.getSongsByAlbum(albumName, artist: artist)
            } else {
                // Fallback: use FTS search with album filter
                var query = "album:\"\(albumName)\""
                if let artist = artist {
                    query += " artist:\"\(artist)\""
                }
                songs = try await songRepo.searchSongsFTS(
                    query: query,
                    limit: 1000,
                    offset: 0
                )
            }

            // Sort by track number
            songs.sort { s1, s2 in
                if let t1 = s1.trackNumber, let t2 = s2.trackNumber {
                    return t1 < t2
                }
                return s1.title.localizedStandardCompare(s2.title) == .orderedAscending
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

        // Configure queue with all album songs, starting from selected
        let playerVM = dependencies.playerViewModel
        playerVM.configureQueue(songs: songs, startIndex: selectedIndex)
        playerVM.playSong(song)
    }

    @MainActor
    private func playAlbum() {
        guard !songs.isEmpty else { return }

        // Play album from the beginning
        let playerVM = dependencies.playerViewModel
        playerVM.configureQueue(songs: songs, startIndex: 0)
        playerVM.playSong(songs[0])
    }

    @MainActor
    private func queueSong() {
        guard selectedIndex < songs.count else { return }
        let song = songs[selectedIndex]

        // Add song to the end of current queue
        let playerVM = dependencies.playerViewModel
        playerVM.addToQueue(song)
    }

    @MainActor
    private func queueAlbum() {
        guard !songs.isEmpty else { return }

        // Add all album songs to queue
        let playerVM = dependencies.playerViewModel
        for song in songs {
            playerVM.addToQueue(song)
        }
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
