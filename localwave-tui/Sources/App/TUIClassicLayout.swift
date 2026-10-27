//
//  TUIClassicLayout.swift
//  localwave-tui
//
//  Created by Claude Code on 05.10.2025.
//
//  iPod/WinAmp inspired 2-column layout

import SwiftTUI
import Foundation
import LocalWaveDomain
import LocalWaveCore

/// Main section types for the sidebar
enum SidebarSection: String, CaseIterable {
    case songs = "Songs"
    case artists = "Artists"
    case albums = "Albums"
    case playlists = "Playlists"

    var icon: String {
        switch self {
        case .songs: return "♪"
        case .artists: return "♫"
        case .albums: return "⊞"
        case .playlists: return "≡"
        }
    }
}

/// Classic iPod/WinAmp style layout
@MainActor
struct TUIClassicLayout: View {
    let dependencies: TUIDependencyContainer

    private var theme: TUITheme { dependencies.theme }

    @State private var selectedSection: SidebarSection = .songs
    @State private var sidebarSelectedIndex = 0
    @State private var songListState = TUIListState(selectedIndex: 0, scrollOffset: 0, visibleHeight: 30)
    @State private var focusedPane: FocusedPane = .songList
    @State private var windowSwitchMode = false

    // Mock data
    @State private var isPlaying = true
    @State private var currentTime: TimeInterval = 145
    @State private var totalTime: TimeInterval = 243

    enum FocusedPane {
        case sidebar
        case songList
    }

    var body: some View {
        VStack(spacing: 0) {
            // Main content area - 2 columns
            HStack(spacing: 0) {
                // Left sidebar - 1/3 width (about 25-30 chars)
                sidebar
                    .frame(width: 30)
                    .background(theme.base)
                    .border(focusedPane == .sidebar ? theme.yellow : theme.overlay0)

                // Right content - 2/3 width
                VStack(spacing: 0) {
                    // Top: WinAmp-style player bar
                    playerBar
                        .background(theme.surface0)
                        .border(theme.overlay0)

                    // Bottom: Song list
                    songList
                        .background(theme.base)
                        .border(focusedPane == .songList ? theme.yellow : theme.overlay0)
                }
            }

            // Bottom: Status bar / help
            statusBar
                .background(theme.surface0)
        }
        .onKeyPress("\u{17}") { windowSwitchMode = true }  // Ctrl+W
        .onKeyPress("h") { handleH() }
        .onKeyPress("l") { handleL() }
        .onKeyPress("1") { selectedSection = .songs }
        .onKeyPress("2") { selectedSection = .artists }
        .onKeyPress("3") { selectedSection = .albums }
        .onKeyPress("4") { selectedSection = .playlists }
        .onKeyPress("j") { handleJ() }
        .onKeyPress("k") { handleK() }
        .onKeyPress("\u{04}") { handleCtrlD() }  // Ctrl+D - page down
        .onKeyPress("\u{15}") { handleCtrlU() }  // Ctrl+U - page up
        .onKeyPress(" ") { isPlaying.toggle() }
        .onKeyPress("q") { exit(0) }
    }

    // MARK: - Navigation Handlers

    private func handleH() {
        if windowSwitchMode {
            focusedPane = .sidebar
            windowSwitchMode = false
        }
    }

    private func handleL() {
        if windowSwitchMode {
            focusedPane = .songList
            windowSwitchMode = false
        }
    }

    private func handleJ() {
        windowSwitchMode = false
        switch focusedPane {
        case .sidebar:
            let currentIndex = SidebarSection.allCases.firstIndex(of: selectedSection) ?? 0
            if currentIndex < SidebarSection.allCases.count - 1 {
                selectedSection = SidebarSection.allCases[currentIndex + 1]
            }
        case .songList:
            // Manual inline navigation to ensure @State updates
            guard mockSongs.count > 0 else { return }
            let visibleHeight = max(5, dependencies.terminalSizeTracker.height - 10)
            let newIndex = min(songListState.selectedIndex + 1, mockSongs.count - 1)
            songListState.selectedIndex = newIndex

            // Auto-scroll if selection goes below visible area
            if newIndex >= songListState.scrollOffset + visibleHeight {
                songListState.scrollOffset = max(0, newIndex - visibleHeight + 1)
            }
        }
    }

