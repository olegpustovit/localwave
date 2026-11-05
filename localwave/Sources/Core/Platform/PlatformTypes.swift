//
//  PlatformTypes.swift
//  localwave
//
//  Created by Claude Code on 04.10.2025.
//

import Foundation
import LocalWaveDomain

public enum LocalWavePreferences {
    public static let current = store(testID: ProcessInfo.processInfo.environment["LOCALWAVE_UI_TEST_ID"])

    static func store(testID: String?) -> UserDefaults {
        #if DEBUG
        if let testID, let id = UUID(uuidString: testID) {
            guard let defaults = UserDefaults(suiteName: "com.olegpustovit.localwave.ui-test." + id.uuidString) else {
                preconditionFailure("UI tests require isolated preferences")
            }
            return defaults
        }
        #endif
        return .standard
    }
}

private let offlineCacheLock = NSRecursiveLock()

public func withOfflineCacheLock<T>(_ operation: () throws -> T) rethrows -> T {
    offlineCacheLock.lock()
    defer { offlineCacheLock.unlock() }
    return try operation()
}

public var sourceBookmarkCreationOptions: URL.BookmarkCreationOptions {
    #if os(macOS)
    return .withSecurityScope
    #else
    return []
    #endif
}

public var sourceBookmarkResolutionOptions: URL.BookmarkResolutionOptions {
    #if os(macOS)
    return .withSecurityScope
    #else
    return []
    #endif
}

#if canImport(UIKit)
import UIKit
public typealias PlatformImage = UIImage
@MainActor public func dismissTextInput() {
    UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
}
#elseif canImport(AppKit)
import AppKit
public typealias PlatformImage = NSImage
@MainActor public func dismissTextInput() {
    NSApp.keyWindow?.makeFirstResponder(nil)
}
#else
// Fallback for platforms without image support
public struct PlatformImage {
    public let data: Data?

    public init?(contentsOfFile path: String) {
        guard let data = try? Data(contentsOf: URL(fileURLWithPath: path)) else {
            return nil
        }
        self.data = data
    }

    public init?(data: Data) {
        self.data = data
    }
}
#endif

// MARK: - Cover Art Helpers

/// Platform-agnostic cover art loading
public func loadCoverArt(for song: Song) -> PlatformImage? {
    guard let coverArtPath = song.coverArtPath else { return nil }

    let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
    let coverURL = docs.appendingPathComponent(coverArtPath)

    #if canImport(UIKit)
    return UIImage(contentsOfFile: coverURL.path)
    #elseif canImport(AppKit)
    return NSImage(contentsOfFile: coverURL.path)
    #else
    return PlatformImage(contentsOfFile: coverURL.path)
    #endif
}

#if canImport(UIKit)
/// iOS-specific cover art function (compatibility with existing code)
public func coverArt(of song: Song) -> UIImage? {
    return loadCoverArt(for: song)
}
#endif

// MARK: - Audio Player Factory

/// Creates the appropriate audio player for the current platform
@MainActor
public func createAudioPlayer() -> AudioPlayerProtocol {
    #if canImport(AVFoundation) && canImport(UIKit)
    return AVAudioPlayerAdapter()
    #elseif os(macOS)
    return CLIAudioPlayerAdapter()
    #else
    fatalError("No audio player implementation available for this platform")
    #endif
}
