#if canImport(UIKit)
import LocalWaveDomain
import SwiftUI
import UIKit
import XCTest
@testable import localwave

@MainActor
final class ArtworkRenderingTests: XCTestCase {
    func testPhotoThumbnailPreservesAspectRatioWithoutDecodingFullSize() throws {
        let renderer = UIGraphicsImageRenderer(size: CGSize(width: 1200, height: 600), format: {
            let format = UIGraphicsImageRendererFormat(); format.scale = 1; return format
        }())
        let image = renderer.image { context in
            UIColor.blue.setFill(); context.fill(CGRect(x: 0, y: 0, width: 1200, height: 600))
        }
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).appendingPathExtension("png")
        defer { try? FileManager.default.removeItem(at: url) }
        try XCTUnwrap(image.pngData()).write(to: url)
        let thumbnail = try XCTUnwrap(LWArtwork.thumbnail(at: url, maxPixelSize: 120)?.cgImage)
        XCTAssertEqual(thumbnail.width, 120)
        XCTAssertEqual(thumbnail.height, 60)
        XCTAssertNil(LWArtwork.thumbnail(at: url.appendingPathExtension("missing"), maxPixelSize: 120))
    }

    func testAllFallbackFamiliesRenderLegibleOpaqueThumbnails() throws {
        // These album titles select the ten deterministic cover families.
        let titles = ["Artwork Study 4", "Artwork Study 7", "Artwork Study 6", "Artwork Study 1", "Artwork Study 0", "Artwork Study 3", "Artwork Study 2", "Artwork Study 12", "Artwork Study 13", "Artwork Study 5"]
        var images: [UIImage] = []
        for (index, title) in titles.enumerated() {
            let song = Song(id: Int64(index), songKey: Int64(index), artist: "Fixture Artist", title: "Fixture", album: title, albumArtist: "Fixture Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 0, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly)
            for size in [32, 128] {
                let renderer = ImageRenderer(content: LWArtwork(song: song, size: CGFloat(size)))
                renderer.scale = 1
                let image = try XCTUnwrap(renderer.uiImage)
                let cgImage = try XCTUnwrap(image.cgImage)
                var pixels = [UInt8](repeating: 0, count: size * size * 4)
                let drew = pixels.withUnsafeMutableBytes { bytes -> Bool in
                    guard let context = CGContext(data: bytes.baseAddress, width: size, height: size, bitsPerComponent: 8, bytesPerRow: size * 4, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return false }
                    context.draw(cgImage, in: CGRect(x: 0, y: 0, width: size, height: size))
                    return true
                }
                XCTAssertTrue(drew)
                XCTAssertTrue(stride(from: 3, to: pixels.count, by: 4).allSatisfy { pixels[$0] == 255 }, "Transparent cover: \(title)")
                let hasSignal = stride(from: 0, to: pixels.count, by: 4).contains { offset in
                    let red = Int(pixels[offset]), green = Int(pixels[offset + 1]), blue = Int(pixels[offset + 2])
                    return blue > red + 12 && blue > green + 12
                }
                XCTAssertTrue(hasSignal, "Signal missing: \(title)")
                if size == 128 { images.append(image) }
            }
        }
        let gallery = ImageRenderer(content: HStack(spacing: 8) {
            ForEach(images.indices, id: \.self) { index in
                VStack(spacing: 6) {
                    Image(uiImage: images[index])
                    Text("\(index + 1)").font(.system(size: 12, design: .monospaced))
                }
            }
        }.padding(12).background(Color(white: 0.95)))
        let attachment = XCTAttachment(image: try XCTUnwrap(gallery.uiImage))
        attachment.name = "Ten native fallback cover families"
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}

#endif
