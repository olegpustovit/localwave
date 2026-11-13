import Foundation

// MARK: - Search Results

public struct PathSearchResult: Sendable {
    public let pathId: Int64
    public let rank: Double
    public init(pathId: Int64, rank: Double) {
        self.pathId = pathId
        self.rank = rank
    }
}

// MARK: - Models

public struct User: Sendable {
    public let id: Int64?
    public let icloudId: Int64

    public init(id: Int64?, icloudId: Int64) {
        self.id = id
        self.icloudId = icloudId
    }
}

public enum SourceType: String, Codable, CaseIterable, Sendable {
    case iCloud
}

public struct Playlist: Identifiable, Sendable {
    public let id: Int64?
    public let name: String
    public let createdAt: Date
    public let updatedAt: Date?

    public init(id: Int64?, name: String, createdAt: Date, updatedAt: Date?) {
        self.id = id
        self.name = name
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }
}

public struct PlaylistSong: Identifiable, Sendable {
    public let id: Int64?
    public let playlistId: Int64
    public let songId: Int64
    public let position: Int // New: For ordering

    public init(id: Int64?, playlistId: Int64, songId: Int64, position: Int) {
        self.id = id
        self.playlistId = playlistId
        self.songId = songId
        self.position = position
    }
}

public struct Source: Sendable, Identifiable {
    public var id: Int64?
    public var dirPath: String
    public var pathId: Int64
    public var userId: Int64
    public var type: SourceType?
    public var totalPaths: Int?
    public var syncError: String?
    public var isCurrent: Bool
    public var createdAt: Date
    public var lastSyncedAt: Date?
    public var updatedAt: Date?
    public var bookmark: Data?

    public init(
        id: Int64?,
        dirPath: String,
        pathId: Int64,
        userId: Int64,
        type: SourceType?,
        totalPaths: Int?,
        syncError: String?,
        isCurrent: Bool,
        createdAt: Date,
        lastSyncedAt: Date?,
        updatedAt: Date?,
        bookmark: Data? = nil
    ) {
        self.id = id
        self.dirPath = dirPath
        self.pathId = pathId
        self.userId = userId
        self.type = type
        self.totalPaths = totalPaths
        self.syncError = syncError
        self.isCurrent = isCurrent
        self.createdAt = createdAt
        self.lastSyncedAt = lastSyncedAt
        self.updatedAt = updatedAt
        self.bookmark = bookmark
    }

    public var stableId: Int64 {
        id ?? Int64(abs(dirPath.hashValue))
    }
}

public struct SongImportProgress: Sendable {
    public let percentage: Double
    public let completedFiles: Int?
    public let totalFiles: Int?
    public let url: URL

    public init(percentage: Double, completedFiles: Int? = nil, totalFiles: Int? = nil, url: URL) {
        self.percentage = percentage
        self.completedFiles = completedFiles
        self.totalFiles = totalFiles
        self.url = url
    }
}

public enum AudioFileSupport {
    public static let extensions: Set<String> = ["mp3", "wav", "m4a", "flac", "aac", "aiff", "aif"]
}

public struct SourcePath: Sendable {
    public let id: Int64?
    public let sourceId: Int64

    public let pathId: Int64
    public let parentPathId: Int64?
    public let name: String
    public let relativePath: String
    public let isDirectory: Bool
    public let fileSize: Int64?
    public var isAudio: Bool { !isDirectory && AudioFileSupport.extensions.contains((name as NSString).pathExtension.lowercased()) }

    public let fileHashSHA256: Data?
    public let runId: Int64

    public let createdAt: Date
    public let updatedAt: Date?

    public init(
        id: Int64?,
        sourceId: Int64,
        pathId: Int64,
        parentPathId: Int64?,
        name: String,
        relativePath: String,
        isDirectory: Bool,
        fileHashSHA256: Data?,
        runId: Int64,
        createdAt: Date,
        updatedAt: Date?,
        fileSize: Int64? = nil
    ) {
        self.id = id
        self.sourceId = sourceId
        self.pathId = pathId
        self.parentPathId = parentPathId
        self.name = name
        self.relativePath = relativePath
        self.isDirectory = isDirectory
        self.fileSize = fileSize
        self.fileHashSHA256 = fileHashSHA256
        self.runId = runId
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }

    public func copyWith(id: Int64?) -> SourcePath {
        return SourcePath(
            id: id,
            sourceId: sourceId,
            pathId: pathId,
            parentPathId: parentPathId,
            name: name,
            relativePath: relativePath,
            isDirectory: isDirectory,
            fileHashSHA256: fileHashSHA256,
            runId: runId,
            createdAt: createdAt,
            updatedAt: updatedAt,
            fileSize: fileSize
        )
    }
}

public struct Album: Identifiable, Hashable {
    public let id: String
    public let name: String
    public let artist: String?
    public let coverArtPath: String?

    public init(name: String, artist: String?, coverArtPath: String?) {
        let cleanedName = name.isEmpty ? "Unknown Album" : name
        let cleanedArtist = artist?.isEmpty ?? true ? nil : artist

        id = "\(cleanedArtist ?? "Unknown Artist")-\(cleanedName)"
        self.name = cleanedName
        self.artist = cleanedArtist
        self.coverArtPath = coverArtPath
    }
}

// 1. Add file state tracking to Song model
public enum FileState: Int, Codable, Sendable {
    case bookmarkOnly
    case copyPending
    case copied
    case failed
}

public enum FileAvailability: String, Codable, Sendable {
    case onDevice
    case inCloud
    case unknown
}

