import SwiftUI
import UIKit
import XCTest
@testable import localwave

@MainActor
final class SourceScanBarTests: XCTestCase {
    func testMeasuredQuarterProgressIsSquareAndExactlyThirtyPixelsWide() throws {
        let renderer = ImageRenderer(content: WaveSourceScanBar(progress: 0.25).frame(width: 120).environment(\.colorScheme, .light))
        renderer.scale = 1
        let image = try XCTUnwrap(renderer.uiImage?.cgImage)
        XCTAssertEqual(image.width, 120)
        XCTAssertEqual(image.height, 4)
        let pixels = try pixelData(image)
        func pixel(_ x: Int, _ y: Int) -> [UInt8] { Array(pixels[(y * 120 + x) * 4..<(y * 120 + x) * 4 + 4]) }
        XCTAssertNotEqual(pixel(29, 2), pixel(30, 2))
        for y in 0..<4 {
            XCTAssertEqual(pixel(0, y), pixel(29, y), "Progress corners must remain square")
            XCTAssertEqual(pixel(30, y), pixel(119, y))
        }
    }

    private func pixelData(_ image: CGImage) throws -> [UInt8] {
        var bytes = [UInt8](repeating: 0, count: image.width * image.height * 4)
        let drew = bytes.withUnsafeMutableBytes { data -> Bool in
            guard let context = CGContext(data: data.baseAddress, width: image.width, height: image.height, bitsPerComponent: 8, bytesPerRow: image.width * 4, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return false }
            context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
            return true
        }
        XCTAssertTrue(drew)
        return bytes
    }
}
