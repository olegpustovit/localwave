import SwiftUI
import UIKit
import XCTest
@testable import localwave

@MainActor
final class MatrixRenderingTests: XCTestCase {
    func testChangedCountSettlesEveryRowToFullInk() async throws {
        final class Count: ObservableObject { @Published var text = "0" }
        struct Display: View {
            @ObservedObject var count: Count
            var body: some View {
                MatrixText(text: count.text, height: 70)
                    .environment(\.dynamicTypeSize, .large)
                    .environment(\.colorScheme, .light)
                    .environment(\.lwReduceMotion, false)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .background(LWTheme.paper)
            }
        }
        let count = Count()
        let controller = UIHostingController(rootView: Display(count: count))
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        let window = UIWindow(windowScene: scene)
        window.frame = scene.coordinateSpace.bounds
        window.rootViewController = controller
        window.isHidden = false
        defer { window.isHidden = true }
        controller.safeAreaRegions = []
        window.layoutIfNeeded()
        try await Task.sleep(for: .milliseconds(100))
        count.text = "92"
        try await Task.sleep(for: .seconds(1))
        let image = UIGraphicsImageRenderer(bounds: controller.view.bounds).image { _ in
            controller.view.drawHierarchy(in: controller.view.bounds, afterScreenUpdates: true)
        }
        let cgImage = try XCTUnwrap(image.cgImage)
        var pixels = [UInt8](repeating: 0, count: cgImage.width * cgImage.height * 4)
        try pixels.withUnsafeMutableBytes { bytes in
            let context = try XCTUnwrap(CGContext(data: bytes.baseAddress, width: cgImage.width, height: cgImage.height, bitsPerComponent: 8, bytesPerRow: cgImage.width * 4, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
            context.draw(cgImage, in: CGRect(x: 0, y: 0, width: cgImage.width, height: cgImage.height))
        }
        let scale = image.scale
        for point in [CGPoint(x: 15, y: 5), CGPoint(x: 15, y: 65), CGPoint(x: 65, y: 65)] {
            let index = (Int(point.y * scale) * cgImage.width + Int(point.x * scale)) * 4
            XCTAssertLessThan(pixels[index], 40, "Changed matrix row at \(point) remains faded after its final frame")
        }
    }

    func testClockRetainsItsIntrinsicSVGSizeBesideRemainingTime() throws {
        let renderer = ImageRenderer(content: HStack(alignment: .bottom) {
            MatrixText(text: "01:41", height: 43)
            Spacer(minLength: 4)
            Text("-3:44").font(LWTheme.data()).lwLineBox(12, 16.8, data: true)
        }.frame(width: 279).background(LWTheme.paper)
            .environment(\.dynamicTypeSize, .large)
            .environment(\.colorScheme, .light)
            .environment(\.lwReduceMotion, true))
        renderer.scale = 1
        let image = try XCTUnwrap(renderer.uiImage)
        XCTAssertEqual(image.size.width, 279, accuracy: 0.01)
        XCTAssertEqual(image.size.height, 43, accuracy: 0.01)
        let cgImage = try XCTUnwrap(image.cgImage)
        var pixels = [UInt8](repeating: 0, count: cgImage.width * cgImage.height * 4)
        try pixels.withUnsafeMutableBytes { bytes in
            let context = try XCTUnwrap(CGContext(data: bytes.baseAddress, width: cgImage.width, height: cgImage.height, bitsPerComponent: 8, bytesPerRow: cgImage.width * 4, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
            context.draw(cgImage, in: CGRect(x: 0, y: 0, width: cgImage.width, height: cgImage.height))
        }
        let ink = (0..<cgImage.height).reduce(0) { total, y in
            total + (0..<152).filter { x in
                let index = (y * cgImage.width + x) * 4
                return pixels[index] < 100 && pixels[index + 1] < 100 && pixels[index + 2] < 100
            }.count
        }
        XCTAssertGreaterThan(ink, 500, "The display has a layout box but no visible matrix pixels.")
    }

    func testAccessibilityDisplayFitsTheAvailableWidth() throws {
        let renderer = ImageRenderer(content: MatrixText(text: "01:41", height: 72)
            .frame(width: 180)
            .environment(\.dynamicTypeSize, .accessibility5)
            .environment(\.lwReduceMotion, true))
        renderer.scale = 1
        let image = try XCTUnwrap(renderer.uiImage)
        XCTAssertEqual(image.size.width, 180, accuracy: 0.01)
        XCTAssertEqual(image.size.height, 180 * 7 / 24.78, accuracy: 0.5)
    }
}
