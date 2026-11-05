import Foundation
import XCTest
import SQLite
import LocalWaveCore
import LocalWaveDomain
@testable import LocalWaveData

final class FileAvailabilityTests: XCTestCase {
    func testKnownEvictedCloudIndexesWithoutReadingContentThenEnrichesSameRecord() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let audio = root.appendingPathComponent("evicted.flac")
        try Data().write(to: audio)
        try FileManager.default.setAttributes([.posixPermissions: 0], ofItemAtPath: audio.path)
        defer { try? FileManager.default.removeItem(at: root) }
        let database = try Connection(.inMemory)
        let sources = try SQLiteSourceRepository(db: database)
        let paths = try SQLiteSourcePathRepository(db: database)
        let songs = try SQLiteSongRepository(db: database)
        let playlists = try SQLitePlaylistRepository(db: database)
        let links = try SQLitePlaylistSongRepository(db: database)
        let source = try await sources.create(source: Source(id: nil, dirPath: root.path, pathId: makeURLHash(root), userId: 1, type: .iCloud, totalPaths: 1, syncError: nil, isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil, bookmark: root.bookmarkData(options: sourceBookmarkCreationOptions)))
        let sourceID = try XCTUnwrap(source.id)
        let path = try await paths.create(path: SourcePath(id: nil, sourceId: sourceID, pathId: makeURLHash(audio), parentPathId: source.pathId, name: "evicted.flac", relativePath: "evicted.flac", isDirectory: false, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil))
        let availability = AvailabilityState(.inCloud)
        let importer = DefaultSongImportService(songRepo: songs, sourcePathRepo: paths, sourceRepo: sources, availabilityInspector: { _ in availability.value })
        try await importer.importPaths(paths: [path], onProgress: nil)
        let placeholderRow = await songs.getSong(sourceId: sourceID, relativePath: path.relativePath)
        let placeholder = try XCTUnwrap(placeholderRow)
        XCTAssertEqual(placeholder.title, "evicted")
        XCTAssertEqual(placeholder.sourceAvailability, .inCloud)
        XCTAssertNil(placeholder.duration)
        XCTAssertNil(placeholder.bookmark)
        let indexedPath = try await paths.getByPathId(sourceId: sourceID, pathId: path.pathId)
        XCTAssertNil(indexedPath?.fileHashSHA256)
        let id = try XCTUnwrap(placeholder.id)
        try await songs.toggleSongAsFavorite(songId: id, state: true)
        let list = try await playlists.create(playlist: Playlist(id: nil, name: "Cloud favorite", createdAt: Date(), updatedAt: nil))
        try await links.addSong(playlistId: XCTUnwrap(list.id), songId: id)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: audio.path)
        try FileManager.default.removeItem(at: audio)
        let fixture = try XCTUnwrap(Bundle.module.url(forResource: "tagged", withExtension: "flac", subdirectory: "Fixtures"))
        try FileManager.default.copyItem(at: fixture, to: audio)
        availability.value = .onDevice
        try await importer.importPaths(paths: [path], onProgress: nil)
        let enrichedRow = await songs.getSong(sourceId: sourceID, relativePath: path.relativePath)
        let enriched = try XCTUnwrap(enrichedRow)
        XCTAssertEqual(enriched.id, id)
        XCTAssertEqual(enriched.title, "FLAC Fixture")
        XCTAssertEqual(enriched.isFavorite, 1)
        XCTAssertEqual(enriched.sourceAvailability, .onDevice)
        XCTAssertGreaterThan(try XCTUnwrap(enriched.duration), 0)
        let memberships = try await links.getSongs(playlistId: XCTUnwrap(list.id))
        XCTAssertEqual(memberships.map(\.id), [id])
        XCTAssertEqual(memberships.first?.sourceAvailability, .onDevice)
    }

    func testUnknownProviderDefersOriginalAndEnrichesOnlyVerifiedMaterialization() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let provider = root.appendingPathComponent("Library/FileProviderStorage", isDirectory: true)
        try FileManager.default.createDirectory(at: provider, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let audio = provider.appendingPathComponent("provider.flac")
        try Data().write(to: audio)
        let database = try Connection(.inMemory)
        let sources = try SQLiteSourceRepository(db: database)
        let paths = try SQLiteSourcePathRepository(db: database)
        let songs = try SQLiteSongRepository(db: database)
        let source = try await sources.create(source: Source(id: nil, dirPath: provider.path, pathId: makeURLHash(provider), userId: 1, type: .iCloud, totalPaths: 1, syncError: nil, isCurrent: true, createdAt: Date(), lastSyncedAt: nil, updatedAt: nil, bookmark: provider.bookmarkData(options: sourceBookmarkCreationOptions)))
        let sourceID = try XCTUnwrap(source.id)
        let path = try await paths.create(path: SourcePath(id: nil, sourceId: sourceID, pathId: makeURLHash(audio), parentPathId: source.pathId, name: audio.lastPathComponent, relativePath: audio.lastPathComponent, isDirectory: false, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil))
        let importer = DefaultSongImportService(songRepo: songs, sourcePathRepo: paths, sourceRepo: sources)
        try await importer.importPaths(paths: [path], onProgress: nil)
        let placeholderRow = await songs.getSong(sourceId: sourceID, relativePath: path.relativePath)
        let placeholder = try XCTUnwrap(placeholderRow)
        let id = try XCTUnwrap(placeholder.id)
        XCTAssertEqual(placeholder.sourceAvailability, .unknown)
        XCTAssertNil(placeholder.duration)
        let initialPath = try await paths.getByPathId(sourceId: sourceID, pathId: path.pathId)
        XCTAssertNil(initialPath?.fileHashSHA256)
        let fixture = try XCTUnwrap(Bundle.module.url(forResource: "tagged", withExtension: "flac", subdirectory: "Fixtures"))
        try FileManager.default.removeItem(at: audio)
        try FileManager.default.copyItem(at: fixture, to: audio)
        let cache = try BackgroundFileService.copyCoordinated(audio, into: root.appendingPathComponent("Cache"), songID: id)
        let copied = try await songs.updateCachedFile(songId: id, localFilePath: cache.path, state: .copied, expectedState: .bookmarkOnly, expectedPath: nil)
        XCTAssertTrue(copied)
        try await songs.toggleSongAsFavorite(songId: id, state: true)
        try await importer.importPaths(paths: [path], onProgress: nil)
        let enrichedRow = await songs.getSong(sourceId: sourceID, relativePath: path.relativePath)
        let enriched = try XCTUnwrap(enrichedRow)
        XCTAssertEqual(enriched.id, id)
        XCTAssertEqual(enriched.title, "FLAC Fixture")
        XCTAssertEqual(enriched.isFavorite, 1)
        XCTAssertEqual(enriched.localFilePath, cache.path)
        XCTAssertEqual(enriched.sourceAvailability, .unknown)
        XCTAssertGreaterThan(try XCTUnwrap(enriched.duration), 0)
        let indexed = try await paths.getByPathId(sourceId: sourceID, pathId: path.pathId)
        let hash = try XCTUnwrap(indexed?.fileHashSHA256)
        let handle = try FileHandle(forWritingTo: audio)
        try handle.seekToEnd(); try handle.write(contentsOf: Data([0])); try handle.close()
        try await importer.importPaths(paths: [try XCTUnwrap(indexed)], onProgress: nil)
        let invalidated = await songs.getSong(sourceId: sourceID, relativePath: path.relativePath)
        XCTAssertNil(invalidated?.localFilePath)
        XCTAssertEqual(invalidated?.title, "FLAC Fixture")
        let unchanged = try await paths.getByPathId(sourceId: sourceID, pathId: path.pathId)
        XCTAssertEqual(unchanged?.fileHashSHA256, hash, "Indexing must not read changed unknown provider bytes")
        XCTAssertTrue(FileManager.default.fileExists(atPath: cache.path), "Invalidation must not delete cached bytes")
        XCTAssertFalse(FileAvailabilityService.isProviderLocation(root.appendingPathComponent("CloudStorage/song.flac")))
    }

    func testActualCacheProofAndUnknownProviderStatusDoNotInventCloudState() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let provider = root.appendingPathComponent("Library/FileProviderStorage", isDirectory: true)
        try FileManager.default.createDirectory(at: provider, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let local = root.appendingPathComponent("local.flac")
        let unknown = provider.appendingPathComponent("provider.flac")
        try Data([1]).write(to: local)
        try Data([1]).write(to: unknown)
        XCTAssertEqual(try FileAvailabilityService.inspect(local), .onDevice)
        XCTAssertEqual(try FileAvailabilityService.inspect(unknown), .unknown)
        let access = SourceAccess(repository: try SQLiteSourceRepository(db: Connection(.inMemory)))
        func song(path: String) -> Song {
            Song(id: 1, songKey: 1, artist: "Artist", title: "Track", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 1, createdAt: Date(), updatedAt: nil, localFilePath: path, fileState: .copied, sourceAvailability: .inCloud)
        }
        let present = await access.availability(for: song(path: local.path))
        let missing = await access.availability(for: song(path: root.appendingPathComponent("missing.flac").path))
        XCTAssertEqual(present, .onDevice)
        XCTAssertEqual(missing, .unknown)
    }
}

private final class AvailabilityState: @unchecked Sendable {
    private let lock = NSLock()
    private var stored: FileAvailability
    init(_ value: FileAvailability) { stored = value }
    var value: FileAvailability {
        get { lock.lock(); defer { lock.unlock() }; return stored }
        set { lock.lock(); stored = newValue; lock.unlock() }
    }
}
