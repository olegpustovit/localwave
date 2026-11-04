//
//  TUITheme.swift
//  localwave-tui
//
//  Created by Claude Code on 05.10.2025.
//

import SwiftTUI
import Foundation

/// Theme colors for the TUI
public struct TUITheme {
    // Base colors
    public let base: Color
    public let surface0: Color
    public let surface1: Color
    public let surface2: Color
    public let overlay0: Color
    public let overlay1: Color
    public let overlay2: Color

    // Text colors
    public let text: Color
    public let subtext0: Color
    public let subtext1: Color

    // Accent colors
    public let blue: Color
    public let lavender: Color
    public let sapphire: Color
    public let sky: Color
    public let teal: Color
    public let green: Color
    public let yellow: Color
    public let peach: Color
    public let maroon: Color
    public let red: Color
    public let mauve: Color
    public let pink: Color
    public let flamingo: Color
    public let rosewater: Color

    /// Catppuccin Latte theme (default)
    public static let catppuccinLatte = TUITheme(
        base: .hex("eff1f5"),
        surface0: .hex("ccd0da"),
        surface1: .hex("bcc0cc"),
        surface2: .hex("acb0be"),
        overlay0: .hex("9ca0b0"),
        overlay1: .hex("8c8fa1"),
        overlay2: .hex("7c7f93"),
        text: .hex("4c4f69"),
        subtext0: .hex("6c6f85"),
        subtext1: .hex("5c5f77"),
        blue: .hex("1e66f5"),
        lavender: .hex("7287fd"),
        sapphire: .hex("209fb5"),
        sky: .hex("04a5e5"),
        teal: .hex("179299"),
        green: .hex("40a02b"),
        yellow: .hex("df8e1d"),
        peach: .hex("fe640b"),
        maroon: .hex("e64553"),
        red: .hex("d20f39"),
        mauve: .hex("8839ef"),
        pink: .hex("ea76cb"),
        flamingo: .hex("dd7878"),
        rosewater: .hex("dc8a78")
    )

    /// Load theme from TOML file at ~/.config/localwave/theme.toml
    /// Falls back to Catppuccin Latte if file doesn't exist
    public static func load() -> TUITheme {
        let _ = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".config/localwave/theme.toml")

        // For now, just return default theme
        // TODO: Add TOML parsing when needed
        return .catppuccinLatte
    }

    // MARK: - Helper Methods (for backward compatibility)

    public struct Icons {
        public static let folder = "📁"
        public static let music = "🎵"
        public static let play = "▶"
        public static let pause = "⏸"
        public static let selected = "►"
        public static let unselected = " "
        public static let error = "✗"
        public static let success = "✓"
        public static let warning = "⚠"
        public static let info = "ℹ"
        public static let artist = "♫"
        public static let checkmark = "✓"
    }

    public struct Layout {
        public static let progressBarWidthWide = 60
        public static let progressBarWidthNarrow = 40
    }

    public static func divider(width: Int) -> String {
        String(repeating: "─", count: width)
    }

    public static func truncate(_ text: String, width: Int) -> String {
        guard width > 0 else { return " " }
        if text.count > width {
            return String(text.prefix(max(0, width - 1))) + "…"
        }
        return text + String(repeating: " ", count: max(0, width - text.count))
    }
}
