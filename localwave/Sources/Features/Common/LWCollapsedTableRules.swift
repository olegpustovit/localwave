#if os(iOS)
import SwiftUI

/// CSS collapsed table rules paint after cell backgrounds and snap to logical
/// pixels, even at DPR2/3. A one-point overflow surface keeps the shared boundary
/// visible without adding row height or clipping it to a Canvas edge.
struct LWCollapsedTableRules: ViewModifier {
    var top = true
    var bottom = false
    var color = LWTheme.line

    func body(content: Content) -> some View {
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
    }

    private func draw(_ boundary: CGFloat, in bounds: CGRect, width: CGFloat, context: inout GraphicsContext) {
        let rect = CGRect(x: 0, y: floor(boundary) - bounds.minY + 1, width: width, height: 1)
        context.fill(Path(rect), with: .color(color), style: FillStyle(antialiased: false))
    }
}

#endif
