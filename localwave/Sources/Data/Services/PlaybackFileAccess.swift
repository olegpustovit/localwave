import Foundation
import LocalWaveCore
import LocalWaveDomain

public enum PlaybackFileAccess {
    public typealias SourceResolver = @Sendable (Song) async throws -> (root: URL, file: URL)?

    public static func prepare(_ song: Song, resolver: SourceResolver?, cacheDirectory: URL? = nil, preferences: UserDefaults = LocalWavePreferences.current) async throws -> (url: URL, renewedBookmark: Data?) {
        try Task.checkCancellation()
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        if let path = song.localFilePath {
            let url = path.hasPrefix("/") ? URL(fileURLWithPath: path) : documents.appendingPathComponent(path)
            let readable = withOfflineCacheLock { () -> Bool in
                guard FileManager.default.isReadableFile(atPath: url.path) else { return false }
                    if let id = song.id, preferences.string(forKey: "lwActiveSongID") == String(id) {
                        preferences.set(url.resolvingSymlinksInPath().path, forKey: "lwActiveCachePath")
                    }
                return true
            }
            if readable {
                try FileAvailabilityService.requireLocalForNetworkSimulation(url, preferences: preferences)
                return (url, nil)
            }
        }
        var sourceError: Error?
        if let resolver {
            do {
                if let access = try await resolver(song) {
                    return try await materialize(source: access.file, scope: access.root, songID: song.id, renewBookmark: false, cacheDirectory: cacheDirectory, preferences: preferences)
                }
            } catch {
                try Task.checkCancellation()
                sourceError = error
            }
        }
        if let bookmark = song.bookmark {
            var stale = false
            let source = try URL(resolvingBookmarkData: bookmark, options: sourceBookmarkResolutionOptions.union(.withoutUI), bookmarkDataIsStale: &stale)
            return try await materialize(source: source, scope: source, songID: song.id, renewBookmark: stale, cacheDirectory: cacheDirectory, preferences: preferences)
        }
        throw sourceError ?? CustomError.genericError("No accessible local file or source folder is available. Reconnect its source folder.")
    }

    private static func materialize(source: URL, scope: URL, songID: Int64?, renewBookmark: Bool, cacheDirectory: URL?, preferences: UserDefaults) async throws -> (url: URL, renewedBookmark: Data?) {
        let scoped = scope.startAccessingSecurityScopedResource()
        defer { if scoped { scope.stopAccessingSecurityScopedResource() } }
        guard scoped || FileManager.default.isReadableFile(atPath: source.path) else {
            throw CustomError.genericError("Unable to read this audio file. Reconnect its source folder.")
        }
        try FileAvailabilityService.requireLocalForNetworkSimulation(source, preferences: preferences)
        let coordinator = NSFileCoordinator(filePresenter: nil)
        return try await withTaskCancellationHandler {
            var coordinationError: NSError?
            var result: Result<(url: URL, renewedBookmark: Data?), Error>?
            coordinator.coordinate(readingItemAt: source, options: .withoutChanges, error: &coordinationError) { readable in
                result = Result {
                    try Task.checkCancellation()
                    let directory = cacheDirectory ?? FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("Music", isDirectory: true)
                    let destination = try BackgroundFileService.copyStream(readable, into: directory, protectingSongID: songID, versionSourceURL: source, preferences: preferences)
                    let renewed = renewBookmark ? try source.bookmarkData(options: sourceBookmarkCreationOptions) : nil
                    return (destination, renewed)
                }
            }
            if let coordinationError { throw coordinationError }
            guard let result else { throw CancellationError() }
            return try result.get()
        } onCancel: { coordinator.cancel() }
    }
}
