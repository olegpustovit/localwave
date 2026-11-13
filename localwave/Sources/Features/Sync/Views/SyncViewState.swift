import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  SyncViewState.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//
import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

enum SyncViewState {
    case noICloud, isInitialising,
         noSourceDirSet, notSyncedYet,
         showTreeView, syncInProgress, unboundView
}

@MainActor
class SyncViewModel: ObservableObject {
    @Published var sources: [Source] = []
    @Published var createdUser: User?
    @Published var errorMessage: String?
    @Published var currentSyncSourceId: Int64?

    @Published var selectedFolderName: String? = nil
    @Published var currentSource: Source?
    @Published var isSyncing = false
    @Published var currentSyncedDir: String? = nil
    @Published var importProgress: Double?
    @Published var currentImportedFile = ""
    @Published var scannedItemCount = 0
    /// Background tag reading for cloud-only songs, e.g. "Reading tags 12/1838"; nil when idle.
    @Published var tagStatus: String?
    @Published var importedFileCount: Int?
    @Published var importFileCount: Int?
    private var scanProgressGeneration: UUID?
    private var lastImportPublication = Date.distantPast.timeIntervalSinceReferenceDate
    private var sourceWork: Task<Void, Never>?
    private var foregroundRefreshQueued = false

    private let userCloudService: UserCloudService?
    private let icloudProvider: ICloudProvider?
    let sourceService: SourceService?
    let songImportService: SongImportService?
    private let sourceAccess: SourceAccess?
    private var sourceWatchTask: Task<Void, Never>?
    private var materializationWatchTask: Task<Void, Never>?
    private var queuedSourceChanges = Set<Int64>()
    private var repeatedSourceChanges = Set<Int64>()
    private var reconciledSources = Set<Int64>()

