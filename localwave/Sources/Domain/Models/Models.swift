import Foundation

// models
struct User: Sendable {
    let id: Int64?
    let icloudId: Int64
}

enum SourceType: String, Codable, CaseIterable {
    case iCloud
}

struct Playlist: Identifiable, Sendable {
    let id: Int64?
    let name: String
    let createdAt: Date
    let updatedAt: Date?
}

struct PlaylistSong: Identifiable, Sendable {
    let id: Int64?
    let playlistId: Int64
    let songId: Int64
    let position: Int // New: For ordering
}

struct Source: Sendable, Identifiable {
    var id: Int64?
    var dirPath: String
    var pathId: Int64
    var userId: Int64
    var type: SourceType?
    var totalPaths: Int?
    var syncError: String?
    var isCurrent: Bool
    var createdAt: Date
    var lastSyncedAt: Date?
    var updatedAt: Date?

    var stableId: Int64 {
        id ?? Int64(abs(dirPath.hashValue))
    }
}

struct SourcePath: Sendable {
    let id: Int64?
    let sourceId: Int64

    let pathId: Int64
    let parentPathId: Int64?
    let name: String
    let relativePath: String
    let isDirectory: Bool

    let fileHashSHA256: Data?
    let runId: Int64

    let createdAt: Date
    let updatedAt: Date?

    func copyWith(id: Int64?) -> SourcePath {
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
            updatedAt: updatedAt
        )
    }
}

struct Album: Identifiable, Hashable {
    let id: String
    let name: String
    let artist: String?
    let coverArtPath: String?

    init(name: String, artist: String?, coverArtPath: String?) {
        let cleanedName = name.isEmpty ? "Unknown Album" : name
        let cleanedArtist = artist?.isEmpty ?? true ? nil : artist

        id = "\(cleanedArtist ?? "Unknown Artist")-\(cleanedName)"
        self.name = cleanedName
        self.artist = cleanedArtist
        self.coverArtPath = coverArtPath
    }
}

// 1. Add file state tracking to Song model
enum FileState: Int, Codable {
    case bookmarkOnly
    case copyPending
    case copied
    case failed
}

/// Example song model, no sourceId. We store all metadata ourselves.
struct Song: Sendable, Identifiable, Equatable {
    let id: Int64?

    /// A unique-ish hash of (artist, title, album).
    let songKey: Int64

    let artist: String
    let title: String
    let album: String

    let albumArtist: String
    let releaseYear: Int?
    let discNumber: Int?

    // trackNumber property for album order
    let trackNumber: Int?
    let coverArtPath: String?
    var bookmark: Data?
    var pathHash: Int64

    /// Timestamps
    let createdAt: Date
    let updatedAt: Date?

    let localFilePath: String? // Path in app's Documents directory
    var fileState: FileState

    let isFavorite: Int64
    
    func copyWith(_ fp: String, _ st: FileState) -> Song {
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
            isFavorite: isFavorite
        )
    }

    func copyWith(id: Int64?) -> Song {
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
            isFavorite: isFavorite
        )
    }

    var needsCopy: Bool {
        return fileState == .bookmarkOnly || fileState == .failed
    }

    static func == (lhs: Song, rhs: Song) -> Bool {
        return lhs.id == rhs.id
    }

    var uniqueId: Int64 {
        return id ?? songKey
    }
}
