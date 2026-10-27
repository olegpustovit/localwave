//
//  TabBar.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI

/// Available tabs in the TUI application
public enum Tab: Int, CaseIterable, Hashable {
    case sync = 1
    case artists = 2
    case albums = 3
    case songs = 4
    case playlists = 5
    case player = 6

    /// Display name for the tab
    public var title: String {
        switch self {
        case .sync: return "Sync"
        case .artists: return "Artists"
        case .albums: return "Albums"
        case .songs: return "Songs"
        case .playlists: return "Playlists"
        case .player: return "Player"
        }
    }

    /// Keyboard shortcut number for quick switching
    public var shortcut: String {
        "\(rawValue)"
    }

    /// Icon/indicator for the tab
    public var icon: String {
        switch self {
        case .sync: return "⟳"
        case .artists: return "♪"
        case .albums: return "◉"
        case .songs: return "♫"
        case .playlists: return "☰"
        case .player: return "▶"
        }
    }
}

/// Tab bar state
public struct TabBarState {
    /// Currently selected tab
    public var selectedTab: Tab = .sync

    /// Previous tab (for tracking)
    public var previousTab: Tab?

    public init(selectedTab: Tab = .artists, previousTab: Tab? = nil) {
        self.selectedTab = selectedTab
        self.previousTab = previousTab
    }

    /// Switch to a specific tab
    public mutating func selectTab(_ tab: Tab) {
        guard selectedTab != tab else { return }
        previousTab = selectedTab
        selectedTab = tab
    }

    /// Handle keyboard shortcut input (1-5)
    public mutating func handleShortcut(_ key: Character) -> Bool {
        guard let number = Int(String(key)),
              let tab = Tab(rawValue: number) else {
            return false
        }
        selectTab(tab)
        return true
    }
}

/// Tab bar view component with keyboard shortcuts
public struct TabBarView: View {
    @Binding var state: TabBarState

    public init(state: Binding<TabBarState>) {
        self._state = state
    }

    public var body: some View {
        HStack(spacing: 2) {
            ForEach(Tab.allCases, id: \.self) { tab in
                TabButton(
                    tab: tab,
                    isSelected: state.selectedTab == tab,
                    action: { state.selectTab(tab) }
                )
            }
        }
    }
}

/// Individual tab button
struct TabButton: View {
    let tab: Tab
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        HStack(spacing: 1) {
            Text(isSelected ? "[\(tab.shortcut)]" : " \(tab.shortcut) ")
            Text(tab.icon)
            Text(tab.title)
            Spacer()
        }
        .frame(width: 15)
    }
}

/// Tab view container that displays content based on selected tab
public struct TUITabView<Content: View>: View {
    @Binding var tabState: TabBarState
    let content: (Tab) -> Content

    public init(
        tabState: Binding<TabBarState>,
        @ViewBuilder content: @escaping (Tab) -> Content
    ) {
        self._tabState = tabState
        self.content = content
    }

    public var body: some View {
        VStack(spacing: 0) {
            // Tab bar at top
            TabBarView(state: $tabState)
                .frame(height: 1)

            // Divider
            Divider()

            // Content for selected tab
            content(tabState.selectedTab)
        }
    }
}

/// Divider component for visual separation
struct Divider: View {
    var body: some View {
        HStack {
            Text(String(repeating: "─", count: 80))
        }
        .frame(height: 1)
    }
}
