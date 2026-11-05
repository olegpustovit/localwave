import SwiftUI
import LocalWaveDomain

/// Shared track actions for Library rows and the compact Now menu.
@MainActor
enum LWTrackMenu {
    static func request(
        song: Song, targets: [Song], model: WaveLibraryModel,
        player: PlayerViewModel, presenter: LWSheetPresenter,
        playlistID: Int64? = nil, sharing: Bool = false,
        onPlay: @escaping () -> Void, onFolder: @escaping (Song) -> Void,
        onShare: @escaping ([Song]) -> Void,
        onFavoriteFeedback: @escaping () -> Void = {},
        onToggleFavorite: (() -> Void)? = nil,
        onRetry: ((Song) -> Void)? = nil
    ) -> LWMenuRequest {
        let one = targets.count == 1 ? targets.first : nil
        let pinned = targets.allSatisfy { model.pins.contains(String($0.uniqueId)) }
        let favored = targets.allSatisfy { $0.isFavorite != 0 }
        let groups = [
            LWMenuGroup([
                LWMenuAction(title: targets.count == 1 ? "Play" : "Play \(targets.count) songs", icon: "play", shortcut: "↩", action: {
                    if targets.count == 1 { onPlay() }
                    else {
                        player.configureQueue(songs: targets, startIndex: 0, context: .selection("Selected songs"))
                        if let first = targets.first { player.playSong(first) }
                    }
                }),
                LWMenuAction(title: "Play next", icon: "next2", action: { queueNext(targets, player: player) }),
                LWMenuAction(title: "Add to queue", icon: "queue", action: {
                    targets.forEach { player.addToQueue($0) }
                    LWPresentationNotice.show(targets.count > 1 ? "\(targets.count) songs added to queue" : "Added to queue")
                })
            ]),
            LWMenuGroup([LWMenuAction(title: "Add to list", icon: "plus", submenu: {
                var actions = model.lists.filter { $0.id != playlistID }.compactMap { list -> LWMenuAction? in
                    guard let id = list.id else { return nil }
                    return LWMenuAction(title: list.name, icon: "queue", action: {
                        Task { await model.perform {
                            try await addToList(songIDs: targets.compactMap(\.id), listID: id, listName: list.name, dependencies: model.dependencies)
                        } }
                    })
                }
                actions.append(LWMenuAction(title: "New list…", icon: "plus", action: {
                    presentNewList(presenter: presenter, model: model, songs: targets)
                }))
                return [LWMenuGroup(actions)]
            })])
        ]
        var result = groups
        var navigation: [LWMenuAction] = []
        if let one {
            navigation.append(LWMenuAction(title: "Go to album", icon: "album", action: {
                NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: PlaybackContext.album(name: one.album, artist: one.albumArtist.isEmpty ? one.artist : one.albumArtist))
            }))
            navigation.append(LWMenuAction(title: "Show in folder", icon: "folder", disabled: one.sourceId == nil, action: { onFolder(one) }))
        }
        navigation.append(LWMenuAction(title: targets.count == 1 ? (favored ? "Unfavorite" : "Favorite") : (favored ? "Unfavorite all" : "Favorite all"), icon: "heart", shortcut: targets.count == 1 ? "L" : nil, action: {
            if targets.count == 1, let onToggleFavorite { onToggleFavorite(); return }
            if targets.count == 1 && !favored { onFavoriteFeedback() }
            Task { await model.perform {
                for id in targets.compactMap(\.id) {
                    try await model.dependencies.songRepository.toggleSongAsFavorite(songId: id, state: !favored)
                }
            } }
        }))
        result.append(LWMenuGroup(navigation))
        var files = [LWMenuAction(title: pinned ? "Stop keeping offline" : "Keep offline", icon: pinned ? "check" : "dl", action: {
            Task { await model.togglePins(targets) }
        })]
        if let one, one.fileState == .failed, let id = one.id {
            files.append(LWMenuAction(title: "Retry download", icon: "rep", action: {
                if let onRetry { onRetry(one); return }
                Task {
                    await model.perform { try await model.dependencies.songRepository.markSongForCopy(songId: id) }
                    NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
                }
            }))
        }
        files.append(LWMenuAction(title: "Share…", icon: "share", disabled: sharing, action: { onShare(targets) }))
        result.append(LWMenuGroup(files))
        if let playlistID {
            result.append(LWMenuGroup([LWMenuAction(title: "Remove from list", icon: "close", shortcut: "⌫", destructive: true, action: {
                Task { await model.remove(targets.compactMap(\.id), from: playlistID) }
            })]))
        }
        let ext = song.fileExtension?.lowercased() ?? URL(fileURLWithPath: song.relativePath ?? "").pathExtension.lowercased()
        let size = song.fileSize.map { String(format: "%.1f MB", Double($0) / 1_048_576) } ?? "size unknown"
        return LWMenuRequest(groups: result, preview: AnyView(LWMenuTrackPreview(song: song, subtitle: song.artist + " · " + ext + " " + size)))
    }

    static func addToList(songIDs: [Int64], listID: Int64, listName: String, dependencies: DependencyContainer) async throws {
        let existing = Set(try await dependencies.playlistSongRepo.getSongs(playlistId: listID).compactMap(\.id))
        var seen = Set<Int64>()
        let added = songIDs.filter { seen.insert($0).inserted && !existing.contains($0) }
        for id in added { try await dependencies.playlistSongRepo.addSong(playlistId: listID, songId: id) }
        LWPresentationNotice.show(added.isEmpty ? "Already in “\(listName)”" : "Added \(added.count) to “\(listName)”")
    }

    static func presentNewList(presenter: LWSheetPresenter, model: WaveLibraryModel, songs: [Song], onCreated: ((Playlist) -> Void)? = nil) {
        let message = songs.isEmpty ? "Name your list." : "\(songs.count) song\(songs.count == 1 ? "" : "s") will be added."
        presenter.present(dialog: LWDialogRequest(title: "New List", message: message, input: LWDialogInput(placeholder: "List name"), primaryTitle: "Create", onSubmit: { value in
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            let name = trimmed.isEmpty ? "Untitled list" : trimmed
            Task {
                guard let list = await model.createList(name: name, songs: songs) else { return }
                LWPresentationNotice.show("Created “\(list.name)”")
                onCreated?(list)
            }
        }))
    }

    static func queueNext(_ songs: [Song], player: PlayerViewModel) {
        let count = player.queue.count
        let destination = min(count, max(0, player.currentQueueIndex + 1))
        songs.forEach { player.addToQueue($0) }
        if count > 0 && !songs.isEmpty {
            player.reorderQueue(from: IndexSet(integersIn: count..<(count + songs.count)), to: destination)
        }
        LWPresentationNotice.show(songs.count > 1 ? "\(songs.count) songs play next" : "Plays next")
    }
}
