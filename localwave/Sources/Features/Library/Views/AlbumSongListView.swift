#if os(iOS) || os(macOS)
import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  AlbumSongListView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct AlbumSongListView: View {
    let album: Album
    @StateObject private var viewModel: SongListViewModel

    init(album: Album, songRepo: SongRepository) {
        self.album = album
        _viewModel = StateObject(
            wrappedValue: SongListViewModel(
                songRepo: songRepo,
                filter: .album(album.name, artist: album.artist)
            )
        )
    }

    var body: some View {
        SongListView(viewModel: viewModel)
            .navigationTitle(album.name)
            .onAppear {
                Task { await viewModel.loadInitialSongs() }
            }
    }
}

#endif
