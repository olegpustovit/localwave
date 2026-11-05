import AVFoundation
import AVKit
import LocalWaveCore
import LocalWaveDomain
import LocalWaveData
import SwiftUI
import CoreTransferable
import UniformTypeIdentifiers

struct WavePlaybackKeys: View {
    @EnvironmentObject private var player: PlayerViewModel
    @Environment(\.lwReduceMotion) private var reduceMotion
    var desktop = false
    @State private var hovered: String?
    var body: some View {
        HStack(spacing: 2) {
            key("prev", label: "Previous", action: player.previousSong)
            Button(action: player.playPause) {
                Group {
                    if player.isLoading { Rectangle().frame(width: 12, height: 12).frame(width: desktop ? 22 : 28, height: desktop ? 22 : 28) }
                    else { LWIcon(player.isPlaying ? "pause" : "play", size: desktop ? 22 : 28).modifier(LWPop(trigger: player.isPlaying)) }
                }
                .frame(maxWidth: desktop ? nil : .infinity)
                .frame(width: desktop ? 68 : nil, height: desktop ? 71 : 84)
                .foregroundStyle(LWTheme.onInk)
                .opacity(desktop && hovered == "play" ? 0.9 : 1)
                .overlay(alignment: .bottom) { if player.isLoading { WaveLoadingFill().frame(height: 4) } }
            }
            .buttonStyle(LWKeyStyle(pressedOpacity: desktop ? 1 : 0.88, background: LWTheme.ink, pressedBackground: desktop ? LWTheme.ink : LWTheme.paper))
            .onHover { hovered = $0 ? "play" : nil }
            .accessibilityLabel(player.isLoading ? "Cancel opening file" : player.isPlaying ? "Pause" : "Play")
            .accessibilityIdentifier("player.playPause")
            key("next", label: "Next", action: player.nextSong)
        }
        .padding(.leading, desktop ? 0 : 2).padding(.trailing, 2).padding(.top, desktop ? 0 : 2)
        .frame(height: desktop ? 71 : 86).background(LWTheme.panel2)
        .buttonStyle(LWKeyStyle())
        .disabled(player.currentSong == nil)
    }
    private func key(_ icon: String, label: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            LWIcon(icon, size: desktop ? 20 : 28)
                .frame(maxWidth: desktop ? nil : .infinity)
                .frame(width: desktop ? 68 : nil, height: desktop ? 71 : 84)
                .foregroundStyle(LWTheme.ink)
        }.buttonStyle(LWKeyStyle(background: desktop ? (hovered == icon ? LWTheme.paper : LWTheme.key) : LWTheme.key2, pressedBackground: desktop ? nil : LWTheme.paper))
            .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: hovered)
            .accessibilityLabel(label).onHover { hovered = $0 ? icon : nil }
    }
}

struct WaveTransport: View {
    @EnvironmentObject private var player: PlayerViewModel
    @EnvironmentObject private var dependencies: DependencyContainer
    @State private var libraryCount = 0
    @State private var shuffling = false
    let compact: Bool
    @ScaledMetric(relativeTo: .caption) private var timeWidth: CGFloat = 44
    var panelVisible = true
    var onExpand: () -> Void

    var body: some View {
        Group {
            if compact {
                HStack(spacing: 0) {
                    Button(action: openOrShuffle) {
                        HStack(spacing: 12) {
                            if player.currentSong != nil { LWArtwork(song: player.currentSong, size: 40, matched: true) }
                            else { LWIcon("shuf", size: 18).frame(width: 40, height: 40).background(LWTheme.panel) }
                            VStack(alignment: .leading, spacing: 0) {
                                Text(player.currentSong?.title ?? "Shuffle everything").font(LWTheme.text(15)).lwLineBox(15, 21).frame(minHeight: 21)
                                Text(player.isLoading ? "Opening file… · tap ■ to cancel" : player.currentSong?.artist ?? "\(libraryCount) songs").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).frame(minHeight: 18.2)
                            }.lineLimit(1).fixedSize(horizontal: false, vertical: true)
                            Spacer(minLength: 0)
                        }.padding(.leading, 16).padding(.trailing, 12).padding(.vertical, 11.5)
                            .frame(maxWidth: .infinity, minHeight: 63)
                            .contentShape(Rectangle())
                    }.accessibilityLabel(player.currentSong == nil ? "Shuffle everything" : "Open now playing")
                    Button {
                        if player.currentSong == nil { openOrShuffle() } else { player.playPause() }
                    } label: {
                        Group {
                            if player.isLoading { Rectangle().frame(width: 10, height: 10).frame(width: 22, height: 22) }
                            else { LWIcon(player.isPlaying ? "pause" : "play", size: 22).modifier(LWPop(trigger: player.isPlaying)) }
                        }.frame(width: 64)
                            .frame(minHeight: 63, maxHeight: .infinity)
                            .foregroundStyle(LWTheme.onInk).background(LWTheme.ink)
                            .overlay(alignment: .bottom) { if player.isLoading { WaveLoadingFill().frame(height: 4) } }
                    }.buttonStyle(LWKeyStyle()).accessibilityLabel(player.isLoading ? "Cancel opening file" : player.isPlaying ? "Pause" : "Play")
                        .disabled(shuffling || (player.currentSong == nil && libraryCount == 0))
                }
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 1)
                .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1) }
                .overlay(alignment: .topLeading) {
                    GeometryReader { proxy in
                        Rectangle().fill(LWTheme.ink).frame(width: proxy.size.width * player.playbackProgress, height: 2)
                    }.frame(height: 2).offset(y: -1).allowsHitTesting(false)
                }
            } else {
                HStack(spacing: 16) {
                    WavePlaybackKeys(desktop: true).frame(width: 210)
                    if !panelVisible, let song = player.currentSong {
                        Button(action: onExpand) {
                            HStack(spacing: 10) {
                                LWArtwork(song: player.currentSong, size: 44)
                                VStack(alignment: .leading, spacing: 0) {
                                    Text(player.currentSong?.title ?? "Nothing playing").font(LWTheme.text(15)).lwLineBox(15, 21).frame(minHeight: 21)
                                    Text(song.artist + " — " + song.album).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary).frame(minHeight: 18.2)
                                }.lineLimit(1).fixedSize(horizontal: false, vertical: true)
                                Spacer(minLength: 0)
                            }.frame(width: 240)
                        }.accessibilityLabel("Open now playing")
                            .modifier(LWAppear(translation: CGSize(width: 0, height: 10)))
                            .transition(.identity)
                    }
                    Text(player.currentTime).font(LWTheme.data()).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary).frame(width: timeWidth, alignment: .leading)
                    WaveformView(song: player.currentSong, sourceURL: player.playbackURL, progress: player.playbackProgress, height: 40, barCount: 200, seek: player.seekByFraction)
                    Text(player.remainingTime).font(LWTheme.data()).lwLineBox(12, 16.8, data: true).foregroundStyle(LWTheme.secondary).frame(width: timeWidth, alignment: .trailing)
                    WaveModeKeys(showQueue: false, panelVisible: panelVisible, onQueue: onExpand)
                    #if targetEnvironment(macCatalyst)
                    WaveVolumeLevels().frame(width: 106, height: 44)
                    #else
                    WaveAudioRoute(background: LWTheme.panel).frame(width: 44, height: 44)
                    #endif
                }.padding(.trailing, 20)
                    .frame(minHeight: 71).fixedSize(horizontal: false, vertical: true)
            }
        }
        .foregroundStyle(LWTheme.ink).background(compact ? LWTheme.paper : LWTheme.panel)
        .buttonStyle(.plain)
        .task { libraryCount = (try? await dependencies.songRepository.totalSongCount(query: "")) ?? 0 }
        .onReceive(NotificationCenter.default.publisher(for: Notification.Name("SongListRefresh"))) { _ in
            Task { libraryCount = (try? await dependencies.songRepository.totalSongCount(query: "")) ?? 0 }
        }
    }

    private func openOrShuffle() {
        guard player.currentSong == nil else { onExpand(); return }
        guard !shuffling else { return }
        shuffling = true
        Task {
            defer { shuffling = false }
            do {
                let songs = try await dependencies.songRepository.searchSongsFTS(query: "", limit: Int.max, offset: 0).shuffled()
                guard let first = songs.first else { return }
                player.setShuffle(false)
                player.configureQueue(songs: songs, startIndex: 0)
                player.setShuffle(true)
                player.playSong(first)
            } catch { player.playbackError = error.localizedDescription }
        }
    }
}

