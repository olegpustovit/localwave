import Foundation
import XCTest
@testable import LocalWaveData

final class SourceFolderPresenterTests: XCTestCase {
    func testCoordinatedChildWriteNotifiesAndInvalidationStopsDelivery() async throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let appeared = expectation(description: "Coordinated child addition reaches the source observer")
        appeared.assertForOverFulfill = false
        let stopped = expectation(description: "No source callback after invalidation")
        stopped.isInverted = true
        let state = PresenterTestState()
        let presenter = SourceFolderPresenter(url: folder) { movedRoot, _ in
            XCTAssertNil(movedRoot)
            if state.isInvalidated { stopped.fulfill() } else { appeared.fulfill() }
        }
        defer { presenter.invalidate() }
        let first = folder.appendingPathComponent("first.flac")
        try await Task.detached { try Self.write(Data([1]), to: first) }.value
        await fulfillment(of: [appeared], timeout: 5)
        XCTAssertEqual(presenter.presentedItemURL?.standardizedFileURL, folder.standardizedFileURL)

        let pending = folder.appendingPathComponent("pending.flac")
        try await Task.detached { try Self.write(Data([2]), to: pending) }.value
        presenter.invalidate()
        state.invalidate()
        let later = folder.appendingPathComponent("later.flac")
        try await Task.detached { try Self.write(Data([3]), to: later) }.value
        await fulfillment(of: [stopped], timeout: 1.2)
    }

    func testCoordinatedRootMoveUpdatesGrantLocationBeforeCallback() async throws {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        let original = base.appendingPathComponent("Before", isDirectory: true)
        let renamed = base.appendingPathComponent("After", isDirectory: true)
        try FileManager.default.createDirectory(at: original, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: base) }
        let moved = expectation(description: "Renamed source root is delivered")
        let presenter = SourceFolderPresenter(url: original) { newURL, _ in
            guard let newURL else { return }
            XCTAssertEqual(newURL.resolvingSymlinksInPath(), renamed.resolvingSymlinksInPath())
            moved.fulfill()
        }
        defer { presenter.invalidate() }
        try await Task.detached {
            let coordinator = NSFileCoordinator(filePresenter: nil)
            var coordinationError: NSError?
            var moveError: Error?
            coordinator.coordinate(writingItemAt: original, options: .forMoving, writingItemAt: renamed, options: .forReplacing, error: &coordinationError) { oldURL, newURL in
                do {
                    coordinator.item(at: oldURL, willMoveTo: newURL)
                    try FileManager.default.moveItem(at: oldURL, to: newURL)
                    coordinator.item(at: oldURL, didMoveTo: newURL)
                } catch { moveError = error }
            }
            if let coordinationError { throw coordinationError }
            if let moveError { throw moveError }
            try Self.write(Data([4]), to: renamed.appendingPathComponent("after.flac"))
        }.value
        await fulfillment(of: [moved], timeout: 5)
        XCTAssertEqual(presenter.presentedItemURL?.resolvingSymlinksInPath(), renamed.resolvingSymlinksInPath())
    }

    private static func write(_ data: Data, to url: URL) throws {
        let coordinator = NSFileCoordinator(filePresenter: nil)
        var coordinationError: NSError?
        var writeError: Error?
        coordinator.coordinate(writingItemAt: url, options: .forReplacing, error: &coordinationError) { coordinatedURL in
            do { try data.write(to: coordinatedURL) }
            catch { writeError = error }
        }
        if let coordinationError { throw coordinationError }
        if let writeError { throw writeError }
    }
}

private final class PresenterTestState: @unchecked Sendable {
    private let lock = NSLock()
    private var invalidated = false
    var isInvalidated: Bool { lock.lock(); defer { lock.unlock() }; return invalidated }
    func invalidate() { lock.lock(); invalidated = true; lock.unlock() }
}
