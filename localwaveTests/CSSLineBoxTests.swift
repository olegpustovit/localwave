import CoreText
import SwiftUI
import Testing
import UIKit
@testable import localwave

struct CSSLineBoxTests {
    /// Measured in the actual reference with zero-height inline baseline markers.
    /// Columns: font, font size, CSS line-height, measured baseline from line top.
    private let browserFixtures: [(String, CGFloat, CGFloat, CGFloat)] = [
        ("HostGrotesk-Regular", 11, 15.4, 11),
        ("HostGrotesk-Regular", 12, 16.8, 12),
        ("HostGrotesk-Regular", 13, 18.2, 13),
        ("HostGrotesk-Regular", 13, 19.5, 14),
        ("HostGrotesk-Regular", 13, 21, 15),
        ("HostGrotesk-Regular", 14, 17.5, 13),
        ("HostGrotesk-Regular", 14, 19.6, 14),
        ("HostGrotesk-Regular", 15, 21, 15),
        ("HostGrotesk-Regular", 17, 23.8, 17),
        ("HostGrotesk-Regular", 18, 25.2, 18),
        ("HostGrotesk-Regular", 19, 26.6, 19),
        ("HostGrotesk-Regular", 20, 28, 21),
        ("HostGrotesk-Regular", 24, 27.6, 21),
        ("HostGrotesk-Regular", 26, 36.4, 27),
        ("HostGrotesk-Regular", 32, 33.6, 27),
        ("HostGrotesk-Regular", 34, 34, 29),
        ("GeistMono-Regular", 11, 15.4, 11),
        ("GeistMono-Regular", 12, 16.8, 12),
        ("GeistMono-Regular", 13, 18.2, 13)
    ]

    @MainActor @Test func customFontScalingMatchesNativeRenderedWidths() throws {
        let categories: [(DynamicTypeSize, UIContentSizeCategory)] = [
            (.xSmall, .extraSmall), (.large, .large), (.xxxLarge, .extraExtraExtraLarge),
            (.accessibility3, .accessibilityExtraLarge), (.accessibility5, .accessibilityExtraExtraExtraLarge)
        ]
        var evidence: [[String: Any]] = []
        for data in [false, true] {
            let face = data ? "GeistMono-Regular" : "HostGrotesk-Regular"
            let style: UIFont.TextStyle = data ? .caption1 : .body
            for size: CGFloat in [11, 13, 32] {
                let base = try #require(UIFont(name: face, size: size))
                for (dynamicType, category) in categories {
                    let traits = UITraitCollection(preferredContentSizeCategory: category)
                    let metrics = UIFontMetrics(forTextStyle: style)
                    let scaled = metrics.scaledFont(for: base, compatibleWith: traits)
                    let line = CTLineCreateWithAttributedString(NSAttributedString(string: "MW", attributes: [.font: scaled]))
                    let expectedWidth = CGFloat(CTLineGetTypographicBounds(line, nil, nil, nil))
                    let native = try measure(Text("MW").font(data ? LWTheme.data(size) : LWTheme.text(size)), dynamicType: dynamicType)
                    #expect(abs(native.width - expectedWidth) < 1, "Native Font.custom must match the scaled font, including point-size rounding")
                    evidence.append(["face": face, "base": size, "category": category.rawValue,
                        "scaledFontSize": scaled.pointSize, "unitTimesBase": metrics.scaledValue(for: 1, compatibleWith: traits) * size,
                        "nativeWidth": native.width, "scaledFontWidth": expectedWidth])
                }
            }
        }
        let json = try JSONSerialization.data(withJSONObject: evidence, options: [.sortedKeys])
        print("LW_FONT_SCALING_PROOF " + String(decoding: json, as: UTF8.self))
    }

