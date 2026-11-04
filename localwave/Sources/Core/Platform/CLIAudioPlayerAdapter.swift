//
//  CLIAudioPlayerAdapter.swift
//  localwave
//
//  Created by Claude Code on 04.10.2025.
//

#if os(macOS) && !canImport(UIKit)
import Foundation
import os

/// macOS CLI implementation of AudioPlayerProtocol using afplay
@MainActor
public class CLIAudioPlayerAdapter: AudioPlayerProtocol {
    private var process: Process?
    private var playbackTimer: Timer?
    private var startTime: Date?
    private var pausedTime: TimeInterval = 0
    private var cachedDuration: TimeInterval = 0
    private var currentURL: URL?
    private var currentPlayingProcess: Process?  // Track which process should trigger delegate
    private let logger = createLogger(subsystem: subsystem, category: "CLIAudioPlayerAdapter")

    public weak var delegate: AudioPlayerDelegate?

    // MARK: - Initialization

    public init() {}

    // MARK: - AudioPlayerProtocol Properties

    public private(set) var isPlaying: Bool = false

    public var currentTime: TimeInterval {
        get {
            if isPlaying, let startTime = startTime {
                return Date().timeIntervalSince(startTime) + pausedTime
            }
            return pausedTime
        }
        set {
            pausedTime = newValue
            if isPlaying {
                // Restart playback from new position
                let wasPlaying = isPlaying
                stop()
                if wasPlaying, let url = currentURL {
                    try? prepareToPlay(url: url)
                    // Seek by skipping forward (afplay doesn't support seeking directly)
                    // For now, we restart from beginning
                    play()
                }
            }
        }
    }

    public private(set) var duration: TimeInterval = 0

    public var volume: Float = 1.0 {
        didSet {
            // afplay doesn't support volume control via CLI
            // Volume would need to be controlled via system audio
            logger.debug("Volume set to \(self.volume) (system-level only)")
        }
    }

    public var rate: Float {
        return isPlaying ? 1.0 : 0.0
    }

    public var url: URL? {
        return currentURL
    }

    // MARK: - AudioPlayerProtocol Methods

    public func prepareToPlay(url: URL) throws {
        logger.debug("Preparing to play: \(url.lastPathComponent)")

        guard FileManager.default.fileExists(atPath: url.path) else {
            logger.error("File not found: \(url.path)")
            throw AudioPlayerError.fileNotFound
        }

        currentURL = url
        cachedDuration = getDuration(for: url)
        duration = cachedDuration
        pausedTime = 0

        logger.debug("Player prepared, duration: \(self.duration)s")
    }

    public func play() {
        guard let url = currentURL else {
            logger.warning("Attempted to play but no URL loaded")
            return
        }

        // If already playing, do nothing
        if isPlaying, let existingProcess = process, existingProcess.isRunning {
            logger.debug("Already playing, ignoring play() call")
            return
        }

        // Check if we're resuming from pause
        let isResuming = pausedTime > 0

        // Stop any existing playback
        if let existingProcess = process, existingProcess.isRunning {
            existingProcess.terminate()
            // Don't wait for termination - continue immediately
        }

        logger.debug("Starting playback with afplay (resuming: \(isResuming))")

        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/afplay")
        task.arguments = [url.path]

        // Set up termination handler
        task.terminationHandler = { [weak self, weak task] process in
            Task { @MainActor [weak self] in
                guard let self = self else { return }

                // Only notify delegate if this process is still the "current" one
                let isCurrentProcess = self.currentPlayingProcess === task

                self.isPlaying = false
                self.stopPlaybackTimer()

                if isCurrentProcess {
                    if process.terminationStatus == 0 {
                        self.logger.debug("Playback finished successfully")
                        self.delegate?.audioPlayerDidFinishPlaying(self, successfully: true)
                    } else {
                        self.logger.warning("Playback terminated with status: \(process.terminationStatus)")
                        self.delegate?.audioPlayerDidFinishPlaying(self, successfully: false)
                    }
                    self.currentPlayingProcess = nil
                } else {
                    self.logger.debug("Old process terminated, ignoring")
                }
            }
        }

        do {
            try task.run()
            process = task
            currentPlayingProcess = task  // Mark this as the process that should trigger delegate
            isPlaying = true

            // Only reset pausedTime if this is NOT a resume
            if !isResuming {
                pausedTime = 0
            }

            startTime = Date()
            startPlaybackTimer()
            logger.debug("Playback started, startTime: \(self.startTime!), pausedTime: \(self.pausedTime)")
        } catch {
            logger.error("Failed to start afplay: \(error)")
        }
    }

    public func pause() {
        guard let process = process, isPlaying else {
            logger.warning("Attempted to pause but not playing")
            return
        }

        // afplay doesn't support pause, so we terminate and track position
        let currentPausedTime = currentTime
        pausedTime = currentPausedTime
        currentPlayingProcess = nil  // Clear so termination handler won't trigger delegate
        process.terminate()
        isPlaying = false
        stopPlaybackTimer()
        self.process = nil

        logger.debug("Playback paused at \(currentPausedTime)s")
    }

    public func stop() {
        guard let process = process else { return }

        currentPlayingProcess = nil  // Clear so termination handler won't trigger delegate
        process.terminate()
        isPlaying = false
        pausedTime = 0
        startTime = nil
        stopPlaybackTimer()
        self.process = nil

        logger.debug("Playback stopped")
    }

    // MARK: - Private Helpers

    private func startPlaybackTimer() {
        playbackTimer = Timer.scheduledTimer(
            withTimeInterval: 0.1,
            repeats: true
        ) { [weak self] _ in
            Task { @MainActor in
                guard let self = self else { return }
                // Timer is just for updating UI, not for detecting end of playback
                // End of playback is detected via afplay process termination
            }
        }
        RunLoop.main.add(playbackTimer!, forMode: .common)
    }

    private func stopPlaybackTimer() {
        playbackTimer?.invalidate()
        playbackTimer = nil
    }

    private func getDuration(for url: URL) -> TimeInterval {
        // Use afinfo to get duration
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/afinfo")
        task.arguments = [url.path]

        let pipe = Pipe()
        task.standardOutput = pipe

        do {
            try task.run()
            task.waitUntilExit()

            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            if let output = String(data: data, encoding: .utf8) {
                // Parse duration from afinfo output
                // Format: "estimated duration: 234.567890 sec"
                if let range = output.range(of: "estimated duration: "),
                   let endRange = output.range(of: " sec", range: range.upperBound..<output.endIndex)
                {
                    let durationString = output[range.upperBound..<endRange.lowerBound]
                    if let duration = Double(durationString) {
                        return duration
                    }
                }
            }
        } catch {
            logger.error("Failed to get duration: \(error)")
        }

        return 0
    }

    deinit {
        // Cleanup - stop playback if needed
        process?.terminate()
        playbackTimer?.invalidate()
    }
}
#endif
