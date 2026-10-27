//
//  TUIColors.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI

/// Color definitions for TUI theming
///
/// Note: SwiftTUI does not currently support color rendering in the terminal.
/// These color definitions serve as:
/// 1. Documentation of the intended color scheme matching the iOS app
/// 2. Future-proofing for when color support is added
/// 3. Semantic naming for UI elements
///
/// The iOS app uses a dark theme with purple accent color.
public struct TUIColors {

    // MARK: - Primary Colors

    /// Primary accent color (purple) - matches iOS app
    /// iOS: .purple
    public static let accent = TUIColorDefinition(
        name: "Accent Purple",
        description: "Primary brand color for highlights and selection"
    )

    /// Background color (dark) - matches iOS dark mode
    /// iOS: dark mode background
    public static let background = TUIColorDefinition(
        name: "Background Dark",
        description: "Main background color"
    )

    /// Primary text color
    /// iOS: .primary (white in dark mode)
    public static let textPrimary = TUIColorDefinition(
        name: "Text Primary",
        description: "Main text color"
    )

    /// Secondary text color (dimmed)
    /// iOS: .secondary (gray in dark mode)
    public static let textSecondary = TUIColorDefinition(
        name: "Text Secondary",
        description: "Secondary text, metadata, timestamps"
    )

    // MARK: - UI Element Colors

    /// Selection indicator color
    public static let selection = TUIColorDefinition(
        name: "Selection",
        description: "Currently selected item indicator"
    )

    /// Tab bar active color
    public static let tabActive = accent

    /// Tab bar inactive color
    public static let tabInactive = textSecondary

    /// Border color for boxes and frames
    public static let border = TUIColorDefinition(
        name: "Border",
        description: "Box borders and dividers"
    )

    // MARK: - Status Colors

    /// Success/playing status color (green)
    public static let success = TUIColorDefinition(
        name: "Success Green",
        description: "Success messages, playing status"
    )

    /// Error status color (red)
    public static let error = TUIColorDefinition(
        name: "Error Red",
        description: "Error messages and alerts"
    )

    /// Warning status color (yellow/orange)
    public static let warning = TUIColorDefinition(
        name: "Warning Yellow",
        description: "Warning messages"
    )

    /// Info status color (blue)
    public static let info = TUIColorDefinition(
        name: "Info Blue",
        description: "Informational messages"
    )

    // MARK: - Progress Colors

    /// Progress bar filled portion
    public static let progressFilled = accent

    /// Progress bar empty portion
    public static let progressEmpty = TUIColorDefinition(
        name: "Progress Empty",
        description: "Unfilled portion of progress bars"
    )

    // MARK: - Player Colors

    /// Now playing indicator
    public static let nowPlaying = accent

    /// Paused state indicator
    public static let paused = textSecondary

    /// Stopped state indicator
    public static let stopped = textSecondary

    // MARK: - Text Styles (for reference)

    /// Semantic text style descriptions
    public struct TextStyles {
        public static let title = "Bold, primary color"
        public static let subtitle = "Regular, secondary color"
        public static let body = "Regular, primary color"
        public static let caption = "Small, secondary color"
        public static let metadata = "Small, secondary color, italic"
    }

    // MARK: - Visual Indicators (Character-based)

    /// Since TUI doesn't support colors, we use these character-based indicators
    public struct Indicators {
        /// Active/selected state
        public static func selected(_ text: String) -> String {
            TUITheme.Icons.selected + " " + text
        }

        /// Unselected state
        public static func unselected(_ text: String) -> String {
            TUITheme.Icons.unselected + " " + text
        }

        /// Playing indicator
        public static func playing(_ text: String) -> String {
            TUITheme.Icons.play + " " + text
        }

        /// Paused indicator
        public static func paused(_ text: String) -> String {
            TUITheme.Icons.pause + " " + text
        }

        /// Error indicator
        public static func error(_ text: String) -> String {
            TUITheme.Icons.error + " " + text
        }

        /// Success indicator
        public static func success(_ text: String) -> String {
            TUITheme.Icons.success + " " + text
        }

        /// Warning indicator
        public static func warning(_ text: String) -> String {
            TUITheme.Icons.warning + " " + text
        }

        /// Info indicator
        public static func info(_ text: String) -> String {
            TUITheme.Icons.info + " " + text
        }
    }
}

/// Color definition for documentation purposes
public struct TUIColorDefinition {
    public let name: String
    public let description: String

    public init(name: String, description: String) {
        self.name = name
        self.description = description
    }
}

// MARK: - SwiftUI Color Mapping (for reference)

#if canImport(SwiftUI)
import SwiftUI

extension TUIColors {
    /// Map TUI color to SwiftUI Color for iOS app
    /// This is for reference only - TUI doesn't use SwiftUI colors
    public static func toSwiftUIColor(_ tuiColor: TUIColorDefinition) -> SwiftUI.Color {
        switch tuiColor.name {
        case "Accent Purple":
            return .purple
        case "Background Dark":
            #if os(macOS)
            return SwiftUI.Color(nsColor: .windowBackgroundColor)
            #else
            return SwiftUI.Color(uiColor: .systemBackground)
            #endif
        case "Text Primary":
            return .primary
        case "Text Secondary":
            return .secondary
        case "Success Green":
            return .green
        case "Error Red":
            return .red
        case "Warning Yellow":
            return .orange
        case "Info Blue":
            return .blue
        default:
            return .primary
        }
    }
}
#endif
