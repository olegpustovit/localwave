#if os(iOS)
import SwiftUI
import UIKit

struct LWPlaybackNotice: View {
    let message: String
    var actionTitle: String? = nil
    var action: (() -> Void)? = nil
    let onExpiry: () -> Void
    @Environment(\.lwReduceMotion) private var reduceMotion
    @Environment(\.lwNoticeLifetimeScale) private var lifetimeScale
    @State private var visible = false
    @State private var entered = false
    @State private var completed = false

    private var actionable: Bool { actionTitle != nil && action != nil }

    var body: some View {
        HStack(spacing: 12) {
            Text(message).lwLineBox(14, 19.6).fixedSize(horizontal: false, vertical: true)
            if let actionTitle, let action {
                Button {
                    guard !completed else { return }
                    completed = true
                    action()
                } label: {
                    Text(actionTitle).underline().lwLineBox(14, 19.6)
                }.buttonStyle(.plain)
            }
        }
            .font(LWTheme.text(14))
            .padding(.vertical, 10).padding(.horizontal, 16)
            .foregroundStyle(LWTheme.onInk).background(LWTheme.ink)
            .opacity(visible || reduceMotion ? 1 : 0)
            // Entry rises10pt. Exit changes opacity only and keeps its settled Y.
            .offset(y: entered || reduceMotion ? 0 : 10)
            .allowsHitTesting(actionable && !completed)
            .accessibilityElement(children: actionable ? .contain : .combine)
            .task {
                await Task.yield()
                guard !Task.isCancelled, !completed else { return }
                withAnimation(reduceMotion ? nil : LWMotion.springAnimation(LWMotion.base)) {
                    entered = true
                    visible = true
                }
                do {
                    // Coalesce rapid replacements before queuing a VoiceOver announcement.
                    try await Task.sleep(for: .seconds(LWMotion.quick))
                    guard !completed else { return }
                    let announcement = NSAttributedString(string: message, attributes: [
                        .accessibilitySpeechAnnouncementPriority: UIAccessibilityPriority.low.rawValue
                    ])
                    UIAccessibility.post(notification: .announcement, argument: announcement)
                    try await Task.sleep(for: .seconds((actionable ? 4 : 2) * lifetimeScale - LWMotion.quick))
                    guard !Task.isCancelled, !completed else { return }
                    withAnimation(reduceMotion ? nil : LWMotion.in(LWMotion.quick)) { visible = false }
                    try await Task.sleep(for: .seconds(reduceMotion ? 0 : LWMotion.quick))
                    guard !Task.isCancelled, !completed else { return }
                    completed = true
                    onExpiry()
                } catch is CancellationError { }
                catch { }
            }
    }
}

/// Tests shrink the reference 2 s / 4 s lifetimes; production always uses 1.
private struct LWNoticeLifetimeScaleKey: EnvironmentKey { static let defaultValue = 1.0 }
extension EnvironmentValues {
    var lwNoticeLifetimeScale: Double {
        get { self[LWNoticeLifetimeScaleKey.self] }
        set { self[LWNoticeLifetimeScaleKey.self] = newValue }
    }
}

#endif
