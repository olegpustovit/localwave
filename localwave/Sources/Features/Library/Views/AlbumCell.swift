import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  AlbumCell.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct AlbumCell: View {
    let album: Album
    @State private var artwork: UIImage?
    @EnvironmentObject private var dependencies: DependencyContainer

    var body: some View {
        VStack(alignment: .leading) {
            ZStack {
                if let artwork = artwork {
                    Image(uiImage: artwork)
                        .resizable()
                        .aspectRatio(1, contentMode: .fill)
                } else {
                    Image(systemName: "music.note")
                        .resizable()
                        .aspectRatio(1, contentMode: .fit)
                        .padding()
                        .background(Color.gray.opacity(0.3))
                }
            }
            .frame(width: 160, height: 160)
            .cornerRadius(8)
            .clipped()

            VStack(alignment: .leading) {
                Text(album.name)
                    .font(.subheadline)
                    .lineLimit(1)
                Text(album.artist ?? "Unknown Artist")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
            }
            .frame(width: 160)
        }
        .onAppear {
            loadArtwork()
        }
        .contextMenu {
            Button("Delete Album", role: .destructive) {
                Task {
                    try? await dependencies.songRepository.deleteAlbum(
                        album: album.name, artist: album.artist
                    )
                    NotificationCenter.default.post(
                        name: Notification.Name("AlbumListRefresh"), object: nil
                    )
                }
            }
        }
    }

    private func loadArtwork() {
        guard let path = album.coverArtPath else { return }
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
        let url = docs.appendingPathComponent(path)

        DispatchQueue.global(qos: .userInitiated).async {
            if let image = UIImage(contentsOfFile: url.path) {
                DispatchQueue.main.async {
                    self.artwork = image
                }
            }
        }
    }
}
