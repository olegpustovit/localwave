//
//  SongSelectionView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct SongSelectionView: View {
    let songRepo: SongRepository
    let onSongsSelected: ([Song]) -> Void
    @State private var selectedSongs = Set<Int64>()
    @StateObject private var songListVM: SongListViewModel

    init(songRepo: SongRepository, onSongsSelected: @escaping ([Song]) -> Void) {
        self.songRepo = songRepo
        self.onSongsSelected = onSongsSelected
        _songListVM = StateObject(
            wrappedValue: SongListViewModel(
                songRepo: songRepo,
                filter: .all
            )
        )
    }

    var body: some View {
        NavigationStack {
            List(songListVM.songs, id: \.uniqueId) { song in
                SelectableSongRow(
                    song: song,
                    isSelected: selectedSongs.contains(song.id ?? -1),
                    onToggle: {
                        if selectedSongs.contains(song.id ?? -1) {
                            selectedSongs.remove(song.id ?? -1)
                        } else {
                            selectedSongs.insert(song.id ?? -1)
                        }
                    },
                    isFavorite: song.isFavorite != 0)
            }
            .toolbar {
                Button("Add") {
                    let songsToAdd = songListVM.songs.filter { selectedSongs.contains($0.id ?? -1) }
                    onSongsSelected(songsToAdd)
                }
            }
            .onAppear {
                Task { await songListVM.loadInitialSongs() }
            }
        }
    }
}
