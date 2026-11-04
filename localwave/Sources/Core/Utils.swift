import AVFoundation
import CryptoKit
import os
import SQLite

public let subsystem = "com.snowbear.localwave"
public let schemaVersion = 29

public enum CustomError: Error {
    case genericError(_ message: String)
}

public extension Collection {
    subscript(safe index: Index) -> Element? {
        return indices.contains(index) ? self[index] : nil
    }
}

public struct SourceSyncResult {
    public let allItems: [SourceSyncResultItem]
    public let audioFiles: [SourceSyncResultItem]
    public let totalAudioFiles: Int

    public init(allItems: [SourceSyncResultItem], audioFiles: [SourceSyncResultItem], totalAudioFiles: Int) {
        self.allItems = allItems
        self.audioFiles = audioFiles
        self.totalAudioFiles = totalAudioFiles
    }
}

public struct SourceSyncResultItem {
    public let relativePath: String
    public let parentURL: URL?
    public let url: URL
    public let isDirectory: Bool
    public let name: String

    public init(rootURL: URL, current: URL, isDirectory: Bool) {
        let fh = FileHelper(fileURL: current)
        relativePath = fh.relativePath(from: rootURL) ?? ""
        parentURL = fh.parent().flatMap {
            $0
        }
        url = current
        self.isDirectory = isDirectory
        name = fh.name()
    }
}

public struct FileHelper {
    public let fileURL: URL

    public init(fileURL: URL) {
        self.fileURL = fileURL
    }

    public func toString() -> String {
        return fileURL.absoluteString
    }

    public func name() -> String {
        return fileURL.lastPathComponent
    }

    public func parent() -> URL? {
        return fileURL.deletingLastPathComponent()
    }

    public func relativePath(from baseURL: URL) -> String? {
        let basePath = baseURL.path
        let fullPath = fileURL.path
        guard fullPath.hasPrefix(basePath) else {
            return nil
        }
        return String(fullPath.dropFirst(basePath.count + 1))
    }

    public static func createURL(baseURL: URL, relativePath: String) -> URL? {
        if relativePath.isEmpty {
            return baseURL.absoluteURL // If the relative path is empty, return the base URL
        }
        return baseURL.appendingPathComponent(relativePath).absoluteURL
    }
}

public func setupSQLiteConnection(dbName: String) -> Connection? {
    let logger = createLogger(subsystem: subsystem, category: "setupSQLiteConnection")
    logger.debug("setting up connection ...")
    let dbPath = NSSearchPathForDirectoriesInDomains(
        .documentDirectory,
        .userDomainMask,
        true
    ).first!
    let dbFullPath = "\(dbPath)/\(dbName)"
    logger.debug("db path: \(dbFullPath)")
    do {
        return try Connection(dbFullPath)
    } catch {
        fatalError("DB init error: \(error)")
    }
}

public func hashStringToInt64(_ str: String) -> Int64 {
    let fnvOffsetBasis: UInt64 = 0xCBF2_9CE4_8422_2325
    let fnvPrime: UInt64 = 0x100_0000_01B3
    var hash = fnvOffsetBasis

    for byte in str.utf8 {
        hash ^= UInt64(byte)
        hash = hash &* fnvPrime
    }

    return Int64(bitPattern: hash & 0x7FFF_FFFF_FFFF_FFFF)
}

public func generateSongKey(artist: String, title: String, album: String) -> Int64 {
    // Normalize or lowercased if you like
    let combined = "\(artist.lowercased())__\(title.lowercased())__\(album.lowercased())"
    return hashStringToInt64(combined) // Using your existing FNV approach
}

public func makeURLHash(_ folderURL: URL) -> Int64 {
    return hashStringToInt64(folderURL.normalizedWithoutTrailingSlash.absoluteString)
}

public func makeBookmarkKey(_ folderURL: URL) -> String {
    return String(makeURLHash(folderURL))
}

// Could be anything
// file://
// or path
// or other url
public func makeURLFromString(_ s: String) -> URL {
    // Trim whitespace and newlines.
    let trimmed = s.trimmingCharacters(in: .whitespacesAndNewlines)

    // If the string is empty, fallback to the current directory.
    if trimmed.isEmpty {
        return URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
    }

    // Try parsing as a URL. If it has a scheme (like "http", "file", etc.), return it.
    if let url = URL(string: trimmed), let scheme = url.scheme, !scheme.isEmpty {
        return url
    }

    // If it starts with "/" or "~", assume it's a file path.
    if trimmed.hasPrefix("/") || trimmed.hasPrefix("~") {
        return URL(fileURLWithPath: (trimmed as NSString).expandingTildeInPath)
    }

    // If it contains a dot and no spaces, assume it’s a web address missing the scheme.
    if trimmed.contains(".") && !trimmed.contains(" ") {
        if let url = URL(string: "http://\(trimmed)") {
            return url
        }
    }

    // Fallback: treat it as a file path.
    return URL(fileURLWithPath: trimmed)
}

public extension URL {
    /// Returns a normalized URL with no trailing slash in its path (unless it's just "/" for root).
    var normalizedWithoutTrailingSlash: URL {
        // Standardize the URL first
        let standardizedURL = standardized
        // Use URLComponents to safely modify the path
        guard var components = URLComponents(url: standardizedURL, resolvingAgainstBaseURL: false)
        else {
            return standardizedURL
        }

        // Only modify if the path isn’t root and ends with a slash
        if components.path != "/" && components.path.hasSuffix("/") {
            // Remove all trailing slashes (leaving at least one character)
            while components.path.count > 1 && components.path.hasSuffix("/") {
                components.path.removeLast()
            }
        }

        return components.url ?? standardizedURL
    }
}

enum NotImplementedError: Error {
    case featureNotImplemented(message: String)
}

public func preprocessFTSQuery(_ input: String) -> String {
    input
        .components(separatedBy: .whitespacesAndNewlines)
        .filter { !$0.isEmpty }
        .map { "\($0)*" }
        .joined(separator: " ")
}
