import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  ArtistSongListView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct ArtistSongListView: View {
    let artist: String
    @StateObject private var viewModel: SongListViewModel
    private var songRepo: SongRepository

    init(artist: String, songRepo: SongRepository) {
        self.artist = artist
        self.songRepo = songRepo
        _viewModel = StateObject(
            wrappedValue: SongListViewModel(
                songRepo: songRepo,
                filter: .artist(artist)
            )
        )
    }

    var body: some View {
        SongListView(viewModel: viewModel)
            .navigationTitle(artist)
            .onAppear {
                Task { await viewModel.loadInitialSongs() }
            }
    }
}
