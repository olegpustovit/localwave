#if os(iOS) || os(macOS)
import CoreText
import SwiftUI
#if os(iOS)
import UIKit
#endif

/// Desktop CSS buttons use fractional max-content widths; native Text rounds
/// its intrinsic width to the display pixel before horizontal padding is added.
struct LWDialogLabelMetrics {
    static func width(for title: String, pointSize: CGFloat, medium: Bool) -> CGFloat? {
#if os(iOS)
        guard let font = UIFont(name: medium ? "HostGrotesk-Medium" : "HostGrotesk-Regular", size: pointSize) else { return nil }
        let ctFont = CTFontCreateWithName(font.fontName as CFString, font.pointSize, nil)
#else
        let ctFont = CTFontCreateWithName((medium ? "HostGrotesk-Medium" : "HostGrotesk-Regular") as CFString, pointSize, nil)
#endif
        return width(for: title, font: ctFont)
    }

    static func scaledWidth(for title: String, medium: Bool, dynamicType: DynamicTypeSize) -> CGFloat? {
#if os(iOS)
        guard let base = UIFont(name: medium ? "HostGrotesk-Medium" : "HostGrotesk-Regular", size: 13) else { return nil }
        let category: UIContentSizeCategory
        switch dynamicType {
        case .xSmall: category = .extraSmall
        case .small: category = .small
        case .medium: category = .medium
        case .large: category = .large
        case .xLarge: category = .extraLarge
        case .xxLarge: category = .extraExtraLarge
        case .xxxLarge: category = .extraExtraExtraLarge
        case .accessibility1: category = .accessibilityMedium
        case .accessibility2: category = .accessibilityLarge
        case .accessibility3: category = .accessibilityExtraLarge
        case .accessibility4: category = .accessibilityExtraExtraLarge
        case .accessibility5: category = .accessibilityExtraExtraExtraLarge
        @unknown default: category = .large
        }
        // Font scaling rounds the actual font size differently from scaledValue
        // or a scaled unit multiplier. Match Font.custom(...relativeTo: .body).
        let font = UIFontMetrics(forTextStyle: .body).scaledFont(for: base,
            compatibleWith: UITraitCollection(preferredContentSizeCategory: category))
        let ctFont = CTFontCreateWithName(font.fontName as CFString, font.pointSize, nil)
        return width(for: title, font: ctFont)
#else
        width(for: title, pointSize: 13, medium: medium)
#endif
    }

    private static func width(for title: String, font: CTFont) -> CGFloat? {
        let line = CTLineCreateWithAttributedString(NSAttributedString(string: title, attributes: [NSAttributedString.Key(kCTFontAttributeName as String): font]))
        let advance = CGFloat(CTLineGetTypographicBounds(line, nil, nil, nil))
        guard advance.isFinite, advance >= 0 else { return nil }
        // Verified against browser Cancel/Create/Save/Delete List measurements.
        return ceil(advance * 64) / 64
    }
}

struct LWDialogLabelWidth: ViewModifier {
    let title: String
    let medium: Bool
    let enabled: Bool
    @Environment(\.dynamicTypeSize) private var dynamicType

    func body(content: Content) -> some View {
        if enabled {
            LWDialogLabelLayout(width: LWDialogLabelMetrics.scaledWidth(for: title, medium: medium, dynamicType: dynamicType)) {
                content.fixedSize(horizontal: true, vertical: false)
            }
        } else { content }
    }
}

struct LWDialogLabelLayout: Layout {
    let width: CGFloat?

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        guard let child = subviews.first else { return .zero }
        let size = child.sizeThatFits(.unspecified)
        return CGSize(width: width ?? size.width, height: size.height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        // Keep the original Text at its natural size, without wrapping, scaling,
        // centering its pixel-ceiled box, or changing its accessibility element.
        subviews.first?.place(at: bounds.origin, anchor: .topLeading, proposal: .unspecified)
    }

    func explicitAlignment(of guide: VerticalAlignment, in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGFloat? {
        guard guide == .firstTextBaseline || guide == .lastTextBaseline else { return nil }
        return subviews.first?.dimensions(in: .unspecified)[guide]
    }
}

#endif
