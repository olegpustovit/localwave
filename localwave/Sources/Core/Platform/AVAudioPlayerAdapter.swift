//
//  AVAudioPlayerAdapter.swift
//  localwave
//
//  Created by Claude Code on 04.10.2025.
//

#if canImport(AVFoundation)
import AVFoundation
import Foundation
import os

/// AVAudioPlayer implementation of AudioPlayerProtocol (iOS and macOS)
@MainActor
public class AVAudioPlayerAdapter: NSObject, AudioPlayerProtocol, @preconcurrency AVAudioPlayerDelegate {
    private var player: AVAudioPlayer?
    private let logger = createLogger(subsystem: subsystem, category: "AVAudioPlayerAdapter")

    public weak var delegate: AudioPlayerDelegate?

    // MARK: - AudioPlayerProtocol Properties

    public var isPlaying: Bool {
        return player?.isPlaying ?? false
    }

    public var currentTime: TimeInterval {
        get { player?.currentTime ?? 0 }
        set { player?.currentTime = newValue }
    }

    public var duration: TimeInterval {
        return player?.duration ?? 0
    }

    public var volume: Float {
        get { player?.volume ?? 1.0 }
        set { player?.volume = newValue }
    }

    public var rate: Float {
        return player?.rate ?? 0
    }

    public var url: URL? {
        return player?.url
    }

    // MARK: - Initialization

    public override init() {
        super.init()
        setupAudioSession()
    }

    // MARK: - AudioPlayerProtocol Methods

    public func prepareToPlay(url: URL) throws {
        logger.debug("Preparing to play: \(url.lastPathComponent)")

        do {
            let audioPlayer = try AVAudioPlayer(contentsOf: url)
            audioPlayer.delegate = self
            player = audioPlayer
            logger.debug("Player prepared successfully")
        } catch {
            logger.error("Failed to create AVAudioPlayer: \(error)")
            throw AudioPlayerError.initializationFailed(error.localizedDescription)
        }
    }

    public func play() {
        guard let player = player else {
            logger.warning("Attempted to play but player is nil")
            return
        }

        let currentTime = player.currentTime
        player.play()
        logger.debug("Playback started, currentTime: \(currentTime), isPlaying: \(player.isPlaying)")
    }

    public func pause() {
        guard let player = player else {
            logger.warning("Attempted to pause but player is nil")
            return
        }

        player.pause()
        logger.debug("Playback paused")
    }

    public func stop() {
        guard let player = player else {
            // No player to stop, this is fine
            return
        }

        player.stop()
        player.currentTime = 0
        logger.debug("Playback stopped")
    }

    // MARK: - AVAudioPlayerDelegate

    nonisolated public func audioPlayerDidFinishPlaying(
        _ player: AVAudioPlayer,
        successfully flag: Bool
    ) {
        Task { @MainActor in
            logger.debug("Playback finished, success: \(flag)")
            delegate?.audioPlayerDidFinishPlaying(self, successfully: flag)
        }
    }

    nonisolated public func audioPlayerDecodeErrorDidOccur(
        _ player: AVAudioPlayer,
        error: Error?
    ) {
        Task { @MainActor in
            if let error = error {
                logger.error("Decode error: \(error)")
            }
        }
    }

    // MARK: - Audio Session Setup

    private func setupAudioSession() {
        #if canImport(UIKit)
        // iOS requires audio session configuration
        do {
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
            try AVAudioSession.sharedInstance().setActive(true)
            logger.debug("Audio session configured")
        } catch {
            logger.error("Audio session setup error: \(error)")
        }
        #else
        // macOS doesn't need audio session setup
        logger.debug("macOS - no audio session setup needed")
        #endif
    }
}
#endif
