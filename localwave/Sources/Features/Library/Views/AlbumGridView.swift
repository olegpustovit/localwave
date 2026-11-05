#if os(iOS) || os(macOS)
import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  AlbumGridView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct AlbumGridView: View {
    @ObservedObject var viewModel: AlbumListViewModel
    private let columns = [GridItem(.adaptive(minimum: 160))]
    private let songRepo: SongRepository

    init(dependencies: DependencyContainer, viewModel: AlbumListViewModel) {
        self.viewModel = viewModel
        songRepo = dependencies.songRepository
    }

    var body: some View {
        ScrollView {
            SearchBar(
                text: $viewModel.searchQuery,
                onChange: { _ in },
                placeholder: "Search albums...",
                debounceSeconds: 0.3
            )
            if viewModel.filteredAlbums.isEmpty {
                emptyStateView
            } else {
                LazyVGrid(columns: columns, spacing: 20) {
                    ForEach(viewModel.filteredAlbums) { album in
                        NavigationLink {
                            AlbumSongListView(album: album, songRepo: songRepo)
                        } label: {
                            AlbumCell(album: album)
                        }
                        .buttonStyle(PlainButtonStyle())
                    }
                }
                .padding()
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: Notification.Name("AlbumListRefresh"))) { _ in
            Task { try? await viewModel.loadAlbums() }
        }
    }

    private var emptyStateView: some View {
        VStack(spacing: 20) {
            Image(systemName: "square.stack.slash")
                .font(.system(size: 60))
                .foregroundColor(.secondary)

            Text("No Albums Found")
                .font(.title2)

            Text("Add a music source with audio files to populate albums")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
        }
        .padding()
        .frame(maxHeight: .infinity)
    }
}

#endif
