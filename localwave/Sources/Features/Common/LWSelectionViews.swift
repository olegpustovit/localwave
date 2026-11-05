#if os(iOS) || os(macOS)
import SwiftUI

struct LWCompactSelectionDeck<Content: View>: View {
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @ViewBuilder let content: () -> Content

    var body: some View {
        Group {
            if dynamicTypeSize.isAccessibilitySize {
                LazyVGrid(columns: [GridItem(.flexible(), spacing: 2), GridItem(.flexible(), spacing: 2)], spacing: 2) { content() }
            } else {
                HStack(spacing: 2) { content() }
            }
        }
        .buttonStyle(.plain).padding(.horizontal, 2).padding(.top, 2).background(LWTheme.panel2)
    }
}

/// The reference's compact selection deck cell; callers own its action.
struct LWCompactSelectionKey: View {
    let title: String
    let icon: String
    var disabled = false
    var destructive = false

    var body: some View {
        VStack(spacing: 4) {
            LWIcon(icon, size: 18)
            Text(title).font(LWTheme.text(12)).lwLineBox(12, 16.8)
        }
        .frame(maxWidth: .infinity).frame(minHeight: 62)
        .foregroundStyle(disabled ? LWTheme.tertiary : destructive ? LWTheme.danger : LWTheme.ink)
        .background(LWTheme.key).contentShape(Rectangle())
    }
}

/// Selection header presentation only; membership and selection remain caller-owned.
struct LWCompactSelectionHeader: View {
    let count: Int
    let allSelected: Bool
    let onToggleAll: () -> Void
    let onDone: () -> Void
    let title: String
    var inset = true

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Button(action: onToggleAll) {
                    Text(allSelected ? "Deselect all" : "Select all").lwLineBox(16, 22.4)
                        .padding(.horizontal, 4).frame(minHeight: 44).contentShape(Rectangle())
                }
                Spacer(minLength: 0)
                Text("\(count) selected").font(LWTheme.text(15).weight(.medium)).lwLineBox(15, 21)
                Spacer(minLength: 0)
                Button(action: onDone) {
                    Text("Done").fontWeight(.medium).lwLineBox(16, 22.4)
                        .padding(.horizontal, 4).frame(minHeight: 44).contentShape(Rectangle())
                }
            }.frame(minHeight: 44)
            if !title.isEmpty {
                Text(title).font(LWTheme.text(32).weight(.medium)).tracking(-1.12)
                    .lwLineBox(32, 33.6).frame(minHeight: 44, alignment: .top)
            }
        }
        .font(LWTheme.text(16)).foregroundStyle(LWTheme.ink).buttonStyle(.plain)
        .padding(.horizontal, inset ? 16 : 0).padding(.top, inset ? 10 : 0)
    }
}

#endif
