//
//  TUIMockAudioPlayer.swift
//  localwave-tui
//
//  Created by Claude Code on 05.10.2025.
//

import Foundation
import LocalWaveCore

/// Mock audio player for TUI
/// Note: This is a placeholder for Phase 4. Real audio playback in TUI would require
/// either AVFoundation (macOS only) or external CLI audio players (mpv, ffplay, etc.)
@MainActor
class TUIMockAudioPlayer: AudioPlayerProtocol {
    var isPlaying: Bool = false
    var currentTime: TimeInterval = 0
    var duration: TimeInterval = 0
    var volume: Float = 1.0
    var rate: Float = 1.0
    var url: URL?
    weak var delegate: AudioPlayerDelegate?

    func prepareToPlay(url: URL) throws {
        self.url = url
        self.duration = 180.0 // Mock 3-minute song
        self.currentTime = 0
    }

    func play() {
        isPlaying = true
        // In a real implementation, this would start audio playback
    }

    func pause() {
        isPlaying = false
    }

    func stop() {
        isPlaying = false
        currentTime = 0
    }
}
