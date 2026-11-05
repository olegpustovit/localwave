import SwiftUI
import Combine
import CoreText
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

@MainActor
final class WaveLibraryModel: ObservableObject {
    @Published var songs: [Song] = [] { didSet { indexLibrary() } }
    private(set) var albums: [Album] = []
    private(set) var artists: [String] = []
    private var albumSongs: [String: [Song]] = [:]
    private var insertionSongs: [Song] = []
    private var artistSongs: [String: [Song]] = [:]
    @Published var sourceCount = 0
    @Published var blockedSources: [Int64: String] = [:]
    @Published var lists: [Playlist] = []
    @Published fileprivate var foundFolders: [WaveFolderResult] = []
    @Published var rankedSongIDs: [Int64]?
    @Published private(set) var rankedSongQuery: String?
    @Published fileprivate var foundFolderQuery: String?
    @Published var listSongs: [Int64: [Song]] = [:]
    @Published var availabilityByID: [Int64: FileAvailability] = [:]
    private var availabilityTask: Task<Void, Never>?
    private var availabilityGeneration = UUID()
    @Published var error: String?
    @Published var loading = false
    private var reloadRequested = false
    @Published var undoRemoval: (listID: Int64, songIDs: [Int64], order: [Int64])?
    @Published var selecting = false
    @Published var selected = Set<Int64>()
    var selectionListID: Int64?
    var selectionAnchor: Int64?
    let dependencies: DependencyContainer
    private let playlistSongRepository: PlaylistSongRepository
    private var refreshObservation: AnyCancellable?

    init(_ dependencies: DependencyContainer, playlistSongRepository: PlaylistSongRepository? = nil) {
        self.dependencies = dependencies
        self.playlistSongRepository = playlistSongRepository ?? dependencies.playlistSongRepo
        // Keep pending refreshes alive across SwiftUI view updates.
        refreshObservation = NotificationCenter.default.publisher(for: Notification.Name("SongListRefresh"))
            .merge(with: NotificationCenter.default.publisher(for: Notification.Name("LibraryRefresh")))
            .debounce(for: .milliseconds(150), scheduler: RunLoop.main)
            .sink { [weak self] _ in
                Task { @MainActor [weak self] in
                    await self?.reload()
                }
            }
    }
    deinit { availabilityTask?.cancel() }

    func reload() async {
        reloadRequested = true
        guard !loading else { return }
        loading = true
        availabilityTask?.cancel()
        availabilityGeneration = UUID()
        defer {
            loading = false
            if reloadRequested {
                // A cancelled view task must not discard a refresh coalesced by another caller.
                Task { @MainActor [weak self] in await self?.reload() }
            }
        }
        repeat {
            reloadRequested = false
            do {
                var loaded: [Song] = []
                var seen = Set<Int64>()
                var offset = 0
                while !Task.isCancelled {
                    let page = try await dependencies.songRepository.searchSongsFTS(query: "", limit: 250, offset: offset)
                    loaded += page.filter { seen.insert($0.uniqueId).inserted }
                    offset += page.count
                    if page.count < 250 { break }
                }
                guard !Task.isCancelled else { return }
                let playlists = try await dependencies.playlistRepo.getAll()
                if let user = try await dependencies.userCloudService.resolveCurrentICloudUser(), let userID = user.id {
                    let sources = try await dependencies.sourceService.repository().findOneByUserId(userId: userID, path: nil)
                    sourceCount = sources.count
                    blockedSources = Dictionary(sources.compactMap { source -> (Int64, String)? in
                        guard source.syncError != nil, let id = source.id else { return nil }
                        return (id, URL(fileURLWithPath: source.dirPath).lastPathComponent)
                    }, uniquingKeysWith: { first, _ in first })
                }
                var contents: [Int64: [Song]] = [:]
                for list in playlists {
                    if let id = list.id { contents[id] = try await playlistSongRepository.getSongs(playlistId: id) }
                }
                guard !Task.isCancelled else { return }
                songs = loaded
                availabilityByID = availabilityByID.filter { seen.contains($0.key) }
                lists = playlists
                listSongs = contents
            } catch is CancellationError { }
            catch { self.error = error.localizedDescription }
        } while reloadRequested && !Task.isCancelled
        inspectAvailability(songs)
    }

    func createList(name: String, songs: [Song]) async -> Playlist? {
        var created: Playlist?
        do {
            let ids = songs.compactMap(\.id)
            guard ids.count == songs.count else { throw CustomError.genericError("Some selected songs are no longer available. Refresh the library and try again.") }
            if !ids.isEmpty {
                let saved = Set(await dependencies.songRepository.getSongs(ids: ids).compactMap(\.id))
                guard Set(ids).isSubset(of: saved) else { throw CustomError.genericError("Some selected songs are no longer available. Refresh the library and try again.") }
            }
            let list = try await dependencies.playlistRepo.create(playlist: Playlist(id: nil, name: name, createdAt: Date(), updatedAt: nil))
            created = list
            guard let id = list.id else { throw CustomError.genericError("The list could not be created.") }
            var inserted = Set<Int64>()
            for songID in songs.compactMap(\.id) where inserted.insert(songID).inserted {
                try await playlistSongRepository.addSong(playlistId: id, songId: songID)
            }
            NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
            await reload()
            return list
        } catch {
            if let id = created?.id { try? await dependencies.playlistRepo.delete(playlistId: id) }
            self.error = error.localizedDescription
            await reload()
            return nil
        }
    }

    private func inspectAvailability(_ snapshot: [Song]) {
        let generation = availabilityGeneration
        let access = dependencies.sourceAccess
        availabilityTask = Task { [weak self] in
            var batch: [Int64: FileAvailability] = [:]
            var lastPublished = Date()
            for song in snapshot {
                guard !Task.isCancelled else { return }
                let state = await access.availability(for: song)
                guard !Task.isCancelled else { return }
                batch[song.uniqueId] = state
                if batch.count >= 250 || Date().timeIntervalSince(lastPublished) >= 0.25 {
                    guard let self, self.availabilityGeneration == generation else { return }
                    self.availabilityByID.merge(batch, uniquingKeysWith: { _, latest in latest })
                    batch = [:]
                    lastPublished = Date()
                    await Task.yield()
                }
            }
            guard !Task.isCancelled, let self, self.availabilityGeneration == generation else { return }
            self.availabilityByID.merge(batch, uniquingKeysWith: { _, latest in latest })
        }
    }

    func perform(_ operation: () async throws -> Void) async {
        do {
            try await operation()
            NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        } catch { self.error = error.localizedDescription }
        await reload()
    }

    func searchSongs(_ query: String) async {
        guard !query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { rankedSongIDs = nil; rankedSongQuery = nil; return }
        do {
            try await Task.sleep(for: .milliseconds(150))
            let text = WaveSongQuery(query).text
            guard !text.isEmpty else { rankedSongIDs = nil; rankedSongQuery = text; return }
            var found: [Song] = []
            while !Task.isCancelled {
                let page = try await dependencies.songRepository.searchSongsFTS(query: text, limit: 250, offset: found.count)
                found += page
                if page.count < 250 { break }
            }
            try Task.checkCancellation()
            rankedSongIDs = found.map(\.uniqueId)
            rankedSongQuery = text
        } catch is CancellationError { }
        catch { self.error = error.localizedDescription }
    }

    var pins: Set<String> { Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? []) }

    func togglePins(_ requested: [Song]) async {
        let songs = requested.filter { waveBlockedSourceName(song: $0, availability: availabilityByID[$0.uniqueId] ?? .unknown, blockedSources: blockedSources) == nil }
        guard !songs.isEmpty else { return }
        var kept = pins
        let releasing = songs.allSatisfy { kept.contains(String($0.uniqueId)) }
        for song in songs {
            let key = String(song.uniqueId)
            if releasing { kept.remove(key) } else { kept.insert(key) }
        }
        withOfflineCacheLock { LocalWavePreferences.current.set(Array(kept), forKey: "lwPinnedSongs") }
        if !releasing {
            await perform {
                for song in songs where song.fileState != .copied {
                    if let id = song.id { try await dependencies.songRepository.markSongForCopy(songId: id) }
                }
            }
        } else { objectWillChange.send() }
        NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
    }

    func searchFolders(_ query: String) async {
        let text = query.split(whereSeparator: \.isWhitespace).map(String.init)
            .filter { !["cloud", "offline", "favorites", "fav", "flac", "mp3", "m4a", "unplayed"].contains($0.lowercased()) && !$0.lowercased().hasPrefix("year:") }.joined(separator: " ")
        guard !text.isEmpty else { foundFolders = []; foundFolderQuery = WaveSongQuery(query).text; return }
        do {
            try await Task.sleep(for: .milliseconds(150))
            guard let user = try await dependencies.userCloudService.resolveCurrentICloudUser(), let userID = user.id else { return }
            let sources = try await dependencies.sourceService.repository().findOneByUserId(userId: userID, path: nil)
            var results: [WaveFolderResult] = []
            for source in sources {
                guard let sourceID = source.id else { continue }
                let paths = try await dependencies.sourceService.importService().search(sourceId: sourceID, query: text)
                results += paths.filter(\.isDirectory).map { WaveFolderResult(source: source, path: $0) }
            }
            try Task.checkCancellation()
            foundFolders = results
            foundFolderQuery = WaveSongQuery(query).text
        } catch is CancellationError { }
        catch { self.error = error.localizedDescription }
    }

    func remove(_ songID: Int64, from listID: Int64) async { await remove([songID], from: listID) }

    func remove(_ songIDs: [Int64], from listID: Int64) async {
        guard !songIDs.isEmpty else { return }
        let order = listSongs[listID]?.compactMap(\.id) ?? []
        var removed: [Int64] = []
        do {
            for id in songIDs {
                try await playlistSongRepository.removeSong(playlistId: listID, songId: id)
                removed.append(id)
            }
        } catch { self.error = error.localizedDescription }
        if !removed.isEmpty {
            selected = []
            selecting = false
            selectionAnchor = nil
            undoRemoval = (listID, removed, order)
        }
        NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        await reload()
    }

    @discardableResult func undoRemove() async -> Bool {
        guard let removed = undoRemoval else { return false }
        var restored = false
        await perform {
            for id in removed.songIDs { try await playlistSongRepository.addSong(playlistId: removed.listID, songId: id) }
            try await playlistSongRepository.reorderSongs(playlistId: removed.listID, newOrder: removed.order)
            restored = true
        }
        if restored, let pending = undoRemoval,
           pending.listID == removed.listID, pending.songIDs == removed.songIDs, pending.order == removed.order {
            undoRemoval = nil
        }
        return restored
    }

    private func indexLibrary() {
        insertionSongs = songs.sorted { $0.uniqueId < $1.uniqueId }
        artistSongs = Dictionary(grouping: insertionSongs, by: \.artist)
        var seen = Set<String>()
        var indexedAlbums: [Album] = []
        var grouped: [String: [Song]] = [:]
        for song in songs {
            let album = Album(name: song.album, artist: song.albumArtist.isEmpty ? song.artist : song.albumArtist, coverArtPath: song.coverArtPath)
            if seen.insert(album.id).inserted { indexedAlbums.append(album) }
            grouped[album.id, default: []].append(song)
        }
        for id in grouped.keys {
            grouped[id]?.sort { ($0.discNumber ?? 1, $0.trackNumber ?? Int.max, $0.title) < ($1.discNumber ?? 1, $1.trackNumber ?? Int.max, $1.title) }
        }
        albums = indexedAlbums.sorted { ($0.artist ?? "", $0.name) < ($1.artist ?? "", $1.name) }
        artists = Array(Set(songs.map(\.artist))).sorted { waveArtistSortName($0).localizedStandardCompare(waveArtistSortName($1)) == .orderedAscending }
        albumSongs = grouped
    }

    func songs(in album: Album) -> [Song] { albumSongs[album.id] ?? [] }
    var firstSongInOriginalOrder: Song? { insertionSongs.first }
    var songsInOriginalOrder: [Song] { insertionSongs }
    func songs(byArtist artist: String) -> [Song] { artistSongs[artist] ?? [] }

    func stateKey(_ song: Song, pins suppliedPins: Set<String>? = nil) -> String {
        if waveBlockedSourceName(song: song, availability: availabilityByID[song.uniqueId] ?? .unknown, blockedSources: blockedSources) != nil { return "blocked" }
        if song.fileState == .copyPending || song.fileState == .failed { return "\(song.fileState.rawValue)" }
        if (suppliedPins ?? pins).contains(String(song.uniqueId)) { return "pinned" }
        return (availabilityByID[song.uniqueId] ?? .unknown).rawValue
    }

    func automatic(_ name: String) -> [Song] {
        switch name {
        case "Favorites": return insertionSongs.filter { $0.isFavorite != 0 }
        case "Offline": return insertionSongs.filter { availabilityByID[$0.uniqueId] == .onDevice }
        case "Kept Offline": return insertionSongs.filter { pins.contains(String($0.uniqueId)) }
        case "Recently Added": return waveRecentlyAddedSongs(insertionSongs)
        case "Most Played":
            let counts = LocalWavePreferences.current.dictionary(forKey: "lwPlayCounts") as? [String: Int] ?? [:]
            return waveMostPlayedSongs(insertionSongs, counts: counts)
        case "Never Played":
            let counts = LocalWavePreferences.current.dictionary(forKey: "lwPlayCounts") as? [String: Int] ?? [:]
            return insertionSongs.filter { counts[String($0.uniqueId), default: 0] == 0 }
        case "Recently Played":
            let played = LocalWavePreferences.current.dictionary(forKey: "lwLastPlayed") as? [String: Double] ?? [:]
            return songs.filter { played[String($0.uniqueId)] != nil }.sorted { played[String($0.uniqueId), default: 0] > played[String($1.uniqueId), default: 0] }
        default: return songs
        }
    }
}

