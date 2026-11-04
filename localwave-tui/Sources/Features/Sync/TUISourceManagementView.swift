//
//  TUISourceManagementView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import Foundation
import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI view for managing music sources
///
/// Features:
/// - List all music sources with song counts
/// - Add new source (file path input)
/// - Remove source with confirmation dialog
/// - Rescan source to update library
struct TUISourceManagementView: View {
    let dependencies: TUIDependencyContainer
    @Binding var navigationState: NavigationState

    // Source list state
    @State private var sources: [Source] = []
    @State private var selectedIndex: Int = 0
    @State private var isLoading = false
    @State private var errorMessage: String?

    // Action states
    @State private var showAddDialog = false
    @State private var showDeleteConfirmation = false
    @State private var sourceToDelete: Source?

    var body: some View {
        VStack(spacing: 1) {
            // Header
            HStack {
                Text("Music Sources (\(sources.count))")
                Spacer()
            }
            Text("")

            if isLoading {
                loadingView
            } else if let error = errorMessage {
                errorView(error)
            } else if showAddDialog {
                addSourceDialog
            } else if showDeleteConfirmation {
                deleteConfirmationDialog
            } else if sources.isEmpty {
                emptyStateView
            } else {
                sourceListView
            }

            Spacer()
            helpBar
        }
        .onAppear {
            Task { await loadSources() }
        }
        // Navigation (only when not in dialogs)
        .onKeyPress("j") {
            if !showAddDialog && !showDeleteConfirmation {
                selectNext()
            }
        }
        .onKeyPress("k") {
            if !showAddDialog && !showDeleteConfirmation {
                selectPrevious()
            }
        }
        .onKeyPress("g") {
            if !showAddDialog && !showDeleteConfirmation {
                selectFirst()
            }
        }
        .onKeyPress("G") {
            if !showAddDialog && !showDeleteConfirmation {
                selectLast()
            }
        }
        // Actions (only when not in dialogs)
        .onKeyPress("a") {
            if !showAddDialog && !showDeleteConfirmation {
                showAddDialog = true
            }
        }
        .onKeyPress("A") {
            if !showAddDialog && !showDeleteConfirmation {
                showAddDialog = true
            }
        }
        .onKeyPress("d") {
            if !showAddDialog && !showDeleteConfirmation {
                confirmDelete()
            }
        }
        .onKeyPress("r") {
            if !showAddDialog && !showDeleteConfirmation {
                Task { await rescanSelected() }
            }
        }
        .onKeyPress("R") {
            if !showAddDialog && !showDeleteConfirmation {
                Task { await loadSources() }
            }
        }
        .onKeyPress("\r") {
            if !showAddDialog && !showDeleteConfirmation {
                openSelected()
            }
        }
        .onKeyPress("\u{1B}") { handleEscape() }  // Escape
        // Delete confirmation handlers
        .onKeyPress("y") {
            if showDeleteConfirmation {
                Task { await deleteConfirmed() }
            }
        }
        .onKeyPress("Y") {
            if showDeleteConfirmation {
                Task { await deleteConfirmed() }
            }
        }
        .onKeyPress("n") {
            if showDeleteConfirmation {
                cancelDelete()
            }
        }
        .onKeyPress("N") {
            if showDeleteConfirmation {
                cancelDelete()
            }
        }
    }

    // MARK: - View Components

    private var loadingView: some View {
        VStack {
            Text("Loading sources...")
            Spacer()
        }
    }

    private func errorView(_ message: String) -> some View {
        VStack {
            HStack {
                Text(TUIColors.Indicators.error("Error: \(message)"))
                Spacer()
            }
            Text("")
            HStack {
                Text("Press 'R' to retry or Esc to go back")
                Spacer()
            }
            Spacer()
        }
    }

    private var emptyStateView: some View {
        VStack {
            Text("No music sources configured")
            Text("")
            Text("Press 'a' to add your first source")
            Text("Press Esc to go back")
            Spacer()
        }
    }

    private var sourceListView: some View {
        VStack(spacing: 0) {
            ForEach(Array(sources.enumerated()), id: \.element.id) { index, source in
                let isSelected = index == selectedIndex
                sourceRow(source: source, isSelected: isSelected)
            }
            Spacer()
        }
    }

