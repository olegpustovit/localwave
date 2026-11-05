#if DEBUG && os(iOS)
import Foundation
import LocalWaveCore
import LocalWaveData
import LocalWaveDomain
import SwiftUI

/// Identical source content for reference/native screenshot comparisons, isolated by UI-test UUID.
enum LWReferenceFixture {
    @MainActor private static var canvasWindow: UIWindow?
    static var enabled: Bool {
        ProcessInfo.processInfo.environment["LOCALWAVE_REFERENCE_FIXTURE"] == "1" &&
        UUID(uuidString: ProcessInfo.processInfo.environment["LOCALWAVE_UI_TEST_ID"] ?? "") != nil
    }
    static var screen: String { ProcessInfo.processInfo.environment["LOCALWAVE_REFERENCE_SCREEN"] ?? "library" }
    static var canvasSize: CGSize {
        ProcessInfo.processInfo.environment["LOCALWAVE_REFERENCE_DESKTOP"] == "1"
            ? CGSize(width: 1440, height: 872) : CGSize(width: 390, height: 778)
    }

    struct Album: Decodable { let id, t, ar, f, rate: String; let y: Int }
    struct Track: Decodable {
        let id, t, al, ar, state, ext, dir, file: String
        let n, dur, plays: Int
        let size, added: Double
        let fav: Bool
        let pin: Bool?
    }
    struct Root: Decodable { let id, name, root, scanned: String; let reconnect: Bool? }
    struct Folder: Decodable { let path, name: String; let parent: String? }
    struct NonAudio: Decodable { let dir, name: String; let size: Double }
    struct List: Decodable { let id, name: String; let auto: Int?; let tracks: [String] }
    struct Player: Decodable { let queue: [String]; let i: Int; let t, vol: Double; let playing: Bool }
    struct Manifest: Decodable {
        let albums: [Album]; let tracks: [Track]; let sources: [Root]; let folders: [Folder]
        let lists: [List]; let player: Player
        let nonAudio: [NonAudio]?
        let waveforms: [String: WavePeaks]
    }
    struct WavePeaks: Decodable { let mobile, desktop: [Float] }
    static let manifest: Manifest? = {
        guard let url = Bundle.main.url(forResource: "reference", withExtension: "json"),
              let data = try? Data(contentsOf: url) else { return nil }
        return try? JSONDecoder().decode(Manifest.self, from: data)
    }()

    static func sourceDisplay(source: Source) -> (name: String, root: String, scanned: String)? {
        guard enabled, let id = source.id, let roots = manifest?.sources,
              id > 0, id <= roots.count else { return nil }
        let root = roots[Int(id) - 1]
        return (root.name, root.root, root.scanned)
    }
    static func peaks(for song: Song?, count: Int) -> [Float]? {
        guard enabled, let song, let data = manifest,
              let track = data.tracks.first(where: { hashStringToInt64($0.id) == song.songKey }),
              let peaks = data.waveforms[track.id] else { return nil }
        return count == 200 ? peaks.desktop : peaks.mobile
    }