/// Example song model, no sourceId. We store all metadata ourselves.
public struct Song: Sendable, Identifiable, Equatable {
    public let id: Int64?

    /// A unique-ish hash of (artist, title, album).
    public let songKey: Int64

    public let artist: String
    public let title: String
    public let album: String

    public let albumArtist: String
    public let releaseYear: Int?
    public let discNumber: Int?

    // trackNumber property for album order
    public let trackNumber: Int?
    public let coverArtPath: String?
    public var bookmark: Data?
    public var pathHash: Int64

    /// Timestamps
    public let createdAt: Date
    public let updatedAt: Date?

    public let localFilePath: String? // Path in app's Documents directory
    public var fileState: FileState

    public let isFavorite: Int64
    public let sourceId: Int64?
    public let relativePath: String?
    public let duration: Double?
    public let fileSize: Int64?
    public let fileExtension: String?
    public let sampleRate: Double?
    public let bitDepth: Int?
    public let sourceAvailability: FileAvailability?

    public init(
        id: Int64?,
        songKey: Int64,
        artist: String,
        title: String,
        album: String,
        albumArtist: String,
        releaseYear: Int?,
        discNumber: Int?,
        trackNumber: Int?,
        coverArtPath: String?,
        bookmark: Data?,
        pathHash: Int64,
        createdAt: Date,
        updatedAt: Date?,
        localFilePath: String?,
        fileState: FileState,
        isFavorite: Int64 = 0,
        sourceId: Int64? = nil,
        relativePath: String? = nil,
        duration: Double? = nil,
        fileSize: Int64? = nil,
        fileExtension: String? = nil,
        sampleRate: Double? = nil,
        bitDepth: Int? = nil,
        sourceAvailability: FileAvailability? = nil
    ) {
        self.id = id
        self.songKey = songKey
        self.artist = artist
        self.title = title
        self.album = album
        self.albumArtist = albumArtist
        self.releaseYear = releaseYear
        self.discNumber = discNumber
        self.trackNumber = trackNumber
        self.coverArtPath = coverArtPath
        self.bookmark = bookmark
        self.pathHash = pathHash
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.localFilePath = localFilePath
        self.fileState = fileState
        self.isFavorite = isFavorite
        self.sourceId = sourceId
        self.relativePath = relativePath
        self.duration = duration
        self.fileSize = fileSize
        self.fileExtension = fileExtension
        self.sampleRate = sampleRate
        self.bitDepth = bitDepth
        self.sourceAvailability = sourceAvailability
    }

    public func copyWith(_ fp: String?, _ st: FileState) -> Song {

        Song(
            id: id,
            songKey: songKey,
            artist: artist,
            title: title,
            album: album,
            albumArtist: albumArtist,
            releaseYear: releaseYear,
            discNumber: discNumber,
            trackNumber: trackNumber,
            coverArtPath: coverArtPath,
            bookmark: bookmark,
            pathHash: pathHash,
            createdAt: createdAt,
            updatedAt: updatedAt,
            localFilePath: fp,
            fileState: st,
            isFavorite: isFavorite,
            sourceId: sourceId,
            relativePath: relativePath,
            duration: duration,
            fileSize: fileSize,
            fileExtension: fileExtension,
            sampleRate: sampleRate,
            bitDepth: bitDepth,
            sourceAvailability: sourceAvailability
        )
    }

    public func copyWith(id: Int64?) -> Song {
        Song(
            id: id,
            songKey: songKey,
            artist: artist,
            title: title,
            album: album,
            albumArtist: albumArtist,
            releaseYear: releaseYear,
            discNumber: discNumber,
            trackNumber: trackNumber,
            coverArtPath: coverArtPath,
            bookmark: bookmark,
            pathHash: pathHash,
            createdAt: createdAt,
            updatedAt: updatedAt,
            localFilePath: localFilePath,
            fileState: fileState,
            isFavorite: isFavorite,
            sourceId: sourceId,
            relativePath: relativePath,
            duration: duration,
            fileSize: fileSize,
            fileExtension: fileExtension,
            sampleRate: sampleRate,
            bitDepth: bitDepth,
            sourceAvailability: sourceAvailability
        )
    }

    public var needsCopy: Bool {
        return fileState == .bookmarkOnly || fileState == .failed
    }

    public static func == (lhs: Song, rhs: Song) -> Bool {
        return lhs.id == rhs.id
    }

    public var uniqueId: Int64 {
        return id ?? songKey
    }
}

public extension Song {
    /// Keep ambiguous band punctuation intact; split explicit guest/collaboration credits only.
    static func artists(in credit: String) -> [String] {
        let unwrapped = credit.replacingOccurrences(
            of: #"(?i)\s*[\(\[](?:feat(?:uring)?\.?|ft\.?)\s+([^\)\]]+)[\)\]]"#,
            with: ";$1", options: .regularExpression)
        let separated = unwrapped.replacingOccurrences(
            of: #"(?i)\s+(?:feat(?:uring)?\.?|ft\.?)\s+|\s+[x×]\s+|\s*;\s*"#,
            with: ";", options: .regularExpression)
        var seen = Set<String>()
        return separated.components(separatedBy: ";").compactMap {
            let name = $0.trimmingCharacters(in: .whitespacesAndNewlines)
            return !name.isEmpty && seen.insert(name.lowercased()).inserted ? name : nil
        }
    }

    var artistNames: [String] { Self.artists(in: artist) }

    func fileDisplayTitle(fallback: String) -> String {
        title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? fallback : title
    }

    var fileDisplaySubtitle: String {
        [artist, album].filter { !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }.joined(separator: " · ")
    }
}
