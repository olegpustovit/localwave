import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  PlaylistListView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct PlaylistListView: View {
    @ObservedObject var viewModel: PlaylistListViewModel

    var body: some View {
        NavigationStack {
            if viewModel.playlists.isEmpty {
                VStack(spacing: 20) {
                    Image(systemName: "music.note.list")
                        .font(.system(size: 60))
                        .foregroundColor(.gray)
                    Text("No Playlists Found")
                        .font(.title2)
                        .foregroundColor(.secondary)
                    Text("Create a playlist to get started.")
                        .multilineTextAlignment(.center)
                        .foregroundColor(.secondary)
                    Button("Create Playlist") {
                        viewModel.showingCreateDialog = true
                    }
                    .buttonStyle(.borderedProminent)
                }
                .padding()
                .navigationTitle("Playlists")
            } else { // OLD: Existing list view for playlists
                List {
                    ForEach(viewModel.playlists) { playlist in
                        NavigationLink {
                            PlaylistDetailView(
                                playlist: playlist,
                                viewModel: PlaylistDetailViewModel(
                                    playlist: playlist,
                                    playlistSongRepo: viewModel.playlistSongRepo,
                                    songRepo: viewModel.songRepo
                                )
                            )
                        } label: {
                            Text(playlist.name)
                                .font(.headline)
                        }
                    }
                    .onDelete { offsets in
                        Task { await viewModel.deletePlaylist(at: offsets) }
                    }
                }
                .toolbar {
                    Button {
                        viewModel.showingCreateDialog = true
                    } label: {
                        Image(systemName: "plus")
                    }
                }
                .navigationTitle("Playlists")
            }
        }
        .alert("New Playlist", isPresented: $viewModel.showingCreateDialog) {
            TextField("Name", text: $viewModel.newPlaylistName)
            Button("Create") {
                Task { await viewModel.createPlaylist() }
            }
            Button("Cancel", role: .cancel) {}
        }
        .onAppear {
            Task { await viewModel.loadPlaylists() }
        }
    }
}
