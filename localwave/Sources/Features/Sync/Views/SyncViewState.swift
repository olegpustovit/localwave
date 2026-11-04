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

    private let userCloudService: UserCloudService?
    private let icloudProvider: ICloudProvider?
    let sourceService: SourceService?
    let songImportService: SongImportService?

    init(
        userCloudService: UserCloudService?,
        icloudProvider: ICloudProvider?,
        sourceService: SourceService?,
        songImportService: SongImportService?
    ) {
        self.userCloudService = userCloudService
        self.icloudProvider = icloudProvider
        self.sourceService = sourceService
        self.songImportService = songImportService
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

    func deleteSource(_ source: Source) {
        Task {
            if let id = source.id {
                do {
                    try await sourceService?.repository().deleteSource(sourceId: id)
                    // Also, remove the source from the local list.
                    sources.removeAll { $0.id == id }
                } catch {
                    logger.error("Failed to delete source: \(error)")
                }
            }
        }
    }

    func createSource(path: String) {
        Task {
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
                logger.debug("source path \(path) is registered, now syncing...")

                sources.append(source)
                try await syncSource(source)
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
        Task {
            do {
                try await syncSource(source)
                loadSources() // Refresh list
            } catch let CustomError.genericError(msg) {
                // TODO: Need to inform user to remove this source and re-add it
                logger.error("loaded error: \(msg)")
                errorMessage = msg
            } catch {
                logger.error("loaded error: \(error.localizedDescription)")
                errorMessage = error.localizedDescription
            }
        }
    }

    private func syncSource(_ source: Source) async throws {
        currentSyncSourceId = source.id
        defer {
            currentSyncSourceId = nil
            NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
        }

        guard let sourceId = source.id else {
            throw CustomError.genericError("Invalid source ID")
        }

        let folderURL = try resolveSourceURL(source)
        let updatedSource = try await sourceService?.syncService().syncDir(
            sourceId: sourceId,
            folderURL: folderURL,
            onCurrentURL: { _ in },
            onSetLoading: { _ in }
        )

        if let updated = updatedSource {
            if let index = sources.firstIndex(where: { $0.id == source.id }) {
                sources[index] = updated
            }
        }
    }

    private func resolveSourceURL(_ source: Source) throws -> URL {
        let folderURL = makeURLFromString(source.dirPath)
        let bookmarkKey = makeBookmarkKey(folderURL)
        logger.debug("Loading bookmark key \(bookmarkKey) of \(folderURL.absoluteString)")
        guard let bookmarkData = UserDefaults.standard.data(forKey: bookmarkKey) else {
            throw CustomError.genericError("Missing bookmark data")
        }

        var isStale = false
        return try URL(
            resolvingBookmarkData: bookmarkData,
            options: [],
            relativeTo: nil,
            bookmarkDataIsStale: &isStale
        )
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

    func registerPath(_ path: String) {
        Task {
            logger.debug("registering \(path)")
            do {
                if let currentUser = self.createdUser {
                    let lib = try await sourceService?.registerSourcePath(
                        userId: currentUser.id!, path: path, type: .iCloud
                    )
                    let libId = lib?.id ?? -1
                    logger.debug("created source \(libId)")
                    self.currentSource = lib
                }
            } catch {
                logger.debug("failed to register lib \(error.localizedDescription)")
            }

            logger.debug("source is set...")
        }
    }

    func registerBookmark(_ folderURL: URL) throws {
        guard folderURL.startAccessingSecurityScopedResource() else {
            logger.error("Unable to access security scoped resource.")
            return
        }
        defer { folderURL.stopAccessingSecurityScopedResource() }
        let bookmarkKey = makeBookmarkKey(folderURL)

        let bookmarkData = try folderURL.bookmarkData(
            options: [],
            includingResourceValuesForKeys: nil,
            relativeTo: nil
        )
        logger.debug("Setting bookmark key \(bookmarkKey) of \(folderURL.absoluteString)")

        UserDefaults.standard.set(bookmarkData, forKey: bookmarkKey)
    }

    func sync() {
        Task {
            self.isSyncing = true
            var currentSrc = self.currentSource
            do {
                // Start syncing with updates
                let folderPath = currentSource?.dirPath
                let sourceId = currentSource?.id
                logger.debug("started syncing...")
                if folderPath != nil && sourceId != nil {
                    let result = try await sourceService?.syncService().syncDir(
                        sourceId: sourceId!, folderURL: makeURLFromString(folderPath!),
                        onCurrentURL: { url in
                            DispatchQueue.main.async {
                                self.currentSyncedDir = url?.absoluteString
                            }
                        },
                        onSetLoading: { loading in
                            DispatchQueue.main.async {
                                self.isSyncing = loading
                            }
                        }
                    )
                    currentSrc?.totalPaths = result?.totalPaths
                } else {
                    logger.error("failed to sync")
                }
                self.isSyncing = false
                currentSrc?.lastSyncedAt = Date()
                currentSrc = try await sourceService?.repository().updateSource(
                    source: currentSrc!)
                logger.debug("finished syncing...")
                self.currentSource = currentSrc
            } catch {
                self.isSyncing = false
                currentSrc?.lastSyncedAt = Date()
                currentSrc?.syncError = error.localizedDescription
                currentSrc = try await sourceService?.repository().updateSource(
                    source: currentSrc!)
                logger.debug("finished with error")
                self.currentSource = currentSrc
            }
        }
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
