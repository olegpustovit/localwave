import XCTest
import Foundation
@testable import LocalWaveData

final class FLACMetadataTests: XCTestCase {
    func testVorbisCommentsAndFrontCover() throws {
        var comments = little(0) + little(4)
        for tag in ["TITLE=Hello=World", "artist=Björk", "TRACKNUMBER=3/12", "ALBUMARTIST=Various Artists"] {
            let value = Data(tag.utf8)
            comments += little(value.count) + value
        }
        let image = Data([0x89, 0x50, 0x4e, 0x47])
        let mime = Data("image/png".utf8)
        let picture = big(3) + big(mime.count) + mime + big(0) + Data(repeating: 0, count: 16) + big(image.count) + image
        let file = Data("fLaC".utf8) + block(4, comments) + block(0x86, picture)
        let metadata = try XCTUnwrap(read(file))
        XCTAssertEqual(metadata.comments["TITLE"], "Hello=World")
        XCTAssertEqual(metadata.comments["ARTIST"], "Björk")
        XCTAssertEqual(metadata.comments["TRACKNUMBER"], "3/12")
        XCTAssertEqual(metadata.artwork, image)
    }

    func testTruncatedAndInvalidInputsDoNotReadOutOfBounds() throws {
        XCTAssertNil(try read(Data("not FLAC".utf8)))
        let truncated = Data("fLaC".utf8) + block(0x84, little(0) + little(1) + little(Int(UInt32.max)))
        XCTAssertEqual(try read(truncated)?.comments.count, 0)
        for count in 0..<truncated.count {
            _ = try read(Data(truncated.prefix(count)))
        }
    }

    private func read(_ data: Data) throws -> FLACMetadata? {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString + ".flac")
        try data.write(to: url)
        defer { try? FileManager.default.removeItem(at: url) }
        return try FLACMetadata.read(url: url)
    }
    private func little(_ value: Int) -> Data {
        Data((0..<4).map { UInt8(truncatingIfNeeded: value >> ($0 * 8)) })
    }
    private func big(_ value: Int) -> Data { Data(little(value).reversed()) }
    private func block(_ type: UInt8, _ data: Data) -> Data {
        Data([type, UInt8(truncatingIfNeeded: data.count >> 16), UInt8(truncatingIfNeeded: data.count >> 8), UInt8(truncatingIfNeeded: data.count)]) + data
    }
}