    init(
        userCloudService: UserCloudService?,
        icloudProvider: ICloudProvider?,
        sourceService: SourceService?,
        songImportService: SongImportService?,
        sourceAccess: SourceAccess? = nil
    ) {
        self.userCloudService = userCloudService
        self.icloudProvider = icloudProvider
        self.sourceService = sourceService
        self.songImportService = songImportService
        self.sourceAccess = sourceAccess
        if let sourceAccess {
            sourceWatchTask = Task { [weak self] in
                let changes = await sourceAccess.changes()
                for await id in changes {
                    guard !Task.isCancelled, let self else { return }
                    self.sourceChanged(id)
                }
            }
            materializationWatchTask = Task { [weak self] in
                for await notification in NotificationCenter.default.notifications(named: Notification.Name("LWSourceMaterialized")) {
                    guard !Task.isCancelled, let self else { return }
                    // Opening a track only copies it to the cache; rescanning the whole folder for that froze the UI. Just refresh lists.
                    if notification.object is Int64 { NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil) }
                }
            }
        }
    }

    deinit { sourceWatchTask?.cancel(); materializationWatchTask?.cancel() }

    private func sourceChanged(_ id: Int64) {
        guard sources.contains(where: { $0.id == id }) else { return }
        guard queuedSourceChanges.insert(id).inserted else { repeatedSourceChanges.insert(id); return }
        let previous = sourceWork
        sourceWork = Task {
            await previous?.value
            defer {
                queuedSourceChanges.remove(id)
                if repeatedSourceChanges.remove(id) != nil { sourceChanged(id) }
            }
            guard let source = sources.first(where: { $0.id == id }) else { return }
            let changedPaths = await sourceAccess?.takeChangedPaths(sourceId: id)
            do { try await syncSource(source, changedPaths: source.syncError == nil ? changedPaths : nil) }
            catch is CancellationError { }
            catch { await recordFailure(error, source: source) }
        }
    }

    func loadSources() {
        Task {
            do {
                guard let currentUser = try await userCloudService?.resolveCurrentICloudUser()
                else {
                    errorMessage = "User not logged in"
                    return
                }

                sources =
                    try await sourceService?.repository()
                        .findOneByUserId(userId: currentUser.id ?? -1, path: nil) ?? []
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    /// Reconcile once per launch, then let folder notifications drive updates. `force` is the explicit rescan.
    func refreshSources(force: Bool = false) {
        guard !foregroundRefreshQueued else { return }
        foregroundRefreshQueued = true
        let previous = sourceWork
        sourceWork = Task {
            await previous?.value
            defer { foregroundRefreshQueued = false }
            do {
                guard let user = try await userCloudService?.resolveCurrentICloudUser() else { return }
                sources = try await sourceService?.repository().findOneByUserId(userId: user.id ?? -1, path: nil) ?? []
                for source in sources {
                    if !force, source.syncError == nil, let id = source.id, reconciledSources.contains(id) { continue }
                    do { try await syncSource(source) }
                    catch is CancellationError { }
                    catch { await recordFailure(error, source: source) }
                }
            } catch { errorMessage = error.localizedDescription }
        }
    }

    private func recordFailure(_ error: Error, source: Source) async {
        var updated = sources.first(where: { $0.id == source.id }) ?? source
        updated.syncError = error.localizedDescription
        LWPresentationNotice.error("Couldn’t sync \(URL(fileURLWithPath: source.dirPath).lastPathComponent): \(error.localizedDescription)")
        if let index = sources.firstIndex(where: { $0.id == source.id }) { sources[index] = updated }
        guard let sourceService else { return }
        do {
            _ = try await sourceService.repository().updateSource(source: updated)
            // Observers reload persisted state; publish only after the failure is committed.
            NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        } catch { errorMessage = error.localizedDescription }
    }

    func cancelCurrentImport() {
        Task { await songImportService?.cancelImport() }
    }

    func awaitPendingWork() async {
        await sourceWork?.value
    }

    func deleteSource(_ source: Source) {
        Task {
            if let id = source.id {
                do {
                    if currentSyncSourceId == id { await songImportService?.cancelImport() }
                    if let sourceAccess { try await sourceAccess.remove(sourceId: id) }
                    else { try await sourceService?.repository().deleteSource(sourceId: id) }
                    // Also, remove the source from the local list.
                    sources.removeAll { $0.id == id }
                    NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
                } catch {
                    logger.error("Failed to delete source: \(error)")
                    errorMessage = error.localizedDescription
                }
            }
        }
    }

    func createSource(path: String) {
        let previous = sourceWork
        sourceWork = Task {
            await previous?.value
            do {
                guard let currentUser = try await userCloudService?.resolveCurrentICloudUser(),
                      let service = sourceService
                else {
                    errorMessage = "User is not available"
                    logger.error("failed to create source: user is not available")
                    return
                }

                let source = try await service.registerSourcePath(
                    userId: currentUser.id ?? -1,
                    path: path,
                    type: .iCloud
                )
                if let id = source.id, let bookmark = LocalWavePreferences.current.data(forKey: makeBookmarkKey(makeURLFromString(path))) {
                    _ = try await service.repository().updateSourceBookmark(sourceId: id, bookmark: bookmark, expectedBookmark: source.bookmark)
                }
                logger.debug("source path \(path) is registered, now syncing...")

                if !sources.contains(where: { $0.id == source.id }) { sources.append(source) }
                try await syncSource(source)
            } catch is CancellationError {
            } catch let CustomError.genericError(msg) {
                errorMessage = msg
                logger.error("failed to register or sync source: \(msg)")
            } catch {
                errorMessage = error.localizedDescription
                logger.error("failed to register or sync source: \(error.localizedDescription)")
            }
        }
    }

    func resyncSource(_ source: Source) {
        let previous = sourceWork
        sourceWork = Task {
            await previous?.value
            do {
                try await syncSource(source)
                loadSources() // Refresh list
            } catch is CancellationError {
            } catch let CustomError.genericError(msg) {
                // TODO: Need to inform user to remove this source and re-add it
                logger.error("loaded error: \(msg)")
                errorMessage = msg
                await recordFailure(CustomError.genericError(msg), source: source)
            } catch {
                logger.error("loaded error: \(error.localizedDescription)")
                errorMessage = error.localizedDescription
                await recordFailure(error, source: source)
            }
        }
    }

    private func syncSource(_ source: Source, changedPaths: Set<String>? = nil) async throws {
        guard let id = source.id, let sourceAccess else { try await syncSourceContents(source, changedPaths: changedPaths); return }
        await sourceAccess.beginScan(sourceId: id)
        do {
            try await syncSourceContents(source, changedPaths: changedPaths)
            await sourceAccess.endScan(sourceId: id)
        } catch {
            await sourceAccess.endScan(sourceId: id)
            throw error
        }
    }

    private func syncSourceContents(_ source: Source, changedPaths: Set<String>?) async throws {
        if changedPaths?.isEmpty == true { return }
        currentSyncSourceId = source.id
        scannedItemCount = 0
        importedFileCount = nil
        importFileCount = nil
        lastImportPublication = Date.distantPast.timeIntervalSinceReferenceDate
        let generation = UUID()
        scanProgressGeneration = generation
        let counter = OSAllocatedUnfairLock(initialState: (count: 0, publishedAt: Date.distantPast.timeIntervalSinceReferenceDate))
        defer {
            scanProgressGeneration = nil
            currentSyncSourceId = nil
            scannedItemCount = 0
            importedFileCount = nil
            importFileCount = nil
            importProgress = nil
            currentImportedFile = ""
            NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
            NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
        }

        guard let sourceId = source.id else {
            throw CustomError.genericError("Invalid source ID")
        }

        let folderURL = makeURLFromString(source.dirPath)
        let updatedSource: Source?
        let incrementalPaths: [SourcePath]?
        if let changedPaths, let sourceService {
            let result = try await sourceService.syncService().syncPaths(sourceId: sourceId, relativePaths: changedPaths)
            updatedSource = result.source
            incrementalPaths = result.audioPaths
        } else {
            incrementalPaths = nil
            updatedSource = try await sourceService?.syncService().syncDir(
            sourceId: sourceId,
            folderURL: folderURL,
            onCurrentURL: { url in
                guard url != nil else { return }
                let count = counter.withLock { state -> Int? in
                    state.count += 1
                    let now = Date.timeIntervalSinceReferenceDate
                    guard state.count == 1 || now - state.publishedAt >= 0.1 else { return nil }
                    state.publishedAt = now
                    return state.count
                }
                if let count {
                    Task { @MainActor [weak self] in
                        guard let self, scanProgressGeneration == generation, importProgress == nil else { return }
                        scannedItemCount = count
                    }
                }
            },
            onSetLoading: { _ in }
        )
        }

        scannedItemCount = counter.withLock { $0.count }
        if let updated = updatedSource {
            if let index = sources.firstIndex(where: { $0.id == source.id }) {
                sources[index] = updated
            }
            if let error = updated.syncError { throw CustomError.genericError(error) }
        }
        if let sourceService, let songImportService, incrementalPaths?.isEmpty != true {
            let paths: [SourcePath]
            if let incrementalPaths { paths = incrementalPaths }
            else { paths = try await sourceService.importService().listItems(sourceId: sourceId, parentPathId: updatedSource?.pathId ?? source.pathId) }
            importProgress = 0
            do {
                // Throttle off the main thread and don't await the hop: a busy UI (scrolling) must never stall indexing.
                let throttle = OSAllocatedUnfairLock(initialState: Date.distantPast.timeIntervalSinceReferenceDate)
                try await songImportService.importPathsWithFileProgress(paths: paths) { progress in
                    let now = Date.timeIntervalSinceReferenceDate
                    guard throttle.withLock({ last in
                        guard progress.percentage >= 100 || now - last >= 0.25 else { return false }
                        last = now; return true
                    }) else { return }
                    Task { @MainActor in
                        guard self.scanProgressGeneration == generation else { return }
                        self.importedFileCount = progress.completedFiles
                        self.importFileCount = progress.totalFiles
                        self.importProgress = progress.percentage
                        self.currentImportedFile = progress.url.lastPathComponent
                    }
                }
            } catch is CancellationError { throw CancellationError() }
            catch {
                await recordFailure(error, source: source)
                throw error
            }
        }
        reconciledSources.insert(sourceId)
    }

    var state: SyncViewState {
        if !hasICloud() {
            return .noICloud
        } else if hasICloud() && (createdUser == nil && errorMessage == nil) {
            return .isInitialising
        } else if createdUser != nil && currentSource == nil {
            return .noSourceDirSet
        } else if createdUser != nil && currentSource != nil && currentSource?.lastSyncedAt == nil
            && !isSyncing
        {
            return .notSyncedYet
        } else if createdUser != nil && currentSource != nil && currentSource?.lastSyncedAt != nil
            && !isSyncing
        {
            return .showTreeView
        } else if isSyncing {
            return .syncInProgress
        }

        return .unboundView
    }

    let logger = Logger(subsystem: subsystem, category: "SyncViewModel")

    func registerBookmark(_ folderURL: URL) throws {
        guard !LocalWaveGeneratedPaths.contains(folderURL) else {
            throw CustomError.genericError("Choose your original music folder. LocalWave’s generated cache cannot be used as a source.")
        }
        let scoped = folderURL.startAccessingSecurityScopedResource()
        guard scoped || FileManager.default.isReadableFile(atPath: folderURL.path) else {
            throw CustomError.genericError("Unable to access this folder. Choose it again in Files.")
        }
        defer { if scoped { folderURL.stopAccessingSecurityScopedResource() } }
        let bookmarkKey = makeBookmarkKey(folderURL)

        let bookmarkData = try folderURL.bookmarkData(
            options: sourceBookmarkCreationOptions,
            includingResourceValuesForKeys: nil,
            relativeTo: nil
        )
        logger.debug("Setting bookmark key \(bookmarkKey) of \(folderURL.absoluteString)")

        LocalWavePreferences.current.set(bookmarkData, forKey: bookmarkKey)
    }

    func hasICloud() -> Bool {
        return icloudProvider?.isICloudAvailable() ?? false
    }

    func initialise() {
        if userCloudService == nil {
            errorMessage = "service is not available"
        }

        Task { @MainActor in
            do {
                let user = try await userCloudService?.resolveCurrentICloudUser()
                self.createdUser = user
                if let user = user {
                    self.currentSource = try await sourceService?.getCurrentSource(
                        userId: user.id!)
                    self.selectedFolderName = self.currentSource?.dirPath
                    let id = self.currentSource?.id ?? -1
                    let path = self.currentSource?.dirPath ?? ""
                    logger.debug("source \(id), path: \(path)")
                }
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }
}
