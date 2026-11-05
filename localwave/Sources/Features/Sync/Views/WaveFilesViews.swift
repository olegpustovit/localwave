#if os(iOS)
import SwiftUI
import CoreText
import UniformTypeIdentifiers
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

struct WaveFolderNavigationState {
    struct DirectorySummary { var count: Int; var bytes: Int64 }
    var browse: SourceBrowseSnapshot
    var availabilityFilter = "All"
    var selecting = false
    var desktopSort = ""
    var desktopAscending = true
    var folderTitle: String?
    var resolvedFolderNames: [String] = []
    var folderSongs: [Song] = []
    var folderOfflineCount = 0
    var rowSongs: [Int64: Song] = [:]
    var availability: [Int64: FileAvailability] = [:]
    var lists: [Playlist] = []
    var pinnedSongIDs = Set<String>()
    var folderKeptOffline = false
    var directorySummaries: [Int64: DirectorySummary] = [:]
    var columnMetrics: [CGFloat] = []
}

struct WaveFilesNavigationState {
    struct SourceStats { var songs = 0; var local = 0; var cloud = 0; var pinned = 0 }
    var sourcePath: [Int64] = []
    var folders: [Int64: WaveFolderNavigationState] = [:]
    var sourceSnapshots: [Int64: Source] = [:]
    var sourceStats: [Int64: SourceStats] = [:]
    var sourceSongs: [Int64: [Song]] = [:]
    var cacheBytes: Int64 = 0
}

struct WaveFilesView: View {
    let dependencies: DependencyContainer
    let addRequest: Int
    private let rootEntryStartedAt: Date?
    private let onDetailChange: ((Bool) -> Void)?
    private let onNavigationStateChange: ((WaveFilesNavigationState) -> Void)?
    private let onSourceRootChange: ((Source?) -> Void)?
    private let onSelectionChange: (([Song]) -> Void)?
    private let onRootBack: (() -> Void)?
    private let onMusicSelection: ((Result<[URL], Error>, Bool) -> Void)?
    @Environment(\.lwLibraryKeyboardCovered) private var keyboardCovered
    @State private var folderStates: [Int64: WaveFolderNavigationState] = [:]
    @State private var skipInitialSourceInspection = false
    @State private var sourcePath: [Int64] = []
    @State private var sourceSnapshots: [Int64: Source] = [:]
    @State private var sourceDestinationStartedAt: Date?
    @State private var sourceDestinationAnimates = false
    @State private var sourceRowsStartedAt: Date?
    @State private var animatesSourceBack = false
    @Environment(\.lwWideLayout) private var wideLayout
    @StateObject private var model: SyncViewModel
    @State private var picker = false
    @State private var audioPicker = false
    @State private var addSheet = false
    @State private var reconnect: Source?
    @State private var sourceOptions: Source?
    @EnvironmentObject private var presenter: LWSheetPresenter
    @State private var displayRevision = 0
    @State private var sourceStats: [Int64: SourceDisplayStats] = [:]
    @State private var sourceSongs: [Int64: [Song]] = [:]
    @State private var cacheBytes: Int64 = 0
    @AppStorage("lwCacheGB") private var cacheGB = 8
    @State private var error: String?
    @EnvironmentObject private var playerVM: PlayerViewModel
    @Environment(\.scenePhase) private var scenePhase
    @ScaledMetric(relativeTo: .body) private var headingHeight = 33.6
    @ScaledMetric(relativeTo: .body) private var sourceMetaHeight = 18.2
    @ScaledMetric(relativeTo: .body) private var sourceCancelWidth = 64

    init(dependencies: DependencyContainer, addRequest: Int = 0, rootEntryStartedAt: Date? = nil, onDetailChange: ((Bool) -> Void)? = nil, initialNavigationState: WaveFilesNavigationState? = nil, onNavigationStateChange: ((WaveFilesNavigationState) -> Void)? = nil, onSourceRootChange: ((Source?) -> Void)? = nil, onSelectionChange: (([Song]) -> Void)? = nil, onRootBack: (() -> Void)? = nil, onMusicSelection: ((Result<[URL], Error>, Bool) -> Void)? = nil) {
        self.dependencies = dependencies
        self.addRequest = addRequest
        self.rootEntryStartedAt = rootEntryStartedAt
        self.onDetailChange = onDetailChange
        self.onNavigationStateChange = onNavigationStateChange
        self.onSourceRootChange = onSourceRootChange
        self.onSelectionChange = onSelectionChange
        self.onRootBack = onRootBack
        self.onMusicSelection = onMusicSelection
        if let initialNavigationState {
            _sourcePath = State(initialValue: initialNavigationState.sourcePath)
            _folderStates = State(initialValue: initialNavigationState.folders)
            _sourceSnapshots = State(initialValue: initialNavigationState.sourceSnapshots)
            _sourceStats = State(initialValue: initialNavigationState.sourceStats)
            _sourceSongs = State(initialValue: initialNavigationState.sourceSongs)
            _cacheBytes = State(initialValue: initialNavigationState.cacheBytes)
            _skipInitialSourceInspection = State(initialValue: true)
        }
        _sourceDestinationStartedAt = State(initialValue: rootEntryStartedAt)
        _sourceRowsStartedAt = State(initialValue: rootEntryStartedAt)
        _model = StateObject(wrappedValue: dependencies.waveSyncViewModel)
    }

    var body: some View {
        NavigationStack(path: sourceNavigationPath) {
            sourcePage.background {
                if let onRootBack, sourcePath.isEmpty, !keyboardCovered, presenter.content == nil {
                    Button("Back in folders", action: onRootBack).keyboardShortcut("[", modifiers: .command).hidden()
                }
            }
        }
        .foregroundStyle(LWTheme.ink).tint(LWTheme.ink).buttonStyle(.plain)
        .onChange(of: sourcePath, initial: true) { _, value in
            onDetailChange?(!value.isEmpty)
            if value.isEmpty { onSourceRootChange?(nil); onSelectionChange?([]) }
            publishFilesState()
        }
        .onDisappear { publishFilesState() }
    }

    private var sourcePage: some View {
        sourceObservedPage
            .lwSheet(isPresented: $addSheet) { addMusic }
            .lwSheet(isPresented: sourceOptionsPresented) {
                if let source = sourceOptions {
                    WaveFolderOptionsSheet(title: URL(fileURLWithPath: source.dirPath).lastPathComponent,
                        path: URL(fileURLWithPath: source.dirPath).lastPathComponent, actions: sourceSheetActions(source))
                }
            }
            .fileImporter(isPresented: $picker, allowedContentTypes: [.folder], allowsMultipleSelection: true) { selection in
                handleFolders(selection)
            }
            .fileImporter(isPresented: $audioPicker, allowedContentTypes: [.audio], allowsMultipleSelection: true) { selection in
                handleAudio(selection)
            }
            .alert("Unable to complete", isPresented: sourceErrorPresented) {
                Button("OK") { error = nil; model.errorMessage = nil }
            } message: { Text(error ?? model.errorMessage ?? "") }
    }

    private var sourceObservedPage: some View {
        sourceScrollContent
            .background(LWTheme.paper)
            .modifier(LWAppear(translation: CGSize(width: -32, height: 0), enabled: animatesSourceBack && !wideLayout))
            .navigationDestination(for: Int64.self) { sourceID in
                if let source = model.sources.first(where: { $0.stableId == sourceID }) ?? sourceSnapshots[sourceID] {
                    WaveFolderView(source: source, dependencies: dependencies, initialEntryStartedAt: sourceDestinationStartedAt,
                                   initialNavigationState: folderStates[sourceID],
                                   onNavigationStateChange: { state in folderStates[sourceID] = state; publishFilesState() },
                                   onSourceRootChange: onSourceRootChange, onBack: closeSource,
                                   onSelectionChange: { songs in if sourcePath.last == sourceID { onSelectionChange?(songs) } })
                        .modifier(LWAppear(translation: CGSize(width: wideLayout ? 0 : 32, height: 0), duration: wideLayout ? LWMotion.base : LWMotion.slow, enabled: sourceDestinationAnimates))
                }
            }
            .toolbar(.hidden, for: .navigationBar)
            .task {
                model.loadSources()
                cacheBytes = (try? await Task.detached(priority: .utility) { try BackgroundFileService.cachedByteCount() }.value) ?? 0
                if addRequest > 0 { addSheet = true }
            }
            .task(id: model.sources.map { String($0.stableId) }.joined(separator: ",") + "-\(displayRevision)") {
                if skipInitialSourceInspection { skipInitialSourceInspection = false; return }
                await refreshSourceDisplay()
            }
            .onReceive(NotificationCenter.default.publisher(for: Notification.Name("LibraryRefresh")).debounce(for: .milliseconds(250), scheduler: RunLoop.main)) { _ in model.loadSources(); displayRevision += 1 }
            .onChange(of: addRequest) { _, request in if request > 0 { addSheet = true } }
            .onChange(of: rootEntryStartedAt) { _, value in
                sourceRowsStartedAt = value
                if !sourcePath.isEmpty { sourceDestinationStartedAt = value; sourceDestinationAnimates = false }
                animatesSourceBack = false
            }
            .onChange(of: scenePhase) { _, phase in
                #if DEBUG
                if LWReferenceFixture.enabled { return }
                #endif
                if phase == .active { model.refreshSources() }
            }
            .refreshable { model.refreshSources() }
    }

