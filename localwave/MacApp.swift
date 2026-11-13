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
    #if DEBUG
    @State private var referenceReady = !LWReferenceFixture.enabled
    #endif

    init() {
        for name in ["HostGrotesk", "GeistMono"] {
            if let url = Bundle.main.url(forResource: name, withExtension: "ttf") {
                CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
            }
        }
        #if DEBUG
        if let testID = ProcessInfo.processInfo.environment["LOCALWAVE_UI_TEST_ID"], UUID(uuidString: testID) != nil {
            LocalWavePreferences.current.set(LWReferenceFixture.screen != "onboarding" && ProcessInfo.processInfo.environment["LOCALWAVE_UI_EMPTY_LIBRARY"] != "1", forKey: "lwDidStart")
        }
        #endif
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
            #if DEBUG
            if !referenceReady {
                Color.clear.task {
                    do { try await LWReferenceFixture.seed(dependencies, player: playerVM) }
                    catch { FileHandle.standardOutput.write(Data("LOCALWAVE_REFERENCE_ERROR=\(error)\n".utf8)) }
                    referenceReady = true
                }
            } else { mainView }
            #else
            mainView
            #endif
        }
        .defaultSize(width: 1440, height: 900)
    }

    private var mainView: some View {
        MainTabView(dependencies: dependencies)
            .environmentObject(tabState)
            .environmentObject(dependencies)
            .environmentObject(playerVM)
            .applyTheme()
            .defaultAppStorage(LocalWavePreferences.current)
            .frame(minWidth: 1024, minHeight: 680)
    }
}
#endif