    @MainActor @Test func lineBoxesFollowActualScaledFontsAcrossBodyAndCaption() throws {
        let categories: [(DynamicTypeSize, UIContentSizeCategory)] = [
            (.xSmall, .extraSmall), (.large, .large), (.xxxLarge, .extraExtraExtraLarge),
            (.accessibility3, .accessibilityExtraLarge), (.accessibility5, .accessibilityExtraExtraExtraLarge)
        ]
        for data in [false, true] {
            let face = data ? "GeistMono-Regular" : "HostGrotesk-Regular"
            for size: CGFloat in [11, 13, 32] {
                let base = try #require(UIFont(name: face, size: size))
                for (dynamicType, category) in categories {
                    let font = UIFontMetrics(forTextStyle: data ? .caption1 : .body).scaledFont(for: base,
                        compatibleWith: UITraitCollection(preferredContentSizeCategory: category))
                    let lineHeight = font.pointSize * 7 / 5
                    let ascent = font.ascender.rounded()
                    let descent = (-font.descender).rounded()
                    // Independent rational oracle: 1.4 =7/5, and scaled font
                    // sizes/rounded font-box metrics here are integral points.
                    #expect(font.pointSize == font.pointSize.rounded())
                    let expectedBaseline = ascent + floor((7 * font.pointSize - 5 * (ascent + descent)) / 10)
                    let sample = try measure(Text("A\nB\nC").font(data ? LWTheme.data(size) : LWTheme.text(size))
                        .lwLineBox(size, size * 1.4, data: data), dynamicType: dynamicType)
                    #expect(abs(sample.first - expectedBaseline) < 0.001, "\(face) \(size) \(category.rawValue)")
                    #expect(abs(sample.last - (expectedBaseline + 2 * lineHeight)) < 0.001, "\(face) \(size) \(category.rawValue)")
                    #expect(abs(sample.height - 3 * lineHeight) < 0.001)
                }
            }
        }
    }

    @Test func halfLeadingSnapsOnlyFloatingPointRoundoff() {
        // GeistMono11 at AX5 is35pt: ascent35/descent10, CSS49pt line.
        // The font-size ratio produces48.999999999999993; mathematical B=37.
        #expect(LWCSSFontLineMetrics(ascent: 35, descent: 10, lineHeight: 49.nextDown).baseline == 37)
        #expect(LWCSSFontLineMetrics(ascent: 35, descent: 10, lineHeight: 49 - 0.00000001).baseline == 36)
        // Tight line boxes also need a stable floor for negative half-leading.
        #expect(LWCSSFontLineMetrics(ascent: 20, descent: 5, lineHeight: 23.nextDown).baseline == 19)
        #expect(LWCSSFontLineMetrics(ascent: 20, descent: 5, lineHeight: 23 - 0.00000001).baseline == 18)
    }

    @MainActor @Test func nativeTextMatchesNineteenMeasuredBrowserBaselines() throws {
        for (face, size, lineHeight, expectedBaseline) in browserFixtures {
            let font = try #require(UIFont(name: face, size: size))
            #expect(font.leading == 0)
            let sample = try measure(Text("Halden Marsh").font(.custom(face, fixedSize: size))
                .lwLineBox(size, lineHeight, data: face.hasPrefix("Geist")))
            #expect(abs(sample.first - expectedBaseline) < 0.001)
            #expect(abs(sample.last - expectedBaseline) < 0.001)
            #expect(abs(sample.height - lineHeight) < 0.001)
        }
    }

    @MainActor @Test func multilineUsesCssAdvanceInsteadOfNativePixelRoundedLeading() throws {
        let sample = try measure(Text("First line\nSecond line\nThird line")
            .font(LWTheme.text(14)).lwLineBox(14, 19.6))
        // Browser baseline markers: 14, 33.59375, 53.1875; DOM quantizes to 1/64 pt.
        #expect(abs(sample.first - 14) < 0.001)
        #expect(abs(sample.last - 53.1875) <= 1.0 / 64)
        #expect(abs(sample.height - 58.78125) <= 1.0 / 32)
    }