struct WaveNowView: View {
    @EnvironmentObject private var player: PlayerViewModel
    @EnvironmentObject private var dependencies: DependencyContainer
    @EnvironmentObject private var libraryModel: WaveLibraryModel
    @EnvironmentObject private var presenter: LWSheetPresenter
    @Environment(\.lwReduceMotion) private var reduceMotion
    let compact: Bool
    var onClose: (() -> Void)? = nil
    var artmatchEnabled = true
    var closing = false
    @State private var queuePresented = false
    @State private var appeared = false
    @State private var favoriteBurst = 0
    @State private var error: String?
    @State private var songToAdd: Song?
    @State private var sharing = false
    @State private var sharedFile: LWPreparedShare?
    @State private var shareDirectory: URL?
    @State private var sourceName = "Source folder"
    @State private var measuredInformationHeight: CGFloat = 50.6
    @State private var panelButtonHovered = false
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize

    var body: some View {
        GeometryReader { geometry in
            if compact {
                VStack(spacing: 0) {
                    Group {
                        if player.currentSong == nil {
                            VStack(spacing: 0) { nowHeader; emptyState }
                        }
                        else {
                            VStack(spacing: 0) {
                                nowHeader
                                ScrollView { nowContent(geometry.size) }
                            }
                        }
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(LWTheme.paper)
                    .modifier(LWAppear(translation: .zero, duration: LWMotion.base))
                    // The reference's Now section overlaps its 86pt keys by 2pt.
                    .offset(y: closing ? max(0, geometry.size.height - 84) : 0)
                    WavePlaybackKeys()
                }.foregroundStyle(LWTheme.ink)
            }
            else if player.currentSong == nil { emptyState }
            else {
                VStack(spacing: 0) {
                    nowHeader
                    if dynamicTypeSize.isAccessibilitySize {
                        ScrollView {
                            nowContent(geometry.size)
                            if !compact { WaveQueueRows(scrolls: false).padding(.top, 14) }
                        }
                    } else {
                        nowContent(geometry.size)
                        WaveQueueRows().padding(.top, 1)
                            .padding(.top, 14)
                            .overlay(alignment: .top) { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.top, 14) }
                    }
                }
                .background(LWTheme.paper).foregroundStyle(LWTheme.ink)
            }
        }
        .buttonStyle(.plain)
        .onAppear { appeared = true }
        .task {
            if libraryModel.songs.isEmpty && !libraryModel.loading { await libraryModel.reload() }
        }
        .task(id: player.currentSong?.sourceId) {
            sourceName = "Source folder"
            if let id = player.currentSong?.sourceId, let source = try? await dependencies.sourceService.repository().getOne(id: id) {
                sourceName = URL(fileURLWithPath: source.dirPath).lastPathComponent
            }
        }
        #if DEBUG
        .task {
            guard LWReferenceFixture.enabled else { return }
            await Task.yield()
            if LWReferenceFixture.screen == "queue" { queuePresented = true }
            if LWReferenceFixture.screen == "track" { presentTrackMenu() }
        }
        #endif
        .lwSheet(isPresented: $queuePresented) { WaveQueueSheet() }
        .sheet(item: $sharedFile, onDismiss: {
            if let shareDirectory { try? FileManager.default.removeItem(at: shareDirectory) }
            shareDirectory = nil
        }) { item in LWShareSheet(urls: item.urls) }
        .sheet(item: $songToAdd) { song in
            PlaylistSelectionView(song: song, songRepo: dependencies.songRepository, playlistRepo: dependencies.playlistRepo, playlistSongRepo: dependencies.playlistSongRepo)
        }
        .alert("Playback", isPresented: Binding(get: { error != nil || player.playbackError != nil }, set: { if !$0 { error = nil; player.playbackError = nil } })) {
            Button("OK") { error = nil; player.playbackError = nil }
        } message: { Text(error ?? player.playbackError ?? "") }
    }

