import SwiftUI
import UIKit
import LocalWaveCore
import LocalWaveData
import LocalWaveDomain

struct LWPreparedShare: Identifiable {
    let id = UUID()
    let urls: [URL]
    let directory: URL
}

@MainActor
enum LWSourceFileSharing {
    static func prepare(songs: [Song], dependencies: DependencyContainer) async throws -> LWPreparedShare {
        guard !songs.isEmpty else { throw CustomError.genericError("No songs were selected to share.") }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("LocalWave Share", isDirectory: true).appendingPathComponent(UUID().uuidString, isDirectory: true)
        do {
            var urls: [URL] = []
            for (index, song) in songs.enumerated() {
                try Task.checkCancellation()
                let source: URL
                let scope: URL
                if let local = readableLocalURL(song) { source = local; scope = local }
                else if let access = try await dependencies.resolveSourceAccess(for: song) { source = access.file; scope = access.root }
                else if let bookmark = song.bookmark {
                    var stale = false
                    source = try URL(resolvingBookmarkData: bookmark, options: sourceBookmarkResolutionOptions.union(.withoutUI), relativeTo: nil, bookmarkDataIsStale: &stale)
                    scope = source
                } else { throw CustomError.genericError("This file is unavailable. Reconnect its folder, then try sharing again.") }
                let destination = songs.count == 1 ? directory : directory.appendingPathComponent(String(index), isDirectory: true)
                let proposedName = URL(fileURLWithPath: song.relativePath ?? source.lastPathComponent).lastPathComponent
                let originalName = ["", ".", ".."].contains(proposedName) ? source.lastPathComponent : proposedName
                let url = try await Task.detached(priority: .userInitiated) {
                    let granted = scope.startAccessingSecurityScopedResource()
                    defer { if granted { scope.stopAccessingSecurityScopedResource() } }
                    try FileAvailabilityService.requireLocalForNetworkSimulation(source)
                    let copied = try BackgroundFileService.copyCoordinated(source, into: destination)
                    let named = destination.appendingPathComponent(originalName)
                    if copied != named { try FileManager.default.moveItem(at: copied, to: named) }
                    return named
                }.value
                try Task.checkCancellation()
                urls.append(url)
            }
            return LWPreparedShare(urls: urls, directory: directory)
        } catch {
            try? FileManager.default.removeItem(at: directory)
            throw error
        }
    }

    static func cleanup(_ share: LWPreparedShare) { try? FileManager.default.removeItem(at: share.directory) }

    private static func readableLocalURL(_ song: Song) -> URL? {
        guard let path = song.localFilePath else { return nil }
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let url = path.hasPrefix("/") ? URL(fileURLWithPath: path) : documents.appendingPathComponent(path)
        return FileManager.default.isReadableFile(atPath: url.path) ? url : nil
    }
}

struct LWShareSheet: UIViewControllerRepresentable {
    let urls: [URL]
    func makeUIViewController(context: Context) -> UIActivityViewController { UIActivityViewController(activityItems: urls, applicationActivities: nil) }
    func updateUIViewController(_ controller: UIActivityViewController, context: Context) { }
}