    @MainActor @Test func accessibilityTypeScalesTheLineBoxAndRetainsAllThreeLines() throws {
        let text = Text("First line\nSecond line\nThird line").font(LWTheme.text(13)).lwLineBox(13, 18.2)
        let normal = try measure(text)
        let accessible = try measure(text, dynamicType: .accessibility3)
        #expect(accessible.height.isFinite)
        #expect(accessible.height > normal.height * 1.5)
        #expect(accessible.first > normal.first)
        #expect(accessible.last > accessible.first)
        #expect(abs((accessible.last - accessible.first) - accessible.height * 2 / 3) < 0.001)
    }

    @MainActor @Test func attributedSearchHighlightSurvivesCustomLineRendering() throws {
        var highlighted = AttributedString("Field Day")
        let match = try #require(highlighted.range(of: "Field"))
        highlighted[match].backgroundColor = Color(red: 1, green: 1, blue: 0)
        let renderer = ImageRenderer(content: Text(highlighted).font(LWTheme.text(14))
            .foregroundStyle(Color.black).lwLineBox(14, 19.6).fixedSize())
        renderer.scale = 3
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
        let yellowPixels = stride(from: 0, to: pixels.count, by: 4).filter { offset -> Bool in
            let red = pixels[offset], green = pixels[offset + 1], blue = pixels[offset + 2], alpha = pixels[offset + 3]
            return red > 150 && green > 150 && blue < 30 && alpha > 200
        }.count
        #expect(yellowPixels > 20, "Search highlights must remain visible through the custom text renderer")
    }

    @MainActor @Test func searchHighlightUsesMeasuredOneAndAHalfPointBottomRule() throws {
        let signal = Color(red: 35.0 / 255, green: 64.0 / 255, blue: 1)
        let content = Text("Night").customAttribute(LWSearchMark(color: signal)).font(LWTheme.text(15))
            .foregroundStyle(Color.black).lwLineBox(15, 21).fixedSize().padding(16).background(Color.white).environment(\.dynamicTypeSize, .large)
        let renderer = ImageRenderer(content: content)
        renderer.scale = 3
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
        // The DOM mark is 20pt tall: baseline15 plus5 descent; its rule spans18.5...20.
        // Sample inside the first character, below its glyph, where only the rule can be blue.
        let x = 17 * 3
        var opaqueRows: [Int] = []
        for y in 0..<image.height {
            let index = (y * image.width + x) * 4
            if pixels[index] < 80 && pixels[index + 1] < 100 && pixels[index + 2] > 245 && pixels[index + 3] > 245 { opaqueRows.append(y) }
        }
        #expect((4...5).contains(opaqueRows.count), "A 1.5pt rule occupies4.5pixels at3x, including its antialiased edge")
        let topOriginMatches = opaqueRows.allSatisfy { (103...108).contains($0) }
        let bottomOriginMatches = opaqueRows.allSatisfy { (103...108).contains(image.height - 1 - $0) }
        #expect(topOriginMatches || bottomOriginMatches, "The rule belongs at the mark bottom, not the font underline position")
    }

    @MainActor private func measure<V: View>(_ content: V, dynamicType: DynamicTypeSize = .large) throws -> BaselineSample {
        let recorder = BaselineRecorder()
        let host = UIHostingController(rootView: BaselineCapture(recorder: recorder) { content }
            .environment(\.dynamicTypeSize, dynamicType))
        host.safeAreaRegions = []
        _ = host.sizeThatFits(in: CGSize(width: 390, height: 2000))
        return try #require(recorder.sample)
    }
}

private struct BaselineSample {
    let width: CGFloat
    let height: CGFloat
    let first: CGFloat
    let last: CGFloat
}
private final class BaselineRecorder {
    var sample: BaselineSample?
}
private struct BaselineCapture: Layout {
    let recorder: BaselineRecorder
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        guard let child = subviews.first else { return .zero }
        let dimensions = child.dimensions(in: proposal)
        recorder.sample = BaselineSample(width: dimensions.width, height: dimensions.height, first: dimensions[.firstTextBaseline], last: dimensions[.lastTextBaseline])
        return CGSize(width: dimensions.width, height: dimensions.height)
    }
    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        subviews.first?.place(at: bounds.origin, anchor: .topLeading, proposal: proposal)
    }
}
