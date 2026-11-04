import Foundation
import SwiftTUI
import LocalWaveDomain

struct TUIPlaylistsView: View {
    let dependencies: TUIDependencyContainer
    @Binding var navigationState: NavigationState
    var songID: Int64? = nil
    @State private var playlists: [Playlist] = []
    @State private var selection = 0
    @State private var creating = false
    @State private var error: String?

    var body: some View {
        VStack {
            Text(songID == nil ? "Playlists" : "Select a playlist")
            if let error { Text(error) }
            if creating {
                TextField(placeholder: "New playlist name") { name in
                    Task { await create(name) }
                }
            } else if playlists.isEmpty {
                Text("No playlists. Press n to create one.")
            } else {
                ForEach(Array(playlists.enumerated().dropFirst((selection / 10) * 10).prefix(10)), id: \.element.id) { index, playlist in
                    Text("\(index == selection ? ">" : " ") \(playlist.name)")
                }
            }
            Spacer()
            Text("[j/k] Select  [Enter] Open/add  [n] New  [Esc] Back")
        }
        .onAppear { Task { await load() } }
        .onKeyPress("j") { if !creating { selection = min(selection + 1, max(0, playlists.count - 1)) } }
        .onKeyPress("k") { if !creating { selection = max(0, selection - 1) } }
        .onKeyPress("n") { if !creating { creating = true } }
        .onKeyPress("\r") { if !creating { Task { await open() } } }
        .onKeyPress("\u{1B}") {
            if creating { creating = false } else { navigationState.pop() }
        }
    }

    private func load() async {
        do { playlists = try await dependencies.playlistRepo.getAll() }
        catch { self.error = error.localizedDescription }
    }

    private func create(_ name: String) async {
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else { error = "Enter a playlist name."; return }
        do {
            _ = try await dependencies.playlistRepo.create(playlist: Playlist(id: nil, name: name, createdAt: Date(), updatedAt: nil))
            creating = false
            await load()
        } catch { self.error = error.localizedDescription }
    }

    private func open() async {
        guard playlists.indices.contains(selection), let id = playlists[selection].id else { return }
        if let songID {
            do {
                try await dependencies.playlistSongRepo.addSong(playlistId: id, songId: songID)
                navigationState.pop()
            } catch { self.error = error.localizedDescription }
        } else {
            navigationState.push(.playlistDetail(playlistId: id))
        }
    }
}

struct TUIPlaylistDetailView: View {
    let dependencies: TUIDependencyContainer
    let playlistID: Int64
    @Binding var navigationState: NavigationState
    @State private var songs: [Song] = []
    @State private var selection = 0
    @State private var error: String?

    var body: some View {
        VStack {
            Text("Playlist • \(songs.count) tracks")
            if let error { Text(error) }
            if songs.isEmpty { Text("Add songs from the Songs, Artists or Albums view using p.") }
            ForEach(Array(songs.enumerated().dropFirst((selection / 10) * 10).prefix(10)), id: \.element.id) { index, song in
                Text("\(index == selection ? ">" : " ") \(song.title) — \(song.artist)")
            }
            Spacer()
            Text("[j/k] Select  [Space] Play  [x] Remove track  [Esc] Back")
        }
        .onAppear { Task { await load() } }
        .onKeyPress("j") { selection = min(selection + 1, max(0, songs.count - 1)) }
        .onKeyPress("k") { selection = max(0, selection - 1) }
        .onKeyPress(" ") {
            guard songs.indices.contains(selection) else { return }
            Task { @MainActor in
                dependencies.playerViewModel.configureQueue(songs: songs, startIndex: selection)
                dependencies.playerViewModel.playSong(songs[selection])
            }
        }
        .onKeyPress("x") { Task { await remove() } }
        .onKeyPress("\u{1B}") { navigationState.pop() }
    }

    private func load() async {
        do {
            songs = try await dependencies.playlistSongRepo.getSongs(playlistId: playlistID)
            selection = min(selection, max(0, songs.count - 1))
        } catch { self.error = error.localizedDescription }
    }

    private func remove() async {
        guard songs.indices.contains(selection), let id = songs[selection].id else { return }
        do {
            try await dependencies.playlistSongRepo.removeSong(playlistId: playlistID, songId: id)
            await load()
        } catch { self.error = error.localizedDescription }
    }
}
