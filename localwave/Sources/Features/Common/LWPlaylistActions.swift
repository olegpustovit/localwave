#if os(iOS) || os(macOS)
import SwiftUI
import LocalWaveCore
import LocalWaveDomain

@MainActor
private final class LWPlaylistMenuContents: ObservableObject {
    @Published var songs: [Song] = []
}

/// The same list actions back the library rows and desktop sidebar.
struct LWPlaylistActions: ViewModifier {
    let playlist: Playlist?
    let dependencies: DependencyContainer
    var onDelete: (() -> Void)? = nil
    @EnvironmentObject private var player: PlayerViewModel
    @StateObject private var contents = LWPlaylistMenuContents()
    private var songs: [Song] { contents.songs }
    var fallbackSong: Song? = nil
    @EnvironmentObject private var presenter: LWSheetPresenter
    @State private var error: String?

    @ViewBuilder func body(content: Content) -> some View {
        if let playlist, let id = playlist.id {
            content.lwContextMenu {
                LWMenuRequest(groups: menuGroups(playlist), preview: AnyView(LWPlaylistMenuPreview(playlist: playlist, songs: songs, fallbackSong: fallbackSong)))
            }
            .task(id: id) { await reload(id) }
            .onReceive(NotificationCenter.default.publisher(for: Notification.Name("LibraryRefresh"))) { _ in Task { await reload(id) } }
            .alert("List", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) {
                Button("OK") { error = nil }
            } message: { Text(error ?? "") }
        } else { content }
    }

    private func menuGroups(_ playlist: Playlist) -> [LWMenuGroup] {
        [
            LWMenuGroup([
                LWMenuAction(title: "Play", icon: "play", disabled: songs.isEmpty) { play(shuffled: false) },
                LWMenuAction(title: "Shuffle", icon: "shuf", disabled: songs.isEmpty) { play(shuffled: true) },
                LWMenuAction(title: "Add to queue", icon: "queue", disabled: songs.isEmpty) { songs.forEach { player.addToQueue($0) } }
            ]),
            LWMenuGroup([
                LWMenuAction(title: "Rename…", icon: "set") { rename(playlist) },
                LWMenuAction(title: "Duplicate", icon: "plus") { duplicate(playlist) },
                LWMenuAction(title: keptOffline ? "Stop keeping offline" : "Keep offline", icon: "dl", disabled: songs.isEmpty) { toggleOffline() }
            ]),
            LWMenuGroup([LWMenuAction(title: "Delete list", icon: "close", destructive: true) { confirmDelete(playlist) }])
        ]
    }

    private func rename(_ playlist: Playlist) {
        guard let id = playlist.id else { return }
        presenter.present(dialog: LWDialogRequest(title: "Rename List", input: LWDialogInput(value: playlist.name, placeholder: "List name"), primaryTitle: "Save", onSubmit: { value in
            let name = value.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !name.isEmpty else { return }
            perform { _ = try await dependencies.playlistRepo.update(playlist: Playlist(id: id, name: name, createdAt: playlist.createdAt, updatedAt: Date())) }
        }))
    }

    private func confirmDelete(_ playlist: Playlist) {
        guard let id = playlist.id else { return }
        presenter.present(dialog: LWDialogRequest(title: "Delete “\(playlist.name)”?", message: "The songs stay in your library and folders.", primaryTitle: "Delete List", destructive: true, mobileConfirmation: true, onSubmit: { _ in
            perform {
                try await dependencies.playlistRepo.delete(playlistId: id)
                LWPresentationNotice.show("Deleted “\(playlist.name)”")
                onDelete?()
            }
        }))
    }
    private var keptOffline: Bool {
        let pins = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
        return !songs.isEmpty && songs.allSatisfy { pins.contains(String($0.uniqueId)) }
    }
    private func reload(_ id: Int64) async {
        do { contents.songs = try await dependencies.playlistSongRepo.getSongs(playlistId: id) }
        catch { self.error = error.localizedDescription }
    }
    private func play(shuffled: Bool) {
        guard let playlist, let id = playlist.id else { return }
        let queue = shuffled ? songs.shuffled() : songs
        guard let first = queue.first else { return }
        player.configureQueue(songs: queue, startIndex: 0, context: .playlist(id: id, name: playlist.name))
        player.playSong(first)
    }
    private func duplicate(_ playlist: Playlist) {
        let contents = songs
        perform {
            let ids = contents.compactMap(\.id)
            guard ids.count == contents.count else { throw CustomError.genericError("Some songs are no longer available. Refresh the list and try again.") }
            if !ids.isEmpty {
                let saved = Set(await dependencies.songRepository.getSongs(ids: ids).compactMap(\.id))
                guard Set(ids).isSubset(of: saved) else { throw CustomError.genericError("Some songs are no longer available. Refresh the list and try again.") }
            }
            let copy = try await dependencies.playlistRepo.create(playlist: Playlist(id: nil, name: playlist.name + " copy", createdAt: Date(), updatedAt: nil))
            guard let id = copy.id else { throw CustomError.genericError("The list could not be created.") }
            do {
                for song in contents { if let songID = song.id { try await dependencies.playlistSongRepo.addSong(playlistId: id, songId: songID) } }
            } catch {
                try? await dependencies.playlistRepo.delete(playlistId: id)
                throw error
            }
            LWPresentationNotice.show("Duplicated as “\(copy.name)”")
        }
    }
    private func toggleOffline() {
        let releasing = keptOffline
        let contents = songs
        withOfflineCacheLock {
            var pins = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
            for song in contents {
                if releasing { pins.remove(String(song.uniqueId)) } else { pins.insert(String(song.uniqueId)) }
            }
            LocalWavePreferences.current.set(Array(pins), forKey: "lwPinnedSongs")
        }
        perform {
            if !releasing {
                for song in contents where song.fileState != .copied { if let id = song.id { try await dependencies.songRepository.markSongForCopy(songId: id) } }
            }
            NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
        }
    }
    private func perform(_ operation: @escaping @MainActor () async throws -> Void) {
        Task {
            do {
                try await operation()
                NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
            } catch { self.error = error.localizedDescription }
        }
    }
}

/// Content only: the shared context host supplies the reference preview surface.
struct LWPlaylistMenuPreview: View {
    let playlist: Playlist
    let songs: [Song]
    var fallbackSong: Song? = nil
    var body: some View {
        HStack(spacing: 12) {
            LWArtwork(song: songs.first ?? fallbackSong, size: 56)
            VStack(alignment: .leading, spacing: 0) {
                Text(playlist.name).font(LWTheme.text(15)).lwLineBox(15, 21).lineLimit(1)
                Text("\(songs.count) songs").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
            }.frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

#endif