    private var nowHeader: some View {
        Group {
            if compact {
                HStack(spacing: 0) {
                    HStack {
                        Button { onClose?() } label: { LWIcon("down", size: 24).frame(width: 44, height: 44) }
                            .buttonStyle(LWIconButtonStyle())
                            .accessibilityLabel("Close now playing")
                        Spacer(minLength: 0)
                    }.frame(width: 88)
                    Button { openContext(player.playbackContext) } label: {
                        (Text("Playing from ").foregroundColor(LWTheme.secondary) + Text(player.playbackContext.title).fontWeight(.medium).foregroundColor(LWTheme.ink))
                            .font(LWTheme.text(13)).lineLimit(1).lwLineBox(13, 18.2).frame(maxWidth: .infinity, minHeight: 44)
                    }.accessibilityLabel("Playing from " + player.playbackContext.title)
                    WaveAudioRoute().frame(width: 44, height: 44)
                    Button(action: presentTrackMenu) { LWIcon("more", size: 22).frame(width: 44, height: 44) }
                        .buttonStyle(LWIconButtonStyle())
                        .accessibilityLabel("Track actions")
                }.padding(.horizontal, 6).frame(minHeight: 48)
                    .modifier(WaveEntrance(appeared: appeared, index: 0))
            } else {
                HStack(spacing: 0) {
                    Text("Now playing").font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
                    Spacer()
                    Button {
                        NotificationCenter.default.post(name: Notification.Name("LWToggleNowPanel"), object: nil)
                    } label: { LWIcon("side", size: 18).frame(width: 36, height: 36) }
                        .background(panelButtonHovered ? LWTheme.panel : Color.clear)
                        .onHover { panelButtonHovered = $0 }
                        .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: panelButtonHovered)
                        .accessibilityLabel("Minimise to small player")
                }.padding(.leading, 20).padding(.trailing, 10).padding(.top, 8)
            }
        }
    }

    private func nowContent(_ size: CGSize) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            LWArtwork(song: player.currentSong, size: artworkSize(size), matched: compact && artmatchEnabled)
                .padding(.top, 4)
                .frame(maxWidth: .infinity, alignment: compact ? .center : .leading)
                .frame(height: compact ? artworkRegionHeight(size) : 204)
                .contextMenu { trackActions }
            trackInformation
                .padding(.top, compact ? 16 : 14)
                .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { measuredInformationHeight = $0 - (compact ? 16 : 14) }
                .modifier(WaveEntrance(appeared: appeared, index: 1))
            playbackClock.padding(.top, 14)
                .modifier(WaveEntrance(appeared: appeared, index: 2))
            if compact {
                VStack(spacing: 6) {
                    WaveformView(song: player.currentSong, sourceURL: player.playbackURL, progress: player.playbackProgress, height: 56, seek: player.seekByFraction)
                    HStack { Text("0:00"); Spacer(); Text(player.duration) }
                        .font(LWTheme.data(11)).lwLineBox(11, 15.4, data: true).foregroundStyle(LWTheme.secondary)
                        .frame(minHeight: 15.4)
                }.padding(.top, 14).modifier(WaveEntrance(appeared: appeared, index: 3))
                WaveModeKeys(showQueue: true) { queuePresented = true }
                    .padding(.top, 14).modifier(WaveEntrance(appeared: appeared, index: 4))
            } else { metadata.padding(.top, 14) }
        }
        .padding(.horizontal, compact ? 16 : 20)
        .padding(.bottom, compact ? 10 : 0)
    }

    private var trackInformation: some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: compact ? 2 : 0) {
                Text(player.currentSong?.title ?? "Nothing playing")
                    .font(LWTheme.text(compact ? 24 : 19).weight(.medium))
                    .tracking(compact ? -0.72 : -0.285)
                    .lwLineBox(compact ? 24 : 19, compact ? 27.6 : 22.8)
                    .frame(minHeight: compact ? 27.6 : 22.8, alignment: .leading)
                if let song = player.currentSong {
                    if compact {
                        Text(song.artist + " — " + song.album)
                            .font(LWTheme.text(15)).foregroundStyle(LWTheme.secondary)
                            .lwLineBox(15, 21)
                            .frame(minHeight: 21, alignment: .leading)
                    } else {
                        PlayerBylineFlow {
                            Text("").font(LWTheme.text(15)).lwLineBox(15, 21).frame(width: 0).accessibilityHidden(true)
                            Button { openContext(.artist(song.artist)) } label: {
                                Text(song.artist).font(LWTheme.text(14)).lwLineBox(14, 19.6)
                            }.padding(.top, 2)
                            Text(" ").font(LWTheme.text(15)).lwLineBox(15, 21).layoutValue(key: PlayerBylineWhitespace.self, value: true).accessibilityHidden(true)
                            Text("—").font(LWTheme.text(14)).lwLineBox(14, 19.6)
                            Text(" ").font(LWTheme.text(15)).lwLineBox(15, 21).layoutValue(key: PlayerBylineWhitespace.self, value: true).accessibilityHidden(true)
                            Button { openContext(.album(name: song.album, artist: song.albumArtist.isEmpty ? song.artist : song.albumArtist)) } label: {
                                Text(song.album).font(LWTheme.text(14)).lwLineBox(14, 19.6)
                            }.padding(.top, 2)
                        }.foregroundStyle(LWTheme.secondary)
                    }
                }
            }.frame(maxWidth: .infinity, alignment: .leading)
            favoriteButton.padding(.top, compact ? -6 : -8).padding(.trailing, -10)
        }
    }

    @ViewBuilder private var playbackClock: some View {
        if player.isLoading {
            HStack(spacing: 8) {
                LWStateGlyph(state: .copyPending, availability: player.currentSong?.sourceAvailability ?? .unknown, pinned: false).accessibilityHidden(true)
                Text(compact ? "Opening file… · press ■ to cancel" : "Opening file…")
                    .font(LWTheme.text(14)).lwLineBox(14, 19.6).fixedSize(horizontal: false, vertical: true)
            }.frame(minHeight: compact ? 51 : 38).foregroundStyle(LWTheme.secondary)
                .accessibilityHint("Press the stop key to cancel.")
        } else {
            timeLayout {
                MatrixText(text: paddedTime, height: compact ? 72 : 43, unlit: false)
                    .accessibilityLabel("Elapsed time " + player.currentTime)
                Spacer(minLength: 4)
                VStack(alignment: .trailing, spacing: 2) {
                    if compact { Text(formatSummary).lwLineBox(12, 16.8, data: true) }
                    Text(player.remainingTime).lwLineBox(12, 16.8, data: true).accessibilityLabel("Remaining time " + player.remainingTime)
                }.font(LWTheme.data()).foregroundStyle(LWTheme.secondary).padding(.bottom, compact ? 2 : 0)
            }
        }
    }

    private func artworkRegionHeight(_ size: CGSize) -> CGFloat {
        if dynamicTypeSize.isAccessibilitySize { return min(184, max(0, size.width - 32)) }
        let clockHeight = player.isLoading ? 51.0 : 72.0
        let fixed = 48.0 + 86 + 10 + 16 + measuredInformationHeight + 14 + clockHeight + 14 + 77.4 + 14 + 48
        return max(124, size.height - fixed)
    }

    @ViewBuilder private var trackActions: some View {
        Button("Show queue", systemImage: "list.bullet") { queuePresented = true }
        Button("Show playing source", systemImage: "arrow.up.backward") {
            NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: player.playbackContext)
        }
        if let song = player.currentSong {
            Button("Play next", systemImage: "text.line.first.and.arrowtriangle.forward") { queueNext(song) }
            Button("Add to queue", systemImage: "text.badge.plus") { player.addToQueue(song) }
            Button("Go to album", systemImage: "square.stack") {
                openContext(.album(name: song.album, artist: song.albumArtist.isEmpty ? song.artist : song.albumArtist))
            }.disabled(song.album.isEmpty)
            Button("Show in folder", systemImage: "folder") { showFolder(for: song) }
            Button("Add to list", systemImage: "text.badge.plus") { songToAdd = song }
            Button(isPinned ? "Stop keeping offline" : "Keep offline", systemImage: "arrow.down") { togglePinned(song) }
            Button("Retry opening file", systemImage: "arrow.clockwise") { player.playSong(song) }
            if let url = player.playbackURL { ShareLink(item: url) }
        }
    }

    private func openContext(_ context: PlaybackContext) {
        NotificationCenter.default.post(name: Notification.Name("LWOpenPlaybackContext"), object: context)
    }

    private func presentTrackMenu() {
        guard let song = player.currentSong, presenter.content == nil else { return }
        let playlistID: Int64?
        if case .playlist(let id, _) = player.playbackContext { playlistID = id }
        else { playlistID = nil }
        presenter.present(menu: LWTrackMenu.request(song: song, targets: [song], model: libraryModel,
            player: player, presenter: presenter, playlistID: playlistID, sharing: sharing,
            onPlay: { player.playQueueEntry(at: player.currentQueueIndex) }, onFolder: showFolder, onShare: share,
            onToggleFavorite: {
                Task {
                    do {
                        try await player.toggleCurrentFavorite()
                        if player.currentSong?.isFavorite == 1 && !reduceMotion { favoriteBurst += 1 }
                    } catch { self.error = error.localizedDescription }
                }
            }, onRetry: { _ in player.playQueueEntry(at: player.currentQueueIndex) }))
    }

    private func share(_ songs: [Song]) {
        guard !sharing, !songs.isEmpty else { return }
        sharing = true
        Task {
            defer { sharing = false }
            do {
                let prepared = try await LWSourceFileSharing.prepare(songs: songs, dependencies: dependencies)
                shareDirectory = prepared.directory
                sharedFile = prepared
            } catch { self.error = error.localizedDescription }
        }
    }

    private func queueNext(_ song: Song) {
        let appended = player.queue.count
        player.addToQueue(song)
        if appended > 0 {
            player.reorderQueue(from: IndexSet(integer: appended), to: min(player.currentQueueIndex + 1, appended))
        }
    }

    private func showFolder(for song: Song) {
        Task {
            do {
                guard let sourceID = song.sourceId,
                      let source = try await dependencies.sourceService.repository().getOne(id: sourceID) else {
                    throw CustomError.genericError("This song has no linked source folder. Rescan its source in Folders.")
                }
                let paths = try await dependencies.sourceService.importService().listItems(sourceId: sourceID, parentPathId: nil)
                let parent = paths.first { $0.relativePath == song.relativePath }?.parentPathId ?? source.pathId
                let name = paths.first { $0.isDirectory && $0.pathId == parent }?.name ?? URL(fileURLWithPath: source.dirPath).lastPathComponent
                openContext(.folder(sourceID: sourceID, pathID: parent, name: name))
            } catch { self.error = error.localizedDescription }
        }
    }

    private var emptyState: some View {
        VStack(alignment: .leading, spacing: 0) {
            LWIcon("album-thin", size: 48).foregroundStyle(LWTheme.tertiary)
                .frame(width: 200, height: 200).background(LWTheme.panel)
            Text("Nothing playing").font(LWTheme.text(19).weight(.medium))
                .lwLineBox(19, 22.8).padding(.top, 14)
            Text(compact ? "Choose a song from your library, or add audio files in Folders." : "Double-click a song, or drop audio files anywhere in this window.")
                .font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary).lwLineBox(14, 21).padding(.top, 2)
            Spacer()
        }.padding(.horizontal, 20).padding(.top, 4).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading).background(LWTheme.paper)
    }

    private var isPinned: Bool {
        guard let id = player.currentSong?.id else { return false }
        return (LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? []).contains(String(id))
    }
    private func togglePinned(_ song: Song) {
        guard let id = song.id else { return }
        let key = String(id)
        var pins = Set(LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? [])
        let shouldPin = !pins.contains(key)
        if shouldPin { pins.insert(key) } else { pins.remove(key) }
        LocalWavePreferences.current.set(Array(pins), forKey: "lwPinnedSongs")
        Task {
            do {
                if shouldPin { try await dependencies.songRepository.markSongForCopy(songId: id) }
                NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
            } catch { self.error = error.localizedDescription }
        }
    }

    private var favoriteButton: some View {
        Button {
            Task {
                do {
                    try await player.toggleCurrentFavorite()
                    if player.currentSong?.isFavorite == 1 && !reduceMotion { favoriteBurst += 1 }
                } catch { self.error = error.localizedDescription }
            }
        } label: {
            LWIcon(player.currentSong?.isFavorite == 1 ? "heart-fill" : "heart", size: compact ? 24 : 22)
                .foregroundStyle(player.currentSong?.isFavorite == 1 ? LWTheme.signal : LWTheme.secondary)
                .frame(width: 44, height: 44)
                .overlay { PlayerFavoriteBurst(trigger: favoriteBurst).allowsHitTesting(false) }
        }
        .accessibilityLabel("Favorite").accessibilityValue(player.currentSong?.isFavorite == 1 ? "On" : "Off")
        .disabled(player.currentSong == nil)
    }

    private var metadata: some View {
        VStack(spacing: 4) {
            metadataRow("Format", fileFormat)
            metadataRow("Size", sizeLabel + " · " + locationLabel)
            metadataRow("File", folderLabel.count <= 30 ? folderLabel : String(folderLabel.prefix(14)) + "…" + String(folderLabel.suffix(15)))
            metadataRow("Plays", String(playCount))
        }.padding(.top, 13).overlay(alignment: .top) { Rectangle().fill(LWTheme.line).frame(height: 1) }
            .font(LWTheme.text(12)).foregroundStyle(LWTheme.secondary)
    }

    @ViewBuilder private func metadataRow(_ label: String, _ value: String) -> some View {
        if dynamicTypeSize.isAccessibilitySize {
            VStack(alignment: .leading, spacing: 4) {
                Text(label)
                Text(value).font(LWTheme.data()).fixedSize(horizontal: false, vertical: true)
            }.frame(maxWidth: .infinity, alignment: .leading).accessibilityElement(children: .combine)
        } else {
            HStack(spacing: 8) {
                Text(label).lwLineBox(12, 16.8).frame(width: 64, alignment: .leading)
                if label == "File", let song = player.currentSong {
                    Button { showFolder(for: song) } label: {
                        Text(value).font(LWTheme.data()).foregroundStyle(LWTheme.ink).underline(true, color: LWTheme.line).lineLimit(1)
                            .lwLineBox(12, 16.8, data: true).fixedSize(horizontal: true, vertical: false)
                            .frame(minWidth: 0, maxWidth: .infinity, alignment: .leading).clipped()
                    }.accessibilityLabel(folderLabel).help(folderLabel)
                } else { Text(value).font(LWTheme.data()).lineLimit(1).truncationMode(.middle).lwLineBox(12, 16.8, data: true) }
            }.frame(maxWidth: .infinity, minHeight: 16.8, alignment: .leading).accessibilityElement(children: .combine)
        }
    }

    private var timeLayout: AnyLayout {
        dynamicTypeSize.isAccessibilitySize ? AnyLayout(VStackLayout(alignment: .leading, spacing: 8)) : AnyLayout(HStackLayout(alignment: .bottom))
    }
    private var paddedTime: String { player.currentTime.count < 5 ? "0" + player.currentTime : player.currentTime }
    private var formatSummary: String {
        let format = player.currentSong?.fileExtension?.lowercased() ?? player.playbackFormat.components(separatedBy: " · ").first?.lowercased() ?? "audio"
        if let rate = player.currentSong?.sampleRate, let depth = player.currentSong?.bitDepth {
            return format + " " + String(format: "%g", rate / 1000) + "/" + String(depth)
        }
        return format
    }
    private var sizeLabel: String {
        guard let bytes = player.currentSong?.fileSize else { return player.playbackFileSize }
        return String(format: "%.1f MB", Double(bytes) / 1_048_576)
    }
    private var locationLabel: String {
        guard player.currentSong?.fileState == .copied else { return "in cloud" }
        #if targetEnvironment(macCatalyst)
        return "on this Mac"
        #else
        return "on device"
        #endif
    }
    private var folderLabel: String {
        guard let relative = player.currentSong?.relativePath else { return sourceName }
        return ([sourceName] + relative.split(separator: "/").dropLast().map(String.init)).joined(separator: " / ")
    }
    private var fileFormat: String {
        guard let song = player.currentSong, let format = song.fileExtension else { return player.playbackFormat }
        var fields = [format.lowercased()]
        if let rate = song.sampleRate, rate.isFinite, rate > 0 { fields.append(String(format: "%g kHz", rate / 1000)) }
        if let bits = song.bitDepth, bits > 0 { fields.append("\(bits) bit") }
        return fields.joined(separator: " · ")
    }
    private var playCount: Int {
        guard let id = player.currentSong?.id else { return 0 }
        return (LocalWavePreferences.current.dictionary(forKey: "lwPlayCounts") as? [String: Int])?[String(id)] ?? 0
    }
    private func artworkSize(_ size: CGSize) -> CGFloat {
        compact ? max(0, min(size.width - 32, artworkRegionHeight(size) - 4, 358)) : min(200, max(0, size.width - 40))
    }
}

