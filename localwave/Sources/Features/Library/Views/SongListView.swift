//
//  SongListView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct SongListView: View {
    @EnvironmentObject private var tabState: TabState
    @EnvironmentObject private var dependencies: DependencyContainer

    @ObservedObject private var viewModel: SongListViewModel
    @State private var searchText: String = ""
    @State private var isPlayerPresented: Bool = false
    @EnvironmentObject private var playerVM: PlayerViewModel
    @State private var songToEdit: Song? = nil

    @State private var showingPlaylistSelection = false
    @State private var songForPlaylist: Song? = nil

    private let logger = Logger(subsystem: subsystem, category: "SongListView")

    init(viewModel: SongListViewModel) {
        self.viewModel = viewModel
    }

    var body: some View {
        return VStack {
            SearchBar(
                text: $searchText,
                onChange: { newValue in
                    Task { await viewModel.searchSongs(query: newValue) }
                },
                placeholder: "Search songs...",
                debounceSeconds: 0.3
            )
            .padding()

            Text("Total songs: \(viewModel.totalSongs)")
                .font(.caption)
                .padding(.horizontal)

            if viewModel.songs.isEmpty && !viewModel.isLoadingPage {
                emptyStateView
            } else {
                List {
                    ForEach(Array(viewModel.songs.enumerated()), id: \.element.uniqueId) {
                        index, song in
                        SongRow(
                            song: song,
                            onPlay: {
                                // Populate the player queue and play the tapped song
                                playerVM.configureQueue(
                                    songs: viewModel.songs, startIndex: index
                                )
                                playerVM.playSong(song)
                            },
                            onDelete: {
                                Task {
                                    if let songId = song.id {
                                        try? await dependencies.songRepository.deleteSong(
                                            songId: songId)
                                        await viewModel.loadInitialSongs()
                                    }
                                }
                            },
                            onAddToPlaylist: {
                                songForPlaylist = song
                                showingPlaylistSelection = true
                            },
                            onEditMetadata: {
                                songToEdit = song
                            },
                            onAddToQueue: {
                                playerVM.addToQueue(song)
                            },
                            onToggleFavorite: { isFavorite in 
                                Task {
                                    try? await viewModel.toggleFavorite(song.id, state: isFavorite)
                                }
                            },
                            isFavorite: (song.isFavorite != 0),
                            readonly: false
                        )
                        .onAppear {
                            viewModel.loadMoreIfNeeded(currentSong: song)
                        }
                    }
                    if viewModel.isLoadingPage {
                        HStack {
                            Spacer()
                            ProgressView()
                            Spacer()
                        }
                    }
                }
                .listStyle(PlainListStyle())
            }
        }
        .onAppear {
            Task {
                if searchText.isEmpty {
                    await viewModel.loadInitialSongs()
                } else {
                    await viewModel.searchSongs(query: searchText)
                }
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: Notification.Name("SongListRefresh"))) { _ in
            Task { await viewModel.loadInitialSongs() }
        }
        .onDisappear {
            viewModel.reset() // Clears the songs array and resets pagination.
        }
        .sheet(isPresented: $showingPlaylistSelection) {
            if let song = songForPlaylist {
                PlaylistSelectionView(
                    song: song,
                    songRepo: dependencies.songRepository,
                    playlistRepo: dependencies.playlistRepo,
                    playlistSongRepo: dependencies.playlistSongRepo
                )
            }
        }
        .sheet(item: $songToEdit) { song in
            SongMetadataEditorView(song: song, songRepo: dependencies.songRepository)
        }
    }

    private var emptyStateView: some View {
        VStack(spacing: 20) {
            Image(systemName: "music.note.list")
                .font(.system(size: 60))
                .foregroundColor(.secondary)

            Text("No Songs Found")
                .font(.title2)

            Text(
                searchText.isEmpty
                    ? "Add a music source to get started"
                    : "No matches found for '\(searchText)'"
            )
            .multilineTextAlignment(.center)
            .foregroundColor(.secondary)

            if searchText.isEmpty {
                Button("Add Source") {
                    tabState.selectedTab = 1
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding()
        .frame(maxHeight: .infinity)
    }
}
