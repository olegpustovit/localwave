import Testing
import Foundation
@testable import localwave

struct MotionTokenTests {
    @Test func scheduledMechanicalStepsCompleteDespiteDatePrecision() {
        let start = Date(timeIntervalSinceReferenceDate: 815_000_000.123456)
        for row in 0..<7 {
            let delay = Double(row) * 0.016
            for (offset, expected) in [(0.07, 0.5), (0.14, 1.0)] {
                let age = start.addingTimeInterval(delay + offset).timeIntervalSince(start) - delay
                #expect(LWMotion.steppedProgress(age, duration: 0.14, count: 2) == expected)
            }
        }
        for step in 0...12 {
            let age = start.addingTimeInterval(Double(step) * 1.6 / 12).timeIntervalSince(start)
            #expect(abs(LWMotion.steppedProgress(age, duration: 1.6, count: 12) - Double(step) / 12) < 0.00000001)
        }
    }
    @Test func outCurveMatchesReferenceBrowserTransformSamples() {
        // Chromium DOMMatrix samples of the artifact's computed --e-out animation.
        let samples: [(Double, Double)] = [(0, 0), (0.1, 0.494391), (0.25, 0.825622), (0.5, 0.971779), (0.75, 0.997677), (1, 1)]
        for (time, progress) in samples {
            #expect(abs(LWMotion.outProgress(time) - progress) < 0.00001)
        }
    }
    @Test func springMatchesReferenceBrowserLinearEasingSamples() {
        let samples: [(Double, Double)] = [(0, 0), (0.0105, 0.009), (0.021, 0.035), (0.044, 0.141), (0.129, 0.723), (0.167, 0.938), (0.205, 1.017), (0.245, 1.043), (0.284, 1.035), (0.377, 1.01), (0.472, 0.997), (0.75, 0.99858), (1, 1)]
        for (time, progress) in samples {
            #expect(abs(LWMotion.springProgress(time) - progress) < 0.000001)
        }
    }
}