private struct PlayerBylineWhitespace: LayoutValueKey { static let defaultValue = false }

private struct PlayerBylineFlow: Layout {
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        positions(width: proposal.width, subviews: subviews).size
    }
    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        for (index, item) in positions(width: bounds.width, subviews: subviews).items.enumerated() {
            subviews[index].place(at: CGPoint(x: bounds.minX + item.origin.x, y: bounds.minY + item.origin.y), proposal: ProposedViewSize(item.size))
        }
    }
    private func positions(width: CGFloat?, subviews: Subviews) -> (size: CGSize, items: [(origin: CGPoint, size: CGSize)]) {
        guard let strut = subviews.first?.dimensions(in: .unspecified) else { return (.zero, []) }
        let available = width ?? .infinity
        let baseAscent = strut[.firstTextBaseline], baseDescent = strut.height - baseAscent
        var result = Array(repeating: (origin: CGPoint.zero, size: CGSize.zero), count: subviews.count)
        result[0].size = CGSize(width: 0, height: strut.height)
        var row: [(index: Int, x: CGFloat, dimensions: ViewDimensions)] = []
        var x: CGFloat = 0, y: CGFloat = 0, maximumWidth: CGFloat = 0
        var ascent = baseAscent, descent = baseDescent
        func finishRow() {
            guard !row.isEmpty else { return }
            for item in row {
                let dimensions = item.dimensions
                result[item.index] = (CGPoint(x: item.x, y: y + ascent - dimensions[.firstTextBaseline]), CGSize(width: dimensions.width, height: dimensions.height))
            }
            maximumWidth = max(maximumWidth, x)
            y += ascent + descent
            row = []; x = 0; ascent = baseAscent; descent = baseDescent
        }
        for index in subviews.indices.dropFirst() {
            let view = subviews[index]
            let natural = view.dimensions(in: .unspecified)
            let dimensions = natural.width > available ? view.dimensions(in: ProposedViewSize(width: available, height: nil)) : natural
            if x > 0 && x + dimensions.width > available { finishRow() }
            if x == 0 && view[PlayerBylineWhitespace.self] { continue }
            row.append((index, x, dimensions))
            x += dimensions.width
            ascent = max(ascent, dimensions[.firstTextBaseline])
            descent = max(descent, dimensions.height - dimensions[.firstTextBaseline])
        }
        finishRow()
        return (CGSize(width: width ?? maximumWidth, height: y), result)
    }
}

