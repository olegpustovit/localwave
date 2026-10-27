//
//  PlayerView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct PlayerView: View {
    @EnvironmentObject private var playerVM: PlayerViewModel
    @State private var showingQueue = false
    @State private var shuffleEnabled: Bool = false
    @State private var repeatEnabled: RepeatMode = .none
    @Environment(\.dismiss) private var dismiss

    // State for playlist creation
    @State private var showingPlaylistAlert = false
    @State private var playlistName = ""
    @State private var editMode = EditMode.inactive
    
    // Helper function to load cover art for a song
    private func coverArt(of song: Song) -> UIImage? {
        guard let path = song.coverArtPath else { return nil }
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
        let url = docs.appendingPathComponent(path)
        return UIImage(contentsOfFile: url.path)
    }

        
    var body: some View {
        ZStack {
            LinearGradient(
                gradient: Gradient(colors: [Color.purple.opacity(0.8), Color.blue.opacity(0.8)]),
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .edgesIgnoringSafeArea(.all)
            VStack {
                // Top Bar
                HStack {
                    Spacer()
                    Button(action: { dismiss() }) {
                        Image(systemName: "xmark.circle.fill")
                            .font(.largeTitle)
                            .foregroundColor(.white)
                    }
                }
                .padding(.top)
                // Artwork and Song Info Section
                if let song = playerVM.currentSong {
                    VStack {
                        if let cover = coverArt(of: song) {
                            Image(uiImage: cover)
                                .resizable()
                                .scaledToFit()
                                .frame(maxWidth: 300, maxHeight: 300)
                                .cornerRadius(8)
                                .shadow(radius: 10)
                        } else {
                            Image(systemName: "music.note")
                                .resizable()
                                .scaledToFit()
                                .frame(width: 200, height: 200)
                                .foregroundColor(.white)
                        }

                        Text(song.title)
                            .font(.title)
                            .fontWeight(.bold)
                            .foregroundColor(.white)
                            .padding(.top, 8)
                        Text(song.artist)
                            .font(.subheadline)
                            .foregroundColor(.white.opacity(0.8))
                    }
                    .padding()
                }

                // Playback Progress Slider
                VStack {
                    Slider(
                        value: Binding(
                            get: { playerVM.playbackProgress },
                            set: { newValue in
                                playerVM.seekByFraction(newValue)
                            }
                        ),
                        in: 0 ... 1
                    )
                    .accentColor(.yellow)
                    .padding(.horizontal)

                    HStack {
                        Text(playerVM.currentTime)
                        Spacer()
                        Text(playerVM.duration)
                    }
                    .font(.caption)
                    .foregroundColor(.white)
                    .padding(.horizontal)
                }
                .padding(.vertical)

                // Playback Control Buttons
                HStack(spacing: 30) {
                    Button(action: {
                        shuffleEnabled.toggle()
                        playerVM.setShuffle(shuffleEnabled)
                    }) {
                        Image(systemName: shuffleEnabled ? "shuffle.circle.fill" : "shuffle.circle")
                            .font(.system(size: 30))
                            .foregroundColor(shuffleEnabled ? .yellow : .white)
                    }

                    Button(action: { playerVM.previousSong() }) {
                        Image(systemName: "backward.fill")
                            .font(.system(size: 30))
                            .foregroundColor(.white)
                    }

                    Button(action: { playerVM.playPause() }) {
                        Image(
                            systemName: playerVM.isPlaying
                                ? "pause.circle.fill" : "play.circle.fill"
                        )
                        .font(.system(size: 60))
                        .foregroundColor(.white)
                    }

                    Button(action: { playerVM.nextSong() }) {
                        Image(systemName: "forward.fill")
                            .font(.system(size: 30))
                            .foregroundColor(.white)
                    }

                    Button {
                        repeatEnabled.toggle()
                        playerVM.setRepeat(repeatEnabled)
                    } label: {
                        Image(systemName: repeatEnabled.iconName)
                            .font(.system(size: 30))
                            .foregroundColor(repeatEnabled.color)
                    }
                }
                .padding()

                // Volume Control - NEW: Updated slider range and styling
                HStack {
                    Image(systemName: "speaker.fill")
                        .foregroundColor(.white)
                    Slider(value: $playerVM.volume, in: 0 ... 1)
                        .accentColor(.yellow)
                    Image(systemName: "speaker.wave.3.fill")
                        .foregroundColor(.white)
                }
                .padding(.horizontal)

                // Queue Toggle Button
                Button(action: {
                    showingQueue.toggle()
                }) {
                    HStack {
                        Image(systemName: "list.bullet")
                        Text("Queue (\(playerVM.queue.count))")
                    }
                    .foregroundColor(.white)
                    .padding(.vertical, 8)
                    .padding(.horizontal, 16)
                    .background(Color.black.opacity(0.3))
                    .cornerRadius(10)
                }
                .padding(.top)

                // Currently Played Queue - NEW: Added current song icon and tap gesture to play new song
                if showingQueue {
                    ScrollView {
                        VStack(alignment: .leading) {
                            ForEach(playerVM.queue.indices, id: \.self) { index in
                                let song = playerVM.queue[index]
                                HStack {
                                    Text("\(index + 1).")
                                        .foregroundColor(.white)
                                    VStack(alignment: .leading) {
                                        Text(song.title)
                                            .font(.headline)
                                            .foregroundColor(.white)
                                        Text(song.artist)
                                            .font(.subheadline)
                                            .foregroundColor(.white.opacity(0.8))
                                    }
                                    Spacer()
                                    if song.id == playerVM.currentSong?.id {
                                        Image(systemName: "speaker.wave.2.fill")
                                            .foregroundColor(.green)
                                    }
                                }
                                .padding(.vertical, 4)
                                .contentShape(Rectangle())
                                .onTapGesture {
                                    playerVM.playSong(song)
                                }
                            }
                            .onMove { indices, newOffset in
                                playerVM.reorderQueue(from: indices, to: newOffset)
                            }
                        }
                        .environment(\.editMode, $editMode)
                        .padding()
                    }
                    .frame(maxHeight: 200)
                    .background(Color.black.opacity(0.2))
                    .cornerRadius(8)
                    .padding(.horizontal)
                    .toolbar {
                        ToolbarItemGroup(placement: .navigationBarTrailing) {
                            Button("Save as Playlist") {
                                showingPlaylistAlert = true
                            }
                            EditButton()
                        }
                    }
                    .alert("New Playlist", isPresented: $showingPlaylistAlert) {
                        TextField("Playlist Name", text: $playlistName)
                        Button("Create") {
                            Task {
                                try await playerVM.createPlaylist(name: playlistName)
                                playlistName = ""
                            }
                        }
                        Button("Cancel", role: .cancel) {}
                    }
                }

                Spacer()
            }
            .padding()
        }
        .onAppear {
            playerVM.updateNowPlayingInfo()
        }
        .onReceive(
            NotificationCenter.default.publisher(for: UIApplication.willEnterForegroundNotification)
        ) { _ in
            playerVM.updateNowPlayingInfo()
        }
    }
}