    private var sourceScrollContent: some View {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    HStack(alignment: .top) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Folders").font(LWTheme.text(32).weight(.medium)).tracking(-1.12).lwLineBox(32, 33.6).frame(height: headingHeight)
                            Text("\(model.sources.count) sources, watched for changes")
                                .font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lwLineBox(13, 18.2)
                        }
                        Spacer()
                        Button { addSheet = true } label: {
                            LWIcon("plus", size: 24).frame(width: 44, height: 44)
                        }.accessibilityLabel("Add music").buttonStyle(LWIconButtonStyle()).padding(.trailing, -10)
                    }
                    .padding(.bottom, 16)
                    if model.sources.isEmpty {
                        MatrixText(text: "00", height: 100)
                        Text("Point it at a folder.").font(LWTheme.text(28).weight(.medium))
                        Text("Your files stay where they are. Nothing is uploaded.")
                            .foregroundStyle(LWTheme.secondary)
                        ForEach(Array(["Choose a folder in Files", "LocalWave indexes its audio files", "Play your music from the library"].enumerated()), id: \.offset) { index, title in
                            HStack(spacing: 16) {
                                Text("\(index + 1)").font(LWTheme.data(13)).foregroundStyle(LWTheme.secondary)
                                Text(title)
                            }.frame(minHeight: 44).frame(maxWidth: .infinity, alignment: .leading)
                                .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1) }
                        }
                    }
                    ForEach(Array(model.sources.enumerated()), id: \.element.stableId) { index, source in
                        sourceCard(source).padding(.bottom, 8).modifier(WaveFilesRise(index: index, startedAt: sourceRowsStartedAt))
                    }
                    VStack(alignment: .leading, spacing: 0) {
                        Rectangle().fill(LWTheme.line).frame(height: 1).padding(.bottom, 16)
                        HStack(alignment: .firstTextBaseline) {
                            Text("Offline space").font(LWTheme.text(14)).lwLineBox(14, 19.6)
                            Spacer()
                            Text("\(Double(cacheBytes) / 1_073_741_824, specifier: "%.1f") of \(cacheGB) GB").font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true)
                        }
                        GeometryReader { bounds in
                            ZStack(alignment: .leading) {
                                Rectangle().fill(LWTheme.line)
                                Rectangle().fill(LWTheme.ink).frame(width: bounds.size.width * min(1, Double(cacheBytes) / (Double(max(1, cacheGB)) * 1_073_741_824)))
                            }
                        }.frame(height: 4).padding(.top, 10)
                        Text("Cloud songs download when played and the next two download ahead. Pinned folders stay.")
                            .font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).padding(.top, 8)
                    }.padding(.top, 16)

                }.padding(.horizontal, 16).padding(.top, 10).padding(.bottom, 24)
            }
    }

    private var sourceOptionsPresented: Binding<Bool> {
        Binding<Bool>(get: { sourceOptions != nil }, set: { shown in if !shown { sourceOptions = nil } })
    }

    private var sourceErrorPresented: Binding<Bool> {
        Binding<Bool>(get: { error != nil || model.errorMessage != nil }, set: { shown in
            if !shown { error = nil; model.errorMessage = nil }
        })
    }

    private func publishFilesState() {
        onNavigationStateChange?(WaveFilesNavigationState(sourcePath: sourcePath, folders: folderStates,
            sourceSnapshots: sourceSnapshots, sourceStats: sourceStats, sourceSongs: sourceSongs, cacheBytes: cacheBytes))
    }

    private var sourceNavigationPath: Binding<[Int64]> {
        Binding(get: { sourcePath }, set: { next in
            var transaction = Transaction(); transaction.disablesAnimations = true
            withTransaction(transaction) { sourcePath = next }
        })
    }

    private func openSource(_ source: Source) {
        var transaction = Transaction(); transaction.disablesAnimations = true
        withTransaction(transaction) {
            sourceSnapshots[source.stableId] = source
            sourceDestinationStartedAt = Date()
            sourceDestinationAnimates = true
            animatesSourceBack = false
            sourcePath = [source.stableId]
        }
    }

    private func closeSource() {
        var transaction = Transaction(); transaction.disablesAnimations = true
        withTransaction(transaction) {
            sourceRowsStartedAt = wideLayout ? nil : Date()
            animatesSourceBack = !wideLayout
            sourcePath = []
        }
    }

    private func sourceCard(_ source: Source) -> some View {
        Group {
            if source.syncError == nil && model.currentSyncSourceId != source.id {
                Button { openSource(source) } label: { sourceCardContents(source) }
                    .buttonStyle(WaveSourceCardStyle())
                    .accessibilityIdentifier("source.open." + String(source.stableId))
            } else { sourceCardContents(source) }
        }.lwContextMenu { sourceMenu(source) }
    }

    private func sourceCardContents(_ source: Source) -> some View {
        let syncing = model.currentSyncSourceId == source.id
        let stats = source.id.flatMap { sourceStats[$0] }
        let display: (name: String, root: String, scanned: String)?
        #if DEBUG
        display = LWReferenceFixture.sourceDisplay(source: source)
        #else
        display = nil
        #endif
        return VStack(alignment: .leading, spacing: 0) {
                HStack(alignment: .top) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(display?.name ?? URL(fileURLWithPath: source.dirPath).lastPathComponent).font(LWTheme.text(17).weight(.medium)).lwLineBox(17, 23.8)
                            .modifier(WaveSourceGeometry(name: "\(source.id ?? -1).title"))
                        Text(display?.root ?? source.dirPath).font(LWTheme.text(13)).lwLineBox(13, 18.2)
                            .foregroundStyle(LWTheme.secondary).lineLimit(2)
                            .modifier(WaveSourceGeometry(name: "\(source.id ?? -1).path"))
                    }
                    Spacer()
                    Text(sourceCountLabel(source, stats: stats)).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
                }
            if syncing {
                WaveSourceScanBar(progress: model.importProgress.map { $0 / 100 })
                    .padding(.top, 14).padding(.bottom, 10)
                Text(model.importProgress == nil ? "Scanning — names first, tags and covers follow" : "Indexing audio")
                    .font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lwLineBox(13, 18.2)
                    .lineLimit(1).truncationMode(.middle)
                    .frame(maxWidth: .infinity, minHeight: sourceMetaHeight, alignment: .leading)
                    .padding(.trailing, model.importProgress == nil ? 0 : sourceCancelWidth)
                    .overlay(alignment: .trailing) {
                        if model.importProgress != nil {
                            Button { model.cancelCurrentImport() } label: {
                                Text("Cancel").font(LWTheme.text(13)).lwLineBox(13, 18.2).frame(width: sourceCancelWidth, height: max(44, sourceMetaHeight))
                            }.accessibilityLabel("Cancel import")
                        }
                    }
            } else if source.syncError == nil {
                GeometryReader { bounds in
                    ZStack(alignment: .leading) {
                        Rectangle().fill(LWTheme.line)
                        Rectangle().fill(LWTheme.ink).frame(width: bounds.size.width * Double(stats?.local ?? 0) / Double(max(1, stats?.songs ?? 0)))
                    }
                }.frame(height: 4).padding(.top, 14).padding(.bottom, 10)
                HStack {
                    if let stats {
                        Text("\(stats.pinned) kept · \(stats.local) on device" + (stats.cloud > 0 ? " · \(stats.cloud) in cloud" : ""))
                            .font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lwLineBox(13, 18.2)
                    }
                    Spacer(minLength: 6)
                    Text(display?.scanned ?? source.lastSyncedAt.map { $0.formatted(.relative(presentation: .numeric, unitsStyle: .abbreviated)) } ?? "Not scanned")
                        .font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lwLineBox(13, 18.2)

                }.modifier(WaveSourceGeometry(name: "\(source.id ?? -1).footer"))
            }
            if let message = source.syncError, !syncing {
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    LWStateGlyph(state: .failed, availability: .unknown, pinned: false)
                    Text(message).font(LWTheme.text(13)).lwLineBox(13, 18.85).fixedSize(horizontal: false, vertical: true)
                }.padding(.vertical, 12)
                HStack(spacing: 2) {
                    Button { reconnect = source; picker = true } label: { Text("Reconnect folder").lwLineBox(14, 19.6) }
                        .frame(maxWidth: .infinity).frame(minHeight: 44).background(LWTheme.ink).foregroundStyle(LWTheme.paper)
                    Button { sourceOptions = source } label: {
                        Text("Options").lwLineBox(14, 19.6).frame(maxWidth: .infinity).frame(minHeight: 44).background(LWTheme.key2)
                    }
                }.font(LWTheme.text(14).weight(.medium)).buttonStyle(LWKeyStyle())
            }
        }
        .padding(17).overlay { Rectangle().strokeBorder(source.syncError == nil || syncing ? LWTheme.line : LWTheme.ink) }
        #if DEBUG
        .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { frame in
            if ProcessInfo.processInfo.environment["LOCALWAVE_SOURCE_GEOMETRY"] == "1" {
                print("LWSourceCardGeometry source=\(source.id ?? -1) frame=\(frame)")
            }
        }
        #endif
    }

    private func sourceCountLabel(_ source: Source, stats: SourceDisplayStats?) -> String {
        guard model.currentSyncSourceId == source.id else { return stats.map { "\($0.songs) songs" } ?? "\(source.totalPaths ?? 0) indexed items" }
        if let completed = model.importedFileCount, let total = model.importFileCount { return "\(completed) of \(total)" }
        if let percentage = model.importProgress, percentage.isFinite { return "\(Int(percentage.rounded()))%" }
        return "\(model.scannedItemCount) items"
    }

    private typealias SourceDisplayStats = WaveFilesNavigationState.SourceStats

    private func refreshSourceDisplay() async {
        let sourceIDs = Set(model.sources.compactMap(\.id))
        let pins = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
        var result: [Int64: SourceDisplayStats] = [:]
        var loadedSongs: [Int64: [Song]] = [:]
        var offset = 0
        do {
            while !Task.isCancelled {
                let page = try await dependencies.songRepository.searchSongsFTS(query: "", limit: 500, offset: offset)
                for song in page {
                    guard !Task.isCancelled else { return }
                    guard let sourceID = song.sourceId, sourceIDs.contains(sourceID) else { continue }
                    loadedSongs[sourceID, default: []].append(song)
                    let state = await dependencies.sourceAccess.availability(for: song)
                    var stats = result[sourceID] ?? SourceDisplayStats()
                    stats.songs += 1
                    if state == .onDevice { stats.local += 1 }
                    if state == .inCloud { stats.cloud += 1 }
                    if let id = song.id, pins.contains(String(id)) { stats.pinned += 1 }
                    result[sourceID] = stats
                }
                if page.count < 500 { break }
                offset += page.count
            }
            guard !Task.isCancelled else { return }
            sourceStats = result
            sourceSongs = loadedSongs
            cacheBytes = (try? await Task.detached(priority: .utility) { try BackgroundFileService.cachedByteCount() }.value) ?? cacheBytes
            publishFilesState()
        } catch { /* Keep the last successful display; operational errors belong to SyncViewModel. */ }
    }

    private func sourceMenu(_ source: Source) -> LWMenuRequest {
        let songs = sourceSongs[source.id ?? -1] ?? []
        return LWMenuRequest(groups: [
            LWMenuGroup([
                LWMenuAction(title: "Open", icon: "folder", action: { openSource(source) }),
                LWMenuAction(title: "Play all", icon: "play", disabled: songs.isEmpty, action: { playSource(source) }),
                LWMenuAction(title: "Rescan now", icon: "rep", action: { model.resyncSource(source) })
            ]),
            LWMenuGroup([source.syncError != nil
                ? LWMenuAction(title: "Reconnect folder…", icon: "folder", action: { reconnect = source; picker = true })
                : LWMenuAction(title: "Keep everything offline", icon: "dl", disabled: songs.isEmpty, action: { keepSourceOffline(source) })]),
            LWMenuGroup([LWMenuAction(title: "Remove source", icon: "close", destructive: true, action: { confirmRemoveSource(source) })])
        ], preview: AnyView(sourceCardContents(source).allowsHitTesting(false)))
    }

    private func sourceSheetActions(_ source: Source) -> [LWMenuAction] {
        let songs = sourceSongs[source.id ?? -1] ?? []
        return [
            LWMenuAction(title: "Keep this folder offline", icon: "dl", disabled: songs.isEmpty, action: { keepSourceOffline(source) }),
            LWMenuAction(title: "Rescan now", icon: "rep", action: { model.resyncSource(source) }),
            LWMenuAction(title: "Add folder to queue", icon: "queue", disabled: songs.isEmpty, action: { orderedSourceSongs(source).forEach { playerVM.addToQueue($0) } }),
            LWMenuAction(title: "Remove source (files are not deleted)", icon: "close", action: { confirmRemoveSource(source) })
        ]
    }

    private func orderedSourceSongs(_ source: Source) -> [Song] {
        (sourceSongs[source.id ?? -1] ?? []).sorted { ($0.relativePath ?? $0.title).localizedStandardCompare($1.relativePath ?? $1.title) == .orderedAscending }
    }

    private func playSource(_ source: Source) {
        let songs = orderedSourceSongs(source)
        guard !songs.isEmpty else { return }
        playerVM.configureQueue(songs: songs, startIndex: 0,
            context: .folder(sourceID: source.id ?? -1, pathID: source.pathId, name: URL(fileURLWithPath: source.dirPath).lastPathComponent))
        playerVM.playQueueEntry(at: 0)
    }

    private func keepSourceOffline(_ source: Source) {
        let songs = orderedSourceSongs(source)
        withOfflineCacheLock {
            var pins = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
            pins.formUnion(songs.map { String($0.uniqueId) })
            LocalWavePreferences.current.set(Array(pins), forKey: "lwPinnedSongs")
        }
        Task {
            do {
                for song in songs where song.fileState != .copied {
                    if let id = song.id { try await dependencies.songRepository.markSongForCopy(songId: id) }
                }
                NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
            } catch { self.error = error.localizedDescription }
        }
    }

    private func confirmRemoveSource(_ source: Source) {
        presenter.present(dialog: LWDialogRequest(title: "Remove \(URL(fileURLWithPath: source.dirPath).lastPathComponent)?",
            message: "Its songs leave the library. Files are not deleted.", primaryTitle: "Remove",
            destructive: true, mobileConfirmation: true, onSubmit: { _ in model.deleteSource(source) }))
    }

    private var addMusic: some View {
            VStack(alignment: .leading, spacing: 0) {
                HStack(alignment: .center) {
                    VStack(alignment: .leading, spacing: 0) {
                        Text("Add music").font(LWTheme.text(20).weight(.medium)).tracking(-0.2).lwLineBox(20, 28).frame(minHeight: 28)
                        Text("Files stay where they are").font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lwLineBox(13, 18.2).frame(minHeight: 18.2)
                    }
                    Spacer()
                    Button { addSheet = false } label: {
                        LWIcon("close", size: 22).frame(width: 44, height: 44)
                    }.accessibilityLabel("Close").buttonStyle(LWIconButtonStyle()).padding(.trailing, -10)
                }.padding(.horizontal, 16).padding(.top, 12).padding(.bottom, 8)
                Button { addSheet = false; picker = true } label: {
                    addOption("A folder", detail: "Watched for changes. iCloud Drive, Google Drive, Dropbox, On My iPhone, USB drives, SMB servers.")
                }.padding(.bottom, 8)
                Button { addSheet = false; audioPicker = true } label: {
                    addOption("Some files", detail: "Play them now; they join the library too.")
                }
                (Text("Google Drive not listed? ").fontWeight(.medium).foregroundStyle(LWTheme.ink) + Text("Install the Google Drive app, then in Files tap Browse → ⋯ → Edit and turn on Google Drive. Songs download from Drive when played.").foregroundStyle(LWTheme.secondary))
                    .font(LWTheme.text(13)).lwLineBox(13, 19.5).padding(15)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .overlay { Rectangle().strokeBorder(LWTheme.line) }
                    .padding(.horizontal, 16).padding(.top, 12)
            }.background(LWTheme.paper).foregroundStyle(LWTheme.ink).buttonStyle(.plain)
    }

    private func addOption(_ title: String, detail: String) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(title).font(LWTheme.text(17).weight(.medium)).lwLineBox(17, 23.8).frame(minHeight: 23.8)
            Text(detail).font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lwLineBox(13, 21).fixedSize(horizontal: false, vertical: true)
        }.frame(maxWidth: .infinity, alignment: .leading).padding(16).background(LWTheme.panel).padding(.horizontal, 16)
    }

    private func handleFolders(_ selection: Result<[URL], Error>) {
        if reconnect == nil, let onMusicSelection {
            onMusicSelection(selection, true)
            return
        }
        do {
            let urls = try selection.get()
            if let source = reconnect {
                reconnect = nil
                guard urls.count == 1, let url = urls.first else {
                    throw CustomError.genericError("Choose one folder to reconnect this source.")
                }
                try model.registerBookmark(url)
                Task {
                    do {
                        let saved = try await dependencies.reconnectSource(source, to: url)
                        model.resyncSource(saved)
                    } catch { self.error = error.localizedDescription }
                }
                return
            }
            for url in urls {
                try model.registerBookmark(url)
                model.createSource(path: url.path)
            }
        } catch {
            let failure = error as NSError
            if failure.domain != NSCocoaErrorDomain || failure.code != NSUserCancelledError { self.error = error.localizedDescription }
        }
        reconnect = nil
    }

    private func handleAudio(_ selection: Result<[URL], Error>) {
        if let onMusicSelection {
            onMusicSelection(selection, false)
            return
        }
        Task {
            do {
                let imported = try await dependencies.addMusicURLs(selection.get())
                if let first = imported.first {
                    playerVM.configureQueue(songs: imported, startIndex: 0, context: .selection("Selected files"))
                    playerVM.playSong(first)
                }
            } catch { self.error = error.localizedDescription }
        }
    }


}

@MainActor
private final class WaveFolderTrackState: ObservableObject {
    @Published var songs: [Int64: Song] = [:]
    init(songs: [Int64: Song] = [:]) { self.songs = songs }
}

struct WaveFolderView: View {
    let source: Source
    let dependencies: DependencyContainer
    private let onBack: (() -> Void)?
    private let backTitle: String
    private let initialEntryStartedAt: Date?
    private let onNavigationStateChange: ((WaveFolderNavigationState) -> Void)?
    private let onSourceRootChange: ((Source?) -> Void)?
    private let onSelectionChange: (([Song]) -> Void)?
    @State private var skipInitialBrowseRead = false
    @State private var initialAncestryResolved = false
    @State private var skipInitialFolderSummary = false
    @State private var skipInitialTrackInspection = false
    @Environment(\.lwLibraryKeyboardCovered) private var keyboardCovered
    @State private var rowEntryStartedAt: Date?
    @StateObject private var model: SourceBrowseViewModel
    private enum FolderEntry { case push, pop }
    @State private var folderEntry: FolderEntry?
    @State private var folderRevision = UUID()
    @State private var navigationTask: Task<Void, Never>?
    @State private var initialFolderRead = true
    @State private var risingPathIDs = Set<Int64>()
    @State private var progress: Double = 0
    @State private var currentName = ""
    @State private var error: String?
    @State private var loadingPathID: Int64?
    @State private var folderKeptOffline = false
    @State private var changingOffline = false
    @StateObject private var trackState = WaveFolderTrackState()
    @State private var availability: [Int64: FileAvailability] = [:]
    @State private var localAvailabilityFilter = "All"
    @Environment(\.lwFolderFilter) private var globalFolderFilter
    private var availabilityFilter: String {
        get { globalFolderFilter?.wrappedValue ?? localAvailabilityFilter }
        nonmutating set {
            if let globalFolderFilter { globalFolderFilter.wrappedValue = newValue }
            else { localAvailabilityFilter = newValue }
        }
    }
    @State private var lists: [Playlist] = []
    @State private var preparingPlayback = false
    @State private var selecting = false
    @AppStorage("lwShowNonAudio") private var showNonAudio = false
    @State private var folderTitle: String?
    @State private var resolvedFolderNames: [String] = []
    @State private var folderSnapshot: [Song] = []
    @State private var folderOfflineCount = 0
    private typealias DirectorySummary = WaveFolderNavigationState.DirectorySummary
    @State private var directorySummaries: [Int64: DirectorySummary] = [:]
    @State private var columnMetrics: [CGFloat] = []
    @State private var modifierKeys: EventModifiers = []
    @State private var selectionAnchor: Int64?
    @ScaledMetric(relativeTo: .body) private var tableTextScale: CGFloat = 1
    @ScaledMetric(relativeTo: .caption) private var tableDataScale: CGFloat = 1
    @State private var desktopSort = ""
    @State private var desktopAscending = true
    @State private var hoveredPathID: Int64?
    @State private var reconnectPicker = false
    @State private var folderOptions = false
    @State private var preparedShare: LWPreparedShare?
    @State private var shareDirectory: URL?
    @State private var sharing = false
    @EnvironmentObject private var presenter: LWSheetPresenter
    @State private var reconnectedSource: Source?
    @Environment(\.lwWideLayout) private var wideLayout
    @ScaledMetric(relativeTo: .body) private var folderHeadingHeight = 33.6
    @ScaledMetric(relativeTo: .body) private var desktopHeadingHeight = 34
    @State private var pinnedSongIDs = Set<String>()
    @EnvironmentObject private var playerVM: PlayerViewModel
    @Environment(\.dismiss) private var dismiss
    private var trackSongs: [Int64: Song] { trackState.songs }

