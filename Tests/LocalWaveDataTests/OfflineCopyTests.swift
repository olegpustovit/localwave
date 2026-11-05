import XCTest
import Foundation
import CryptoKit
@testable import LocalWaveData

final class OfflineCopyTests: XCTestCase {
    func testConcurrentCopiesPromoteOneCompleteDestination() async throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let source = folder.appendingPathComponent("same.flac")
        let content = Data(repeating: 42, count: 1024 * 1024)
        try content.write(to: source)
        let destination = folder.appendingPathComponent("Music")
        let tasks = (0..<8).map { _ in Task.detached { try BackgroundFileService.copyStream(source, into: destination) } }
        var files: [URL] = []
        for task in tasks { files.append(try await task.value) }
        XCTAssertEqual(Set(files).count, 1)
        XCTAssertEqual(try Data(contentsOf: XCTUnwrap(files.first)), content)
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: destination.path).count, 1)
    }
    func testCoordinatedCopyStreamsAllChunksAndReusesIdenticalFile() throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let source = folder.appendingPathComponent("large.FLAC")
        let content = Data((0..<(768 * 1024 + 37)).map { UInt8($0 % 251) })
        try content.write(to: source)
        let destination = folder.appendingPathComponent("Music")
        let copied = try BackgroundFileService.copyCoordinated(source, into: destination)
        let expectedHash = SHA256.hash(data: content).map { String(format: "%02x", $0) }.joined()
        XCTAssertEqual(copied.lastPathComponent, expectedHash + ".flac")
        XCTAssertEqual(try Data(contentsOf: copied), content)
        let again = try BackgroundFileService.copyCoordinated(source, into: destination)
        XCTAssertEqual(again, copied)
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: destination.path), [copied.lastPathComponent])
    }

    func testFailedCopyDoesNotLeavePartialFile() throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: folder) }
        XCTAssertThrowsError(try BackgroundFileService.copyCoordinated(folder.appendingPathComponent("missing.flac"), into: folder))
        let leftovers = (try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? []
        XCTAssertTrue(leftovers.isEmpty)
    }
}
