#if os(macOS)
import CoreText
import LocalWaveCore
import LocalWaveData
import SwiftUI

@main
struct LocalWaveMac: App {
    @StateObject private var dependencies: DependencyContainer
    @StateObject private var playerVM: PlayerViewModel
    @StateObject private var tabState = TabState()

    init() {
        for name in ["HostGrotesk", "GeistMono"] {
            if let url = Bundle.main.url(forResource: name, withExtension: "ttf") {
                CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
            }
        }
        let container = try! DependencyContainer()
        _dependencies = StateObject(wrappedValue: container)
        _playerVM = StateObject(wrappedValue: PlayerViewModel(
            playerPersistenceService: container.playerPersistenceService,
            songRepo: container.songRepository,
            playlistRepo: container.playlistRepo,
            playlistSongRepo: container.playlistSongRepo,
            sourceAccessResolver: { song in try await container.resolveSourceAccess(for: song) }
        ))
    }

    var body: some Scene {
        WindowGroup {
            MainTabView(dependencies: dependencies)
                .environmentObject(tabState)
                .environmentObject(dependencies)
                .environmentObject(playerVM)
                .applyTheme()
                .defaultAppStorage(LocalWavePreferences.current)
                .frame(minWidth: 1024, minHeight: 680)
        }
        .defaultSize(width: 1440, height: 900)
    }
}
#endif
