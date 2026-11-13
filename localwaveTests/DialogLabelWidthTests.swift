#if canImport(UIKit)
import SwiftUI
import Testing
import UIKit
@testable import localwave

struct DialogLabelWidthTests {
    /// Actual Chromium DOM max-content widths at13px, not native rounded Text.
    private let browser: [(String, Bool, CGFloat)] = [
        ("Cancel", false, 40.625), ("Create", true, 39.171875),
        ("Save", true, 27.703125), ("Delete List", true, 61.75)
    ]

    @MainActor @Test func dialogLabelsMatchFourBrowserIntrinsicWidths() throws {
        for (title, medium, expected) in browser {
            let measured = try #require(LWDialogLabelMetrics.width(for: title, pointSize: 13, medium: medium))
            #expect(abs(measured - expected) < 0.001)
            for scale in [CGFloat(1), 2, 3] {
                let size = measure(label(title, medium: medium).environment(\.displayScale, scale))
                #expect(abs(size.width - (expected + 28)) < 0.001)
                #expect(size.height == 30)
            }
        }
    }

    @MainActor @Test func desktopNewListButtonsRetainMeasuredRightAlignedGeometry() {
        let size = measure(HStack(spacing: 8) {
            label("Cancel", medium: false)
            label("Create", medium: true)
        })
        // Browser: Cancel x766.203125..834.828125,8gap,Create..910.
        #expect(abs(size.width - 143.796875) < 0.001)
        #expect(abs((910 - size.width) - 766.203125) < 0.001)
    }

    @MainActor @Test func largeTypeUsesTheSameScaledFontAndRetainsTheNativeLabel() throws {
        let normal = measure(label("Delete List", medium: true))
        let large = measure(label("Delete List", medium: true), dynamicType: .accessibility3)
        let metrics = UIFontMetrics(forTextStyle: .body)
        let traits = UITraitCollection(preferredContentSizeCategory: .accessibilityExtraLarge)
        let baseFont = try #require(UIFont(name: "HostGrotesk-Medium", size: 13))
        let scaledFont = metrics.scaledFont(for: baseFont, compatibleWith: traits)
        let expected = try #require(LWDialogLabelMetrics.width(for: "Delete List", pointSize: scaledFont.pointSize, medium: true))
        let original = measure(Text("Delete List").font(LWTheme.text(13).weight(.medium)), dynamicType: .accessibility3)
        let fixed = measure(Text("Delete List").font(.custom("HostGrotesk-Regular", fixedSize: scaledFont.pointSize).weight(.medium)))
        // Measured native Font.custom13/body resolves to28pt at accessibility3;
        // scaledValue13 yields28.3333 and scaling a unit yields30.3333 instead.
        #expect(scaledFont.pointSize == 28)
        #expect(expected == 132.984375)
        #expect(original == fixed)
        #expect(large.width > normal.width * 1.5)
        #expect(abs(large.width - (expected + 28)) < 0.001)
        #expect(large.height > normal.height)
    }

    private func label(_ title: String, medium: Bool) -> some View {
        Text(title).font(LWTheme.text(13).weight(medium ? .medium : .regular))
            .lwLineBox(13, 18.2)
            .modifier(LWDialogLabelWidth(title: title, medium: medium, enabled: true))
            .padding(.horizontal, 14).frame(minHeight: 30)
    }

    @MainActor private func measure<V: View>(_ view: V, dynamicType: DynamicTypeSize = .large) -> CGSize {
        let recorder = DialogLabelSizeRecorder()
        let host = UIHostingController(rootView: DialogLabelSizeCapture(recorder: recorder) {
            view.environment(\.dynamicTypeSize, dynamicType)
        })
        host.safeAreaRegions = []
        _ = host.sizeThatFits(in: CGSize(width: 1000, height: 1000))
        return recorder.size
    }
}

private final class DialogLabelSizeRecorder { var size: CGSize = .zero }
private struct DialogLabelSizeCapture: Layout {
    let recorder: DialogLabelSizeRecorder
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        recorder.size = subviews.first?.sizeThatFits(.unspecified) ?? .zero
        return recorder.size
    }
    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        subviews.first?.place(at: bounds.origin, anchor: .topLeading, proposal: .unspecified)
    }
}

#endif
