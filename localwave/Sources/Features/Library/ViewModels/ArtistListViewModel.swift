import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  ArtistListViewModel.swift
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
class ArtistListViewModel: ObservableObject {
    @Published var artists: [String] = []
    @Published var searchQuery = ""
    private let songRepo: SongRepository

    init(songRepo: SongRepository) {
        self.songRepo = songRepo
    }

    func loadArtists() async throws {
        artists = try await songRepo.getAllArtists()
    }

    var filteredArtists: [String] {
        guard !searchQuery.isEmpty else { return artists }
        return artists.filter { $0.localizedCaseInsensitiveContains(searchQuery) }
    }
}