    init(source: Source, dependencies: DependencyContainer, initialParentPathId: Int64? = nil, backTitle: String = "Folders", initialEntryStartedAt: Date? = nil, initialNavigationState: WaveFolderNavigationState? = nil, onNavigationStateChange: ((WaveFolderNavigationState) -> Void)? = nil, onSourceRootChange: ((Source?) -> Void)? = nil, onBack: (() -> Void)? = nil, onSelectionChange: (([Song]) -> Void)? = nil, includeAncestorHistory: Bool = false) {
        self.source = source
        self.dependencies = dependencies
        self.onBack = onBack
        self.backTitle = backTitle
        self.initialEntryStartedAt = initialEntryStartedAt
        self.onNavigationStateChange = onNavigationStateChange
        self.onSourceRootChange = onSourceRootChange
        self.onSelectionChange = onSelectionChange
        _rowEntryStartedAt = State(initialValue: initialEntryStartedAt)
        let browse = SourceBrowseViewModel(service: dependencies.sourceService.importService(), sourceId: source.id ?? -1,
                                          initialParentPathId: initialParentPathId ?? source.pathId,
                                          initialSnapshot: initialNavigationState?.browse)
        _model = StateObject(wrappedValue: browse)
        _initialAncestryResolved = State(initialValue: browse.restoredSnapshot || !includeAncestorHistory || browse.parentPathId == source.pathId)
        if browse.restoredSnapshot, let state = initialNavigationState {
            _skipInitialBrowseRead = State(initialValue: true)
            _skipInitialFolderSummary = State(initialValue: true)
            _skipInitialTrackInspection = State(initialValue: true)
            _initialFolderRead = State(initialValue: false)
            _localAvailabilityFilter = State(initialValue: state.availabilityFilter)
            _selecting = State(initialValue: state.selecting)
            _desktopSort = State(initialValue: state.desktopSort)
            _desktopAscending = State(initialValue: state.desktopAscending)
            _folderTitle = State(initialValue: state.folderTitle)
            _resolvedFolderNames = State(initialValue: state.resolvedFolderNames)
            _folderSnapshot = State(initialValue: state.folderSongs)
            _folderOfflineCount = State(initialValue: state.folderOfflineCount)
            _trackState = StateObject(wrappedValue: WaveFolderTrackState(songs: state.rowSongs))
            _availability = State(initialValue: state.availability)
            _lists = State(initialValue: state.lists)
            _pinnedSongIDs = State(initialValue: state.pinnedSongIDs)
            _folderKeptOffline = State(initialValue: state.folderKeptOffline)
            _directorySummaries = State(initialValue: state.directorySummaries)
            _columnMetrics = State(initialValue: state.columnMetrics)
        }
    }

    var body: some View {
        folderObservedPage
            .fileImporter(isPresented: $reconnectPicker, allowedContentTypes: [.folder]) { result in
                Task {
                    do {
                        let saved = try await dependencies.reconnectSource(reconnectedSource ?? source, to: result.get())
                        reconnectedSource = saved
                        dependencies.waveSyncViewModel.resyncSource(saved)
                        await dependencies.waveSyncViewModel.awaitPendingWork()
                        model.resetRoot(to: saved.pathId)
                    } catch { self.error = error.localizedDescription }
                }
            }
            .lwSheet(isPresented: $folderOptions) {
                WaveFolderOptionsSheet(title: displayedFolderTitle,
                    path: ([URL(fileURLWithPath: source.dirPath).lastPathComponent] + resolvedFolderNames).joined(separator: " / "),
                    actions: currentFolderSheetActions())
            }
            .sheet(item: $preparedShare, onDismiss: {
                if let shareDirectory { try? FileManager.default.removeItem(at: shareDirectory) }
                shareDirectory = nil
            }) { value in LWShareSheet(urls: value.urls) }
            .alert("Folder action failed", isPresented: folderErrorPresented) { Button("OK") { error = nil } } message: { Text(error ?? "") }
    }

    private var folderPageCore: some View {
        ZStack {
            Group {
                if wideLayout { desktopFolder } else { mobileFolder }
            }
            .modifier(LWAppear(
                translation: CGSize(width: wideLayout ? 0 : folderEntry == .pop ? -32 : 32, height: 0),
                duration: wideLayout ? LWMotion.base : LWMotion.slow,
                enabled: folderEntry != nil && (!wideLayout || folderEntry == .push)))
            .id(folderRevision)
        }
        .background(LWTheme.paper).foregroundStyle(LWTheme.ink).buttonStyle(.plain)
            .toolbar(.hidden, for: .navigationBar)
            .task(id: model.searchTerm) {
                if skipInitialBrowseRead {
                    risingPathIDs = Set((wideLayout ? desktopItems : visibleItems).prefix(wideLayout ? 16 : 14).map(\.pathId))
                    skipInitialBrowseRead = false
                    publishFolderState()
                    return
                }
                if !initialAncestryResolved {
                    if let paths = try? await dependencies.sourceService.importService().listItems(sourceId: source.id ?? -1, parentPathId: nil), !Task.isCancelled {
                        model.seedInitialAncestry(paths: paths, rootPathId: source.pathId)
                    }
                    guard !Task.isCancelled else { return }
                    initialAncestryResolved = true
                }
                await model.loadItems()
                guard !Task.isCancelled else { return }
                if initialFolderRead {
                    initialFolderRead = false
                    risingPathIDs = Set((wideLayout ? desktopItems : visibleItems).prefix(wideLayout ? 16 : 14).map(\.pathId))
                }
            }
            .onDisappear { navigationTask?.cancel(); publishFolderState() }
            .onReceive(model.objectWillChange) { _ in Task { await Task.yield(); publishFolderState() } }
            .onChange(of: availabilityFilter) { _, _ in publishFolderState() }
            .onChange(of: selecting) { _, _ in publishFolderState() }
            .onChange(of: desktopSort) { _, _ in publishFolderState() }
            .onChange(of: desktopAscending) { _, _ in publishFolderState() }
            .onChange(of: model.parentPathId, initial: true) { _, _ in publishSourceRoot() }
    }

    private var folderPageInputs: some View {
        folderPageCore
            .background {
                if !keyboardCovered {
                    Button("Back in folder", action: folderBack).keyboardShortcut("[", modifiers: .command).hidden()
                    if selecting && !wideLayout {
                        Button("Cancel folder selection", action: finishCompactSelection).keyboardShortcut(.escape, modifiers: []).hidden()
                    }
                }
            }
            .onChange(of: initialEntryStartedAt) { _, value in
                rowEntryStartedAt = value
                risingPathIDs = value == nil ? [] : Set((wideLayout ? desktopItems : visibleItems).prefix(wideLayout ? 16 : 14).map(\.pathId))
            }
            .onChange(of: showNonAudio) { _, _ in risingPathIDs = [] }
            .onChange(of: wideLayout) { _, _ in risingPathIDs = [] }
            .onChange(of: playerVM.currentSong?.id) { _, _ in updateColumnMetrics() }
            .onChange(of: tableTextScale) { _, _ in updateColumnMetrics() }
            .onChange(of: tableDataScale) { _, _ in updateColumnMetrics() }
            .onChange(of: source.pathId) { _, id in model.resetRoot(to: id) }
    }

    private var folderObservedPage: some View {
        folderPageInputs
            .task(id: model.parentPathId) {
                if skipInitialFolderSummary { skipInitialFolderSummary = false; return }
                if let songs = try? await folderSongs(), !Task.isCancelled { updateFolderKeptState(songs) }
            }
            .task(id: model.items.map(\.pathId)) {
                if skipInitialTrackInspection { skipInitialTrackInspection = false; return }
                await refreshTracks()
            }
            .onReceive(LWRefresh.libraryChanges) { _ in
                Task { await refreshTracks() }
            }

    }

    private var folderErrorPresented: Binding<Bool> {
        Binding<Bool>(get: { error != nil }, set: { shown in if !shown { error = nil } })
    }

    private func folderBack() {
        if selecting { finishCompactSelection() }
        if model.canGoBack { navigateBack() }
        else if let onBack { onBack() }
        else { dismiss() }
    }

    private func publishSourceRoot() {
        let current = reconnectedSource ?? source
        onSourceRootChange?(model.parentPathId == current.pathId ? current : nil)
    }

    private func publishFolderState() {
        guard initialAncestryResolved else { return }
        onNavigationStateChange?(WaveFolderNavigationState(browse: model.snapshot,
            availabilityFilter: availabilityFilter, selecting: selecting, desktopSort: desktopSort,
            desktopAscending: desktopAscending, folderTitle: folderTitle, resolvedFolderNames: resolvedFolderNames,
            folderSongs: folderSnapshot, folderOfflineCount: folderOfflineCount, rowSongs: trackSongs,
            availability: availability, lists: lists, pinnedSongIDs: pinnedSongIDs,
            folderKeptOffline: folderKeptOffline, directorySummaries: directorySummaries, columnMetrics: columnMetrics))
        publishSourceRoot()
        onSelectionChange?(wideLayout
            ? Self.savedAudioSongs(orderedPaths: desktopItems, selected: model.selectedPathIds, cachedSongs: trackSongs)
            : (selecting ? compactScope.selectedSongs : []))
    }

    private func navigateIntoFolder(_ item: SourcePath) {
        navigationTask?.cancel()
        navigationTask = Task {
            guard await model.navigateIntoFolder(with: item.pathId), !Task.isCancelled else { return }
            folderTitle = item.name
            resolvedFolderNames = item.relativePath.split(separator: "/").map(String.init)
            commitFolderEntry(.push)
        }
    }

    private func navigateBack() {
        navigationTask?.cancel()
        navigationTask = Task {
            guard await model.navigateBack(), !Task.isCancelled else { return }
            if !resolvedFolderNames.isEmpty { resolvedFolderNames.removeLast() }
            folderTitle = resolvedFolderNames.last
            commitFolderEntry(.pop)
        }
    }

    private func commitFolderEntry(_ direction: FolderEntry) {
        selecting = false
        model.selectedPathIds = []
        // This turn follows the guarded rows/history commit; metadata-only refreshes
        // leave revision unchanged and cannot replay navigation or row choreography.
        folderSnapshot = []
        folderOfflineCount = 0
        directorySummaries = [:]
        hoveredPathID = nil
        selectionAnchor = nil
        folderEntry = direction
        desktopSort = ""
        desktopAscending = true
        rowEntryStartedAt = wideLayout && direction == .pop ? nil : Date()
        risingPathIDs = wideLayout && direction == .pop ? [] : Set((wideLayout ? desktopItems : visibleItems).prefix(wideLayout ? 16 : 14).map(\.pathId))
        folderRevision = UUID()
    }

    private var displayedRisingPathIDs: Set<Int64> {
        if skipInitialBrowseRead, rowEntryStartedAt != nil {
            return Set((wideLayout ? desktopItems : visibleItems).prefix(wideLayout ? 16 : 14).map(\.pathId))
        }
        return risingPathIDs
    }

