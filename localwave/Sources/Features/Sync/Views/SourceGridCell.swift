import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  SourceGridCell.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct SourceGridCell: View {
    let source: Source
    let isSyncing: Bool
    let onResync: () -> Void
    let onDelete: () -> Void

    private var lastSyncText: String {
        let formatter = DateFormatter()
        formatter.dateStyle = .short
        formatter.timeStyle = .short
        formatter.doesRelativeDateFormatting = true
        guard let date = source.lastSyncedAt else { return "Never synced" }
        return "Last sync: \(formatter.string(from: date))"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Image(systemName: sourceTypeIcon)
                    .font(.title)
                    .foregroundColor(sourceTypeColor)

                VStack(alignment: .leading) {
                    Text(dirName)
                        .font(.headline)
                        .lineLimit(1)
                    Text(source.dirPath)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }

                Spacer()

                if isSyncing {
                    ProgressView()
                } else {
                    Button(action: onResync) {
                        Image(systemName: "arrow.clockwise")
                    }
                }
            }

            Text(lastSyncText)
                .font(.caption2)
                .foregroundColor(.secondary)
        }
        .padding()
        .background(Color(.tertiarySystemBackground))
        .cornerRadius(8)
        .contextMenu {
            Button(role: .destructive) {
                onDelete()
            } label: {
                Label("Delete Source", systemImage: "trash")
            }
        }
    }

    private var dirName: String {
        return makeURLFromString(source.dirPath).lastPathComponent
    }

    private var sourceTypeIcon: String {
        switch source.type {
        case .iCloud: return "icloud.fill"
        default: return "folder.fill"
        }
    }

    private var sourceTypeColor: Color {
        .gray
    }
}
