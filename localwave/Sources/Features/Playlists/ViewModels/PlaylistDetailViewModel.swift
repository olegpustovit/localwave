import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  PlaylistDetailViewModel.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

@MainActor
class PlaylistDetailViewModel: ObservableObject {
    @Published var songs: [Song] = []
    @Published var showAddSongs = false
    @Published var selectedSongs = Set<Int64>()

    let playlist: Playlist
    let playlistSongRepo: PlaylistSongRepository
    let songRepo: SongRepository

    init(playlist: Playlist, playlistSongRepo: PlaylistSongRepository, songRepo: SongRepository) {
        self.playlist = playlist
        self.playlistSongRepo = playlistSongRepo
        self.songRepo = songRepo
    }

    func loadSongs() async {
        songs = (try? await playlistSongRepo.getSongs(playlistId: playlist.id!)) ?? []
    }

    func deleteSong(at offsets: IndexSet) async {
        guard let playlistId = playlist.id else { return }
        for index in offsets {
            let songId = songs[index].id!
            try? await playlistSongRepo.removeSong(playlistId: playlistId, songId: songId)
        }
        await loadSongs()
    }

    func reorderSongs(from source: IndexSet, to destination: Int) async {
        var updatedSongs = songs
        updatedSongs.move(fromOffsets: source, toOffset: destination)

        let newOrder = updatedSongs.map { $0.id! }
        try? await playlistSongRepo.reorderSongs(playlistId: playlist.id!, newOrder: newOrder)
        await loadSongs()
    }
}
