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
    private let audioSession: AudioSessionController
    private var activationTask: Task<Void, Never>?
    private var sessionEvents: Task<Void, Never>?
    private var playbackGeneration = UUID()
    private var resumeAfterInterruption = false
    private var wantsPlayback = false
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
        audioSession = .shared
        super.init()
        observeAudioSession()
    }

    init(audioSession: AudioSessionController) {
        self.audioSession = audioSession
        super.init()
        observeAudioSession()
    }

    private func observeAudioSession() {
        sessionEvents = Task { [weak self, audioSession] in
            let stream = await audioSession.events()
            for await event in stream {
                guard let self else { break }
                switch event {
                case .interruptionBegan:
                    let shouldResume = self.isPlaying || self.wantsPlayback
                    self.pause()
                    self.resumeAfterInterruption = shouldResume
                case let .interruptionEnded(shouldResume):
                    let resume = shouldResume && self.resumeAfterInterruption
                    self.resumeAfterInterruption = false
                    if resume { self.play() }
                case .oldOutputUnavailable:
                    self.pause()
                }
            }
        }
    }

    // MARK: - AudioPlayerProtocol Methods

    public func prepareToPlay(url: URL) throws {
        logger.debug("Preparing to play: \(url.lastPathComponent)")

        stop()
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
        guard let player else {
            logger.warning("Attempted to play but player is nil")
            return
        }
        activationTask?.cancel()
        playbackGeneration = UUID()
        let generation = playbackGeneration
        wantsPlayback = true
        activationTask = Task { [weak self, audioSession] in
            do {
                try await audioSession.activate()
                guard let self, !Task.isCancelled, self.playbackGeneration == generation,
                      self.wantsPlayback, self.player === player else { return }
                self.activationTask = nil
                guard player.play() else {
                    self.wantsPlayback = false
                    self.logger.error("The prepared audio file could not start playback")
                    return
                }
                self.logger.debug("Playback started")
            } catch is CancellationError { }
            catch {
                guard let self, self.playbackGeneration == generation else { return }
                self.activationTask = nil
                self.wantsPlayback = false
                self.logger.error("Audio session activation failed: \(error)")
            }
        }
    }

    public func pause() {
        cancelActivation()
        player?.pause()
    }

    public func stop() {
        cancelActivation()
        player?.stop()
        player?.currentTime = 0
    }

    private func cancelActivation() {
        resumeAfterInterruption = false
        playbackGeneration = UUID()
        activationTask?.cancel()
        activationTask = nil
        wantsPlayback = false
    }

    // MARK: - AVAudioPlayerDelegate

    nonisolated public func audioPlayerDidFinishPlaying(
        _ player: AVAudioPlayer,
        successfully flag: Bool
    ) {
        Task { @MainActor in
            guard self.player === player else { return }
            wantsPlayback = false
            logger.debug("Playback finished, success: \(flag)")
            delegate?.audioPlayerDidFinishPlaying(self, successfully: flag)
        }
    }

    nonisolated public func audioPlayerDecodeErrorDidOccur(
        _ player: AVAudioPlayer,
        error: Error?
    ) {
        Task { @MainActor in
            guard self.player === player else { return }
            pause()
            if let error = error {
                logger.error("Decode error: \(error)")
            }
        }
    }

    deinit {
        activationTask?.cancel()
        sessionEvents?.cancel()
    }
}
#endif