    private var mobileFolder: some View {
        let rowRiseIDs = displayedRisingPathIDs
        return VStack(spacing: 0) {
            if selecting {
                LWCompactSelectionHeader(count: compactScope.selectedSongs.count,
                    allSelected: !compactScope.songs.isEmpty && compactScope.selectedSongs.count == compactScope.songs.count,
                    onToggleAll: toggleAllCompactSongs, onDone: finishCompactSelection, title: displayedFolderTitle)
                    .accessibilityIdentifier("folder.selectionHeader")
            } else {
                HStack {
                    Button { folderBack() } label: {
                        HStack(spacing: 4) { LWIcon("back", size: 18); Text(backTitle).lwLineBox(15, 21) }.font(LWTheme.text(15)).foregroundStyle(LWTheme.secondary).frame(minHeight: 44)
                    }.accessibilityIdentifier("folder.back")
                    Spacer()
                    Text("\(model.items.count) items").font(LWTheme.data(12)).foregroundStyle(LWTheme.secondary)
                    if !folderSnapshot.isEmpty {
                        Button("Select", action: beginCompactSelection).frame(minHeight: 44).accessibilityIdentifier("folder.select")
                    }
                }.padding(.horizontal, 16)
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    if !selecting {
                    HStack(alignment: .top) {
                        Text(displayedFolderTitle)
                            .font(LWTheme.text(32).weight(.medium)).tracking(-1.12).lwLineBox(32, 33.6).lineLimit(1).frame(height: folderHeadingHeight).accessibilityIdentifier("folder.title")
                        Spacer(minLength: 12)
                        Button { folderOptions = true } label: {
                            LWIcon("more", size: 22).frame(width: 44, height: 44)
                        }.accessibilityLabel("Folder options").buttonStyle(LWIconButtonStyle()).padding(.trailing, -10)
                            .lwContextMenu { administrationMenu() }
                    }
                    }
                    HStack(spacing: 6) {
                        Text(URL(fileURLWithPath: source.dirPath).lastPathComponent)
                        ForEach(Array(resolvedFolderNames.enumerated()), id: \.offset) { _, name in
                            Text("/"); Text(name)
                        }
                    }.font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lineLimit(1)
                    HStack(spacing: 8) {
                        Button { Task { await playFolder(shuffled: false, useSelection: false) } } label: {
                            HStack(spacing: 8) { LWIcon("play", size: 18); Text("Play folder").lwLineBox(15, 21) }
                                .frame(maxWidth: .infinity).frame(minHeight: 52).background(LWTheme.ink).foregroundStyle(LWTheme.paper)
                        }
                        Button { Task { await playFolder(shuffled: true, useSelection: false) } } label: {
                            LWIcon("shuf", size: 20).frame(width: 52, height: 52).background(LWTheme.key2)
                        }.accessibilityLabel("Shuffle folder")
                        Button { Task { await toggleFolderOffline() } } label: {
                            LWIcon(folderKeptOffline ? "check" : "dl", size: 20)
                                .frame(width: 52, height: 52)
                                .background(folderKeptOffline ? LWTheme.ink : LWTheme.key2)
                                .foregroundStyle(folderKeptOffline ? LWTheme.paper : LWTheme.ink)
                        }.accessibilityLabel(folderKeptOffline ? "Kept offline" : "Keep folder offline")
                            .disabled(changingOffline)
                    }.font(LWTheme.text(15).weight(.medium)).buttonStyle(LWKeyStyle())
                        .disabled(model.isImporting || preparingPlayback || model.items.isEmpty)
                    if model.isImporting {
                        WaveSourceScanBar(progress: progress / 100)
                        Text(currentName).font(LWTheme.data(12)).lineLimit(1)
                        Button("Cancel import") { Task { await dependencies.songImportService.cancelImport() } }
                    }
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 20) {
                            ForEach(["All", "Offline", "In cloud"], id: \.self) { filter in
                                Button { risingPathIDs = []; availabilityFilter = filter } label: {
                                    HStack(spacing: 6) {
                                        Text(filter).font(LWTheme.text(15)).lwLineBox(15, 21)
                                        Text("\(filterCount(filter))").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
                                    }.fixedSize().frame(minHeight: 44)
                                        .foregroundStyle(availabilityFilter == filter ? LWTheme.ink : LWTheme.secondary)
                                        .overlay(alignment: .bottom) { if availabilityFilter == filter { Rectangle().fill(LWTheme.ink).frame(height: 2) } }
                                }.accessibilityLabel("\(filter) \(filterCount(filter))").accessibilityAddTraits(availabilityFilter == filter ? .isSelected : [])
                            }
                        }
                    }
                    if let message = model.errorMessage {
                        Text(message).foregroundStyle(LWTheme.signal).font(LWTheme.text(13))
                        Button("Retry reading folder") { Task { await model.loadItems() } }.frame(minHeight: 44)
                    } else if model.items.isEmpty { Text("No files in this folder.").foregroundStyle(LWTheme.secondary).padding(.vertical, 24) }
                    LazyVStack(spacing: 0) {
                        ForEach(Array(visibleItems.enumerated()), id: \.element.pathId) { index, item in
                            Group {
                                if let song = trackSongs[item.pathId] {
                                    folderRow(item).modifier(LWOrderedSongDrag(title: song.title, ids: { folderDragIDs(item) }))
                                } else { folderRow(item) }
                            }.modifier(WaveFilesRise(index: index, enabled: rowRiseIDs.contains(item.pathId), startedAt: rowEntryStartedAt))
                        }
                    }.padding(.horizontal, -16)
                    if !showNonAudio {
                        let count = model.items.filter { !$0.isDirectory && !$0.isAudio }.count
                        if count > 0 {
                            HStack(spacing: 4) {
                                Text("\(count) non-audio file\(count == 1 ? "" : "s") hidden ·")
                                Button("show") { showNonAudio = true }.underline()
                            }.font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary).padding(.vertical, 16)
                        }
                    }
                }.padding(16)
            }
        }
        .safeAreaInset(edge: .bottom, spacing: 0) { if selecting { compactSelectionDeck } }
    }

    private var displayedFolderTitle: String {
        #if DEBUG
        if model.parentPathId == source.pathId, let display = LWReferenceFixture.sourceDisplay(source: source) { return display.name }
        #endif
        return folderTitle ?? model.folderNames.last ?? URL(fileURLWithPath: source.dirPath).lastPathComponent
    }

    private var desktopFolder: some View {
        GeometryReader { bounds in
        let tableWidth = max(680, bounds.size.width)
        let rowRiseIDs = displayedRisingPathIDs
        let widths = desktopColumnWidths(available: tableWidth)
        ScrollView([.horizontal, .vertical]) {
            VStack(alignment: .leading, spacing: 0) {
                if model.canGoBack || model.parentPathId != (reconnectedSource ?? source).pathId {
                    Button { folderBack() } label: {
                        HStack(spacing: 4) { LWIcon("back", size: 16); Text("Back") }
                            .font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary)
                    }.accessibilityIdentifier("folder.back").padding(.leading, 26).padding(.top, 16).frame(height: 40, alignment: .top)
                }
                if let message = (reconnectedSource ?? source).syncError {
                    HStack(spacing: 12) {
                        Rectangle().strokeBorder(LWTheme.signal, lineWidth: 1.5).frame(width: 10, height: 10)
                        Text(message).font(LWTheme.text(14)).frame(maxWidth: .infinity, alignment: .leading)
                        Button("Reconnect folder") { reconnectPicker = true }
                            .font(LWTheme.text(15).weight(.medium)).padding(.horizontal, 14).frame(height: 36)
                            .background(LWTheme.ink).foregroundStyle(LWTheme.onInk).buttonStyle(LWKeyStyle())
                    }.padding(.horizontal, 16).padding(.vertical, 14)
                        .overlay { Rectangle().strokeBorder(LWTheme.ink, lineWidth: 1.5) }
                        .padding(.horizontal, 32).padding(.top, 20)
                }
                VStack(alignment: .leading, spacing: 0) {
                    if model.parentPathId != (reconnectedSource ?? source).pathId {
                        Text(([URL(fileURLWithPath: source.dirPath).lastPathComponent] + resolvedFolderNames).joined(separator: " / "))
                            .font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary).lwLineBox(14, 19.6).padding(.top, 6).padding(.bottom, 8)
                    }
                    Text(displayedFolderTitle).accessibilityIdentifier("folder.title").font(LWTheme.text(34).weight(.medium)).tracking(-0.85).lwLineBox(34, 34).lineLimit(1).frame(height: desktopHeadingHeight).padding(.top, model.parentPathId == (reconnectedSource ?? source).pathId ? 8 : 0)
                    Text("\(folderSnapshot.count) songs · \(folderBytes(folderSnapshot)) · \(folderOfflineCount) offline")
                        .font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary).lwLineBox(14, 19.6).padding(.top, 6)
                }.padding(.horizontal, 32).padding(.top, 28).padding(.bottom, 18)
                HStack(spacing: 8) {
                    desktopTool("Play folder", icon: "play") { Task { await playFolder(shuffled: false, useSelection: false) } }
                    desktopTool("Shuffle", icon: "shuf") { Task { await playFolder(shuffled: true, useSelection: false) } }
                    desktopTool(folderKeptOffline ? "Kept offline" : "Keep offline", icon: folderKeptOffline ? "check" : "dl") { Task { await toggleFolderOffline() } }
                    Spacer(minLength: 0)
                    HStack(spacing: 6) {
                        Rectangle().fill(LWTheme.ink).frame(width: 10, height: 10)
                            .overlay { Rectangle().strokeBorder(LWTheme.ink, lineWidth: 1.5).padding(-3.5) }
                        Text("kept offline").lwLineBox(12, 16.8)
                        Rectangle().fill(LWTheme.ink).frame(width: 10, height: 10)
                        Text("on this Mac").lwLineBox(12, 16.8)
                        Rectangle().strokeBorder(LWTheme.secondary, lineWidth: 1.5).frame(width: 10, height: 10)
                        Text("in cloud").lwLineBox(12, 16.8)
                    }.font(LWTheme.text(12)).foregroundStyle(LWTheme.secondary).fixedSize()
                }.disabled(model.isImporting || preparingPlayback || model.items.isEmpty)
                    .padding(.horizontal, 32).padding(.bottom, 15)
                    .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1) }
                if model.isImporting {
                    WaveSourceScanBar(progress: progress / 100).padding(.horizontal, 32)
                    Button("Cancel import") { Task { await dependencies.songImportService.cancelImport() } }.padding(.horizontal, 32)
                }
                if let message = model.errorMessage {
                    Text(message).font(LWTheme.text(13)).foregroundStyle(LWTheme.signal).padding(32)
                }
                LazyVStack(spacing: 0, pinnedViews: [.sectionHeaders]) {
                    Section {
                        ForEach(Array(desktopItems.enumerated()), id: \.element.pathId) { index, item in
                            Group {
                                if let song = trackSongs[item.pathId] {
                                    desktopFolderRow(item, index: index, widths: widths).modifier(LWOrderedSongDrag(title: song.title, ids: { folderDragIDs(item) }))
                                } else { desktopFolderRow(item, index: index, widths: widths) }
                            }.modifier(WaveFilesRise(index: index, enabled: rowRiseIDs.contains(item.pathId), startedAt: rowEntryStartedAt))
                        }
                    } header: { desktopFolderHeading(widths: widths) }
                }
                if model.items.isEmpty { Text("Nothing here.").font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary).padding(32) }
            }.frame(width: tableWidth, alignment: .leading)
        }.lwContextMenu { administrationMenu() }
        }
    }

    private var desktopItems: [SourcePath] {
        Self.sortPaths(model.items, songs: trackSongs, column: desktopSort, ascending: desktopAscending)
    }

    static func sortPaths(_ paths: [SourcePath], songs: [Int64: Song], column: String, ascending: Bool) -> [SourcePath] {
        let folders = paths.filter(\.isDirectory)
        let files = paths.filter(\.isAudio)
        let otherFiles = paths.filter { !$0.isDirectory && !$0.isAudio }
        guard !column.isEmpty else { return folders + files + otherFiles }
        return folders + files.sorted { first, second in
            let order: ComparisonResult
            switch column {
            case "Size":
                let a = songs[first.pathId]?.fileSize ?? 0, b = songs[second.pathId]?.fileSize ?? 0
                order = a == b ? .orderedSame : a < b ? .orderedAscending : .orderedDescending
            case "Time":
                let a = songs[first.pathId]?.duration ?? 0, b = songs[second.pathId]?.duration ?? 0
                order = a == b ? .orderedSame : a < b ? .orderedAscending : .orderedDescending
            default: order = first.name.localizedStandardCompare(second.name)
            }
            return ascending ? order == .orderedAscending : order == .orderedDescending
        } + otherFiles
    }

    private func desktopFolderHeading(widths: [CGFloat]) -> some View {
        HStack(spacing: 0) {
            Button { risingPathIDs = []; desktopSort = "" } label: { Text("#").lwLineBox(12, 16.8) }.padding(.leading, 32).padding(.trailing, 8).frame(width: widths[0], alignment: .leading)
            desktopSortButton("Name").padding(.horizontal, 8).frame(width: widths[1], alignment: .leading)
            Text("Kind").lwLineBox(12, 16.8).padding(.horizontal, 8).frame(width: widths[2], alignment: .leading)
            desktopSortButton("Size").padding(.horizontal, 8).frame(width: widths[3], alignment: .leading)
            Text("On device").lwLineBox(12, 16.8).padding(.horizontal, 8).frame(width: widths[4], alignment: .leading)
            desktopSortButton("Time").padding(.leading, 8).padding(.trailing, 20).frame(width: widths[5], alignment: .trailing)
        }.font(LWTheme.text(12).weight(.medium)).foregroundStyle(LWTheme.secondary)
            // CSS th: 10px vertical padding, then the table separator occupies its own pixel.
            .padding(.vertical, 10).padding(.bottom, 1)
            .background(LWTheme.paper)
            .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1) }
    }

    private func desktopSortButton(_ title: String) -> some View {
        Button {
            risingPathIDs = []
            if desktopSort != title { desktopSort = title; desktopAscending = true }
            else if desktopAscending { desktopAscending = false }
            else { desktopSort = "" }
        } label: {
            Text(title + (desktopSort == title ? desktopAscending ? " ↑" : " ↓" : "")).lwLineBox(12, 16.8)
        }.accessibilityLabel("Sort by " + title)
            .accessibilityValue(desktopSort == title ? desktopAscending ? "Ascending" : "Descending" : "Original order")
    }

    private func desktopTool(_ title: String, icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 6) { LWIcon(icon, size: 16); Text(title).lwLineBox(13, 18.2) }
                .font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).padding(.horizontal, 10).frame(height: 30)
        }
    }

    private func desktopFolderRow(_ item: SourcePath, index: Int, widths: [CGFloat]) -> some View {
        let song = trackSongs[item.pathId]
        let current = song != nil && song?.id == playerVM.currentSong?.id
        let summary = directorySummaries[item.pathId] ?? DirectorySummary(count: 0, bytes: 0)
        let row = HStack(spacing: 0) {
            Group {
                if item.isDirectory { LWIcon("folder", size: 16) }
                else if !item.isAudio { LWIcon("file", size: 14) }
                else if current { LWEqualizer(playing: playerVM.isPlaying) }
                else { Text("\(index + 1 - model.items.filter(\.isDirectory).count)").font(LWTheme.text(13)).lwLineBox(13, 18.2) }
            }.padding(.leading, 32).padding(.trailing, 8).frame(width: widths[0], alignment: .leading).foregroundStyle(LWTheme.secondary)
            Text(item.name).lwLineBox(14, 19.6).lineLimit(1).padding(.horizontal, 8).frame(width: widths[1], alignment: .leading)
            Text(item.isDirectory ? "Folder · \(summary.count) songs" : item.isAudio ? "\(song?.fileExtension ?? URL(fileURLWithPath: item.name).pathExtension) audio" : "Not audio")
                .foregroundStyle(LWTheme.secondary).lwLineBox(14, 19.6).lineLimit(1).padding(.horizontal, 8).frame(width: widths[2], alignment: .leading)
            Text(item.isDirectory ? byteLabel(summary.bytes) : (song?.fileSize ?? item.fileSize).map { String(format: "%.1f MB", Double($0) / 1_048_576) } ?? "—")
                .font(LWTheme.data(12)).foregroundStyle(LWTheme.secondary).lwLineBox(12, 16.8, data: true).padding(.horizontal, 8).frame(width: widths[3], alignment: .leading)
            ZStack(alignment: .leading) { Color.clear; if item.isAudio { availabilityGlyph(item) } }
                .frame(height: 10).padding(.horizontal, 8).frame(width: widths[4], alignment: .leading)
            Text(song?.duration.map { String(format: "%d:%02d", Int($0) / 60, Int($0) % 60) } ?? "")
                .font(LWTheme.data(12)).foregroundStyle(LWTheme.secondary).lwLineBox(12, 16.8, data: true).padding(.leading, 8).padding(.trailing, 20).frame(width: widths[5], alignment: .trailing)
        }.font(LWTheme.text(14)).fontWeight(current ? .medium : .regular).frame(minHeight: 38)
            .background(model.selectedPathIds.contains(item.pathId) ? LWTheme.panel2 : hoveredPathID == item.pathId ? LWTheme.panel.opacity(0.6) : Color.clear)
            .opacity(item.isDirectory || item.isAudio ? 1 : 0.45)
            .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1) }
        return Group {
            if item.isDirectory || item.isAudio {
            row.contentShape(Rectangle()).onHover { hoveredPathID = $0 ? item.pathId : nil }
            .gesture(WaveModifierCapture { modifierKeys = $0 })
            .onTapGesture(count: 2) {
                if item.isDirectory { navigateIntoFolder(item) }
                else if item.isAudio { Task { await play(item) } }
            }
            .onTapGesture { selectPath(item.pathId) }
            .lwContextMenu { folderMenu(item) }
            .accessibilityAction(named: Text(model.selectedPathIds.contains(item.pathId) ? "Deselect" : "Select")) {
                selecting = true; model.toggleSelection(item.pathId)
            }
            .accessibilityElement(children: .combine)
            .accessibilityAction {
                if item.isDirectory { navigateIntoFolder(item) }
                else if item.isAudio { Task { await play(item) } }
            }
            } else { row.accessibilityElement(children: .combine) }
        }
    }

    private func selectPath(_ id: Int64) {
        let order = desktopItems.filter { $0.isDirectory || $0.isAudio }.map(\.pathId)
        if modifierKeys.contains(.shift), let selectionAnchor, let start = order.firstIndex(of: selectionAnchor), let end = order.firstIndex(of: id) {
            model.selectedPathIds.formUnion(order[min(start, end)...max(start, end)])
        } else if modifierKeys.contains(.command) {
            model.toggleSelection(id)
            selectionAnchor = id
        } else { model.selectedPathIds = [id]; selectionAnchor = id }
    }

    private func desktopColumnWidths(available: CGFloat) -> [CGFloat] {
        var preferred = columnMetrics.count == 6 ? columnMetrics : [44, 0, 0, 0, 0, 0]
        for (index, label) in ["#", "Name", "Kind", "Size", "On device", "Time"].enumerated() {
            let title = label + (desktopSort == label ? desktopAscending ? " ↑" : " ↓" : "")
            let padding: CGFloat = index == 0 ? 40 : index == 5 ? 28 : 16
            preferred[index] = max(preferred[index], Self.textWidth(title, face: "HostGrotesk-Medium", size: 12 * tableTextScale) + padding)
        }
        return Self.layoutColumns(preferred: preferred, available: available)
    }

    static func layoutColumns(preferred: [CGFloat], available: CGFloat) -> [CGFloat] {
        guard preferred.count == 6 else { return [] }
        let remainder = max(0, available - preferred[0])
        let total = max(1, preferred.dropFirst().reduce(0, +))
        return [preferred[0]] + preferred.dropFirst().map { $0 * remainder / total }
    }

    private static func textWidth(_ text: String, face: String, size: CGFloat) -> CGFloat {
        let font = CTFontCreateWithName(face as CFString, size, nil)
        let line = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: [NSAttributedString.Key(kCTFontAttributeName as String): font]))
        return ceil(CGFloat(CTLineGetTypographicBounds(line, nil, nil, nil)) * 64) / 64
    }

    private func updateColumnMetrics() {
        var widths: [CGFloat] = [44, 0, 0, 0, 10 + 16, 0]
        var number = 0
        for item in model.items {
            let song = trackSongs[item.pathId]
            let current = song != nil && song?.id == playerVM.currentSong?.id
            let face = current ? "HostGrotesk-Medium" : "HostGrotesk-Regular"
            let dataFace = current ? "GeistMono-Medium" : "GeistMono-Regular"
            let summary = directorySummaries[item.pathId] ?? DirectorySummary(count: 0, bytes: 0)
            if item.isAudio { number += 1 }
            if item.isDirectory { widths[0] = max(widths[0], 56) }
            else if !item.isAudio || current { widths[0] = max(widths[0], 54) }
            else { widths[0] = max(widths[0], Self.textWidth(String(number), face: face, size: 13 * tableTextScale) + 40) }
            widths[1] = max(widths[1], Self.textWidth(item.name, face: face, size: 14 * tableTextScale) + 16)
            let kind = item.isDirectory ? "Folder · \(summary.count) songs" : item.isAudio ? "\(song?.fileExtension ?? URL(fileURLWithPath: item.name).pathExtension) audio" : "Not audio"
            widths[2] = max(widths[2], Self.textWidth(kind, face: face, size: 14 * tableTextScale) + 16)
            let size = item.isDirectory ? byteLabel(summary.bytes) : (song?.fileSize ?? item.fileSize).map { String(format: "%.1f MB", Double($0) / 1_048_576) } ?? "—"
            widths[3] = max(widths[3], Self.textWidth(size, face: dataFace, size: 12 * tableDataScale) + 16)
            if let duration = song?.duration {
                widths[5] = max(widths[5], Self.textWidth(String(format: "%d:%02d", Int(duration) / 60, Int(duration) % 60), face: dataFace, size: 12 * tableDataScale) + 28)
            }
        }
        columnMetrics = widths
    }

    private func byteLabel(_ bytes: Int64) -> String {
        bytes >= 1_073_741_824 ? String(format: "%.1f GB", Double(bytes) / 1_073_741_824) : String(format: "%.1f MB", Double(bytes) / 1_048_576)
    }

    private func folderBytes(_ songs: [Song]) -> String {
        let bytes = songs.compactMap(\.fileSize).reduce(Int64(0), +)
        return byteLabel(bytes)
    }

    private func importSelection() async {
        model.isImporting = true
        defer {
            model.isImporting = false
            NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
            NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
        }
        do {
            let paths: [SourcePath]
            if model.selectedPathIds.isEmpty { paths = model.items }
            else {
                paths = try await dependencies.sourceService.importService().listItems(sourceId: source.id ?? -1, parentPathId: nil)
                    .filter { model.selectedPathIds.contains($0.pathId) }
            }
            try await dependencies.songImportService.importPathsWithFileProgress(paths: paths) { event in
                await MainActor.run { progress = event.percentage; currentName = event.url.lastPathComponent }
            }
            model.selectedPathIds = []
        } catch is CancellationError { } catch { self.error = error.localizedDescription }
    }

    private func play(_ item: SourcePath) async {
        guard loadingPathID == nil, !model.isImporting else { return }
        loadingPathID = item.pathId
        defer { loadingPathID = nil }
        do {
            let song = try await song(for: item)
            var queue: [Song] = []
            for path in model.items where !path.isDirectory {
                if let queuedSong = await dependencies.songRepository.getSong(sourceId: path.sourceId, relativePath: path.relativePath) { queue.append(queuedSong) }
            }
            playerVM.configureQueue(songs: queue.isEmpty ? [song] : queue, startIndex: queue.firstIndex(where: { $0.id == song.id }) ?? 0,
                context: .folder(sourceID: source.id ?? -1, pathID: model.parentPathId, name: folderTitle ?? model.folderNames.last ?? URL(fileURLWithPath: source.dirPath).lastPathComponent))
            playerVM.playSong(song)
        } catch { self.error = error.localizedDescription }
    }

    @ViewBuilder private func folderRow(_ item: SourcePath) -> some View {
        if item.isDirectory || item.isAudio { audioFolderRow(item) }
        else {
            HStack(spacing: 12) {
                LWIcon("file", size: 16).frame(width: 16).foregroundStyle(LWTheme.secondary)
                VStack(alignment: .leading, spacing: 0) {
                    Text(item.name).font(LWTheme.text(15)).lwLineBox(15, 21).frame(minHeight: 21)
                    Text("not audio" + (item.fileSize.map { String(format: " · %.1f MB", Double($0) / 1_048_576) } ?? ""))
                        .font(LWTheme.data(12)).foregroundStyle(LWTheme.secondary).lwLineBox(12, 16.8, data: true).frame(minHeight: 16.8)
                }.frame(maxWidth: .infinity, alignment: .leading)
            }.padding(.leading, 16).padding(.trailing, 6).frame(minHeight: 56).opacity(0.45)
                .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.leading, 44) }
                .accessibilityElement(children: .combine)
        }
    }

    private func audioFolderRow(_ item: SourcePath) -> some View {
        HStack(spacing: 12) {
            Group {
                if item.isDirectory { LWIcon("folder", size: 18).foregroundStyle(LWTheme.secondary) }
                else if trackSongs[item.pathId]?.id == playerVM.currentSong?.id, trackSongs[item.pathId] != nil { LWEqualizer(playing: playerVM.isPlaying) }
                else { availabilityGlyph(item) }
            }.frame(width: 16)
            Button {
                if selecting && item.isAudio { toggleCompactSong(item) }
                else if item.isDirectory { navigateIntoFolder(item) }
                else { Task { await play(item) } }
            } label: {
                VStack(alignment: .leading, spacing: 0) {
                    Text(item.name).font(LWTheme.text(15)).lwLineBox(15, 21).lineLimit(1).frame(minHeight: 21)
                    Text(folderFileDetail(item)).font(LWTheme.data(12)).foregroundStyle(LWTheme.secondary).lwLineBox(12, 16.8, data: true).lineLimit(1).frame(minHeight: 16.8)
                }.frame(maxWidth: .infinity, alignment: .leading).frame(minHeight: 56)
            }.accessibilityIdentifier(item.isDirectory ? "folder.directory." + item.name : "folder.audio." + item.name)
            if selecting && compactScope.pathIDs.contains(item.pathId) {
                Button { toggleCompactSong(item) } label: {
                    ZStack {
                        Rectangle().fill(model.selectedPathIds.contains(item.pathId) ? LWTheme.ink : Color.clear)
                        if model.selectedPathIds.contains(item.pathId) { LWIcon("check", size: 14).foregroundStyle(LWTheme.onInk) }
                        else { Rectangle().strokeBorder(LWTheme.secondary, lineWidth: 1.5) }
                    }.frame(width: 20, height: 20).frame(width: 44, height: 44)
                }.accessibilityLabel((model.selectedPathIds.contains(item.pathId) ? "Deselect " : "Select ") + item.name)
                    .accessibilityValue(model.selectedPathIds.contains(item.pathId) ? "Selected" : "Not selected")
            }
            if loadingPathID == item.pathId { ProgressView().tint(LWTheme.signal) }
            LWMenuButton(request: { folderMenu(item) }) {
                LWIcon("more", size: 18).frame(width: 44, height: 44)
            }.accessibilityLabel("Actions for \(item.name)")
                .accessibilityValue(trackSongs[item.pathId]?.isFavorite == 1 ? "Favorite" : "Not favorite")
                .id("folder-actions-\(item.pathId)-\(trackSongs[item.pathId]?.isFavorite ?? 0)-\(trackSongs[item.pathId]?.fileState.rawValue ?? -1)-\(trackSongs[item.pathId].map { pinnedSongIDs.contains(String($0.uniqueId)) } ?? false)")
        }.padding(.leading, 16).padding(.trailing, 6)
            .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.leading, 44) }
            .lwContextMenu { folderMenu(item) }
            .accessibilityActions {
                if compactScope.pathIDs.contains(item.pathId) {
                    Button(model.selectedPathIds.contains(item.pathId) ? "Deselect" : "Select") {
                        if !selecting { beginCompactSelection() }
                        toggleCompactSong(item)
                    }
                }
            }
    }

    private func folderFileDetail(_ item: SourcePath) -> String {
        guard let song = trackSongs[item.pathId] else { return item.isDirectory ? "Folder" : URL(fileURLWithPath: item.name).pathExtension.uppercased() }
        var parts: [String] = []
        if let format = song.fileExtension { parts.append(format.uppercased()) }
        if let bytes = song.fileSize { parts.append(String(format: "%.1f MB", Double(bytes) / 1_048_576)) }
        if let duration = song.duration { parts.append(String(format: "%d:%02d", Int(duration) / 60, Int(duration) % 60)) }
        return parts.joined(separator: " · ")
    }

    private func refreshTracks() async {
        let items = model.items
        var tracks: [Int64: Song] = [:]
        var states: [Int64: FileAvailability] = [:]
        for item in items where item.isAudio {
            guard !Task.isCancelled else { return }
            if let song = await dependencies.songRepository.getSong(sourceId: item.sourceId, relativePath: item.relativePath) {
                tracks[item.pathId] = song
                states[item.pathId] = await dependencies.sourceAccess.availability(for: song)
            }
        }
        guard items.map(\.pathId) == model.items.map(\.pathId) else { return }
        trackState.songs = tracks
        availability = states
        pinnedSongIDs = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
        lists = (try? await dependencies.playlistRepo.getAll()) ?? []
        if let songs = try? await folderSongs(), !Task.isCancelled, model.items.map(\.pathId) == items.map(\.pathId) {
            updateFolderKeptState(songs)
            var local = 0
            let pathIDsBySong = Dictionary(grouping: trackSongs.keys, by: { trackSongs[$0]?.id ?? -1 })
            for song in songs {
                guard !Task.isCancelled else { return }
                let proof = await dependencies.sourceAccess.availability(for: song)
                if proof == .onDevice { local += 1 }
                for pathID in pathIDsBySong[song.uniqueId] ?? [] { states[pathID] = proof }
            }
            guard items.map(\.pathId) == model.items.map(\.pathId), !Task.isCancelled else { return }
            availability = states
            folderOfflineCount = local
        }
        publishFolderState()
    }

    private var visibleItems: [SourcePath] {
        model.items.filter { item in
            if item.isDirectory { return true }
            if !item.isAudio { return showNonAudio && availabilityFilter == "All" }
            return availabilityFilter == "All" || availability[item.pathId] == (availabilityFilter == "Offline" ? .onDevice : .inCloud)
        }
    }

    private func filterCount(_ filter: String) -> Int {
        model.items.filter { item in
            item.isAudio && (filter == "All" || availability[item.pathId] == (filter == "Offline" ? .onDevice : .inCloud))
        }.count
    }

    private func availabilityGlyph(_ item: SourcePath) -> some View {
        let state = availability[item.pathId] ?? .unknown
        let song = trackSongs[item.pathId]
        let pinned = song.map { pinnedSongIDs.contains(String($0.uniqueId)) } ?? false
        let kept = pinned && song?.fileState == .copied && state == .onDevice
        let pending = song?.fileState == .copyPending
        let failed = song?.fileState == .failed
        let label = failed ? "Download failed" : pending ? "Queued for download" : kept ? "Kept offline" : state == .onDevice ? "On this device" : state == .inCloud ? "In cloud" : "Availability unknown"
        return LWStateGlyph(state: song?.fileState ?? .bookmarkOnly, availability: state, pinned: pinned)
            .accessibilityLabel(label)
    }

    private func song(for item: SourcePath) async throws -> Song {
        guard item.isAudio else { throw CustomError.genericError("This file is not supported audio.") }
        if let saved = await dependencies.songRepository.getSong(sourceId: item.sourceId, relativePath: item.relativePath) { return saved }
        try await dependencies.songImportService.importPaths(paths: [item], onProgress: nil)
        guard let saved = await dependencies.songRepository.getSong(sourceId: item.sourceId, relativePath: item.relativePath) else {
            throw CustomError.genericError("This file could not be imported. Reconnect its folder or retry the import.")
        }
        NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        await refreshTracks()
        return saved
    }

    private func playFolder(shuffled: Bool, folder: SourcePath? = nil, useSelection: Bool = true) async {
        guard !preparingPlayback else { return }
        preparingPlayback = true
        model.isImporting = true
        defer {
            preparingPlayback = false
            model.isImporting = false
            NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        }
        do {
            let all = try await dependencies.sourceService.importService().listItems(sourceId: source.id ?? -1, parentPathId: nil)
            let prefix = folder?.relativePath ?? all.first(where: { $0.pathId == model.parentPathId })?.relativePath ?? ""
            let selected = folder == nil && useSelection ? all.filter { model.selectedPathIds.contains($0.pathId) } : []
            let paths = all.filter { path in
                path.isAudio && (selected.isEmpty
                    ? (prefix.isEmpty || path.relativePath.hasPrefix(prefix + "/"))
                    : selected.contains { $0.pathId == path.pathId || ($0.isDirectory && path.relativePath.hasPrefix($0.relativePath + "/")) })
            }.sorted { $0.relativePath.localizedStandardCompare($1.relativePath) == .orderedAscending }
            var missing: [SourcePath] = []
            for path in paths {
                if await dependencies.songRepository.getSong(sourceId: path.sourceId, relativePath: path.relativePath) == nil { missing.append(path) }
            }
            if !missing.isEmpty {
                try await dependencies.songImportService.importPathsWithFileProgress(paths: missing) { event in
                    await MainActor.run { progress = event.percentage; currentName = event.url.lastPathComponent }
                }
            }
            var songs: [Song] = []
            for path in paths {
                if let saved = await dependencies.songRepository.getSong(sourceId: path.sourceId, relativePath: path.relativePath) { songs.append(saved) }
            }
            guard !songs.isEmpty else { throw CustomError.genericError("No audio files in this folder.") }
            if shuffled { songs.shuffle() }
            playerVM.configureQueue(songs: songs, startIndex: 0, context: .folder(sourceID: source.id ?? -1, pathID: folder?.pathId ?? model.parentPathId, name: folder?.name ?? folderTitle ?? model.folderNames.last ?? URL(fileURLWithPath: source.dirPath).lastPathComponent))
            playerVM.playQueueEntry(at: 0)
        } catch is CancellationError { } catch { self.error = error.localizedDescription }
    }

    private func administrationMenu() -> LWMenuRequest {
        LWMenuRequest(groups: [LWMenuGroup([
            LWMenuAction(title: model.selectedPathIds.isEmpty ? "Import folder" : "Import selected", icon: "folder", disabled: model.isImporting || model.items.isEmpty, action: { Task { await importSelection() } })
        ])])
    }

    private func currentFolderSheetActions() -> [LWMenuAction] {
        let currentSource = reconnectedSource ?? source
        var actions = [
            LWMenuAction(title: "Keep this folder offline", icon: "dl", disabled: changingOffline || model.isImporting || folderSnapshot.isEmpty, action: { Task { await toggleFolderOffline() } }),
            LWMenuAction(title: "Rescan now", icon: "rep", action: { dependencies.waveSyncViewModel.resyncSource(currentSource) }),
            LWMenuAction(title: "Add folder to queue", icon: "queue", disabled: folderSnapshot.isEmpty, action: {
                folderSnapshot.sorted { ($0.relativePath ?? $0.title).localizedStandardCompare($1.relativePath ?? $1.title) == .orderedAscending }.forEach { playerVM.addToQueue($0) }
            })
        ]
        if model.parentPathId == currentSource.pathId {
            actions.append(LWMenuAction(title: "Remove source (files are not deleted)", icon: "close", action: { confirmRemoveCurrentSource() }))
        }
        return actions
    }

    private func confirmRemoveCurrentSource() {
        let currentSource = reconnectedSource ?? source
        guard let id = currentSource.id else { return }
        presenter.present(dialog: LWDialogRequest(title: "Remove \(URL(fileURLWithPath: currentSource.dirPath).lastPathComponent)?",
            message: "Its songs leave the library. Files are not deleted.", primaryTitle: "Remove",
            destructive: true, mobileConfirmation: true, onSubmit: { _ in
                Task {
                    do {
                        if dependencies.waveSyncViewModel.currentSyncSourceId == id { await dependencies.songImportService.cancelImport() }
                        try await dependencies.sourceAccess.remove(sourceId: id)
                        dependencies.waveSyncViewModel.loadSources()
                        onBack?()
                    } catch { self.error = error.localizedDescription }
                }
            }))
    }

    struct ContextTargets {
        let selectedPathIds: Set<Int64>
        let anchor: Int64?
        let songs: [Song]
    }

    /// Match selIds(): selection order is the displayed row order, never Set order.
    /// Only persisted audio rows can participate in batch commands.
    static func contextTargets(clicked: SourcePath, orderedPaths: [SourcePath], selected: Set<Int64>,
                               anchor: Int64?, cachedSongs: [Int64: Song], wide: Bool) -> ContextTargets {
        guard clicked.isAudio else { return ContextTargets(selectedPathIds: selected, anchor: anchor, songs: []) }
        let selectedPaths = wide && !selected.contains(clicked.pathId) ? Set([clicked.pathId]) : selected
        let nextAnchor = wide && !selected.contains(clicked.pathId) ? clicked.pathId : anchor
        let targetPaths = wide ? selectedPaths : Set([clicked.pathId])
        return ContextTargets(selectedPathIds: selectedPaths, anchor: nextAnchor,
            songs: savedAudioSongs(orderedPaths: orderedPaths, selected: targetPaths, cachedSongs: cachedSongs))
    }

    private static func savedAudioSongs(orderedPaths: [SourcePath], selected: Set<Int64>, cachedSongs: [Int64: Song]) -> [Song] {
        var seen = Set<Int64>()
        return orderedPaths.filter { $0.isAudio && selected.contains($0.pathId) }.compactMap { path -> Song? in
            guard let song = cachedSongs[path.pathId], let id = song.id, seen.insert(id).inserted else { return nil }
            return song
        }
    }

    struct CompactSelectionScope {
        let pathIDs: Set<Int64>
        let songs: [Song]
        let selectedSongs: [Song]
    }

    /// Recursive selection is independent of the visible availability filter.
    /// Own files precede child folders, with natural order within each folder.
    static func compactSelectionScope(folderSongs: [Song], cachedSongs: [Int64: Song], selectedPaths: Set<Int64>, blocked: Bool, availability: [Int64: FileAvailability] = [:]) -> CompactSelectionScope {
        let verifiedLocalIDs = Set(cachedSongs.compactMap { entry -> Int64? in
            availability[entry.key] == .onDevice ? entry.value.id : nil
        })
        let cachedIDs = Set(cachedSongs.values.compactMap(\.id))
        var seen = Set<Int64>()
        let songs = folderSongs.filter { song in
            guard let id = song.id, cachedIDs.contains(id), !blocked || verifiedLocalIDs.contains(id) else { return false }
            return seen.insert(id).inserted
        }.sorted { left, right in
            let a = (left.relativePath ?? left.title).split(separator: "/")
            let b = (right.relativePath ?? right.title).split(separator: "/")
            for index in 0..<min(a.count, b.count) {
                let ad = index < a.count - 1, bd = index < b.count - 1
                if ad != bd { return !ad }
                let order = String(a[index]).localizedStandardCompare(String(b[index]))
                if order != .orderedSame { return order == .orderedAscending }
            }
            return (left.id ?? 0) < (right.id ?? 0)
        }
        let allowedIDs = Set(songs.compactMap(\.id))
        let paths = Set(cachedSongs.compactMap { entry -> Int64? in
            guard let id = entry.value.id, allowedIDs.contains(id) else { return nil }
            return entry.key
        })
        let selectedIDs = Set(cachedSongs.filter { selectedPaths.contains($0.key) }.values.compactMap(\.id))
        return CompactSelectionScope(pathIDs: paths, songs: songs, selectedSongs: songs.filter { selectedIDs.contains($0.uniqueId) })
    }

    private var compactScope: CompactSelectionScope {
        Self.compactSelectionScope(folderSongs: folderSnapshot, cachedSongs: trackSongs,
            selectedPaths: model.selectedPathIds, blocked: (reconnectedSource ?? source).syncError != nil, availability: availability)
    }

    private func beginCompactSelection() {
        model.selectedPathIds = []
        selectionAnchor = nil
        selecting = true
        publishFolderState()
    }
    private func finishCompactSelection() {
        selecting = false
        model.selectedPathIds = []
        selectionAnchor = nil
        publishFolderState()
    }
    private func toggleAllCompactSongs() {
        let scope = compactScope
        model.selectedPathIds = !scope.songs.isEmpty && scope.selectedSongs.count == scope.songs.count ? [] : scope.pathIDs
        publishFolderState()
    }
    private func toggleCompactSong(_ item: SourcePath) {
        guard item.isAudio, let id = trackSongs[item.pathId]?.id, compactScope.pathIDs.contains(item.pathId) else { return }
        let aliases = Set(compactScope.pathIDs.filter { trackSongs[$0]?.id == id })
        if !model.selectedPathIds.isDisjoint(with: aliases) { model.selectedPathIds.subtract(aliases) }
        else { model.selectedPathIds.formUnion(aliases) }
        publishFolderState()
    }

    private var compactSelectionDeck: some View {
        let songs = compactScope.selectedSongs
        let empty = songs.isEmpty
        return LWCompactSelectionDeck {
            Button { queueSavedSongs(songs, next: true); finishCompactSelection() } label: {
                LWCompactSelectionKey(title: "Play next", icon: "next2", disabled: empty)
            }.disabled(empty)
            LWMenuButton(request: { LWMenuRequest(groups: savedAddToListGroups(songs, onChoose: finishCompactSelection)) }) {
                LWCompactSelectionKey(title: "Add to list", icon: "plus", disabled: empty)
            }.disabled(empty)
            Button { toggleSavedSongsOffline(songs); finishCompactSelection() } label: {
                LWCompactSelectionKey(title: "Offline", icon: "dl", disabled: empty)
            }.disabled(empty)
            Button { queueSavedSongs(songs, next: false); finishCompactSelection() } label: {
                LWCompactSelectionKey(title: "Queue", icon: "queue", disabled: empty)
            }.disabled(empty)
        }
            .accessibilityIdentifier("folder.selectionDeck")
    }

    private func queueSavedSongs(_ songs: [Song], next: Bool) {
        let count = playerVM.queue.count
        songs.forEach { playerVM.addToQueue($0) }
        if next && count > 0 { playerVM.reorderQueue(from: IndexSet(integersIn: count..<(count + songs.count)), to: min(playerVM.currentQueueIndex + 1, count)) }
    }

    private func toggleSavedSongsOffline(_ songs: [Song]) {
        let allPinned = withOfflineCacheLock {
            var current = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
            let ids = songs.map { String($0.uniqueId) }
            let release = !ids.isEmpty && ids.allSatisfy { current.contains($0) }
            if release { current.subtract(ids) } else { current.formUnion(ids) }
            LocalWavePreferences.current.set(Array(current), forKey: "lwPinnedSongs")
            return release
        }
        performSavedAudioAction {
            if !allPinned {
                for song in songs where song.fileState != .copied {
                    if let id = song.id { try await dependencies.songRepository.markSongForCopy(songId: id) }
                }
            }
        }
    }

    private func savedAudioMenu(_ songs: [Song]) -> LWMenuRequest {
        let allFavorite = !songs.isEmpty && songs.allSatisfy { $0.isFavorite != 0 }
        let pins = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
        let allPinned = !songs.isEmpty && songs.allSatisfy { pins.contains(String($0.uniqueId)) }
        var navigation: [LWMenuAction] = []
        if let song = songs.first, songs.count == 1 {
            navigation.append(LWMenuAction(title: "Go to album", icon: "album", action: {
                NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: PlaybackContext.album(name: song.album, artist: song.albumArtist.isEmpty ? song.artist : song.albumArtist))
            }))
            navigation.append(LWMenuAction(title: "Show in folder", icon: "folder", action: {
                let path = model.items.first { trackSongs[$0.pathId]?.id == song.id }
                let parent = path?.parentPathId ?? model.parentPathId
                let name = song.relativePath?.split(separator: "/").dropLast().last.map(String.init) ?? URL(fileURLWithPath: source.dirPath).lastPathComponent
                NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: PlaybackContext.folder(sourceID: song.sourceId ?? source.id ?? -1, pathID: parent, name: name))
            }))
        }
        navigation.append(LWMenuAction(title: songs.count > 1 ? (allFavorite ? "Unfavorite all" : "Favorite all") : (allFavorite ? "Unfavorite" : "Favorite"), icon: "heart", shortcut: songs.count == 1 ? "L" : nil, disabled: songs.isEmpty, action: {
            performSavedAudioAction {
                for song in songs { if let id = song.id { try await dependencies.songRepository.toggleSongAsFavorite(songId: id, state: !allFavorite) } }
            }
        }))
        var storage = [LWMenuAction(title: allPinned ? "Stop keeping offline" : "Keep offline", icon: allPinned ? "check" : "dl", disabled: songs.isEmpty, action: {
            toggleSavedSongsOffline(songs)
        })]
        if songs.count == 1, let song = songs.first, song.fileState == .failed, let id = song.id {
            storage.append(LWMenuAction(title: "Retry download", icon: "rep", action: {
                performSavedAudioAction { try await dependencies.songRepository.markSongForCopy(songId: id) }
            }))
        }
        storage.append(LWMenuAction(title: "Share…", icon: "share", disabled: songs.isEmpty || sharing, action: { shareSavedSongs(songs) }))
        return LWMenuRequest(groups: [
            LWMenuGroup([
                LWMenuAction(title: songs.count > 1 ? "Play \(songs.count) songs" : "Play", icon: "play", shortcut: "↩", disabled: songs.isEmpty, action: {
                    let queue = songs.count == 1
                        ? Self.savedAudioSongs(orderedPaths: model.items, selected: Set(model.items.map(\.pathId)), cachedSongs: trackSongs)
                        : songs
                    let index = songs.count == 1 ? queue.firstIndex(where: { $0.id == songs.first?.id }) ?? 0 : 0
                    let context: PlaybackContext = songs.count == 1
                        ? .folder(sourceID: source.id ?? -1, pathID: model.parentPathId, name: displayedFolderTitle)
                        : .selection("Selected songs")
                    playerVM.configureQueue(songs: queue, startIndex: index, context: context)
                    playerVM.playQueueEntry(at: index)
                }),
                LWMenuAction(title: "Play next", icon: "next2", disabled: songs.isEmpty, action: {
                    queueSavedSongs(songs, next: true)
                }),
                LWMenuAction(title: "Add to queue", icon: "queue", disabled: songs.isEmpty, action: { queueSavedSongs(songs, next: false) })
            ]),
            LWMenuGroup([LWMenuAction(title: "Add to list", icon: "plus", disabled: songs.isEmpty, submenu: { savedAddToListGroups(songs) })]),
            LWMenuGroup(navigation), LWMenuGroup(storage)
        ], preview: songs.count == 1 ? songs.first.map { AnyView(LWMenuTrackPreview(song: $0, subtitle: $0.artist + " · " + ($0.fileExtension ?? "audio"))) } : nil)
    }

    private func performSavedAudioAction(_ operation: @escaping @MainActor () async throws -> Void) {
        Task {
            do { try await operation() }
            catch { self.error = error.localizedDescription }
            await refreshTracks()
            NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
            NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        }
    }

    private func savedAddToListGroups(_ songs: [Song], onChoose: (() -> Void)? = nil) -> [LWMenuGroup] {
        var actions = lists.compactMap { list -> LWMenuAction? in
            guard let listID = list.id else { return nil }
            return LWMenuAction(title: list.name, icon: "queue", action: {
                onChoose?()
                performSavedAudioAction {
                    try await LWTrackMenu.addToList(songIDs: songs.compactMap(\.id), listID: listID, listName: list.name, dependencies: dependencies)
                }
            })
        }
        actions.append(LWMenuAction(title: "New list…", icon: "plus", action: {
            onChoose?()
            presenter.present(dialog: LWDialogRequest(title: "New list", input: LWDialogInput(placeholder: "List name"), primaryTitle: "Create", onSubmit: { value in
                let name = value.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !name.isEmpty else { return }
                performSavedAudioAction {
                    let created = try await dependencies.playlistRepo.create(playlist: Playlist(id: nil, name: name, createdAt: Date(), updatedAt: nil))
                    guard let listID = created.id else { return }
                    do { for song in songs { if let songID = song.id { try await dependencies.playlistSongRepo.addSong(playlistId: listID, songId: songID) } } }
                    catch { try? await dependencies.playlistRepo.delete(playlistId: listID); throw error }
                }
            }))
        }))
        return [LWMenuGroup(actions)]
    }

    private func shareSavedSongs(_ songs: [Song]) {
        guard !sharing, !songs.isEmpty else { return }
        sharing = true
        Task {
            defer { sharing = false }
            do {
                let value = try await LWSourceFileSharing.prepare(songs: songs, dependencies: dependencies)
                shareDirectory = value.directory
                preparedShare = value
            } catch { self.error = error.localizedDescription }
        }
    }

    private func folderDragIDs(_ item: SourcePath) -> [Int64] {
        let targets = Self.contextTargets(clicked: item, orderedPaths: wideLayout ? desktopItems : model.items,
            selected: model.selectedPathIds, anchor: selectionAnchor, cachedSongs: trackSongs, wide: wideLayout)
        if wideLayout {
            model.selectedPathIds = targets.selectedPathIds
            selectionAnchor = targets.anchor
            publishFolderState()
        }
        return targets.songs.compactMap(\.id)
    }

    private func folderMenu(_ item: SourcePath) -> LWMenuRequest {
        if wideLayout, item.isAudio {
            let targets = Self.contextTargets(clicked: item, orderedPaths: desktopItems, selected: model.selectedPathIds,
                                              anchor: selectionAnchor, cachedSongs: trackSongs, wide: true)
            model.selectedPathIds = targets.selectedPathIds
            selectionAnchor = targets.anchor
            publishFolderState()
            return savedAudioMenu(targets.songs)
        }
        var groups: [LWMenuGroup] = []
        if item.isDirectory {
            groups.append(LWMenuGroup([
                LWMenuAction(title: "Open folder", icon: "folder", action: { navigateIntoFolder(item) }),
                LWMenuAction(title: "Play folder", icon: "play", action: { Task { await playFolder(shuffled: false, folder: item) } }),
                LWMenuAction(title: "Shuffle folder", icon: "shuf", action: { Task { await playFolder(shuffled: true, folder: item) } })
            ]))
        } else if item.isAudio {
            groups.append(LWMenuGroup([
                LWMenuAction(title: "Play", icon: "play", shortcut: "↩", action: { Task { await play(item) } }),
                LWMenuAction(title: "Play next", icon: "next2", action: {
                    performTrackAction(item) { song in
                        let count = playerVM.queue.count
                        playerVM.addToQueue(song)
                        if count > 0 { playerVM.reorderQueue(from: IndexSet(integer: count), to: min(playerVM.currentQueueIndex + 1, count)) }
                    }
                }),
                LWMenuAction(title: "Add to queue", icon: "queue", action: { performTrackAction(item) { playerVM.addToQueue($0) } })
            ]))
            groups.append(LWMenuGroup([LWMenuAction(title: "Add to list", icon: "plus", submenu: { folderAddToListGroups(item) })]))
            let favorite = trackSongs[item.pathId]?.isFavorite != 0 && trackSongs[item.pathId] != nil
            groups.append(LWMenuGroup([
                LWMenuAction(title: "Go to album", icon: "album", action: {
                    performTrackAction(item) { song in
                        NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: PlaybackContext.album(name: song.album, artist: song.albumArtist.isEmpty ? song.artist : song.albumArtist))
                    }
                }),
                LWMenuAction(title: "Show in folder", icon: "folder", action: {
                    NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: PlaybackContext.folder(sourceID: item.sourceId, pathID: item.parentPathId, name: displayedFolderTitle))
                }),
                LWMenuAction(title: favorite ? "Unfavorite" : "Favorite", icon: "heart", shortcut: "L", action: {
                    performTrackAction(item) { song in
                        if let id = song.id { try await dependencies.songRepository.toggleSongAsFavorite(songId: id, state: !favorite) }
                    }
                })
            ]))
            let pinned = trackSongs[item.pathId].map { pinnedSongIDs.contains(String($0.uniqueId)) } ?? false
            var storage = [LWMenuAction(title: pinned ? "Stop keeping offline" : "Keep offline", icon: pinned ? "check" : "dl", action: {
                performTrackAction(item) { song in
                    guard let id = song.id else { return }
                    if !pinned && song.fileState != .copied { try await dependencies.songRepository.markSongForCopy(songId: id) }
                    withOfflineCacheLock {
                        var pins = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
                        if pinned { pins.remove(String(id)) } else { pins.insert(String(id)) }
                        LocalWavePreferences.current.set(Array(pins), forKey: "lwPinnedSongs")
                    }
                    if let songs = try? await folderSongs() { updateFolderKeptState(songs) }
                }
            })]
            if trackSongs[item.pathId]?.fileState == .failed {
                storage.append(LWMenuAction(title: "Retry download", icon: "rep", action: {
                    performTrackAction(item) { song in if let id = song.id { try await dependencies.songRepository.markSongForCopy(songId: id) } }
                }))
            }
            storage.append(LWMenuAction(title: "Share…", icon: "share", disabled: sharing, action: { share(item) }))
            groups.append(LWMenuGroup(storage))
        }
        let preview: AnyView?
        if let song = trackSongs[item.pathId] { preview = AnyView(LWMenuTrackPreview(song: song, subtitle: song.artist + " · " + (song.fileExtension ?? "audio"))) }
        else { preview = AnyView(Text(item.name).font(LWTheme.text(17)).padding(16)) }
        return LWMenuRequest(groups: groups, preview: preview)
    }

    private func folderAddToListGroups(_ item: SourcePath) -> [LWMenuGroup] {
        var actions = lists.compactMap { list -> LWMenuAction? in
            guard let listID = list.id else { return nil }
            return LWMenuAction(title: list.name, icon: "queue", action: {
                performTrackAction(item) { song in
                    if let songID = song.id { try await LWTrackMenu.addToList(songIDs: [songID], listID: listID, listName: list.name, dependencies: dependencies) }
                }
            })
        }
        actions.append(LWMenuAction(title: "New list…", icon: "plus", action: {
            presenter.present(dialog: LWDialogRequest(title: "New list", input: LWDialogInput(placeholder: "List name"), primaryTitle: "Create", onSubmit: { value in
                let name = value.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !name.isEmpty else { return }
                performTrackAction(item) { song in
                    let created = try await dependencies.playlistRepo.create(playlist: Playlist(id: nil, name: name, createdAt: Date(), updatedAt: nil))
                    guard let listID = created.id, let songID = song.id else { return }
                    do { try await dependencies.playlistSongRepo.addSong(playlistId: listID, songId: songID) }
                    catch { try? await dependencies.playlistRepo.delete(playlistId: listID); throw error }
                    NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
                }
            }))
        }))
        return [LWMenuGroup(actions)]
    }

    private func share(_ item: SourcePath) {
        guard !sharing else { return }
        sharing = true
        Task {
            defer { sharing = false }
            do {
                let saved = try await song(for: item)
                let value = try await LWSourceFileSharing.prepare(songs: [saved], dependencies: dependencies)
                shareDirectory = value.directory
                preparedShare = value
            } catch { self.error = error.localizedDescription }
        }
    }

    private func performTrackAction(_ item: SourcePath, action: @escaping (Song) async throws -> Void) {
        Task {
            do {
                let saved = try await song(for: item)
                try await action(saved)
                await refreshTracks()
                NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
            } catch { self.error = error.localizedDescription }
        }
    }

    private func folderSongs() async throws -> [Song] {
        guard let sourceID = source.id else { return [] }
        let parent = model.parentPathId
        let allPaths = try await dependencies.sourceService.importService().listItems(sourceId: sourceID, parentPathId: nil)
        guard parent == model.parentPathId, !Task.isCancelled else { return [] }
        let current = allPaths.first(where: { $0.pathId == parent })
        folderTitle = current?.name
        let prefix = current?.relativePath ?? ""
        resolvedFolderNames = prefix.split(separator: "/").map(String.init)
        var songs: [Song] = []
        var mapped: [Int64: Song] = [:]
        for path in allPaths where path.isAudio && (prefix.isEmpty || path.relativePath.hasPrefix(prefix + "/")) {
            if let song = await dependencies.songRepository.getSong(sourceId: path.sourceId, relativePath: path.relativePath) {
                songs.append(song)
                mapped[path.pathId] = song
            }
        }
        guard parent == model.parentPathId, !Task.isCancelled else { return [] }
        trackState.songs.merge(mapped) { _, refreshed in refreshed }
        return songs
    }

    private func updateFolderKeptState(_ songs: [Song]) {
        folderSnapshot = songs
        var summaries: [Int64: DirectorySummary] = [:]
        for item in model.items where item.isDirectory {
            let descendants = songs.filter { $0.relativePath?.hasPrefix(item.relativePath + "/") == true }
            summaries[item.pathId] = DirectorySummary(count: descendants.count, bytes: descendants.compactMap(\.fileSize).reduce(0, +))
        }
        directorySummaries = summaries
        updateColumnMetrics()
        let pinned = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
        folderKeptOffline = !songs.isEmpty && songs.allSatisfy { pinned.contains(String($0.id ?? -1)) }
        publishFolderState()
    }

    private func toggleFolderOffline() async {
        changingOffline = true
        defer { changingOffline = false }
        do {
            let songs = try await folderSongs()
            guard !songs.isEmpty else { throw CustomError.genericError("Import this folder before keeping it offline.") }
            var pinned = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
            let release = songs.allSatisfy { pinned.contains(String($0.id ?? -1)) }
            for song in songs {
                guard let id = song.id else { continue }
                if release { pinned.remove(String(id)) }
                else {
                    pinned.insert(String(id))
                    if song.fileState != .copied { try await dependencies.songRepository.markSongForCopy(songId: id) }
                }
            }
            withOfflineCacheLock { LocalWavePreferences.current.set(Array(pinned), forKey: "lwPinnedSongs") }
            updateFolderKeptState(songs)
            NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
            NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        } catch { self.error = error.localizedDescription }
    }
}