    private func handleK() {
        windowSwitchMode = false
        switch focusedPane {
        case .sidebar:
            let currentIndex = SidebarSection.allCases.firstIndex(of: selectedSection) ?? 0
            if currentIndex > 0 {
                selectedSection = SidebarSection.allCases[currentIndex - 1]
            }
        case .songList:
            // Manual inline navigation to ensure @State updates
            let newIndex = max(songListState.selectedIndex - 1, 0)
            songListState.selectedIndex = newIndex

            // Auto-scroll if selection goes above visible area
            if newIndex < songListState.scrollOffset {
                songListState.scrollOffset = newIndex
            }
        }
    }

    private func handleCtrlD() {
        windowSwitchMode = false
        guard focusedPane == .songList, mockSongs.count > 0 else { return }

        let visibleHeight = max(5, dependencies.terminalSizeTracker.height - 10)
        let pageSize = visibleHeight / 2  // Half page down
        let newIndex = min(songListState.selectedIndex + pageSize, mockSongs.count - 1)

        songListState.selectedIndex = newIndex

        // Auto-scroll if selection goes below visible area
        if newIndex >= songListState.scrollOffset + visibleHeight {
            let newScrollOffset = max(0, min(newIndex - visibleHeight + 1, mockSongs.count - visibleHeight))
            songListState.scrollOffset = newScrollOffset
        }
    }

    private func handleCtrlU() {
        windowSwitchMode = false
        guard focusedPane == .songList else { return }

        let visibleHeight = max(5, dependencies.terminalSizeTracker.height - 10)
        let pageSize = visibleHeight / 2  // Half page up
        let newIndex = max(songListState.selectedIndex - pageSize, 0)
        songListState.selectedIndex = newIndex

        // Auto-scroll if selection goes above visible area
        if newIndex < songListState.scrollOffset {
            songListState.scrollOffset = newIndex
        }
    }

    // MARK: - Sidebar

