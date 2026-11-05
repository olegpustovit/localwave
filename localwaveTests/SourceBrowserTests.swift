import Foundation
import Testing
import LocalWaveDomain
@testable import localwave

@MainActor
struct SourceBrowserTests {
    @Test func asyncNavigationCommitsOnlyCurrentDestinationAndCancellationPreservesHistory() async {
        let service = DeferredBrowserService()
        let model = SourceBrowseViewModel(service: service, sourceId: 1, initialParentPathId: 10)
        let initial = browserPath(20, name: "First")
        model.items = [initial]
        model.selectedPathIds = [20]
        let first = Task { await model.navigateIntoFolder(with: 20) }
        await service.waitForRequest(20)
        // Loading a target must leave the displayed folder, selection and history intact.
        #expect(model.parentPathId == 10)
        #expect(model.items.map(\.pathId) == [20])
        #expect(model.selectedPathIds == [20])
        #expect(!model.canGoBack)
        await service.finish(20, paths: [browserPath(30, name: "Second"), browserPath(40, name: "Newest")])
        #expect(await first.value)
        #expect(model.parentPathId == 20)
        #expect(model.folderNames == ["First"])
        #expect(model.selectedPathIds.isEmpty)

        let obsolete = Task { await model.navigateIntoFolder(with: 30) }
        await service.waitForRequest(30)
        let latest = Task { await model.navigateIntoFolder(with: 40) }
        await service.waitForRequest(40)
        await service.finish(40, paths: [browserPath(41, name: "Current")])
        #expect(await latest.value)
        await service.finish(30, paths: [browserPath(31, name: "Obsolete")])
        #expect(await obsolete.value == false)
        #expect(model.parentPathId == 40)
        #expect(model.items.map(\.pathId) == [41])
        #expect(model.folderNames == ["First", "Newest"])

        let cancelled = Task { await model.navigateIntoFolder(with: 50) }
        await service.waitForRequest(50)
        cancelled.cancel()
        await service.finish(50, paths: [])
        #expect(await cancelled.value == false)
        #expect(model.parentPathId == 40)
        #expect(model.folderNames == ["First", "Newest"])
        let back = Task { await model.navigateBack() }
        await service.waitForRequest(20)
        await service.finish(20, paths: [initial])
        #expect(await back.value)
        #expect(model.parentPathId == 20)
        #expect(model.folderNames == ["First"])
    }

    @Test func directFolderRouteSeedsIndexedAncestryButSnapshotKeepsItsPrivateHistory() async {
        let service = BrowserService()
        let artist = SourcePath(id: nil, sourceId: 1, pathId: 20, parentPathId: 10, name: "Artist", relativePath: "Artist", isDirectory: true, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil)
        let album = SourcePath(id: nil, sourceId: 1, pathId: 40, parentPathId: 20, name: "Album", relativePath: "Artist/Album", isDirectory: true, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil)
        let model = SourceBrowseViewModel(service: service, sourceId: 1, initialParentPathId: 40)
        model.seedInitialAncestry(paths: [artist, album], rootPathId: 10)
        #expect(model.snapshot.pathStack == [10, 20, 40])
        #expect(model.folderNames == ["Artist", "Album"])
        #expect(await service.requestedParents.isEmpty)
        let restored = SourceBrowseViewModel(service: service, sourceId: 1, initialSnapshot: model.snapshot)
        restored.seedInitialAncestry(paths: [], rootPathId: 99)
        #expect(restored.snapshot.pathStack == [10, 20, 40])
        restored.goBack()
        #expect(restored.parentPathId == 20)
        restored.goBack()
        #expect(restored.parentPathId == 10)
    }

