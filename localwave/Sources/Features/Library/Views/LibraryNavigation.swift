#if os(iOS) || os(macOS)
import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  LibraryNavigation.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//
import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

class LibraryNavigation: ObservableObject {
    @Published var path = NavigationPath()
}

struct LibraryView: View {
    let logger = Logger(subsystem: subsystem, category: "LibraryView")

    @EnvironmentObject private var dependencies: DependencyContainer
    @EnvironmentObject private var libraryNavigation: LibraryNavigation
    @StateObject private var artistVM: ArtistListViewModel
    @StateObject private var albumVM: AlbumListViewModel
    @StateObject private var songListVM: SongListViewModel
    @EnvironmentObject private var tabState: TabState // already exists

    init(dependencies: DependencyContainer) {
        let dc = dependencies
        _artistVM = StateObject(wrappedValue: dc.makeArtistListViewModel())
        _albumVM = StateObject(wrappedValue: dc.makeAlbumListViewModel())
        _songListVM = StateObject(wrappedValue: dc.makeSongListViewModel(filter: .all))
    }

    var body: some View {
        NavigationStack(path: $libraryNavigation.path) {
            List {
                NavigationLink(
                    "Playlists",
                    destination: PlaylistListView(
                        viewModel: dependencies.makePlaylistListViewModel())
                )
                NavigationLink(
                    "Artists",
                    destination: ArtistListView(dependencies: dependencies, viewModel: artistVM)
                )
                NavigationLink(
                    "Albums",
                    destination: AlbumGridView(dependencies: dependencies, viewModel: albumVM)
                )
                NavigationLink("Songs", destination: SongListView(viewModel: songListVM))
                NavigationLink("About & Privacy", destination: PrivacyView())
            }
            .navigationTitle("Library")
        }
        .onAppear {
            Task {
                try? await artistVM.loadArtists()
                try? await albumVM.loadAlbums()
            }
        }

        .onChange(of: tabState.selectedTab) { newTab, _ in
            if newTab == 0 { // library tab
                Task {
                    do {
                        try await artistVM.loadArtists()
                        try await albumVM.loadAlbums()
                    } catch {
                        logger.error("failed to resync view \(error)")
                    }
                }
            }
        }

        .onReceive(NotificationCenter.default.publisher(for: Notification.Name("LibraryRefresh"))) {
            _ in
            Task {
                do {
                    try await artistVM.loadArtists()
                    try await albumVM.loadAlbums()
                } catch {
                    logger.error("failed to resync view \(error)")
                }
            }
        }
    }
}

#endif
