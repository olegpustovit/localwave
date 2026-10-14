//
//  SongRow.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct SongRow: View {
    @EnvironmentObject private var playerVM: PlayerViewModel

    let song: Song
    let onPlay: () -> Void

    var onDelete: (() -> Void)? = nil
    var onAddToPlaylist: (() -> Void)? = nil
    var onEditMetadata: (() -> Void)? = nil
    var onAddToQueue: (() -> Void)? = nil

    var onToggleFavorite: ((Bool) -> Void)? = nil
    
    @State var isFavorite: Bool = false
    
    var readonly: Bool
    
    var body: some View {
        HStack {
            Image(systemName: "star.fill")
                .foregroundColor(isFavorite ? .yellow : .white)
                .onTapGesture {
                    if (!readonly) {
                        isFavorite.toggle()
                        onToggleFavorite?(isFavorite)
                    }
                }
            
            VStack(alignment: .leading) {
                Text(song.title)
                    .font(.headline)
                Text(song.artist)
                    .font(.subheadline)
                    .foregroundColor(.secondary)
            }
            Spacer()
            if song.id == playerVM.currentSong?.id && playerVM.isPlaying {
                // This icon serves as a playing indicator.
                Image(systemName: "speaker.wave.2.fill")
                    .foregroundColor(.green)
            }
        }
        .contentShape(Rectangle()) // Make the whole row tappable
        .onTapGesture {
            onPlay()
        }
        .padding(.vertical, 4)
        .contextMenu {
            if (!readonly) {
                Button("Add to Queue") {
                    onAddToQueue?()
                }
                Button("Delete Song", role: .destructive) {
                    onDelete?()
                }
                Button("Add to Playlist") {
                    onAddToPlaylist?()
                }
                Button("Edit Metadata") {
                    onEditMetadata?()
                }
            }
        }
    }
}
