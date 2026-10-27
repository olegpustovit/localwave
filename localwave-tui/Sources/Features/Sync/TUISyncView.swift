//
//  TUISyncView.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData

/// TUI view for browsing and selecting music source folders
struct TUISyncView: View {
    let dependencies: TUIDependencyContainer
    @Binding var navigationState: NavigationState

    @State private var sources: [Source] = []
    @State private var selectedSourceId: Int64?
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(spacing: 1) {
            if isLoading {
                loadingView
            } else if let error = errorMessage {
                errorView(error)
            } else if sources.isEmpty {
                emptyStateView
            } else {
                sourceListView
            }

            helpBar
        }
        .onAppear {
            Task { await loadSources() }
        }
        // Vim-style navigation
        .onKeyPress("j") { selectNext() }
        .onKeyPress("k") { selectPrevious() }
        .onKeyPress("g") { selectFirst() }
        .onKeyPress("G") { selectLast() }
        // Actions
        .onKeyPress("l") { openSelectedSource() }  // vim: go right/into
        .onKeyPress("\r") { openSelectedSource() }  // Enter key
        .onKeyPress("m") { openSourceManagement() }  // manage sources
        .onKeyPress("r") { Task { await loadSources() } }  // refresh
    }

    private var loadingView: some View {
        VStack {
            Text("Loading sources...")
            Spacer()
        }
    }

    private func errorView(_ message: String) -> some View {
        VStack {
            Text("Error: \(message)")
            Text("")
            Text("Press 'r' to retry")
            Spacer()
        }
    }

    private var emptyStateView: some View {
        VStack {
            Text("No music sources found")
            Text("")
            Text("Press 'a' to add a new source")
            Text("Press 'q' to return")
            Spacer()
        }
    }

    private var sourceListView: some View {
        VStack(spacing: 0) {
            Text("Music Sources (\(sources.count))")
            Text("")

            ForEach(sources) { source in
                let isSelected = source.id == selectedSourceId
                HStack {
                    Text(isSelected ? " > " : "   ")
                    Text(source.dirPath)
                    Spacer()
                }
            }

            Spacer()
        }
    }

    private var helpBar: some View {
        VStack(spacing: 0) {
            Text(String(repeating: "─", count: 60))
            HStack {
                Text("[j/k] Navigate")
                Text(" ")
                Text("[Enter/l] Open")
                Text(" ")
                Text("[m] Manage")
                Text(" ")
                Text("[r] Refresh")
                Spacer()
            }
        }
    }

    // MARK: - Actions

    private func loadSources() async {
        isLoading = true
        errorMessage = nil

        do {
            // Get current user's sources
            let userId: Int64 = 1  // TODO: Get from user cloud service
            let sourceRepo = dependencies.sourceService.repository()
            let loadedSources = try await sourceRepo.findOneByUserId(userId: userId, path: nil)
            sources = loadedSources
            if let first = sources.first {
                selectedSourceId = first.id
            }
        } catch {
            errorMessage = error.localizedDescription
        }

        isLoading = false
    }

    private func openSelectedSource() {
        guard let sourceId = selectedSourceId else { return }
        guard let source = sources.first(where: { $0.id == sourceId }) else { return }

        // Navigate to source browse view with source's pathId as root
        navigationState.push(.sourceBrowse(sourceId: sourceId, parentPathId: source.pathId))
    }

    private func openSourceManagement() {
        navigationState.push(.sourceManagement)
    }

    private func selectNext() {
        guard !sources.isEmpty,
              let currentId = selectedSourceId,
              let currentIndex = sources.firstIndex(where: { $0.id == currentId }),
              currentIndex < sources.count - 1 else { return }
        selectedSourceId = sources[currentIndex + 1].id
    }

    private func selectPrevious() {
        guard !sources.isEmpty,
              let currentId = selectedSourceId,
              let currentIndex = sources.firstIndex(where: { $0.id == currentId }),
              currentIndex > 0 else { return }
        selectedSourceId = sources[currentIndex - 1].id
    }

    private func selectFirst() {
        guard let first = sources.first else { return }
        selectedSourceId = first.id
    }

    private func selectLast() {
        guard let last = sources.last else { return }
        selectedSourceId = last.id
    }
}
