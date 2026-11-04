import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  AlbumListViewModel.swift
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
class AlbumListViewModel: ObservableObject {
    @Published var albums: [Album] = []
    @Published var searchQuery = ""
    private let songRepo: SongRepository

    init(songRepo: SongRepository) {
        self.songRepo = songRepo
    }

    func loadAlbums() async throws {
        albums = try await songRepo.getAllAlbums()
    }

    var filteredAlbums: [Album] {
        guard !searchQuery.isEmpty else { return albums }
        return albums.filter {
            $0.name.localizedCaseInsensitiveContains(searchQuery)
                || $0.artist?.localizedCaseInsensitiveContains(searchQuery) ?? false
        }
    }
}
