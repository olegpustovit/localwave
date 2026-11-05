import SwiftUI
import LocalWaveDomain

struct LWCommandPalette: View {
    let dependencies: DependencyContainer
    let songs: [Song]
    let sources: [Source]
    let dismiss: () -> Void
    let runCommand: (String) -> Void
    let navigate: (PlaybackContext) -> Void
    @EnvironmentObject private var player: PlayerViewModel
    @State private var query = ""
    @State private var selected = 0
    @State private var folders: [FolderMatch] = []
    @State private var matchedSongs: [Song] = []
    @State private var searchError: String?
    @FocusState private var focused: Bool

    private struct SearchInput: Equatable {
        let query: String
        let songs: [Song]
        let sourceIDs: [Int64]
    }

    private struct FolderMatch { let source: Source; let path: SourcePath }
    private struct Item: Identifiable {
        let id: String
        let group: String
        let label: String
        var hint = ""
        var icon = "chev"
        var song: Song?
        let action: () -> Void
    }
    private var commands: [Item] {
        var result = [
            Item(id: "playPause", group: "Commands", label: "Play / pause", hint: "Space", action: { runCommand("playPause") }),
            Item(id: "shuffle", group: "Commands", label: "Shuffle on or off", hint: "S", action: { runCommand("shuffle") }),
            Item(id: "repeat", group: "Commands", label: "Cycle repeat", hint: "R", action: { runCommand("repeat") }),
            Item(id: "addMusic", group: "Commands", label: "Add folder or files…", hint: "⌘O", action: { runCommand("addMusic") })
        ]
        if let source = sources.first(where: { $0.dirPath.localizedCaseInsensitiveContains("Google Drive") }) {
            result.append(Item(id: "rescan", group: "Commands", label: "Rescan Google Drive", action: { dependencies.waveSyncViewModel.resyncSource(source) }))
        }
        result += [
            Item(id: "network", group: "Commands", label: "Simulate no network on / off", action: { runCommand("network") }),
            Item(id: "theme", group: "Commands", label: "Switch light / dark", action: { runCommand("theme") }),
            Item(id: "panel", group: "Commands", label: "Minimise or restore the player", hint: "I", action: { runCommand("panel") }),
            Item(id: "shortcuts", group: "Commands", label: "Show keyboard shortcuts", hint: "?", action: { runCommand("shortcuts") }),
            Item(id: "settings", group: "Commands", label: "Settings", hint: "⌘,", action: { runCommand("Settings") })
        ]
        return result
    }
    private var items: [Item] {
        let text = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else {
            let automatic = ["Favorites", "Recently Added", "Most Played"].map { name in
                Item(id: "auto:\(name)", group: "Lists", label: name == "Recently Added" ? "Recently added" : name == "Most Played" ? "Most played" : name, icon: "queue", action: { navigate(.automatic(name)) })
            }
            return commands.filter { $0.id != "settings" } + automatic
        }
        let matched = matchedSongs
        var result = matched.prefix(6).map { song in
            Item(id: "song:\(song.uniqueId)", group: "Songs", label: song.title, hint: song.artist, song: song, action: {
                player.configureQueue(songs: matched, startIndex: matched.firstIndex(where: { $0.uniqueId == song.uniqueId }) ?? 0, context: .selection(text))
                player.playSong(song)
            })
        }
        var albumIDs = Set<String>()
        let albums = songs.filter { ($0.album + " " + $0.artist).localizedCaseInsensitiveContains(text) && albumIDs.insert($0.album + "\u{1f}" + $0.artist).inserted }
        result += albums.prefix(3).map { song in
            Item(id: "album:\(song.album):\(song.artist)", group: "Albums", label: song.album, hint: song.artist, song: song, action: { navigate(.album(name: song.album, artist: song.artist)) })
        }
        result += Array(Set(songs.map(\.artist))).sorted().filter { $0.localizedCaseInsensitiveContains(text) }.prefix(3).map { artist in
            Item(id: "artist:\(artist)", group: "Artists", label: artist, icon: "person", action: { navigate(.artist(artist)) })
        }
        result += folders.prefix(3).map { match in
            Item(id: "folder:\(match.path.sourceId):\(match.path.pathId)", group: "Folders", label: match.path.name, hint: URL(fileURLWithPath: match.source.dirPath).lastPathComponent, icon: "folder", action: { navigate(.folder(sourceID: match.path.sourceId, pathID: match.path.pathId, name: match.path.name)) })
        }
        if matched.count > 6 {
            result.append(Item(id: "allSongs", group: "Songs", label: "Show all \(matched.count) songs", icon: "rows", action: { runCommand("find:" + text) }))
        }
        result += commands.filter { $0.label.localizedCaseInsensitiveContains(text) }
        return result
    }
    var body: some View {
        GeometryReader { bounds in
            let rows = items
            let groupCount = rows.enumerated().filter { $0.offset == 0 || rows[$0.offset - 1].group != $0.element.group }.count
            ZStack(alignment: .top) {
                Color(red: 10/255, green: 10/255, blue: 10/255).opacity(0.28).ignoresSafeArea().onTapGesture(perform: dismiss)
                VStack(spacing: 0) {
                    TextField("Search songs, folders, or type a command", text: $query,
                              prompt: Text("Search songs, folders, or type a command").foregroundStyle(Color(white: 117.0 / 255)))
                        .font(LWTheme.text(20)).textFieldStyle(.plain).padding(.horizontal, 20).frame(height: 59)
                        .focused($focused).onSubmit { activate(rows, index: selected) }
                        .autocorrectionDisabled().textInputAutocapitalization(.never)
                    Rectangle().fill(LWTheme.line).frame(height: 1)
                    ScrollViewReader { proxy in
                        ScrollView {
                            LazyVStack(spacing: 0) {
                                ForEach(Array(rows.enumerated()), id: \.element.id) { index, item in
                                    if index == 0 || rows[index - 1].group != item.group {
                                        Text(item.group).font(LWTheme.text(12)).lwLineBox(12, 16.8)
                                            .foregroundStyle(LWTheme.secondary).frame(maxWidth: .infinity, alignment: .leading)
                                            .padding(.horizontal, 12).padding(.top, 10).padding(.bottom, 4)
                                    }
                                    Button { activate(rows, index: index) } label: {
                                        HStack(spacing: 12) {
                                            if let song = item.song { LWArtwork(song: song, size: 28) }
                                            else { LWIcon(item.icon, size: 16) }
                                            waveHighlightedLabel(item.label, query: query).font(LWTheme.text(14)).lwLineBox(14, 19.6).lineLimit(1)
                                            Spacer(minLength: 0)
                                            Text(item.hint).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).lineLimit(1)
                                        }.padding(.horizontal, 12).frame(height: 44).background(index == selected ? LWTheme.panel2 : Color.clear).contentShape(Rectangle())
                                    }.id(item.id).accessibilityAddTraits(index == selected ? .isSelected : [])
                                }
                                if rows.isEmpty { Text(searchError ?? "No matches.").font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary).padding(24) }
                            }.padding(6)
                        }.onChange(of: selected) { _, index in if rows.indices.contains(index) { proxy.scrollTo(rows[index].id) } }
                    }.frame(height: min(420, max(60, CGFloat(rows.count) * 44 + CGFloat(groupCount) * 30.8 + 12), max(60, bounds.size.height - 176)))
                }.background(LWTheme.paper)
                    .background { Rectangle().fill(.black.opacity(0.4)).padding(20).blur(radius: 40).offset(y: 30) }
                    .overlay { Rectangle().inset(by: -0.5).stroke(LWTheme.line, lineWidth: 1) }
                    .frame(maxWidth: 600).padding(.horizontal, 16).padding(.top, min(96, bounds.size.height * 0.12))
                    .accessibilityElement(children: .contain).accessibilityLabel("Search and commands")
                    .accessibilityAddTraits(.isModal)
                    .accessibilityAction(.escape, dismiss)
            }
        }
        .onAppear { focused = true }
        .onChange(of: query) { _, _ in selected = 0; matchedSongs = []; folders = [] }
        .onKeyPress(.downArrow) { selected = items.isEmpty ? 0 : (selected + 1) % items.count; return .handled }
        .onKeyPress(.upArrow) { selected = items.isEmpty ? 0 : (selected + items.count - 1) % items.count; return .handled }
        .onKeyPress(.escape) { dismiss(); return .handled }
        .task(id: SearchInput(query: query, songs: songs, sourceIDs: sources.map(\.stableId))) { await searchCatalogue() }
    }
    private func activate(_ rows: [Item], index: Int) {
        guard rows.indices.contains(index) else { return }
        dismiss(); rows[index].action()
    }
    private func searchCatalogue() async {
        folders = []; searchError = nil
        let text = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        do {
            try await Task.sleep(for: .milliseconds(150))
            let filter = WaveSongQuery(text)
            let candidates = try await dependencies.songRepository.searchSongsFTS(query: filter.text, limit: Int.max, offset: 0)
            var matches: [Song] = []
            for song in candidates {
                try Task.checkCancellation()
                let availability = filter.hasFilters ? await dependencies.sourceAccess.availability(for: song) : .unknown
                if filter.matches(song, availability: availability) { matches.append(song) }
            }
            var found: [FolderMatch] = []
            for source in sources where !filter.text.isEmpty {
                guard let id = source.id else { continue }
                let paths = try await dependencies.sourceService.importService().search(sourceId: id, query: filter.text)
                found += paths.filter(\.isDirectory).map { FolderMatch(source: source, path: $0) }
            }
            try Task.checkCancellation()
            matchedSongs = matches
            folders = found
        } catch is CancellationError { }
        catch { searchError = error.localizedDescription }
    }
}

