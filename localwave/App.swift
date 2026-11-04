import LocalWaveDomain
import LocalWaveCore
import LocalWaveData
import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

@main
struct LocalWave: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    @StateObject private var dependencies: DependencyContainer
    @StateObject private var playerVM: PlayerViewModel
    @StateObject private var tabState = TabState()

    init() {
        let c = try! DependencyContainer()
        _dependencies = StateObject(wrappedValue: c)

        _playerVM = StateObject(
            wrappedValue: PlayerViewModel(
                playerPersistenceService: c.playerPersistenceService,
                songRepo: c.songRepository,
                playlistRepo: c.playlistRepo,
                playlistSongRepo: c.playlistSongRepo
            ))
    }

    var body: some Scene {
        WindowGroup {
            MainTabView()
                .environmentObject(tabState)
                .environmentObject(dependencies)
                .environmentObject(playerVM)
                .applyTheme()
                #if DEBUG
                .task {
                    guard let testID = ProcessInfo.processInfo.environment["LOCALWAVE_UI_TEST_ID"], UUID(uuidString: testID) != nil else { return }
                    let song = Song(id: nil, songKey: 100, artist: "Test Artist", title: "Playlist Test Track", album: "Test Album", albumArtist: "Test Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 100, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly)
                    _ = try? await dependencies.songRepository.upsertSong(song)
                    _ = try? await dependencies.playlistRepo.create(playlist: Playlist(id: nil, name: "UI Test Playlist", createdAt: Date(), updatedAt: nil))
                    NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
                }
                #endif
        }
    }
}
