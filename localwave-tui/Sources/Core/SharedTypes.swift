//
//  SharedTypes.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import Foundation

// MARK: - Platform Bridging for TUI

/// This file provides TUI-specific type aliases for SwiftTUI.
/// Platform-agnostic types are now in Shared_Core/Platform/

#if canImport(SwiftTUI)
import SwiftTUI
public typealias TUIColor = SwiftTUI.Color
#endif

// Note: All other platform types (PlatformImage, RepeatMode, etc.) are now
// defined in the shared Core/Platform/ directory and accessed via symlink.

// MARK: - Terminal Size Helper

/// Helper to calculate available height for lists based on UI chrome
public struct TerminalSizeHelper {
    /// Calculate available height for a list given terminal height and reserved lines
    ///
    /// Example usage:
    /// ```swift
    /// // Reserve space for: tab bar (1), header (1), mini player (2), divider (1)
    /// let availableHeight = TerminalSizeHelper.calculateListHeight(
    ///     terminalHeight: 24,
    ///     reservedLines: 5
    /// )
    /// listState.updateAvailableHeight(terminalHeight: 24, reservedLines: 5)
    /// ```
    public static func calculateListHeight(terminalHeight: Int, reservedLines: Int) -> Int {
        return max(1, terminalHeight - reservedLines)
    }

    /// Standard reserved lines for common UI layouts
    public struct ReservedLines {
        /// Tab bar (1) + mini player (2) = 3 lines
        public static let mainView = 3

        /// Main view + header (1) + divider (1) = 5 lines
        public static let listView = 5

        /// Main view + table header (2) = 5 lines
        public static let tableView = 5

        /// Main view + full player header (3) = 6 lines
        public static let playerQueue = 6
    }
}
