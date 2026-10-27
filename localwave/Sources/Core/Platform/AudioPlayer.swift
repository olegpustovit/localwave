//
//  AudioPlayer.swift
//  localwave
//
//  Created by Claude Code on 04.10.2025.
//

import Foundation

// MARK: - Audio Player Protocol

/// Platform-agnostic audio player protocol.
/// Allows PlayerViewModel to work with both AVAudioPlayer (iOS) and CLI-based players (TUI).
@MainActor
public protocol AudioPlayerProtocol: AnyObject {
    /// Whether the player is currently playing
    var isPlaying: Bool { get }

    /// Current playback time in seconds
    var currentTime: TimeInterval { get set }

    /// Total duration of the current track in seconds
    var duration: TimeInterval { get }

    /// Playback volume (0.0 to 1.0)
    var volume: Float { get set }

    /// Playback rate (1.0 = normal speed)
    var rate: Float { get }

    /// The current audio file URL being played
    var url: URL? { get }

    /// Delegate to receive playback events
    var delegate: AudioPlayerDelegate? { get set }

    /// Prepare the player with an audio file
    /// - Parameter url: The URL of the audio file to load
    /// - Throws: Error if file cannot be loaded
    func prepareToPlay(url: URL) throws

    /// Start or resume playback
    func play()

    /// Pause playback
    func pause()

    /// Stop playback and reset position
    func stop()
}

// MARK: - Audio Player Delegate

/// Delegate protocol for audio player events
public protocol AudioPlayerDelegate: AnyObject {
    /// Called when audio finishes playing
    /// - Parameters:
    ///   - player: The audio player
    ///   - successfully: Whether playback completed successfully
    func audioPlayerDidFinishPlaying(_ player: AudioPlayerProtocol, successfully: Bool)
}

// MARK: - Playback Error

/// Errors that can occur during audio playback
public enum AudioPlayerError: Error, LocalizedError {
    case fileNotFound
    case invalidFormat
    case initializationFailed(String)
    case playbackFailed(String)

    public var errorDescription: String? {
        switch self {
        case .fileNotFound:
            return "Audio file not found"
        case .invalidFormat:
            return "Invalid audio file format"
        case .initializationFailed(let message):
            return "Failed to initialize player: \(message)"
        case .playbackFailed(let message):
            return "Playback failed: \(message)"
        }
    }
}
