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
let colors = [CGColor(srgbRed: 0.08, green: 0.20, blue: 0.68, alpha: 1), CGColor(srgbRed: 0.16, green: 0.30, blue: 0.95, alpha: 1)]
let gradient = CGGradient(colorsSpace: colorSpace, colors: colors as CFArray, locations: [0, 1])!
context.drawLinearGradient(gradient, start: CGPoint(x: artwork.minX, y: artwork.minY), end: CGPoint(x: artwork.maxX, y: artwork.maxY), options: [])
context.setStrokeColor(CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 0.96))
context.setLineWidth(34)
context.setLineCap(.round)
for (index, height) in [112.0, 210, 326, 454, 326, 210, 112].enumerated() {
    let x = 298.0 + Double(index) * 71.0
    context.move(to: CGPoint(x: x, y: 512 - height / 2))
    context.addLine(to: CGPoint(x: x, y: 512 + height / 2))
    context.strokePath()
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
