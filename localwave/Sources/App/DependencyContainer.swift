import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
import Foundation
import os
import SwiftUI

// MARK: - Complete SQLitePlaylistSongRepository implementation

@MainActor
class DependencyContainer: ObservableObject {
    let userService: UserService
    let userCloudService: UserCloudService
    let icloudProvider: ICloudProvider
    let sourceService: SourceService
    let songRepository: SongRepository
    let songImportService: SongImportService
    let playerPersistenceService: PlayerPersistenceService
    let playlistRepo: PlaylistRepository
    let playlistSongRepo: PlaylistSongRepository

    private var backgroundFileService: BackgroundFileService?

    @Published var sourceBrowseViewModels: [Int64: SourceBrowseViewModel] = [:]

    let logger = Logger(subsystem: subsystem, category: "DependencyContainer")

    func handleAppLaunch() {
        logger.debug("Handling app launch...")
        startBackgroundServices()
        verifyPendingCopies()
        logger.debug("App launch handling complete")
    }

    private func verifyPendingCopies() {
        Task(priority: .utility) {
            let pending = await songRepository.getSongsNeedingCopy()
            logger.debug("Found \(pending.count) songs needing copy verification")
        }
    }

    init() throws {
        var databaseName = "musicApp\(schemaVersion).sqlite"
        #if DEBUG
        if let testID = ProcessInfo.processInfo.environment["LOCALWAVE_UI_TEST_ID"], UUID(uuidString: testID) != nil {
            databaseName = "uitest-\(testID).sqlite"
        }
        #endif
        guard let db = setupSQLiteConnection(dbName: databaseName) else {
            throw CustomError.genericError("database initialisation failed")
        }

        let userRepo = try SQLiteUserRepository(db: db)
        let sourceRepo = try SQLiteSourceRepository(db: db)
        let sourcePathRepo = try SQLiteSourcePathRepository(db: db)
        let sourcePathSearchRepository = try SQLiteSourcePathSearchRepository(db: db)
        let songRepo = try SQLiteSongRepository(db: db)

        userService = DefaultUserService(userRepository: userRepo)
        icloudProvider = DefaultICloudProvider()
        userCloudService = DefaultUserCloudService(
            userService: userService,
            iCloudProvider: icloudProvider
        )

        let sourceSyncService = DefaultSourceSyncService(
            sourceRepository: sourceRepo,
            sourcePathSearchRepository: sourcePathSearchRepository,
            sourcePathRepository: sourcePathRepo
        )

        let sourceImportService = DefaultSourceImportService(
            sourceRepository: sourceRepo,
            sourcePathRepository: sourcePathRepo,
            sourcePathSearchRepository: sourcePathSearchRepository
        )
        sourceService = DefaultSourceService(
            sourceRepo: sourceRepo,
            sourceSyncService: sourceSyncService,
            sourceImportService: sourceImportService
        )
        songRepository = songRepo
        songImportService = DefaultSongImportService(
            songRepo: songRepo,
            sourcePathRepo: sourcePathRepo,
            sourceRepo: sourceRepo
        )
        playerPersistenceService = DefaultPlayerPersistenceService(songRepo: songRepo)
        playlistRepo = try SQLitePlaylistRepository(db: db)
        playlistSongRepo = try SQLitePlaylistSongRepository(db: db)
        backgroundFileService = BackgroundFileService(songRepo: songRepo)
    }

    func makeSongListViewModel(filter: SongListViewModel.Filter) -> SongListViewModel {
        SongListViewModel(songRepo: songRepository, filter: filter)
    }

    func makeArtistListViewModel() -> ArtistListViewModel {
        ArtistListViewModel(songRepo: songRepository)
    }

    func makeAlbumListViewModel() -> AlbumListViewModel {
        AlbumListViewModel(songRepo: songRepository)
    }

    func makePlaylistListViewModel() -> PlaylistListViewModel {
        PlaylistListViewModel(
            playlistRepo: playlistRepo,
            playlistSongRepo: playlistSongRepo,
            songRepo: songRepository
        )
    }

    func makeSyncViewModel() -> SyncViewModel {
        SyncViewModel(
            userCloudService: userCloudService,
            icloudProvider: icloudProvider,
            sourceService: sourceService,
            songImportService: songImportService
        )
    }

    private func startBackgroundServices() {
        logger.debug("starting background service...")
        guard let service = backgroundFileService else {
            logger.error("failed to initialise background file service")
            return
        }

        Task {
            await service.start()
            logger.debug("background file service startup triggered")
        }
    }
}
