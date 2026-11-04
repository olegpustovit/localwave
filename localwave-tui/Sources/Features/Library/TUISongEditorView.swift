//
//  TUISongEditorView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import Foundation
import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI modal form to edit song metadata
///
/// Features:
/// - Edit title, artist, album, year fields
/// - Tab/Shift+Tab to navigate between fields
/// - Enter to save, Esc to cancel
/// - Validation for year field
/// - Updates database using upsertSong
struct TUISongEditorView: View {
    let dependencies: TUIDependencyContainer
    let song: Song
    let onSave: () async -> Void
    let onCancel: () -> Void

    @State private var editedTitle: String
    @State private var editedArtist: String
    @State private var editedAlbum: String
    @State private var editedYear: String
    @State private var currentField: Field = .title
    @State private var isSaving = false
    @State private var errorMessage: String?

    enum Field {
        case title, artist, album, year
    }

    init(dependencies: TUIDependencyContainer, song: Song, onSave: @escaping () async -> Void, onCancel: @escaping () -> Void) {
        self.dependencies = dependencies
        self.song = song
        self.onSave = onSave
        self.onCancel = onCancel
        _editedTitle = State(initialValue: song.title)
        _editedArtist = State(initialValue: song.artist)
        _editedAlbum = State(initialValue: song.album)
        _editedYear = State(initialValue: song.releaseYear != nil ? String(song.releaseYear!) : "")
    }

    var body: some View {
        VStack(spacing: 1) {
            Spacer()

            // Modal box
            VStack(spacing: 1) {
                headerView
                Text("")
                fieldsView
                Text("")
                errorAndActionsView
            }

            Spacer()
        }
        .onKeyPress("\t") { nextField() }
        .onKeyPress("\r") { Task { await save() } }
        .onKeyPress("\u{1B}") { onCancel() }
    }

    // MARK: - View Components

    private var headerView: some View {
        VStack(spacing: 1) {
            HStack {
                Text(TUITheme.divider(width: 60))
                Spacer()
            }
            HStack {
                Text(" Edit Song")
                Spacer()
            }
            HStack {
                Text(TUITheme.divider(width: 60))
                Spacer()
            }
        }
    }

    private var fieldsView: some View {
        VStack(spacing: 1) {
            titleFieldView
            artistFieldView
            albumFieldView
            yearFieldView
        }
    }

    private var errorAndActionsView: some View {
        VStack(spacing: 1) {
            if let error = errorMessage {
                HStack {
                    Text(TUIColors.Indicators.error("Error: \(error)"))
                    Spacer()
                }
                Text("")
            }

            if isSaving {
                HStack {
                    Text("Saving...")
                    Spacer()
                }
            } else {
                HStack {
                    Text("[Enter] Save")
                    Text("  ")
                    Text("[Esc] Cancel")
                    Text("  ")
                    Text("[Tab] Next Field")
                    Spacer()
                }
            }

            HStack {
                Text(TUITheme.divider(width: 60))
                Spacer()
            }
        }
    }

    // MARK: - Field Views

    private var titleFieldView: some View {
        HStack {
            Text(currentField == .title ? " > " : "   ")
            Text("Title:  ")
            if currentField == .title {
                TextField(placeholder: "Enter title") { value in
                    editedTitle = value
                    nextField()
                }
            } else {
                Text(TUITheme.truncate(editedTitle, width: 40))
            }
            Spacer()
        }
    }

    private var artistFieldView: some View {
        HStack {
            Text(currentField == .artist ? " > " : "   ")
            Text("Artist: ")
            if currentField == .artist {
                TextField(placeholder: "Enter artist") { value in
                    editedArtist = value
                    nextField()
                }
            } else {
                Text(TUITheme.truncate(editedArtist, width: 40))
            }
            Spacer()
        }
    }

    private var albumFieldView: some View {
        HStack {
            Text(currentField == .album ? " > " : "   ")
            Text("Album:  ")
            if currentField == .album {
                TextField(placeholder: "Enter album") { value in
                    editedAlbum = value
                    nextField()
                }
            } else {
                Text(TUITheme.truncate(editedAlbum, width: 40))
            }
            Spacer()
        }
    }

    private var yearFieldView: some View {
        HStack {
            Text(currentField == .year ? " > " : "   ")
            Text("Year:   ")
            if currentField == .year {
                TextField(placeholder: "Enter year (optional)") { value in
                    editedYear = value
                    nextField()
                }
            } else {
                Text(editedYear.isEmpty ? "(none)" : editedYear)
            }
            Spacer()
        }
    }

    // MARK: - Actions

    private func nextField() {
        switch currentField {
        case .title:
            currentField = .artist
        case .artist:
            currentField = .album
        case .album:
            currentField = .year
        case .year:
            currentField = .title
        }
    }

    private func save() async {
        guard !isSaving else { return }
        isSaving = true
        errorMessage = nil

        // Validate
        guard !editedTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            errorMessage = "Title cannot be empty"
            isSaving = false
            return
        }

        guard !editedArtist.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            errorMessage = "Artist cannot be empty"
            isSaving = false
            return
        }

        // Parse year
        var yearValue: Int? = nil
        if !editedYear.isEmpty {
            if let year = Int(editedYear), year > 1900 && year <= 2100 {
                yearValue = year
            } else {
                errorMessage = "Year must be a valid number between 1900 and 2100"
                isSaving = false
                return
            }
        }

        do {
            let songRepo = await dependencies.songRepository

            // Create updated song (keeping the same ID and other fields)
            let updatedSong = Song(
                id: song.id,
                songKey: song.songKey,  // Keep same key for now
                artist: editedArtist.trimmingCharacters(in: .whitespacesAndNewlines),
                title: editedTitle.trimmingCharacters(in: .whitespacesAndNewlines),
                album: editedAlbum.trimmingCharacters(in: .whitespacesAndNewlines),
                albumArtist: song.albumArtist,
                releaseYear: yearValue,
                discNumber: song.discNumber,
                trackNumber: song.trackNumber,
                coverArtPath: song.coverArtPath,
                bookmark: song.bookmark,
                pathHash: song.pathHash,
                createdAt: song.createdAt,
                updatedAt: Date(),
                localFilePath: song.localFilePath,
                fileState: song.fileState
            )

            _ = try await songRepo.upsertSong(updatedSong)

            // Success - call onSave callback
            await onSave()
        } catch {
            errorMessage = "Failed to save: \(error.localizedDescription)"
            isSaving = false
        }
    }
}
