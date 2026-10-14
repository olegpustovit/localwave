import Foundation

protocol PlayerPersistenceService {
    func getVolume() async -> Float?
    func restore() async -> ([Song], Int, Song?)?
    func savePlaybackState(volume: Float, currentIndex: Int, songs: [Song]) async
}

protocol PlaylistRepository {
    func create(playlist: Playlist) async throws -> Playlist
    func update(playlist: Playlist) async throws -> Playlist
    func delete(playlistId: Int64) async throws
    func getAll() async throws -> [Playlist]
    func getOne(id: Int64) async throws -> Playlist?
}

protocol PlaylistSongRepository {
    func addSong(playlistId: Int64, songId: Int64) async throws
    func removeSong(playlistId: Int64, songId: Int64) async throws
    func getSongs(playlistId: Int64) async throws -> [Song]
    func reorderSongs(playlistId: Int64, newOrder: [Int64]) async throws // New
}

protocol SongRepository {
    /// Upsert a song based on its songKey (hash of artist/title/album).
    /// If a row with the same key exists, update it; otherwise insert new.
    func upsertSong(_ song: Song) async throws -> Song

    /// Full-text search by artist/title/album, returning at most `limit` songs,
    /// ordered by `bm25(...)`.
    func searchSongsFTS(query: String, limit: Int, offset: Int) async throws -> [Song]
    func totalSongCount(query: String) async throws -> Int

    func getAllArtists() async throws -> [String]
    func getAllAlbums() async throws -> [Album]

    func getSongs(ids: [Int64]) async -> [Song]
    func getSongByURL(_ url: URL) async -> Song?
    func updateBookmark(songId: Int64, bookmark: Data) async throws
    func deleteSong(songId: Int64) async throws
    func deleteAlbum(album: String, artist: String?) async throws
    func getSongsNeedingCopy() async -> [Song]
    func markSongForCopy(songId: Int64) async throws
    func toggleSongAsFavorite(songId: Int64, state: Bool) async throws
}

protocol SourceImportService {
    func listItems(sourceId: Int64, parentPathId: Int64?) async throws -> [SourcePath]
    func search(sourceId: Int64, query: String) async throws -> [SourcePath]
    func deleteOne(sourceId: Int64) async throws
}

protocol SourcePathSearchRepository {
    func batchUpsertIntoFTS(paths: [SourcePath]) async throws
    func search(sourceId: Int64, query: String, limit: Int) async throws -> [PathSearchResult]
    func batchDeleteFTS(sourceId: Int64, excludingRunId: Int64) async throws
    func deleteAllFTS(sourceId: Int64) async throws
}

protocol SourceSyncService {
    func syncDir(
        sourceId: Int64,
        folderURL: URL,
        onCurrentURL: ((_ url: URL?) -> Void)?,
        onSetLoading: ((_ loading: Bool) -> Void)?
    ) async throws
        -> Source?
}

protocol SongImportService {
    func importPaths(
        paths: [SourcePath],
        onProgress: ((Double, URL) async -> Void)?
    ) async throws

    func cancelImport() async
}

protocol SourceService {
    func registerSourcePath(userId: Int64, path: String, type: SourceType) async throws -> Source
    func getCurrentSource(userId: Int64) async throws -> Source?
    func syncService() -> SourceSyncService
    func importService() -> SourceImportService
    func repository() -> SourceRepository
}

protocol SourcePathRepository {
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
}

protocol SourceRepository {
    func deleteSource(sourceId: Int64) async throws
    func create(source: Source) async throws -> Source
    func findOneByUserId(userId: Int64, path: String?) async throws -> [Source]
    func getOne(id: Int64) async throws -> Source?
    func updateSource(source: Source) async throws -> Source
    // needs to set isCurrent true to the source with userId
    // and for the rest of users libraries set isCurrentFalse
    func setCurrentSource(userId: Int64, sourceId: Int64) async throws -> Source
}

protocol UserRepository {
    func findByIcloudId(icloudId: Int64) async throws -> User?
    func create(user: User) async throws -> User
}

protocol UserService {
    func getOrCreateUser(icloudId: Int64) async throws -> User
}

protocol UserCloudService {
    func resolveCurrentICloudUser() async throws -> User?
}

protocol ICloudProvider {
    func getCurrentICloudUserID() async throws -> Int64?
    func isICloudAvailable() -> Bool
}