private struct WaveLibraryMenuPreview: View {
    let song: Song?
    let title: String
    let subtitle: String
    var body: some View {
        HStack(spacing: 12) {
            LWArtwork(song: song, size: 56)
            VStack(alignment: .leading, spacing: 0) {
                Text(title).font(LWTheme.text(15)).lwLineBox(15, 21).lineLimit(1)
                Text(subtitle).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).lineLimit(1)
            }.frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

private struct WaveFolderResult: Identifiable {
    let source: Source
    let path: SourcePath
    var id: String { "\(path.sourceId)-\(path.pathId)" }
}

struct WaveLibraryNavigationRequest {
    let id = UUID()
    let context: PlaybackContext

    init(context: PlaybackContext) { self.context = context }
}

private enum WaveLibraryRoute: Hashable {
    case album(Album), artist(String), list(Int64), automatic(String)
}

@MainActor
private final class WaveLibraryNavigationMotion: ObservableObject {
    @Published var generation = 0
    @Published var forward = true
    @Published var target: WaveLibraryRoute?
    @Published var startedAt = Date.distantPast
    @Published var animatesPage = false

    init(rowEntryStartedAt: Date? = nil, target: WaveLibraryRoute? = nil) {
        if let rowEntryStartedAt {
            generation = 1
            self.target = target
            startedAt = rowEntryStartedAt
        }
    }

    func begin(from previous: [WaveLibraryRoute], to next: [WaveLibraryRoute]) {
        animatesPage = true
        forward = next.count > previous.count
        target = next.last
        startedAt = Date()
        generation += 1
    }

    func refreshRows(on route: WaveLibraryRoute?, startedAt: Date = Date()) {
        animatesPage = false
        forward = true
        target = route
        self.startedAt = startedAt
        generation += 1
    }

    func reset() { generation = 0; target = nil; startedAt = .distantPast; animatesPage = false }
}

private struct WaveLibraryNavigationKey: EnvironmentKey {
    static let defaultValue: (@MainActor (WaveLibraryRoute) -> Void)? = nil
}

private struct WaveLibraryEntry: Equatable {
    var generation = 0
    var enabled = false
    var startedAt = Date.distantPast
}

private struct WaveLibraryEntryKey: EnvironmentKey { static let defaultValue = WaveLibraryEntry() }

private extension EnvironmentValues {
    var waveLibraryNavigate: (@MainActor (WaveLibraryRoute) -> Void)? {
        get { self[WaveLibraryNavigationKey.self] }
        set { self[WaveLibraryNavigationKey.self] = newValue }
    }
    var waveLibraryEntry: WaveLibraryEntry {
        get { self[WaveLibraryEntryKey.self] }
        set { self[WaveLibraryEntryKey.self] = newValue }
    }
}

/// Keep NavigationStack's controllers/history while making the route mutation explicit.
private struct WaveLibraryRouteLink<Label: View>: View {
    let value: WaveLibraryRoute
    let label: Label
    @Environment(\.waveLibraryNavigate) private var navigate
    init(value: WaveLibraryRoute, @ViewBuilder label: () -> Label) {
        self.value = value; self.label = label()
    }
    var body: some View {
        if let navigate { Button { navigate(value) } label: { label } }
        else { NavigationLink(value: value) { label } }
    }
}

private extension WaveLibraryRouteLink where Label == Text {
    init(_ title: LocalizedStringKey, value: WaveLibraryRoute) {
        self.init(value: value) { Text(title) }
    }
}

private struct WaveLibraryPageEntrance: ViewModifier {
    let route: WaveLibraryRoute?
    @EnvironmentObject private var motion: WaveLibraryNavigationMotion
    @Environment(\.lwWideLayout) private var wide
    func body(content: Content) -> some View {
        // A native interactive back reveals a different target before its path commit.
        // Leave that gesture's presentation alone instead of replaying the previous push.
        let entering = motion.generation > 0 && motion.target == route && (!wide || motion.forward)
        content.modifier(LWAppear(translation: CGSize(width: wide ? 0 : motion.forward ? 32 : -32, height: 0), duration: wide ? LWMotion.base : LWMotion.slow, enabled: entering && motion.animatesPage))
            .environment(\.waveLibraryEntry, WaveLibraryEntry(generation: motion.generation, enabled: entering, startedAt: motion.startedAt))
    }
}

struct WaveLibraryRootState: Equatable {
    var mode = "Albums"
    var query = ""
    var albumSort = "Artist"
    var songSort = "Title"
    var showAllFindSongs = false
}

struct WaveLibraryView: View {
    let dependencies: DependencyContainer
    let section: String
    @StateObject private var model: WaveLibraryModel
    @State private var mode = "Albums"
    @State private var query = ""
    @FocusState private var findFocused: Bool
    @Environment(\.lwLibraryKeyboardCovered) private var keyboardCovered
    @State private var showAllFindSongs = false
    @State private var albumSort = "Artist"
    @State private var songSort = "Title"
    @State private var tableSort: String?
    @State private var tableAscending = true
    @State private var keyboardScrollTarget: Int64?
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    private let countHeight: CGFloat = 46
    @ScaledMetric(relativeTo: .body) private var tableTextScale: CGFloat = 1
    @State private var path: [WaveLibraryRoute] = []
    @StateObject private var navigationMotion = WaveLibraryNavigationMotion()
    @State private var albumFolders: [String: WaveFolderResult] = [:]
    @State private var contentWidth: CGFloat = 650
    @Environment(\.lwWideLayout) private var wideLayout
    @EnvironmentObject private var playerVM: PlayerViewModel
    @EnvironmentObject private var presenter: LWSheetPresenter
    @Environment(\.lwReduceMotion) private var reduceMotion

    private let rootEntryStartedAt: Date?
    private let onRootStateChange: ((WaveLibraryRootState) -> Void)?
    private let onDetailChange: ((Bool) -> Void)?
    private let onSidebarRouteChange: ((String?) -> Void)?
    private let navigationRequest: WaveLibraryNavigationRequest?
    private let onRootBack: (() -> Void)?
    private let onNavigationPathChange: (([PlaybackContext]) -> Void)?
    private let rootBackBoundaryDepth: Int
    private let onSelectionChange: (([Song]) -> Void)?

    init(dependencies: DependencyContainer, section: String, initialContext: PlaybackContext? = nil, initialQuery: String? = nil, initialShowAll: Bool? = nil, rootState: WaveLibraryRootState? = nil, onRootStateChange: ((WaveLibraryRootState) -> Void)? = nil, rootEntryStartedAt: Date? = nil, onDetailChange: ((Bool) -> Void)? = nil, navigationRequest: WaveLibraryNavigationRequest? = nil, onRootBack: (() -> Void)? = nil, onSidebarRouteChange: ((String?) -> Void)? = nil, initialNavigationPath: [PlaybackContext]? = nil, onNavigationPathChange: (([PlaybackContext]) -> Void)? = nil, rootBackBoundaryDepth: Int? = nil, onSelectionChange: (([Song]) -> Void)? = nil, sharedModel: WaveLibraryModel? = nil) {
        self.dependencies = dependencies
        self.section = section
        self.rootEntryStartedAt = rootEntryStartedAt
        self.onRootStateChange = onRootStateChange
        self.onDetailChange = onDetailChange
        self.onSidebarRouteChange = onSidebarRouteChange
        self.navigationRequest = navigationRequest
        self.onRootBack = onRootBack
        self.onNavigationPathChange = onNavigationPathChange
        self.onSelectionChange = onSelectionChange
        _query = State(initialValue: initialQuery ?? rootState?.query ?? "")
        _showAllFindSongs = State(initialValue: initialShowAll ?? rootState?.showAllFindSongs ?? false)
        let explicitMode = ["Albums", "Artists", "Songs"].contains(section) ? section : nil
        let detailContext = initialContext.flatMap(Self.route(for:)) != nil
        _mode = State(initialValue: detailContext ? rootState?.mode ?? explicitMode ?? "Albums" : explicitMode ?? rootState?.mode ?? "Albums")
        _albumSort = State(initialValue: rootState?.albumSort ?? "Artist")
        _songSort = State(initialValue: rootState?.songSort ?? "Title")
        var initialPath: [WaveLibraryRoute] = []
        if section.hasPrefix("List:"), let id = Int64(section.dropFirst(5)) { initialPath = [.list(id)] }
        if let initialContext {
            switch initialContext {
            case let .album(name, artist): initialPath = [.album(Album(name: name, artist: artist, coverArtPath: nil))]
            case .artist(let artist): initialPath = [.artist(artist)]
            case .playlist(let id, _): initialPath = [.list(id)]
            case .automatic(let name): initialPath = [.automatic(name)]
            default: break
            }
        }
        let seededBoundary = !initialPath.isEmpty && onRootBack != nil ? 1 : 0
        if let initialNavigationPath { initialPath = initialNavigationPath.compactMap(Self.route(for:)) }
        self.rootBackBoundaryDepth = min(initialPath.count, max(0, rootBackBoundaryDepth ?? seededBoundary))
        _path = State(initialValue: initialPath)
        _navigationMotion = StateObject(wrappedValue: WaveLibraryNavigationMotion(rowEntryStartedAt: rootEntryStartedAt, target: initialPath.last))
        _model = StateObject(wrappedValue: sharedModel ?? WaveLibraryModel(dependencies))
    }

    private var rootStateSnapshot: WaveLibraryRootState {
        WaveLibraryRootState(mode: mode, query: query, albumSort: albumSort, songSort: songSort, showAllFindSongs: showAllFindSongs)
    }

    private var sort: String {
        get { activeMode == "Songs" ? songSort : albumSort }
        nonmutating set {
            if activeMode == "Songs" { songSort = newValue }
            else { albumSort = newValue }
        }
    }

    private static func route(for context: PlaybackContext) -> WaveLibraryRoute? {
        switch context {
        case let .album(name, artist): return .album(Album(name: name, artist: artist, coverArtPath: nil))
        case .artist(let artist): return .artist(artist)
        case .playlist(let id, _): return .list(id)
        case .automatic(let name): return .automatic(name)
        default: return nil
        }
    }

    private var navigationPathSnapshot: [PlaybackContext] {
        path.map { route in
            switch route {
            case .album(let album): return .album(name: album.name, artist: album.artist ?? "Unknown Artist")
            case .artist(let artist): return .artist(artist)
            case .list(let id): return .playlist(id: id, name: model.lists.first { $0.id == id }?.name ?? "List")
            case .automatic(let name): return .automatic(name)
            }
        }
    }

    private var selectionSnapshot: [Song] {
        guard !model.selected.isEmpty else { return [] }
        return tableSongs(selectionScope, compact: !wideLayout).filter { model.selected.contains($0.uniqueId) }
    }

    private var canGoBack: Bool { path.count > rootBackBoundaryDepth || onRootBack != nil }

    private var navigationPath: Binding<[WaveLibraryRoute]> {
        Binding(get: { path }, set: { next in
            // A root-detail overlay's native back gesture crosses its retained Root boundary.
            if next.count < rootBackBoundaryDepth {
                onRootBack?()
                return
            }
            // Interactive navigation owns its gesture; only synchronize native history here.
            var transaction = Transaction(); transaction.disablesAnimations = true
            withTransaction(transaction) { path = next }
        })
    }

    private func pushPage(_ route: WaveLibraryRoute) { setPagePath(path + [route], animate: true) }

    private func openRequestedContext(_ context: PlaybackContext) {
        if let route = Self.route(for: context) { pushPage(route) }
    }

    private func popPage() {
        guard path.count > rootBackBoundaryDepth else { onRootBack?(); return }
        setPagePath(Array(path.dropLast()), animate: true)
    }

    private func setPagePath(_ next: [WaveLibraryRoute], animate: Bool) {
        var transaction = Transaction(); transaction.disablesAnimations = true
        withTransaction(transaction) {
            if animate { navigationMotion.begin(from: path, to: next) }
            else { navigationMotion.reset() }
            path = next
        }
    }

    var body: some View {
        GeometryReader { geometry in
            let compact = !wideLayout
            NavigationStack(path: navigationPath) {
                ScrollViewReader { proxy in
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        if section == "Find" { if compact { findHeader } }
                        else if section == "Lists" { listsHeader }
                        else if !["Favorites", "Offline", "Kept Offline", "Recently Added", "Recently Played", "Most Played", "Never Played"].contains(section) { libraryHeader(compact: compact) }

                        if section == "Lists" { listsBody }
                        else if section == "Find" {
                            if compact { findResults(compact: true) }
                            else { desktopFindResults }
                        }
                        else if ["Favorites", "Offline", "Kept Offline", "Recently Added", "Recently Played", "Most Played", "Never Played"].contains(section) {
                            automaticContent(section, compact: compact)
                        } else if activeMode == "Albums" { albumGrid(sortedAlbums, compact: compact) }
                        else if activeMode == "Artists" { artistRows(model.artists) }
                        else { songRows(sortedSongs, compact: compact) }

                        if model.songs.isEmpty && !model.loading && section != "Lists" {
                            Text("Your music starts with a folder. Add music in Folders to build your library.")
                                .foregroundStyle(LWTheme.secondary).padding(24)
                        }
                        if model.loading && model.songs.isEmpty { ProgressView().frame(maxWidth: .infinity).padding(28) }
                    }
                    .padding(.bottom, compact ? 24 : 0)
                }
                .background(LWTheme.paper)
                .overlay(alignment: .trailing) { if compact && isLibrary && activeMode != "Albums" { indexRail(proxy) } }
                .onChange(of: keyboardScrollTarget) { _, id in
                    if let id { withAnimation(reduceMotion ? nil : LWMotion.out(LWMotion.quick)) { proxy.scrollTo("song-" + String(id), anchor: .center) } }
                }
                }
                .modifier(WaveLibraryPageEntrance(route: nil))
                .accessibilityHidden(keyboardCovered)
                .toolbar(.hidden, for: .navigationBar)
                .navigationDestination(for: WaveLibraryRoute.self) { route in
                    detail(route, compact: compact).modifier(WaveLibraryPageEntrance(route: route)).accessibilityHidden(keyboardCovered)
                }
            }
            .background(LWTheme.paper)
            .environmentObject(navigationMotion)
            .environment(\.waveLibraryNavigate, pushPage)
            .tint(LWTheme.ink)
            .onAppear { contentWidth = geometry.size.width }
            .onChange(of: geometry.size.width) { _, width in contentWidth = width }
            .onChange(of: navigationRequest?.id) { _, _ in
                if let navigationRequest { openRequestedContext(navigationRequest.context) }
            }
            .onChange(of: section) { _, value in
                findFocused = false
                if ["Albums", "Artists", "Songs"].contains(value) { mode = value }
                let initial: [WaveLibraryRoute]
                if value.hasPrefix("List:"), let id = Int64(value.dropFirst(5)) { initial = [.list(id)] }
                else { initial = [] }
                setPagePath(initial, animate: false)
            }
        }
        .overlayPreferenceValue(WaveSelectionTableEnd.self) { anchor in
            GeometryReader { geometry in
                if wideLayout && model.selecting && model.selected.count > 1 {
                    let naturalTop = anchor.map { geometry[$0].maxY + 12 } ?? .infinity
                    ViewThatFits(in: .horizontal) {
                        desktopSelectionBar
                        ScrollView(.horizontal, showsIndicators: false) { desktopSelectionBar }.frame(height: 36)
                    }.padding(.horizontal, 12).frame(width: geometry.size.width)
                        .modifier(WaveSelectionBarEntrance())
                        .offset(y: min(naturalTop, geometry.size.height - 48))
                }
            }
        }
        .safeAreaInset(edge: .bottom, spacing: 0) {
            if !wideLayout && model.selecting { selectionBar }
        }
        .background { keyboardCommands }
        .onChange(of: path) { _, _ in findFocused = false; tableSort = nil; model.selected = []; model.selecting = false }
        .onChange(of: path, initial: true) { _, value in onDetailChange?(!value.isEmpty) }
        .onChange(of: navigationPathSnapshot, initial: true) { _, value in onNavigationPathChange?(value) }
        .onChange(of: selectionSnapshot, initial: true) { _, value in onSelectionChange?(value) }
        .onChange(of: sidebarRoute, initial: true) { _, value in onSidebarRouteChange?(value) }
        .onChange(of: rootStateSnapshot, initial: true) { _, value in onRootStateChange?(value) }
        .onChange(of: rootEntryStartedAt) { _, value in
            if let value { navigationMotion.refreshRows(on: path.last, startedAt: value) }
        }
        .onChange(of: query) { _, _ in showAllFindSongs = false }
        .task(id: query) { if section == "Find" { await model.searchSongs(query); await model.searchFolders(query) } }
        .task { await model.reload() }

    }

    private func askNewList(songs: [Song] = []) {
        LWTrackMenu.presentNewList(presenter: presenter, model: model, songs: songs) { list in
            if songs.isEmpty, let id = list.id { pushPage(.list(id)) }
        }
    }

    private func addToListGroups(_ songs: [Song], excluding excludedID: Int64? = nil, finishSelection: Bool = false) -> [LWMenuGroup] {
        let entries = model.lists.filter { $0.id != excludedID }.compactMap { list -> LWMenuAction? in
            guard let id = list.id else { return nil }
            return LWMenuAction(title: list.name, icon: "queue", action: {
                Task {
                    var added = false
                    await model.perform {
                        try await LWTrackMenu.addToList(songIDs: songs.compactMap(\.id), listID: id, listName: list.name, dependencies: dependencies)
                        added = true
                    }
                    if finishSelection && added { model.selecting = false; model.selected = [] }
                }
            })
        }
        return [LWMenuGroup(entries + [LWMenuAction(title: "New list…", icon: "plus", action: {
            if finishSelection { model.selecting = false; model.selected = [] }
            askNewList(songs: songs)
        })])]
    }

    private var isLibrary: Bool { ["Library", "Albums", "Artists", "Songs"].contains(section) }
    private var activeMode: String { isLibrary ? mode : section }
    private var sidebarRoute: String? {
        if let route = path.last {
            switch route {
            case .album, .artist: return nil
            case .list(let id): return "List:\(id)"
            case .automatic(let name): return automaticSidebarRoute(name)
            }
        }
        if isLibrary { return mode }
        if ["Favorites", "Recently Added", "Most Played", "Kept Offline", "Offline", "Never Played", "Recently Played"].contains(section) { return section }
        return nil
    }

    private func automaticSidebarRoute(_ name: String) -> String {
        ["Recently added": "Recently Added", "Most played": "Most Played", "Kept offline": "Kept Offline", "On this device now": "Offline", "Never played": "Never Played"][name] ?? name
    }

    private var sortedAlbums: [Album] {
        model.albums.sorted { a, b in
            if sort == "Title" { return a.name.localizedStandardCompare(b.name) == .orderedAscending }
            if sort == "Year" { return (model.songs(in: a).first?.releaseYear ?? 0) > (model.songs(in: b).first?.releaseYear ?? 0) }
            if sort == "Added" { return (model.songs(in: a).map(\.createdAt).max() ?? .distantPast) > (model.songs(in: b).map(\.createdAt).max() ?? .distantPast) }
            return (waveArtistSortName(a.artist ?? ""), a.name) < (waveArtistSortName(b.artist ?? ""), b.name)
        }
    }
    private var librarySize: String {
        let megabytes = Double(model.songs.reduce(Int64(0)) { $0 + ($1.fileSize ?? 0) }) / 1_048_576
        return String(format: "%.1f %@", megabytes >= 1024 ? megabytes / 1024 : megabytes, megabytes >= 1024 ? "GB" : "MB")
    }

    private var sortOptions: [String] { activeMode == "Songs" ? ["Title", "Artist", "Album", "Added", "Duration"] : ["Artist", "Title", "Year", "Added"] }
    private func sortLabel(_ value: String) -> String { value == "Added" ? "Recently added" : value }

    private var sortedSongs: [Song] {
        if wideLayout { return model.songsInOriginalOrder }
        return model.songsInOriginalOrder.sorted {
            if sort == "Artist" { return (waveArtistSortName($0.artist), $0.album, $0.trackNumber ?? 0) < (waveArtistSortName($1.artist), $1.album, $1.trackNumber ?? 0) }
            if sort == "Album" { return ($0.album, $0.trackNumber ?? 0) < ($1.album, $1.trackNumber ?? 0) }
            if sort == "Duration" { return ($0.duration ?? .infinity) < ($1.duration ?? .infinity) }
            if sort == "Added" { return $0.createdAt > $1.createdAt }
            if sort == "Year" { return ($0.releaseYear ?? 0) > ($1.releaseYear ?? 0) }
            return $0.title.localizedStandardCompare($1.title) == .orderedAscending
        }
    }

