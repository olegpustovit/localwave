#if os(iOS) || os(macOS)
import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  SongMetadataEditorView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct SongMetadataEditorView: View {
    @Environment(\.dismiss) var dismiss
    let song: Song
    let songRepo: SongRepository
    @State private var title: String
    @State private var artist: String
    @State private var album: String
    @State private var albumArtist: String
    @State private var releaseYear: String
    @State private var discNumber: String
    @State private var trackNumber: String

    private let logger = Logger(subsystem: subsystem, category: "SongMetadataEditorView")

    init(song: Song, songRepo: SongRepository) {
        self.song = song
        self.songRepo = songRepo
        _title = State(initialValue: song.title)
        _artist = State(initialValue: song.artist)
        _album = State(initialValue: song.album)
        _albumArtist = State(initialValue: song.albumArtist)
        _releaseYear = State(initialValue: song.releaseYear != nil ? "\(song.releaseYear!)" : "")
        _discNumber = State(initialValue: song.discNumber != nil ? "\(song.discNumber!)" : "")
        _trackNumber = State(initialValue: song.trackNumber != nil ? "\(song.trackNumber!)" : "")
    }

    var body: some View {
        NavigationStack {
            Form {
                Section(header: Text("Basic Info")) {
                    TextField("Title", text: $title)
                    TextField("Artist", text: $artist)
                    TextField("Album", text: $album)
                    TextField("Album Artist", text: $albumArtist)
                }
                Section(header: Text("Additional Info")) {
                    TextField("Release Year", text: $releaseYear)
#if os(iOS)
                        .keyboardType(.numberPad)
#endif
                    TextField("Disc Number", text: $discNumber)
#if os(iOS)
                        .keyboardType(.numberPad)
#endif
                    TextField("Track Number", text: $trackNumber)
#if os(iOS)
                        .keyboardType(.numberPad)
#endif
                }
            }
            .navigationTitle("Edit Metadata")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        Task {
                            let updatedSong = Song(
                                id: song.id,
                                songKey: song.songKey,
                                artist: artist,
                                title: title,
                                album: album,
                                albumArtist: albumArtist,
                                releaseYear: Int(releaseYear),
                                discNumber: Int(discNumber),
                                trackNumber: Int(trackNumber),
                                coverArtPath: song.coverArtPath,
                                bookmark: song.bookmark,
                                pathHash: song.pathHash,
                                createdAt: song.createdAt,
                                updatedAt: Date(),
                                localFilePath: song.localFilePath,
                                fileState: song.fileState,
                                isFavorite: song.isFavorite,
                                sourceId: song.sourceId,
                                relativePath: song.relativePath,
                                duration: song.duration,
                                fileSize: song.fileSize,
                                fileExtension: song.fileExtension,
                                sampleRate: song.sampleRate,
                                bitDepth: song.bitDepth,
                                sourceAvailability: song.sourceAvailability
                            )
                            do {
                                _ = try await songRepo.upsertSong(updatedSong)
                                NotificationCenter.default.post(
                                    name: Notification.Name("SongListRefresh"), object: nil
                                )
                                dismiss()
                            } catch {
                                logger.error("failed to upsert: \(error)")
                            }
                        }
                    }
                }
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
            }
        }
    }
}

#endif