struct WaveSettingsView: View {
    let onBack: (() -> Void)?
    private let initialEntryStartedAt: Date?

    init(onBack: (() -> Void)? = nil, initialEntryStartedAt: Date? = nil) {
        self.onBack = onBack
        self.initialEntryStartedAt = initialEntryStartedAt
    }

    @AppStorage("lwDownloadAhead") private var downloadAhead = true
    @AppStorage("lwShowNonAudio") private var showNonAudio = false
    @AppStorage("lwSimulateNoNetwork") private var simulateNoNetwork = false
    @AppStorage("lwAppearance") private var appearance = "system"
    @AppStorage("lwSignal") private var signal = "blue"
    @AppStorage("lwReduceMotion") private var reduceMotion = false
    @AppStorage("lwCacheGB") private var cacheGB = 8
    @EnvironmentObject private var dependencies: DependencyContainer
    @EnvironmentObject private var playerVM: PlayerViewModel
    @EnvironmentObject private var presenter: LWSheetPresenter
    @Environment(\.lwLibraryKeyboardCovered) private var keyboardCovered
    @Environment(\.lwWideLayout) private var wideLayout
    @State private var cacheBytes: Int64 = 0
    @State private var cacheError: String?

    @Environment(\.dismiss) private var dismiss
    @ScaledMetric(relativeTo: .body) private var headingHeight = 33.6
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @Environment(\.lwReduceMotion) private var systemReduceMotion

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                Button(action: settingsBack) {
                    HStack(spacing: 4) { LWIcon("back", size: 18); Text("Library").lwLineBox(15, 21) }.font(LWTheme.text(15))
                        .foregroundStyle(LWTheme.secondary).frame(minHeight: 44)
                }.padding(.horizontal, 10)
                Text("Settings").font(LWTheme.text(32).weight(.medium)).tracking(-1.12).lwLineBox(32, 33.6).frame(height: headingHeight).frame(height: 44, alignment: .top)
                    .padding(.horizontal, 16).padding(.bottom, 12)
                toggleSetting("Download ahead", detail: "Fetch the next two cloud songs in the queue", value: $downloadAhead)
                // Desktop folder tables always show every file type, so this control applies to compact browsing.
                if !wideLayout {
                    toggleSetting("Show non-audio files", detail: "Covers, logs and text in folders", index: 1, value: $showNonAudio)
                }
                toggleSetting("Simulate no network", detail: "Prototype only — shows skipped songs and failed downloads", index: wideLayout ? 1 : 2, value: $simulateNoNetwork)
                setting("Offline space", detail: [4, 8, 16].contains(cacheGB) ? "Oldest unpinned downloads go first" : "\(cacheGB) GB limit · Oldest unpinned downloads go first", index: wideLayout ? 2 : 3, flexibleControl: true) {
                    WaveSettingsFlexLayout(gap: 0) {
                        ForEach([4, 8, 16], id: \.self) { cacheOption($0) }
                    }.padding(2).background(LWTheme.panel)
                    .lwContextMenu { LWMenuRequest(groups: [LWMenuGroup([
                        LWMenuAction(title: "2 GB", icon: cacheGB == 2 ? "check" : nil, action: { cacheGB = 2 }),
                        LWMenuAction(title: "32 GB", icon: cacheGB == 32 ? "check" : nil, action: { cacheGB = 32 })
                    ])]) }
                    .accessibilityAction(named: "Use 2 GB") { cacheGB = 2 }
                    .accessibilityAction(named: "Use 32 GB") { cacheGB = 32 }
                }
                setting("Appearance", detail: "", index: wideLayout ? 3 : 4) { options(["system", "light", "dark"], selection: $appearance) }
                    .lwContextMenu {
                        LWMenuRequest(groups: [LWMenuGroup([
                            LWMenuAction(title: "Signal", submenu: {
                                [LWMenuGroup(["blue", "red", "green", "mono"].map { value in
                                    LWMenuAction(title: value.capitalized, icon: signal == value ? "check" : nil, action: { signal = value })
                                })]
                            }),
                            LWMenuAction(title: "Reduce motion", icon: reduceMotion ? "check" : nil, action: { reduceMotion.toggle() })
                        ])])
                    }
                    .accessibilityAction(named: "Toggle reduced motion") { reduceMotion.toggle() }
                    .accessibilityAction(named: "Use blue signal") { signal = "blue" }
                    .accessibilityAction(named: "Use red signal") { signal = "red" }
                    .accessibilityAction(named: "Use green signal") { signal = "green" }
                    .accessibilityAction(named: "Use monochrome signal") { signal = "mono" }
                Button { dependencies.waveSyncViewModel.refreshSources() } label: {
                    setting("Rescan all sources", detail: "Usually not needed — folders are watched", index: wideLayout ? 4 : 5) {
                        LWIcon("chev", size: 18)
                    }.contentShape(Rectangle())
                }
                VStack(alignment: .leading, spacing: 8) {
                    Text("LocalWave keeps folder bookmarks and caches songs as needed. Individually added files are copied; library data stays on this device.").lwLineBox(14, 19.6)
                    NavigationLink { PrivacyView() } label: {
                        Text("About & Privacy").lwLineBox(14, 19.6).underline()
                    }.frame(minHeight: 44, alignment: .leading)
                    Text("\(ByteCountFormatter.string(fromByteCount: cacheBytes, countStyle: .file)) cached. Kept songs may exceed the limit; Added Files are never removed.").lwLineBox(14, 19.6)
                    if let cacheError { Text(cacheError).foregroundStyle(LWTheme.signal) }
                }.font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary).padding(16).padding(.top, 8)
            }.padding(.top, 10).padding(.bottom, 24)
        }.background(LWTheme.paper).foregroundStyle(LWTheme.ink).buttonStyle(.plain)
            .toolbar(.hidden, for: .navigationBar)
            .background {
                if !keyboardCovered, presenter.content == nil {
                    Button("Back in settings", action: settingsBack).keyboardShortcut("[", modifiers: .command).hidden()
                }
            }
            .onChange(of: downloadAhead) { _, enabled in playerVM.setDownloadAhead(enabled) }
            .task(id: cacheGB) { await updateCacheUsage() }
            .onReceive(NotificationCenter.default.publisher(for: Notification.Name("LibraryRefresh"))) { _ in Task { await updateCacheUsage() } }
    }

    private func settingsBack() {
        if let onBack { onBack() } else { dismiss() }
    }

    private func toggleSetting(_ title: String, detail: String, index: Int = 0, value: Binding<Bool>) -> some View {
        Button { value.wrappedValue.toggle() } label: {
            setting(title, detail: detail, index: index) {
                ZStack(alignment: .leading) {
                    Rectangle().fill(value.wrappedValue ? LWTheme.ink : LWTheme.line).frame(width: 48, height: 28)
                    Rectangle().fill(LWTheme.paper).frame(width: 22, height: 22).offset(x: value.wrappedValue ? 23 : 3)
                }.frame(width: 48, height: 44)
                    .accessibilityHidden(true)
            }.contentShape(Rectangle())
        }.accessibilityRepresentation { Toggle(title, isOn: value).accessibilityHint(detail) }
            .animation(systemReduceMotion ? nil : LWMotion.out(LWMotion.base), value: value.wrappedValue)
    }

    private func cacheOption(_ value: Int) -> some View {
        Button { withAnimation(systemReduceMotion ? nil : LWMotion.out(LWMotion.quick)) { cacheGB = value } } label: {
            Text("\(value) GB").font(LWTheme.text(13)).multilineTextAlignment(.center).lwLineBox(13, 18.2).fixedSize(horizontal: false, vertical: true).frame(minHeight: 18.2).padding(.horizontal, 10).padding(.vertical, 5)
                .frame(maxWidth: .infinity)
                .background(cacheGB == value ? LWTheme.ink : LWTheme.panel)
                .foregroundStyle(cacheGB == value ? LWTheme.paper : LWTheme.secondary)
        }.accessibilityAddTraits(cacheGB == value ? .isSelected : [])
    }

    private func updateCacheUsage() async {
        do {
            try await dependencies.trimOfflineCache()
            cacheBytes = try await Task.detached(priority: .utility) { try BackgroundFileService.cachedByteCount() }.value
            cacheError = nil
        } catch { cacheError = error.localizedDescription }
    }

    private func options(_ values: [String], selection: Binding<String>) -> some View {
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 0) { ForEach(values, id: \.self) { value in option(value, selection: selection) } }.padding(2).background(LWTheme.panel)
            VStack(spacing: 0) { ForEach(values, id: \.self) { value in option(value, selection: selection) } }.padding(2).background(LWTheme.panel)
        }
    }

    private func option(_ value: String, selection: Binding<String>) -> some View {
        Button { withAnimation(systemReduceMotion ? nil : LWMotion.out(LWMotion.quick)) { selection.wrappedValue = value } } label: {
            Text(value.capitalized).font(LWTheme.text(13)).fixedSize().lwLineBox(13, 18.2).frame(minHeight: 18.2).padding(.horizontal, 10).padding(.vertical, 5)
                .background(selection.wrappedValue == value ? LWTheme.ink : LWTheme.panel)
                .foregroundStyle(selection.wrappedValue == value ? LWTheme.paper : LWTheme.secondary)
        }.accessibilityAddTraits(selection.wrappedValue == value ? .isSelected : [])
    }

    private func setting<Content: View>(_ title: String, detail: String, index: Int = 0, flexibleControl: Bool = false, @ViewBuilder content: () -> Content) -> some View {
        Group {
            if dynamicTypeSize.isAccessibilitySize {
                VStack(alignment: .leading, spacing: 12) {
                    settingLabel(title, detail: detail)
                    content()
                }.padding(.vertical, 12)
            } else if flexibleControl {
                WaveSettingsFlexLayout(gap: 16) {
                    settingLabel(title, detail: detail)
                    content()
                }
            } else {
                HStack(spacing: 16) {
                    settingLabel(title, detail: detail).frame(maxWidth: .infinity, alignment: .leading)
                    content().fixedSize()
                }
            }
        }.frame(minHeight: 60).padding(.horizontal, 16)
            .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.leading, 16) }
            .modifier(WaveFilesRise(index: index, startedAt: initialEntryStartedAt))
    }

    private func settingLabel(_ title: String, detail: String) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(title).font(LWTheme.text(15)).lwLineBox(15, 21).frame(minHeight: 21)
            if !detail.isEmpty { Text(detail).font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lwLineBox(13, 18.2).fixedSize(horizontal: false, vertical: true) }
        }
    }
}

