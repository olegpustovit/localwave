//
//  PlaylistDetailView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct PlaylistDetailView: View {
    let playlist: Playlist
    @ObservedObject var viewModel: PlaylistDetailViewModel
    @EnvironmentObject var playerVM: PlayerViewModel

    var body: some View {
        VStack {
            if viewModel.songs.isEmpty {
                VStack(spacing: 20) {
                    Image(systemName: "music.note")
                        .font(.system(size: 60))
                        .foregroundColor(.gray)
                    Text("No Songs in this Playlist")
                        .font(.title2)
                        .foregroundColor(.secondary)
                    Text("Tap 'Add Songs' to add your favorite tracks.")
                        .multilineTextAlignment(.center)
                        .foregroundColor(.secondary)
                }
                .padding()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List {
                    ForEach(viewModel.songs) { song in
                        SongRow(song: song,
                                onPlay: {
                                    if let index = viewModel.songs.firstIndex(of: song) {
                                        playerVM.configureQueue(songs: viewModel.songs, startIndex: index)
                                        playerVM.playSong(song)
                                    }
                                },
                                onDelete: {},
                                onAddToPlaylist: {},
                                onEditMetadata: {},
                                onAddToQueue: {},
                                onToggleFavorite: { isFavorite in
                                },
                                isFavorite: (song.isFavorite != 0),
                                readonly: true)
                    }
                    .onDelete { offsets in
                        Task { await viewModel.deleteSong(at: offsets) }
                    }
                    .onMove { from, to in
                        Task { await viewModel.reorderSongs(from: from, to: to) }
                    }
                }
            }
        }
        .toolbar {
            ToolbarItemGroup(placement: .navigationBarTrailing) {
                Button("Add Songs") {
                    viewModel.showAddSongs = true
                }
                EditButton()
            }
        }
        .sheet(isPresented: $viewModel.showAddSongs) {
            SongSelectionView(
                songRepo: viewModel.songRepo,
                onSongsSelected: { selected in
                    Task {
                        guard let playlistId = playlist.id else { return }
                        for song in selected {
                            try? await viewModel.playlistSongRepo.addSong(
                                playlistId: playlistId,
                                songId: song.id!
                            )
                        }
                        await viewModel.loadSongs()
                    }
                }
            )
        }
        .navigationTitle(playlist.name)
        .onAppear {
            Task { await viewModel.loadSongs() }
        }
    }
}
