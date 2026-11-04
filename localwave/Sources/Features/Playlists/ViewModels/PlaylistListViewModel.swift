import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  PlaylistListViewModel.swift
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
class PlaylistListViewModel: ObservableObject {
    @Published var playlists: [Playlist] = []
    @Published var showingCreateDialog = false
    @Published var newPlaylistName = ""

    let playlistRepo: PlaylistRepository
    let playlistSongRepo: PlaylistSongRepository
    let songRepo: SongRepository

    private let logger = Logger(subsystem: subsystem, category: "PlaylistListViewModel")

    init(
        playlistRepo: PlaylistRepository,
        playlistSongRepo: PlaylistSongRepository,
        songRepo: SongRepository
    ) {
        self.playlistRepo = playlistRepo
        self.playlistSongRepo = playlistSongRepo
        self.songRepo = songRepo
    }

    func loadPlaylists() async {
        playlists = (try? await playlistRepo.getAll()) ?? []
    }

    func deletePlaylist(at offsets: IndexSet) async {
        for index in offsets {
            let playlist = playlists[index]
            if let id = playlist.id {
                do {
                    try await playlistRepo.delete(playlistId: id)
                } catch {
                    // Log or handle error as needed.
                    logger.debug("failed to delete song with id: \(id)")
                }
            }
        }
        await loadPlaylists()
    }

    func createPlaylist() async {
        guard !newPlaylistName.isEmpty else { return }
        let playlist = Playlist(id: nil, name: newPlaylistName, createdAt: Date(), updatedAt: nil)
        if let created = try? await playlistRepo.create(playlist: playlist) {
            playlists.append(created)
            newPlaylistName = ""
            showingCreateDialog = false
        }
    }
}