struct WaveSourceScanBar: View {
    let progress: Double?
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var began = Date()

    var body: some View {
        GeometryReader { bounds in
            if let progress, progress.isFinite {
                Rectangle().fill(LWTheme.signal).frame(width: bounds.size.width * max(0, min(1, progress)))
            } else {
                TimelineView(.animation(minimumInterval: LWMotion.ambient / 12, paused: reduceMotion)) { timeline in
                    let segment = bounds.size.width / 4
                    let step = Int(max(0, timeline.date.timeIntervalSince(began)) / LWMotion.ambient * 12) % 12
                    let offset = reduceMotion ? (bounds.size.width - segment) / 2 : CGFloat(step) / 11 * (bounds.size.width + segment) - segment
                    Rectangle().fill(LWTheme.ink).frame(width: segment).offset(x: offset)
                }
            }
        }.background(LWTheme.line).frame(height: 4).clipped()
            .accessibilityElement(children: .ignore).accessibilityLabel(progress == nil ? "Scanning folder" : "Indexing audio")
            .accessibilityValue(progress.flatMap { $0.isFinite ? "\(Int(($0 * 100).rounded())) percent" : nil } ?? "In progress")
    }
}

private struct WaveFilesRise: ViewModifier {
    let index: Int
    var enabled = true
    var startedAt: Date? = nil
    @Environment(\.lwReduceMotion) private var reduceMotion
    @Environment(\.lwWideLayout) private var wideLayout
    @State private var appeared = false

