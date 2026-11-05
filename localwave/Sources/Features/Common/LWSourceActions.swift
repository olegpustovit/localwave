import SwiftUI
import UniformTypeIdentifiers
import LocalWaveCore
import LocalWaveDomain

struct LWSourceActions: ViewModifier {
    let source: Source?
    let songs: [Song]
    let dependencies: DependencyContainer
    let onOpen: () -> Void
    let onRemoved: () -> Void
    @EnvironmentObject private var player: PlayerViewModel
    @State private var reconnecting = false
    @EnvironmentObject private var presenter: LWSheetPresenter
    @State private var error: String?

    @ViewBuilder func body(content: Content) -> some View {
        if let source, source.id != nil {
            content.lwContextMenu {
                LWMenuRequest(groups: menuGroups(source), preview: AnyView(
                    VStack(alignment: .leading, spacing: 4) {
                        Text(URL(fileURLWithPath: source.dirPath).lastPathComponent).font(LWTheme.text(17).weight(.medium))
                        Text(source.dirPath).font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary)
                    }.padding(16).frame(maxWidth: 300, alignment: .leading).background(LWTheme.paper)))
            }
            .fileImporter(isPresented: $reconnecting, allowedContentTypes: [.folder]) { result in
                perform {
                    let saved = try await dependencies.reconnectSource(source, to: result.get())
                    dependencies.waveSyncViewModel.resyncSource(saved)
                }
            }
            .alert("Source", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) {
                Button("OK") { error = nil }
            } message: { Text(error ?? "") }
        } else { content }
    }
    private func menuGroups(_ source: Source) -> [LWMenuGroup] {
        [LWMenuGroup([
            LWMenuAction(title: "Open", icon: "folder", action: onOpen),
            LWMenuAction(title: "Play all", icon: "play", disabled: songs.isEmpty, action: { play(source) }),
            LWMenuAction(title: "Rescan now", icon: "rep", action: { dependencies.waveSyncViewModel.resyncSource(source) })
        ]), LWMenuGroup([
            source.syncError != nil
                ? LWMenuAction(title: "Reconnect folder…", icon: "folder", action: { reconnecting = true })
                : LWMenuAction(title: "Keep everything offline", icon: "dl", disabled: songs.isEmpty, action: { keepOffline() })
        ]), LWMenuGroup([
            LWMenuAction(title: "Remove source", icon: "close", destructive: true, action: { confirmRemove(source) })
        ])]
    }

    private func confirmRemove(_ source: Source) {
        guard let id = source.id else { return }
        presenter.present(dialog: LWDialogRequest(title: "Remove \(URL(fileURLWithPath: source.dirPath).lastPathComponent)?",
            message: "Its songs leave the library. Files are not deleted.", primaryTitle: "Remove", destructive: true,
            mobileConfirmation: true, onSubmit: { _ in
                perform {
                    if dependencies.waveSyncViewModel.currentSyncSourceId == id { await dependencies.songImportService.cancelImport() }
                    try await dependencies.sourceAccess.remove(sourceId: id)
                    dependencies.waveSyncViewModel.loadSources()
                    onRemoved()
                }
            }))
    }

    private func play(_ source: Source) {
        guard let id = source.id else { return }
        let queue = songs.sorted { ($0.relativePath ?? $0.title).localizedStandardCompare($1.relativePath ?? $1.title) == .orderedAscending }
        guard let first = queue.first else { return }
        player.configureQueue(songs: queue, startIndex: 0, context: .folder(sourceID: id, pathID: source.pathId, name: URL(fileURLWithPath: source.dirPath).lastPathComponent))
        player.playSong(first)
    }
    private func keepOffline() {
        let ids = songs.map { String($0.uniqueId) }
        withOfflineCacheLock {
            var pins = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
            pins.formUnion(ids)
            LocalWavePreferences.current.set(Array(pins), forKey: "lwPinnedSongs")
        }
        perform {
            for song in songs where song.fileState != .copied {
                if let id = song.id { try await dependencies.songRepository.markSongForCopy(songId: id) }
            }
            NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
        }
    }
    private func perform(_ operation: @escaping @MainActor () async throws -> Void) {
        Task {
            do { try await operation() }
            catch { self.error = error.localizedDescription }
            NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        }
    }
}
