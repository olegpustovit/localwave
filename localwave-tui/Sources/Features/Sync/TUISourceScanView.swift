//
//  TUISourceScanView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import Foundation
import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI view for scanning and importing songs from a selected source path
///
/// Shows real-time progress as songs are scanned and imported:
/// - Progress bar with percentage
/// - Current file being processed
/// - Stats (added, updated, errors)
/// - Cancel option
struct TUISourceScanView: View {
    let dependencies: TUIDependencyContainer
    let sourceId: Int64
    let pathId: Int64
    let path: String
    @Binding var navigationState: NavigationState

    // Scanning state
    @State private var isScanning = false
    @State private var isCancelled = false
    @State private var isComplete = false
    @State private var errorMessage: String?

    // Progress tracking
    @State private var progress: Double = 0.0  // 0.0 to 1.0
    @State private var currentFile: String = ""
    @State private var totalFiles: Int = 0
    @State private var processedFiles: Int = 0

    // Time tracking
    @State private var startTime: Date?
    @State private var estimatedTimeRemaining: String = ""
    @State private var elapsedTime: String = ""

    // Stats
    @State private var addedCount: Int = 0
    @State private var updatedCount: Int = 0
    @State private var errorCount: Int = 0
    @State private var skippedCount: Int = 0
    @State private var errorMessages: [String] = []

    var body: some View {
        VStack(spacing: 1) {
            // Header
            HStack {
                Text("Scanning: \(path)")
                Spacer()
            }
            Text("")

            if let error = errorMessage {
                errorView(error)
            } else if isComplete {
                completionView
            } else if isScanning {
                scanningView
            } else {
                startView
            }

            Spacer()
            helpBar
        }
        .onAppear {
            // Auto-start scanning when view appears
            Task { await startScanning() }
        }
        .onKeyPress("c") {
            if isScanning {
                cancelScanning()
            }
        }
        .onKeyPress("h") {  // Back key (vim-style)
            navigationState.pop()
        }
        .onKeyPress("\u{1B}") {  // Escape key - always go back
            navigationState.pop()
        }
    }

    private var startView: some View {
        VStack {
            Text("Preparing to scan...")
            Text("")
            Text("Press Esc to cancel")
        }
    }

    private var scanningView: some View {
        VStack(spacing: 1) {
            // Progress bar with time estimate
            HStack {
                Text("Scanning...")
                Text(TUITheme.Icons.music)
                Spacer()
            }

            HStack {
                TUIProgressBar(
                    value: progress,
                    width: TUITheme.Layout.progressBarWidthWide,
                    label: nil,
                    showPercentage: true
                )
                if !estimatedTimeRemaining.isEmpty && progress > 0.05 {
                    Text(" - \(estimatedTimeRemaining) remaining")
                }
                Spacer()
            }
            Text("")

            // Current file
            HStack {
                if totalFiles > 0 {
                    Text("Processing (\(processedFiles)/\(totalFiles)):")
                } else {
                    Text("Processing:")
                }
                Spacer()
            }
            HStack {
                Text("  ")
                Text(currentFile.isEmpty ? "(initializing...)" : TUITheme.truncate(currentFile, width: 60))
                Spacer()
            }
            Text("")

            // Stats
            HStack {
                Text(TUIColors.Indicators.success("Added: \(addedCount)"))
                Text("  ")
                Text("Updated: \(updatedCount)")
                if skippedCount > 0 {
                    Text("  ")
                    Text("Skipped: \(skippedCount)")
                }
                Spacer()
            }
            if errorCount > 0 {
                HStack {
                    Text(TUIColors.Indicators.error("Errors: \(errorCount)"))
                    Spacer()
                }
            }

            // Elapsed time
            if !elapsedTime.isEmpty {
                Text("")
                HStack {
                    Text("Elapsed: \(elapsedTime)")
                    Spacer()
                }
            }
        }
    }

    private var completionView: some View {
        VStack(spacing: 1) {
            HStack {
                Text(TUIColors.Indicators.success("Scan Complete!"))
                Spacer()
            }
            Text("")

            // Summary stats
            HStack {
                Text(TUITheme.Icons.checkmark + " Added: \(addedCount) new songs")
                Spacer()
            }
            HStack {
                Text(TUITheme.Icons.checkmark + " Updated: \(updatedCount) existing songs")
                Spacer()
            }
            if skippedCount > 0 {
                HStack {
                    Text("  Skipped: \(skippedCount) files")
                    Spacer()
                }
            }
            if errorCount > 0 {
                HStack {
                    Text(TUITheme.Icons.error + " Errors: \(errorCount)")
                    Spacer()
                }
                // Show first few error messages
                if !errorMessages.isEmpty {
                    Text("")
                    HStack {
                        Text("Error details:")
                        Spacer()
                    }
                    ForEach(errorMessages.prefix(3), id: \.self) { msg in
                        HStack {
                            Text("  • \(TUITheme.truncate(msg, width: 70))")
                            Spacer()
                        }
                    }
                    if errorMessages.count > 3 {
                        HStack {
                            Text("  ... and \(errorMessages.count - 3) more")
                            Spacer()
                        }
                    }
                }
            }

            // Total time
            if !elapsedTime.isEmpty {
                Text("")
                HStack {
                    Text("Total time: \(elapsedTime)")
                    Spacer()
                }
            }

            Text("")
            HStack {
                Text("Press 'h' or Esc to go back")
                Spacer()
            }
        }
    }