private struct WaveEntrance: ViewModifier {
    @Environment(\.lwReduceMotion) private var reduceMotion
    let appeared: Bool
    let index: Int
    func body(content: Content) -> some View {
        content.opacity(appeared || reduceMotion ? 1 : 0)
            .offset(y: appeared || reduceMotion ? 0 : 10)
            .animation(reduceMotion ? nil : LWMotion.out().delay(LWMotion.quick + Double(index) * 0.045), value: appeared)
    }
}

struct WaveModeKeys: View {
    @EnvironmentObject private var player: PlayerViewModel
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var hoveredMode: String?
    var showQueue: Bool
    var panelVisible = true
    var onQueue: () -> Void
    var body: some View {
        (showQueue && dynamicTypeSize.isAccessibilitySize ? AnyLayout(VStackLayout(spacing: 2)) : AnyLayout(HStackLayout(spacing: 2))) {
            modeButton("Shuffle", icon: "shuf", active: player.isShuffleEnabled) { player.setShuffle(!player.isShuffleEnabled) }
            modeButton(player.repeatMode == .one ? "One" : "Repeat", icon: "rep", active: player.repeatMode != .none) {
                var mode = player.repeatMode
                mode.toggle()
                player.setRepeat(mode)
            }
            modeButton(showQueue ? "\(max(0, player.queue.count - player.currentQueueIndex - 1)) up next" : "Now playing", icon: showQueue ? "queue" : "side", active: !showQueue && panelVisible, action: onQueue)
                .accessibilityLabel(showQueue ? "Queue" : "Toggle now playing panel")
        }.padding(2).background(LWTheme.panel2).buttonStyle(LWKeyStyle())
            .fixedSize(horizontal: !showQueue, vertical: true)
    }
    private func modeButton(_ text: String, icon: String, active: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 6) {
                LWIcon(icon, size: showQueue ? 16 : 18)
                if showQueue {
                    if icon == "queue" {
                        (Text("\(max(0, player.queue.count - player.currentQueueIndex - 1))").font(LWTheme.data(11)) + Text(" up next"))
                            .lwLineBox(14, 19.6)
                    } else { Text(text).lwLineBox(14, 19.6) }
                }
            }
            .font(LWTheme.text(14)).fixedSize(horizontal: false, vertical: true)
            .frame(maxWidth: showQueue ? .infinity : nil)
            .frame(width: showQueue ? nil : 44).frame(minHeight: showQueue ? 44 : 40)
            .foregroundStyle(active ? LWTheme.onInk : LWTheme.ink)
            .background(active ? LWTheme.ink : !showQueue && hoveredMode == icon ? LWTheme.paper : LWTheme.key)
            .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: active)
            .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: hoveredMode)
            .overlay(alignment: .bottomTrailing) {
                if !showQueue && icon == "rep" && player.repeatMode == .one { Text("1").font(LWTheme.data(9)).padding(.trailing, 5).padding(.bottom, 4) }
            }
        }.onHover { hoveredMode = $0 ? icon : nil }
            .accessibilityLabel(text).accessibilityValue(active ? "On" : "Off")
    }
}

/// Internal drag representation: queue occurrences are UUIDs, never song text payloads.
struct LWQueueEntryDragPayload: Codable, Transferable {
    let entryID: UUID
    static let contentType = UTType(exportedAs: "com.olegpustovit.localwave.queue-entry", conformingTo: .data)
    var encodedData: Data { Data(entryID.uuidString.utf8) }
    static func decode(_ data: Data) -> Self? {
        guard let string = String(data: data, encoding: .utf8), let id = UUID(uuidString: string) else { return nil }
        return Self(entryID: id)
    }
    static var transferRepresentation: some TransferRepresentation {
        DataRepresentation(contentType: contentType, exporting: { $0.encodedData }, importing: { data in
            guard let payload = decode(data) else { throw CocoaError(.coderReadCorrupt) }
            return payload
        })
    }
}

private struct WaveQueueEntry: Identifiable {
    let id: UUID
    let offset: Int
    let element: Song
}

private extension PlayerViewModel {
    var waveQueueEntries: [WaveQueueEntry] {
        zip(queueEntryIdentifiers, queue.enumerated()).map {
            WaveQueueEntry(id: $0.0, offset: $0.1.offset, element: $0.1.element)
        }
    }
}

enum LWQueueDropPlacement {
    static func targetID(at point: CGPoint, frames: [UUID: CGRect], upcomingIDs: [UUID]) -> UUID? {
        upcomingIDs.filter { id in
            guard let frame = frames[id] else { return false }
            return frame.height > 0 && frame.midY.isFinite
        }.min { abs(frames[$0]!.midY - point.y) < abs(frames[$1]!.midY - point.y) }
    }
}

@MainActor
private struct LWQueueDropDelegate: DropDelegate {
    let player: PlayerViewModel
    let dependencies: DependencyContainer
    let frames: [UUID: CGRect]
    let reduceMotion: Bool
    let songTargeted: (Bool) -> Void

