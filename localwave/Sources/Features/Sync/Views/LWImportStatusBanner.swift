#if os(iOS) || os(macOS)
import SwiftUI

/// Shows folder scan/index progress anywhere in the app; the same state otherwise only renders inside the Folders screen.
struct LWImportStatusBanner: View {
    @ObservedObject var model: SyncViewModel
    var picking = false

    private var text: String? {
        if let percent = model.importProgress, percent.isFinite { return "Indexing audio \(Int(percent.rounded()))%" }
        if model.currentSyncSourceId != nil { return "Scanning folder… \(model.scannedItemCount) items" }
        return picking ? "Adding music…" : model.tagStatus
    }

    var body: some View {
        if let text {
            HStack(spacing: 10) {
                ProgressView().controlSize(.small)
                Text(text).font(LWTheme.data(12))
            }
            .padding(.horizontal, 14).frame(height: 36)
            .background(LWTheme.ink).foregroundStyle(LWTheme.onInk)
            .padding(16).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
            .padding(.bottom, 64).allowsHitTesting(false)
            .accessibilityElement(children: .combine)
        }
    }
}
#endif
