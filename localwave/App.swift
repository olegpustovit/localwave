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
        }
    }
}