    @ViewBuilder private func libraryHeader(compact: Bool) -> some View {
        if !compact && isLibrary { desktopLibraryHeader }
        else {
        VStack(alignment: .leading, spacing: 0) {
            if compact && model.selecting {
                WaveMobileSelectionHeader(model: model, songs: selectionScope, title: isLibrary ? "Library" : section, inset: false)
            } else {
            HStack(alignment: .top) {
                Text(isLibrary ? (compact ? "Library" : activeMode) : section).font(LWTheme.text(compact ? 32 : 34).weight(.medium)).tracking(compact ? -1.12 : -0.85).lwLineBox(compact ? 32 : 34, compact ? 33.6 : 34)
                Spacer()
                if isLibrary {
                    Button { NotificationCenter.default.post(name: Notification.Name("LWCommand"), object: "Settings") } label: { LWIcon("set", size: 22).frame(width: 44, height: 44) }.buttonStyle(LWIconButtonStyle()).padding(.trailing, -10).accessibilityLabel("Settings")
                }
                if !model.songs.isEmpty && (!compact || !isLibrary) {
                    Button(model.selecting ? "Done" : "Select") { model.selecting.toggle(); model.selected = []; model.selectionListID = nil }.font(LWTheme.text(13))
                    Button { play(model.songs.shuffled()) } label: { Label("Shuffle all", systemImage: "shuffle").font(LWTheme.text(13)) }
                }
            }.frame(minHeight: 44)
            }
            if isLibrary {
                (dynamicTypeSize.isAccessibilitySize ? AnyLayout(VStackLayout(alignment: .leading, spacing: 8)) : AnyLayout(HStackLayout(alignment: .bottom, spacing: 10))) {
                    MatrixText(text: String(model.songs.count), height: countHeight, unlit: false)
                        .overlay(alignment: .topTrailing) { if playerVM.isPlaying { Rectangle().fill(LWTheme.signal).frame(width: 9, height: 9).offset(x: 14) } }
                        .accessibilityLabel("Songs in library").accessibilityValue(String(model.songs.count)).accessibilityIdentifier("library-song-count")
                    Text("songs").font(LWTheme.text(15)).lwLineBox(15, 21).padding(.bottom, 2).padding(.leading, playerVM.isPlaying ? 14 : 0)
                    if !dynamicTypeSize.isAccessibilitySize { Spacer() }
                    VStack(alignment: .trailing, spacing: 0) {
                        Text("\(model.albums.count) albums · \(librarySize)").lwLineBox(13, 19.5)
                        Text("from \(model.sourceCount) sources").lwLineBox(13, 19.5)
                    }.font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary)
                }.padding(.top, -6).padding(.bottom, 14)
            } else {
                Text("\(model.automatic(section).count) songs in your library")
                    .font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary)
            }
            if dynamicTypeSize.isAccessibilitySize { libraryModeControls(vertical: true) }
            else { ScrollView(.horizontal, showsIndicators: false) { libraryModeControls(vertical: false) } }
        }.padding(.horizontal, compact ? 16 : 26).padding(.top, compact ? 10 : 20).padding(.bottom, 1)
            .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1) }
        }
    }

    private func libraryModeControls(vertical: Bool) -> some View {
        let layout = vertical ? AnyLayout(VStackLayout(alignment: .leading, spacing: 0)) : AnyLayout(HStackLayout(spacing: 20))
        return layout {
                if isLibrary {
                    ForEach(["Albums", "Artists", "Songs"], id: \.self) { item in
                        Button {
                            navigationMotion.reset()
                            mode = item
                        } label: {
                            HStack(spacing: 6) {
                                Text(item).lwLineBox(15, 21)
                                Text(String(item == "Albums" ? model.albums.count : item == "Artists" ? model.artists.count : model.songs.count)).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
                            }.fixedSize(horizontal: true, vertical: false).font(LWTheme.text(15))
                                .foregroundStyle(mode == item ? LWTheme.ink : LWTheme.secondary)
                                .frame(minHeight: 44)
                                .overlay(alignment: .bottom) { if mode == item { Rectangle().fill(LWTheme.ink).frame(height: 2).offset(y: 1) } }
                        }.accessibilityIdentifier("library-mode-" + item).accessibilityAddTraits(mode == item ? .isSelected : [])
                    }
                }
                if !vertical { Spacer(minLength: 0) }
                if activeMode != "Artists" {
                LWMenuButton(request: { sortMenuRequest }) {
                    Text(sortLabel(sort) + " ↓").font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.ink).fixedSize()
                }
                .accessibilityAction(named: "Select songs") { model.selecting.toggle(); model.selected = []; model.selectionListID = nil }
                .accessibilityAction(named: "Shuffle all") { play(model.songs.shuffled(), context: .library) }
                }
        }.frame(minWidth: max(0, contentWidth - 32), alignment: .leading)
    }

    private var sortMenuRequest: LWMenuRequest {
        LWMenuRequest(groups: [LWMenuGroup(sortOptions.map { value in
            LWMenuAction(title: sortLabel(value), icon: sort == value ? "check" : nil, action: {
                sort = value
                if !wideLayout { navigationMotion.refreshRows(on: path.last) }
            })
        })])
    }

    private var desktopLibraryHeader: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(activeMode).font(LWTheme.text(34).weight(.medium)).tracking(-0.85).lwLineBox(34, 34).padding(.top, 28)
            Text(activeMode == "Songs" ? "Click a header to sort. Double-click or Return plays." : activeMode == "Artists" ? "Sorted ignoring “The”" : "\(model.songs.count) songs across \(model.sourceCount) sources · \(librarySize)")
                .font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).padding(.top, 6).padding(.bottom, 18)
            if activeMode == "Albums" {
                HStack(spacing: 8) {
                    desktopSorts
                    Spacer(minLength: 0)
                    Button { play(model.songs.shuffled()) } label: { HStack(spacing: 6) { LWIcon("shuf", size: 16); Text("Shuffle all").lwLineBox(13, 18.2) }.font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).padding(.horizontal, 10).frame(height: 30) }
                }.padding(.bottom, 14)
            }
        }.padding(.horizontal, 32)
            .overlay(alignment: .bottom) { if activeMode == "Albums" { Rectangle().fill(LWTheme.line).frame(height: 1) } }
            .padding(.bottom, activeMode == "Albums" ? 1 : 0)
    }

    private var desktopSorts: some View {
        HStack(spacing: 8) {
            ForEach(["Artist", "Year", "Added"], id: \.self) { value in
                Button { sort = value } label: {
                    Text(value == "Added" ? "Size" : value).font(LWTheme.text(13)).lwLineBox(13, 18.2).padding(.horizontal, 10).frame(height: 30)
                        .foregroundStyle(sort == value ? LWTheme.ink : LWTheme.secondary)
                        .background(sort == value ? LWTheme.panel2 : Color.clear)
                        .overlay { if sort == value { Rectangle().strokeBorder(LWTheme.ink, lineWidth: 1.5) } }
                }
            }
        }
    }

    private func albumGrid(_ albums: [Album], compact: Bool, artistDetail: Bool = false) -> some View {
        Group {
            if artistDetail && !compact {
                // LazyVGrid rounds the measured223.7pt card row up to224pt.
                // Native Grid preserves the CSS fractional row height and downstream table origin.
                let available = max(1, contentWidth - 64)
                let minimum: CGFloat = dynamicTypeSize.isAccessibilitySize ? 260 : 150
                let columns = max(1, Int((available + 20) / (minimum + 20)))
                let proposed = max(1, (available - CGFloat(columns - 1) * 20) / CGFloat(columns))
                let width = dynamicTypeSize.isAccessibilitySize ? proposed : min(180, proposed)
                Grid(alignment: .topLeading, horizontalSpacing: 20, verticalSpacing: 28) {
                    ForEach(Array(stride(from: 0, to: albums.count, by: columns)), id: \.self) { start in
                        GridRow {
                            ForEach(start..<min(start + columns, albums.count), id: \.self) { index in
                                albumCell(albums[index], index: index, compact: compact, artistDetail: artistDetail).frame(width: width)
                            }
                        }
                    }
                }.frame(maxWidth: .infinity, alignment: .leading)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: dynamicTypeSize.isAccessibilitySize ? 260 : compact ? 145 : 150), spacing: compact ? 12 : 20)], alignment: .leading, spacing: compact ? 20 : 28) {
                    ForEach(Array(albums.enumerated()), id: \.element.id) { index, album in
                        albumCell(album, index: index, compact: compact, artistDetail: artistDetail)
                    }
                }
            }
        }.padding(.horizontal, compact ? 16 : 32).padding(.top, artistDetail ? 0 : compact ? 16 : 22).padding(.bottom, compact ? 16 : 40)
    }

    private func albumCell(_ album: Album, index: Int, compact: Bool, artistDetail: Bool) -> some View {
        let first = model.songs(in: album).first
        let year: String? = first?.releaseYear.map { String($0) }
        let artist: String = album.artist ?? "Unknown Artist"
        let subtitle: String = artistDetail ? (year ?? "—") : artist + (compact ? "" : year.map { " · " + $0 } ?? "")
        return WaveLibraryRouteLink(value: WaveLibraryRoute.album(album)) {
            VStack(alignment: .leading, spacing: 0) {
                GeometryReader { proxy in
                    LWArtwork(song: first, size: proxy.size.width)
                }.aspectRatio(1, contentMode: .fit).clipped()
                Text(album.name).font(LWTheme.text(14).weight(.medium)).lineLimit(1).lwLineBox(14, 17.5).padding(.top, 8)
                Text(subtitle).font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lineLimit(1).lwLineBox(13, 18.2)
            }.contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .lwContextMenu {
            LWMenuRequest(groups: albumMenuGroups(album), preview: AnyView(WaveLibraryMenuPreview(song: model.songs(in: album).first, title: album.name, subtitle: album.artist ?? "Unknown Artist")))
        }
        .modifier(WaveLibraryEntrance(index: index))
    }

    @ViewBuilder private func artistRows(_ artists: [String]) -> some View {
        if wideLayout { desktopArtists(artists) }
        else {
            LazyVStack(spacing: 0) {
                ForEach(Array(artists.enumerated()), id: \.element) { index, artist in
                    let songs = model.songs(byArtist: artist)
                    let albumCount = Set(songs.map(\.album)).count
                    WaveLibraryRouteLink(value: WaveLibraryRoute.artist(artist)) {
                        HStack(spacing: 14) {
                            LWArtwork(song: songs.first, size: 44)
                            VStack(alignment: .leading, spacing: 0) {
                                Text(artist).font(LWTheme.text(15).weight(.medium)).lwLineBox(15, 21)
                                Text("\(albumCount) album\(albumCount == 1 ? "" : "s") · \(songs.count) songs").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
                            }
                            Spacer()
                            LWIcon("chev", size: 18).foregroundStyle(LWTheme.ink)
                        }.padding(.horizontal, 16).frame(minHeight: 64).contentShape(Rectangle())
                    }.buttonStyle(.plain).id("artist-" + artist).modifier(WaveLibraryEntrance(index: index))
                        .overlay(alignment: .top) { if index > 0 { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.leading, 16) } }
                }
            }
        }
    }

    private func artistAlbumNames(_ songs: [Song]) -> String {
        var seen = Set<String>()
        return songs.map(\.album).filter { seen.insert($0).inserted }.joined(separator: ", ")
    }

    private func artistMinutes(_ songs: [Song]) -> String {
        "\(Int((songs.compactMap(\.duration).reduce(0, +) / 60).rounded())) min"
    }

    private func artistColumnWidths(_ artists: [String]) -> [CGFloat] {
        func width(_ text: String, face: String, size: CGFloat) -> CGFloat {
            let font = UIFont(name: face, size: size * tableTextScale) ?? .systemFont(ofSize: size * tableTextScale)
            return (text as NSString).size(withAttributes: [.font: font]).width
        }
        let headings = ["Name", "Albums", "Songs", "Time"]
        var preferred = headings.enumerated().map { index, title in width(title, face: "HostGrotesk-Medium", size: 12) + (index == 0 ? 40 : index == 3 ? 28 : 16) }
        for artist in artists {
            let songs = model.songs(byArtist: artist)
            preferred[0] = max(preferred[0], min(240, width(artist, face: "HostGrotesk-Regular", size: 14) + 80))
            preferred[1] = max(preferred[1], min(240, width(artistAlbumNames(songs), face: "HostGrotesk-Regular", size: 14) + 16))
            preferred[2] = max(preferred[2], width(String(songs.count), face: "GeistMono-Regular", size: 12) + 16)
            preferred[3] = max(preferred[3], width(artistMinutes(songs), face: "GeistMono-Regular", size: 12) + 28)
        }
        let total = max(1, preferred.reduce(0, +))
        return preferred.map { $0 * contentWidth / total }
    }

    private func desktopArtists(_ artists: [String]) -> some View {
        let widths = artistColumnWidths(artists)
        return LazyVStack(spacing: 0, pinnedViews: [.sectionHeaders]) {
            Section {
                ForEach(Array(artists.enumerated()), id: \.element) { index, artist in
                    let songs = model.songs(byArtist: artist)
                    WaveLibraryRouteLink(value: WaveLibraryRoute.artist(artist)) {
                        HStack(alignment: .firstTextBaseline, spacing: 0) {
                            HStack(spacing: 12) { LWArtwork(song: songs.first, size: 28); Text(artist).font(LWTheme.text(14)).lwLineBox(14, 19.6).lineLimit(1) }
                                .padding(.leading, 32).padding(.trailing, 8).frame(width: widths[0], alignment: .leading)
                            Text(artistAlbumNames(songs)).font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).lineLimit(1).padding(.horizontal, 8).frame(width: widths[1], alignment: .leading)
                            Text(String(songs.count)).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary).padding(.horizontal, 8).frame(width: widths[2], alignment: .leading)
                            Text(artistMinutes(songs)).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary).padding(.leading, 8).padding(.trailing, 20).frame(width: widths[3], alignment: .trailing)
                        }.frame(minHeight: 38).contentShape(Rectangle())
                    }.buttonStyle(.plain).modifier(WaveTableHover()).id("artist-" + artist).modifier(WaveLibraryEntrance(index: index))
                        .modifier(LWCollapsedTableRules(top: true, bottom: index == artists.count - 1))
                }
            } header: {
                HStack(spacing: 0) {
                    ForEach(Array(["Name", "Albums", "Songs", "Time"].enumerated()), id: \.offset) { index, label in
                        Text(label).font(LWTheme.text(12).weight(.medium)).lwLineBox(12, 16.8).foregroundStyle(LWTheme.secondary)
                            .padding(.leading, index == 0 ? 32 : 8).padding(.trailing, index == 3 ? 20 : 8)
                            .frame(width: widths[index], alignment: index == 3 ? .trailing : .leading)
                    }
                }.padding(.vertical, 10).padding(.bottom, 0.5).background(LWTheme.paper)
                    .modifier(LWCollapsedTableRules(top: false, bottom: true))
            }
        }
    }

    private func tableSongs(_ songs: [Song], compact: Bool) -> [Song] {
        compact ? songs : waveSortedSongs(songs, column: tableSort, ascending: tableAscending, availability: model.availabilityByID)
    }

    private func tableHeading(_ label: String) -> some View {
        WaveSongTableHeading(label: label, column: $tableSort, ascending: $tableAscending)
    }

    private func songRows(_ songs: [Song], compact: Bool, playlistID: Int64? = nil, playbackSongs: [Song]? = nil, highlight: String = "", numbered: Bool = false, hideArtist: Bool = false, hideAlbum: Bool = false, entranceIndexOffset: Int = 0, showsEmptyMessage: Bool = true) -> some View {
        let ordered = tableSongs(songs, compact: compact)
        let mixedFormats = !compact && Set(songs.compactMap(\.fileExtension)).count > 1
        let pins = model.pins
        let mixedStates = !compact && Set(songs.map { model.stateKey($0, pins: pins) }).count > 1
        let columns: WaveSongColumns? = compact ? nil : WaveSongColumns.make(songs: ordered, available: contentWidth, scale: tableTextScale, artist: !hideArtist && contentWidth >= 400, album: !hideAlbum && contentWidth >= 520, format: mixedFormats && contentWidth >= 680, state: mixedStates, numbered: numbered, currentID: playerVM.currentSong?.id, sort: tableSort)
        let selectionOrder = ordered.map(\.uniqueId)
        return LazyVStack(spacing: 0, pinnedViews: [.sectionHeaders]) {
            Section {
                ForEach(Array(ordered.enumerated()), id: \.element.uniqueId) { index, song in
                    WaveLibrarySongRow(song: song, index: index, compact: compact, showArtist: !hideArtist && contentWidth >= 400, showAlbum: !hideAlbum && contentWidth >= 520, showFormat: mixedFormats && contentWidth >= 680, showTime: true, model: model, playlistID: playlistID, selectionOrder: selectionOrder, highlight: highlight, numbered: numbered, tableColumns: columns) { play(playbackSongs ?? ordered, startingAt: playbackSongs?.firstIndex { $0.uniqueId == song.uniqueId } ?? index) }
                        .id("song-" + String(song.uniqueId)).modifier(WaveLibraryEntrance(index: entranceIndexOffset + index))
                        .anchorPreference(key: WaveSelectionTableEnd.self, value: .bounds) { !compact && index == ordered.count - 1 ? $0 : nil }
                }
                if !compact && model.selecting && model.selected.count > 1 { Color.clear.frame(height: 48) }
                if ordered.isEmpty && showsEmptyMessage { Text("No songs here yet.").font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).padding(32).frame(maxWidth: .infinity, alignment: .leading) }
            } header: { if !compact, let columns { WaveSongColumnsHeading(columns: columns, sort: $tableSort, ascending: $tableAscending) } }
        }
    }

    private func indexRail(_ proxy: ScrollViewProxy) -> some View {
        var seen = Set<String>()
        let entries: [(String, String)] = activeMode == "Artists"
            ? model.artists.map { (String(waveArtistSortName($0).prefix(1)).uppercased(), "artist-" + $0) }
            : sortedSongs.map { (String($0.title.prefix(1)).uppercased(), "song-" + String($0.uniqueId)) }
        let anchors = entries.filter { seen.insert($0.0).inserted }
        return VStack(spacing: 0) {
            ForEach(anchors, id: \.0) { letter, anchor in
                Button {
                    withAnimation(reduceMotion ? nil : LWMotion.out(LWMotion.base)) { proxy.scrollTo(anchor, anchor: .top) }
                } label: { HStack(spacing: 3) { Text(letter).font(LWTheme.text(11)); Rectangle().frame(width: 8, height: 1.5) }.frame(width: 28, height: 17, alignment: .trailing).padding(.trailing, 6) }
                .accessibilityLabel("Jump to " + letter)
            }
        }.foregroundStyle(LWTheme.secondary).offset(x: -2)
    }

    private var listsHeader: some View {
        HStack {
            Text("Lists").font(LWTheme.text(32).weight(.medium)).tracking(-1.12).lwLineBox(32, 33.6)
            Spacer()

        }.frame(minHeight: 44, alignment: .top).padding(.horizontal, 16).padding(.top, 10)
    }

    private var listsBody: some View {
        VStack(alignment: .leading, spacing: 0) {
            sectionLabel("Automatic")
            ForEach(Array(["Favorites", "Recently Added", "Most Played", "Kept Offline", "Offline", "Never Played"].enumerated()), id: \.element) { index, name in
                WaveLibraryRouteLink(value: WaveLibraryRoute.automatic(name)) {
                    listLabel(["Recently Added": "Recently added", "Most Played": "Most played", "Kept Offline": "Kept offline", "Offline": "On this device now", "Never Played": "Never played"][name] ?? name, songs: model.automatic(name), automatic: true, divider: name != "Favorites")
                }.buttonStyle(.plain)
                .lwContextMenu {
                    let songs = model.automatic(name)
                    return LWMenuRequest(groups: automaticMenuGroups(name, songs: songs), preview: AnyView(WaveLibraryMenuPreview(song: songs.first ?? model.firstSongInOriginalOrder, title: automaticLabel(name), subtitle: "\(songs.count) songs")))
                }
                .modifier(WaveLibraryEntrance(index: index, mobileOnly: true))
            }
            sectionLabel("Yours")
            ForEach(Array(model.lists.enumerated()), id: \.element.id) { index, list in
                if let id = list.id {
                    WaveLibraryRouteLink(value: WaveLibraryRoute.list(id)) {
                        listLabel(list.name, songs: model.listSongs[id] ?? [], automatic: false, divider: list.id != model.lists.first?.id)
                    }.buttonStyle(.plain)
                    .modifier(LWPlaylistActions(playlist: list, dependencies: dependencies, fallbackSong: model.firstSongInOriginalOrder))
                    .modifier(WaveLibraryEntrance(index: 6 + index, mobileOnly: true))
                }
            }
            Button { askNewList() } label: {
                HStack(spacing: 8) { LWIcon("plus", size: 18); Text("New list").font(LWTheme.text(15)).lwLineBox(15, 21) }
                    .frame(maxWidth: .infinity).frame(height: 56)
                    .overlay { Rectangle().strokeBorder(LWTheme.line, style: StrokeStyle(lineWidth: 1.5, dash: [5, 4])) }
            }.foregroundStyle(LWTheme.secondary).padding(.horizontal, 16).padding(.top, 12)
        }
    }

    private func listLabel(_ name: String, songs: [Song], automatic: Bool, divider: Bool = true) -> some View {
        // The actual reference clips its collage to the first cover; empty lists use library art.
        let cover = songs.first ?? model.firstSongInOriginalOrder
        return HStack(spacing: 14) {
            LWArtwork(song: cover, size: 44)
            VStack(alignment: .leading, spacing: 0) {
                Text(name).font(LWTheme.text(15).weight(.medium)).lwLineBox(15, 21)
                Text(automatic ? "updates itself" : "\(Int((songs.compactMap(\.duration).reduce(0, +) / 60).rounded())) min").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
            }
            Spacer()
            Text(String(songs.count)).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
        }.padding(.horizontal, 16).frame(minHeight: 64).contentShape(Rectangle())
            .overlay(alignment: .top) { if divider { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.leading, 16) } }
    }

    private var findHeader: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Find").font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lwLineBox(13, 21)
            ZStack(alignment: .trailing) {
                TextField("Songs, albums, folders", text: $query, prompt: Text("Songs, albums, folders").foregroundStyle(LWTheme.tertiary))
                    .font(LWTheme.text(26).weight(.medium)).tracking(-0.52).autocorrectionDisabled().focused($findFocused).padding(.trailing, 40)
                Button { query = "" } label: { LWIcon("close", size: 18).frame(width: 44, height: 44) }.accessibilityLabel("Clear search").offset(x: 10)
            }.frame(height: 56).overlay(alignment: .bottom) { Rectangle().fill(LWTheme.ink).frame(height: 1.5) }
            WaveTokenLayout(spacing: 6) {
                    ForEach(["flac", "cloud", "offline", "year:2019", "unplayed"], id: \.self) { token in
                        let selected = query.lowercased().split(separator: " ").contains(Substring(token))
                        Button {
                            var words = query.split(separator: " ").map(String.init)
                            if selected { words.removeAll { $0.lowercased() == token } } else { words.append(token) }
                            query = words.joined(separator: " ")
                        } label: { Text(token).font(LWTheme.text(13)).padding(.horizontal, 12).frame(height: 32).foregroundStyle(selected ? LWTheme.paper : LWTheme.secondary).background(selected ? LWTheme.ink : LWTheme.panel) }
                    }
            }.padding(.top, 12)
        }.padding(.horizontal, 16).padding(.top, 20)
    }

    private var foundSongs: [Song] {
        let filter = WaveSongQuery(query)
        let ordered = waveFindSongsInOriginalOrder(model.songsInOriginalOrder, matchingIDs: model.rankedSongIDs, matchingQuery: model.rankedSongQuery, currentQuery: filter.text)
        return ordered.filter { filter.matches($0, availability: model.availabilityByID[$0.uniqueId] ?? .unknown) }
    }

    private var matchingFindFolders: [WaveFolderResult] {
        model.foundFolderQuery == WaveSongQuery(query).text ? model.foundFolders : []
    }

    private func matchingFindAlbums(_ found: [Song]) -> [Album] {
        let filter = WaveSongQuery(query)
        guard !filter.text.isEmpty else { return [] }
        let matchingIDs = Set(found.map { Album(name: $0.album, artist: $0.albumArtist.isEmpty ? $0.artist : $0.albumArtist, coverArtPath: $0.coverArtPath).id })
        return model.albums.filter { matchingIDs.contains($0.id) && filter.matchesWords(in: $0.name + " " + ($0.artist ?? "")) }
    }

    private var desktopFindResults: some View {
        let found = query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? [] : foundSongs
        let albums = matchingFindAlbums(found)
        let folderCount = WaveSongQuery(query).hasFilters ? 0 : matchingFindFolders.count
        let available = found.filter { waveBlockedSourceName(song: $0, availability: model.availabilityByID[$0.uniqueId] ?? .unknown, blockedSources: model.blockedSources) == nil }
        let pins = model.pins
        let pinned = !available.isEmpty && available.allSatisfy { pins.contains(String($0.uniqueId)) }
        let summary = "\(found.count) songs" + (albums.isEmpty ? "" : " · \(albums.count) albums") + (folderCount == 0 ? "" : " · \(folderCount) folders")
        return VStack(alignment: .leading, spacing: 0) {
            if onRootBack != nil { desktopBack }
            VStack(alignment: .leading, spacing: 0) {
                // .dk-head p overrides .meta's13px font in the rendered reference.
                Text("Search").font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).padding(.top, 6)
                Text("“" + query + "”").font(LWTheme.text(34).weight(.medium)).tracking(-0.85).lwLineBox(34, 34)
                    .accessibilityIdentifier("desktop-find-query").accessibilityAddTraits(.isHeader)
                Text(summary).font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).padding(.top, 6)
            }.padding(.horizontal, 32).padding(.top, 28).padding(.bottom, 18)
            HStack(spacing: 8) {
                Button { play(found, context: .selection("Find results")) } label: {
                    HStack(spacing: 6) { LWIcon("play", size: 16); Text("Play all").lwLineBox(13, 18.2) }.padding(.horizontal, 10).frame(height: 30)
                }.buttonStyle(LWIconButtonStyle()).modifier(WaveTableHover(opacity: 1)).disabled(found.isEmpty)
                Button { Task { await model.togglePins(available) } } label: {
                    HStack(spacing: 6) { LWIcon(pinned ? "check" : "dl", size: 18); Text(pinned ? "Kept offline" : "Keep offline").lwLineBox(13, 18.2) }.padding(.horizontal, 10).frame(height: 30)
                }.buttonStyle(LWIconButtonStyle()).modifier(WaveTableHover(opacity: 1, selected: pinned))
                    .foregroundStyle(pinned ? LWTheme.ink : LWTheme.secondary).disabled(available.isEmpty)
                    .accessibilityValue(pinned ? "Kept offline" : "Not kept offline")
                Spacer(minLength: 0)
            }.font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).padding(.horizontal, 32).padding(.bottom, 15)
                .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1) }
            songRows(found, compact: false, playbackSongs: found, highlight: query, showsEmptyMessage: false)
        }
    }

    @ViewBuilder private func findResults(compact: Bool) -> some View {
        if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            Text("Search spans every source, including folders that are not in an album. Add a word like **flac** or **cloud** to filter.")
                .font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).padding(.horizontal, 16).padding(.vertical, 40)
        } else {
            let filter = WaveSongQuery(query)
            let found = foundSongs
            let albums = matchingFindAlbums(found)
            let artists = filter.text.isEmpty || filter.hasFilters ? [] : model.artists.filter { filter.matchesWords(in: $0) }
            if !albums.isEmpty {
                sectionLabel("Albums \(albums.count)")
                LazyVStack(spacing: 0) {
                    ForEach(Array(albums.enumerated()), id: \.element.id) { index, album in
                        WaveLibraryRouteLink(value: WaveLibraryRoute.album(album)) {
                            HStack(spacing: 14) {
                                LWArtwork(song: model.songs(in: album).first, size: 44)
                                VStack(alignment: .leading, spacing: 0) {
                                    waveHighlightedLabel(album.name, query: query).font(LWTheme.text(15).weight(.medium)).lwLineBox(15, 21)
                                    waveHighlightedLabel((album.artist ?? "Unknown Artist") + (model.songs(in: album).first?.releaseYear.map { " · " + String($0) } ?? ""), query: query).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
                                }.frame(maxWidth: .infinity, alignment: .leading)
                                LWIcon("chev", size: 18).foregroundStyle(LWTheme.ink)
                            }.padding(.horizontal, 16).frame(minHeight: 64)
                        }.buttonStyle(.plain)
                            .modifier(WaveLibraryEntrance(index: index, mobileOnly: true))
                            .overlay(alignment: .bottom) { if album.id != albums.last?.id { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.leading, 16) } }
                    }
                }
            }
            sectionLabel("Songs \(found.count)")
            songRows(showAllFindSongs ? found : Array(found.prefix(8)), compact: compact, playbackSongs: found, highlight: query, entranceIndexOffset: compact ? albums.count : 0)
            if found.count > 8 && !showAllFindSongs {
                Button { showAllFindSongs = true } label: {
                    Text("Show all \(found.count) songs").font(LWTheme.text(15)).frame(maxWidth: .infinity).frame(minHeight: 56).background(LWTheme.panel)
                }.accessibilityIdentifier("find-show-all-songs").padding(.horizontal, 16).padding(.top, 8)
            }
            if !found.isEmpty {
                let pinned = found.allSatisfy { model.pins.contains(String($0.uniqueId)) }
                HStack(spacing: 8) {
                    Button { play(found, context: .selection("Find results")) } label: {
                        Label("Play results", systemImage: "play.fill").frame(maxWidth: .infinity).frame(minHeight: 52).background(LWTheme.key2)
                    }
                    Button { Task { await model.togglePins(found) } } label: {
                        Label(pinned ? "Kept offline" : "Keep results offline", systemImage: pinned ? "checkmark" : "arrow.down").frame(maxWidth: .infinity).frame(minHeight: 52).background(LWTheme.key2)
                    }
                }.font(LWTheme.text(14)).padding(16)
            }
            if !artists.isEmpty {
                sectionLabel("Artists")
                LazyVStack(spacing: 0) {
                    ForEach(Array(artists.enumerated()), id: \.element) { index, artist in
                        WaveLibraryRouteLink(value: WaveLibraryRoute.artist(artist)) {
                            HStack(spacing: 14) {
                                LWArtwork(song: found.first { $0.artist == artist }, size: 44)
                                waveHighlightedLabel(artist, query: query).font(LWTheme.text(15).weight(.medium)).lwLineBox(15, 21).frame(maxWidth: .infinity, alignment: .leading)
                                LWIcon("chev", size: 18).foregroundStyle(LWTheme.ink)
                            }.padding(.horizontal, 16).frame(minHeight: 64)
                        }.buttonStyle(.plain)
                            .modifier(WaveLibraryEntrance(index: albums.count + min(found.count, showAllFindSongs ? found.count : 8) + index, mobileOnly: true))
                            .overlay(alignment: .bottom) { if artist != artists.last { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.leading, 16) } }
                    }
                }
            }
            if !filter.hasFilters && !matchingFindFolders.isEmpty {
                sectionLabel("Folders")
                ForEach(Array(matchingFindFolders.enumerated()), id: \.element.id) { index, folder in
                    Button { openFolder(folder) } label: {
                        HStack(spacing: 12) {
                            LWIcon("folder", size: 18).frame(width: 16).foregroundStyle(LWTheme.secondary)
                            VStack(alignment: .leading, spacing: 0) {
                                waveHighlightedLabel(folder.path.name, query: query).font(LWTheme.text(15)).lwLineBox(15, 21)
                                Text(albumFolderLabel(folder)).font(LWTheme.data(13)).lwLineBox(13, 18.2, data: true).foregroundStyle(LWTheme.secondary).lineLimit(1)
                            }.frame(maxWidth: .infinity, alignment: .leading)
                        }.padding(.leading, 16).padding(.trailing, 6).frame(minHeight: 56)
                    }.buttonStyle(.plain)
                        .modifier(WaveLibraryEntrance(index: albums.count + min(found.count, showAllFindSongs ? found.count : 8) + artists.count + index, mobileOnly: true))
                        .overlay(alignment: .top) { if index > 0 { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.leading, 44) } }
                }
            }
        }
    }

    @ViewBuilder private func detail(_ route: WaveLibraryRoute, compact: Bool) -> some View {
        switch route {
        case .album(let album): albumDetail(album, compact: compact)
        case .artist(let artist): artistDetail(artist, compact: compact)
        case .automatic(let name): automaticDetail(name, compact: compact)
        case .list(let id): WaveListDetail(model: model, playlistID: id, onBack: {
            if path.last == .list(id) { popPage() }
        })
        }
    }

    private func albumDetail(_ album: Album, compact: Bool) -> some View {
        let songs = model.songs(in: album)
        return ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                if compact {
                    if model.selecting { WaveMobileSelectionHeader(model: model, songs: songs, title: "") }
                    else {
                    HStack(spacing: 0) {
                        detailBack
                        Spacer(minLength: 0)
                        HStack(spacing: 4) {
                            selectionTools(songs)
                            LWMenuButton(request: { LWMenuRequest(groups: albumMenuGroups(album)) }) { LWIcon("more", size: 22).frame(width: 44, height: 44) }.buttonStyle(LWIconButtonStyle()).accessibilityLabel("Album actions")
                        }.padding(.trailing, 16)
                    }.padding(.top, 10)
                    }
                    HStack(alignment: .bottom, spacing: 16) {
                        LWArtwork(song: songs.first, size: 152)
                        phoneAlbumMetadata(album, songs: songs)
                    }.padding(.horizontal, 16).padding(.top, 4)
                    playBar(songs)
                } else {
                    desktopBack
                    desktopAlbumHeading(album, songs: songs)
                }
                songRows(songs, compact: compact, numbered: true, hideArtist: true, hideAlbum: true)
                if compact { albumSourceFooter(album, songs: songs) }
            }
        }
        .task(id: album.id + "|" + String(songs.first?.sourceId ?? -1)) { await loadAlbumFolder(album) }
        .background(LWTheme.paper).toolbar(.hidden, for: .navigationBar)
    }

    private func phoneAlbumMetadata(_ album: Album, songs: [Song]) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(album.name).font(LWTheme.text(24).weight(.medium)).tracking(-0.72).lwLineBox(24, 27.6)
            if let artist = album.artist {
                WaveLibraryRouteLink(value: WaveLibraryRoute.artist(artist)) { Text(artist).font(LWTheme.text(17)).lwLineBox(17, 23.8).foregroundStyle(LWTheme.secondary) }.padding(.top, 2)
            }
            Text(albumSummary(songs)).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).padding(.top, 8)
            Text(albumFormat(songs)).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary).padding(.top, 2)
        }.frame(maxWidth: .infinity, alignment: .leading)
    }

    private func desktopAlbumHeading(_ album: Album, songs: [Song]) -> some View {
        ViewThatFits(in: .horizontal) {
            HStack(alignment: .bottom, spacing: 32) {
                LWArtwork(song: songs.first, size: 240)
                desktopAlbumMetadata(album, songs: songs).frame(minWidth: 360, maxWidth: .infinity, alignment: .leading)
            }
            VStack(alignment: .leading, spacing: 24) {
                LWArtwork(song: songs.first, size: min(240, max(120, contentWidth - 64)))
                desktopAlbumMetadata(album, songs: songs)
            }
        }.padding(.horizontal, 32).padding(.top, 8).padding(.bottom, 24)
    }

    private func desktopAlbumMetadata(_ album: Album, songs: [Song]) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Album").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
            Text(album.name).font(LWTheme.text(40).weight(.medium)).tracking(-1.2).lwLineBox(40, 40.8)
            if let artist = album.artist {
                WaveLibraryRouteLink(value: WaveLibraryRoute.artist(artist)) { Text(artist).font(LWTheme.text(18)).lwLineBox(18, 25.2).foregroundStyle(LWTheme.secondary) }.padding(.top, 6)
            }
            ViewThatFits(in: .horizontal) {
                HStack(alignment: .firstTextBaseline, spacing: 0) {
                    Text(albumSummary(songs) + " · ").font(LWTheme.text(13)).lwLineBox(13, 18.2)
                    Text(albumFormat(songs)).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true)
                }
                VStack(alignment: .leading, spacing: 2) {
                    Text(albumSummary(songs)).font(LWTheme.text(13)).lwLineBox(13, 18.2)
                    Text(albumFormat(songs)).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true)
                }
            }.foregroundStyle(LWTheme.secondary).padding(.top, 12)
            if let target = albumFolders[album.id] {
                HStack(alignment: .firstTextBaseline, spacing: 4) {
                    Text("Folder:").foregroundStyle(LWTheme.secondary)
                    Button { openFolder(target) } label: { Text(albumFolderLabel(target)).underline().lineLimit(1) }
                }.font(LWTheme.text(13)).lwLineBox(13, 18.2).padding(.top, 4)
            }
            playKeys(songs, showOffline: true).frame(maxWidth: 360).padding(.top, 20)
        }
    }

    private func albumFormat(_ songs: [Song]) -> String {
        guard let first = songs.first else { return "" }
        let parts: [String?] = [first.fileExtension?.lowercased(), first.sampleRate.map { String(format: "%g kHz", $0 / 1000) }, first.bitDepth.map { "\($0) bit" }]
        return parts.compactMap { $0 }.joined(separator: " · ")
    }

    private func albumFolderLabel(_ target: WaveFolderResult) -> String {
        URL(fileURLWithPath: target.source.dirPath).lastPathComponent + (target.path.relativePath.isEmpty ? "" : " / " + target.path.relativePath)
    }

    @ViewBuilder private func albumSourceFooter(_ album: Album, songs: [Song]) -> some View {
        if let target = albumFolders[album.id] {
            Button { openFolder(target) } label: {
                HStack(spacing: 10) {
                    LWIcon("folder", size: 18)
                    Text(albumFolderLabel(target)).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.ink).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
                    let cloud = songs.filter { model.availabilityByID[$0.uniqueId] == .inCloud }.count
                    Text(cloud > 0 ? "\(cloud) in cloud" : songs.allSatisfy { model.availabilityByID[$0.uniqueId] == .onDevice } ? "all offline" : "checking files").font(LWTheme.text(13)).lwLineBox(13, 18.2)
                }.padding(14).background(LWTheme.panel)
            }.foregroundStyle(LWTheme.secondary).padding(16)
        }
    }

    private func albumMenuGroups(_ album: Album) -> [LWMenuGroup] {
        let songs = model.songs(in: album)
        let context = PlaybackContext.album(name: album.name, artist: album.artist ?? "Unknown Artist")
        let pinned = !songs.isEmpty && songs.allSatisfy { model.pins.contains(String($0.uniqueId)) }
        return [
            LWMenuGroup([
                LWMenuAction(title: "Play", icon: "play", disabled: songs.isEmpty) { play(songs, context: context) },
                LWMenuAction(title: "Shuffle", icon: "shuf", disabled: songs.isEmpty) { play(songs.shuffled(), context: context) },
                LWMenuAction(title: "Play next", icon: "next2", disabled: songs.isEmpty) { LWTrackMenu.queueNext(songs, player: playerVM) },
                LWMenuAction(title: "Add to queue", icon: "queue", disabled: songs.isEmpty, action: { songs.forEach { playerVM.addToQueue($0) }; LWPresentationNotice.show("\(songs.count) songs added to queue") })
            ]),
            LWMenuGroup([LWMenuAction(title: "Add to list", icon: "plus", submenu: { addToListGroups(songs) })]),
            LWMenuGroup([
                LWMenuAction(title: "Go to artist", icon: "person") { NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: PlaybackContext.artist(album.artist ?? "Unknown Artist")) },
                LWMenuAction(title: "Show in folder", icon: "folder", disabled: songs.first?.sourceId == nil) { Task { await loadAlbumFolder(album); if let folder = albumFolders[album.id] { openFolder(folder) } } },
                LWMenuAction(title: pinned ? "Stop keeping offline" : "Keep offline", icon: "dl", disabled: songs.isEmpty) { Task { await model.togglePins(songs) } }
            ])
        ]
    }

    private func automaticMenuGroups(_ name: String, songs: [Song]) -> [LWMenuGroup] {
        [
            LWMenuGroup([
                LWMenuAction(title: "Play", icon: "play", disabled: songs.isEmpty) { play(songs, context: .automatic(name)) },
                LWMenuAction(title: "Shuffle", icon: "shuf", disabled: songs.isEmpty) { play(songs.shuffled(), context: .automatic(name)) },
                LWMenuAction(title: "Add to queue", icon: "queue", disabled: songs.isEmpty, action: { songs.forEach { playerVM.addToQueue($0) }; LWPresentationNotice.show("\(songs.count) songs added to queue") })
            ]),
            LWMenuGroup([LWMenuAction(title: !songs.isEmpty && songs.allSatisfy { model.pins.contains(String($0.uniqueId)) } ? "Stop keeping offline" : "Keep offline", icon: "dl", disabled: songs.isEmpty) { Task { await model.togglePins(songs) } }])
        ]
    }

    private func openFolder(_ target: WaveFolderResult) {
        guard let sourceID = target.source.id else { return }
        NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: PlaybackContext.folder(sourceID: sourceID, pathID: target.path.pathId, name: target.path.name))
    }

    private func albumsForArtist(_ songs: [Song]) -> [Album] {
        var seen = Set<String>()
        return songs.map { Album(name: $0.album, artist: $0.albumArtist.isEmpty ? $0.artist : $0.albumArtist, coverArtPath: $0.coverArtPath) }.filter { seen.insert($0.id).inserted }
    }

    private func artistDetail(_ artist: String, compact: Bool) -> some View {
        let songs = model.songs(byArtist: artist)
        let albums = albumsForArtist(songs)
        let summary = "\(albums.count) album\(albums.count == 1 ? "" : "s") · \(songs.count) songs"
        return ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                if compact {
                    if model.selecting { WaveMobileSelectionHeader(model: model, songs: songs, title: artist) }
                    else {
                    HStack(spacing: 0) { detailBack; Spacer(minLength: 0) }.padding(.top, 10)
                    VStack(alignment: .leading, spacing: 0) {
                        Text(artist).font(LWTheme.text(32).weight(.medium)).tracking(-1.12).lwLineBox(32, 33.6)
                        Text(summary).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).padding(.top, 4)
                    }.padding(.horizontal, 16)
                    }
                    playBar(songs, showOffline: false)
                } else {
                    desktopBack
                    ViewThatFits(in: .horizontal) {
                        HStack(alignment: .bottom, spacing: 24) {
                            desktopArtistTitle(artist, summary: summary).frame(minWidth: 260, maxWidth: .infinity, alignment: .leading)
                            playKeys(songs, showOffline: false).frame(width: 300)
                        }
                        VStack(alignment: .leading, spacing: 20) {
                            desktopArtistTitle(artist, summary: summary)
                            playKeys(songs, showOffline: false)
                        }
                    }.padding(.horizontal, 32).padding(.top, 28).padding(.bottom, 18)
                }
                albumGrid(albums, compact: compact, artistDetail: true)
                if compact { sectionLabel("Songs") }
                songRows(compact ? Array(songs.prefix(12)) : songs, compact: compact, playbackSongs: songs, hideArtist: true, entranceIndexOffset: albums.count)
            }
        }.background(LWTheme.paper).toolbar(.hidden, for: .navigationBar)
    }

    private func desktopArtistTitle(_ artist: String, summary: String) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Artist").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).padding(.top, 6)
            Text(artist).font(LWTheme.text(48).weight(.medium)).tracking(-1.2).lwLineBox(48, 48)
            Text(summary).font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).padding(.top, 6)
        }
    }

    private func automaticDetail(_ name: String, compact: Bool) -> some View {
        ScrollView { automaticContent(name, compact: compact) }.background(LWTheme.paper).toolbar(.hidden, for: .navigationBar)
    }

    private func automaticContent(_ name: String, compact: Bool) -> some View {
        let songs = model.automatic(name)
        return VStack(alignment: .leading, spacing: 0) {
                if compact {
                    if model.selecting { WaveMobileSelectionHeader(model: model, songs: songs, title: automaticLabel(name)) }
                    else {
                    HStack {
                        detailBack
                        Spacer()
                        HStack(spacing: 4) {
                            selectionTools(songs)
                            LWMenuButton(request: { LWMenuRequest(groups: automaticMenuGroups(name, songs: songs)) }) {
                                LWIcon("more", size: 22).frame(width: 44, height: 44)
                            }.buttonStyle(LWIconButtonStyle()).accessibilityLabel("List actions")
                        }.padding(.trailing, 16)
                    }.padding(.top, 10)
                    VStack(alignment: .leading, spacing: 0) {
                        Text(automaticLabel(name)).font(LWTheme.text(32).weight(.medium)).tracking(-1.12).lwLineBox(32, 33.6)
                        Text("\(songs.count) songs · updates itself").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).padding(.top, 4)
                    }.padding(.horizontal, 16)
                    }
                    playBar(songs, showOffline: false)
                } else {
                    VStack(alignment: .leading, spacing: 0) {
                        Text("Automatic list").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).padding(.top, 6)
                        Text(automaticLabel(name)).font(LWTheme.text(34).weight(.medium)).tracking(-0.85).lwLineBox(34, 34)
                        Text(artistMinutes(songs) + " · updates itself").font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).padding(.top, 6)
                    }.padding(.horizontal, 32).padding(.top, 28).padding(.bottom, 18)
                    desktopPlayTools(songs)
                }
                songRows(songs, compact: compact)
        }
    }

    private func automaticLabel(_ name: String) -> String {
        ["Recently Added": "Recently added", "Most Played": "Most played", "Kept Offline": "Kept offline", "Offline": "On this device now", "Never Played": "Never played"][name] ?? name
    }

    private func desktopPlayTools(_ songs: [Song]) -> some View {
        HStack(spacing: 8) {
            Button { play(songs) } label: { HStack(spacing: 6) { LWIcon("play", size: 16); Text("Play").lwLineBox(13, 18.2) }.padding(.horizontal, 10).frame(height: 30) }
            Button { play(songs.shuffled()) } label: { HStack(spacing: 6) { LWIcon("shuf", size: 16); Text("Shuffle").lwLineBox(13, 18.2) }.padding(.horizontal, 10).frame(height: 30) }
            Spacer(minLength: 0)
        }.font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).padding(.horizontal, 32).padding(.bottom, 14).padding(.bottom, 1)
            .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1) }
    }

    private func albumSummary(_ songs: [Song]) -> String {
        let prefix = songs.first?.releaseYear.map { String($0) + " · " } ?? ""
        let duration = songs.compactMap(\.duration).reduce(0.0, +)
        let minutes = Int((duration / 60).rounded())
        return prefix + "\(songs.count) songs · \(minutes) min"
    }

    private func loadAlbumFolder(_ album: Album) async {
        guard let song = model.songs(in: album).first, let sourceID = song.sourceId else { return }
        do {
            guard let source = try await dependencies.sourceService.repository().getOne(id: sourceID) else { return }
            let paths = try await dependencies.sourceService.importService().listItems(sourceId: sourceID, parentPathId: nil)
            let file = paths.first { $0.relativePath == song.relativePath }
            let parentID = file?.parentPathId ?? source.pathId
            if let folder = paths.first(where: { $0.isDirectory && $0.pathId == parentID }) {
                guard !Task.isCancelled else { return }
                albumFolders[album.id] = WaveFolderResult(source: source, path: folder)
            }
        } catch is CancellationError { }
        catch { model.error = error.localizedDescription }
    }

    private func selectionTools(_ songs: [Song]) -> some View {
        HStack {
            if model.selecting { Button("Select all") { model.selected = Set(songs.map(\.uniqueId)) } }
            Button(model.selecting ? "Done" : "Select") { model.selecting.toggle(); model.selected = []; model.selectionListID = nil }
        }.font(LWTheme.text(16)).lwLineBox(16, 22.4).padding(.horizontal, 4).frame(height: 44)
    }

    private var selectionScope: [Song] {
        if let current = path.last {
            switch current {
            case .album(let album): return model.songs(in: album)
            case .artist(let artist): return model.songs(byArtist: artist)
            case .list(let id): return model.listSongs[id] ?? []
            case .automatic(let name): return model.automatic(name)
            }
        }
        if section == "Find" { return query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? [] : foundSongs }
        if !isLibrary && section != "Lists" { return model.automatic(section) }
        return sortedSongs
    }

    private func moveSelection(by delta: Int, extend: Bool = false) {
        let ordered = tableSongs(selectionScope, compact: false)
        guard !ordered.isEmpty else { return }
        let current = keyboardScrollTarget.flatMap { id in ordered.firstIndex { $0.uniqueId == id } }
            ?? ordered.firstIndex { model.selected.contains($0.uniqueId) } ?? 0
        let index = max(0, min(ordered.count - 1, current + delta))
        let id = ordered[index].uniqueId
        if extend { model.selected.insert(id) } else { model.selected = [id]; model.selectionAnchor = id }
        keyboardScrollTarget = id
    }

    private var keyboardCommands: some View {
        Group {
            Button("Back in library", action: popPage).keyboardShortcut("[", modifiers: .command).disabled(!canGoBack)
            Button("Select all songs") {
                model.selecting = true
                model.selected = Set(selectionScope.map(\.uniqueId))
                if case .list(let id)? = path.last { model.selectionListID = id }
            }.keyboardShortcut("a", modifiers: .command).disabled((isLibrary && activeMode != "Songs" && path.isEmpty) || (section == "Find" && !wideLayout && !model.selecting))
            Button("Previous song selection") { moveSelection(by: -1) }.keyboardShortcut(.upArrow, modifiers: []).disabled(!wideLayout || !model.selecting)
            Button("Next song selection") { moveSelection(by: 1) }.keyboardShortcut(.downArrow, modifiers: []).disabled(!wideLayout || !model.selecting)
            Button("Extend selection up") { moveSelection(by: -1, extend: true) }.keyboardShortcut(.upArrow, modifiers: .shift).disabled(!wideLayout || !model.selecting)
            Button("Extend selection down") { moveSelection(by: 1, extend: true) }.keyboardShortcut(.downArrow, modifiers: .shift).disabled(!wideLayout || !model.selecting)
            Button("Cancel song selection") { model.selecting = false; model.selected = [] }.keyboardShortcut(.escape, modifiers: [])
                .disabled(!model.selecting)
            Button("Play selected songs") {
                play(selectionScope.filter { model.selected.contains($0.uniqueId) }); model.selecting = false
            }.keyboardShortcut(.return, modifiers: []).disabled(!model.selecting || model.selected.isEmpty)
            Button("Remove selected songs") {
                if let listID = model.selectionListID {
                    let ids = model.selected.sorted()
                    Task { await model.remove(ids, from: listID) }
                }
            }.keyboardShortcut(.delete, modifiers: []).disabled(model.selectionListID == nil || model.selected.isEmpty)
        }.disabled(findFocused || keyboardCovered).hidden().accessibilityHidden(true)
    }

    private var desktopSelectionBar: some View {
        let selected = tableSongs(selectionScope, compact: false).filter { model.selected.contains($0.uniqueId) }
        return HStack(spacing: 2) {
            Text("\(selected.count) selected").fontWeight(.medium).lwLineBox(13, 18.2)
                .padding(.horizontal, 12).frame(height: 32)
                .foregroundStyle(LWTheme.onInk).background(LWTheme.ink)
            Button { LWTrackMenu.queueNext(selected, player: playerVM) } label: { desktopSelectionKey("Play next") }
            LWMenuButton(request: { LWMenuRequest(groups: addToListGroups(selected, excluding: model.selectionListID)) }) { desktopSelectionKey("Add to list…") }
            Button { Task { await model.togglePins(selected) } } label: { desktopSelectionKey("Keep offline") }
            if let listID = model.selectionListID {
                Button { Task { await model.remove(selected.compactMap(\.id), from: listID) } } label: { desktopSelectionKey("Remove ⌫") }
            }
        }.font(LWTheme.text(13)).foregroundStyle(LWTheme.ink).buttonStyle(.plain)
            .padding(2).background(LWTheme.panel2).fixedSize()
    }

    private func desktopSelectionKey(_ title: String) -> some View {
        Text(title).lwLineBox(13, 18.2).padding(.horizontal, 12).frame(height: 32)
            .background(LWTheme.key).contentShape(Rectangle())
    }

    private var selectionBar: some View {
        let selected = tableSongs(selectionScope, compact: true).filter { model.selected.contains($0.uniqueId) }
        return LWCompactSelectionDeck {
            Button { LWTrackMenu.queueNext(selected, player: playerVM); model.selecting = false; model.selected = [] } label: { mobileSelectionKey("Play next", icon: "next2") }
            LWMenuButton(request: {
                LWMenuRequest(groups: addToListGroups(selected, excluding: model.selectionListID, finishSelection: true), preview: AnyView(Text("Add \(selected.count) song\(selected.count == 1 ? "" : "s") to…").font(LWTheme.text(15))))
            }) { mobileSelectionKey("Add to list", icon: "plus") }
            Button { Task { await model.togglePins(selected); model.selecting = false; model.selected = [] } } label: { mobileSelectionKey("Offline", icon: "dl") }
            if let listID = model.selectionListID {
                Button { Task { await model.remove(selected.compactMap(\.id), from: listID) } } label: { mobileSelectionKey("Remove", icon: "close", destructive: true) }
            } else {
                Button { selected.forEach { playerVM.addToQueue($0) }; model.selecting = false; model.selected = [] } label: { mobileSelectionKey("Queue", icon: "queue") }
            }
        }.disabled(selected.isEmpty)
    }

    private func mobileSelectionKey(_ title: String, icon: String, destructive: Bool = false) -> some View {
        LWCompactSelectionKey(title: title, icon: icon, disabled: model.selected.isEmpty, destructive: destructive)
    }

    private var detailBackLabel: String {
        if path.count > 1 {
            switch path[path.count - 2] {
            case .album(let album): return album.name
            case .artist(let artist): return artist
            case .list(let id): return model.lists.first { $0.id == id }?.name ?? "List"
            case .automatic(let name): return automaticLabel(name)
            }
        }
        if section == "Find" { return "Find" }
        if section == "Lists" || section.hasPrefix("List:") || !isLibrary { return "Lists" }
        return "Library"
    }

    private var detailBack: some View {
        Button { if !path.isEmpty { popPage() } else { NotificationCenter.default.post(name: Notification.Name("LWCommand"), object: "Lists") } } label: {
            HStack(spacing: 4) { LWIcon("back", size: 18); Text(detailBackLabel).font(LWTheme.text(15)).lwLineBox(15, 21) }
                .frame(height: 44).padding(.leading, 10).padding(.trailing, 12)
        }.foregroundStyle(LWTheme.secondary).accessibilityLabel("Back")
    }

    private var desktopBack: some View {
        Button(action: popPage) {
            HStack(spacing: 4) { LWIcon("back", size: 16); Text("Back").font(LWTheme.text(14)).lwLineBox(14, 19.6) }
                .padding(.horizontal, 26).padding(.top, 16).frame(height: 40, alignment: .bottom)
        }.foregroundStyle(LWTheme.secondary)
    }

    private func sectionLabel(_ text: String) -> some View {
        Text(text).font(LWTheme.text(13).weight(.medium)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).padding(.horizontal, 16).padding(.top, 20).padding(.bottom, 8)
    }

    private func playBar(_ songs: [Song], showOffline: Bool = true) -> some View {
        playKeys(songs, showOffline: showOffline).padding(16)
    }

    private func playKeys(_ songs: [Song], showOffline: Bool) -> some View {
        Group {
            if dynamicTypeSize.isAccessibilitySize {
                VStack(spacing: 8) { playKeyButtons(songs, showOffline: showOffline) }
            } else {
                ViewThatFits(in: .horizontal) {
                    HStack(spacing: 8) { playKeyButtons(songs, showOffline: showOffline) }
                    VStack(spacing: 8) { playKeyButtons(songs, showOffline: showOffline) }
                }
            }
        }.font(LWTheme.text(15).weight(.medium)).disabled(songs.isEmpty)
    }

    @ViewBuilder private func playKeyButtons(_ songs: [Song], showOffline: Bool) -> some View {
        Button { play(songs) } label: {
            HStack(spacing: 8) { LWIcon("play", size: 18); Text("Play").lwLineBox(15, 21).fixedSize(horizontal: true, vertical: false) }
                .frame(maxWidth: .infinity).frame(minHeight: 52).foregroundStyle(LWTheme.onInk).background(LWTheme.ink)
        }.accessibilityIdentifier("library-play-context")
        Button { play(songs.shuffled()) } label: {
            HStack(spacing: 8) { LWIcon("shuf", size: 18); Text("Shuffle").lwLineBox(15, 21).fixedSize(horizontal: true, vertical: false) }
                .frame(maxWidth: .infinity).frame(minHeight: 52).background(LWTheme.key2)
        }
        if showOffline {
            let pinned = !songs.isEmpty && songs.allSatisfy { model.pins.contains(String($0.uniqueId)) }
            Button { Task { await model.togglePins(songs) } } label: {
                HStack(spacing: 8) { LWIcon(pinned ? "check" : "dl", size: 18); Text(pinned ? "Kept offline" : "Keep offline").lwLineBox(15, 21).fixedSize(horizontal: true, vertical: false) }
                    .frame(maxWidth: .infinity).frame(minHeight: 52).background(LWTheme.key2)
            }
        }
    }

    private var playbackContext: PlaybackContext {
        if let current = path.last {
            switch current {
            case .album(let album): return .album(name: album.name, artist: album.artist ?? "Unknown Artist")
            case .artist(let artist): return .artist(artist)
            case .list(let id): return .playlist(id: id, name: model.lists.first { $0.id == id }?.name ?? "List")
            case .automatic(let name): return .automatic(name)
            }
        }
        if section == "Find" { return .selection("Find results") }
        return isLibrary ? .library : .automatic(section)
    }

    private func play(_ songs: [Song], startingAt index: Int = 0, context: PlaybackContext? = nil) {
        guard songs.indices.contains(index) else { return }
        playerVM.configureQueue(songs: songs, startIndex: index, context: context ?? playbackContext)
        playerVM.playSong(songs[index])
    }

}

