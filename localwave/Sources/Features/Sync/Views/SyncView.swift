import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  SyncView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct SyncView: View {
    @State private var showingFolderPicker = false
    private let logger = Logger(subsystem: subsystem, category: "SyncView")
    @StateObject private var syncViewModel: SyncViewModel
    private var dependencies: DependencyContainer

    @State private var showGrid: Bool = false

    init(dependencies: DependencyContainer) {
        _syncViewModel = StateObject(wrappedValue: dependencies.makeSyncViewModel())
        self.dependencies = dependencies
    }

    var body: some View {
        NavigationStack {
            contentView
                .navigationTitle("Music Sources")
                .toolbar {
                    // Always allow adding a source.
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button {
                            showingFolderPicker = true
                        } label: {
                            Image(systemName: "plus")
                        }
                    }
                }
                .fileImporter(
                    isPresented: $showingFolderPicker,
                    allowedContentTypes: [.folder],
                    allowsMultipleSelection: false
                ) { result in
                    handleFolderSelection(result: result)
                }
                .onAppear {
                    syncViewModel.loadSources()
                }
        }
    }

    @ViewBuilder
    private var contentView: some View {
        if syncViewModel.sources.isEmpty {
            emptyStateView
        }
        // If there is only one source and we are not forcing grid view...
        else if syncViewModel.sources.count == 1 && !showGrid {
            if let singleSource = syncViewModel.sources.first,
               let sourceId = singleSource.id
            {
                let browseVM =
                    dependencies.sourceBrowseViewModels[sourceId]
                        ?? SourceBrowseViewModel(
                            service: syncViewModel.sourceService!.importService(),
                            sourceId: sourceId,
                            initialParentPathId: singleSource.pathId
                        )
                SourceBrowseView(
                    sourceId: sourceId,
                    parentPathId: singleSource.pathId,
                    sourceImportService: syncViewModel.sourceService?.importService(),
                    songImportService: syncViewModel.songImportService,
                    viewModel: browseVM
                )
                .onAppear {
                    DispatchQueue.main.async {
                        dependencies.sourceBrowseViewModels[sourceId] = browseVM
                    }
                }
                .toolbar {
                    ToolbarItem(placement: .navigationBarLeading) {
                        Button("Grid") {
                            showGrid = true
                        }
                    }
                }
            }
        } else {
            sourceGridView
                .toolbar {
                    ToolbarItem(placement: .navigationBarLeading) {
                        Button("Back to Grid") {
                            showGrid = true
                        }
                    }
                }
        }
    }

    private var sourceGridView: some View {
        ScrollView {
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 200), spacing: 20)], spacing: 20) {
                ForEach(syncViewModel.sources, id: \.stableId) { (source: Source) in
                    NavigationLink {
                        if let sourceId = source.id {
                            let browseVM =
                                dependencies.sourceBrowseViewModels[sourceId]
                                    ?? SourceBrowseViewModel(
                                        service: syncViewModel.sourceService!.importService(),
                                        sourceId: sourceId,
                                        initialParentPathId: source.pathId
                                    )
                            SourceBrowseView(
                                sourceId: sourceId,
                                parentPathId: source.pathId,
                                sourceImportService: syncViewModel.sourceService?.importService(),
                                songImportService: syncViewModel.songImportService,
                                viewModel: browseVM
                            )
                            .onAppear {
                                DispatchQueue.main.async {
                                    dependencies.sourceBrowseViewModels[sourceId] = browseVM
                                }
                            }
                        }
                    } label: {
                        SourceGridCell(
                            source: source,
                            isSyncing: syncViewModel.currentSyncSourceId == source.id,
                            onResync: {
                                logger.debug(
                                    "resyncing source: \(source.id ?? -1), path: \(source.dirPath)"
                                )
                                syncViewModel.resyncSource(source)
                            },
                            onDelete: {
                                syncViewModel.deleteSource(source)
                            }
                        )
                        .padding()
                        .background(Color(.secondarySystemBackground))
                        .cornerRadius(12)
                    }
                    .buttonStyle(PlainButtonStyle())
                }
            }
            .padding()
        }
    }

    private var emptyStateView: some View {
        VStack(spacing: 20) {
            Image(systemName: "folder.badge.plus")
                .font(.system(size: 60))
                .foregroundColor(.blue)
            Text("No Sources Added")
                .font(.title2)
            Text("Get started by adding your first music source from iCloud")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
            Button("Add iCloud Source") {
                showingFolderPicker = true
            }
            .buttonStyle(.borderedProminent)
        }
        .padding()
        .frame(maxHeight: .infinity)
    }

    private func handleFolderSelection(result: Result<[URL], Error>) {
        switch result {
        case let .success(urls):
            guard let url = urls.first else { return }
            do {
                logger.debug("new path is getting synced: \(url)")
                try syncViewModel.registerBookmark(url)
                syncViewModel.createSource(path: url.path)

                showGrid = true
            } catch {
                logger.error("Folder selection error: \(error.localizedDescription)")
            }
        case let .failure(error):
            logger.error("Folder picker error: \(error.localizedDescription)")
        }
    }
}
