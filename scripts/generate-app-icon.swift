import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

// A vector waveform on an opaque background. Regenerate with `swift scripts/generate-app-icon.swift`.
let output = CommandLine.arguments.dropFirst().first ?? "localwave/Assets.xcassets/AppIcon.appiconset/AppIcon.png"
let size = 1024
let space = CGColorSpace(name: CGColorSpace.sRGB)!
let context = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8,
                        bytesPerRow: size * 4, space: space,
                        bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
let colors = [CGColor(srgbRed: 0.06, green: 0.20, blue: 0.45, alpha: 1),
              CGColor(srgbRed: 0.08, green: 0.48, blue: 0.80, alpha: 1)]
let gradient = CGGradient(colorsSpace: space, colors: colors as CFArray, locations: [0, 1])!
context.drawLinearGradient(gradient, start: CGPoint(x: 0, y: 0),
                           end: CGPoint(x: size, y: size), options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
context.setStrokeColor(CGColor(gray: 1, alpha: 1))
context.setLineWidth(48)
context.setLineCap(.round)
for (index, height) in [140.0, 250, 390, 560, 390, 250, 140].enumerated() {
    let x = 242.0 + Double(index) * 90
    context.move(to: CGPoint(x: x, y: 512 - height / 2))
    context.addLine(to: CGPoint(x: x, y: 512 + height / 2))
    context.strokePath()
}
let destination = CGImageDestinationCreateWithURL(URL(fileURLWithPath: output) as CFURL,
                                                  UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(destination, context.makeImage()!, nil)
precondition(CGImageDestinationFinalize(destination), "Unable to write app icon")