    private var sidebar: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text(" LIBRARY").bold()
                Spacer()
            }
            .foregroundColor(theme.blue)

            Text(String(repeating: "─", count: 28))
                .foregroundColor(theme.overlay1)

            // Sections
            ForEach(Array(SidebarSection.allCases.enumerated()), id: \.offset) { index, section in
                sidebarItem(section: section, isSelected: selectedSection == section)
            }

            Spacer()

            // Stats
            Text(String(repeating: "─", count: 28))
                .foregroundColor(theme.overlay1)
            HStack {
                Text(" ♪ \(mockSongs.count) songs")
                Spacer()
            }
            .foregroundColor(theme.subtext0)
        }
    }

    private func sidebarItem(section: SidebarSection, isSelected: Bool) -> some View {
        HStack {
            Text(isSelected ? "►" : " ")
                .foregroundColor(theme.green)
            Text(section.icon)
                .foregroundColor(isSelected ? theme.sapphire : theme.subtext0)
            Text(section.rawValue)
                .foregroundColor(isSelected ? theme.text : theme.subtext0)
                .bold(isSelected)
            Spacer()
        }
        .background(isSelected ? theme.surface1 : Color.clear)
    }

    // MARK: - Player Bar

    private var playerBar: some View {
        VStack(spacing: 0) {
            // Title row
            HStack {
                Text("  ♪")
                    .foregroundColor(theme.green)
                Text(" ")
                Text(isPlaying ? "▶" : "⏸")
                    .foregroundColor(theme.peach)
                Text(" ")
                Text("Bohemian Rhapsody - Queen")
                    .foregroundColor(theme.text)
                    .bold()
                Spacer()
                Text(formatTime(currentTime))
                    .foregroundColor(theme.mauve)
                Text(" / ")
                    .foregroundColor(theme.subtext0)
                Text(formatTime(totalTime))
                    .foregroundColor(theme.subtext0)
                Text("  ")
            }

            // Progress bar with spectrum visualizer
            HStack(spacing: 0) {
                Text("  [")
                    .foregroundColor(theme.sky)

                // Progress
                progressBar

                Text("]")
                    .foregroundColor(theme.sky)

                Text("   ")

                // Spectrum visualizer
                spectrumVisualizer
                    .foregroundColor(theme.teal)

                Text("  ")
            }
        }
        .background(theme.base)
    }

    private var progressBar: some View {
        let totalWidth = 40
        let progress = currentTime / totalTime
        let filledWidth = max(0, min(totalWidth, Int(Double(totalWidth) * progress)))
        let emptyWidth = max(0, totalWidth - filledWidth)

        return HStack(spacing: 0) {
            Text(String(repeating: "━", count: filledWidth))
                .foregroundColor(theme.peach)
            Text(String(repeating: "─", count: emptyWidth))
                .foregroundColor(theme.overlay0)
        }
    }

    private var spectrumVisualizer: some View {
        // WinAmp-style spectrum analyzer
        let bars = mockSpectrumData
        var result = ""
        for height in bars {
            result += spectrumBar(height: height)
        }
        return Text(result)
    }

    private func spectrumBar(height: Int) -> String {
        // Using braille-like dotted characters for better visualization
        switch height {
        case 0: return "⡀"
        case 1: return "⡄"
        case 2: return "⡆"
        case 3: return "⡇"
        case 4: return "⣇"
        case 5: return "⣧"
        case 6: return "⣷"
        default: return "⣿"
        }
    }

    // MARK: - Song List

    private var songList: some View {
        let terminalWidth = max(50, dependencies.terminalSizeTracker.width)  // Ensure minimum terminal size
        let terminalHeight = max(20, dependencies.terminalSizeTracker.height)
        let sidebarWidth = 30
        let availableWidth = max(10, terminalWidth - sidebarWidth - 4)  // Ensure positive width
        let titleWidth = max(5, Int(Double(availableWidth) * 0.5))
        let artistWidth = max(5, Int(Double(availableWidth) * 0.35))

        // Calculate visible height: total height - player bar (3) - header (2) - status (3) - borders (2)
        let calculatedHeight = max(5, terminalHeight - 10)

        return VStack(spacing: 0) {
            // Header
            HStack {
                Text("  #")
                Text("  ")
                Text("Title")
                    .bold()
                Spacer()
                Text("Artist")
                Text("   ")
                Text("Time")
                Text(" ")
            }
            .foregroundColor(theme.blue)

            Text(String(repeating: "─", count: availableWidth))
                .foregroundColor(theme.overlay1)

            // Songs - use calculated height directly with bounds checking
            // Only render if we have items to show
            if !mockSongs.isEmpty && calculatedHeight > 0 {
                // ULTRA-DEFENSIVE: Ensure all values are valid before creating Range
                let safeScrollOffset = max(0, min(songListState.scrollOffset, mockSongs.count - 1))
                let safeHeight = max(1, calculatedHeight)
                let safeCount = mockSongs.count

                // Calculate range bounds with overflow protection
                let rawEnd = safeScrollOffset + safeHeight
                let visibleStart = safeScrollOffset
                let visibleEnd = min(rawEnd, safeCount)

                // Triple-check range validity before creating ForEach
                let rangeValid = visibleStart >= 0 &&
                   visibleStart < safeCount &&
                   visibleEnd > 0 &&
                   visibleEnd <= safeCount &&
                   visibleStart < visibleEnd

                if rangeValid {
                    ForEach(visibleStart..<visibleEnd, id: \.self) { index in
                        songRow(
                            song: mockSongs[index],
                            index: index,
                            isSelected: index == songListState.selectedIndex,
                            titleWidth: titleWidth,
                            artistWidth: artistWidth
                        )
                    }
                } else {
                    Text("ERROR: Invalid range - skipping render")
                }
            }

            Spacer()
        }
    }

    private func songRow(song: MockSong, index: Int, isSelected: Bool, titleWidth: Int, artistWidth: Int) -> some View {
        HStack {
            // Selection indicator
            Text(isSelected ? "►" : " ")
                .foregroundColor(theme.green)

            // Track number
            Text(String(format: "%2d", index + 1))
                .foregroundColor(theme.subtext0)

            Text("  ")

            // Title
            Text(truncate(song.title, width: titleWidth))
                .foregroundColor(isSelected ? theme.text : theme.text)
                .bold(isSelected)

            Spacer()

            // Artist
            Text(truncate(song.artist, width: artistWidth))
                .foregroundColor(theme.teal)

            Text("   ")

            // Duration
            Text(song.duration)
                .foregroundColor(theme.mauve)

            Text(" ")
        }
        .background(isSelected ? theme.surface1 : Color.clear)
    }

    // MARK: - Status Bar

    private var statusBar: some View {
        let width = max(10, dependencies.terminalSizeTracker.width)

        return VStack(spacing: 0) {
            Text(String(repeating: "═", count: width))
                .foregroundColor(theme.overlay1)

            HStack {
                Group {
                    Text(" [1-4]")
                        .foregroundColor(theme.yellow)
                    Text("Sections  ")
                    Text("[j/k]")
                        .foregroundColor(theme.yellow)
                    Text("Nav  ")
                    Text("[^D/^U]")
                        .foregroundColor(theme.yellow)
                }
                Group {
                    Text("Page  ")
                    Text("[Space]")
                        .foregroundColor(theme.yellow)
                    Text("Play  ")
                    Text("[q]")
                        .foregroundColor(theme.yellow)
                    Text("Quit")
                }
                Spacer()
            }
            .foregroundColor(theme.text)
        }
        .background(theme.base)
    }

    // MARK: - Helpers

    private func formatTime(_ seconds: TimeInterval) -> String {
        let mins = Int(seconds) / 60
        let secs = Int(seconds) % 60
        return String(format: "%d:%02d", mins, secs)
    }

    private func truncate(_ text: String, width: Int) -> String {
        guard width > 0 else { return " " }
        if text.count > width {
            return String(text.prefix(max(0, width - 1))) + "…"
        }
        return text + String(repeating: " ", count: max(0, width - text.count))
    }
}

