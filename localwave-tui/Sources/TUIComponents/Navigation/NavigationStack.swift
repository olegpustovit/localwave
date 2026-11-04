//
//  NavigationStack.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI

/// Represents a route in the navigation hierarchy
public enum NavigationRoute: Hashable, Equatable {
    case artists
    case artistDetail(artist: String)
    case albums
    case albumDetail(album: String, artist: String?)
    case songs
    case songDetail(songId: Int64)
    case playlists
    case playlistDetail(playlistId: Int64)
    case player
    case queue
    case search(query: String)
    case globalSearch
    // Sync routes
    case sync
    case sourceManagement
    case sourceBrowse(sourceId: Int64, parentPathId: Int64?)
    case sourceScan(sourceId: Int64, pathId: Int64, path: String)
}

/// Navigation state data
public struct NavigationState {
    public var path: [NavigationRoute] = []

    public init(path: [NavigationRoute] = []) {
        self.path = path
    }

    /// Current route (top of stack) or nil if at root
    public var currentRoute: NavigationRoute? {
        path.last
    }

    /// Check if we can go back
    public var canGoBack: Bool {
        !path.isEmpty
    }

    /// Breadcrumb string for display
    public var breadcrumb: String {
        var components: [String] = ["LocalWave"]

        for route in path {
            switch route {
            case .artists:
                components.append("Artists")
            case .artistDetail(let artist):
                components.append(artist)
            case .albums:
                components.append("Albums")
            case .albumDetail(let album, _):
                components.append(album)
            case .songs:
                components.append("Songs")
            case .songDetail:
                components.append("Song Details")
            case .playlists:
                components.append("Playlists")
            case .playlistDetail:
                components.append("Playlist")
            case .player:
                components.append("Player")
            case .queue:
                components.append("Queue")
            case .search(let query):
                components.append("Search: \(query)")
            case .globalSearch:
                components.append("Search")
            case .sync:
                components.append("Sync")
            case .sourceManagement:
                components.append("Source Management")
            case .sourceBrowse(_, let parentPathId):
                if parentPathId == nil {
                    components.append("Browse Root")
                } else {
                    components.append("Browse")
                }
            case .sourceScan(_, _, let path):
                components.append("Scan: \(path)")
            }
        }

        return components.joined(separator: " > ")
    }

    /// Push a new route onto the navigation stack
    public mutating func push(_ route: NavigationRoute) {
        path.append(route)
    }

    /// Pop the current route from the stack (go back)
    public mutating func pop() {
        guard !path.isEmpty else { return }
        path.removeLast()
    }

    /// Pop to root (clear entire stack)
    public mutating func popToRoot() {
        path.removeAll()
    }

    /// Pop to a specific route in the stack
    public mutating func popTo(_ route: NavigationRoute) {
        guard let index = path.firstIndex(of: route) else { return }
        path.removeSubrange((index + 1)...)
    }
}

/// Navigation container view that wraps content with navigation support
public struct NavigationContainer<Content: View>: View {
    @Binding var navigationState: NavigationState
    private let content: (Binding<NavigationState>) -> Content

    public init(navigationState: Binding<NavigationState>, @ViewBuilder content: @escaping (Binding<NavigationState>) -> Content) {
        self._navigationState = navigationState
        self.content = content
    }

    public var body: some View {
        VStack {
            // Breadcrumb at top
            BreadcrumbView(breadcrumb: navigationState.breadcrumb)

            // Main content
            content($navigationState)
        }
    }
}

/// Breadcrumb display component
struct BreadcrumbView: View {
    let breadcrumb: String

    var body: some View {
        HStack {
            Text(breadcrumb)
            Spacer()
        }
    }
}

