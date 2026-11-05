import SwiftUI
import XCTest
@testable import localwave

@MainActor
final class ReducedMotionTransactionTests: XCTestCase {
    func testReduceMotionSuppressesExplicitAnimationsWithAnAnimatedControl() async throws {
        try XCTSkipIf(UIAccessibility.isReduceMotionEnabled, "The animated control requires system Reduce Motion to be off")
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        for reduced in [false, true] {
            let suiteName = "ReducedMotionTransactionTests-\(UUID().uuidString)"
            let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
            defaults.set(reduced, forKey: "lwReduceMotion")
            defer { defaults.removePersistentDomain(forName: suiteName) }
            let model = Position()
            let samples = Samples()
            let controller = UIHostingController(rootView:
                Probe(model: model, samples: samples)
                    .modifier(ThemeProvider())
                    .defaultAppStorage(defaults)
            )
            let window = UIWindow(windowScene: scene)
            window.frame = CGRect(x: 0, y: 0, width: 240, height: 120)
            window.rootViewController = controller
            window.isHidden = false
            defer { window.isHidden = true; window.rootViewController = nil }
            try await Task.sleep(for: .milliseconds(60))
            samples.values = []
            withAnimation(LWMotion.out(0.24)) { model.value = 100 }
            try await Task.sleep(for: .milliseconds(340))
            XCTAssertTrue(samples.values.contains { abs($0 - 100) < 0.001 })
            let intermediate = samples.values.contains { $0 > 0.001 && $0 < 99.999 }
            XCTAssertEqual(intermediate, !reduced, "The enabled control must animate; reduced motion must settle immediately")
        }
    }
}

@MainActor private final class Position: ObservableObject { @Published var value: CGFloat = 0 }
@MainActor private final class Samples { var values: [CGFloat] = [] }

private struct Probe: View {
    @ObservedObject var model: Position
    let samples: Samples
    var body: some View {
        Color.black.frame(width: 10, height: 10)
            .modifier(SamplingOffset(value: model.value, samples: samples))
    }
}

private struct SamplingOffset: AnimatableModifier {
    var value: CGFloat
    let samples: Samples
    var animatableData: CGFloat { get { value } set { value = newValue } }
    func body(content: Content) -> some View {
        samples.values.append(value)
        return content.offset(x: value)
    }
}
