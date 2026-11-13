import Foundation

public protocol PlayerPersistenceService {
    func getVolume() async -> Float?
    func restore() async -> ([Song], Int, Song?)?
    func savePlaybackState(volume: Float, currentIndex: Int, songs: [Song]) async
}

public protocol PlaylistRepository {
    func create(playlist: Playlist) async throws -> Playlist
    func update(playlist: Playlist) async throws -> Playlist
    func delete(playlistId: Int64) async throws
    func getAll() async throws -> [Playlist]
    func getOne(id: Int64) async throws -> Playlist?
}

public protocol PlaylistSongRepository {
    func addSong(playlistId: Int64, songId: Int64) async throws
    func removeSong(playlistId: Int64, songId: Int64) async throws
    func getSongs(playlistId: Int64) async throws -> [Song]
    func reorderSongs(playlistId: Int64, newOrder: [Int64]) async throws // New
}

public protocol SongRepository {
    /// Upsert a song based on its songKey (hash of artist/title/album).
    /// If a row with the same key exists, update it; otherwise insert new.
    func upsertSong(_ song: Song) async throws -> Song

    /// Full-text search by artist/title/album, returning at most `limit` songs,
    /// ordered by `bm25(...)`.
    func searchSongsFTS(query: String, limit: Int, offset: Int) async throws -> [Song]
    func totalSongCount(query: String) async throws -> Int

    func getAllArtists() async throws -> [String]
    func getAllAlbums() async throws -> [Album]
    func getSongsByArtist(_ artist: String) async throws -> [Song]
    func getSongsByAlbum(_ album: String, artist: String?) async throws -> [Song]

    func getSongs(ids: [Int64]) async -> [Song]
    func getSongByURL(_ url: URL) async -> Song?
    func getSong(sourceId: Int64, relativePath: String) async -> Song?
    func updateBookmark(songId: Int64, bookmark: Data) async throws
    func updateSourceAvailability(songId: Int64, availability: FileAvailability) async throws
    func updateSourceLocation(songId: Int64, sourceId: Int64, relativePath: String) async throws
    func deleteSong(songId: Int64) async throws
    func deleteAlbum(album: String, artist: String?) async throws
    func getSongsNeedingCopy() async -> [Song]
    func getSongsMissingMetadata() async -> [Song]
    func markSongForCopy(songId: Int64) async throws
    func updateCachedFile(songId: Int64, localFilePath: String?, state: FileState, expectedState: FileState, expectedPath: String?) async throws -> Bool
    func toggleSongAsFavorite(songId: Int64, state: Bool) async throws
}

public protocol SourceImportService {
    func listItems(sourceId: Int64, parentPathId: Int64?) async throws -> [SourcePath]
    func search(sourceId: Int64, query: String) async throws -> [SourcePath]
    func deleteOne(sourceId: Int64) async throws
}

public protocol SourcePathSearchRepository {
    func batchUpsertIntoFTS(paths: [SourcePath]) async throws
    func search(sourceId: Int64, query: String, limit: Int) async throws -> [PathSearchResult]
    func batchDeleteFTS(sourceId: Int64, excludingRunId: Int64) async throws
    func deleteAllFTS(sourceId: Int64) async throws
    func deletePaths(sourceId: Int64, pathIds: [Int64]) async throws
}

public protocol SourceSyncService {
    func syncPaths(sourceId: Int64, relativePaths: Set<String>) async throws -> (source: Source?, audioPaths: [SourcePath])
    func syncDir(
        sourceId: Int64,
        folderURL: URL,
        onCurrentURL: ((_ url: URL?) -> Void)?,
        onSetLoading: ((_ loading: Bool) -> Void)?
    ) async throws
        -> Source?
}

public protocol SongImportService {
    func importPaths(
        paths: [SourcePath],
        onProgress: ((Double, URL) async -> Void)?
    ) async throws

    func importPathsWithFileProgress(paths: [SourcePath], onProgress: ((SongImportProgress) async -> Void)?) async throws
    func cancelImport() async
}

public extension SongImportService {
    func importPathsWithFileProgress(paths: [SourcePath], onProgress: ((SongImportProgress) async -> Void)?) async throws {
        try await importPaths(paths: paths) { percentage, url in
            await onProgress?(SongImportProgress(percentage: percentage, url: url))
        }
    }
}

public protocol SourceService {
    func registerSourcePath(userId: Int64, path: String, type: SourceType) async throws -> Source
    func getCurrentSource(userId: Int64) async throws -> Source?
    func syncService() -> SourceSyncService
    func importService() -> SourceImportService
    func repository() -> SourceRepository
}

public protocol SourcePathRepository {
    func getByParentId(sourceId: Int64, parentPathId: Int64?) async throws -> [SourcePath]
    func getByPathId(sourceId: Int64, pathId: Int64) async throws -> SourcePath?
    func create(path: SourcePath) async throws -> SourcePath
    func updateFileHash(pathId: Int64, fileHash: Data?) async throws
    func deleteMany(sourceId: Int64) async throws
    func getByParentId(parentId: Int64) async throws -> [SourcePath]
    func getByPath(relativePath: String, sourceId: Int64) async throws -> SourcePath?
    func batchUpsert(paths: [SourcePath]) async throws
    func deleteMany(sourceId: Int64, excludingRunId: Int64) async throws -> Int
    func deleteAllPaths(sourceId: Int64) async throws
    func deletePaths(sourceId: Int64, pathIds: [Int64]) async throws
}

public protocol SourceRepository {
    func updateSourceBookmark(sourceId: Int64, bookmark: Data, expectedBookmark: Data?) async throws -> Bool
    func reconnectSource(sourceId: Int64, dirPath: String, pathId: Int64, bookmark: Data) async throws -> Source?
    func updateResolvedSourceRoot(sourceId: Int64, dirPath: String, pathId: Int64, bookmark: Data, expectedBookmark: Data?) async throws -> Bool
    func deleteSource(sourceId: Int64) async throws
    func create(source: Source) async throws -> Source
    func findOneByUserId(userId: Int64, path: String?) async throws -> [Source]
    func getOne(id: Int64) async throws -> Source?
    func updateSource(source: Source) async throws -> Source
    // needs to set isCurrent true to the source with userId
    // and for the rest of users libraries set isCurrentFalse
    func setCurrentSource(userId: Int64, sourceId: Int64) async throws -> Source
}

public protocol UserRepository {
    func findByIcloudId(icloudId: Int64) async throws -> User?
    func create(user: User) async throws -> User
}

public protocol UserService {
    func getOrCreateUser(icloudId: Int64) async throws -> User
}

public protocol UserCloudService {
    func resolveCurrentICloudUser() async throws -> User?
}

public protocol ICloudProvider {
    func getCurrentICloudUserID() async throws -> Int64?
    func isICloudAvailable() -> Bool
}

public extension SongRepository {
    /// Cloud placeholders waiting for real tags (no duration yet). Default empty for test doubles.
    func getSongsMissingMetadata() async -> [Song] { [] }

    func getSongsByArtist(_ artist: String) async throws -> [Song] {
        try await searchSongsFTS(query: artist, limit: Int.max, offset: 0).filter { $0.artist == artist || $0.artistNames.contains { $0.localizedCaseInsensitiveCompare(artist) == .orderedSame } }
    }

    func getSongsByAlbum(_ album: String, artist: String?) async throws -> [Song] {
        try await searchSongsFTS(query: album, limit: Int.max, offset: 0).filter {
            $0.album == album && (artist == nil || $0.artist == artist)
        }
    }
}