private struct WaveLibrarySongRow: View {
    let song: Song
    let index: Int
    let compact: Bool
    var showArtist = true
    var showAlbum = true
    var showFormat = false
    var showTime = true
    var showSize = false
    @ObservedObject var model: WaveLibraryModel
    var playlistID: Int64?
    var selectionOrder: [Int64] = []
    var highlight = ""
    var numbered = false
    var tableColumns: WaveSongColumns?
    var dropTargeted = false
    var dragInteractionActive = false
    let onPlay: () -> Void
    @EnvironmentObject private var playerVM: PlayerViewModel
    @EnvironmentObject private var presenter: LWSheetPresenter
    @State private var editMetadata = false
    @State private var sharedFile: LWPreparedShare?
    @State private var shareDirectory: URL?
    @State private var sharing = false
    @State private var favoriteBurst = false
    @State private var swipeOffset: CGFloat = 0
    @State private var swiping = false
    @State private var swipeRejected = false
    @State private var modifierKeys: EventModifiers = []
    @State private var hovering = false
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @Environment(\.lwReduceMotion) private var reduceMotion

    var body: some View {
        HStack(spacing: compact ? 12 : 0) {
            Button(action: selectOrPlay) {
                if !compact, let columns = tableColumns { desktopCells(columns) }
                else { HStack(spacing: 12) {
                    if compact && model.selecting {
                        ZStack {
                            Rectangle().fill(model.selected.contains(song.uniqueId) ? LWTheme.ink : .clear)
                            Rectangle().strokeBorder(model.selected.contains(song.uniqueId) ? .clear : LWTheme.secondary, lineWidth: 1.5)
                            Path { path in path.move(to: CGPoint(x: 5 * 14 / 24, y: 12.5 * 14 / 24)); path.addLine(to: CGPoint(x: 9.5 * 14 / 24, y: 17 * 14 / 24)); path.addLine(to: CGPoint(x: 19 * 14 / 24, y: 7.5 * 14 / 24)) }
                                    .stroke(LWTheme.onInk, style: StrokeStyle(lineWidth: 1.4, lineCap: .round, lineJoin: .round))
                                    .frame(width: 14, height: 14)
                                    .opacity(model.selected.contains(song.uniqueId) ? 1 : 0)
                                    .modifier(LWPop(trigger: model.selected.contains(song.uniqueId)))
                        }.frame(width: 20, height: 20).accessibilityHidden(true)
                    }
                    if compact {
                        if numbered {
                            Group { if song.id == playerVM.currentSong?.id && !model.selecting { LWEqualizer(playing: playerVM.isPlaying) } else { Text(String(song.trackNumber ?? index + 1)).font(LWTheme.text(13).monospacedDigit()).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary) } }.frame(width: 22, alignment: .trailing)
                        } else { LWArtwork(song: song, size: 40) }
                    }
                    else {
                        Group {
                            if song.id == playerVM.currentSong?.id { LWEqualizer(playing: playerVM.isPlaying) }
                            else { Text(String(index + 1)).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary) }
                        }.frame(width: 26, alignment: .trailing)
                    }
                    VStack(alignment: .leading, spacing: 0) {
                        waveHighlightedLabel(song.title, query: highlight).foregroundStyle(song.id == playerVM.currentSong?.id && !(compact && model.selecting) ? LWTheme.signal : LWTheme.ink).font(LWTheme.text(compact ? 15 : 14).weight(.regular)).lineLimit(dynamicTypeSize.isAccessibilitySize ? 2 : 1).lwLineBox(compact ? 15 : 14, compact ? 21 : 19.6)
                        if compact { waveHighlightedLabel(blockedSource.map { $0 + " needs reconnecting" } ?? (model.selecting ? song.artist : numbered ? albumFileSubtitle : song.artist + " · " + song.album), query: highlight).font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lineLimit(dynamicTypeSize.isAccessibilitySize ? 2 : 1).lwLineBox(13, 18.2) }
                    }.frame(maxWidth: .infinity, alignment: .leading)
                    if !compact {
                        if showArtist { waveHighlightedLabel(song.artist, query: highlight).font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading) }
                        if showAlbum { waveHighlightedLabel(song.album, query: highlight).font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading) }
                    }
                    if !compact && showFormat { Text(song.fileExtension?.uppercased() ?? "—").font(LWTheme.data(11)).foregroundStyle(LWTheme.secondary).frame(width: 44) }
                    if compact && !model.selecting {
                        HStack(spacing: 8) {
                            if blockedSource != nil || model.pins.contains(String(song.uniqueId)) || model.availabilityByID[song.uniqueId] != .onDevice || song.fileState == .copyPending || song.fileState == .failed { stateGlyph.frame(width: model.pins.contains(String(song.uniqueId)) ? 14 : 10) }
                            if showTime && blockedSource == nil { Text(waveDuration(song.duration)).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary) }
                        }
                    } else if !compact {
                        stateGlyph.frame(width: 62)
                        if showTime && blockedSource == nil { Text(waveDuration(song.duration)).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary).frame(width: 44, alignment: .trailing) }
                    }
                    if !compact && showSize { Text(song.fileSize.map { ByteCountFormatter.string(fromByteCount: $0, countStyle: .file) } ?? "—").font(LWTheme.data(11)).foregroundStyle(LWTheme.secondary).lineLimit(1).frame(width: 64, alignment: .trailing) }
                }.contentShape(Rectangle()) }
            }.buttonStyle(WaveRowButtonStyle()).disabled(compact && (swiping || dragInteractionActive))
                .accessibilityIdentifier("play-song-" + String(song.uniqueId))
                .accessibilityAction(named: "Edit metadata") { editMetadata = true }
                .simultaneousGesture(TapGesture(count: 2).onEnded { if !compact { onPlay() } })
            if compact && model.selecting && blockedSource == nil {
                if playlistID != nil {
                    LWIcon("drag", size: 18).foregroundStyle(LWTheme.secondary).frame(width: 44, height: 44).contentShape(Rectangle())
                        .draggable(LWSongDragPayload.encode([song.uniqueId]))
                        .accessibilityElement(children: .ignore).accessibilityLabel("Reorder \(song.title)")
                        .accessibilityIdentifier("reorder-song-" + String(song.uniqueId))
                }
            } else if compact && blockedSource != nil { Color.clear.frame(width: compact ? 44 : 52, height: compact ? 44 : 38) }
            else { LWMenuButton(request: { menuRequest() }) { LWIcon("more", size: compact ? 18 : 16).frame(width: compact ? 44 : 32, height: compact ? 44 : 32).padding(.trailing, compact ? 0 : 20).foregroundStyle(LWTheme.secondary) }.accessibilityLabel("Actions for \(song.title)") }
        }
        .padding(.leading, compact ? 16 : 0).padding(.trailing, compact ? 6 : 0).frame(minHeight: compact ? 56 : 38)
        .offset(x: swipeOffset)
        .background {
            HStack {
                HStack(spacing: 8) { LWIcon("next2", size: 16); Text("Play next").lwLineBox(14, 19.6) }
                    .padding(.horizontal, 12).frame(maxHeight: .infinity)
                    .foregroundStyle(swiping && swipeOffset > 88 ? LWTheme.onInk : LWTheme.secondary)
                    .background(swiping && swipeOffset > 88 ? LWTheme.ink : .clear)
                Spacer()
                HStack(spacing: 8) { Text(playlistID == nil ? "Add to queue" : "Remove").lwLineBox(14, 19.6); LWIcon(playlistID == nil ? "queue" : "close", size: 16) }
                    .padding(.horizontal, 12).frame(maxHeight: .infinity)
                    .foregroundStyle(swiping && swipeOffset < -88 ? LWTheme.onInk : LWTheme.secondary)
                    .background(swiping && swipeOffset < -88 ? LWTheme.ink : .clear)
            }.font(LWTheme.text(14)).padding(.horizontal, 16).opacity(swiping ? 1 : 0)
        }
        .background {
            if !compact && dropTargeted { LWTheme.paper.overlay(LWTheme.signal.opacity(0.08)) }
            else { (!compact && model.selected.contains(song.uniqueId) ? LWTheme.panel2 : hovering && !compact ? LWTheme.panel.opacity(0.6) : LWTheme.paper) }
        }
        .accessibilityValue(compact && model.selecting ? (model.selected.contains(song.uniqueId) ? "Selected" : "Not selected") : "")
        .opacity(compact && blockedSource != nil ? 0.45 : 1)
        .disabled(compact && blockedSource != nil)
        .onHover { hovering = $0 }
        .gesture(WaveModifierCapture { modifierKeys = $0 })
        .overlay(alignment: .top) { if compact && index > 0 { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.leading, 16) } }
        .modifier(LWCollapsedTableRules(top: !compact, bottom: !compact && selectionOrder.last == song.uniqueId))
        .overlay { if !compact && dropTargeted { Rectangle().strokeBorder(LWTheme.signal, lineWidth: 2) } }
        .modifier(WaveConditionalClip(enabled: compact))
        .simultaneousGesture(DragGesture(minimumDistance: 8).onChanged { gesture in
            guard blockedSource == nil && compact && !model.selecting && !swipeRejected else { return }
            if !swiping {
                guard abs(gesture.translation.width) > abs(gesture.translation.height) else { swipeRejected = true; return }
                withAnimation(reduceMotion ? nil : LWMotion.out(LWMotion.quick)) { swiping = true }
            }
            var transaction = Transaction(); transaction.animation = nil
            withTransaction(transaction) { swipeOffset = gesture.translation.width }
        }.onEnded { _ in
            defer { swipeRejected = false }
            guard swiping else { return }
            if !model.selecting {
                if swipeOffset > 88 { playNext() }
                else if swipeOffset < -88 {
                    if let playlistID, let id = song.id { Task { await model.remove(id, from: playlistID) } }
                    else { playerVM.addToQueue(song) }
                }
            }
            withAnimation(reduceMotion ? nil : LWMotion.out(LWMotion.quick)) { swiping = false }
            withAnimation(reduceMotion ? nil : LWMotion.springAnimation(LWMotion.slow)) { swipeOffset = 0 }
        }, including: compact && !model.selecting ? .all : .none)
        .lwContextMenu(enabled: !compact || !model.selecting) { menuRequest() }
        .modifier(WaveSongDrag(enabled: !compact || !model.selecting, title: song.title, ids: dragSongIDs))
        .overlay(alignment: .trailing) { if sharing { ProgressView().frame(width: 44, height: 44).background(LWTheme.paper) } }
        .sheet(item: $sharedFile, onDismiss: {
            if let shareDirectory { try? FileManager.default.removeItem(at: shareDirectory) }
            shareDirectory = nil
        }) { item in LWShareSheet(urls: item.urls) }
        .sheet(isPresented: $editMetadata) { SongMetadataEditorView(song: song, songRepo: model.dependencies.songRepository) }
        .overlay { if favoriteBurst { WaveFavoriteBurst().allowsHitTesting(false) } }
    }

    private func desktopCells(_ columns: WaveSongColumns) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 0) {
            ForEach(columns.items.filter { $0 != .actions }, id: \.self) { column in
                Group {
                    switch column {
                    case .number:
                        if song.id == playerVM.currentSong?.id { LWEqualizer(playing: playerVM.isPlaying) }
                        else { Text(String(numbered ? song.trackNumber ?? index + 1 : index + 1)).font(LWTheme.text(13).monospacedDigit()).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary) }
                    case .title: waveHighlightedLabel(song.title, query: highlight).font(LWTheme.text(14).weight(song.id == playerVM.currentSong?.id ? .medium : .regular)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.ink).lineLimit(1)
                    case .artist: waveHighlightedLabel(song.artist, query: highlight).font(LWTheme.text(14).weight(song.id == playerVM.currentSong?.id ? .medium : .regular)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).lineLimit(1)
                    case .album: waveHighlightedLabel(song.album, query: highlight).font(LWTheme.text(14).weight(song.id == playerVM.currentSong?.id ? .medium : .regular)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).lineLimit(1)
                    case .format: Text(song.fileExtension?.lowercased() ?? "—").font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary)
                    case .state: stateGlyph
                    case .time: Text(waveDuration(song.duration)).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary)
                    case .actions: EmptyView()
                    }
                }.padding(.leading, column == .number ? 32 : 8).padding(.trailing, 8)
                    .frame(width: columns[column], alignment: column == .time ? .trailing : .leading)
            }
        }.contentShape(Rectangle())
    }

    private var blockedSource: String? { waveBlockedSourceName(song: song, availability: model.availabilityByID[song.uniqueId] ?? .unknown, blockedSources: model.blockedSources) }

    private var albumFileSubtitle: String {
        let size = song.fileSize.map { String(format: "%.1f MB", Double($0) / 1_048_576) } ?? "Size unknown"
        return size + (model.availabilityByID[song.uniqueId] == .inCloud ? " · in cloud" : "")
    }

    private func selectOrPlay() {
        guard !compact || blockedSource == nil else { return }
        if compact && !model.selecting { onPlay(); return }
        model.selectionListID = playlistID
        if modifierKeys.contains(.shift), let anchor = model.selectionAnchor,
           let start = selectionOrder.firstIndex(of: anchor), let end = selectionOrder.firstIndex(of: song.uniqueId) {
            model.selected.formUnion(selectionOrder[min(start, end)...max(start, end)])
        } else if model.selecting && compact || modifierKeys.contains(.command) {
            if model.selected.contains(song.uniqueId) { model.selected.remove(song.uniqueId) } else { model.selected.insert(song.uniqueId) }
            model.selectionAnchor = song.uniqueId
        } else {
            model.selected = [song.uniqueId]
            model.selectionAnchor = song.uniqueId
        }
        model.selecting = true
    }

    private func playNext() { LWTrackMenu.queueNext([song], player: playerVM) }

    @ViewBuilder private var stateGlyph: some View {
        let pinned = model.pins.contains(String(song.uniqueId))
        let state = model.availabilityByID[song.uniqueId] ?? .unknown
        let label = song.fileState == .copyPending ? "Preparing offline copy" : song.fileState == .failed ? "File unavailable — retry" : state == .onDevice ? (pinned ? "Kept offline" : "On this device") : state == .inCloud ? "In cloud — downloads when played" : model.availabilityByID[song.uniqueId] == nil ? "Checking availability" : "Availability unknown"
        LWStateGlyph(state: song.fileState, availability: state, pinned: pinned, blocked: blockedSource != nil).accessibilityLabel(blockedSource.map { $0 + " needs reconnecting" } ?? label)

    }

    private func share(_ targets: [Song]) {
        guard !sharing, !targets.isEmpty else { return }
        sharing = true
        Task {
            defer { sharing = false }
            do {
                let prepared = try await LWSourceFileSharing.prepare(songs: targets, dependencies: model.dependencies)
                shareDirectory = prepared.directory
                sharedFile = prepared
            } catch { model.error = error.localizedDescription }
        }
    }

    private func showFolder(_ target: Song) {
        guard let sourceID = target.sourceId else { return }
        Task {
            do {
                guard let source = try await model.dependencies.sourceService.repository().getOne(id: sourceID) else { return }
                let paths = try await model.dependencies.sourceService.importService().listItems(sourceId: sourceID, parentPathId: nil)
                let parentID = paths.first { $0.relativePath == target.relativePath }?.parentPathId ?? source.pathId
                let name = paths.first { $0.isDirectory && $0.pathId == parentID }?.name ?? URL(fileURLWithPath: source.dirPath).lastPathComponent
                NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: PlaybackContext.folder(sourceID: sourceID, pathID: parentID, name: name))
            } catch { model.error = error.localizedDescription }
        }
    }

    private func dragSongIDs() -> [Int64] {
        guard !compact else { return [song.uniqueId] }
        model.selectionListID = playlistID
        if !model.selected.contains(song.uniqueId) {
            model.selected = [song.uniqueId]
            model.selectionAnchor = song.uniqueId
        }
        model.selecting = true
        let ordered = selectionOrder.filter { model.selected.contains($0) }
        return ordered.isEmpty ? [song.uniqueId] : ordered
    }

    private func menuRequest() -> LWMenuRequest {
        if !compact { model.selectionListID = playlistID }
        if !compact && !model.selected.contains(song.uniqueId) {
            model.selected = [song.uniqueId]
            model.selectionAnchor = song.uniqueId
            model.selecting = true
        }
        let byID = Dictionary(model.songs.map { ($0.uniqueId, $0) }, uniquingKeysWith: { first, _ in first })
        let selected = !compact && model.selected.count > 1
            ? selectionOrder.filter { model.selected.contains($0) }.compactMap { byID[$0] } : [song]
        let targets = selected.isEmpty ? [song] : selected
        return LWTrackMenu.request(song: song, targets: targets, model: model, player: playerVM, presenter: presenter,
            playlistID: playlistID, sharing: sharing, onPlay: onPlay, onFolder: showFolder, onShare: share,
            onFavoriteFeedback: {
                guard !reduceMotion else { return }
                withAnimation(LWMotion.springAnimation(LWMotion.slow)) { favoriteBurst = true }
                Task { try? await Task.sleep(for: .milliseconds(420)); favoriteBurst = false }
            })
    }

}

