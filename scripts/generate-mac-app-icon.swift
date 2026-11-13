import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let output = CommandLine.arguments.dropFirst().first ?? "localwave/Assets.xcassets/MacAppIcon.appiconset"
let side = 1024
let colorSpace = CGColorSpace(name: CGColorSpace.sRGB)!
let context = CGContext(data: nil, width: side, height: side, bitsPerComponent: 8, bytesPerRow: side * 4, space: colorSpace, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
let artwork = CGRect(x: 100, y: 100, width: 824, height: 824)
let shape = CGPath(roundedRect: artwork, cornerWidth: 184, cornerHeight: 184, transform: nil)
context.saveGState()
context.setShadow(offset: CGSize(width: 0, height: -16), blur: 28, color: CGColor(gray: 0, alpha: 0.18))
context.addPath(shape)
context.setFillColor(CGColor(gray: 0.12, alpha: 1))
context.fillPath()
context.restoreGState()
context.addPath(shape)
context.clip()
// Matches the UI kit: ink ground, square dot-matrix cells (like MatrixText), paper-coloured lit dots.
context.setFillColor(CGColor(srgbRed: 0x11 / 255, green: 0x11 / 255, blue: 0x11 / 255, alpha: 1))
context.fill(artwork)
let lit = [3, 5, 9, 5, 7, 3, 9, 5, 3]
let cell = 62.0, dot = 46.0, rows = 9
let origin = (512 - Double(lit.count) * cell / 2, 512 - Double(rows) * cell / 2)
for (column, count) in lit.enumerated() {
    for row in 0..<rows {
        let on = abs(row - rows / 2) <= count / 2
        context.setFillColor(on ? CGColor(srgbRed: 0xF3 / 255, green: 0xF3 / 255, blue: 0xF3 / 255, alpha: 1) : CGColor(gray: 1, alpha: 0.09))
        context.fill(CGRect(x: origin.0 + Double(column) * cell + (cell - dot) / 2, y: origin.1 + Double(row) * cell + (cell - dot) / 2, width: dot, height: dot))
    }
}
let icon = context.makeImage()!
let directory = URL(fileURLWithPath: output, isDirectory: true)
try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
for points in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let pixels = points * scale
        let resized = CGContext(data: nil, width: pixels, height: pixels, bitsPerComponent: 8, bytesPerRow: pixels * 4, space: colorSpace, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        resized.interpolationQuality = .high
        resized.draw(icon, in: CGRect(x: 0, y: 0, width: pixels, height: pixels))
        let suffix = scale == 1 ? ".png" : "@2x.png"
        let file = directory.appendingPathComponent("icon_\(points)x\(points)\(suffix)")
        let destination = CGImageDestinationCreateWithURL(file as CFURL, UTType.png.identifier as CFString, 1, nil)!
        CGImageDestinationAddImage(destination, resized.makeImage()!, nil)
        precondition(CGImageDestinationFinalize(destination), "Unable to write \(file.lastPathComponent)")
    }
}
