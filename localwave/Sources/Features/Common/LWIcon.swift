import SwiftUI

/// The reference's 24-unit vectors, compiled as template SVG assets.
struct LWIcon: View {
    let name: String
    var size: CGFloat = 20

    init(_ name: String, size: CGFloat = 20) {
        self.name = name
        self.size = size
    }

    var body: some View {
        Image("LW-" + name).resizable().renderingMode(.template)
            .frame(width: size, height: size).accessibilityHidden(true)
    }
}
