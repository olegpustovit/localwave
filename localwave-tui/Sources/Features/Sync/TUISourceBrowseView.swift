//
//  TUISourceBrowseView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//  Rewritten to match iOS SourceBrowseView 1:1
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI view for browsing source folders with checkboxes (matches iOS)
///
/// Features (matching iOS SourceBrowseViewModel):
/// - Hierarchical folder navigation with path stack
/// - Checkbox for every item (folders and files)
/// - Search functionality
/// - Import selected items
/// - Pagination with Ctrl+u/d
struct TUISourceBrowseView: View {
    let dependencies: TUIDependencyContainer
    let sourceId: Int64
    let initialParentPathId: Int64?
    @Binding var navigationState: NavigationState

    // Path stack (like iOS pathStack)
    @State private var pathStack: [Int64?] = []

    // Items and display
    @State private var items: [SourcePath] = []
    @State private var selectedPathIds = Set<Int64>()
    @State private var selectedIndex = 0

    // Pagination
    @State private var pageOffset = 0
    private let pageSize = 10

    // Search
    @State private var searchMode = false
    @State private var searchTerm = ""

    // Loading and errors
    @State private var isLoading = false
    @State private var errorMessage: String?

    // Import progress
    @State private var isImporting = false
    @State private var importProgress: Double = 0.0
    @State private var currentFileName: String = ""

    var currentParentPathId: Int64? {
        pathStack.last ?? nil
    }

    var canGoBack: Bool {
        pathStack.count > 1
    }

    var paginatedItems: ArraySlice<SourcePath> {
        let start = pageOffset
        let end = min(start + pageSize, items.count)
        guard start < items.count else { return [] }
        return items[start..<end]
    }

    var body: some View {
        VStack(spacing: 1) {
            // Header with breadcrumb
            headerView

            // Search bar or import status
            if searchMode {
                searchBarView
            } else if isImporting {
                importProgressView
            } else if selectedPathIds.count > 0 {
                importButtonView
            } else if !searchTerm.isEmpty {
                searchResultsHeader
            }

            Text("")

            if isLoading {
                loadingView
            } else if let error = errorMessage {
                errorView(error)
            } else if items.isEmpty {
                emptyView
            } else {
                itemListView
            }

            Spacer()
            helpBar
        }
        .onAppear {
            // Initialize path stack with source's pathId as root
            if pathStack.isEmpty {
                if let initialParent = initialParentPathId {
                    // Start with source's pathId as root
                    pathStack = [initialParent]
                } else {
                    pathStack = [nil]
                }
            }
            Task { await loadItems() }
        }
        // Navigation (only when not in search mode)
        .onKeyPress("j") { if !searchMode { selectNext() } }
        .onKeyPress("k") { if !searchMode { selectPrevious() } }
        .onKeyPress("g") { if !searchMode { selectFirst() } }
        .onKeyPress("G") { if !searchMode { selectLast() } }
        // Pagination (only when not in search mode)
        .onKeyPress("u") { if !searchMode { pageUp() } }
        .onKeyPress("d") { if !searchMode { pageDown() } }
        // Actions (only when not in search mode)
        .onKeyPress("l") { if !searchMode { openOrEnterSelected() } }
        .onKeyPress("\r") { if !searchMode { openOrEnterSelected() } }
        .onKeyPress("h") { if !searchMode { goBack() } }
        .onKeyPress(" ") { if !searchMode { toggleSelectedCheckbox() } }
        .onKeyPress("i") { if !searchMode { Task { await importSelected() } } }
        .onKeyPress("/") { if !searchMode { searchMode = true } }
        .onKeyPress("\u{1B}") { handleEscape() }
    }

    // MARK: - View Components

    private var headerView: some View {
        HStack {
            if canGoBack {
                Text("[h=Back] ")
            }
            Text("Browse: \(currentBreadcrumb())")
            Spacer()
        }
    }

