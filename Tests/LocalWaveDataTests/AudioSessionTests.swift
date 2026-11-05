import Foundation
import XCTest
@testable import LocalWaveCore

final class AudioSessionTests: XCTestCase {
    func testConfigurationAndActivationAreSerializedOffMainAndInterruptionReactivates() async throws {
        let calls = SessionCallProbe()
        let controller = AudioSessionController(configure: { calls.configure() }, activate: { calls.activate() })
        try await withThrowingTaskGroup(of: Void.self) { group in
            for _ in 0..<8 { group.addTask { try await controller.activate() } }
            try await group.waitForAll()
        }
        XCTAssertEqual(calls.snapshot.configured, 1)
        XCTAssertEqual(calls.snapshot.activated, 1)
        XCTAssertFalse(calls.snapshot.ranOnMain)
        let stream = await controller.events()
        await controller.receive(.interruptionBegan)
        var iterator = stream.makeAsyncIterator()
        let event = await iterator.next()
        XCTAssertEqual(event, .interruptionBegan)
        try await controller.activate()
        XCTAssertEqual(calls.snapshot.configured, 1)
        XCTAssertEqual(calls.snapshot.activated, 2)
    }
}

private final class SessionCallProbe: @unchecked Sendable {
    private let lock = NSLock()
    private var configured = 0
    private var activated = 0
    private var ranOnMain = false
    var snapshot: (configured: Int, activated: Int, ranOnMain: Bool) {
        lock.lock()
        defer { lock.unlock() }
        return (configured, activated, ranOnMain)
    }
    func configure() {
        lock.lock()
        defer { lock.unlock() }
        configured += 1
        ranOnMain = ranOnMain || Thread.isMainThread
    }
    func activate() {
        lock.lock()
        defer { lock.unlock() }
        activated += 1
        ranOnMain = ranOnMain || Thread.isMainThread
    }
}