    @Test func snapshotRestoresPrivateHistorySelectionAndRowsWithoutReadingProvider() async {
        let service = BrowserService()
        let snapshot = SourceBrowseSnapshot(sourceId: 1, pathStack: [10, 20, 40],
            folderNames: ["First", "Newest"], items: [browserPath(41, name: "Current")],
            selectedPathIds: [41], searchTerm: "Current", errorMessage: "Folder read failed")
        let restored = SourceBrowseViewModel(service: service, sourceId: 1, initialParentPathId: 10, initialSnapshot: snapshot)
        #expect(restored.restoredSnapshot)
        #expect(restored.parentPathId == 40)
        #expect(restored.canGoBack)
        #expect(restored.items.map(\.pathId) == [41])
        #expect(restored.selectedPathIds == [41])
        #expect(restored.searchTerm == "Current")
        #expect(restored.errorMessage == "Folder read failed")
        #expect(await service.requestedParents.isEmpty)
        restored.goBack()
        #expect(restored.parentPathId == 20)
        #expect(restored.folderNames == ["First"])
        restored.goBack()
        #expect(restored.parentPathId == 10)
        #expect(!restored.canGoBack)
        #expect(snapshot.pathStack == [10, 20, 40])
        #expect(snapshot.selectedPathIds == [41])
        // A cached page from another source must never install its rows or history.
        let other = SourceBrowseViewModel(service: service, sourceId: 2, initialParentPathId: 50, initialSnapshot: snapshot)
        #expect(!other.restoredSnapshot)
        #expect(other.parentPathId == 50)
        #expect(other.items.isEmpty)
    }

    @Test func backStopsAtSelectedSourceRootAndSelectionToggles() async {
        let service = BrowserService()
        let model = SourceBrowseViewModel(service: service, sourceId: 1, initialParentPathId: 10)
        #expect(!model.canGoBack)
        #expect(model.parentPathId == 10)
        model.goIntoFolder(with: 20)
        #expect(model.canGoBack)
        #expect(model.parentPathId == 20)
        model.goBack()
        #expect(!model.canGoBack)
        #expect(model.parentPathId == 10)
        model.goBack()
        #expect(model.parentPathId == 10)
        model.toggleSelection(3)
        #expect(model.selectedPathIds == [3])
        model.toggleSelection(3)
        #expect(model.selectedPathIds.isEmpty)
        // Both immediate wrappers queued reads before this newer explicit request.
        // Awaiting the newer request must establish its error before returning.
        await model.loadItems()
        #expect(model.errorMessage == "Folder is unavailable")
        #expect(await service.requestedParents == [10])
    }
}

private actor BrowserService: SourceImportService {
    private(set) var requestedParents: [Int64?] = []
    func listItems(sourceId: Int64, parentPathId: Int64?) async throws -> [SourcePath] {
        requestedParents.append(parentPathId)
        throw NSError(domain: "SourceBrowserTests", code: 1, userInfo: [NSLocalizedDescriptionKey: "Folder is unavailable"])
    }
    func search(sourceId: Int64, query: String) async throws -> [SourcePath] { [] }
    func deleteOne(sourceId: Int64) async throws { }
}

private func browserPath(_ id: Int64, name: String) -> SourcePath {
    SourcePath(id: nil, sourceId: 1, pathId: id, parentPathId: 10, name: name,
               relativePath: name, isDirectory: true, fileHashSHA256: nil,
               runId: 1, createdAt: Date(), updatedAt: nil)
}

private actor DeferredBrowserService: SourceImportService {
    private var pending: [Int64: CheckedContinuation<[SourcePath], Never>] = [:]
    private var waiting: [Int64: [CheckedContinuation<Void, Never>]] = [:]

    func listItems(sourceId: Int64, parentPathId: Int64?) async throws -> [SourcePath] {
        let id = parentPathId ?? -1
        return await withCheckedContinuation { continuation in
            pending[id] = continuation
            for observer in waiting.removeValue(forKey: id) ?? [] { observer.resume() }
        }
    }
    func waitForRequest(_ id: Int64) async {
        if pending[id] != nil { return }
        await withCheckedContinuation { waiting[id, default: []].append($0) }
    }
    func finish(_ id: Int64, paths: [SourcePath]) {
        pending.removeValue(forKey: id)?.resume(returning: paths)
    }
    func search(sourceId: Int64, query: String) async throws -> [SourcePath] { [] }
    func deleteOne(sourceId: Int64) async throws { }
}