    private var searchBarView: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Search mode - type and press Enter to search, Esc to exit")
                Spacer()
            }
            HStack {
                Text("Query: ")
                TextField(placeholder: "Type search term...") { query in
                    searchTerm = query
                    searchMode = false  // Exit search mode after submitting
                    Task { await loadItems() }
                }
                Spacer()
            }
        }
    }

    private var searchResultsHeader: some View {
        HStack {
            Text("Search results for: \"\(searchTerm)\"")
            Text(" - Press '/' to search again, Esc to clear")
            Spacer()
        }
    }

    private var importButtonView: some View {
        HStack {
            Text(TUIColors.Indicators.success("[\(selectedPathIds.count) selected]"))
            Text(" - Press 'i' to import")
            Spacer()
        }
    }

    private var importProgressView: some View {
        VStack(spacing: 1) {
            HStack {
                Text("Importing...")
                Spacer()
            }
            HStack {
                TUIProgressBar(
                    value: importProgress / 100.0,
                    width: TUITheme.Layout.progressBarWidthWide,
                    label: nil,
                    showPercentage: true
                )
                Spacer()
            }
            HStack {
                Text(currentFileName.isEmpty ? "(initializing...)" : TUITheme.truncate(currentFileName, width: 60))
                Spacer()
            }
        }
    }

    private var loadingView: some View {
        VStack {
            Text("Loading...")
            Spacer()
        }
    }

    private func errorView(_ message: String) -> some View {
        VStack {
            HStack {
                Text(TUIColors.Indicators.error("Error: \(message)"))
                Spacer()
            }
            Spacer()
        }
    }

    private var emptyView: some View {
        VStack {
            Text("No items in this folder")
            Spacer()
        }
    }

    private var itemListView: some View {
        VStack(spacing: 0) {
            // Page indicator
            if items.count > pageSize {
                HStack {
                    Text("Showing \(pageOffset + 1)-\(min(pageOffset + pageSize, items.count)) of \(items.count)")
                    Text(" [u/d for pages]")
                    Spacer()
                }
                Text("")
            }

            ForEach(Array(paginatedItems.enumerated()), id: \.element.pathId) { index, item in
                let globalIndex = pageOffset + index
                let isSelected = globalIndex == selectedIndex
                itemRow(item: item, isSelected: isSelected, isChecked: selectedPathIds.contains(item.pathId))
            }
            Spacer()
        }
    }

    private func itemRow(item: SourcePath, isSelected: Bool, isChecked: Bool) -> some View {
        HStack {
            Text(isSelected ? " > " : "   ")
            Text(isChecked ? "[✓]" : "[ ]")
            Text(" ")
            Text(item.isDirectory ? TUITheme.Icons.folder : TUITheme.Icons.music)
            Text(" ")
            Text(TUITheme.truncate(item.name, width: 40))
            Spacer()
        }
    }

    private var helpBar: some View {
        VStack(spacing: 0) {
            Text(TUITheme.divider(width: 80))
            HStack {
                if searchMode {
                    Text("[Esc] Exit Search")
                } else {
                    Text("[Space] Check")
                    Text("  ")
                    Text("[l/Enter] Open")
                    Text("  ")
                    Text("[h] Back")
                    Text("  ")
                    Text("[i] Import")
                    Text("  ")
                    Text("[/] Search")
                }
                Spacer()
            }
        }
    }

    // MARK: - Actions

    private func loadItems() async {
        isLoading = true
        errorMessage = nil

        do {
            let sourceImportService = await dependencies.sourceService.importService()

            if searchTerm.isEmpty {
                items = try await sourceImportService.listItems(
                    sourceId: sourceId,
                    parentPathId: currentParentPathId
                )
            } else {
                items = try await sourceImportService.search(
                    sourceId: sourceId,
                    query: searchTerm
                )
            }

            selectedIndex = 0
            pageOffset = 0
        } catch {
            errorMessage = error.localizedDescription
        }

        isLoading = false
    }

    private func openOrEnterSelected() {
        guard selectedIndex < items.count else { return }
        let item = items[selectedIndex]

        if item.isDirectory {
            // Navigate into folder (push to stack)
            pathStack.append(item.pathId)
            searchTerm = ""
            searchMode = false
            Task { await loadItems() }
        }
    }

    private func goBack() {
        guard canGoBack else {
            // At root, go back to sync view
            navigationState.pop()
            return
        }

        pathStack.removeLast()
        searchTerm = ""
        searchMode = false
        Task { await loadItems() }
    }

    private func toggleSelectedCheckbox() {
        guard selectedIndex < items.count else { return }
        let item = items[selectedIndex]

        if selectedPathIds.contains(item.pathId) {
            selectedPathIds.remove(item.pathId)
        } else {
            selectedPathIds.insert(item.pathId)
        }
    }

    private func importSelected() async {
        guard !selectedPathIds.isEmpty, !isImporting else { return }

        isImporting = true
        importProgress = 0.0
        currentFileName = ""

        do {
            let selectedPaths = items.filter { selectedPathIds.contains($0.pathId) }
            let songImportService = await dependencies.songImportService

            try await songImportService.importPaths(
                paths: selectedPaths,
                onProgress: { pct, fileURL async in
                    await MainActor.run {
                        self.importProgress = pct
                        self.currentFileName = fileURL.lastPathComponent
                    }
                }
            )

            // Clear selection on success
            selectedPathIds = []
        } catch {
            errorMessage = "Import failed: \(error.localizedDescription)"
        }

        isImporting = false
    }

    private func handleEscape() {
        if searchMode {
            // Cancel search input mode
            searchMode = false
            searchTerm = ""
        } else if !searchTerm.isEmpty {
            // Clear search results and go back to browse
            searchTerm = ""
            Task { await loadItems() }
        } else {
            // Exit browse view
            navigationState.pop()
        }
    }

    private func currentBreadcrumb() -> String {
        // TODO: Could enhance by showing path names instead of IDs
        if let current = currentParentPathId {
            return "Folder \(current)"
        } else {
            return "Root"
        }
    }

    // MARK: - Navigation Helpers

    private func selectNext() {
        guard !items.isEmpty else { return }
        if selectedIndex < items.count - 1 {
            selectedIndex += 1
            // Auto-scroll to next page if needed
            if selectedIndex >= pageOffset + pageSize {
                pageOffset = min(selectedIndex, items.count - pageSize)
            }
        }
    }

    private func selectPrevious() {
        guard !items.isEmpty else { return }
        if selectedIndex > 0 {
            selectedIndex -= 1
            // Auto-scroll to previous page if needed
            if selectedIndex < pageOffset {
                pageOffset = max(0, selectedIndex)
            }
        }
    }

    private func selectFirst() {
        selectedIndex = 0
        pageOffset = 0
    }

    private func selectLast() {
        guard !items.isEmpty else { return }
        selectedIndex = items.count - 1
        pageOffset = max(0, items.count - pageSize)
    }

    private func pageUp() {
        // Ctrl+U - page up
        pageOffset = max(0, pageOffset - pageSize)
        selectedIndex = max(0, min(selectedIndex, pageOffset + pageSize - 1))
    }

    private func pageDown() {
        // Ctrl+D - page down
        let maxOffset = max(0, items.count - pageSize)
        pageOffset = min(maxOffset, pageOffset + pageSize)
        selectedIndex = min(items.count - 1, pageOffset)
    }
}
