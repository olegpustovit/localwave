import SwiftUI
import Combine
import UniformTypeIdentifiers
import LocalWaveDomain
import LocalWaveCore

/// One navigation state and content tree; only the surrounding instrument changes with width.
struct MainTabView: View {
    @EnvironmentObject private var dependencies: DependencyContainer
    @EnvironmentObject private var playerVM: PlayerViewModel
    @Environment(\.lwReduceMotion) private var reduceMotion
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @State private var navigation = LWRootNavigationHistory()
    @State private var selectedSongs: [Song] = []
    @StateObject private var libraryModel: WaveLibraryModel
    @State private var folderFilter = "All"
    @State private var libraryRootState = WaveLibraryRootState()
    @State private var libraryHasDetail = false
    @State private var libraryNavigationRequest: WaveLibraryNavigationRequest?
    @State private var contentEntryStartedAt: Date?
    @State private var showNow = false
    private enum ContentEntry: Equatable { case fade, push, pop }
    @State private var contentEntry: ContentEntry?
    @State private var nowClosing = false
    @State private var closeNowTask: Task<Void, Never>?
    @State private var artworkMatching = true
    @State private var showPanel = true
    @State private var isWideLayout = false
    @State private var showCommands = false
    @State private var showKeyboardHelp = false
    @State private var dropTargeted = false
    @State private var hoveredSideItem: String?
    @State private var targetedSidebarListRoute: String?
    @State private var scanningSourceID: Int64?
    @State private var sourceScanProgress: Double?
    @State private var sourceScanCompleted: Int?
    @State private var sourceScanTotal: Int?
    @State private var scannedSourceItems = 0
    @State private var addMusicRequest = 0
    @State private var nativeFolderPicker = false
    @State private var nativeAudioPicker = false
    @State private var nativeImportInProgress = false
    @State private var songs: [Song] = []
    @State private var sources: [Source] = []
    @State private var songsBySource: [Int64: [Song]] = [:]
    @State private var lists: [Playlist] = []
    @State private var offlineCount = 0
    @State private var offlineBytes: Int64 = 0
    @State private var playlistCounts: [Int64: Int] = [:]
    @State private var countsRevision = UUID()
    @State private var refreshObservation: AnyCancellable?
    @StateObject private var sheetPresenter = LWSheetPresenter()
    @StateObject private var canvasSnapshotSource = LWCanvasSnapshotSource()
    private struct PlaybackNotice: Identifiable {
        struct Removal: Equatable {
            let listID: Int64
            let songIDs: [Int64]
            let order: [Int64]
            func matches(_ value: (listID: Int64, songIDs: [Int64], order: [Int64])?) -> Bool {
                guard let value else { return false }
                return listID == value.listID && songIDs == value.songIDs && order == value.order
            }
        }
        let id = UUID()
        let text: String
        var playback = true
        var removal: Removal? = nil
    }
    @State private var activePlaybackNotice: PlaybackNotice?
    @State private var removalUndoInFlightID: UUID?
    @State private var dataError: String?
    @AppStorage("lwDidStart") private var didStart = false
    @AppStorage("lwSimulateNoNetwork") private var simulateNoNetwork = false
    @Namespace private var playerTransition

    init(dependencies: DependencyContainer) {
        _libraryModel = StateObject(wrappedValue: WaveLibraryModel(dependencies))
    }

