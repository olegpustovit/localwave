import Foundation

/// Native FLAC metadata blocks (RFC 9639 sections 8.6 and 8.8).
/// Read headers without loading the audio stream into memory.
struct FLACMetadata {
    var comments: [String: String] = [:]
    var artwork: Data?

    static func read(url: URL) throws -> FLACMetadata? {
        let file = try FileHandle(forReadingFrom: url)
        defer { try? file.close() }
        guard try file.read(upToCount: 4) == Data("fLaC".utf8) else { return nil }
        var result = FLACMetadata()
        var total = 0
        while let header = try file.read(upToCount: 4), header.count == 4 {
            let length = Int(header[1]) << 16 | Int(header[2]) << 8 | Int(header[3])
            total += length + 4
            guard total <= 64 * 1024 * 1024 else { return result }
            let type = header[0] & 0x7f
            if type == 4 || type == 6 {
                guard let block = try file.read(upToCount: length), block.count == length else { return result }
                if type == 4 { result.readComments(block) }
                else { result.readPicture(block) }
            } else {
                try file.seek(toOffset: file.offset() + UInt64(length))
            }
            if header[0] & 0x80 != 0 { break }
        }
        return result
    }

    private mutating func readComments(_ data: Data) {
        var cursor = Cursor(data: data, littleEndian: true)
        guard let vendorLength = cursor.integer(), cursor.take(vendorLength) != nil,
              let count = cursor.integer(), count <= data.count / 4 else { return }
        for _ in 0..<count {
            guard let length = cursor.integer(), let bytes = cursor.take(length),
                  let value = String(data: bytes, encoding: .utf8),
                  let separator = value.firstIndex(of: "=") else { return }
            let key = value[..<separator].uppercased()
            let text = String(value[value.index(after: separator)...])
            if comments[key] == nil { comments[key] = text }
        }
    }

    private mutating func readPicture(_ data: Data) {
        var cursor = Cursor(data: data)
        guard let type = cursor.integer(), let mimeLength = cursor.integer(),
              let mime = cursor.take(mimeLength), mime != Data("-->".utf8),
              let descriptionLength = cursor.integer(), cursor.take(descriptionLength) != nil,
              cursor.take(16) != nil, let imageLength = cursor.integer(),
              let image = cursor.take(imageLength) else { return }
        if artwork == nil || type == 3 { artwork = image }
    }

    private struct Cursor {
        let data: Data
        var littleEndian = false
        var offset = 0

        mutating func take(_ count: Int) -> Data? {
            guard count >= 0, count <= data.count - offset else { return nil }
            defer { offset += count }
            return data.subdata(in: offset..<(offset + count))
        }

        mutating func integer() -> Int? {
            guard let bytes = take(4) else { return nil }
            let ordered = littleEndian ? Array(bytes.reversed()) : Array(bytes)
            return ordered.reduce(0) { ($0 << 8) | Int($1) }
        }
    }
}