    private func sourceRow(source: Source, isSelected: Bool) -> some View {
        HStack {
            Text(isSelected ? " > " : "   ")
            Text(TUITheme.Icons.folder)
            Text(" ")
            Text(TUITheme.truncate(source.dirPath, width: 50))
            Spacer()
            Text("\(source.totalPaths ?? 0) songs")
        }
    }

    private var addSourceDialog: some View {
        VStack(spacing: 1) {
            Spacer()

            VStack(spacing: 1) {
                // Title
                HStack {
                    Text("Add New Music Source")
                    Spacer()
                }
                Text("")

                // Instructions
                HStack {
                    Text("Enter the full path to your music folder:")
                    Spacer()
                }
                Text("")

                // Input field - SwiftTUI's TextField handles keyboard input automatically
                HStack {
                    Text("Path: ")
                    TextField(placeholder: "/Users/you/Music") { path in
                        Task { await submitNewSource(path: path) }
                    }
                    Spacer()
                }
            }

            VStack(spacing: 1) {
                Text("")

                // Help text
                HStack {
                    Text(TUIColors.Indicators.info("Tip: Type the full path and press Enter"))
                    Spacer()
                }
                Text("")

                // Actions
                HStack {
                    Text("[Enter] Add Source")
                    Text("  ")
                    Text("[Esc] Cancel")
                    Spacer()
                }
            }

            Spacer()
        }
    }

    private var deleteConfirmationDialog: some View {
        VStack(spacing: 1) {
            Spacer()

            if let source = sourceToDelete {
                VStack(spacing: 1) {
                    // Title
                    HStack {
                        Text(TUIColors.Indicators.warning("Confirm Delete"))
                        Spacer()
                    }
                    Text("")

                    // Warning
                    HStack {
                        Text("Delete this source?")
                        Spacer()
                    }
                    Text("")
                    HStack {
                        Text("  \(TUITheme.truncate(source.dirPath, width: 60))")
                        Spacer()
                    }
                    Text("")
                }

                VStack(spacing: 1) {
                    // Details
                    HStack {
                        Text("This will remove \(source.totalPaths ?? 0) songs from your library.")
                        Spacer()
                    }
                    HStack {
                        Text("This action cannot be undone!")
                        Spacer()
                    }
                    Text("")

                    // Actions
                    HStack {
                        Text("[y] Yes, Delete")
                        Text("  ")
                        Text("[n/Esc] Cancel")
                        Spacer()
                    }
                }
            }

            Spacer()
        }
    }

    private var helpBar: some View {
        VStack(spacing: 0) {
            Text(TUITheme.divider(width: 80))
            HStack {
                if showAddDialog || showDeleteConfirmation {
                    Text("[Esc] Cancel")
                } else {
                    VStack(spacing: 0) {
                        HStack {
                            Text("[j/k] Navigate")
                            Text("  ")
                            Text("[Enter] Browse")
                            Text("  ")
                            Text("[a] Add")
                            Text("  ")
                            Text("[d/Del] Delete")
                            Spacer()
                        }
                        HStack {
                            Text("[r] Rescan")
                            Text("  ")
                            Text("[R] Refresh")
                            Spacer()
                        }
                    }
                }
                Spacer()
            }
        }
    }

    // MARK: - Actions

    private func loadSources() async {
        isLoading = true
        errorMessage = nil

        do {
            let userId: Int64 = 1  // TODO: Get from user cloud service
            let sourceRepo = dependencies.sourceService.repository()
            sources = try await sourceRepo.findOneByUserId(userId: userId, path: nil)

            if !sources.isEmpty && selectedIndex >= sources.count {
                selectedIndex = sources.count - 1
            }
        } catch {
            errorMessage = error.localizedDescription
        }

        isLoading = false
    }

    private func openSelected() {
        guard selectedIndex < sources.count else { return }
        let source = sources[selectedIndex]
        guard let sourceId = source.id else { return }

        // Navigate to source browse view with source's pathId as root
        navigationState.push(.sourceBrowse(sourceId: sourceId, parentPathId: source.pathId))
    }

    private func confirmDelete() {
        guard selectedIndex < sources.count else { return }
        sourceToDelete = sources[selectedIndex]
        showDeleteConfirmation = true
    }

    private func cancelDelete() {
        showDeleteConfirmation = false
        sourceToDelete = nil
    }