    func validateDrop(info: DropInfo) -> Bool {
        info.hasItemsConforming(to: [LWQueueEntryDragPayload.contentType, .plainText])
    }
    func dropEntered(info: DropInfo) { songTargeted(isSong(info)) }
    func dropExited(info: DropInfo) { songTargeted(false) }
    func dropUpdated(info: DropInfo) -> DropProposal? {
        DropProposal(operation: isSong(info) ? .copy : .move)
    }
    private func isSong(_ info: DropInfo) -> Bool {
        !info.hasItemsConforming(to: [LWQueueEntryDragPayload.contentType]) && info.hasItemsConforming(to: [.plainText])
    }
    func performDrop(info: DropInfo) -> Bool {
        songTargeted(false)
        if let provider = info.itemProviders(for: [LWQueueEntryDragPayload.contentType]).first {
            let upcoming = player.waveQueueEntries.filter { $0.offset > player.currentQueueIndex }.map(\.id)
            guard let targetID = LWQueueDropPlacement.targetID(at: info.location, frames: frames, upcomingIDs: upcoming) else { return false }
            provider.loadDataRepresentation(forTypeIdentifier: LWQueueEntryDragPayload.contentType.identifier) { data, _ in
                guard let data, let payload = LWQueueEntryDragPayload.decode(data) else { return }
                Task { @MainActor in
                    guard let source = player.queueEntryIdentifiers.firstIndex(of: payload.entryID),
                          let target = player.queueEntryIdentifiers.firstIndex(of: targetID),
                          source > player.currentQueueIndex, target > player.currentQueueIndex, source != target else { return }
                    withAnimation(reduceMotion ? nil : LWMotion.out()) {
                        player.reorderQueue(from: IndexSet(integer: source), to: target + (source < target ? 1 : 0))
                    }
                }
            }
            return true
        }
        let providers = info.itemProviders(for: [.plainText])
        guard !providers.isEmpty else { return false }
        Task { @MainActor in
            var values: [String] = []
            for provider in providers {
                let value: String? = await withCheckedContinuation { continuation in
                    provider.loadObject(ofClass: NSString.self) { object, _ in
                        continuation.resume(returning: object as? String)
                    }
                }
                if let value { values.append(value) }
            }
            _ = appendDroppedSongs(values, dependencies: dependencies, player: player)
        }
        return true
    }
}

struct WaveQueueRows: View {
    @EnvironmentObject private var player: PlayerViewModel
    @EnvironmentObject private var dependencies: DependencyContainer
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var dropTargeted = false
    @State private var rowFrames: [UUID: CGRect] = [:]
    @Namespace private var dropSpace
    var scrolls = true
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Up next · " + player.playbackContext.title).lineLimit(1).lwLineBox(13, 18.2)
                Spacer()
                Text("\(upcoming.count)").font(LWTheme.data()).lwLineBox(12, 16.8, data: true)
            }.font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary)
                .frame(minHeight: 18.2)
                .padding(.horizontal, 20).padding(.top, 10).padding(.bottom, 4)
            if scrolls { ScrollView { rows } }
            else { rows }
        }
        .contentShape(Rectangle())
        .background { if dropTargeted { LWTheme.paper.overlay(LWTheme.signal.opacity(0.08)) } }
        .overlay { if dropTargeted { Rectangle().strokeBorder(LWTheme.signal, lineWidth: 2) } }
        .coordinateSpace(name: dropSpace)
        .onDrop(of: [LWQueueEntryDragPayload.contentType, .plainText], delegate: LWQueueDropDelegate(
            player: player, dependencies: dependencies, frames: rowFrames, reduceMotion: reduceMotion,
            songTargeted: { dropTargeted = $0 }))
        .onChange(of: player.queueEntryIdentifiers) { _, ids in rowFrames = rowFrames.filter { ids.contains($0.key) } }
    }
    private var rows: some View {
        LazyVStack(alignment: .leading, spacing: 0) {
            ForEach(upcoming) { entry in
                PlayerQueueTrackRow(entry: entry, desktop: true)
                    .onGeometryChange(for: CGRect.self) { proxy in proxy.frame(in: .named(dropSpace)) } action: { rowFrames[entry.id] = $0 }
                    .onDisappear { rowFrames.removeValue(forKey: entry.id) }
            }
            if upcoming.isEmpty {
                Text("Queue is empty.").font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary)
                    .padding(.horizontal, 10).padding(.vertical, 16)
            }
        }
        .padding(.horizontal, 10).padding(.bottom, 10)
        .animation(reduceMotion ? nil : LWMotion.out(), value: player.queueEntryIdentifiers)
    }
    private var upcoming: [WaveQueueEntry] { player.waveQueueEntries.filter { $0.offset > player.currentQueueIndex } }
}

private struct PlayerQueueTrackRow: View {
    @EnvironmentObject private var player: PlayerViewModel
    @Environment(\.lwReduceMotion) private var reduceMotion
    let entry: WaveQueueEntry
    let desktop: Bool
    private var pinned: Bool { (LocalWavePreferences.current.stringArray(forKey: "lwPinnedSongs") ?? []).contains(String(entry.element.uniqueId)) }
    var body: some View {
        HStack(spacing: 10) {
            LWIcon("drag", size: 18).foregroundStyle(LWTheme.tertiary)
                .frame(width: desktop ? 28 : 40, height: desktop ? 36 : 44)
                .contentShape(Rectangle()).draggable(LWQueueEntryDragPayload(entryID: entry.id))
                .accessibilityElement(children: .ignore)
                .accessibilityLabel("Drag to reorder " + entry.element.title)
            LWArtwork(song: entry.element, size: desktop ? 30 : 36)
            Button { player.playQueueEntry(at: entry.offset) } label: {
                VStack(alignment: .leading, spacing: 0) {
                    Text(entry.element.title).font(LWTheme.text(desktop ? 13 : 15)).lwLineBox(desktop ? 13 : 15, desktop ? 18.2 : 21)
                        .frame(minHeight: desktop ? 18.2 : 21, alignment: .leading)
                    Text(entry.element.artist).font(LWTheme.text(desktop ? 12 : 13)).foregroundStyle(LWTheme.secondary)
                        .lwLineBox(desktop ? 12 : 13, desktop ? 16.8 : 18.2)
                        .frame(minHeight: desktop ? 16.8 : 18.2, alignment: .leading)
                }.lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
            }
            if entry.element.fileState != .copied || pinned {
                LWStateGlyph(state: entry.element.fileState, availability: entry.element.fileState == .copied ? .onDevice : entry.element.sourceAvailability ?? .unknown, pinned: pinned)
                    .padding(.horizontal, pinned ? 2 : 0)
            }
            Button { withAnimation(reduceMotion ? nil : LWMotion.out()) { player.removeFromQueue(at: entry.offset) } } label: {
                LWIcon("close", size: 16).foregroundStyle(LWTheme.secondary).frame(width: 44, height: 44)
            }.accessibilityLabel("Remove " + entry.element.title)
        }
        .padding(.leading, 4).padding(.trailing, 6).frame(minHeight: desktop ? 44 : 56)
        .background(LWTheme.paper).contentShape(Rectangle())
        .transition(.asymmetric(
            insertion: .offset(y: 6).combined(with: .opacity).animation(reduceMotion ? nil : LWMotion.out(LWMotion.base)),
            removal: .identity
        ))
        .contextMenu {
            Button("Play", systemImage: "play") { player.playQueueEntry(at: entry.offset) }
            Button("Remove from queue", systemImage: "minus.circle") { player.removeFromQueue(at: entry.offset) }
        }
        .accessibilityAction(named: Text("Move up")) { move(by: -1) }
        .accessibilityAction(named: Text("Move down")) { move(by: 1) }
    }
    private func move(by delta: Int) {
        let target = entry.offset + delta
        guard target > player.currentQueueIndex, player.queue.indices.contains(target) else { return }
        withAnimation(reduceMotion ? nil : LWMotion.out()) {
            player.reorderQueue(from: IndexSet(integer: entry.offset), to: target + (delta > 0 ? 1 : 0))
        }
    }
}

@MainActor private func appendDroppedSongs(_ values: [String], dependencies: DependencyContainer, player: PlayerViewModel) -> Bool {
    let ids = LWSongDragPayload.decode(values)
    guard !ids.isEmpty else { return false }
    Task {
        let songs = await dependencies.songRepository.getSongs(ids: ids)
        let byID = Dictionary(songs.compactMap { song in song.id.map { ($0, song) } }, uniquingKeysWith: { first, _ in first })
        for id in ids { if let song = byID[id] { player.addToQueue(song) } }
    }
    return true
}

