import SwiftUI

/// Chromium rounds font-box metrics to CSS pixels, then floors the half-leading.
/// The fixtures in CSSLineBoxTests come from zero-height inline baseline markers.
struct LWCSSFontLineMetrics {
    let ascent: CGFloat
    let descent: CGFloat
    let lineHeight: CGFloat

    var baseline: CGFloat {
        let a = ascent.rounded()
        let d = descent.rounded()
        let halfLeading = (lineHeight - a - d) / 2
        // Ratio-based Dynamic Type scaling can place a mathematically integral
        // half-leading a few floating-point ULPs below that integer. Snap only
        // arithmetic roundoff, preserving genuinely fractional CSS leading.
        let nearest = halfLeading.rounded()
        let tolerance = 4 * max(lineHeight.ulp, max(a.ulp, d.ulp))
        let stableLeading = abs(halfLeading - nearest) <= tolerance ? nearest : halfLeading
        return a + floor(stableLeading)
    }
}

struct LWCSSLineBoxLayout: Layout {
    let metrics: LWCSSFontLineMetrics

    private func lineCount(_ dimensions: ViewDimensions) -> Int {
        let span = dimensions[.lastTextBaseline] - dimensions[.firstTextBaseline]
        // Read SwiftUI's actual pixel-rounded native line box, rather than assuming
        // UIFont.lineHeight or a display scale. Native text keeps zero line spacing.
        let nativeLineBox = max(1, dimensions.height - span)
        return max(1, Int((span / nativeLineBox).rounded()) + 1)
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        guard let text = subviews.first else { return .zero }
        let dimensions = text.dimensions(in: ProposedViewSize(width: proposal.width, height: nil))
        return CGSize(width: dimensions.width, height: metrics.lineHeight * CGFloat(lineCount(dimensions)))
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        guard let text = subviews.first else { return }
        let textProposal = ProposedViewSize(width: bounds.width, height: nil)
        let dimensions = text.dimensions(in: textProposal)
        text.place(at: CGPoint(x: bounds.minX, y: bounds.minY + metrics.baseline - dimensions[.firstTextBaseline]), anchor: .topLeading, proposal: textProposal)
    }

    func explicitAlignment(of guide: VerticalAlignment, in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGFloat? {
        if guide == .firstTextBaseline { return metrics.baseline }
        if guide == .lastTextBaseline { return bounds.height - metrics.lineHeight + metrics.baseline }
        return nil
    }
}

/// Marks only search spans; ordinary attributed backgrounds still use native drawing.
struct LWSearchMark: TextAttribute, Hashable {
    let color: Color
}

private struct LWCSSLineRenderer: TextRenderer {
    let lineHeight: CGFloat
    let ascent: CGFloat
    let descent: CGFloat

    func draw(layout: Text.Layout, in context: inout GraphicsContext) {
        guard let first = layout.first else { return }
        let firstBaseline = first.typographicBounds.origin.y
        for (index, line) in layout.enumerated() {
            var lineContext = context
            let desiredBaseline = firstBaseline + CGFloat(index) * lineHeight
            lineContext.translateBy(x: 0, y: desiredBaseline - line.typographicBounds.origin.y)
            for run in line {
                guard let mark = run[LWSearchMark.self] else { continue }
                let bounds = run.typographicBounds
                let rectangle = CGRect(x: bounds.rect.minX, y: bounds.origin.y - ascent.rounded(), width: bounds.width, height: ascent.rounded() + descent.rounded())
                guard rectangle.width > 0 && rectangle.height > 0 else { continue }
                lineContext.fill(Path(rectangle), with: .color(mark.color.opacity(0.16)))
                let thickness = min(1.5, rectangle.height)
                let rule = CGRect(x: rectangle.minX, y: rectangle.maxY - thickness, width: rectangle.width, height: thickness)
                lineContext.fill(Path(rule), with: .color(mark.color))
            }
            lineContext.draw(line)
        }
    }
}

private struct LWCSSLineBox: ViewModifier {
    let fontSize: CGFloat
    let lineHeight: CGFloat
    let data: Bool
    @Environment(\.dynamicTypeSize) private var dynamicType

    private var contentSizeCategory: UIContentSizeCategory {
        switch dynamicType {
        case .xSmall: return .extraSmall
        case .small: return .small
        case .medium: return .medium
        case .large: return .large
        case .xLarge: return .extraLarge
        case .xxLarge: return .extraExtraLarge
        case .xxxLarge: return .extraExtraExtraLarge
        case .accessibility1: return .accessibilityMedium
        case .accessibility2: return .accessibilityLarge
        case .accessibility3: return .accessibilityExtraLarge
        case .accessibility4: return .accessibilityExtraExtraLarge
        case .accessibility5: return .accessibilityExtraExtraExtraLarge
        @unknown default: return .large
        }
    }

    func body(content: Content) -> some View {
        let face = data ? "GeistMono-Regular" : "HostGrotesk-Regular"
        let base = UIFont(name: face, size: fontSize) ?? UIFont.systemFont(ofSize: fontSize)
        // Font.custom scales the font itself, including point-size rounding.
        // Scaling a one-point value and multiplying it by the base size gives
        // different glyph metrics, especially for caption/accessibility sizes.
        let font = UIFontMetrics(forTextStyle: data ? .caption1 : .body).scaledFont(for: base,
            compatibleWith: UITraitCollection(preferredContentSizeCategory: contentSizeCategory))
        let scale = font.pointSize / base.pointSize
        let metrics = LWCSSFontLineMetrics(ascent: font.ascender, descent: -font.descender, lineHeight: max(0.01, lineHeight * scale))
        LWCSSLineBoxLayout(metrics: metrics) {
            content.lineSpacing(0).textRenderer(LWCSSLineRenderer(lineHeight: metrics.lineHeight, ascent: metrics.ascent, descent: metrics.descent))
        }
    }
}

extension View {
    /// Keeps the original native Text and accessibility while matching CSS line boxes.
    func lwLineBox(_ fontSize: CGFloat, _ lineHeight: CGFloat, data: Bool = false) -> some View {
        modifier(LWCSSLineBox(fontSize: fontSize, lineHeight: lineHeight, data: data))
    }
}
