//
//  TUITerminalSize.swift
//  localwave-tui
//
//  Created by Claude Code on 05.10.2025.
//

import SwiftTUI
import Combine
import Foundation

/// Observable object that tracks terminal size changes
@MainActor
public class TerminalSizeTracker: ObservableObject {
    @Published public var width: Int = 80
    @Published public var height: Int = 24

    /// Timer to periodically check terminal size
    private var timer: Foundation.Timer?

    public init() {
        updateSize()
        startMonitoring()
    }

    /// Start monitoring terminal size changes
    private func startMonitoring() {
        // Check size every 0.5 seconds
        timer = Foundation.Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            Task { @MainActor in
                self?.updateSize()
            }
        }
    }

    /// Update size by querying the terminal
    private func updateSize() {
        #if os(macOS) || os(Linux)
        var size = winsize()
        guard ioctl(STDOUT_FILENO, UInt(TIOCGWINSZ), &size) == 0,
              size.ws_col > 0, size.ws_row > 0 else {
            return
        }
        let newWidth = Int(size.ws_col)
        let newHeight = Int(size.ws_row)

        if newWidth != width || newHeight != height {
            width = newWidth
            height = newHeight
        }
        #endif
    }

    deinit {
        timer?.invalidate()
    }
}

/// Helper view that wraps content and updates list state based on terminal size
@MainActor
public struct TerminalAwareList<Content: View>: View {
    @Binding var listState: TUIListState
    let sizeTracker: TerminalSizeTracker
    let reservedLines: Int
    let content: Content

    public init(
        state: Binding<TUIListState>,
        sizeTracker: TerminalSizeTracker,
        reservedLines: Int,
        @ViewBuilder content: () -> Content
    ) {
        self._listState = state
        self.sizeTracker = sizeTracker
        self.reservedLines = reservedLines
        self.content = content()
    }

    public var body: some View {
        // Update list height when size changes
        let calculated = max(1, sizeTracker.height - reservedLines)
        if listState.availableHeight != calculated {
            DispatchQueue.main.async {
                self.listState.updateAvailableHeight(
                    terminalHeight: self.sizeTracker.height,
                    reservedLines: self.reservedLines
                )
            }
        }
        return content
    }
}