    private func errorView(_ message: String) -> some View {
        VStack {
            HStack {
                Text(TUIColors.Indicators.error("Scan Failed"))
                Spacer()
            }
            Text("")
            HStack {
                Text(message)
                Spacer()
            }
            Text("")
            HStack {
                Text("Press 'h' or Esc to go back")
                Spacer()
            }
        }
    }

    private var helpBar: some View {
        VStack(spacing: 0) {
            Text(TUITheme.divider(width: 60))
            HStack {
                if isScanning {
                    Text("[c] Cancel Scan")
                    Text("  ")
                }
                Text("[h/Esc] Back")
                Spacer()
            }
        }
    }

    // MARK: - Actions

    private func startScanning() async {
        guard !isScanning else { return }

        isScanning = true
        isCancelled = false
        isComplete = false
        errorMessage = nil
        progress = 0.0
        currentFile = ""
        addedCount = 0
        updatedCount = 0
        errorCount = 0
        skippedCount = 0
        errorMessages = []
        startTime = Date()

        do {
            // Get the SourcePath to scan
            let sourcePathRepo = dependencies.sourcePathRepository
            guard let sourcePath = try await sourcePathRepo.getByPathId(sourceId: sourceId, pathId: pathId) else {
                errorMessage = "Source path not found (sourceId: \(sourceId), pathId: \(pathId)). Try re-syncing the source from management view."
                isScanning = false
                return
            }

            // Note: We skip counting total files to avoid hanging on large directories
            // Progress will be based on percentage from import service
            totalFiles = 0  // Unknown until scan progresses

            // Get song import service
            let songImportService = dependencies.songImportService

            // Track stats
            var addedSongs = 0
            var updatedSongs = 0
            var skipped = 0
            var errors = 0
            var errorsList: [String] = []
            var lastProcessedFile = ""

            // Import with progress callback
            try await songImportService.importPaths(
                paths: [sourcePath],
                onProgress: { percent, url async in
                    if Task.isCancelled {
                        return
                    }

                    let newFile = url.lastPathComponent
                    if newFile != lastProcessedFile {
                        lastProcessedFile = newFile
                        addedSongs += 1
                    }

                    await MainActor.run {
                        self.progress = percent / 100.0
                        self.currentFile = newFile
                        self.processedFiles = Int(percent * Double(self.totalFiles) / 100.0)
                        self.addedCount = addedSongs
                        self.updatedCount = updatedSongs
                        self.errorCount = errors
                        self.skippedCount = skipped

                        // Update time estimates
                        self.updateTimeEstimates()
                    }
                }
            )

            // Completion
            isComplete = true
            isScanning = false
            elapsedTime = formatElapsedTime(from: startTime ?? Date())

        } catch is CancellationError {
            errorMessage = "Scan cancelled by user"
            isScanning = false
            elapsedTime = formatElapsedTime(from: startTime ?? Date())
        } catch {
            errorMessage = error.localizedDescription
            isScanning = false
            elapsedTime = formatElapsedTime(from: startTime ?? Date())
        }
    }

    private func cancelScanning() {
        isCancelled = true
        isScanning = false

        // Cancel the import task
        Task {
            await dependencies.songImportService.cancelImport()
        }
    }

    // MARK: - Helper Methods

    /// Update time estimates based on current progress
    private func updateTimeEstimates() {
        guard let start = startTime, progress > 0.05 else {
            estimatedTimeRemaining = ""
            elapsedTime = ""
            return
        }

        let elapsed = Date().timeIntervalSince(start)
        elapsedTime = formatElapsedTime(from: start)

        // Estimate remaining time based on progress
        let estimatedTotal = elapsed / progress
        let remaining = estimatedTotal - elapsed

        if remaining > 0 {
            estimatedTimeRemaining = formatTimeInterval(remaining)
        } else {
            estimatedTimeRemaining = ""
        }
    }

    /// Format elapsed time from start date
    private func formatElapsedTime(from start: Date) -> String {
        let elapsed = Date().timeIntervalSince(start)
        return formatTimeInterval(elapsed)
    }

    /// Format time interval as human-readable string
    private func formatTimeInterval(_ interval: TimeInterval) -> String {
        let seconds = Int(interval)
        let hours = seconds / 3600
        let minutes = (seconds % 3600) / 60
        let secs = seconds % 60

        if hours > 0 {
            return String(format: "%dh %dm", hours, minutes)
        } else if minutes > 0 {
            return String(format: "%dm %ds", minutes, secs)
        } else {
            return String(format: "%ds", secs)
        }
    }

    /// Recursively gather all file paths under a directory
    private func gatherAllPaths(sourcePathRepo: SourcePathRepository, startPath: SourcePath) async throws -> [SourcePath] {
        var result = [SourcePath]()

        if startPath.isDirectory {
            let children = try await sourcePathRepo.getByParentId(sourceId: sourceId, parentPathId: startPath.pathId)
            for child in children {
                let childPaths = try await gatherAllPaths(sourcePathRepo: sourcePathRepo, startPath: child)
                result.append(contentsOf: childPaths)
            }
        } else {
            result.append(startPath)
        }

        return result
    }
}
