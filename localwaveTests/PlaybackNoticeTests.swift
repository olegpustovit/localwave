import SwiftUI
import UIKit
import XCTest
@testable import localwave

@MainActor
final class PlaybackNoticeTests: XCTestCase {
    func testReplacingAnIdenticalNoticeCancelsTheOldExpiry() async throws {
        struct Event { let id = UUID(); let text = "Skipped an unavailable song" }
        final class Model: ObservableObject {
            @Published var event = Event()
            var expired: [UUID] = []
        }
        struct Surface: View {
            @ObservedObject var model: Model
            var body: some View {
                let event = model.event
                LWPlaybackNotice(message: event.text) { model.expired.append(event.id) }
                    .id(event.id).environment(\.lwReduceMotion, true).environment(\.lwNoticeLifetimeScale, 0.3)
            }
        }
        let model = Model()
        let controller = UIHostingController(rootView: Surface(model: model))
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        let window = UIWindow(windowScene: scene)
        window.frame = scene.coordinateSpace.bounds
        window.rootViewController = controller
        window.isHidden = false
        defer { window.isHidden = true }
        window.layoutIfNeeded()
        try await Task.sleep(for: .milliseconds(360))
        let replacement = Event()
        model.event = replacement
        try await Task.sleep(for: .milliseconds(330))
        XCTAssertTrue(model.expired.isEmpty, "The first notice must not dismiss its identical replacement")
        try await Task.sleep(for: .milliseconds(390))
        XCTAssertEqual(model.expired, [replacement.id])
    }
}

extension PlaybackNoticeTests {
    func testReplacingAnIdenticalUndoNoticeCancelsItsOriginalFourSecondExpiry() async throws {
        struct Event { let id = UUID(); let text = "Removed 1 from “Late drive”" }
        final class Model: ObservableObject {
            @Published var event = Event()
            var expired: [UUID] = []
        }
        struct Surface: View {
            @ObservedObject var model: Model
            var body: some View {
                let event = model.event
                LWPlaybackNotice(message: event.text, actionTitle: "Undo", action: {},
                    onExpiry: { model.expired.append(event.id) })
                    .id(event.id).environment(\.lwReduceMotion, true).environment(\.lwNoticeLifetimeScale, 0.3)
            }
        }
        let model = Model()
        let (_, window) = try hostNotice(Surface(model: model))
        defer { window.isHidden = true }
        try await Task.sleep(for: .milliseconds(360))
        let replacement = Event()
        model.event = replacement
        try await Task.sleep(for: .milliseconds(930))
        XCTAssertTrue(model.expired.isEmpty, "The original4second deadline must not remove its identical replacement")
        try await Task.sleep(for: .milliseconds(360))
        XCTAssertEqual(model.expired, [replacement.id])
    }

    func testUndoNoticeOutlivesNormalNoticeAndExpiresAfterFourSeconds() async throws {
        let expired = expectation(description: "Undo lifetime")
        expired.assertForOverFulfill = true
        var expiredAt: Date?
        let began = Date()
        let (_, window) = try hostNotice(LWPlaybackNotice(message: "Removed 1 from “Late drive”",
            actionTitle: "Undo", action: {}, onExpiry: { expiredAt = Date(); expired.fulfill() })
            .environment(\.lwReduceMotion, true).environment(\.lwNoticeLifetimeScale, 0.3))
        defer { window.isHidden = true }
        try await Task.sleep(for: .milliseconds(660))
        XCTAssertNil(expiredAt, "Undo must outlive a normal2second notice")
        await fulfillment(of: [expired], timeout: 2)
        let elapsed = try XCTUnwrap(expiredAt).timeIntervalSince(began)
        XCTAssertGreaterThanOrEqual(elapsed, 1.18, "Undo has the reference4second lifetime")
    }

    private func hostNotice<V: View>(_ view: V) throws -> (UIHostingController<V>, UIWindow) {
        let controller = UIHostingController(rootView: view)
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        let window = UIWindow(windowScene: scene)
        window.frame = scene.coordinateSpace.bounds
        window.rootViewController = controller
        window.isHidden = false
        window.layoutIfNeeded()
        return (controller, window)
    }

}
