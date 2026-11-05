import Foundation
import XCTest
import LocalWaveDomain
@testable import LocalWaveData

final class NetworkSimulationTests: XCTestCase {
    func testUnconfirmedProviderIsRejectedBeforeEitherCopyPathCreatesFiles() throws {
        let root = try folder()
        defer { try? FileManager.default.removeItem(at: root) }
        let source = try providerFile(in: root)
        let suite = "network-simulation-" + UUID().uuidString
        let preferences = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { preferences.removePersistentDomain(forName: suite) }
        preferences.set(true, forKey: "lwSimulateNoNetwork")
        XCTAssertEqual(try FileAvailabilityService.inspect(source), .unknown)
        let destination = root.appendingPathComponent("Cache")
        XCTAssertThrowsError(try BackgroundFileService.copyCoordinated(source, into: destination, preferences: preferences)) {
            XCTAssertEqual(($0 as? URLError)?.code, .notConnectedToInternet)
        }
        XCTAssertThrowsError(try BackgroundFileService.copyStream(source, into: destination, preferences: preferences)) {
            XCTAssertEqual(($0 as? URLError)?.code, .notConnectedToInternet)
        }
        XCTAssertFalse(FileManager.default.fileExists(atPath: destination.path))
        XCTAssertEqual(try Data(contentsOf: source), Data([1, 2, 3, 4]))
    }

    func testKnownLocalPlaybackRemainsAvailableDuringSimulation() async throws {
        let root = try folder()
        defer { try? FileManager.default.removeItem(at: root) }
        let source = root.appendingPathComponent("cached.flac")
        try Data([1, 2, 3, 4]).write(to: source)
        let suite = "network-simulation-" + UUID().uuidString
        let preferences = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { preferences.removePersistentDomain(forName: suite) }
        preferences.set(true, forKey: "lwSimulateNoNetwork")
        let cached = song(localFilePath: source.path)
        let prepared = try await PlaybackFileAccess.prepare(cached, resolver: { _ in
            XCTFail("A verified local copy must not resolve or fetch its provider")
            return nil
        }, preferences: preferences)
        XCTAssertEqual(prepared.url, source)
        XCTAssertNil(prepared.renewedBookmark)
    }

    func testProviderPlaybackFailsWithoutACacheThenRetriesAfterSimulationEnds() async throws {
        let root = try folder()
        defer { try? FileManager.default.removeItem(at: root) }
        let source = try providerFile(in: root)
        let destination = root.appendingPathComponent("Cache")
        let suite = "network-simulation-" + UUID().uuidString
        let preferences = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { preferences.removePersistentDomain(forName: suite) }
        preferences.set(true, forKey: "lwSimulateNoNetwork")
        do {
            _ = try await PlaybackFileAccess.prepare(song(), resolver: { _ in (root, source) }, cacheDirectory: destination, preferences: preferences)
            XCTFail("Unconfirmed provider data must fail before materialization")
        } catch { XCTAssertEqual((error as? URLError)?.code, .notConnectedToInternet) }
        XCTAssertFalse(FileManager.default.fileExists(atPath: destination.path))
        preferences.set(false, forKey: "lwSimulateNoNetwork")
        let prepared = try await PlaybackFileAccess.prepare(song(), resolver: { _ in (root, source) }, cacheDirectory: destination, preferences: preferences)
        XCTAssertEqual(prepared.url.deletingLastPathComponent().path, destination.path)
        XCTAssertEqual(try Data(contentsOf: prepared.url), Data([1, 2, 3, 4]))
    }

    private func folder() throws -> URL {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        return root
    }

    private func providerFile(in root: URL) throws -> URL {
        let directory = root.appendingPathComponent("Library/CloudStorage/Provider", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("unconfirmed.flac")
        try Data([1, 2, 3, 4]).write(to: source)
        return source
    }

    private func song(localFilePath: String? = nil) -> Song {
        Song(id: 9001, songKey: 9001, artist: "Artist", title: "Track", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 9001, createdAt: Date(), updatedAt: nil, localFilePath: localFilePath, fileState: localFilePath == nil ? .bookmarkOnly : .copied)
    }
}