    @MainActor static func seed(_ dependencies: DependencyContainer, player: PlayerViewModel) async throws {
        guard let data = manifest else { throw CustomError.genericError("Reference fixture resource is missing.") }
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let directory = docs.appendingPathComponent("reference-" + (ProcessInfo.processInfo.environment["LOCALWAVE_UI_TEST_ID"] ?? ""))
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let user = try await dependencies.userCloudService.resolveCurrentICloudUser()
        guard let userID = user?.id else { throw CustomError.genericError("Reference fixture user is unavailable.") }
        let sourceRepo = dependencies.sourceService.repository()
        var sources: [String: Source] = [:]
        for root in data.sources {
            let url = directory.appendingPathComponent(root.name)
            try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
            let source = try await sourceRepo.create(source: Source(id: nil, dirPath: url.path, pathId: hashStringToInt64(root.id), userId: userID, type: root.id == "icloud" ? .iCloud : nil, totalPaths: data.tracks.filter { $0.dir.hasPrefix(root.id + "/") }.count, syncError: root.reconnect == true ? "Access expired. The folder was moved, renamed, or the app was reinstalled." : nil, isCurrent: false, createdAt: Date(), lastSyncedAt: Date().addingTimeInterval(root.id == "icloud" ? -120 : root.id == "gdrive" ? -840 : root.id == "dropbox" ? -259200 : 0), updatedAt: nil, bookmark: try url.bookmarkData(options: sourceBookmarkCreationOptions, includingResourceValuesForKeys: nil, relativeTo: nil)))
            sources[root.id] = source
        }
        var paths: [SourcePath] = []
        for folder in data.folders {
            let rootID = String(folder.path.split(separator: "/")[0])
            guard let sourceID = sources[rootID]?.id else { continue }
            paths.append(SourcePath(id: nil, sourceId: sourceID, pathId: hashStringToInt64(folder.path), parentPathId: folder.parent.map(hashStringToInt64), name: folder.name, relativePath: folder.path == rootID ? "" : String(folder.path.dropFirst(rootID.count + 1)), isDirectory: true, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil))
        }
        for file in data.nonAudio ?? [] {
            let rootID = String(file.dir.split(separator: "/")[0])
            guard let source = sources[rootID], let sourceID = source.id else { continue }
            let parentRelative = file.dir == rootID ? "" : String(file.dir.dropFirst(rootID.count + 1))
            let parentURL = URL(fileURLWithPath: source.dirPath).appendingPathComponent(parentRelative)
            try FileManager.default.createDirectory(at: parentURL, withIntermediateDirectories: true)
            let url = parentURL.appendingPathComponent(file.name)
            if file.name.hasSuffix(".jpg"), let album = data.albums.first(where: { item in data.tracks.contains { $0.al == item.id && $0.dir == file.dir } }),
               let art = Bundle.main.url(forResource: "reference-" + album.id, withExtension: "png") {
                if !FileManager.default.fileExists(atPath: url.path) { try FileManager.default.copyItem(at: art, to: url) }
            } else { try Data("LocalWave reference rip log.\n".utf8).write(to: url) }
            let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize.map(Int64.init)
            let relative = parentRelative.isEmpty ? file.name : parentRelative + "/" + file.name
            paths.append(SourcePath(id: nil, sourceId: sourceID, pathId: hashStringToInt64(file.dir + "/" + file.name), parentPathId: hashStringToInt64(file.dir), name: file.name, relativePath: relative, isDirectory: false, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil, fileSize: size))
        }
        var songs: [String: Song] = [:]
        var counts: [String: Int] = [:]
        var dates: [String: Double] = [:]
        var pins: [String] = []
        for track in data.tracks {
            guard let album = data.albums.first(where: { $0.id == track.al }),
                  let rootID = track.dir.split(separator: "/").first.map(String.init),
                  let sourceID = sources[rootID]?.id else { continue }
            let relative = String((track.dir + "/" + track.file).dropFirst(rootID.count + 1))
            let cover = "reference-\(ProcessInfo.processInfo.environment["LOCALWAVE_UI_TEST_ID"] ?? "")/reference-\(album.id).png"
            let coverURL = docs.appendingPathComponent(cover)
            if !FileManager.default.fileExists(atPath: coverURL.path), let original = Bundle.main.url(forResource: "reference-" + album.id, withExtension: "png") { try FileManager.default.copyItem(at: original, to: coverURL) }
            let song = Song(id: nil, songKey: hashStringToInt64(track.id), artist: track.ar, title: track.t, album: album.t, albumArtist: album.ar, releaseYear: album.y, discNumber: 1, trackNumber: track.n, coverArtPath: cover, bookmark: nil, pathHash: hashStringToInt64(track.dir + "/" + track.file), createdAt: Date(timeIntervalSince1970: track.added / 1000), updatedAt: nil, localFilePath: nil, fileState: track.state == "local" ? .copied : .bookmarkOnly, isFavorite: track.fav ? 1 : 0, sourceId: sourceID, relativePath: relative, duration: Double(track.dur), fileSize: Int64(track.size * 1024 * 1024), fileExtension: track.ext, sampleRate: album.rate.contains("96 kHz") ? 96000 : album.rate.contains("48 kHz") ? 48000 : 44100, bitDepth: album.rate.contains("24 bit") ? 24 : album.f == "flac" ? 16 : nil, sourceAvailability: track.state == "local" ? .onDevice : .inCloud)
            let saved = try await dependencies.songRepository.upsertSong(song)
            songs[track.id] = saved
            counts[String(saved.uniqueId)] = track.plays
            if track.pin == true { pins.append(String(saved.uniqueId)) }
            dates[String(saved.uniqueId)] = track.plays > 0 ? track.added / 1000 : nil
            paths.append(SourcePath(id: nil, sourceId: sourceID, pathId: song.pathHash, parentPathId: hashStringToInt64(track.dir), name: track.file, relativePath: relative, isDirectory: false, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil))
        }
        try await dependencies.sourcePathRepository.batchUpsert(paths: paths)
        for list in data.lists where list.auto != 1 {
            let saved = try await dependencies.playlistRepo.create(playlist: Playlist(id: nil, name: list.name, createdAt: Date(), updatedAt: nil))
            if let id = saved.id { for key in list.tracks { if let songID = songs[key]?.id { try await dependencies.playlistSongRepo.addSong(playlistId: id, songId: songID) } } }
        }
        LocalWavePreferences.current.set(counts, forKey: "lwPlayCounts")
        LocalWavePreferences.current.set(dates, forKey: "lwLastPlayed")
        LocalWavePreferences.current.set(pins, forKey: "lwPinnedSongs")
        let queue = data.player.queue.compactMap { songs[$0] }
        player.applyReferenceSnapshot(queue: queue, index: data.player.i, elapsed: data.player.t, volume: data.player.vol)
        NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
        showCanvas(dependencies, player: player)
        FileHandle.standardOutput.write(Data("LOCALWAVE_REFERENCE_READY=1\n".utf8))
    }
    @MainActor private static func showCanvas(_ dependencies: DependencyContainer, player: PlayerViewModel) {
        guard canvasWindow == nil,
              let scene = UIApplication.shared.connectedScenes.compactMap({ $0 as? UIWindowScene }).first else { return }
        let old = scene.windows.first(where: \.isKeyWindow)
        let root = MainTabView(dependencies: dependencies).environmentObject(dependencies).environmentObject(player)
            .environmentObject(TabState()).applyTheme().defaultAppStorage(LocalWavePreferences.current)
            .frame(width: canvasSize.width, height: canvasSize.height)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .background(LWTheme.paper)
            .ignoresSafeArea().statusBarHidden(true).persistentSystemOverlays(.hidden)
        let host = LWReferenceHostingController(rootView: AnyView(root))
        host.safeAreaRegions = []
        host.view.insetsLayoutMarginsFromSafeArea = false
        host.view.accessibilityViewIsModal = true
        let window = LWReferenceWindow(windowScene: scene)
        // UIKit event and accessibility coordinates require a scene-sized window.
        // The constrained SwiftUI content above defines the comparison viewport.
        window.frame = scene.coordinateSpace.bounds
        window.windowLevel = .normal + 1
        window.rootViewController = host
        canvasWindow = window
        window.makeKeyAndVisible()
        old?.accessibilityElementsHidden = true
        old?.isHidden = true
        // The canvas replaces the original root; its hidden graph must not keep
        // command receivers or native text-field responders alive.
        old?.rootViewController = nil
        host.setNeedsStatusBarAppearanceUpdate()
    }
}

private final class LWReferenceHostingController: UIHostingController<AnyView> {
    override var prefersStatusBarHidden: Bool { true }
    override var childForStatusBarHidden: UIViewController? { nil }
}

private final class LWReferenceWindow: UIWindow {
    override var safeAreaInsets: UIEdgeInsets { .zero }
}
#endif