    func body(content: Content) -> some View {
        let limit = wideLayout ? 16 : 14
        let elapsed = startedAt.map { Date().timeIntervalSince($0) }
        let entering = enabled && !reduceMotion && index < limit && elapsed.map {
            $0 >= 0 && $0 < LWMotion.slow + Double(limit - 1) * LWMotion.stagger
        } == true
        content.opacity(appeared || !entering ? 1 : 0).offset(y: appeared || !entering ? 0 : 10)
            .task(id: startedAt) {
                guard entering else { appeared = true; return }
                var transaction = Transaction(); transaction.disablesAnimations = true
                withTransaction(transaction) { appeared = false }
                await Task.yield()
                guard !Task.isCancelled else { return }
                withAnimation(LWMotion.out().delay(Double(index) * LWMotion.stagger)) { appeared = true }
            }
    }
}

private struct WaveSourceCardStyle: ButtonStyle {
    @Environment(\.lwReduceMotion) private var reduceMotion
    func makeBody(configuration: Configuration) -> some View {
        configuration.label.background(configuration.isPressed ? LWTheme.panel : LWTheme.paper)
            .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: configuration.isPressed)
    }
}

// CSS flex-shrink distributes compression in proportion to each child's natural width.
// In particular, cache units wrap when the label and segment group compete for space.
private struct WaveSettingsFlexLayout: Layout {
    let gap: CGFloat