/// Cancelling a recognized swipe's button tracking must not dim the row's content.
private struct WaveRowButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View { configuration.label }
}

private struct WaveSongDrag: ViewModifier {
    let enabled: Bool
    let title: String
    let ids: () -> [Int64]
    @ViewBuilder func body(content: Content) -> some View {
        if enabled { content.modifier(LWOrderedSongDrag(title: title, ids: ids)) } else { content }
    }
}

struct WaveModifierCapture: UIGestureRecognizerRepresentable {
    var changed: (EventModifiers) -> Void

    func makeCoordinator(converter: CoordinateSpaceConverter) -> Coordinator { Coordinator(changed: changed) }
    func makeUIGestureRecognizer(context: Context) -> UITapGestureRecognizer {
        let recognizer = UITapGestureRecognizer()
        recognizer.cancelsTouchesInView = false
        recognizer.delegate = context.coordinator
        return recognizer
    }
    func updateUIGestureRecognizer(_ recognizer: UITapGestureRecognizer, context: Context) { context.coordinator.changed = changed }

    final class Coordinator: NSObject, UIGestureRecognizerDelegate {
        var changed: (EventModifiers) -> Void
        init(changed: @escaping (EventModifiers) -> Void) { self.changed = changed }
        func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldReceive event: UIEvent) -> Bool {
            var modifiers: EventModifiers = []
            if event.modifierFlags.contains(.command) { modifiers.insert(.command) }
            if event.modifierFlags.contains(.shift) { modifiers.insert(.shift) }
            changed(modifiers)
            return true
        }
        func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith otherGestureRecognizer: UIGestureRecognizer) -> Bool { true }
    }
}

