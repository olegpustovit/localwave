import SwiftUI
import Testing
import UIKit
@testable import localwave

struct CollapsedTableRulesTests {
    @MainActor @Test func collapsedBordersMatchBrowserRastersAtAllThreeDisplayScales() throws {
        // Actual reference table captures at DPR1/2/3. Integer offsets300/400/200
        // remove unrelated content above Album/Artist/List without changing phase.
        let fixtures: [(CGFloat, [[Int]])] = [
            (49.296875, [[49, 87, 125, 163], [98, 174, 250, 326], [147, 261, 375, 489]]),
            (84.765625, [[84, 122, 160, 198], [168, 244, 320, 396], [252, 366, 480, 594]]),
            (12.078125, [[12, 50, 88, 126], [24, 100, 176, 252], [36, 150, 264, 378]])
        ]
        for (origin, browserStarts) in fixtures {
            for scale in 1...3 {
                let content = BorderFixture(origin: origin).environment(\.displayScale, CGFloat(scale))
                let renderer = ImageRenderer(content: content)
                renderer.scale = CGFloat(scale)
                let image = try #require(renderer.uiImage?.cgImage)
                var pixels = [UInt8](repeating: 0, count: image.width * image.height * 4)
                let drawn = pixels.withUnsafeMutableBytes { buffer -> Bool in
                    guard let context = CGContext(data: buffer.baseAddress, width: image.width, height: image.height,
                        bitsPerComponent: 8, bytesPerRow: image.width * 4, space: CGColorSpaceCreateDeviceRGB(),
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue) else { return false }
                    context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
                    return true
                }
                #expect(drawn)
                let actual = (0..<image.height).filter { y in
                    [2, image.width / 2, image.width - 3].allSatisfy { x in
                        let index = (y * image.width + x) * 4
                        return pixels[index] == 218 && pixels[index + 1] == 218 && pixels[index + 2] == 218 && pixels[index + 3] == 255
                    }
                }
                let expected = browserStarts[scale - 1].flatMap { start in Array(start..<(start + scale)) }
                #expect(actual == expected, "Collapsed rules must match the browser without moving rows or splitting into antialiased gray bands")
            }
        }
    }
}

private struct BorderFixture: View {
    let origin: CGFloat
    private let paper = Color(red: 243.0 / 255, green: 243.0 / 255, blue: 243.0 / 255)
    private let line = Color(red: 218.0 / 255, green: 218.0 / 255, blue: 218.0 / 255)

    var body: some View {
        VStack(spacing: 0) {
            Color.clear.frame(height: origin)
            VStack(spacing: 0) {
                ForEach(0..<3) { index in
                    Rectangle().fill(paper).frame(height: 38)
                        .modifier(LWCollapsedTableRules(top: true, bottom: index == 2, color: line))
                }
            }
            Spacer(minLength: 0)
        }.frame(width: 100, height: 240).background(paper)
    }
}
