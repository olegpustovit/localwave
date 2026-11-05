#if os(macOS)
import AVFoundation
import LocalWaveCore
import LocalWaveData
import LocalWaveDomain
import SwiftUI
import UniformTypeIdentifiers

@main
struct LocalWaveMac: App {
    @StateObject private var dependencies: DependencyContainer
    @StateObject private var player: PlayerViewModel
    @State private var songs: [Song] = []
    @State private var query = ""
    @State private var selection: Int64?
    @State private var sortOrder = [KeyPathComparator<MacSongRow>(\.title)]
    @State private var songRows: [MacSongRow] = []
    @State private var artists: [String] = []
    @State private var albums: [Album] = []
    @State private var playlists: [Playlist] = []
    @State private var chosenArtist: String?
    @State private var chosenAlbum: Album?
    @State private var chosenPlaylist: Playlist?
    @State private var selectedSource: Source?
    @State private var sourcePathStack: [Int64?] = []
    @State private var sourceFolderNames: [String] = []
    @State private var sourceItems: [SourcePath] = []
    @State private var folderPicker = false
    @State private var filePicker = false
    @State private var error: String?
    @State private var showingError = false
    @State private var section = "Songs"
    @StateObject private var syncViewModel: SyncViewModel

    init() {
        let container = try! DependencyContainer()
        _dependencies = StateObject(wrappedValue: container)
        _syncViewModel = StateObject(wrappedValue: container.makeSyncViewModel())
        _player = StateObject(wrappedValue: PlayerViewModel(
            playerPersistenceService: container.playerPersistenceService,
            songRepo: container.songRepository,
            playlistRepo: container.playlistRepo,
            playlistSongRepo: container.playlistSongRepo,
            sourceAccessResolver: { song in try await container.resolveSourceAccess(for: song) }
        ))
    }

