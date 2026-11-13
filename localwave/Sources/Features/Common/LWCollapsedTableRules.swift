#if os(iOS) || os(macOS)
import SwiftUI

/// CSS collapsed table rules paint after cell backgrounds and snap to logical
/// pixels, even at DPR2/3. A one-point overflow surface keeps the shared boundary
/// visible without adding row height or clipping it to a Canvas edge.
struct LWCollapsedTableRules: ViewModifier {
    var top = true
    var bottom = false
    var color = LWTheme.line

    func body(content: Content) -> some View {
#if os(macOS)
        // AppKit scrolls on whole backing pixels and rows have integral heights, so static rules land on the same pixels as the snapped Canvas,
        // without a per-row GeometryReader + Canvas redraw on every scroll frame (measured: the main cost of scrolling the song table).
        content
            .overlay(alignment: .top) { if top { Rectangle().fill(color).frame(height: 1).allowsHitTesting(false).accessibilityHidden(true) } }
            .overlay(alignment: .bottom) { if bottom { Rectangle().fill(color).frame(height: 1).offset(y: 1).allowsHitTesting(false).accessibilityHidden(true) } }
#else
        content.overlay {
            if top || bottom {
                GeometryReader { proxy in
                    // Existing normal .offset animations use integer-point endpoints;
                    // SwiftUI composites their target raster during presentation. Fractional
                    // endpoints require a stable untransformed coordinate space before snapping.
                    let bounds = proxy.frame(in: .global)
                    Canvas { context, size in
                        if top { draw(bounds.minY, in: bounds, width: size.width, context: &context) }
                        if bottom { draw(bounds.maxY, in: bounds, width: size.width, context: &context) }
                    }
                    .frame(height: proxy.size.height + 2)
                    .offset(y: -1)
                }
                .allowsHitTesting(false)
                .accessibilityHidden(true)
            }
        }
#endif
    }

    private func draw(_ boundary: CGFloat, in bounds: CGRect, width: CGFloat, context: inout GraphicsContext) {
        let rect = CGRect(x: 0, y: floor(boundary) - bounds.minY + 1, width: width, height: 1)
        context.fill(Path(rect), with: .color(color), style: FillStyle(antialiased: false))
    }
}

#endif