private struct WaveListDetail: View {
    @ObservedObject var model: WaveLibraryModel
    let playlistID: Int64
    var onBack: (() -> Void)? = nil
    @EnvironmentObject private var playerVM: PlayerViewModel
    @EnvironmentObject private var presenter: LWSheetPresenter
    @Environment(\.dismiss) private var dismiss
    @State private var addingSongs = false
    @State private var tableSort: String?
    @State private var tableAscending = true
    @ScaledMetric(relativeTo: .body) private var tableTextScale: CGFloat = 1
    @Environment(\.lwWideLayout) private var wideLayout
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @State private var selection = Set<Int64>()

    @State private var targetedSongID: Int64?

    private var playlist: Playlist? { model.lists.first { $0.id == playlistID } }
    private var songs: [Song] { model.listSongs[playlistID] ?? [] }
    private var displayedSongs: [Song] { waveSortedSongs(songs, column: wideLayout ? tableSort : nil, ascending: tableAscending, availability: model.availabilityByID) }

    var body: some View {
        GeometryReader { geometry in
        let formats = wideLayout && Set(songs.compactMap(\.fileExtension)).count > 1
        let pins = model.pins
        let columns: WaveSongColumns? = wideLayout ? WaveSongColumns.make(songs: displayedSongs, available: geometry.size.width, scale: tableTextScale, artist: geometry.size.width >= 400, album: geometry.size.width >= 520, format: formats && geometry.size.width >= 680, state: Set(songs.map { model.stateKey($0, pins: pins) }).count > 1, numbered: false, currentID: playerVM.currentSong?.id, sort: tableSort) : nil
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                listHeader
                LazyVStack(spacing: 0, pinnedViews: [.sectionHeaders]) {
                    Section {
                        if songs.isEmpty && !wideLayout {
                            Text("Empty for now.").font(LWTheme.text(14)).lwLineBox(14, 19.6)
                                .foregroundStyle(LWTheme.secondary).frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.vertical, 40).padding(.horizontal, 16)
                        }
                ForEach(Array(displayedSongs.enumerated()), id: \.element.uniqueId) { index, song in
                    WaveLibrarySongRow(song: song, index: index, compact: !wideLayout, showArtist: geometry.size.width >= 400, showAlbum: geometry.size.width >= 520, showFormat: geometry.size.width >= 680 && Set(songs.compactMap(\.fileExtension)).count > 1, showTime: true, model: model, playlistID: playlistID, selectionOrder: displayedSongs.map(\.uniqueId), tableColumns: columns, dropTargeted: wideLayout && targetedSongID == song.uniqueId, dragInteractionActive: targetedSongID != nil) { play(displayedSongs, at: index) }
                        .modifier(WaveLibraryEntrance(index: index))
                        .anchorPreference(key: WaveSelectionTableEnd.self, value: .bounds) { wideLayout && index == displayedSongs.count - 1 ? $0 : nil }
                        .dropDestination(for: String.self) { values, _ in
                            guard tableSort == nil else { return false }
                            let ids = LWSongDragPayload.decode(values)
                            let reordered = wideLayout
                                ? wavePlaylistSongsMovingGroup(songs, draggedIDs: ids, before: song.uniqueId)
                                : ids.first.flatMap { wavePlaylistSongsMoving(songs, draggedID: $0, targetID: song.uniqueId) }
                            guard let reordered else { return false }
                            persistOrder(reordered)
                            return true
                        } isTargeted: { targeted in
                            if targeted {
                                guard tableSort == nil else { return }
                                targetedSongID = song.uniqueId
                            } else if targetedSongID == song.uniqueId { targetedSongID = nil }
                        }
                        .accessibilityAction(named: "Move up") { moveSong(song.uniqueId, by: -1) }
                        .accessibilityAction(named: "Move down") { moveSong(song.uniqueId, by: 1) }
                }
                        if wideLayout && model.selecting && model.selected.count > 1 { Color.clear.frame(height: 48) }
                    } header: {
                        if wideLayout, let columns { WaveSongColumnsHeading(columns: columns, sort: $tableSort, ascending: $tableAscending) }
                    }
                }
            }
        }
        .background(LWTheme.paper).toolbar(.hidden, for: .navigationBar)
        }
        .toolbar(.hidden, for: .navigationBar)
        .sheet(isPresented: $addingSongs) {
            NavigationStack {
                List(model.songs, id: \.uniqueId) { song in
                    Button {
                        if selection.contains(song.uniqueId) { selection.remove(song.uniqueId) } else { selection.insert(song.uniqueId) }
                    } label: {
                        HStack {
                            Image(systemName: selection.contains(song.uniqueId) ? "checkmark.square.fill" : "square")
                            VStack(alignment: .leading) { Text(song.title); Text(song.artist).font(.caption).foregroundStyle(.secondary) }
                        }.foregroundStyle(LWTheme.ink)
                    }
                }
                .navigationTitle("Add Songs")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button("Cancel") { addingSongs = false; selection = [] } }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Add") {
                            let selected = model.songs.filter { selection.contains($0.uniqueId) }.compactMap(\.id)
                            Task { await model.perform { try await LWTrackMenu.addToList(songIDs: selected, listID: playlistID, listName: playlist?.name ?? "List", dependencies: model.dependencies) }; addingSongs = false; selection = [] }
                        }.disabled(selection.isEmpty)
                    }
                }
            }
        }
    }

    private func persistOrder(_ reordered: [Song]) {
        withAnimation(LWMotion.out()) { model.listSongs[playlistID] = reordered }
        Task { await model.perform { try await model.dependencies.playlistSongRepo.reorderSongs(playlistId: playlistID, newOrder: reordered.compactMap(\.id)) } }
    }

    private func moveSong(_ id: Int64, by delta: Int) {
        guard tableSort == nil, let index = songs.firstIndex(where: { $0.uniqueId == id }),
              songs.indices.contains(index + delta),
              let reordered = wavePlaylistSongsMoving(songs, draggedID: id, targetID: songs[index + delta].uniqueId) else { return }
        persistOrder(reordered)
    }

    @ViewBuilder private var listHeader: some View {
        if wideLayout {
            VStack(alignment: .leading, spacing: 0) {
                Text("List").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).padding(.top, 6)
                Text(playlist?.name ?? "List").font(LWTheme.text(34).weight(.medium)).tracking(-0.85).lwLineBox(34, 34)
                Text(listMinutes).font(LWTheme.text(14)).lwLineBox(14, 19.6).foregroundStyle(LWTheme.secondary).padding(.top, 6)
            }.padding(.horizontal, 32).padding(.top, 28).padding(.bottom, 18)
                .lwContextMenu { LWMenuRequest(groups: listMenuGroups) }
            HStack(spacing: 8) {
                Button { play(songs) } label: { HStack(spacing: 6) { LWIcon("play", size: 16); Text("Play").lwLineBox(13, 18.2) }.padding(.horizontal, 10).frame(height: 30) }
                Button { play(songs.shuffled()) } label: { HStack(spacing: 6) { LWIcon("shuf", size: 16); Text("Shuffle").lwLineBox(13, 18.2) }.padding(.horizontal, 10).frame(height: 30) }
                Spacer(minLength: 0)
            }.font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).padding(.horizontal, 32).padding(.bottom, 14).padding(.bottom, 1)
                .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1) }
        } else {
            if model.selecting { WaveMobileSelectionHeader(model: model, songs: songs, title: playlist?.name ?? "List") }
            else {
            HStack(spacing: 0) {
                Button { if let onBack { onBack() } else { dismiss() } } label: { HStack(spacing: 4) { LWIcon("back", size: 18); Text("Lists").lwLineBox(15, 21) }.frame(height: 44).padding(.leading, 10).padding(.trailing, 12) }.foregroundStyle(LWTheme.secondary).accessibilityLabel("Back")
                Spacer(minLength: 0)
                HStack(spacing: 4) {
                    if !songs.isEmpty {
                        Button("Edit") { tableSort = nil; model.selecting = true; model.selected = []; model.selectionListID = playlistID }
                            .accessibilityAction(named: "Add songs") { addingSongs = true }
                            .accessibilityAction(named: "Select songs") { model.selecting = true; model.selected = []; model.selectionListID = playlistID }
                            .accessibilityAction(named: "Reorder songs") { tableSort = nil; model.selecting = true; model.selected = []; model.selectionListID = playlistID }
                            .font(LWTheme.text(16)).lwLineBox(16, 22.4).padding(.horizontal, 4).frame(height: 44)
                    }
                    LWMenuButton(request: { LWMenuRequest(groups: listMenuGroups) }) { LWIcon("more", size: 22).frame(width: 44, height: 44) }.buttonStyle(LWIconButtonStyle()).accessibilityLabel("List actions")
                        .accessibilityAction(named: "Add songs") { addingSongs = true }
                }.padding(.trailing, 16)
            }.font(LWTheme.text(15)).padding(.top, 10)
            Text(playlist?.name ?? "List").font(LWTheme.text(32).weight(.medium)).tracking(-1.12).lwLineBox(32, 33.6).padding(.horizontal, 16)
            Text("\(songs.count) songs").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).padding(.horizontal, 16).padding(.top, 4)
            }
            (dynamicTypeSize.isAccessibilitySize ? AnyLayout(VStackLayout(spacing: 8)) : AnyLayout(HStackLayout(spacing: 8))) {
                Button { play(songs) } label: { HStack(spacing: 8) { LWIcon("play", size: 18); Text("Play").lwLineBox(15, 21) }.frame(maxWidth: .infinity).frame(minHeight: 52).foregroundStyle(LWTheme.onInk).background(LWTheme.ink) }
                    .accessibilityIdentifier("library-play-context")
                Button { play(songs.shuffled()) } label: { HStack(spacing: 8) { LWIcon("shuf", size: 18); Text("Shuffle").lwLineBox(15, 21) }.frame(maxWidth: .infinity).frame(minHeight: 52).background(LWTheme.key2) }
            }.font(LWTheme.text(15).weight(.medium)).padding(16)
        }
    }

    private var listMinutes: String { "\(Int((songs.compactMap(\.duration).reduce(0, +) / 60).rounded())) min" }

    private var listMenuGroups: [LWMenuGroup] {
        [
            LWMenuGroup([
                LWMenuAction(title: "Play", icon: "play", disabled: songs.isEmpty) { play(songs) },
                LWMenuAction(title: "Shuffle", icon: "shuf", disabled: songs.isEmpty) { play(songs.shuffled()) },
                LWMenuAction(title: "Add to queue", icon: "queue", disabled: songs.isEmpty, action: { songs.forEach { playerVM.addToQueue($0) }; LWPresentationNotice.show("\(songs.count) songs added to queue") })
            ]),
            LWMenuGroup([
                LWMenuAction(title: "Rename…", icon: "set") { renameList() },
                LWMenuAction(title: "Duplicate", icon: "plus") {
                    guard let playlist else { return }
                    Task {
                        if let copy = await model.createList(name: playlist.name + " copy", songs: songs) {
                            LWPresentationNotice.show("Duplicated as “\(copy.name)”")
                        }
                    }
                },
                LWMenuAction(title: !songs.isEmpty && songs.allSatisfy { model.pins.contains(String($0.uniqueId)) } ? "Stop keeping offline" : "Keep offline", icon: "dl", disabled: songs.isEmpty) { Task { await model.togglePins(songs) } }
            ]),
            LWMenuGroup([LWMenuAction(title: "Delete list", icon: "close", destructive: true) { deleteList() }])
        ]
    }

    private func renameList() {
        guard let playlist else { return }
        presenter.present(dialog: LWDialogRequest(title: "Rename List", input: LWDialogInput(value: playlist.name, placeholder: "List name"), primaryTitle: "Save", onSubmit: { value in
            let name = value.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !name.isEmpty else { return }
            Task { await model.perform { _ = try await model.dependencies.playlistRepo.update(playlist: Playlist(id: playlistID, name: name, createdAt: playlist.createdAt, updatedAt: Date())) } }
        }))
    }

    private func deleteList() {
        guard let playlist else { return }
        presenter.present(dialog: LWDialogRequest(title: "Delete “\(playlist.name)”?", message: "The songs stay in your library and folders.", primaryTitle: "Delete List", destructive: true, mobileConfirmation: true, onSubmit: { _ in
            Task {
                do {
                    try await model.dependencies.playlistRepo.delete(playlistId: playlistID)
                    NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
                    LWPresentationNotice.show("Deleted “\(playlist.name)”")
                    if let onBack { onBack() } else { dismiss() }
                } catch { model.error = error.localizedDescription }
                await model.reload()
            }
        }))
    }

    private func heading(_ label: String) -> some View {
        WaveSongTableHeading(label: label, column: $tableSort, ascending: $tableAscending)
    }

    private func play(_ songs: [Song], at index: Int = 0) {
        guard songs.indices.contains(index) else { return }
        playerVM.configureQueue(songs: songs, startIndex: index, context: .playlist(id: playlistID, name: playlist?.name ?? "List"))
        playerVM.playSong(songs[index])
    }
}

