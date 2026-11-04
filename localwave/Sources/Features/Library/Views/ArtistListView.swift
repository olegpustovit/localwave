import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  ArtistListView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct ArtistListView: View {
    @ObservedObject var viewModel: ArtistListViewModel
    private let songRepo: SongRepository

    init(dependencies: DependencyContainer, viewModel: ArtistListViewModel) {
        songRepo = dependencies.songRepository
        self.viewModel = viewModel
    }

    var body: some View {
        VStack {
            if viewModel.filteredArtists.isEmpty {
                emptyStateView
            } else {
                SearchBar(
                    text: $viewModel.searchQuery,
                    onChange: { _ in },
                    placeholder: "Search artists...",
                    debounceSeconds: 0.3
                )

                List(viewModel.filteredArtists, id: \.self) { artist in
                    NavigationLink {
                        ArtistSongListView(artist: artist, songRepo: songRepo)
                    } label: {
                        Text(artist)
                            .font(.headline)
                            .padding(.vertical, 8)
                    }
                }
                .listStyle(.plain)
            }
        }
    }

    private var emptyStateView: some View {
        VStack(spacing: 20) {
            Image(systemName: "person.2.slash")
                .font(.system(size: 60))
                .foregroundColor(.secondary)

            Text("No Artists Found")
                .font(.title2)

            Text("Add a music source with audio files to populate artists")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
        }
        .padding()
        .frame(maxHeight: .infinity)
    }
}