struct LWKeyboardHelp: View {
    let dismiss: () -> Void
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    private let shortcuts = [
        ("Play / pause", "Space"), ("Seek 5 s · 30 s", "← →  ·  ⌥← ⌥→"),
        ("Previous / next song", "⇧← ⇧→"), ("Search and commands", "⌘K or /"),
        ("Play selected song", "Return"), ("Move selection", "↑ ↓"),
        ("Shuffle · repeat", "S · R"), ("Small player / full panel", "I"),
        ("Favorite current song", "L"), ("Volume", "⌘↑ ⌘↓"),
        ("Back", "⌘["), ("Close panels", "Esc")
    ]
    var body: some View {
        GeometryReader { bounds in
            ZStack(alignment: .top) {
                Color(red: 10/255, green: 10/255, blue: 10/255).opacity(0.28).ignoresSafeArea().onTapGesture(perform: dismiss)
                Group {
                    if dynamicTypeSize.isAccessibilitySize {
                        ScrollView { contents }.scrollIndicators(.hidden)
                            .frame(maxHeight: max(60, bounds.size.height - min(96, bounds.size.height * 0.12) - 16))
                    } else { contents }
                }.background(LWTheme.paper)
                    .overlay { Rectangle().inset(by: -0.5).stroke(LWTheme.line, lineWidth: 1) }
                    .frame(width: min(520, max(0, bounds.size.width - 32)))
                    .padding(.top, min(96, bounds.size.height * 0.12))
                    .accessibilityElement(children: .contain).accessibilityLabel("Keyboard shortcuts")
                    .accessibilityAddTraits(.isModal)
                    .accessibilityAction(.escape, dismiss)
            }
        }.onKeyPress(.escape) { dismiss(); return .handled }
    }
    private var contents: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Keyboard").font(LWTheme.text(18).weight(.medium)).lwLineBox(18, 25.2)
            if dynamicTypeSize.isAccessibilitySize {
                VStack(alignment: .leading, spacing: 16) {
                    ForEach(shortcuts, id: \.0) { label, key in
                        VStack(alignment: .leading, spacing: 4) {
                            Text(label).font(LWTheme.text(14)).lwLineBox(14, 19.6)
                            Text(key).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary)
                        }.fixedSize(horizontal: false, vertical: true)
                    }
                }
            } else {
                Grid(horizontalSpacing: 16, verticalSpacing: 8) {
                    ForEach(shortcuts, id: \.0) { label, key in
                        GridRow {
                            Text(label).font(LWTheme.text(14)).lwLineBox(14, 19.6).frame(maxWidth: .infinity, alignment: .leading)
                            Text(key).font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary).fixedSize().frame(maxWidth: .infinity, alignment: .trailing)
                        }
                    }
                }
            }
        }.padding(.horizontal, 24).padding(.vertical, 22).frame(maxWidth: .infinity, alignment: .leading)
    }

}