func waveHighlightedText(_ text: String, query: String) -> AttributedString {
    var result = AttributedString(text)
    let words = WaveSongQuery(query).text.split(whereSeparator: \.isWhitespace).map { NSRegularExpression.escapedPattern(for: String($0)) }
    guard !words.isEmpty, let regex = try? NSRegularExpression(pattern: words.joined(separator: "|"), options: .caseInsensitive) else { return result }
    for match in regex.matches(in: text, range: NSRange(text.startIndex..<text.endIndex, in: text)) {
        guard let range = Range(match.range, in: text),
              let lower = AttributedString.Index(range.lowerBound, within: result),
              let upper = AttributedString.Index(range.upperBound, within: result) else { continue }
        result[lower..<upper].backgroundColor = LWTheme.signal.opacity(0.16)
    }
    return result
}

func waveHighlightedLabel(_ text: String, query: String) -> Text {
    let attributed = waveHighlightedText(text, query: query)
    var result = Text("")
    for run in attributed.runs {
        var segment = AttributedString(attributed[run.range])
        let label: Text
        if run.backgroundColor != nil {
            segment.backgroundColor = nil
            label = Text(segment).customAttribute(LWSearchMark(color: LWTheme.signal))
        } else { label = Text(segment) }
        result = Text("\(result)\(label)")
    }
    return result
}

func waveDuration(_ duration: Double?) -> String {
    guard let duration, duration.isFinite, duration >= 0, duration < Double(Int.max) else { return "—" }
    return String(format: "%d:%02d", Int(duration) / 60, Int(duration) % 60)
}

/// Repository search determines membership; a result page keeps the library's
/// genuine insertion order, independently of FTS relevance or duplicate matches.
func waveFindSongsInOriginalOrder(_ songs: [Song], matchingIDs: [Int64]?, matchingQuery: String? = nil, currentQuery: String? = nil) -> [Song] {
    if let currentQuery, matchingQuery != currentQuery { return songs }
    guard let matchingIDs else { return songs }
    let matching = Set(matchingIDs)
    return songs.filter { matching.contains($0.uniqueId) }
}

func wavePlaylistSongsMoving(_ songs: [Song], draggedID: Int64, targetID: Int64) -> [Song]? {
    guard let from = songs.firstIndex(where: { $0.uniqueId == draggedID }),
          let target = songs.firstIndex(where: { $0.uniqueId == targetID }), from != target else { return nil }
    var reordered = songs
    reordered.move(fromOffsets: IndexSet(integer: from), toOffset: target > from ? target + 1 : target)
    return reordered
}

/// Desktop drops insert the displayed selection before the target after removing that group.
func wavePlaylistSongsMovingGroup(_ songs: [Song], draggedIDs: [Int64], before targetID: Int64) -> [Song]? {
    let byID = Dictionary(songs.map { ($0.uniqueId, $0) }, uniquingKeysWith: { first, _ in first })
    var seen = Set<Int64>()
    let moving = draggedIDs.compactMap { id in seen.insert(id).inserted ? byID[id] : nil }
    guard !moving.isEmpty else { return nil }
    let movingIDs = Set(moving.map(\.uniqueId))
    var remaining = songs.filter { !movingIDs.contains($0.uniqueId) }
    let destination = remaining.firstIndex { $0.uniqueId == targetID } ?? remaining.count
    remaining.insert(contentsOf: moving, at: destination)
    return remaining.map(\.uniqueId) == songs.map(\.uniqueId) ? nil : remaining
}

func waveSortedSongs(_ songs: [Song], column: String?, ascending: Bool, availability: [Int64: FileAvailability] = [:]) -> [Song] {
    guard let column else { return songs }
    func number(_ song: Song) -> Double? {
        switch column {
        case "Time": return song.duration.flatMap { $0.isFinite && $0 >= 0 ? $0 : nil }
        case "Size": return song.fileSize.map(Double.init)
        default:
            guard let state = availability[song.uniqueId], state != .unknown else { return nil }
            return state == .onDevice ? 1 : 0
        }
    }
    return songs.sorted { left, right in
        let comparison: ComparisonResult
        if ["Time", "Size", "On device"].contains(column) {
            let lhs = number(left), rhs = number(right)
            if lhs == nil || rhs == nil {
                if lhs != nil { return true }
                if rhs != nil { return false }
                return left.uniqueId < right.uniqueId
            }
            comparison = lhs == rhs ? .orderedSame : lhs! < rhs! ? .orderedAscending : .orderedDescending
        } else {
            let lhs = column == "Artist" ? left.artist : column == "Album" ? left.album : column == "Format" ? left.fileExtension ?? "" : left.title
            let rhs = column == "Artist" ? right.artist : column == "Album" ? right.album : column == "Format" ? right.fileExtension ?? "" : right.title
            comparison = lhs.localizedStandardCompare(rhs)
        }
        if comparison == .orderedSame { return left.uniqueId < right.uniqueId }
        return comparison == (ascending ? .orderedAscending : .orderedDescending)
    }
}

private struct WaveSongTableHeading: View {
    let label: String
    @Binding var column: String?
    @Binding var ascending: Bool

    var body: some View {
        Button {
            if label == "#" { column = nil }
            else if column == label { ascending.toggle() }
            else { column = label; ascending = true }
        } label: {
            Text(label + (column == label ? (ascending ? " ↑" : " ↓") : ""))
                .frame(maxWidth: .infinity, alignment: label == "Time" ? .trailing : .leading).contentShape(Rectangle())
        }.buttonStyle(.plain).font(LWTheme.text(12).weight(.medium)).lwLineBox(12, 16.8).foregroundStyle(LWTheme.secondary)
            .accessibilityLabel("Sort by " + (label == "#" ? "original order" : label))
            .accessibilityValue(column == label ? (ascending ? "Ascending" : "Descending") : "Unsorted")
    }
}

