import Foundation
import LocalWaveCore

/// A real local source, imported through the same pipeline as a chosen folder.
/// This never seeds library rows or simulated provider/availability state.
@MainActor
enum LWDemoLibrary {
    private static var installing = false

    /// Returns false if another import populated the library before installation.
    @discardableResult
    static func installIfEmpty(dependencies: DependencyContainer) async throws -> Bool {
        guard !installing else { throw CustomError.genericError("The demo library is already being added.") }
        installing = true
        defer { installing = false }
        guard try await dependencies.songRepository.totalSongCount(query: "") == 0 else { return false }
        let folder = try await Task.detached(priority: .userInitiated) { try prepareFiles() }.value
        try Task.checkCancellation()
        // Copying can take a moment. Recheck before the normal importer can
        // merge any metadata keys with songs added by another user action.
        guard try await dependencies.songRepository.totalSongCount(query: "") == 0 else { return false }
        let songs = try await dependencies.addMusicURLs([folder])
        guard songs.count == 92 else {
            throw CustomError.genericError("Some demo songs could not be indexed. The Demo Library folder is available in Folders; rescan it to retry, or explore the songs already added.")
        }
        return true
    }

    private struct Catalog: Decodable {
        let version: Int
        let tracks: [Track]
    }
    private struct Track: Decodable {
        let resource: String
        let relativePath: String
        let bytes: Int64
    }

    nonisolated private static func prepareFiles() throws -> URL {
        let manager = FileManager.default
        guard let catalogURL = Bundle.main.url(forResource: "localwave-demo-catalog", withExtension: "json") else {
            throw CustomError.genericError("The bundled demo library is unavailable.")
        }
        let catalogData = try Data(contentsOf: catalogURL)
        let catalog = try JSONDecoder().decode(Catalog.self, from: catalogData)
        guard catalog.version == 1, catalog.tracks.count == 92 else {
            throw CustomError.genericError("The bundled demo catalog is incomplete.")
        }
        let support = try manager.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
            .appendingPathComponent("LocalWave", isDirectory: true)
        try manager.createDirectory(at: support, withIntermediateDirectories: true)
        let destination = support.appendingPathComponent("Demo Library", isDirectory: true)
        let markerName = ".localwave-demo-catalog-v1.json"
        if manager.fileExists(atPath: destination.path) {
            // Only reuse our own completed folder. Never replace user edits or
            // an unrelated directory at the reserved path.
            guard (try? Data(contentsOf: destination.appendingPathComponent(markerName))) == catalogData else {
                throw CustomError.genericError("The Demo Library folder already exists and cannot be replaced.")
            }
            for track in catalog.tracks {
                let file = try fileURL(track.relativePath, under: destination)
                guard manager.isReadableFile(atPath: file.path) else {
                    throw CustomError.genericError("A demo file was removed. Open Folders to manage the existing Demo Library source.")
                }
            }
            return destination
        }
        let staging = support.appendingPathComponent(".demo-staging-" + UUID().uuidString, isDirectory: true)
        try manager.createDirectory(at: staging, withIntermediateDirectories: true)
        defer { try? manager.removeItem(at: staging) }
        for track in catalog.tracks {
            try Task.checkCancellation()
            let name = URL(fileURLWithPath: track.resource)
            guard name.lastPathComponent == track.resource,
                  let resource = Bundle.main.url(forResource: name.deletingPathExtension().lastPathComponent, withExtension: name.pathExtension) else {
                throw CustomError.genericError("A bundled demo recording is missing.")
            }
            let file = try fileURL(track.relativePath, under: staging)
            try manager.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
            try manager.copyItem(at: resource, to: file)
            guard try file.resourceValues(forKeys: [.fileSizeKey]).fileSize.map(Int64.init) == track.bytes else {
                throw CustomError.genericError("A bundled demo recording is incomplete.")
            }
        }
        try catalogData.write(to: staging.appendingPathComponent(markerName), options: .atomic)
        // A single same-volume rename makes the complete source visible.
        try manager.moveItem(at: staging, to: destination)
        return destination
    }

    nonisolated private static func fileURL(_ relativePath: String, under root: URL) throws -> URL {
        let components = relativePath.split(separator: "/", omittingEmptySubsequences: false)
        guard !components.isEmpty, components.allSatisfy({ !$0.isEmpty && $0 != "." && $0 != ".." }), !relativePath.hasPrefix("/") else {
            throw CustomError.genericError("The demo catalog contains an invalid file path.")
        }
        return root.appendingPathComponent(relativePath)
    }
}
