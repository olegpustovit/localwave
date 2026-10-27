//
//  TUIMainView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI

/// Main view for the TUI application with tab bar and navigation
struct TUIMainView: View {
    let dependencies: TUIDependencyContainer

    @State var tabState = TabBarState()
    @State var navigationState = NavigationState()

    var body: some View {
        VStack(spacing: 0) {
            // Main content area with tabs and navigation
            TUITabView(tabState: $tabState) { selectedTab in
                NavigationContainer(navigationState: $navigationState) { navState in
                    contentView(for: selectedTab, navigationState: navState)
                }
            }

            // Mini player always visible at bottom
            TUIMiniPlayerView(playerViewModel: dependencies.playerViewModel)
        }
        // Tab switching with number keys (1-6)
        .onKeyPress("1") { tabState.selectTab(.sync) }
        .onKeyPress("2") { tabState.selectTab(.artists) }
        .onKeyPress("3") { tabState.selectTab(.albums) }
        .onKeyPress("4") { tabState.selectTab(.songs) }
        .onKeyPress("5") { tabState.selectTab(.playlists) }
        .onKeyPress("6") { tabState.selectTab(.player) }
        // Vim-style tab navigation
        .onKeyPress("H") { selectPreviousTab() }
        .onKeyPress("L") { selectNextTab() }
        // Navigation
        .onKeyPress("q") { /* TODO: quit app */ }
        .onKeyPress("?") { /* TODO: show help */ }
        .onKeyPress("/") { navigationState.push(.globalSearch) }
    }

    @ViewBuilder
    private func contentView(for tab: Tab, navigationState: Binding<NavigationState>) -> some View {
        // Check if we have a navigation stack route
        if let route = navigationState.wrappedValue.currentRoute {
            routeContent(for: route, navigationState: navigationState)
        } else {
            // Show tab content
            tabContent(for: tab, navigationState: navigationState)
        }
    }

    @ViewBuilder
    private func routeContent(for route: NavigationRoute, navigationState: Binding<NavigationState>) -> some View {
        switch route {
        case .sync:
            TUISyncView(dependencies: dependencies, navigationState: navigationState)
        case .sourceManagement:
            TUISourceManagementView(dependencies: dependencies, navigationState: navigationState)
        case .sourceBrowse(let sourceId, let parentPathId):
            TUISourceBrowseView(
                dependencies: dependencies,
                sourceId: sourceId,
                initialParentPathId: parentPathId,
                navigationState: navigationState
            )
        case .sourceScan(let sourceId, let pathId, let path):
            TUISourceScanView(
                dependencies: dependencies,
                sourceId: sourceId,
                pathId: pathId,
                path: path,
                navigationState: navigationState
            )
        case .artistDetail(let artist):
            TUIArtistDetailView(dependencies: dependencies, artist: artist, navigationState: navigationState)
        case .albumDetail(let album, let artist):
            TUIAlbumDetailView(dependencies: dependencies, albumName: album, artist: artist, navigationState: navigationState)
        case .globalSearch:
            TUIGlobalSearchView(dependencies: dependencies, navigationState: navigationState)
        case .queue:
            TUIQueueView(playerViewModel: dependencies.playerViewModel, navigationState: navigationState, dependencies: dependencies)
        default:
            Text("Route not implemented yet")
        }
    }

    @ViewBuilder
    private func tabContent(for tab: Tab, navigationState: Binding<NavigationState>) -> some View {
        switch tab {
        case .sync:
            TUISyncView(dependencies: dependencies, navigationState: navigationState)
        case .artists:
            TUIArtistListView(dependencies: dependencies, navigationState: navigationState)
        case .albums:
            TUIAlbumGridView(dependencies: dependencies, navigationState: navigationState)
        case .songs:
            TUISongListView(dependencies: dependencies, navigationState: navigationState)
        case .playlists:
            PlaylistsPlaceholder()
        case .player:
            TUIPlayerView(
                playerViewModel: dependencies.playerViewModel,
                dependencies: dependencies,
                navigationState: navigationState
            )
        }
    }

    private func selectPreviousTab() {
        guard let currentIndex = Tab.allCases.firstIndex(of: tabState.selectedTab),
              currentIndex > 0 else { return }
        tabState.selectTab(Tab.allCases[currentIndex - 1])
    }

    private func selectNextTab() {
        guard let currentIndex = Tab.allCases.firstIndex(of: tabState.selectedTab),
              currentIndex < Tab.allCases.count - 1 else { return }
        tabState.selectTab(Tab.allCases[currentIndex + 1])
    }
}

// MARK: - Placeholder Views

struct ArtistsPlaceholder: View {
    var body: some View {
        VStack {
            Text("Artists View - Coming Soon - Press '1' for Sync")
            Spacer()
        }
    }
}

struct AlbumsPlaceholder: View {
    var body: some View {
        VStack {
            Text("Albums View - Coming Soon")
            Spacer()
        }
    }
}

struct SongsPlaceholder: View {
    var body: some View {
        VStack {
            Text("Songs View - Coming Soon")
            Spacer()
        }
    }
}

struct PlaylistsPlaceholder: View {
    var body: some View {
        VStack {
            Text("Playlists View - Coming Soon")
            Spacer()
        }
    }
}

struct PlayerPlaceholder: View {
    var body: some View {
        VStack {
            Text("Player View - Coming Soon")
            Spacer()
        }
    }
}
