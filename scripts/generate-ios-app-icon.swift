import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

// The Mac icon's dot-matrix waveform, full-bleed and opaque for iOS (the system applies the mask).
let output = CommandLine.arguments.dropFirst().first ?? "localwave/Assets.xcassets/AppIcon.appiconset/AppIcon.png"
let side = 1024
let space = CGColorSpace(name: CGColorSpace.sRGB)!
let context = CGContext(data: nil, width: side, height: side, bitsPerComponent: 8, bytesPerRow: side * 4, space: space, bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
context.setFillColor(CGColor(srgbRed: 0x11 / 255, green: 0x11 / 255, blue: 0x11 / 255, alpha: 1))
context.fill(CGRect(x: 0, y: 0, width: side, height: side))
let scale = 1024.0 / 824.0
let lit = [3, 5, 9, 5, 7, 3, 9, 5, 3]
let cell = 62.0 * scale, dot = 46.0 * scale, rows = 9
let origin = (512 - Double(lit.count) * cell / 2, 512 - Double(rows) * cell / 2)
for (column, count) in lit.enumerated() {
    for row in 0..<rows {
        let on = abs(row - rows / 2) <= count / 2
        // Off dots: white at 9% over #111, flattened so the icon stays opaque.
        let off = (0x11 + (255 - 0x11) * 0.09) / 255
        context.setFillColor(on ? CGColor(srgbRed: 0xF3 / 255, green: 0xF3 / 255, blue: 0xF3 / 255, alpha: 1) : CGColor(srgbRed: off, green: off, blue: off, alpha: 1))
        context.fill(CGRect(x: origin.0 + Double(column) * cell + (cell - dot) / 2, y: origin.1 + Double(row) * cell + (cell - dot) / 2, width: dot, height: dot))
    }
}
let destination = CGImageDestinationCreateWithURL(URL(fileURLWithPath: output) as CFURL, UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(destination, context.makeImage()!, nil)
precondition(CGImageDestinationFinalize(destination))