struct LWStateGlyph: View {
    let state: FileState
    let availability: FileAvailability
    let pinned: Bool
    var blocked = false
    @Environment(\.lwReduceMotion) private var reduceMotion

    var body: some View {
        let motionReduced = reduceMotion
        TimelineView(.animation(minimumInterval: 0.5, paused: state != .copyPending || motionReduced)) { timeline in
            Canvas { context, canvasSize in
                // CSS box shadows extend beyond the ten-point glyph. Give the
                // drawing a real overflow surface while keeping its layout ten points.
                let size = CGSize(width: 10, height: 10)
                if pinned { context.translateBy(x: 3.5, y: 3.5) }
                let box = CGRect(x: 0.75, y: 0.75, width: size.width - 1.5, height: size.height - 1.5)
                let ink = state == .failed || state == .copyPending ? LWTheme.signal : LWTheme.ink
                if blocked {
                    context.stroke(Path(box), with: .color(LWTheme.tertiary), lineWidth: 1.5)
                    context.fill(Path(CGRect(x: 0, y: size.height - 2, width: size.width, height: 2)), with: .color(LWTheme.tertiary))
                } else if availability == .onDevice && state != .copyPending && state != .failed {
                    let full = CGRect(origin: .zero, size: size)
                    if pinned {
                        context.fill(Path(full.insetBy(dx: -3.5, dy: -3.5)), with: .color(ink))
                        context.fill(Path(full.insetBy(dx: -2, dy: -2)), with: .color(LWTheme.paper))
                    }
                    context.fill(Path(full), with: .color(ink))
                } else {
                    if availability == .unknown && state != .copyPending && state != .failed {
                        context.stroke(Path(box), with: .color(LWTheme.tertiary), style: StrokeStyle(lineWidth: 1, dash: [2, 2]))
                    } else {
                        context.stroke(Path(box), with: .color(state == .bookmarkOnly ? LWTheme.secondary : ink), lineWidth: 1.5)
                    }
                    if state == .copyPending {
                        let left = reduceMotion || Int(timeline.date.timeIntervalSinceReferenceDate * 2).isMultiple(of: 2)
                        let half = CGRect(x: left ? 0 : size.width / 2, y: 0, width: size.width / 2, height: size.height)
                        context.fill(Path(half), with: .color(ink))
                    } else if state == .failed {
                        var slash = Path(); slash.move(to: CGPoint(x: 1, y: size.height - 1)); slash.addLine(to: CGPoint(x: size.width - 1, y: 1))
                        context.stroke(slash, with: .color(ink), lineWidth: 1.5)
                    }
                }
            }.frame(width: pinned ? 17 : 10, height: pinned ? 17 : 10)
                .frame(width: 10, height: 10)
        }
        .keyframeAnimator(initialValue: CGFloat(1), trigger: state) { content, scale in
            content.scaleEffect(motionReduced ? 1 : scale)
        } keyframes: { _ in
            LinearKeyframe(0.3, duration: 0)
            LinearKeyframe(0.306300, duration: LWMotion.base * 0.010500)
            LinearKeyframe(0.324500, duration: LWMotion.base * 0.010500)
            LinearKeyframe(0.398700, duration: LWMotion.base * 0.023000)
            LinearKeyframe(0.806100, duration: LWMotion.base * 0.085000)
            LinearKeyframe(0.956600, duration: LWMotion.base * 0.038000)
            LinearKeyframe(1.011900, duration: LWMotion.base * 0.038000)
            LinearKeyframe(1.030100, duration: LWMotion.base * 0.040000)
            LinearKeyframe(1.024500, duration: LWMotion.base * 0.039000)
            LinearKeyframe(1.007000, duration: LWMotion.base * 0.093000)
            LinearKeyframe(0.997900, duration: LWMotion.base * 0.095000)
            LinearKeyframe(1.000000, duration: LWMotion.base * 0.528000)
        }
    }
}

struct LWEqualizer: View {
    let playing: Bool
    @State private var beganAt = Date()
    @Environment(\.lwReduceMotion) private var reduceMotion

    private func easeInOut(_ target: Double) -> Double {
        var low = 0.0, high = 1.0
        for _ in 0..<20 {
            let t = (low + high) / 2
            let x = 3 * (1 - t) * (1 - t) * t * 0.42 + 3 * (1 - t) * t * t * 0.58 + t * t * t
            if x < target { low = t } else { high = t }
        }
        let t = (low + high) / 2
        return 3 * (1 - t) * t * t + t * t * t
    }

    var body: some View {
        TimelineView(.animation(paused: !playing || reduceMotion)) { timeline in
            Canvas { context, size in
                for index in 0..<3 {
                    let phase: Double = (max(0, timeline.date.timeIntervalSince(beganAt)) / 0.9 + Double(index) / 3).truncatingRemainder(dividingBy: 1)
                    let eased: Double = easeInOut(phase < 0.5 ? phase * 2 : (1 - phase) * 2)
                    let fraction: CGFloat = playing && !reduceMotion ? CGFloat(0.4 + 0.6 * eased) : 0.4
                    let height: CGFloat = size.height * fraction
                    let originX: CGFloat = CGFloat(index * 4)
                    let rect = CGRect(x: originX, y: size.height - height, width: 2, height: height)
                    context.fill(Path(rect), with: .color(LWTheme.signal))
                }
            }.frame(width: 14, height: 11)
        }.onAppear { if playing { beganAt = Date() } }
            .onChange(of: playing) { _, value in if value { beganAt = Date() } }
            .accessibilityLabel(playing ? "Playing" : "Paused")
    }
}

private struct WaveFavoriteBurst: View {
    @State private var expanded = false
    private let directions: [CGSize] = [CGSize(width: 0, height: -1), CGSize(width: 0.9, height: -0.5), CGSize(width: 0.9, height: 0.5), CGSize(width: 0, height: 1), CGSize(width: -0.9, height: 0.5), CGSize(width: -0.9, height: -0.5)]

    var body: some View {
        ZStack {
            Image(systemName: "heart.fill").foregroundStyle(LWTheme.signal).scaleEffect(expanded ? 1 : 0.7)
            ForEach(directions.indices, id: \.self) { index in
                Rectangle().fill(LWTheme.signal).frame(width: 4, height: 4)
                    .offset(x: expanded ? directions[index].width * 18 : 0, y: expanded ? directions[index].height * 18 : 0)
                    .scaleEffect(expanded ? 0.25 : 1).opacity(expanded ? 0 : 1)
            }
        }.onAppear { withAnimation(LWMotion.out()) { expanded = true } }
    }
}

private struct WaveSelectionTableEnd: PreferenceKey {
    static var defaultValue: Anchor<CGRect>? { nil }
    static func reduce(value: inout Anchor<CGRect>?, nextValue: () -> Anchor<CGRect>?) { if let next = nextValue() { value = next } }
}

private struct WaveMobileSelectionHeader: View {
    @ObservedObject var model: WaveLibraryModel
    let songs: [Song]
    let title: String
    var inset = true

    private var selectableIDs: Set<Int64> {
        Set(songs.filter { waveBlockedSourceName(song: $0, availability: model.availabilityByID[$0.uniqueId] ?? .unknown, blockedSources: model.blockedSources) == nil }.map(\.uniqueId))
    }
    var body: some View {
        LWCompactSelectionHeader(count: model.selected.count,
            allSelected: !model.selected.isEmpty && model.selected == selectableIDs,
            onToggleAll: { model.selected = !model.selected.isEmpty && model.selected == selectableIDs ? [] : selectableIDs },
            onDone: { model.selecting = false; model.selected = []; model.selectionAnchor = nil },
            title: title, inset: inset)
    }
}

private struct WaveSelectionBarEntrance: ViewModifier {
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var visible = false

    func body(content: Content) -> some View {
        content.opacity(visible || reduceMotion ? 1 : 0)
            .offset(y: visible || reduceMotion ? 0 : 10)
            .task {
                await Task.yield()
                guard !Task.isCancelled else { return }
                withAnimation(reduceMotion ? nil : LWMotion.springAnimation(LWMotion.base)) { visible = true }
            }
    }
}

private struct WaveLibraryEntrance: ViewModifier {
    let index: Int
    var mobileOnly = false
    @State private var visible = false
    @Environment(\.lwReduceMotion) private var reduceMotion
    @Environment(\.lwWideLayout) private var wide
    @Environment(\.waveLibraryEntry) private var entry

    func body(content: Content) -> some View {
        let limit = wide ? 16 : 14
        let entering = entry.enabled && (!mobileOnly || !wide) && index < limit && !reduceMotion
            && Date().timeIntervalSince(entry.startedAt) < LWMotion.slow + Double(limit - 1) * LWMotion.stagger
        content.opacity(visible || !entering ? 1 : 0)
            .offset(y: visible || !entering ? 0 : 10)
            .task(id: entry) {
                guard entering else { visible = true; return }
                var transaction = Transaction(); transaction.disablesAnimations = true
                withTransaction(transaction) { visible = false }
                await Task.yield()
                guard !Task.isCancelled else { return }
                withAnimation(LWMotion.out().delay(Double(index) * LWMotion.stagger)) { visible = true }
            }
    }
}

struct WaveSongQuery {
    private let words: [String]

    init(_ query: String) { words = query.lowercased().split(whereSeparator: \.isWhitespace).map(String.init) }

    var text: String {
        words.filter { !["cloud", "offline", "favorites", "fav", "flac", "mp3", "m4a", "wav", "aiff", "unplayed"].contains($0) && !$0.hasPrefix("year:") }.joined(separator: " ")
    }

    var hasFilters: Bool { words.count != text.split(whereSeparator: \.isWhitespace).count }

    func matchesWords(in value: String) -> Bool {
        text.split(whereSeparator: \.isWhitespace).allSatisfy { value.localizedStandardContains(String($0)) }
    }

    func matches(_ song: Song, availability: FileAvailability = .unknown) -> Bool {
        words.allSatisfy { word in
            switch word {
            case "offline": return availability == .onDevice
            case "cloud": return availability == .inCloud
            case "favorites", "fav": return song.isFavorite != 0
            case "unplayed":
                let counts = LocalWavePreferences.current.dictionary(forKey: "lwPlayCounts") as? [String: Int] ?? [:]
                return counts[String(song.uniqueId), default: 0] == 0
            case "flac", "mp3", "m4a", "wav", "aiff":
                if let format = song.fileExtension { return format.lowercased() == word }
                if let relativePath = song.relativePath { return URL(fileURLWithPath: relativePath).pathExtension.lowercased() == word }
                if let local = song.localFilePath { return URL(fileURLWithPath: local).pathExtension.lowercased() == word }
                guard let bookmark = song.bookmark else { return false }
                var stale = false
                let url = try? URL(resolvingBookmarkData: bookmark, options: sourceBookmarkResolutionOptions.union(.withoutUI), bookmarkDataIsStale: &stale)
                return url?.pathExtension.lowercased() == word
            default:
                if word.hasPrefix("year:"), let year = Int(word.dropFirst(5)) { return song.releaseYear == year }
                return [song.title, song.artist, song.album, song.albumArtist].contains { $0.localizedStandardContains(word) }
            }
        }
    }
}

/// Reference filter tokens wrap as their intrinsic widths exceed the content column.
private struct WaveTokenLayout: Layout {
    var spacing: CGFloat
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        arrange(width: proposal.width ?? 390, subviews: subviews).size
    }
    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let result = arrange(width: bounds.width, subviews: subviews)
        for (index, point) in result.positions.enumerated() {
            subviews[index].place(at: CGPoint(x: bounds.minX + point.x, y: bounds.minY + point.y), proposal: .unspecified)
        }
    }
    private func arrange(width: CGFloat, subviews: Subviews) -> (size: CGSize, positions: [CGPoint]) {
        var x: CGFloat = 0, y: CGFloat = 0, rowHeight: CGFloat = 0
        var positions: [CGPoint] = []
        for view in subviews {
            let size = view.sizeThatFits(.unspecified)
            if x > 0 && x + size.width > width { x = 0; y += rowHeight + spacing; rowHeight = 0 }
            positions.append(CGPoint(x: x, y: y)); x += size.width + spacing; rowHeight = max(rowHeight, size.height)
        }
        return (CGSize(width: width, height: y + rowHeight), positions)
    }
}

func waveRecentlyAddedSongs(_ songs: [Song]) -> [Song] {
    Array(songs.sorted { $0.createdAt > $1.createdAt }.prefix(24))
}

func waveMostPlayedSongs(_ songs: [Song], counts: [String: Int]) -> [Song] {
    Array(songs.filter { counts[String($0.uniqueId), default: 0] > 0 }
        .sorted { counts[String($0.uniqueId), default: 0] > counts[String($1.uniqueId), default: 0] }.prefix(24))
}

private func waveArtistSortName(_ artist: String) -> String { artist.hasPrefix("The ") ? String(artist.dropFirst(4)) : artist }

private struct LWLibraryKeyboardCoveredKey: EnvironmentKey {
    static let defaultValue = false
}

extension EnvironmentValues {
    var lwLibraryKeyboardCovered: Bool {
        get { self[LWLibraryKeyboardCoveredKey.self] }
        set { self[LWLibraryKeyboardCoveredKey.self] = newValue }
    }
}

private struct WaveTableHover: ViewModifier {
    var opacity = 0.6
    var selected = false
    @State private var hovering = false
    @Environment(\.lwReduceMotion) private var reduceMotion
    func body(content: Content) -> some View {
        content.background(selected ? LWTheme.panel2 : hovering ? LWTheme.panel.opacity(opacity) : LWTheme.paper)
            .onHover { value in withAnimation(reduceMotion ? nil : LWMotion.out(LWMotion.quick)) { hovering = value } }
    }
}

private enum WaveSongColumn: String, Hashable {
    case number = "#", title = "Title", artist = "Artist", album = "Album", format = "Format", state = "On device", time = "Time", actions = "Actions"
}

private struct WaveSongColumns {
    let items: [WaveSongColumn]
    let widths: [WaveSongColumn: CGFloat]
    subscript(_ column: WaveSongColumn) -> CGFloat { widths[column] ?? 0 }

    static func make(songs: [Song], available: CGFloat, scale: CGFloat, artist: Bool, album: Bool, format: Bool, state: Bool, numbered: Bool, currentID: Int64?, sort: String?) -> WaveSongColumns {
        func width(_ text: String, face: String, size: CGFloat, tabular: Bool = false) -> CGFloat {
            var font = UIFont(name: face, size: size * scale) ?? .systemFont(ofSize: size * scale)
            if tabular {
                let descriptor = font.fontDescriptor.addingAttributes([.featureSettings: [[UIFontDescriptor.FeatureKey.type: kNumberSpacingType, UIFontDescriptor.FeatureKey.selector: kMonospacedNumbersSelector]]])
                font = UIFont(descriptor: descriptor, size: size * scale)
            }
            return (text as NSString).size(withAttributes: [.font: font]).width
        }
        var columns: [WaveSongColumn] = [.number, .title]
        if artist { columns.append(.artist) }; if album { columns.append(.album) }
        if format { columns.append(.format) }; if state { columns.append(.state) }
        columns += [.time, .actions]
        var preferred: [WaveSongColumn: CGFloat] = [:]
        for column in columns {
            let text = column.rawValue + (sort == column.rawValue ? " ↑" : "")
            preferred[column] = width(text, face: "HostGrotesk-Medium", size: 12) + (column == .number ? 40 : 16)
        }
        preferred[.number] = max(44, preferred[.number] ?? 44)
        preferred[.actions] = 52
        for (index, song) in songs.enumerated() {
            let face = song.id == currentID ? "HostGrotesk-Medium" : "HostGrotesk-Regular"
            let number = numbered ? song.trackNumber ?? index + 1 : index + 1
            preferred[.number] = max(preferred[.number] ?? 44, width(String(number), face: "HostGrotesk-Regular", size: 13, tabular: true) + 40)
            if song.id == currentID { preferred[.number] = max(preferred[.number] ?? 44, 54) }
            preferred[.title] = max(preferred[.title] ?? 0, min(240, width(song.title, face: face, size: 14) + 16))
            if artist { preferred[.artist] = max(preferred[.artist] ?? 0, min(240, width(song.artist, face: face, size: 14) + 16)) }
            if album { preferred[.album] = max(preferred[.album] ?? 0, min(240, width(song.album, face: face, size: 14) + 16)) }
            if format { preferred[.format] = max(preferred[.format] ?? 0, width(song.fileExtension?.lowercased() ?? "—", face: "GeistMono-Regular", size: 12) + 16) }
            preferred[.time] = max(preferred[.time] ?? 0, width(waveDuration(song.duration), face: "GeistMono-Regular", size: 12) + 16)
        }
        let flexible = columns.filter { $0 != .number && $0 != .actions }
        let total = max(1, flexible.reduce(CGFloat(0)) { $0 + (preferred[$1] ?? 0) })
        let remaining = max(0, available - (preferred[.number] ?? 44) - 52)
        var widths = preferred
        for column in flexible { widths[column] = (preferred[column] ?? 0) * remaining / total }
        return WaveSongColumns(items: columns, widths: widths)
    }
}

private struct WaveSongColumnsHeading: View {
    let columns: WaveSongColumns
    @Binding var sort: String?
    @Binding var ascending: Bool

    var body: some View {
        HStack(spacing: 0) {
            ForEach(columns.items, id: \.self) { column in
                Group {
                    if column == .actions { Text("Actions").frame(width: 1, height: 1).clipped().frame(height: 16.8) }
                    else if column == .state { Text(column.rawValue).font(LWTheme.text(12).weight(.medium)).lwLineBox(12, 16.8) }
                    else { WaveSongTableHeading(label: column.rawValue, column: $sort, ascending: $ascending) }
                }
                .padding(.leading, column == .number ? 32 : column == .actions ? 0 : 8)
                .padding(.trailing, column == .actions ? 20 : 8)
                .frame(width: columns[column], alignment: column == .time || column == .actions ? .trailing : .leading)
            }
        }.foregroundStyle(LWTheme.secondary).padding(.vertical, 10).padding(.bottom, 0.5).background(LWTheme.paper)
            .modifier(LWCollapsedTableRules(top: false, bottom: true))
    }
}

private struct WaveConditionalClip: ViewModifier {
    let enabled: Bool
    func body(content: Content) -> some View {
        if enabled { content.clipped() } else { content }
    }
}

func waveBlockedSourceName(song: Song, availability: FileAvailability, blockedSources: [Int64: String]) -> String? {
    guard availability != .onDevice, let sourceID = song.sourceId else { return nil }
    return blockedSources[sourceID]
}