struct WaveQueueSheet: View {
    @EnvironmentObject private var player: PlayerViewModel
    @EnvironmentObject private var dependencies: DependencyContainer
    @EnvironmentObject private var presenter: LWSheetPresenter
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var name = ""
    @State private var naming = false
    @State private var error: String?
    @State private var queueBeforeClear: [Song]?
    @State private var rowFrames: [UUID: CGRect] = [:]
    @Namespace private var dropSpace
    private var upcoming: [WaveQueueEntry] { player.waveQueueEntries.filter { $0.offset > player.currentQueueIndex } }
    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 0) {
                    Text("Queue").font(LWTheme.text(20).weight(.medium)).tracking(-0.2).accessibilityAddTraits(.isHeader)
                        .lwLineBox(20, 28)
                        .frame(minHeight: 28, alignment: .leading)
                    Text("From " + player.playbackContext.title + (player.isShuffleEnabled ? " · shuffled" : ""))
                        .font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary)
                        .lwLineBox(13, 18.2)
                        .frame(minHeight: 18.2, alignment: .leading)
                }.frame(maxWidth: .infinity, alignment: .leading)
                Button { presenter.dismiss() } label: { LWIcon("close", size: 22).frame(width: 44, height: 44) }
                    .buttonStyle(LWIconButtonStyle())
                    .padding(.trailing, -10)
                    .accessibilityLabel("Close queue")
            }.padding(.horizontal, 16).padding(.top, 12).padding(.bottom, 8)
                .contextMenu { Button("Save as list") { naming = true } }
                .accessibilityAction(named: Text("Save as list")) { naming = true }
            LazyVStack(alignment: .leading, spacing: 0) {
                if let song = player.currentSong {
                    HStack(spacing: 10) {
                        LWArtwork(song: song, size: 36)
                        VStack(alignment: .leading, spacing: 0) {
                            Text(song.title).font(LWTheme.text(15).weight(.semibold))
                                .lwLineBox(15, 21)
                                .frame(minHeight: 21, alignment: .leading)
                            Text("Now · " + song.artist).font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary)
                                .lwLineBox(13, 18.2)
                                .frame(minHeight: 18.2, alignment: .leading)
                        }.lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
                        LWEqualizer(playing: player.isPlaying)
                        Color.clear.frame(width: 14, height: 1)
                    }.padding(.leading, 14).padding(.trailing, 6).frame(minHeight: 56)
                }
                (Text("Up next ") + Text("\(upcoming.count)").font(LWTheme.data()))
                    .font(LWTheme.text(13).weight(.medium)).foregroundStyle(LWTheme.secondary)
                    .lwLineBox(13, 18.2)
                    .frame(minHeight: 18.2, alignment: .leading)
                    .padding(.leading, 6).padding(.top, 20).padding(.bottom, 8)
                ForEach(upcoming) { entry in
                    PlayerQueueTrackRow(entry: entry, desktop: false)
                        .onGeometryChange(for: CGRect.self) { proxy in proxy.frame(in: .named(dropSpace)) } action: { rowFrames[entry.id] = $0 }
                        .onDisappear { rowFrames.removeValue(forKey: entry.id) }
                }
                if upcoming.isEmpty {
                    HStack(spacing: 4) {
                        Text(queueBeforeClear == nil ? "Nothing queued." : "Up next cleared.")
                        if let queueBeforeClear { Button("Undo") {
                            var transaction = Transaction(); transaction.disablesAnimations = true
                            withTransaction(transaction) { player.restoreQueue(queueBeforeClear); self.queueBeforeClear = nil }
                        }.underline() }
                    }.font(LWTheme.text(14)).foregroundStyle(LWTheme.secondary).padding(.vertical, 40).padding(.horizontal, 16)
                } else {
                    Button {
                        queueBeforeClear = player.queue
                        var transaction = Transaction(); transaction.disablesAnimations = true
                        withTransaction(transaction) { player.clearUpcomingQueue() }
                    } label: {
                        Text("Clear up next").font(LWTheme.text(16)).lwLineBox(16, 22.4).foregroundStyle(LWTheme.secondary)
                            .frame(maxWidth: .infinity, minHeight: 52, alignment: .leading).padding(.horizontal, 16)
                    }
                }
            }.padding(.horizontal, 10)
                .animation(reduceMotion ? nil : LWMotion.out(), value: player.queueEntryIdentifiers)
        }
        .coordinateSpace(name: dropSpace)
        .onDrop(of: [LWQueueEntryDragPayload.contentType, .plainText], delegate: LWQueueDropDelegate(
            player: player, dependencies: dependencies, frames: rowFrames, reduceMotion: reduceMotion,
            songTargeted: { _ in }))
        .onChange(of: player.queueEntryIdentifiers) { _, ids in rowFrames = rowFrames.filter { ids.contains($0.key) } }
        .background(LWTheme.paper).foregroundStyle(LWTheme.ink).buttonStyle(.plain)
        .alert("New List", isPresented: $naming) {
            TextField("List name", text: $name)
            Button("Create") {
                Task {
                    do { try await player.createPlaylist(name: name.trimmingCharacters(in: .whitespacesAndNewlines)); name = "" }
                    catch { self.error = error.localizedDescription }
                }
            }.disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            Button("Cancel", role: .cancel) {}
        }
        .alert("Could not save list", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) { Button("OK") { error = nil } } message: { Text(error ?? "") }
    }
}

struct WaveVolumeLevels: View {
    @EnvironmentObject private var player: PlayerViewModel
    @Environment(\.lwReduceMotion) private var reduceMotion
    var body: some View {
        HStack(spacing: 8) {
            LWIcon("vol", size: 16).foregroundStyle(LWTheme.secondary)
            GeometryReader { proxy in
                HStack(spacing: 2) {
                    ForEach(0..<12) { index in
                        let on = index < Int((player.volume * 12).rounded())
                        Rectangle().fill(on ? LWTheme.ink : LWTheme.panel2).frame(width: 5, height: 10)
                            .animation(reduceMotion ? nil : LWMotion.steps(1, duration: LWMotion.instant), value: on)
                    }
                }.frame(height: proxy.size.height).contentShape(Rectangle())
                    .gesture(DragGesture(minimumDistance: 0).onChanged { value in
                        player.volume = Float(max(0, min(1, value.location.x / max(1, proxy.size.width))))
                    })
            }.frame(width: 82)
        }
        .accessibilityElement().accessibilityLabel("Volume")
        .accessibilityValue("\(Int(player.volume * 100)) percent")
        .accessibilityAdjustableAction { direction in
            player.volume = max(0, min(1, player.volume + (direction == .increment ? 1.0 / 12 : -1.0 / 12)))
        }
    }
}

struct WaveAudioRoute: View {
    var background = LWTheme.paper
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var pressed = false
    var body: some View {
        PlayerNativeAudioRoute(pressed: $pressed).overlay {
            (pressed ? LWTheme.panel : background)
                .overlay { LWIcon("route", size: 22).foregroundStyle(LWTheme.ink) }
                .allowsHitTesting(false)
        }
        .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: pressed)
    }
}

