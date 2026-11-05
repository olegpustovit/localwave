import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  SourceBrowseViewModel.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct SourceBrowseSnapshot: Sendable {
    var sourceId: Int64
    var pathStack: [Int64?]
    var folderNames: [String]
    var items: [SourcePath]
    var selectedPathIds: Set<Int64>
    var searchTerm: String
    var errorMessage: String?
}

@MainActor
class SourceBrowseViewModel: ObservableObject {
    private let service: SourceImportService
    private let sourceId: Int64
    private var readGeneration = UUID()
    let restoredSnapshot: Bool
    @Published var isImporting = false
    /// A stack of visited parentPathIds; the last element is the "current folder."
    @Published private var pathStack: [Int64?] = [nil]

    @Published var items: [SourcePath] = []
    @Published var searchTerm: String = ""
    @Published var errorMessage: String?
    @Published private(set) var folderNames: [String] = []

    /// For checkboxes on any item (folders or files).
    @Published var selectedPathIds = Set<Int64>()

    /// The current parent path we’re displaying.
    var parentPathId: Int64? {
        pathStack.last ?? nil
    }

    /// Whether we can go back one level.
    var canGoBack: Bool {
        pathStack.count > 1
    }

    init(service: SourceImportService, sourceId: Int64, initialParentPathId: Int64? = nil, initialSnapshot: SourceBrowseSnapshot? = nil) {
        self.service = service
        self.sourceId = sourceId
        let valid = initialSnapshot.flatMap { value in
            value.sourceId == sourceId && !value.pathStack.isEmpty && value.items.allSatisfy { $0.sourceId == sourceId } ? value : nil
        }
        restoredSnapshot = valid != nil
        if let valid {
            pathStack = valid.pathStack
            folderNames = valid.folderNames
            items = valid.items
            selectedPathIds = valid.selectedPathIds
            searchTerm = valid.searchTerm
            errorMessage = valid.errorMessage
        } else if let initialParent = initialParentPathId {
            pathStack = [initialParent]
        }
    }

    /// A direct Show-in-folder route starts at its complete source-relative ancestry.
    /// Values come from indexed SQL metadata; no folder/provider is opened here.
    func seedInitialAncestry(paths: [SourcePath], rootPathId: Int64) {
        guard !restoredSnapshot, pathStack.count == 1, let leaf = parentPathId, leaf != rootPathId else { return }
        let indexed = Dictionary(paths.filter { $0.sourceId == sourceId && $0.isDirectory }.map { ($0.pathId, $0) }, uniquingKeysWith: { first, _ in first })
        var reversed: [SourcePath] = []
        var visited = Set<Int64>()
        var cursor: Int64? = leaf
        while let id = cursor, id != rootPathId, visited.insert(id).inserted, let path = indexed[id] {
            reversed.append(path)
            cursor = path.parentPathId
        }
        guard cursor == rootPathId else { return }
        let descendants = reversed.reversed()
        pathStack = [rootPathId] + descendants.map { Optional($0.pathId) }
        folderNames = descendants.map(\.name)
    }

    var snapshot: SourceBrowseSnapshot {
        SourceBrowseSnapshot(sourceId: sourceId, pathStack: pathStack, folderNames: folderNames,
                             items: items, selectedPathIds: selectedPathIds, searchTerm: searchTerm,
                             errorMessage: errorMessage)
    }

    func loadItems() async {
        let generation = UUID()
        readGeneration = generation
        await readItems(generation: generation, search: searchTerm, parent: parentPathId)
    }

    private func scheduleLoad() {
        // Claim ownership now, not when this unstructured task eventually starts.
        // A later explicit refresh or navigation supersedes all older queued work.
        let generation = UUID()
        readGeneration = generation
        let search = searchTerm
        let parent = parentPathId
        Task { await readItems(generation: generation, search: search, parent: parent) }
    }

    private func readItems(generation: UUID, search requestedSearch: String, parent requestedParent: Int64?) async {
        guard !Task.isCancelled, readGeneration == generation,
              searchTerm == requestedSearch, parentPathId == requestedParent else { return }
        do {
            let loaded: [SourcePath]
            if requestedSearch.isEmpty {
                loaded = try await service.listItems(
                    sourceId: sourceId, parentPathId: requestedParent
                ).sorted {
                    if $0.isDirectory != $1.isDirectory { return $0.isDirectory }
                    return $0.name.localizedStandardCompare($1.name) == .orderedAscending
                }
            } else {
                loaded = try await service.search(sourceId: sourceId, query: requestedSearch)
            }
            guard !Task.isCancelled, readGeneration == generation, searchTerm == requestedSearch, parentPathId == requestedParent else { return }
            items = loaded
            errorMessage = nil
        } catch {
            guard !Task.isCancelled, readGeneration == generation, searchTerm == requestedSearch, parentPathId == requestedParent else { return }
            errorMessage = error.localizedDescription
            logger.error("Load items error: \(error)")
        }
    }

    /// Destination data and history become visible in one MainActor turn. A superseding
    /// request or cancelled presentation never installs stale rows or pushes history.
    func navigateIntoFolder(with pathId: Int64) async -> Bool {
        let name = items.first(where: { $0.pathId == pathId })?.name ?? "Folder"
        return await navigate(stack: pathStack + [pathId], names: folderNames + [name])
    }

    func navigateBack() async -> Bool {
        guard canGoBack else { return false }
        return await navigate(stack: Array(pathStack.dropLast()), names: Array(folderNames.dropLast()))
    }

    private func navigate(stack: [Int64?], names: [String]) async -> Bool {
        let generation = UUID()
        readGeneration = generation
        let originalStack = pathStack
        let originalSearch = searchTerm
        var loaded: [SourcePath] = []
        var failure: String?
        do {
            loaded = try await service.listItems(sourceId: sourceId, parentPathId: stack.last ?? nil).sorted {
                if $0.isDirectory != $1.isDirectory { return $0.isDirectory }
                return $0.name.localizedStandardCompare($1.name) == .orderedAscending
            }
        } catch is CancellationError { return false }
        catch { failure = error.localizedDescription }
        guard !Task.isCancelled, readGeneration == generation,
              pathStack == originalStack, searchTerm == originalSearch else { return false }
        pathStack = stack
        folderNames = names
        selectedPathIds = []
        searchTerm = ""
        items = loaded
        errorMessage = failure
        return true
    }

    let logger = Logger(subsystem: subsystem, category: "SourceBrowseViewModel")

    /// Navigate into a subfolder (push on stack).
    func goIntoFolder(with pathId: Int64) {
        selectedPathIds = []
        folderNames.append(items.first(where: { $0.pathId == pathId })?.name ?? "Folder")
        pathStack.append(pathId)
        searchTerm = ""
        scheduleLoad()
    }

    /// Go up one level (pop from stack).
    func goBack() {
        guard canGoBack else { return }
        selectedPathIds = []
        pathStack.removeLast()
        if !folderNames.isEmpty { folderNames.removeLast() }
        searchTerm = ""
        scheduleLoad()
    }

    func resetRoot(to pathId: Int64) {
        pathStack = [pathId]
        folderNames = []
        selectedPathIds = []
        searchTerm = ""
        scheduleLoad()
    }

    /// Toggle selection for a path (folder or file).
    func toggleSelection(_ pathId: Int64) {
        if selectedPathIds.contains(pathId) {
            selectedPathIds.remove(pathId)
        } else {
            selectedPathIds.insert(pathId)
        }
    }
}
