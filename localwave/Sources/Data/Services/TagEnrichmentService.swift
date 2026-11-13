import Foundation
import LocalWaveCore
import LocalWaveDomain

/// Background tag reading for cloud-only placeholder songs. Placeholders (no duration yet) are the persisted queue,
/// so it resumes after relaunch. Parsing itself is DefaultSongImportService.enrichCloudPaths.
public actor TagEnrichmentService {
    private let songRepo: SongRepository
    private let sourcePathRepo: SourcePathRepository
    private let importer: DefaultSongImportService
    private let sourceAccess: SourceAccess
    private let onProgress: @Sendable (_ done: Int, _ total: Int) async -> Void
    private let logger = createLogger(subsystem: subsystem, category: "TagEnrichment")
    private var running = false
    private var passActive = false
    private var pausedUntil = Date.distantPast
    private var attempted = Set<Int64>() // retried next launch, not every cycle
    private let chunkSize = 25

    public init(songRepo: SongRepository, sourcePathRepo: SourcePathRepository, importer: DefaultSongImportService, sourceAccess: SourceAccess,
                onProgress: @escaping @Sendable (_ done: Int, _ total: Int) async -> Void) {
        self.songRepo = songRepo
        self.sourcePathRepo = sourcePathRepo
        self.importer = importer
        self.sourceAccess = sourceAccess
        self.onProgress = onProgress
    }

    public func start() {
        guard !running else { return }
        running = true
        Task(priority: .background) { // never compete with scrolling for CPU/disk
            while running {
                await runOnce()
                do { try await Task.sleep(nanoseconds: 60 * 1_000_000_000) } catch { running = false }
            }
        }
    }

    /// The user opened a track: stop downloading ahead for the next 45 s so their file gets the disk and network.
    /// shortcut: cancels whichever import is running if a pass is active (a user import started in the same instant would be cancelled too); upgrade with a per-pass task handle if that ever shows up.
    public func yieldToUser() async {
        pausedUntil = Date().addingTimeInterval(45)
        if passActive { await importer.cancelImport() }
    }

    /// Whether the machine can afford a background download pass right now.
    private var mayRun: Bool {
        if ProcessInfo.processInfo.isLowPowerModeEnabled { return false }
        let free = (try? URL(fileURLWithPath: NSHomeDirectory()).resourceValues(forKeys: [.volumeAvailableCapacityForImportantUsageKey]))?
            .volumeAvailableCapacityForImportantUsage ?? .max
        return free > 5_000_000_000
    }

    func runOnce() async {
        guard Date() >= pausedUntil, mayRun, !(await importer.isBusy) else { return }
        var paths: [SourcePath] = []
        var songIDByPath: [Int64: Int64] = [:]
        for song in await songRepo.getSongsMissingMetadata() {
            guard let id = song.id, !attempted.contains(id), let sourceId = song.sourceId, let relative = song.relativePath,
                  let path = try? await sourcePathRepo.getByPath(relativePath: relative, sourceId: sourceId), path.isAudio else { continue }
            paths.append(path)
            songIDByPath[path.pathId] = id
        }
        guard !paths.isEmpty else { return }
        paths.sort { $0.relativePath.localizedStandardCompare($1.relativePath) == .orderedAscending } // finish albums together
        logger.debug("reading tags for \(paths.count) cloud songs")

        // Downloading and evicting files flips the folder fingerprint; without this the watcher rescans and cancels us.
        let sourceIDs = Set(paths.map(\.sourceId))
        for id in sourceIDs { await sourceAccess.beginScan(sourceId: id) }
        passActive = true
        defer { passActive = false }
        var done = 0
        var failures = 0
        chunks: for start in stride(from: 0, to: paths.count, by: chunkSize) {
            let chunk = Array(paths[start..<min(start + chunkSize, paths.count)])
            let base = done
            do {
                try await importer.enrichCloudPaths(paths: chunk) { [onProgress, total = paths.count] event in
                    await onProgress(base + (event.completedFiles ?? 0), total)
                }
            } catch is CancellationError {
                break chunks // a user import took over; retry next cycle
            } catch {
                // One bad file must not sink the chunk: retry each alone to isolate it.
                for path in chunk {
                    do { try await importer.enrichCloudPaths(paths: [path], onProgress: nil) }
                    catch is CancellationError { break chunks }
                    catch { failures += 1; logger.error("tags failed for \(path.relativePath): \(error)") }
                }
            }
            chunk.forEach { path in songIDByPath[path.pathId].map { _ = attempted.insert($0) } }
            done += chunk.count
            await onProgress(done, paths.count)
        }
        for id in sourceIDs { await sourceAccess.endScan(sourceId: id) }
        await onProgress(0, 0)
        NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
        if failures > 0 {
            NotificationCenter.default.post(name: Notification.Name("LWShowError"), object: "Couldn’t read tags for \(failures) song\(failures == 1 ? "" : "s"). They will be retried next launch.")
        }
    }
}
