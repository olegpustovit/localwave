//
//  SourceBrowseViewInternal.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct SourceBrowseViewInternal: View {
    @StateObject var viewModel: SourceBrowseViewModel
    @State private var showImportProgress = false
    @State private var importProgress: Double = 0
    @State private var currentFileName: String = ""
    let songImportService: SongImportService

    init(
        sourceId: Int64,
        parentPathId: Int64? = nil,
        sourceImportService: SourceImportService,
        songImportService: SongImportService
    ) {
        _viewModel = StateObject(
            wrappedValue: SourceBrowseViewModel(
                service: sourceImportService,
                sourceId: sourceId,
                initialParentPathId: parentPathId
            )
        )
        self.songImportService = songImportService
    }

    private var logger = Logger(subsystem: subsystem, category: "SourceBrowsViewInternal")
        
    private func importSelection(selectedPaths: [SourcePath]) async {
        viewModel.isImporting = true
        showImportProgress = true
        defer {
            viewModel.isImporting = false
            showImportProgress = false
            NotificationCenter.default.post(
                name: Notification.Name("LibraryRefresh"), object: nil
            )
        }
        
        do {
            try await songImportService.importPaths(
                paths: selectedPaths,
                onProgress: { pct, fileURL in
                    await MainActor.run {
                        importProgress = pct
                        currentFileName = fileURL.lastPathComponent
                    }
                }
            )
            // Clear selection only if completed successfully
            viewModel.selectedPathIds = []
        } catch {
            logger.error("Import error: \(error)")
            // Don't clear selection if cancelled
            if !(error is CancellationError) {
                viewModel.selectedPathIds = []
            }
        }
        
    }
    
    var body: some View {
        VStack {
            VStack {
                // Top bar with optional "Back" button
                HStack {
                    if viewModel.canGoBack {
                        Button("Back") {
                            viewModel.goBack()
                        }
                        .padding(.leading)
                    }
                    Spacer()
                    if viewModel.selectedPathIds.count > 0 {
                        Button(
                            viewModel.isImporting
                                ? "Importing..." : "Import \(viewModel.selectedPathIds.count) items"
                        ) {
                            guard !viewModel.isImporting else { return }
                            let selectedPaths = viewModel.items.filter {
                                viewModel.selectedPathIds.contains($0.pathId)
                            }
                            Task { await importSelection(selectedPaths: selectedPaths) }
                        }
                        .disabled(viewModel.selectedPathIds.isEmpty || viewModel.isImporting)
                    } else {
                        Button("Import all") {
                            Task { await importSelection(selectedPaths: viewModel.items) }
                        }
                    }
                }
                if showImportProgress {
                    VStack {
                        if viewModel.isImporting {
                            Text("Importing \(currentFileName) ...")
                            ProgressView(value: importProgress, total: 100)
                            Button("Cancel Import") {
                                Task {
                                    await songImportService.cancelImport()
                                    showImportProgress = false
                                }
                            }
                            .padding()
                        } else {
                            Text(importProgress >= 100 ? "Complete!" : "Cancelled")
                        }
                    }
                    .padding()
                }

                SearchBar(
                    text: $viewModel.searchTerm,
                    onChange: { _ in
                        Task { await viewModel.loadItems() }
                    }, placeholder: "Search paths...", debounceSeconds: 0.1
                )
                .padding(.horizontal)
                // File/Folder list
                List(viewModel.items, id: \.pathId) { item in
                    HStack {
                        // Icon: folder or doc
                        Image(systemName: item.isDirectory ? "folder.fill" : "doc.fill")
                            .foregroundColor(.gray)

                        // Name + relative path
                        VStack(alignment: .leading) {
                            Text(item.name)
                                .fontWeight(.medium)
                            Text(item.relativePath)
                                .font(.caption)
                                .foregroundColor(.gray)
                                .lineLimit(1)
                        }

                        Spacer()

                        // Checkboxes on everything (folder or file)
                        Button {
                            viewModel.toggleSelection(item.pathId)
                        } label: {
                            Image(
                                systemName: viewModel.selectedPathIds.contains(item.pathId)
                                    ? "checkmark.square"
                                    : "square")
                        }
                        .buttonStyle(BorderlessButtonStyle())
                    }
                    .contentShape(Rectangle()) // Entire row is tappable
                    .onTapGesture {
                        if item.isDirectory {
                            viewModel.goIntoFolder(with: item.pathId)
                        }
                    }
                }
                .listStyle(.plain)
            }
            .navigationTitle("Source Browser")
        }
        .onAppear {
            Task { await viewModel.loadItems() }
        }
    }
}