    var body: some View {
        ZStack(alignment: .bottom) {
            LWCanvasHost(source: canvasSnapshotSource) {
                ZStack(alignment: .bottom) {
                    mainCanvas.allowsHitTesting(!canvasCovered).accessibilityHidden(canvasCovered)
                    if sheetPresenter.kind == .sheet {
                        LWRootSheetHost(presentsSheets: true).environment(\.lwWideLayout, isWideLayout).zIndex(50)
                    }
                    if let notice = activePlaybackNotice {
                        LWPlaybackNotice(message: notice.text,
                            actionTitle: notice.removal == nil ? nil : "Undo",
                            action: notice.removal.map { removal in { consumeRemovalNotice(notice, removal: removal) } },
                            onExpiry: { expireNotice(notice) })
                            .id(notice.id).padding(.horizontal, 16).padding(.bottom, isWideLayout ? 92 : 150)
                            .zIndex(60)
                    }
                }
            }
            .allowsHitTesting(!externalModalVisible)
            .accessibilityHidden(externalModalVisible)
            .zIndex(0)
            if sheetPresenter.kind != .sheet {
                LWRootSheetHost(presentsSheets: false).environment(\.lwWideLayout, isWideLayout)
                    .environment(\.lwCanvasSnapshotSource, canvasSnapshotSource).zIndex(80)
            }
            Group {
                if showCommands { commandPalette.transition(.asymmetric(insertion: .opacity, removal: .identity)) }
            }.animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: showCommands).zIndex(70)
            Group {
                if showKeyboardHelp { LWKeyboardHelp { showKeyboardHelp = false }.transition(.asymmetric(insertion: .opacity, removal: .identity)) }
            }.animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: showKeyboardHelp).zIndex(70)
        }
        .environmentObject(sheetPresenter)
        .environmentObject(libraryModel)
        .onReceive(playerVM.$playbackNotice) { value in
            if let value { replaceNotice(PlaybackNotice(text: value)) }
            else if activePlaybackNotice?.playback == true { activePlaybackNotice = nil }
        }
        .onReceive(NotificationCenter.default.publisher(for: Notification.Name("LWShowNotice"))) { notification in
            if let message = notification.object as? String { replaceNotice(PlaybackNotice(text: message, playback: false)) }
        }
        // @Published emits before committing its value. Handle undo after that
        // commit, rejecting queued events superseded by another removal.
        .onReceive(libraryModel.$undoRemoval.receive(on: RunLoop.main)) { handleRemovalNotice($0) }
        .onChange(of: eligibleSelectionCount, initial: true) { _, count in
            NotificationCenter.default.post(name: Notification.Name("LWSelectionChanged"), object: count)
        }
        .onChange(of: simulateNoNetwork) { _, _ in
            playerVM.setDownloadAhead(playerVM.downloadAheadEnabled)
        }
        .task {
            refreshObservation = NotificationCenter.default.publisher(for: Notification.Name("SongListRefresh"))
                .merge(with: NotificationCenter.default.publisher(for: Notification.Name("LibraryRefresh")))
                .debounce(for: .milliseconds(150), scheduler: RunLoop.main)
                .sink { _ in Task { @MainActor in await refreshCounts() } }
            dependencies.handleAppLaunch()
            await refreshCounts()
            #if DEBUG
            if LWReferenceFixture.enabled {
                switch LWReferenceFixture.screen {
                case "songs": section = "Songs"
                case "artists": section = "Artists"
                case "album": section = "Albums"; playbackDestination = .album(name: "Field Recordings 04", artist: "Halden Marsh")
                case "artist": section = "Artists"; playbackDestination = .artist("Juno Abara")
                case "list": if let list = lists.first, let id = list.id { section = "List:\(id)" }
                case "favorites": section = "Favorites"
                case "find-results":
                    navigation.replace(LWRootNavigationFrame(section: "Find", query: "cloud"))
                    libraryRootState.query = "cloud"
                case "lists": section = "Lists"
                case "find": section = "Find"
                case "folders", "add": section = "Folders"
                case "folder":
                    section = "Folders"
                    playbackDestination = .folder(sourceID: 1, pathID: hashStringToInt64("icloud/Halden Marsh/Field Recordings 04"), name: "Field Recordings 04")
                case "settings": showSettings = true
                case "commands": showCommands = true
                case "shortcuts": showKeyboardHelp = true
                case "now", "queue", "track": showNow = true
                default: break
                }
                if LWReferenceFixture.screen == "add" { addMusicRequest = 1; section = "Add Music" }
                return
            }
            #endif
            dependencies.waveSyncViewModel.refreshSources()
        }
        .onChange(of: scenePhase) { _, phase in
            #if DEBUG
            if LWReferenceFixture.enabled { return }
            #endif
            if phase == .active { dependencies.waveSyncViewModel.refreshSources() }
        }
        .onReceive(NotificationCenter.default.publisher(for: Notification.Name("LWOpenPlaybackContext"))) { notification in
            guard let destination = notification.object as? PlaybackContext else { return }
            openPlaybackContext(destination)
        }
        .onReceive(NotificationCenter.default.publisher(for: Notification.Name("LWCommand"))) { notification in
            guard let action = notification.object as? String, !rootCommandBlocked else { return }
            switch action {
            case "togglePanel": showPanel.toggle()
            case "playPause": playerVM.playPause()
            case "next": playerVM.nextSong()
            case "previous": playerVM.previousSong()
            case "shuffle": playerVM.setShuffle(!playerVM.isShuffleEnabled)
            case "repeat": var mode = playerVM.repeatMode; mode.toggle(); playerVM.setRepeat(mode)
            case "addMusic":
                if isWideLayout { openNativeMusicPicker(folder: false) }
                else { addMusicRequest += 1; select("Add Music") }
            case "addFolder": openNativeMusicPicker(folder: true)
            case "addFiles": openNativeMusicPicker(folder: false)
            case "commands", "Find": openSearch()
            case "shortcuts": showKeyboardHelp = true
            case "New List": requestNewList()
            case "New List from Selection": requestNewList(songs: selectedSongs)
            case "panel": showPanel.toggle()
            case "network": simulateNoNetwork.toggle()
            case "Settings": openSettings()
            default: select(action)
            }
        }
        .onReceive(dependencies.waveSyncViewModel.$currentSyncSourceId.combineLatest(dependencies.waveSyncViewModel.$importProgress)) { sourceID, progress in
            scanningSourceID = sourceID; sourceScanProgress = progress
        }
        .onReceive(dependencies.waveSyncViewModel.$importedFileCount.combineLatest(dependencies.waveSyncViewModel.$importFileCount, dependencies.waveSyncViewModel.$scannedItemCount)) { completed, total, scanned in
            sourceScanCompleted = completed; sourceScanTotal = total; scannedSourceItems = scanned
        }
        .onReceive(NotificationCenter.default.publisher(for: Notification.Name("LWToggleNowPanel"))) { _ in
            showPanel.toggle()
        }
        .onDisappear {
            if let notice = activePlaybackNotice { expireNotice(notice) }
            refreshObservation?.cancel(); refreshObservation = nil
            sheetPresenter.dismissImmediately()
        }
        .modifier(LWNativeMusicImporters(folder: $nativeFolderPicker, audio: $nativeAudioPicker) { result, folders in
            completeNativeMusicPicker(result, folders: folders)
        })
        .alert("LocalWave", isPresented: Binding(get: { dataError != nil || libraryModel.error != nil || (!showNow && (!isWideLayout || !showPanel) && playerVM.playbackError != nil) }, set: { if !$0 { dataError = nil; libraryModel.error = nil; playerVM.playbackError = nil } })) {
            Button("OK") { dataError = nil; libraryModel.error = nil; playerVM.playbackError = nil }
        } message: { Text(dataError ?? libraryModel.error ?? playerVM.playbackError ?? "") }
        .background {
            Group {
                Button("Add Folder…") { openNativeMusicPicker(folder: true) }.keyboardShortcut("o", modifiers: .command).disabled(rootCommandBlocked)
                Button("Add Files…") { openNativeMusicPicker(folder: false) }.keyboardShortcut("o", modifiers: [.command, .shift]).disabled(rootCommandBlocked)
                Button("Find") { openSearch() }.keyboardShortcut("f", modifiers: .command)
                Button("Commands") { openSearch() }.keyboardShortcut("k", modifiers: .command)
                Button("Close panels") {
                    if sheetPresenter.content != nil { sheetPresenter.dismiss() }
                    else if showCommands { showCommands = false }
                    else if showKeyboardHelp { showKeyboardHelp = false }
                    else if showNow && !isWideLayout { closeNow() }
                    else if showSettings { closeSettings() }
                }.keyboardShortcut(.escape, modifiers: []).disabled(!showCommands && !showKeyboardHelp && (!showNow || isWideLayout) && !showSettings && sheetPresenter.content == nil)
                Button("New List") { requestNewList() }.keyboardShortcut("n", modifiers: .command)
                Button("New List from Selection") { requestNewList(songs: selectedSongs) }.keyboardShortcut("n", modifiers: [.command, .shift]).disabled(eligibleSelectionCount == 0)
                Button("Play or pause") { playerVM.playPause() }.keyboardShortcut(.space, modifiers: []).disabled(rootCommandBlocked || showCommands || showKeyboardHelp)
                Button("Next song") { playerVM.nextSong() }.keyboardShortcut(.rightArrow, modifiers: .command).disabled(rootCommandBlocked)
                Button("Previous song") { playerVM.previousSong() }.keyboardShortcut(.leftArrow, modifiers: .command).disabled(rootCommandBlocked)
                Button("Settings") { openSettings() }.keyboardShortcut(",", modifiers: .command)
                Group {
                    Button("Albums") { select("Albums") }.keyboardShortcut("1", modifiers: .command)
                    Button("Artists") { select("Artists") }.keyboardShortcut("2", modifiers: .command)
                    Button("Songs") { select("Songs") }.keyboardShortcut("3", modifiers: .command)
                }.disabled(rootCommandBlocked)
                transportShortcuts
            }.hidden().accessibilityHidden(true)
        }
    }

    private func discardRemoval(_ notice: PlaybackNotice) {
        if let removal = notice.removal, removal.matches(libraryModel.undoRemoval) {
            libraryModel.undoRemoval = nil
        }
    }

    private func replaceNotice(_ notice: PlaybackNotice) {
        if let previous = activePlaybackNotice {
            if previous.removal != notice.removal { discardRemoval(previous) }
            if previous.playback && !notice.playback { playerVM.dismissPlaybackNotice() }
        }
        activePlaybackNotice = notice
    }

    private func expireNotice(_ notice: PlaybackNotice) {
        guard activePlaybackNotice?.id == notice.id else { return }
        activePlaybackNotice = nil
        discardRemoval(notice)
        if notice.playback { playerVM.dismissPlaybackNotice() }
    }

    private func handleRemovalNotice(_ value: (listID: Int64, songIDs: [Int64], order: [Int64])?) {
        guard let value else {
            if activePlaybackNotice?.removal != nil && libraryModel.undoRemoval == nil,
               activePlaybackNotice?.id != removalUndoInFlightID { activePlaybackNotice = nil }
            return
        }
        let removal = PlaybackNotice.Removal(listID: value.listID, songIDs: value.songIDs, order: value.order)
        guard removal.matches(libraryModel.undoRemoval), activePlaybackNotice?.removal != removal else { return }
        let name = libraryModel.lists.first(where: { $0.id == value.listID })?.name
            ?? lists.first(where: { $0.id == value.listID })?.name ?? "list"
        replaceNotice(PlaybackNotice(text: "Removed \(value.songIDs.count) from “\(name)”", playback: false, removal: removal))
    }

    private func consumeRemovalNotice(_ notice: PlaybackNotice, removal: PlaybackNotice.Removal) {
        guard activePlaybackNotice?.id == notice.id else { return }
        guard removal.matches(libraryModel.undoRemoval) else { return }
        removalUndoInFlightID = notice.id
        Task { @MainActor in
            defer { if removalUndoInFlightID == notice.id { removalUndoInFlightID = nil } }
            guard removal.matches(libraryModel.undoRemoval) else { expireNotice(notice); return }
            let restored = await libraryModel.undoRemove()
            guard activePlaybackNotice?.id == notice.id else { return }
            if restored {
                replaceNotice(PlaybackNotice(text: "Undone", playback: false))
            } else if removal.matches(libraryModel.undoRemoval) {
                // The existing shared model error route reports the failure;
                // a fresh notice keeps the original operation retryable.
                replaceNotice(PlaybackNotice(text: notice.text, playback: false, removal: removal))
            } else { expireNotice(notice) }
        }
    }

    private var canvasCovered: Bool { sheetPresenter.content != nil || showCommands || showKeyboardHelp }
    private var externalModalVisible: Bool {
        (sheetPresenter.content != nil && sheetPresenter.kind != .sheet) || showCommands || showKeyboardHelp
    }
    private var mainCanvas: some View {
        GeometryReader { geometry in
            let wide = geometry.size.width >= 900
            let compactSelection = !wide && !showNow && !showSettings && (libraryModel.selecting || folderSelectionActive)
            let contentDuration = !wide && (contentEntry == .push || contentEntry == .pop) ? LWMotion.slow : LWMotion.base
            VStack(spacing: 0) {
                HStack(spacing: 0) {
                    if wide { sidebar.frame(width: 231); rule }
                    ZStack {
                        content
                            .modifier(LWAppear(translation: CGSize(width: wide ? 0 : contentEntry == .push ? 32 : contentEntry == .pop ? -32 : 0, height: 0), duration: contentDuration, enabled: contentEntry != nil))
                            .environment(\.lwLibraryKeyboardCovered, !didStart || (showNow && !wide) || showCommands || showKeyboardHelp || showSettings || sheetPresenter.content != nil || nativeImporterActive)
                            .allowsHitTesting(!showSettings).accessibilityHidden(showSettings)
                            .opacity(showSettings ? 0 : 1)
                            .offset(x: (showSettings) && !wide && !reduceMotion ? -32 : 0)
                            .animation(overlayAnimation(presented: showSettings), value: showSettings)
                            .id(navigation.current.id)
                            .id(playbackDestination)
                            .transition(.identity)
                        if showSettings {
                            NavigationStack {
                                WaveSettingsView(onBack: closeSettings, initialEntryStartedAt: contentEntryStartedAt)
                                    .environment(\.lwLibraryKeyboardCovered, !didStart || showCommands || showKeyboardHelp || sheetPresenter.content != nil || nativeImporterActive)
                                    .toolbar(.hidden, for: .navigationBar)
                            }.background(LWTheme.paper)
                                .modifier(LWAppear(translation: CGSize(width: wide ? 0 : 32, height: 0), duration: wide ? LWMotion.base : LWMotion.slow, enabled: !wide || contentEntry != nil))
                                .transition(.identity)
                        }
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    if wide && showPanel {
                        rule
                        WaveNowView(compact: false)
                            .frame(width: 319)
                            .modifier(LWAppear(translation: CGSize(width: 32, height: 0)))
                            .transition(.identity)
                    }
                }
                .frame(maxHeight: .infinity)
                .animation(nil, value: showNow)
                if wide { Rectangle().fill(LWTheme.line).frame(height: 1) }
                if !wide && !showNow && !compactSelection && simulateNoNetwork { offlineBanner }
                if showNow && !wide {
                    Color.clear.frame(height: 84)
                } else if !compactSelection {
                WaveTransport(compact: !wide, panelVisible: showPanel) {
                    if wide { showPanel.toggle() }
                    else { _ = UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil); closeNowTask?.cancel(); closeNowTask = nil; artworkMatching = true; nowClosing = false; withAnimation(reduceMotion ? nil : LWMotion.out(LWMotion.slow)) { showNow = true } }
                }
                }
                if !wide && !showNow && !compactSelection { navigationDeck }
            }
            .allowsHitTesting(didStart && (!showNow || wide) && sheetPresenter.content == nil && !showCommands && !showKeyboardHelp)
            .accessibilityHidden(!didStart || (showNow && !wide) || sheetPresenter.content != nil || showCommands || showKeyboardHelp)
            .onChange(of: wide, initial: true) { _, value in
                isWideLayout = value
                if value { promoteMobileOverlays() }
            }
            .environment(\.lwWideLayout, wide)
            .environment(\.lwFolderFilter, $folderFilter)
            .background(LWTheme.paper)
            .overlay { if !didStart { onboarding } }
            .overlay {
                if showNow && !wide {
                    WaveNowView(compact: true, onClose: closeNow, artmatchEnabled: artworkMatching, closing: nowClosing)
                        .transition(.identity)
                        .zIndex(10)
                }
            }
            .environment(\.lwArtworkNamespace, playerTransition)
            .overlay {
                if dropTargeted {
                    VStack(spacing: 12) {
                        Text("Drop to play").font(LWTheme.text(40)).fontWeight(.medium)
                        Text("Folders are indexed. Audio files join Added Files.").foregroundStyle(LWTheme.secondary)
                    }.frame(maxWidth: .infinity, maxHeight: .infinity)
                        .background(LWTheme.paper.opacity(0.94)).padding(12)
                        .overlay { Rectangle().strokeBorder(LWTheme.ink, style: StrokeStyle(lineWidth: 2, dash: [6])).padding(12) }
                        .allowsHitTesting(false)
                }
            }
            .dropDestination(for: URL.self) { urls, _ in
                guard !urls.isEmpty else { return false }
                Task {
                    do {
                        let imported = try await dependencies.addMusicURLs(urls)
                        select("Folders")
                        if let first = imported.first {
                            playerVM.configureQueue(songs: imported, startIndex: 0, context: .selection("Dropped music"))
                            playerVM.playSong(first)
                        }
                    }
                    catch { dataError = error.localizedDescription }
                }
                return true
            } isTargeted: { dropTargeted = $0 }
        }
    }

    private var transportShortcuts: some View {
        Group {
            Group {
                Button("Shuffle") { playerVM.setShuffle(!playerVM.isShuffleEnabled) }.keyboardShortcut("s", modifiers: [])
                Button("Repeat") { var mode = playerVM.repeatMode; mode.toggle(); playerVM.setRepeat(mode) }.keyboardShortcut("r", modifiers: [])
                Button("Toggle player panel") { showPanel.toggle() }.keyboardShortcut("i", modifiers: [])
                Button("Favorite current song") {
                    Task { do { try await playerVM.toggleCurrentFavorite() } catch { dataError = error.localizedDescription } }
                }.keyboardShortcut("l", modifiers: [])
                Button("Keyboard shortcuts") { showKeyboardHelp.toggle() }.keyboardShortcut("?", modifiers: [])
                Button("Search and commands") { openSearch() }.keyboardShortcut("/", modifiers: [])
            }
            Group {
                Button("Seek back five seconds") { playerVM.seekBySeconds(-5) }.keyboardShortcut(.leftArrow, modifiers: [])
                Button("Seek forward five seconds") { playerVM.seekBySeconds(5) }.keyboardShortcut(.rightArrow, modifiers: [])
                Button("Seek back thirty seconds") { playerVM.seekBySeconds(-30) }.keyboardShortcut(.leftArrow, modifiers: .option)
                Button("Seek forward thirty seconds") { playerVM.seekBySeconds(30) }.keyboardShortcut(.rightArrow, modifiers: .option)
                Button("Previous track") { playerVM.previousSong() }.keyboardShortcut(.leftArrow, modifiers: .shift)
                Button("Next track") { playerVM.nextSong() }.keyboardShortcut(.rightArrow, modifiers: .shift)
                Button("Volume up") { playerVM.volume = min(1, playerVM.volume + 1/12) }.keyboardShortcut(.upArrow, modifiers: .command)
                Button("Volume down") { playerVM.volume = max(0, playerVM.volume - 1/12) }.keyboardShortcut(.downArrow, modifiers: .command)
            }
        }.disabled(showCommands || showKeyboardHelp || sheetPresenter.content != nil || (!isWideLayout && section == "Find"))
    }
    private var rule: some View { Rectangle().fill(LWTheme.line).frame(width: 1) }
    private var folderSelectionActive: Bool {
        if case .folder? = playbackDestination { return navigation.current.folderState?.selecting == true }
        if section.hasPrefix("Source:") { return navigation.current.folderState?.selecting == true }
        guard section == "Folders" || section == "Add Music",
              let state = navigation.current.filesState, let sourceID = state.sourcePath.last else { return false }
        return state.folders[sourceID]?.selecting == true
    }
    private var section: String {
        get { navigation.current.section }
        nonmutating set {
            navigation.update(navigation.current.id) {
                $0.section = newValue
                $0.libraryBoundaryDepth = newValue.hasPrefix("List:") ? 1 : 0
            }
        }
    }
    private var playbackDestination: PlaybackContext? {
        get { navigation.current.playbackDestination }
        nonmutating set {
            navigation.update(navigation.current.id) {
                $0.playbackDestination = newValue
                $0.libraryPath = nil
                $0.libraryBoundaryDepth = isLibraryDetail(newValue ?? .library) ? 1 : 0
            }
        }
    }
    private var showSettings: Bool {
        get { navigation.current.showsSettings }
        nonmutating set { navigation.update(navigation.current.id) { $0.showsSettings = newValue } }
    }
    @ViewBuilder private var content: some View {
        let frame = navigation.current
        let frameID = frame.id
        if section == "Settings" { Color.clear }
        else if case .folder(let sourceID, let pathID, _)? = playbackDestination, let source = sources.first(where: { $0.id == sourceID }) {
            NavigationStack {
                WaveFolderView(source: source, dependencies: dependencies, initialParentPathId: pathID, backTitle: rootBackTitle,
                    initialEntryStartedAt: contentEntryStartedAt, initialNavigationState: frame.folderState,
                    onNavigationStateChange: { state in navigation.update(frameID) { $0.folderState = state } },
                    onSourceRootChange: { source in updateSidebar(source.map { "Source:\($0.stableId)" }, frameID: frameID) },
                    onBack: rootBack, onSelectionChange: { updateSelection($0, frameID: frameID) }, includeAncestorHistory: frame.includesFolderAncestors)
            }
        } else if section == "Folders" || section == "Add Music" {
            WaveFilesView(dependencies: dependencies, addRequest: section == "Add Music" ? addMusicRequest : 0,
                rootEntryStartedAt: contentEntryStartedAt, onDetailChange: { value in if navigation.current.id == frameID { libraryHasDetail = value } },
                initialNavigationState: frame.filesState,
                onNavigationStateChange: { state in navigation.update(frameID) { $0.filesState = state } },
                onSourceRootChange: { source in updateSidebar(source.map { "Source:\($0.stableId)" }, frameID: frameID) },
                onSelectionChange: { updateSelection($0, frameID: frameID) }, onRootBack: navigation.canGoBack ? rootBack : nil,
                onMusicSelection: { result, folders in completeNativeMusicPicker(result, folders: folders) })
        } else if section.hasPrefix("Source:"), let id = Int64(section.dropFirst(7)), let source = sources.first(where: { $0.id == id }) {
            NavigationStack {
                WaveFolderView(source: source, dependencies: dependencies, backTitle: rootBackTitle,
                    initialEntryStartedAt: contentEntryStartedAt, initialNavigationState: frame.folderState,
                    onNavigationStateChange: { state in navigation.update(frameID) { $0.folderState = state } },
                    onSourceRootChange: { source in updateSidebar(source.map { "Source:\($0.stableId)" }, frameID: frameID) },
                    onBack: rootBack, onSelectionChange: { updateSelection($0, frameID: frameID) }, includeAncestorHistory: frame.includesFolderAncestors)
            }
        } else {
            WaveLibraryView(dependencies: dependencies, section: section, initialContext: playbackDestination,
                initialQuery: section == "Find" ? frame.searchQuery : nil, initialShowAll: section == "Find" && frame.searchQuery != nil ? frame.searchShowAll : nil,
                rootState: libraryRootState, onRootStateChange: { value in
                    guard navigation.current.id == frameID else { return }
                    libraryRootState = value
                    if section == "Find" { navigation.update(frameID) { $0.searchQuery = value.query; $0.searchShowAll = value.showAllFindSongs } }
                }, rootEntryStartedAt: contentEntryStartedAt,
                onDetailChange: { value in if navigation.current.id == frameID { libraryHasDetail = value } }, navigationRequest: libraryNavigationRequest,
                onRootBack: navigation.canGoBack || frame.libraryBoundaryDepth > 0 || isListSection ? rootBack : nil,
                onSidebarRouteChange: { updateSidebar($0, frameID: frameID) }, initialNavigationPath: frame.libraryPath,
                onNavigationPathChange: { path in navigation.update(frameID) { $0.libraryPath = path } }, rootBackBoundaryDepth: frame.libraryBoundaryDepth,
                onSelectionChange: { updateSelection($0, frameID: frameID) }, sharedModel: libraryModel)
        }
    }
    private func requestNewList(songs: [Song] = []) {
        guard sheetPresenter.content == nil || sheetPresenter.kind == .menu else { return }
        let message = songs.isEmpty ? "Name your list." : "\(songs.count) song\(songs.count == 1 ? "" : "s") will be added."
        sheetPresenter.present(dialog: LWDialogRequest(title: "New List", message: message, input: LWDialogInput(placeholder: "List name"), primaryTitle: "Create", onSubmit: { value in
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            let name = trimmed.isEmpty ? "Untitled list" : trimmed
            Task {
                do {
                    let list = try await dependencies.playlistRepo.create(playlist: Playlist(id: nil, name: name, createdAt: Date(), updatedAt: nil))
                    if let id = list.id {
                        do {
                            for song in songs { if let songID = song.id { try await dependencies.playlistSongRepo.addSong(playlistId: id, songId: songID) } }
                        } catch {
                            try? await dependencies.playlistRepo.delete(playlistId: id)
                            throw error
                        }
                        NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
                        if songs.isEmpty { select("List:\(id)") }
                        LWPresentationNotice.show("Created “\(name)”")
                    }
                } catch { dataError = error.localizedDescription }
            }
        }))
    }
    private var eligibleSelectionCount: Int {
        if showSettings || showCommands || showKeyboardHelp || (sheetPresenter.content != nil && sheetPresenter.kind != .menu) { return 0 }
        return selectedSongs.count
    }
    private func updateSelection(_ songs: [Song], frameID: UUID) {
        guard navigation.current.id == frameID, !showSettings else { return }
        selectedSongs = songs
    }
    private func updateSidebar(_ route: String?, frameID: UUID) {
        navigation.update(frameID) { $0.sidebarRoute = route }
    }
    private var rootBackTitle: String {
        guard let previous = navigation.previous.last else { return "Folders" }
        if let context = previous.playbackDestination { return context.title }
        if ["Library", "Albums", "Artists", "Songs"].contains(previous.section) { return "Library" }
        return previous.section
    }
    private func rootBack() {
        guard navigation.canGoBack else {
            let listDestination: Bool
            switch playbackDestination {
            case .playlist?, .automatic?: listDestination = true
            default: listDestination = isListSection
            }
            if listDestination { select("Lists") }
            else if isLibraryDetail(playbackDestination ?? .library) { select("Library") }
            else { select("Folders") }
            return
        }
        var transaction = Transaction(); transaction.disablesAnimations = true
        withTransaction(transaction) {
            _ = navigation.back()
            clearRootSelection()
            navigation.update(navigation.current.id) { frame in
                if let state = frame.folderState { frame.folderState = clearingSelection(state) }
                if var state = frame.filesState { state.folders = state.folders.mapValues(clearingSelection); frame.filesState = state }
            }
            libraryNavigationRequest = nil
            selectedSongs = []
            libraryHasDetail = false
            contentEntry = isWideLayout ? nil : .pop
            contentEntryStartedAt = isWideLayout ? nil : Date()
            if section == "Find", let query = navigation.current.searchQuery {
                libraryRootState.query = query
                libraryRootState.showAllFindSongs = navigation.current.searchShowAll
            }
        }
    }
    private func clearingSelection(_ input: WaveFolderNavigationState) -> WaveFolderNavigationState {
        var state = input
        state.browse.selectedPathIds = []
        state.selecting = false
        state.desktopSort = ""; state.desktopAscending = true
        return state
    }
    private func clearRootSelection() {
        selectedSongs = []
        libraryModel.selected = []; libraryModel.selecting = false
        libraryModel.selectionAnchor = nil; libraryModel.selectionListID = nil
    }
    private func pushDesktopFrame(_ next: LWRootNavigationFrame, animate: Bool = true) {
        if ["Library", "Albums", "Artists", "Songs"].contains(section), playbackDestination == nil {
            navigation.update(navigation.current.id) { $0.section = libraryRootState.mode }
        }
        navigation.push(next)
        clearRootSelection()
        libraryNavigationRequest = nil
        selectedSongs = []
        libraryHasDetail = false
        showCommands = false; showKeyboardHelp = false
        contentEntry = animate ? .fade : nil
        contentEntryStartedAt = animate ? Date() : nil
    }
    private func promoteMobileOverlays() {
        let settings = showSettings && section != "Settings"
        if settings { showSettings = false }
        if settings { pushDesktopFrame(LWRootNavigationFrame(section: "Settings"), animate: false) }
    }
    private func openPlaybackContext(_ destination: PlaybackContext) {
        dismissNowForNavigation()
        if isWideLayout {
            if case .folder(let id, _, _) = destination, !sources.contains(where: { $0.id == id }) { select("Folders"); return }
            if isLibrarySurface && !showSettings && isLibraryDetail(destination) {
                libraryNavigationRequest = WaveLibraryNavigationRequest(context: destination)
            } else {
                let target: String
                switch destination {
                case .library, .selection: target = "Songs"
                case .album: target = "Albums"
                case .artist: target = "Artists"
                case .playlist: target = "Lists"
                case .automatic(let title): target = title
                case .folder(let id, _, _): target = "Source:\(id)"
                }
                pushDesktopFrame(LWRootNavigationFrame(section: target, context: destination))
            }
            return
        }
        if case .folder(let id, _, _) = destination {
            guard sources.contains(where: { $0.id == id }) else { select("Folders"); return }
            var frame = LWRootNavigationFrame(section: "Folders", context: destination)
            frame.includesFolderAncestors = true
            navigation.reset(frame)
            clearRootSelection()
            libraryNavigationRequest = nil; selectedSongs = []
            contentEntry = .push; contentEntryStartedAt = Date()
            return
        }
        let target: String
        switch destination {
        case .library, .selection: target = "Songs"
        case .album: target = "Albums"
        case .artist: target = "Artists"
        case .playlist: target = "Lists"
        case .automatic(let title): target = title
        case .folder: target = "Folders"
        }
        navigation.reset(LWRootNavigationFrame(section: target, context: destination))
        clearRootSelection()
        libraryNavigationRequest = nil; selectedSongs = []
        contentEntry = .push; contentEntryStartedAt = Date()
    }
    private func isLibraryDetail(_ context: PlaybackContext) -> Bool {
        switch context {
        case .album, .artist, .playlist, .automatic: return true
        default: return false
        }
    }
    private var isLibrarySurface: Bool {
        section != "Settings" && section != "Folders" && section != "Add Music" && !section.hasPrefix("Source:")
    }
    private var rootCommandBlocked: Bool {
        !didStart || nativeImporterActive || (sheetPresenter.content != nil && sheetPresenter.kind != .menu)
    }
    private var nativeImporterActive: Bool { nativeFolderPicker || nativeAudioPicker || nativeImportInProgress }

    private func openNativeMusicPicker(folder: Bool) {
        guard !rootCommandBlocked else { return }
        if sheetPresenter.kind == .menu { sheetPresenter.dismissImmediately() }
        if folder { nativeFolderPicker = true }
        else { nativeAudioPicker = true }
    }

    private func completeNativeMusicPicker(_ result: Result<[URL], Error>, folders: Bool) {
        let urls: [URL]
        do { urls = try result.get() }
        catch {
            let failure = error as NSError
            if failure.domain == NSCocoaErrorDomain && failure.code == NSUserCancelledError { return }
            dataError = error.localizedDescription
            return
        }
        guard !urls.isEmpty, !nativeImportInProgress else { return }
        nativeImportInProgress = true
        Task {
            defer { nativeImportInProgress = false }
            do {
                let imported = try await dependencies.addMusicURLs(urls)
                guard let user = try await dependencies.userCloudService.resolveCurrentICloudUser(), let userID = user.id else {
                    throw CustomError.genericError("The music source is unavailable.")
                }
                let loadedSources = try await dependencies.sourceService.repository().findOneByUserId(userId: userID, path: nil)
                // Make the newly imported source available to the existing
                // navigation guard before opening its exact root folder.
                sources = loadedSources
                let destination: Source?
                if folders {
                    let selectedPath = urls[0].standardizedFileURL.path
                    destination = loadedSources.first { URL(fileURLWithPath: $0.dirPath).standardizedFileURL.path == selectedPath }
                } else if let sourceID = imported.first?.sourceId {
                    destination = loadedSources.first { $0.id == sourceID }
                } else {
                    let addedFiles = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("Added Files").path
                    destination = loadedSources.first { $0.dirPath == addedFiles }
                }
                guard let source = destination, let sourceID = source.id else {
                    throw CustomError.genericError("The music was added, but its source folder could not be opened. Open Folders to find it.")
                }
                if !folders, let first = imported.first {
                    playerVM.configureQueue(songs: imported, startIndex: 0, context: .selection("Selected files"))
                    playerVM.playSong(first)
                }
                showCommands = false; showKeyboardHelp = false; showSettings = false
                openPlaybackContext(.folder(sourceID: sourceID, pathID: source.pathId, name: URL(fileURLWithPath: source.dirPath).lastPathComponent))
            } catch { dataError = error.localizedDescription }
        }
    }

    private func openSearch() {
        guard !rootCommandBlocked else { return }
        if sheetPresenter.kind == .menu { sheetPresenter.dismiss() }
        if isWideLayout { showCommands = true }
        else { closeNow(); select("Find") }
    }
    private func select(_ next: String) {
        if next == "Add Music" && isWideLayout { openNativeMusicPicker(folder: false); return }
        if next == "New List" { requestNewList(); return }
        if isWideLayout { pushDesktopFrame(LWRootNavigationFrame(section: next)); dismissNowForNavigation(); return }
        let sameLibrary = next == "Library" && ["Library", "Albums", "Artists", "Songs"].contains(section)
        let changed = (!sameLibrary && section != next) || playbackDestination != nil || showSettings || libraryHasDetail
        if changed {
            navigation.reset(LWRootNavigationFrame(section: next))
            clearRootSelection()
            contentEntry = .fade; contentEntryStartedAt = Date()
        }
        libraryHasDetail = false; libraryNavigationRequest = nil; selectedSongs = []
        showCommands = false; showSettings = false; showKeyboardHelp = false
        if showNow { closeNow() }
    }
    private func closeNow() {
        guard showNow, !nowClosing else { return }
        closeNowTask?.cancel()
        artworkMatching = false
        withAnimation(reduceMotion ? nil : LWMotion.in(LWMotion.base)) { nowClosing = true }
        closeNowTask = Task { @MainActor in
            if !reduceMotion { try? await Task.sleep(for: .seconds(LWMotion.base)) }
            guard !Task.isCancelled else { return }
            var transaction = Transaction(); transaction.disablesAnimations = true
            withTransaction(transaction) { showNow = false; nowClosing = false; closeNowTask = nil }
        }
    }
    private func dismissNowForNavigation() {
        _ = UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
        closeNowTask?.cancel(); closeNowTask = nil
        var transaction = Transaction(); transaction.disablesAnimations = true
        withTransaction(transaction) { showNow = false; nowClosing = false; artworkMatching = false }
    }
    private var isListSection: Bool {
        section.hasPrefix("List:") || ["Favorites", "Recently Added", "Most Played", "Kept Offline", "Offline", "Never Played"].contains(section)
    }
    private func overlayAnimation(presented: Bool) -> Animation? {
        guard !reduceMotion, !isWideLayout || presented else { return nil }
        return LWMotion.out(isWideLayout ? LWMotion.base : LWMotion.slow)
    }
    private func openSettings() {
        guard !rootCommandBlocked else { return }
        if sheetPresenter.kind == .menu { sheetPresenter.dismiss() }
        if isWideLayout { pushDesktopFrame(LWRootNavigationFrame(section: "Settings")) }
        else { contentEntryStartedAt = Date(); showSettings = true }
    }
    private func closeSettings() {
        if section == "Settings" && navigation.canGoBack { rootBack(); return }
        if !isWideLayout { contentEntryStartedAt = Date() }
        showSettings = false
    }
    private var navigationDeck: some View {
        LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 2), count: dynamicTypeSize.isAccessibilitySize ? 2 : 4), spacing: 2) {
            ForEach(["Library", "Folders", "Lists", "Find"], id: \.self) { label in
                let active = label == "Library" ? ["Library", "Albums", "Artists", "Songs"].contains(section) : section == label || (label == "Lists" && isListSection) || (label == "Folders" && section == "Add Music")
                Button { select(label) } label: {
                    HStack(spacing: 6) {
                        if active { Rectangle().fill(LWTheme.ink).frame(width: 6, height: 6) }
                        Text(label).lwLineBox(15, 21)
                    }.font(LWTheme.text(15)).fontWeight(active ? .medium : .regular)
                        .lineLimit(1).minimumScaleFactor(0.75)
                        .foregroundStyle(active ? LWTheme.ink : LWTheme.secondary)
                        .frame(maxWidth: .infinity).frame(minHeight: dynamicTypeSize.isAccessibilitySize ? 54 : 52)
                        .background(active ? LWTheme.panel : LWTheme.key)
                }.accessibilityAddTraits(active ? .isSelected : [])
            }
        }.padding(.horizontal, 2).padding(.top, 2).background(LWTheme.panel2).buttonStyle(LWKeyStyle())
            .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: section)
    }
    private var offlineBanner: some View {
        Button { simulateNoNetwork = false } label: {
            HStack(spacing: 8) {
                Text("You’re offline · cloud songs are skipped")
                Spacer(minLength: 0)
                Text("Go online").underline()
            }
            .font(LWTheme.text(13)).lwLineBox(13, 18.2)
            .padding(.horizontal, 16).frame(minHeight: 36)
            .background(LWTheme.ink).foregroundStyle(LWTheme.onInk)
        }.accessibilityLabel("You’re offline · cloud songs are skipped. Go online")
    }
    private var sidebar: some View {
        GeometryReader { bounds in
            let rows = 3 + sources.count + 1 + 6 + lists.count + 1 + (simulateNoNetwork ? 1 : 0)
            let fixed = 28.0 + 28 + 3 * 36.8 + 73.6 + 12 + Double(5 + rows) * 2
            let factor = min(1, max(0.62, (bounds.size.height - fixed) / Double(rows * 32 + 34)))
            let rowHeight = CGFloat(32 * factor)
            let counts = LocalWavePreferences.current.dictionary(forKey: "lwPlayCounts") as? [String: Int] ?? [:]
            let pins = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
            let menu = VStack(alignment: .leading, spacing: 2) {
                    if !dynamicTypeSize.isAccessibilitySize { Color.clear.frame(height: 28).accessibilityHidden(true) }
                    Button { showCommands = true } label: {
                        HStack(spacing: 8) {
                            LWIcon("search", size: 16)
                            Text("Search").font(LWTheme.text(14)).lwLineBox(14, 19.6)
                            Spacer(minLength: 0)
                            Text("⌘K").font(LWTheme.data(11)).lwLineBox(11, 15.4, data: true)
                        }.foregroundStyle(LWTheme.secondary).padding(.horizontal, 10)
                            .frame(minHeight: dynamicTypeSize.isAccessibilitySize ? 44 : CGFloat(34 * factor)).background(LWTheme.panel)
                    }.accessibilityLabel("Search").padding(.bottom, 12)
                    groupLabel("Library")
                    sideItem("Albums", count: Set(songs.map { $0.album + "\u{001F}" + $0.artist }).count, height: rowHeight)
                    sideItem("Artists", count: Set(songs.map(\.artist)).count, height: rowHeight)
                    sideItem("Songs", count: songs.count, height: rowHeight)
                    groupLabel("Sources")
                    ForEach(sources, id: \.id) { source in
                        if let id = source.id {
                            let scanning = scanningSourceID == id
                            sideItem(URL(fileURLWithPath: source.dirPath).lastPathComponent,
                                count: scanning ? nil : source.totalPaths, route: "Source:\(id)", height: rowHeight,
                                warning: !scanning && source.syncError != nil ? "Reconnect" : nil,
                                status: scanning ? "scanning…" : nil,
                                progress: scanning ? sourceScanProgress : nil,
                                countText: scanning ? sourceScanCountLabel : nil)
                        }
                    }
                    if simulateNoNetwork {
                        Button { simulateNoNetwork = false } label: {
                            HStack(spacing: 8) {
                                LWStateGlyph(state: .failed, availability: .unknown, pinned: false)
                                Text("Offline · go online").font(LWTheme.text(14)).lwLineBox(14, 19.6)
                                Spacer(minLength: 0)
                            }.padding(.horizontal, 8)
                                .frame(minHeight: dynamicTypeSize.isAccessibilitySize ? max(44, rowHeight) : rowHeight)
                                .foregroundStyle(LWTheme.ink)
                                .background(hoveredSideItem == "network" ? LWTheme.panel : Color.clear)
                                .contentShape(Rectangle())
                        }
                        .onHover { hoveredSideItem = $0 ? "network" : nil }
                        .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: hoveredSideItem)
                    }
                    sideItem("Add folder or files", symbol: "plus", route: "Add Music", height: rowHeight)
                    groupLabel("Lists")
                    sideItem("Favorites", count: songs.filter { $0.isFavorite != 0 }.count, height: rowHeight)
                    sideItem("Recently added", count: min(24, songs.count), route: "Recently Added", height: rowHeight)
                    sideItem("Most played", count: min(24, songs.filter { (counts[String($0.uniqueId)] ?? 0) > 0 }.count), route: "Most Played", height: rowHeight)
                    sideItem("Kept offline", count: songs.filter { pins.contains(String($0.uniqueId)) }.count, route: "Kept Offline", height: rowHeight)
                    sideItem("On this device now", count: offlineCount, route: "Offline", height: rowHeight)
                    sideItem("Never played", count: songs.filter { (counts[String($0.uniqueId)] ?? 0) == 0 }.count, route: "Never Played", height: rowHeight)
                    ForEach(lists) { list in
                        if let id = list.id { sideItem(list.name, count: playlistCounts[id], route: "List:\(id)", height: rowHeight) }
                    }
                    sideItem("New list", symbol: "plus", route: "New List", height: rowHeight)
            }
            VStack(spacing: 0) {
                if dynamicTypeSize.isAccessibilitySize {
                    Color.clear.frame(height: 30).accessibilityHidden(true)
                    ScrollView { menu }.scrollIndicators(.hidden)
                } else { menu }
                Spacer(minLength: 0)
                VStack(alignment: .leading, spacing: 0) {
                    HStack {
                        Text("Offline").font(LWTheme.text(12)).lwLineBox(12, 16.8)
                        Spacer(minLength: 0)
                        Text(String(format: "%.1f / %d GB", Double(offlineBytes) / 1_073_741_824, LocalWavePreferences.current.integer(forKey: "lwCacheGB") == 0 ? 8 : LocalWavePreferences.current.integer(forKey: "lwCacheGB"))).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true)
                    }.frame(height: dynamicTypeSize.isAccessibilitySize ? nil : 16.8)
                    GeometryReader { meter in
                        let limit = Double(max(1, LocalWavePreferences.current.integer(forKey: "lwCacheGB") == 0 ? 8 : LocalWavePreferences.current.integer(forKey: "lwCacheGB"))) * 1_073_741_824
                        Rectangle().fill(LWTheme.ink).frame(width: meter.size.width * min(1, Double(offlineBytes) / limit))
                    }.frame(height: 4).background(LWTheme.line).padding(.top, 10)
                    Button { showKeyboardHelp = true } label: {
                        HStack(spacing: 5) {
                            Text("?").font(LWTheme.data(11)).padding(.horizontal, 6).padding(.vertical, 1).background(LWTheme.panel).overlay { Rectangle().strokeBorder(LWTheme.line) }
                            Text("shortcuts").font(LWTheme.text(12)).lwLineBox(12, 16.8)
                        }.frame(height: dynamicTypeSize.isAccessibilitySize ? nil : 16.8)
                    }.accessibilityLabel("Keyboard shortcuts").padding(.top, 10)
                }.foregroundStyle(LWTheme.secondary).padding(.horizontal, 8).padding(.top, 14).padding(.bottom, 4)
            }.padding(.horizontal, 10).padding(.vertical, 14)
        }.background(LWTheme.paper)
    }
    private func groupLabel(_ title: String) -> some View {
        Text(title).font(LWTheme.text(12).weight(.medium)).lwLineBox(12, 16.8).foregroundStyle(LWTheme.secondary).frame(height: dynamicTypeSize.isAccessibilitySize ? nil : 16.8).padding(.horizontal, 8).padding(.top, 14).padding(.bottom, 6)
    }
    private func sideItem(_ label: String, count: Int? = nil, symbol: String? = nil, route: String? = nil, height: CGFloat = 32, warning: String? = nil, status: String? = nil, progress: Double? = nil, countText: String? = nil) -> some View {
        Button { select(route ?? label) } label: {
            HStack(spacing: 8) {
                if let symbol { LWIcon(symbol, size: 16) }
                Text(label).lwLineBox(14, 19.6)
                if let warning { Text(warning).font(LWTheme.text(11)).lwLineBox(11, 15.4).padding(.horizontal, 6).padding(.vertical, 1).background(LWTheme.ink).foregroundStyle(LWTheme.onInk).padding(.leading, -2) }
                if let status { Text(status).font(LWTheme.text(12)).lwLineBox(12, 16.8) }
                Spacer(minLength: 0)
                if let countText { Text(countText).font(LWTheme.text(12)).monospacedDigit().lwLineBox(12, 16.8) }
                else if let progress { Text(String(format: "%.0f%%", progress)).font(LWTheme.text(12)).monospacedDigit().lwLineBox(12, 16.8) }
                else if let count { Text("\(count)").font(LWTheme.text(12)).monospacedDigit().lwLineBox(12, 16.8) }
            }.font(LWTheme.text(14)).padding(.horizontal, 8).frame(minHeight: dynamicTypeSize.isAccessibilitySize ? max(44, height) : height)
                .foregroundStyle(selectedSidebarRoute == (route ?? label) ? LWTheme.ink : LWTheme.secondary)
                .background {
                    if let route, route.hasPrefix("List:"), targetedSidebarListRoute == route {
                        LWTheme.paper.overlay(LWTheme.signal.opacity(0.08))
                    } else { (selectedSidebarRoute == (route ?? label) ? LWTheme.panel2 : hoveredSideItem == (route ?? label) ? LWTheme.panel : Color.clear) }
                }
                .overlay { if let route, route.hasPrefix("List:"), targetedSidebarListRoute == route { Rectangle().strokeBorder(LWTheme.signal, lineWidth: 2) } }
                .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: hoveredSideItem)
                .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: selectedSidebarRoute)
                .contentShape(Rectangle())
        }.accessibilityLabel(label)
        .onHover { hoveredSideItem = $0 ? (route ?? label) : nil }
        .modifier(LWPlaylistActions(playlist: sidebarPlaylist(route), dependencies: dependencies, onDelete: {
            if let route, section == route { select("Lists") }
        }))
        .modifier(LWSourceActions(source: sidebarSource(route), songs: sidebarSourceSongs(route), dependencies: dependencies,
            onOpen: { if let route { select(route) } }, onRemoved: {
                if let route, section == route { select("Folders") }
                else if case .folder(let sourceID, _, _)? = playbackDestination, route == "Source:\(sourceID)" { select("Folders") }
            }))
        .accessibilityValue(countText ?? progress.map { String(format: "%.0f percent", $0) } ?? status ?? count.map(String.init) ?? "")
        .accessibilityAddTraits(selectedSidebarRoute == (route ?? label) ? .isSelected : [])
        .dropDestination(for: String.self) { values, _ in
            guard let route, route.hasPrefix("List:"), let listID = Int64(route.dropFirst(5)) else { return false }
            let ids = LWSongDragPayload.decode(values)
            guard !ids.isEmpty else { return false }
            Task {
                do {
                    try await LWTrackMenu.addToList(songIDs: ids, listID: listID, listName: label, dependencies: dependencies)
                    NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
                } catch { dataError = error.localizedDescription }
            }
            return true
        } isTargeted: { targeted in
            guard let route, route.hasPrefix("List:") else { return }
            if targeted { targetedSidebarListRoute = route }
            else if targetedSidebarListRoute == route { targetedSidebarListRoute = nil }
        }
    }
    private var sourceScanCountLabel: String? {
        if let completed = sourceScanCompleted, let total = sourceScanTotal, total > 0 { return "\(completed)/\(total)" }
        return scannedSourceItems > 0 ? String(scannedSourceItems) : nil
    }
    private var selectedSidebarRoute: String? {
        guard !showSettings else { return nil }
        return navigation.current.sidebarRoute
    }
    private func sidebarSource(_ route: String?) -> Source? {
        guard let route, route.hasPrefix("Source:"), let id = Int64(route.dropFirst(7)) else { return nil }
        return sources.first { $0.id == id }
    }
    private func sidebarSourceSongs(_ route: String?) -> [Song] {
        guard let id = sidebarSource(route)?.id else { return [] }
        return songsBySource[id] ?? []
    }
    private func sidebarPlaylist(_ route: String?) -> Playlist? {
        guard let route, route.hasPrefix("List:"), let id = Int64(route.dropFirst(5)) else { return nil }
        return lists.first { $0.id == id }
    }
    private var onboarding: some View {
        WaveOnboardingView(dependencies: dependencies, onImported: {
            didStart = true
            select("Folders")
        }, onExplore: {
            didStart = true
            select("Albums")
        })
    }
    private var commandPalette: some View {
        LWCommandPalette(dependencies: dependencies, songs: songs, sources: sources,
            dismiss: { showCommands = false }, runCommand: { command in
                if command.hasPrefix("find:") {
                    let query = String(command.dropFirst(5))
                    if isWideLayout { pushDesktopFrame(LWRootNavigationFrame(section: "Find", query: query, showAll: true)) }
                    else {
                        navigation.reset(LWRootNavigationFrame(section: "Find", query: query, showAll: true))
                        contentEntry = .fade; contentEntryStartedAt = Date(); showCommands = false
                    }
                    libraryRootState.query = query; libraryRootState.showAllFindSongs = true
                    return
                }
                switch command {
                case "theme":
                    let current = LocalWavePreferences.current.string(forKey: "lwAppearance") ?? "system"
                    let dark = current == "dark" || (current == "system" && colorScheme == .dark)
                    LocalWavePreferences.current.set(dark ? "light" : "dark", forKey: "lwAppearance")
                case "shortcuts": showKeyboardHelp = true
                default: NotificationCenter.default.post(name: Notification.Name("LWCommand"), object: command)
                }
            }, navigate: { context in
                NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: context)
            })
    }
    private func refreshCounts() async {
        let revision = UUID()
        countsRevision = revision
        do {
            let loaded = try await dependencies.songRepository.searchSongsFTS(query: "", limit: Int.max, offset: 0)
            guard countsRevision == revision else { return }
            songs = loaded
            songsBySource = loaded.reduce(into: [:]) { groups, song in
                if let id = song.sourceId { groups[id, default: []].append(song) }
            }
            lists = try await dependencies.playlistRepo.getAll()
            for list in lists { if let id = list.id { playlistCounts[id] = try await dependencies.playlistSongRepo.getSongs(playlistId: id).count } }
            if let user = try await dependencies.userCloudService.resolveCurrentICloudUser(), let id = user.id {
                sources = try await dependencies.sourceService.repository().findOneByUserId(userId: id, path: nil)
            }
            let currentID = navigation.current.id
            navigation.reconcile(sourceIDs: Set(sources.compactMap(\.id)), listIDs: Set(lists.compactMap(\.id)))
            if navigation.current.id != currentID { contentEntry = nil; contentEntryStartedAt = nil; selectedSongs = [] }
            var count = 0
            var bytes: Int64 = 0
            for song in loaded {
                guard countsRevision == revision, !Task.isCancelled else { return }
                if await dependencies.sourceAccess.availability(for: song) == .onDevice { count += 1; bytes += song.fileSize ?? 0 }
            }
            if countsRevision == revision { offlineCount = count; offlineBytes = bytes }
        } catch { dataError = error.localizedDescription }
    }
}

/// Keep the two native picker overloads outside the root's lifecycle expression.
private struct LWNativeMusicImporters: ViewModifier {
    @Binding var folder: Bool
    @Binding var audio: Bool
    let complete: (Result<[URL], Error>, Bool) -> Void
    func body(content: Content) -> some View {
        content.fileImporter(isPresented: $folder, allowedContentTypes: [.folder], allowsMultipleSelection: true) { complete($0, true) }
            .fileImporter(isPresented: $audio, allowedContentTypes: [.audio], allowsMultipleSelection: true) { complete($0, false) }
    }
}
