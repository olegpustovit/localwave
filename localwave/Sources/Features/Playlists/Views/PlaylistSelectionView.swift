#if os(iOS)
import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  PlaylistSelectionView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct PlaylistSelectionView: View {
    let song: Song
    @StateObject private var viewModel: PlaylistListViewModel
    @Environment(\.dismiss) var dismiss

    init(
        song: Song, songRepo: SongRepository, playlistRepo: PlaylistRepository,
        playlistSongRepo: PlaylistSongRepository
    ) {
        self.song = song
        _viewModel = StateObject(
            wrappedValue: PlaylistListViewModel(
                playlistRepo: playlistRepo,
                playlistSongRepo: playlistSongRepo,
                songRepo: songRepo // Assuming access to song repo
            )
        )
    }

    var body: some View {
        NavigationStack {
            List(viewModel.playlists) { playlist in
                Button(playlist.name) {
                    Task {
                        guard let playlistId = playlist.id, let songId = song.id else { return }
                        try? await viewModel.playlistSongRepo.addSong(
                            playlistId: playlistId,
                            songId: songId
                        )
                        dismiss()
                    }
                }
            }
            .navigationTitle("Select Playlist")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
            }
            .onAppear {
                Task { await viewModel.loadPlaylists() }
            }
        }
    }
}

#endif
