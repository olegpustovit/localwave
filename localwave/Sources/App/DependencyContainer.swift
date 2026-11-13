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
    let sourceAccess: SourceAccess
    let songRepository: SongRepository
    let sourcePathRepository: SourcePathRepository
    let songImportService: SongImportService
    let playerPersistenceService: PlayerPersistenceService
    let playlistRepo: PlaylistRepository
    let playlistSongRepo: PlaylistSongRepository

    private var backgroundFileService: BackgroundFileService?
    private var tagEnrichmentService: TagEnrichmentService?

    @Published var sourceBrowseViewModels: [Int64: SourceBrowseViewModel] = [:]
    lazy var waveSyncViewModel = makeSyncViewModel()

    let logger = Logger(subsystem: subsystem, category: "DependencyContainer")

    func handleAppLaunch() {
        #if DEBUG
        if LWReferenceFixture.enabled { return }
        #endif
        logger.debug("Handling app launch...")
        startBackgroundServices()
        verifyPendingCopies()
        logger.debug("App launch handling complete")
    }

    func trimOfflineCache() async throws {
        try await backgroundFileService?.enforceCacheLimit()
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
        self.sourcePathRepository = sourcePathRepo
        let sourcePathSearchRepository = try SQLiteSourcePathSearchRepository(db: db)
        let songRepo = try SQLiteSongRepository(db: db)
        let access = SourceAccess(repository: sourceRepo, sourcePathRepository: sourcePathRepo, songRepository: songRepo)
        sourceAccess = access

        userService = DefaultUserService(userRepository: userRepo)
        icloudProvider = DefaultICloudProvider()
        userCloudService = DefaultUserCloudService(
            userService: userService,
            iCloudProvider: icloudProvider
        )

        let sourceSyncService = DefaultSourceSyncService(
            sourceRepository: sourceRepo,
            sourcePathSearchRepository: sourcePathSearchRepository,
            sourcePathRepository: sourcePathRepo,
            sourceAccess: access
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
        let importer = DefaultSongImportService(
            songRepo: songRepo,
            sourcePathRepo: sourcePathRepo,
            sourceRepo: sourceRepo,
            sourceAccess: access
        )
        songImportService = importer
        playerPersistenceService = DefaultPlayerPersistenceService(songRepo: songRepo)
        playlistRepo = try SQLitePlaylistRepository(db: db)
        playlistSongRepo = try SQLitePlaylistSongRepository(db: db)
        backgroundFileService = BackgroundFileService(songRepo: songRepo, sourceAccess: access)
        tagEnrichmentService = TagEnrichmentService(songRepo: songRepo, sourcePathRepo: sourcePathRepo, importer: importer, sourceAccess: access) { [weak self] done, total in
            // Fire-and-forget: awaiting the main thread per file stalled indexing whenever the UI was busy.
            Task { @MainActor in self?.waveSyncViewModel.tagStatus = total > 0 ? "Reading tags \(done)/\(total)" : nil }
        }
        if let tags = tagEnrichmentService {
            NotificationCenter.default.addObserver(forName: Notification.Name("LWPlaybackOpening"), object: nil, queue: nil) { _ in Task { await tags.yieldToUser() } }
        }
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
            songImportService: songImportService,
            sourceAccess: sourceAccess
        )
    }

    func resolveSourceAccess(for song: Song) async throws -> (root: URL, file: URL)? {
        if let sourceId = song.sourceId, let relativePath = song.relativePath {
            return try await sourceAccess.fileURL(sourceId: sourceId, relativePath: relativePath)
        }
        guard let user = try await userCloudService.resolveCurrentICloudUser(), let userID = user.id else { return nil }
        let sources = try await sourceService.repository().findOneByUserId(userId: userID, path: nil)
        for source in sources {
            guard let sourceID = source.id else { continue }
            guard let root = try? await sourceAccess.rootURL(sourceId: sourceID) else { continue }
            let paths = try await sourceService.importService().listItems(sourceId: sourceID, parentPathId: nil)
            guard let path = paths.first(where: { !$0.isDirectory && ($0.pathId == song.pathHash || makeURLHash(root.appendingPathComponent($0.relativePath)) == song.pathHash) }) else { continue }
            if let id = song.id { try await songRepository.updateSourceLocation(songId: id, sourceId: sourceID, relativePath: path.relativePath) }
            return (root, try SourceAccess.fileURL(root: root, relativePath: path.relativePath))
        }
        return nil
    }

    func reconnectSource(_ source: Source, to url: URL) async throws -> Source {
        guard let id = source.id else { throw CustomError.genericError("This music source was removed.") }
        return try await sourceAccess.reconnect(sourceId: id, to: url)
    }

    @discardableResult
    func addMusicURLs(_ urls: [URL]) async throws -> [Song] {
        var folders: [URL] = []
        var audio: [URL] = []
        var selectedFiles: [URL] = []
        let supported = AudioFileSupport.extensions
        for url in urls {
            guard url.isFileURL else { throw CustomError.genericError("Choose local files or folders from Files.") }
            let scoped = url.startAccessingSecurityScopedResource()
            defer { if scoped { url.stopAccessingSecurityScopedResource() } }
            if try url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory == true { folders.append(url) }
            else if supported.contains(url.pathExtension.lowercased()) { audio.append(url) }
            else { throw CustomError.genericError("Unsupported audio file: \(url.lastPathComponent)") }
        }
        let selectedFolders = folders
        if !audio.isEmpty {
            let imported = try await Task.detached(priority: .userInitiated) { try Self.copyAudio(audio) }.value
            folders.append(imported.folder)
            selectedFiles = imported.files
        }
        waveSyncViewModel.errorMessage = nil
        for folder in folders {
            try waveSyncViewModel.registerBookmark(folder)
            waveSyncViewModel.createSource(path: folder.path)
        }
        await waveSyncViewModel.awaitPendingWork()
        if let error = waveSyncViewModel.errorMessage { throw CustomError.genericError(error) }
        var results: [Song] = []
        for file in selectedFiles {
            if let song = await songRepository.getSongByURL(file.resolvingSymlinksInPath()) { results.append(song) }
        }
        if let user = try await userCloudService.resolveCurrentICloudUser(), let id = user.id {
            for folder in selectedFolders {
                let sources = try await sourceService.repository().findOneByUserId(userId: id, path: folder.path)
                for source in sources {
                    guard let sourceID = source.id else { continue }
                    let paths = try await sourceService.importService().listItems(sourceId: sourceID, parentPathId: nil)
                    for path in paths.filter(\.isAudio).sorted(by: { $0.relativePath.localizedStandardCompare($1.relativePath) == .orderedAscending }) {
                        if let song = await songRepository.getSong(sourceId: sourceID, relativePath: path.relativePath) { results.append(song) }
                    }
                }
            }
        }
        var seen = Set<Int64>()
        return results.filter { seen.insert($0.songKey).inserted }
    }

    nonisolated static func copyAudio(_ urls: [URL]) throws -> (folder: URL, files: [URL]) {
            let folder = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("Added Files", isDirectory: true)
            var copied: [URL] = []
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            for url in urls {
                let scoped = url.startAccessingSecurityScopedResource()
                defer { if scoped { url.stopAccessingSecurityScopedResource() } }
                var destination = folder.appendingPathComponent(url.lastPathComponent)
                if FileManager.default.fileExists(atPath: destination.path) {
                    if FileManager.default.contentsEqual(atPath: url.path, andPath: destination.path) { copied.append(destination); continue }
                    destination = folder.appendingPathComponent(UUID().uuidString + "-" + url.lastPathComponent)
                }
                var coordinationError: NSError?
                var copyError: Error?
                NSFileCoordinator().coordinate(readingItemAt: url, options: [], error: &coordinationError) { readableURL in
                    let staging = folder.appendingPathComponent(".copy-" + UUID().uuidString)
                    defer { try? FileManager.default.removeItem(at: staging) }
                    do {
                        try FileManager.default.copyItem(at: readableURL, to: staging)
                        try FileManager.default.moveItem(at: staging, to: destination)
                    } catch { copyError = error }
                }
                if let coordinationError { throw coordinationError }
                if let copyError { throw copyError }
                copied.append(destination)
            }
            return (folder, copied)
    }

    private func startBackgroundServices() {
        logger.debug("starting background service...")
        guard let service = backgroundFileService else {
            logger.error("failed to initialise background file service")
            LWPresentationNotice.error("Offline copying is unavailable: the background file service could not start.")
            return
        }

        Task {
            await service.start()
            await tagEnrichmentService?.start()
            logger.debug("background file service startup triggered")
        }
    }
}