    private func widths(_ subviews: Subviews, available: CGFloat?) -> [CGFloat] {
        let natural = subviews.map { $0.sizeThatFits(.unspecified).width }
        let total = natural.reduce(0, +)
        guard let available, total > 0 else { return natural }
        let contentWidth = max(0, available - gap * CGFloat(max(0, subviews.count - 1)))
        let ratio = min(1, contentWidth / total)
        return natural.map { $0 * ratio }
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let widths = widths(subviews, available: proposal.width)
        let height = zip(subviews, widths).map { pair in pair.0.sizeThatFits(ProposedViewSize(width: pair.1, height: nil)).height }.max() ?? 0
        return CGSize(width: proposal.width ?? widths.reduce(0, +) + gap * CGFloat(max(0, subviews.count - 1)), height: height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let widths = widths(subviews, available: bounds.width)
        var x = bounds.minX
        for (view, width) in zip(subviews, widths) {
            let size = view.sizeThatFits(ProposedViewSize(width: width, height: nil))
            view.place(at: CGPoint(x: x, y: bounds.midY), anchor: .leading, proposal: ProposedViewSize(width: width, height: size.height))
            x += width + gap
        }
    }
}

private struct WaveSourceGeometry: ViewModifier {
    let name: String
    func body(content: Content) -> some View {
        #if DEBUG
        content.onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { frame in
            if ProcessInfo.processInfo.environment["LOCALWAVE_SOURCE_GEOMETRY"] == "1" {
                print("LWSourceComponentGeometry name=\(name) frame=\(frame)")
            }
        }
        #else
        content
        #endif
    }
}

private struct WaveFolderOptionsSheet: View {
    let title: String
    let path: String
    let actions: [LWMenuAction]
    @EnvironmentObject private var presenter: LWSheetPresenter

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 0) {
                Text(title).font(LWTheme.text(20).weight(.medium)).tracking(-0.2).lwLineBox(20, 28)
                Text(path).font(LWTheme.data(13)).foregroundStyle(LWTheme.secondary).lwLineBox(13, 18.2, data: true)
            }.padding(.horizontal, 16).padding(.top, 12).padding(.bottom, 8)
            ForEach(actions) { action in
                Button {
                    presenter.dismiss()
                    action.action()
                } label: {
                    HStack(spacing: 14) {
                        if let icon = action.icon { LWIcon(icon, size: 20).foregroundStyle(LWTheme.secondary) }
                        Text(action.title).font(LWTheme.text(16)).lwLineBox(16, 22.4)
                        Spacer(minLength: 0)
                    }.frame(maxWidth: .infinity, minHeight: 52, alignment: .leading).padding(.horizontal, 16)
                }.disabled(action.disabled).buttonStyle(WaveSourceCardStyle())
            }
        }.foregroundStyle(LWTheme.ink).background(LWTheme.paper)
    }
}

#endif