    var body: some Scene {
        WindowGroup {
            NavigationSplitView {
                List(["Songs", "Albums", "Artists", "Folders", "Playlists", "Find"], id: \.self, selection: $section) { Text($0) }
                    .navigationTitle("LocalWave")
                    .dropDestination(for: URL.self) { urls, _ in
                        importURLs(.success(urls))
                        return true
                    }
                    .toolbar {
                        Menu {
                            Button("Add Folder…") { folderPicker = true }
                            Button("Add Audio Files…") { filePicker = true }
                        } label: { Label("Add Music", systemImage: "plus") }
                    }
            } detail: {
                VStack(spacing: 0) {
                    if section == "Folders" {
                        if let selectedSource {
                            sourceBrowser(selectedSource)
                        } else {
                            List(syncViewModel.sources, id: \.pathId) { source in
                                Button {
                                    selectedSource = source
                                    sourcePathStack = [source.pathId]
                                    sourceFolderNames = []
                                } label: {
                                    Label(URL(fileURLWithPath: source.dirPath).lastPathComponent, systemImage: "folder")
                                }.buttonStyle(.plain)
                            }
                            .overlay {
                                if syncViewModel.sources.isEmpty {
                                    ContentUnavailableView("Music Folders", systemImage: "folder", description: Text("Choose a folder to add your music."))
                                }
                            }
                        }
                    } else if section == "Albums" && chosenAlbum == nil {
                        albumList
                    } else if section == "Artists" && chosenArtist == nil {
                        artistList
                    } else if section == "Playlists" && chosenPlaylist == nil {
                        playlistList
                    } else {
                        songTable
                    }
                    Divider()
                    MacTransport(player: player)
                }
                .navigationTitle(section)
                .alert("LocalWave", isPresented: $showingError) { Button("OK") { error = nil } } message: { Text(error ?? "") }
                .fileImporter(isPresented: $folderPicker, allowedContentTypes: [.folder], allowsMultipleSelection: false) { result in
                    importURLs(result)
                }
                .fileImporter(isPresented: $filePicker, allowedContentTypes: [.audio], allowsMultipleSelection: true) { result in
                    importURLs(result)
                }
                .task { dependencies.handleAppLaunch(); syncViewModel.loadSources(); await reload(); await reloadCatalog() }
                .onChange(of: chosenAlbum?.id) { _, _ in Task { await reloadSelection() } }
                .onChange(of: chosenArtist) { _, _ in Task { await reloadSelection() } }
                .onChange(of: chosenPlaylist?.id) { _, _ in Task { await reloadSelection() } }
                .onChange(of: section) { _, newSection in
                    query = ""
                    chosenAlbum = nil; chosenArtist = nil; chosenPlaylist = nil
                    selectedSource = nil; sourcePathStack = []; sourceFolderNames = []; sourceItems = []
                    if newSection == "Folders" { syncViewModel.loadSources() }
                    Task { await reloadCatalog(); await reload() }
                }
                .toolbar {
                    if section == "Folders", selectedSource != nil {
                        Button("Back") {
                            if sourcePathStack.count > 1 {
                                sourcePathStack.removeLast()
                                sourceFolderNames.removeLast()
                            } else {
                                selectedSource = nil
                                sourceItems = []
                            }
                        }
                    } else if chosenAlbum != nil || chosenArtist != nil || chosenPlaylist != nil {
                        Button("Back") { chosenAlbum = nil; chosenArtist = nil; chosenPlaylist = nil }
                    }
                }
                .dropDestination(for: URL.self) { urls, _ in
                    importURLs(.success(urls))
                    return true
                }
            }
            .frame(minWidth: 800, minHeight: 520)
        }
        .commands {
            CommandGroup(after: .newItem) {
                Button("Add Folder…") { folderPicker = true }.keyboardShortcut("o")
                Button("Add Audio Files…") { filePicker = true }.keyboardShortcut("o", modifiers: [.command, .shift])
            }
            CommandMenu("Playback") {
                Button("Play or Pause") { player.playPause() }.keyboardShortcut(.space, modifiers: [])
                Button("Next Song") { player.nextSong() }.keyboardShortcut(.rightArrow, modifiers: [])
                Button("Previous Song") { player.previousSong() }.keyboardShortcut(.leftArrow, modifiers: [])
            }
            CommandMenu("Navigate") {
                Button("Find") { section = "Find" }.keyboardShortcut("f", modifiers: .command)
            }
        }
        Settings {
            Form {
                Slider(value: $player.volume, in: 0...1, label: { Text("Volume") })
                Toggle("Download upcoming tracks", isOn: Binding(
                    get: { player.downloadAheadEnabled },
                    set: { player.setDownloadAhead($0) }
                ))
            }
            .padding()
            .frame(width: 380)
            .defaultAppStorage(LocalWavePreferences.current)
        }
    }

