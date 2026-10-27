//
//  TUIProgressBar.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI

/// Progress bar component using Unicode block characters
///
/// Example usage:
/// ```swift
/// TUIProgressBar(
///     value: 0.65,
///     width: 40,
///     label: "Scanning: 65/100 files"
/// )
/// ```
public struct TUIProgressBar: View {
    let value: Double  // 0.0 to 1.0
    let width: Int
    let label: String?
    let showPercentage: Bool

    public init(
        value: Double,
        width: Int = 40,
        label: String? = nil,
        showPercentage: Bool = true
    ) {
        self.value = max(0.0, min(1.0, value))  // Clamp to 0-1
        self.width = width
        self.label = label
        self.showPercentage = showPercentage
    }

    public var body: some View {
        VStack(spacing: 0) {
            if let label = label {
                HStack {
                    Text(label)
                    Spacer()
                }
            }

            HStack {
                Text(progressBarString)
                if showPercentage {
                    Text(" \(Int(value * 100))%")
                }
                Spacer()
            }
        }
    }

    private var progressBarString: String {
        // Unicode block characters for smooth progress
        let blocks = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"]
        let filled = "█"
        let empty = " "

        let totalWidth = width
        let filledWidth = value * Double(totalWidth)

        let fullBlocks = Int(filledWidth)
        let partialBlock = filledWidth - Double(fullBlocks)
        let partialIndex = Int(partialBlock * 8.0)

        var bar = ""

        // Full blocks
        bar += String(repeating: filled, count: fullBlocks)

        // Partial block
        if fullBlocks < totalWidth && partialIndex > 0 {
            bar += blocks[partialIndex]
        }

        // Empty space
        let remaining = totalWidth - fullBlocks - (partialIndex > 0 ? 1 : 0)
        bar += String(repeating: empty, count: max(0, remaining))

        return "[" + bar + "]"
    }
}

/// Indeterminate progress indicator (spinner)
///
/// Example usage:
/// ```swift
/// TUISpinner(
///     frame: spinnerFrame,
///     label: "Loading..."
/// )
/// ```
public struct TUISpinner: View {
    let frame: Int
    let label: String?

    private let spinnerChars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

    public init(frame: Int = 0, label: String? = nil) {
        self.frame = frame
        self.label = label
    }

    public var body: some View {
        HStack {
            Text(spinnerChars[frame % spinnerChars.count])
            if let label = label {
                Text(" ")
                Text(label)
            }
            Spacer()
        }
    }
}

/// Simple vertical progress meter
///
/// Example usage:
/// ```swift
/// TUIVerticalMeter(
///     value: 0.75,
///     height: 10,
///     label: "Volume"
/// )
/// ```
public struct TUIVerticalMeter: View {
    let value: Double  // 0.0 to 1.0
    let height: Int
    let label: String?

    public init(
        value: Double,
        height: Int = 10,
        label: String? = nil
    ) {
        self.value = max(0.0, min(1.0, value))
        self.height = height
        self.label = label
    }

    public var body: some View {
        HStack {
            if let label = label {
                Text(label)
                Text(" ")
            }

            VStack(spacing: 0) {
                let filledHeight = Int(value * Double(height))
                let emptyHeight = height - filledHeight

                // Empty blocks on top
                if emptyHeight > 0 {
                    ForEach(0..<emptyHeight, id: \.self) { _ in
                        Text("░")
                    }
                }

                // Filled blocks on bottom
                if filledHeight > 0 {
                    ForEach(0..<filledHeight, id: \.self) { _ in
                        Text("█")
                    }
                }
            }

            Spacer()
        }
    }
}

/// Progress with status text (multi-line)
///
/// Example usage:
/// ```swift
/// TUIProgressWithStatus(
///     progress: 0.45,
///     status: "Processing file 45/100",
///     detail: "/Music/Artist/Album/Song.mp3"
/// )
/// ```
public struct TUIProgressWithStatus: View {
    let progress: Double
    let status: String
    let detail: String?

    public init(
        progress: Double,
        status: String,
        detail: String? = nil
    ) {
        self.progress = progress
        self.status = status
        self.detail = detail
    }

    public var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(status)
                Spacer()
            }

            TUIProgressBar(value: progress, width: 60, showPercentage: true)

            if let detail = detail {
                Text("")
                HStack {
                    Text("  \(detail)")
                    Spacer()
                }
            }
        }
    }
}
