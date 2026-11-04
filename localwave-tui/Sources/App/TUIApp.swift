//
//  TUIApp.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import Foundation
import os
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData
import SwiftTUI

/// Main TUI application class that coordinates the terminal interface.
/// Follows the same architectural patterns as the iOS app but adapted for CLI.
@MainActor
class TUIApp {
    private let dependencies: TUIDependencyContainer
    private let logger: Logger

    init() throws {
        logger = Logger(subsystem: subsystem, category: "TUIApp")
        logger.info("Initializing LocalWave TUI...")

        // Initialize TUI-specific dependency container
        // Reuses all Data/Domain layer code from iOS app
        dependencies = try TUIDependencyContainer()

        logger.info("TUI initialization complete")
    }

    /// Start the TUI application.
    /// This will be the main entry point that sets up the terminal interface.
    func start() async throws {
        logger.info("Starting LocalWave TUI...")

        // Trigger app launch logic (same as iOS)
        dependencies.handleAppLaunch()

        // Launch SwiftTUI application with classic iPod/WinAmp layout
        try await Application(rootView: TUIClassicLayout(dependencies: dependencies)).start()
    }


    // MARK: - Helper Methods for Future Phases

    /// Cleanup and shutdown
    func shutdown() {
        logger.info("Shutting down LocalWave TUI...")
        // Future: cleanup resources, save state, etc.
    }
}