    private var albumList: some View {
        ScrollView {
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), spacing: 16)], alignment: .leading, spacing: 16) {
                ForEach(albums) { album in
                    Button { chosenAlbum = album } label: {
                        VStack(alignment: .leading, spacing: 8) {
                            RoundedRectangle(cornerRadius: 10)
                                .fill(albumColor(album))
                                .overlay { Text(String(album.name.prefix(1))).font(.system(size: 34, weight: .medium)).foregroundStyle(.white) }
                                .frame(height: 110)
                            Text(album.name).lineLimit(1)
                            Text(album.artist ?? "Unknown Artist").font(.caption).foregroundStyle(.secondary).lineLimit(1)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding()
        }
        .overlay {
            if albums.isEmpty { ContentUnavailableView("No albums", systemImage: "square.stack", description: Text("Albums appear as music is added.")) }
        }
        .searchable(text: $query, prompt: "Search albums")
        .onChange(of: query) { _, _ in Task { await reloadCatalog() } }
    }

    private var artistList: some View {
        List(artists, id: \.self) { artist in
            Button(artist) { chosenArtist = artist }.buttonStyle(.plain)
        }
        .searchable(text: $query, prompt: "Search artists")
        .onChange(of: query) { _, _ in Task { await reloadCatalog() } }
    }

    private var playlistList: some View {
        List(playlists) { playlist in
            Button(playlist.name) { chosenPlaylist = playlist }.buttonStyle(.plain)
        }
    }

    private var songTable: some View {
        Group {
            if section == "Songs" && songRows.isEmpty && query.isEmpty {
                HStack(spacing: 40) {
                    VStack(alignment: .leading, spacing: 12) {
                        Image(systemName: "waveform").font(.system(size: 34)).foregroundStyle(.tint)
                        Text("Your music, locally.").font(.largeTitle.bold())
                        Text("Add a folder or drop music here to build your library.")
                            .foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                        Text("0 songs").font(.caption).foregroundStyle(.secondary)
                        Button("Choose Music Folder…") { folderPicker = true }.buttonStyle(.borderedProminent)
                        Button("Try the demo library") {
                            Task { do { try await LWDemoLibrary.installIfEmpty(dependencies: dependencies); await reload() } catch { self.error = error.localizedDescription; showingError = true } }
                        }
                    }
                    VStack(alignment: .leading, spacing: 16) {
                        Label("Choose a folder or audio files", systemImage: "folder.badge.plus")
                        Label("Drop folders or audio files into this window", systemImage: "arrow.down.doc")
                        Label("Browse, search and play your collection", systemImage: "music.note.list")
                    }
                    .foregroundStyle(.secondary)
                }
                .padding(40)
                .frame(maxWidth: 700, maxHeight: .infinity)
            } else {
                Table(songRows, selection: $selection, sortOrder: $sortOrder) {
                    TableColumn("Title", value: \MacSongRow.title)
                    TableColumn("Artist", value: \MacSongRow.artist)
                    TableColumn("Album", value: \MacSongRow.album)
                    TableColumn("Duration", value: \MacSongRow.durationSort) { row in
                        Text(row.durationText).monospacedDigit()
                    }
                }
            }
        }
        .searchable(text: $query, prompt: "Search songs")
        .onChange(of: query) { _, _ in Task { await reload() } }
        .onChange(of: sortOrder) { _, newOrder in songRows.sort(using: newOrder) }
        .onChange(of: selection) { _, id in
            guard let id, let index = songRows.firstIndex(where: { $0.id == id }) else { return }
            player.configureQueue(songs: songs, startIndex: songs.firstIndex(where: { $0.uniqueId == id }) ?? index, context: .library)
            player.playQueueEntry(at: songs.firstIndex(where: { $0.uniqueId == id }) ?? index)
        }
    }

    private func importURLs(_ result: Result<[URL], Error>) {
        Task {
            do {
                _ = try await dependencies.addMusicURLs(result.get())
                syncViewModel.loadSources()
                await reload()
            } catch let failure { error = failure.localizedDescription; showingError = true }
        }
    }

    @ViewBuilder private func sourceBrowser(_ source: Source) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(([URL(fileURLWithPath: source.dirPath).lastPathComponent] + sourceFolderNames).joined(separator: " / "))
                .font(.headline).padding()
            List(sourceItems, id: \.pathId) { item in
                Button {
                    if item.isDirectory {
                        sourcePathStack.append(item.pathId)
                        sourceFolderNames.append(item.name)
                    } else if item.isAudio {
                        Task { await playSourceItem(item, source: source) }
                    }
                } label: {
                    Label(item.name, systemImage: item.isDirectory ? "folder" : "waveform")
                }
                .buttonStyle(.plain)
                .disabled(!item.isDirectory && !item.isAudio)
            }
            .overlay {
                if sourceItems.isEmpty { ContentUnavailableView("No music here", systemImage: "music.note", description: Text("This folder has no indexed items.")) }
            }
        }
        .task(id: sourcePathStack) {
            await loadSourceItems(source, parent: sourcePathStack.last ?? source.pathId)
        }
    }

    @MainActor private func loadSourceItems(_ source: Source, parent: Int64?) async {
        guard let sourceID = source.id else { return }
        do { sourceItems = try await dependencies.sourceService.importService().listItems(sourceId: sourceID, parentPathId: parent) }
        catch { self.error = error.localizedDescription; showingError = true }
    }

    @MainActor private func playSourceItem(_ item: SourcePath, source: Source) async {
        guard let sourceID = source.id, let song = await dependencies.songRepository.getSong(sourceId: sourceID, relativePath: item.relativePath) else { return }
        player.configureQueue(songs: [song], startIndex: 0, context: .folder(sourceID: sourceID, pathID: item.pathId, name: item.name))
        player.playQueueEntry(at: 0)
    }

    @MainActor private func reload() async {
        do {
            if let chosenAlbum { songs = try await dependencies.songRepository.getSongsByAlbum(chosenAlbum.name, artist: chosenAlbum.artist) }
            else if let chosenArtist { songs = try await dependencies.songRepository.getSongsByArtist(chosenArtist) }
            else if let chosenPlaylist, let id = chosenPlaylist.id { songs = try await dependencies.playlistSongRepo.getSongs(playlistId: id) }
            else { songs = try await dependencies.songRepository.searchSongsFTS(query: section == "Find" || section == "Songs" ? query : "", limit: Int.max, offset: 0) }
            songRows = songs.map(MacSongRow.init).sorted(using: sortOrder)
            if let selection, !songRows.contains(where: { $0.id == selection }) { self.selection = nil }
        } catch let failure { error = failure.localizedDescription; showingError = true }
    }

    @MainActor private func reloadCatalog() async {
        do {
            if section == "Albums" { albums = try await dependencies.songRepository.getAllAlbums().filter { query.isEmpty || $0.name.localizedCaseInsensitiveContains(query) || ($0.artist?.localizedCaseInsensitiveContains(query) ?? false) } }
            if section == "Artists" { artists = try await dependencies.songRepository.getAllArtists().filter { query.isEmpty || $0.localizedCaseInsensitiveContains(query) } }
            if section == "Playlists" { playlists = try await dependencies.playlistRepo.getAll() }
        } catch let failure { error = failure.localizedDescription; showingError = true }
    }

    @MainActor private func reloadSelection() async { selection = nil; await reload() }
    private func albumColor(_ album: Album) -> Color {
        let colors: [Color] = [.blue, .purple, .teal, .orange, .pink, .indigo]
        let first = album.name.unicodeScalars.first?.value ?? 0
        return colors[Int(first) % colors.count]
    }

    private func time(_ seconds: Double) -> String { String(format: "%d:%02d", Int(seconds) / 60, Int(seconds) % 60) }
}