// MARK: - Mock Data

struct MockSong {
    let title: String
    let artist: String
    let duration: String
}

private let mockSongs: [MockSong] = [
    MockSong(title: "Bohemian Rhapsody", artist: "Queen", duration: "5:55"),
    MockSong(title: "Stairway to Heaven", artist: "Led Zeppelin", duration: "8:02"),
    MockSong(title: "Hotel California", artist: "Eagles", duration: "6:30"),
    MockSong(title: "Imagine", artist: "John Lennon", duration: "3:03"),
    MockSong(title: "Smells Like Teen Spirit", artist: "Nirvana", duration: "5:01"),
    MockSong(title: "Sweet Child O' Mine", artist: "Guns N' Roses", duration: "5:56"),
    MockSong(title: "Billie Jean", artist: "Michael Jackson", duration: "4:54"),
    MockSong(title: "Purple Haze", artist: "Jimi Hendrix", duration: "2:51"),
    MockSong(title: "Like a Rolling Stone", artist: "Bob Dylan", duration: "6:13"),
    MockSong(title: "Hey Jude", artist: "The Beatles", duration: "7:11"),
    MockSong(title: "What's Going On", artist: "Marvin Gaye", duration: "3:53"),
    MockSong(title: "Born to Run", artist: "Bruce Springsteen", duration: "4:31"),
    MockSong(title: "Thunder Road", artist: "Bruce Springsteen", duration: "4:49"),
    MockSong(title: "Paranoid Android", artist: "Radiohead", duration: "6:23"),
    MockSong(title: "Champagne Supernova", artist: "Oasis", duration: "7:27"),
    MockSong(title: "Wonderwall", artist: "Oasis", duration: "4:18"),
    MockSong(title: "Under the Bridge", artist: "Red Hot Chili Peppers", duration: "4:24"),
    MockSong(title: "Tears in Heaven", artist: "Eric Clapton", duration: "4:32"),
    MockSong(title: "Black", artist: "Pearl Jam", duration: "5:43"),
    MockSong(title: "November Rain", artist: "Guns N' Roses", duration: "8:57"),
    MockSong(title: "Comfortably Numb", artist: "Pink Floyd", duration: "6:23"),
    MockSong(title: "Wish You Were Here", artist: "Pink Floyd", duration: "5:34"),
    MockSong(title: "The Sound of Silence", artist: "Simon & Garfunkel", duration: "3:08"),
    MockSong(title: "Bridge Over Troubled Water", artist: "Simon & Garfunkel", duration: "4:52"),
    MockSong(title: "Hallelujah", artist: "Leonard Cohen", duration: "4:36"),
    MockSong(title: "Your Song", artist: "Elton John", duration: "4:01"),
    MockSong(title: "Tiny Dancer", artist: "Elton John", duration: "6:16"),
    MockSong(title: "Rocket Man", artist: "Elton John", duration: "4:41"),
    MockSong(title: "Let It Be", artist: "The Beatles", duration: "4:03"),
    MockSong(title: "Yesterday", artist: "The Beatles", duration: "2:05"),
    MockSong(title: "Come Together", artist: "The Beatles", duration: "4:20"),
    MockSong(title: "While My Guitar Gently Weeps", artist: "The Beatles", duration: "4:45"),
    MockSong(title: "Sweet Home Alabama", artist: "Lynyrd Skynyrd", duration: "4:43"),
    MockSong(title: "Free Bird", artist: "Lynyrd Skynyrd", duration: "9:08"),
    MockSong(title: "Dream On", artist: "Aerosmith", duration: "4:27"),
    MockSong(title: "Walk This Way", artist: "Aerosmith", duration: "3:40"),
    MockSong(title: "Back in Black", artist: "AC/DC", duration: "4:15"),
    MockSong(title: "Highway to Hell", artist: "AC/DC", duration: "3:28"),
    MockSong(title: "Thunderstruck", artist: "AC/DC", duration: "4:52"),
    MockSong(title: "Smoke on the Water", artist: "Deep Purple", duration: "5:40"),
    MockSong(title: "Enter Sandman", artist: "Metallica", duration: "5:31"),
    MockSong(title: "Nothing Else Matters", artist: "Metallica", duration: "6:28"),
    MockSong(title: "One", artist: "Metallica", duration: "7:27"),
    MockSong(title: "Master of Puppets", artist: "Metallica", duration: "8:35"),
    MockSong(title: "Kashmir", artist: "Led Zeppelin", duration: "8:37"),
    MockSong(title: "Black Dog", artist: "Led Zeppelin", duration: "4:54"),
    MockSong(title: "Whole Lotta Love", artist: "Led Zeppelin", duration: "5:34"),
    MockSong(title: "Immigrant Song", artist: "Led Zeppelin", duration: "2:26"),
    MockSong(title: "Don't Stop Believin'", artist: "Journey", duration: "4:10"),
    MockSong(title: "Separate Ways", artist: "Journey", duration: "5:29"),
    MockSong(title: "Every Breath You Take", artist: "The Police", duration: "4:13"),
    MockSong(title: "Message in a Bottle", artist: "The Police", duration: "4:51"),
    MockSong(title: "Roxanne", artist: "The Police", duration: "3:12"),
    MockSong(title: "Africa", artist: "Toto", duration: "4:55"),
    MockSong(title: "Rosanna", artist: "Toto", duration: "5:32"),
    MockSong(title: "Hold the Line", artist: "Toto", duration: "3:56"),
    MockSong(title: "Take On Me", artist: "A-ha", duration: "3:47"),
    MockSong(title: "Sweet Dreams", artist: "Eurythmics", duration: "3:36"),
    MockSong(title: "Livin' on a Prayer", artist: "Bon Jovi", duration: "4:09"),
    MockSong(title: "You Give Love a Bad Name", artist: "Bon Jovi", duration: "3:43"),
    MockSong(title: "Wanted Dead or Alive", artist: "Bon Jovi", duration: "5:09"),
    MockSong(title: "With or Without You", artist: "U2", duration: "4:56"),
    MockSong(title: "Where the Streets Have No Name", artist: "U2", duration: "5:37"),
    MockSong(title: "One", artist: "U2", duration: "4:36"),
    MockSong(title: "I Still Haven't Found What I'm Looking For", artist: "U2", duration: "4:38"),
    MockSong(title: "Sunday Bloody Sunday", artist: "U2", duration: "4:40"),
    MockSong(title: "Enjoy the Silence", artist: "Depeche Mode", duration: "6:13"),
    MockSong(title: "Personal Jesus", artist: "Depeche Mode", duration: "4:56"),
    MockSong(title: "Just Like Heaven", artist: "The Cure", duration: "3:32"),
    MockSong(title: "Friday I'm in Love", artist: "The Cure", duration: "3:37"),
    MockSong(title: "Creep", artist: "Radiohead", duration: "3:58"),
    MockSong(title: "Karma Police", artist: "Radiohead", duration: "4:21"),
    MockSong(title: "No Surprises", artist: "Radiohead", duration: "3:48"),
    MockSong(title: "Losing My Religion", artist: "R.E.M.", duration: "4:27"),
    MockSong(title: "Everybody Hurts", artist: "R.E.M.", duration: "5:17"),
    MockSong(title: "Man in the Box", artist: "Alice in Chains", duration: "4:46"),
    MockSong(title: "Rooster", artist: "Alice in Chains", duration: "6:15"),
    MockSong(title: "Would?", artist: "Alice in Chains", duration: "3:28"),
    MockSong(title: "Heart-Shaped Box", artist: "Nirvana", duration: "4:41"),
    MockSong(title: "Come As You Are", artist: "Nirvana", duration: "3:39"),
    MockSong(title: "Lithium", artist: "Nirvana", duration: "4:17"),
    MockSong(title: "In Bloom", artist: "Nirvana", duration: "4:14"),
    MockSong(title: "Californication", artist: "Red Hot Chili Peppers", duration: "5:21"),
    MockSong(title: "Scar Tissue", artist: "Red Hot Chili Peppers", duration: "3:37"),
    MockSong(title: "Give It Away", artist: "Red Hot Chili Peppers", duration: "4:43"),
    MockSong(title: "Otherside", artist: "Red Hot Chili Peppers", duration: "4:15"),
    MockSong(title: "Jeremy", artist: "Pearl Jam", duration: "5:18"),
    MockSong(title: "Alive", artist: "Pearl Jam", duration: "5:41"),
    MockSong(title: "Even Flow", artist: "Pearl Jam", duration: "4:53"),
    MockSong(title: "Yellow Ledbetter", artist: "Pearl Jam", duration: "5:00"),
    MockSong(title: "Plush", artist: "Stone Temple Pilots", duration: "5:13"),
    MockSong(title: "Interstate Love Song", artist: "Stone Temple Pilots", duration: "3:15"),
    MockSong(title: "Mr. Brightside", artist: "The Killers", duration: "3:42"),
    MockSong(title: "Somebody Told Me", artist: "The Killers", duration: "3:18"),
    MockSong(title: "Sex on Fire", artist: "Kings of Leon", duration: "3:23"),
    MockSong(title: "Use Somebody", artist: "Kings of Leon", duration: "3:51"),
    MockSong(title: "Seven Nation Army", artist: "The White Stripes", duration: "3:51"),
    MockSong(title: "Fell in Love with a Girl", artist: "The White Stripes", duration: "1:50"),
]

// Mock spectrum data (animates in real app)
private let mockSpectrumData: [Int] = [2, 5, 7, 6, 4, 6, 8, 5, 3, 6, 7, 4, 2, 5, 6]