private struct PlayerNativeAudioRoute: UIViewRepresentable {
    @Binding var pressed: Bool
    func makeCoordinator() -> Coordinator { Coordinator(pressed: $pressed) }
    func makeUIView(context: Context) -> AVRoutePickerView {
        let view = AVRoutePickerView()
        view.activeTintColor = .label
        view.tintColor = .secondaryLabel
        view.accessibilityLabel = "Audio output"
        let touch = UILongPressGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.trackTouch(_:)))
        touch.minimumPressDuration = 0
        touch.allowableMovement = .greatestFiniteMagnitude
        touch.cancelsTouchesInView = false
        touch.delaysTouchesBegan = false
        touch.delaysTouchesEnded = false
        touch.delegate = context.coordinator
        view.addGestureRecognizer(touch)
        return view
    }
    func updateUIView(_ uiView: AVRoutePickerView, context: Context) { context.coordinator.pressed = $pressed }
    final class Coordinator: NSObject, UIGestureRecognizerDelegate {
        var pressed: Binding<Bool>
        init(pressed: Binding<Bool>) { self.pressed = pressed }
        @objc func trackTouch(_ recognizer: UILongPressGestureRecognizer) {
            pressed.wrappedValue = (recognizer.state == .began || recognizer.state == .changed)
                && recognizer.view.map { $0.bounds.contains(recognizer.location(in: $0)) } == true
        }
        func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith otherGestureRecognizer: UIGestureRecognizer) -> Bool { true }
    }
}

private extension View {
    func monospaced() -> some View { font(LWTheme.data()).monospacedDigit() }
}

struct WaveformView: View {
    @Environment(\.lwReduceMotion) private var reduceMotion
    let song: Song?
    var sourceURL: URL? = nil
    let progress: Double
    var height: CGFloat = 56
    var barCount = 96
    let seek: (Double) -> Void
    @State private var peaks: [Float] = []
    @State private var loadedAt = Date.distantPast
    @State private var animating = false
    @State private var scrubPosition: Double?
    @State private var hasLoadedWaveform = false

    var body: some View {
        let displayedPeaks = peaks
        GeometryReader { proxy in
            TimelineView(.animation(paused: reduceMotion || !animating)) { timeline in
            Canvas { context, size in
                let fraction = max(0, min(1, scrubPosition ?? progress))
                let baseline = size.height - 4
                if !displayedPeaks.isEmpty {
                    let count = min(displayedPeaks.count, barCount)
                    let step = size.width / CGFloat(count)
                    for index in 0..<count {
                        let lower = index * displayedPeaks.count / count
                        let upper = max(lower + 1, (index + 1) * displayedPeaks.count / count)
                        let peak = displayedPeaks[lower..<upper].max() ?? 0
                        let delay = (Double(index) * 280 / Double(count)).rounded() / 1000
                        let age = timeline.date.timeIntervalSince(loadedAt) - delay
                        let localReveal = reduceMotion || !animating ? 1 : LWMotion.outProgress(age / LWMotion.slow)
                        let barHeight = max(2, CGFloat(peak) * baseline) * localReveal
                        let bar = CGRect(x: CGFloat(index) * step, y: baseline - barHeight, width: step / 4, height: barHeight)
                        context.fill(Path(bar), with: .color(LWTheme.ink.opacity(0.42)))
                    }
                }
                context.fill(Path(CGRect(x: 0, y: baseline + 1, width: size.width, height: 1)), with: .color(LWTheme.line))
                context.fill(Path(CGRect(x: 0, y: baseline, width: size.width * fraction, height: 3)), with: .color(LWTheme.ink))
                let svgUnit = size.width / CGFloat(barCount * 4)
                context.fill(Path(CGRect(x: size.width * fraction - svgUnit, y: 0, width: 2 * svgUnit, height: size.height)), with: .color(LWTheme.signal))
            }
            }
            .contentShape(Rectangle())
            .gesture(DragGesture(minimumDistance: 0).onChanged { value in
                scrubPosition = max(0, min(1, value.location.x / max(1, proxy.size.width)))
            }.onEnded { value in
                seek(max(0, min(1, value.location.x / max(1, proxy.size.width))))
                scrubPosition = nil
            })
        }
        .frame(height: height)
        .clipped()
        .accessibilityElement()
        .accessibilityLabel("Playback position")
        .accessibilityValue(playbackAccessibilityValue)
        .accessibilityAdjustableAction { direction in
            let step = song?.duration.flatMap { $0.isFinite && $0 > 0 ? 5 / $0 : nil } ?? 0.02
            seek(max(0, min(1, progress + (direction == .increment ? step : -step))))
        }
        .task(id: sourceURL?.path ?? String(song?.songKey ?? 0)) {
            peaks = []
            animating = false
            guard let song else { return }
            let values: [Float]?
            #if DEBUG
            if let fixture = LWReferenceFixture.peaks(for: song, count: barCount) { values = fixture }
            else { values = try? await WaveformCache.shared.peaks(for: song, sourceURL: sourceURL) }
            #else
            values = try? await WaveformCache.shared.peaks(for: song, sourceURL: sourceURL)
            #endif
            guard !Task.isCancelled else { return }
            peaks = values ?? []
            let shouldAnimate = hasLoadedWaveform && !reduceMotion
            hasLoadedWaveform = !(values ?? []).isEmpty
            loadedAt = shouldAnimate ? Date() : .distantPast
            animating = shouldAnimate
            if shouldAnimate { try? await Task.sleep(for: .seconds(LWMotion.slow + 0.28)) }
            guard !Task.isCancelled else { return }
            animating = false
        }
    }

    private var playbackAccessibilityValue: String {
        let fraction = max(0, min(1, progress))
        guard let total = song?.duration, total.isFinite, total > 0, total < Double(Int.max) else { return "\(Int(fraction * 100)) percent" }
        func time(_ seconds: Double) -> String {
            let value = Int(seconds)
            return "\(value / 60):" + String(format: "%02d", value % 60)
        }
        return time(fraction * total) + " of " + time(total)
    }

}

private struct WaveLoadingFill: View {
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var started = Date()
    var body: some View {
        TimelineView(.explicit(reduceMotion ? [] : (0...12).map { started.addingTimeInterval(Double($0) * LWMotion.ambient / 12) })) { timeline in
            Canvas { context, size in
                let progress = LWMotion.steppedProgress(timeline.date.timeIntervalSince(started), duration: LWMotion.ambient, count: 12)
                context.fill(Path(CGRect(x: 0, y: 0, width: size.width * (reduceMotion ? 1 : progress), height: size.height)), with: .color(LWTheme.signal))
            }
        }.accessibilityHidden(true)
    }
}

private struct PlayerFavoriteBurst: View {
    let trigger: Int
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var changedAt = Date.distantPast
    @State private var active = false
    private let rays: [(Double, Double)] = [(0, -1), (0.9, -0.5), (0.9, 0.5), (0, 1), (-0.9, 0.5), (-0.9, -0.5)]
    var body: some View {
        TimelineView(.animation(paused: reduceMotion || !active)) { timeline in
            Canvas { context, size in
                let age = timeline.date.timeIntervalSince(changedAt)
                guard active, age >= 0, age < LWMotion.slow, !reduceMotion else { return }
                let progress = LWMotion.outProgress(age / LWMotion.slow)
                for (x, y) in rays {
                    let rect = CGRect(x: size.width / 2 + x * 18 * progress - 2, y: size.height / 2 + y * 18 * progress - 2, width: 4, height: 4)
                    context.fill(Path(rect), with: .color(LWTheme.signal.opacity(1 - progress)))
                }
            }
        }
        .onChange(of: trigger) { _, _ in changedAt = Date(); active = !reduceMotion }
        .onChange(of: reduceMotion) { _, value in if value { active = false } }
        .onDisappear { active = false }
        .task(id: changedAt) {
            guard active else { return }
            do { try await Task.sleep(for: .seconds(LWMotion.slow)) }
            catch { return }
            guard !Task.isCancelled else { return }
            active = false
        }
    }
}