private struct MacSongRow: Identifiable {
    let id: Int64
    let song: Song
    let title: String
    let artist: String
    let album: String
    let durationSort: String
    let durationText: String

    init(_ song: Song) {
        id = song.uniqueId
        self.song = song
        title = song.title
        artist = song.artist
        album = song.album
        let seconds = song.duration ?? 0
        durationSort = String(format: "%012.3f", seconds)
        durationText = String(format: "%d:%02d", Int(seconds) / 60, Int(seconds) % 60)
    }
}
private struct MacTransport: View {
    @ObservedObject var player: PlayerViewModel

    var body: some View {
        HStack(spacing: 16) {
            Button { player.previousSong() } label: { Image(systemName: "backward.end.fill") }.disabled(player.currentSong == nil)
            Button { player.playPause() } label: { Image(systemName: player.isPlaying ? "pause.fill" : "play.fill") }.disabled(player.currentSong == nil)
            Button { player.nextSong() } label: { Image(systemName: "forward.end.fill") }.disabled(player.currentSong == nil)
            VStack(alignment: .leading) {
                Text(player.currentSong?.title ?? "Nothing playing").lineLimit(1)
                Text(player.currentSong?.artist ?? "Choose a song to play").foregroundStyle(.secondary).lineLimit(1)
            }.frame(maxWidth: .infinity, alignment: .leading)
            Slider(value: $player.volume, in: 0...1).frame(width: 90).accessibilityLabel("Volume")
            Text(player.currentTime).monospacedDigit()
            Slider(value: Binding(get: { player.playbackProgress }, set: { player.seek(to: $0) }), in: 0...1).frame(width: 150).disabled(player.currentSong == nil)
        }
        .padding(12)
    }
}
#endif