    private func deleteConfirmed() async {
        guard let source = sourceToDelete, let sourceId = source.id else {
            cancelDelete()
            return
        }

        showDeleteConfirmation = false
        isLoading = true

        do {
            // Delete the source
            let sourceRepo = dependencies.sourceService.repository()
            try await sourceRepo.deleteSource(sourceId: sourceId)

            // Note: In a real implementation, we'd have a method to delete songs by source
            // For now, we'll just reload the sources list

            sourceToDelete = nil
            await loadSources()
        } catch {
            errorMessage = "Failed to delete source: \(error.localizedDescription)"
            isLoading = false
        }
    }

    private func rescanSelected() async {
        guard selectedIndex < sources.count else { return }
        let source = sources[selectedIndex]
        guard let sourceId = source.id else { return }

        // Navigate to scan view for this source's root
        navigationState.push(.sourceScan(
            sourceId: sourceId,
            pathId: source.pathId,
            path: source.dirPath
        ))
    }

    private func handleEscape() {
        if showAddDialog {
            showAddDialog = false
        } else if showDeleteConfirmation {
            cancelDelete()
        } else {
            navigationState.pop()
        }
    }

    private func submitNewSource(path: String) async {
        // Strip whitespace and quotes (single or double)
        var cleanPath = path.trimmingCharacters(in: .whitespacesAndNewlines)

        // Remove surrounding quotes if present
        if (cleanPath.hasPrefix("'") && cleanPath.hasSuffix("'")) ||
           (cleanPath.hasPrefix("\"") && cleanPath.hasSuffix("\"")) {
            cleanPath = String(cleanPath.dropFirst().dropLast())
        }

        guard !cleanPath.isEmpty else {
            showAddDialog = false
            return
        }

        showAddDialog = false
        isLoading = true
        errorMessage = nil

        do {
            let userId: Int64 = 1  // TODO: Get from user cloud service

            // Add the source using registerSourcePath
            let newSource = try await dependencies.sourceService.registerSourcePath(
                userId: userId,
                path: cleanPath,
                type: .iCloud
            )

            // Sync the directory structure to populate SourcePaths
            guard let sourceId = newSource.id else {
                errorMessage = "Source created but has no ID"
                isLoading = false
                return
            }

            let url = URL(fileURLWithPath: cleanPath)

            // Create a security-scoped bookmark for the TUI
            // On macOS, TUI has direct file access but the sync service expects bookmarks
            let bookmarkKey = makeBookmarkKey(url)
            if UserDefaults.standard.data(forKey: bookmarkKey) == nil {
                do {
                    // Create a file-reference bookmark (works for command-line apps)
                    let bookmarkData = try url.bookmarkData(
                        options: [.withSecurityScope, .securityScopeAllowOnlyReadAccess],
                        includingResourceValuesForKeys: nil,
                        relativeTo: nil
                    )
                    UserDefaults.standard.set(bookmarkData, forKey: bookmarkKey)
                } catch {
                    // Fallback: create without security scope (for non-sandboxed apps)
                    if let bookmarkData = try? url.bookmarkData(
                        options: [],
                        includingResourceValuesForKeys: nil,
                        relativeTo: nil
                    ) {
                        UserDefaults.standard.set(bookmarkData, forKey: bookmarkKey)
                    } else {
                        errorMessage = "Failed to create bookmark: \(error.localizedDescription)"
                        isLoading = false
                        return
                    }
                }
            }

            let syncService = dependencies.sourceService.syncService()
            let syncedSource = try await syncService.syncDir(
                sourceId: sourceId,
                folderURL: url,
                onCurrentURL: nil,
                onSetLoading: nil
            )

            // Verify sync succeeded
            if syncedSource == nil {
                errorMessage = "Failed to sync directory structure for: \(cleanPath)"
                isLoading = false
                return
            }

            // Reload sources to get updated pathId
            await loadSources()
        } catch {
            errorMessage = "Failed to add source: \(error.localizedDescription)"
            isLoading = false
        }
    }

    // MARK: - Navigation Helpers

    private func selectNext() {
        guard !sources.isEmpty else { return }
        selectedIndex = min(selectedIndex + 1, sources.count - 1)
    }

    private func selectPrevious() {
        guard !sources.isEmpty else { return }
        selectedIndex = max(selectedIndex - 1, 0)
    }

    private func selectFirst() {
        selectedIndex = 0
    }

    private func selectLast() {
        guard !sources.isEmpty else { return }
        selectedIndex = sources.count - 1
    }
}
