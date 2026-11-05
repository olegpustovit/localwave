#if os(iOS) || os(macOS)
import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  SourceBrowseView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct SourceBrowseView: View {
    let sourceId: Int64
    let parentPathId: Int64?
    let sourceImportService: SourceImportService?
    let songImportService: SongImportService?
    @StateObject var viewModel: SourceBrowseViewModel

    init(
        sourceId: Int64,
        parentPathId: Int64?,
        sourceImportService: SourceImportService?,
        songImportService: SongImportService?,
        viewModel: SourceBrowseViewModel? = nil

    ) {
        self.sourceId = sourceId
        self.parentPathId = parentPathId
        self.sourceImportService = sourceImportService
        self.songImportService = songImportService
        if let vm = viewModel {
            _viewModel = StateObject(wrappedValue: vm)
        } else {
            _viewModel = StateObject(
                wrappedValue: SourceBrowseViewModel(
                    service: sourceImportService!,
                    sourceId: sourceId,
                    initialParentPathId: parentPathId
                ))
        }
    }

    var body: some View {
        if let service: any SourceImportService = sourceImportService,
           let importService = songImportService
        {
            NavigationStack {
                // The SourceBrowseViewInternal (which lists files/folders) remains unchanged.
                SourceBrowseViewInternal(
                    sourceId: sourceId,
                    parentPathId: parentPathId,
                    sourceImportService: service,
                    songImportService: importService
                )
            }
        } else {
            Text("Services not available")
                .foregroundColor(.red)
        }
    }
}

#endif
