import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  MiniPlayerViewInner.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct MiniPlayerViewInner: View {
    let currentSong: Song?
    let onTap: () -> Void
    let playPauseAction: () -> Void
    let isPlaying: Bool

    var body: some View {
        if currentSong != nil {
            Button(action: {
                onTap()
            }) {
                HStack {
                    if let song = currentSong, let cover = coverArt(of: song) {
                        Image(uiImage: cover)
                            .resizable()
                            .frame(width: 50, height: 50)
                            .cornerRadius(5)
                    } else {
                        Image(systemName: "music.note")
                            .scaleEffect(1.6)
                            .frame(width: 50, height: 50)
                            .cornerRadius(5)
                    }

                    VStack(alignment: .leading) {
                        Text(currentSong?.title ?? "No Song")
                        Text(
                            "\(currentSong?.artist ?? "Unknown") - \(currentSong?.album ?? "")"
                        )
                        .font(Oxanium(14))
                        .foregroundColor(.secondary)
                    }
                    Spacer()
                    Button(action: playPauseAction) {
                        Image(systemName: isPlaying ? "pause.fill" : "play.fill")
                            .font(.system(size: 24))
                    }
                    .buttonStyle(PlainButtonStyle())
                }
                .padding(.trailing, 15)
                .background(Color(UIColor.secondarySystemBackground))
                .overlay(
                    Rectangle()
                        .frame(height: 0.2) // Height for the top border
                        .foregroundColor(.secondary),
                    alignment: .top // Align to top
                )
            }
            .buttonStyle(PlainButtonStyle())
        }
    }
}
