import AVFoundation
import CryptoKit
import os
import LocalWaveDomain
import LocalWaveCore

public actor BackgroundFileService {
    private let songRepo: SongRepository
    private let logger = createLogger(subsystem: subsystem, category: "BackgroundFileService")
    private var isRunning = false
    private let maxRetries = 3

    public init(songRepo: SongRepository) {
        self.songRepo = songRepo
        logger.debug("Initialised")
    }

    public func start() {
        logger.debug("attempting to start myself!")
        guard !isRunning else {
            logger.debug("service already running - aborting restart")
            return
        }
        isRunning = true
        logger.debug("service started successfully")

        Task {
            while isRunning {
                logger.debug("beginning processing cycle")
                await processQueue()
                logger.debug("processing cycle completed")
                try await Task.sleep(nanoseconds: 30 * 1_000_000_000) // 30 seconds
            }
        }
    }

    private func processQueue() async {
        let songs = await songRepo.getSongsNeedingCopy()
        logger.debug("Processing \(songs.count) files needing copy")

        for song in songs {
            do {
                logger.debug("attempting cop-y for song id \(song.id ?? -1)")
                let updatedSong = try await attemptFileCopy(song: song)
                _ = try await songRepo.upsertSong(updatedSong)
                logger.debug("succesfully copied song id \(song.id ?? -1)")
            } catch {
                logger.error("Failed to copy file for song \(song.id ?? -1): \(error)")
                await markFailed(song: song)
            }
        }
    }

    private func attemptFileCopy(song: Song) async throws -> Song {
        guard let bookmark = song.bookmark else {
            throw CustomError.genericError("No bookmark available")
        }

        var isStale = false
        let sourceURL = try URL(resolvingBookmarkData: bookmark, bookmarkDataIsStale: &isStale)
        guard sourceURL.startAccessingSecurityScopedResource() else {
            throw CustomError.genericError("Couldn't access security scoped resource")
        }
        defer { sourceURL.stopAccessingSecurityScopedResource() }

        // Generate unique filename using hash
        let fileData = try Data(contentsOf: sourceURL)
        let fileHash = SHA256.hash(data: fileData)
        let hashString = fileHash.compactMap { String(format: "%02hhx", $0) }.joined()
        let fileExtension = sourceURL.pathExtension
        let fileName = "\(hashString).\(fileExtension)"

        // Prepare directories
        let docsDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let musicDir = docsDir.appendingPathComponent("Music", isDirectory: true)
        try FileManager.default.createDirectory(at: musicDir, withIntermediateDirectories: true)

        // Atomic write using temporary file
        let tempFile = musicDir.appendingPathComponent(UUID().uuidString)
        let finalFile = musicDir.appendingPathComponent(fileName)

        // Cleanup if destination exists
        if FileManager.default.fileExists(atPath: finalFile.path) {
            try FileManager.default.removeItem(at: finalFile)
        }

        try fileData.write(to: tempFile)
        try FileManager.default.moveItem(at: tempFile, to: finalFile)

        return song.copyWith("Music/\(fileName)", .copied)
    }

    private func markFailed(song: Song) async {
        var updated = song
        updated.fileState = .failed
        _ = try? await songRepo.upsertSong(updated)
    }
}
