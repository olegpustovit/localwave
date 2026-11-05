#if os(iOS)
import SwiftUI
import UIKit
import LocalWaveDomain

struct LWMenuTrackPreview: View {
    let song: Song
    let subtitle: String
    var body: some View {
        HStack(spacing: 12) {
            LWArtwork(song: song, size: 56)
            VStack(alignment: .leading, spacing: 0) {
                Text(song.title).font(LWTheme.text(15)).lwLineBox(15, 21).foregroundStyle(LWTheme.ink)
                subtitleText.lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
            }.lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
        }
    }
    private var subtitleText: Text {
        guard let split = subtitle.range(of: " · ", options: .backwards) else { return Text(subtitle).font(LWTheme.text(13)) }
        return Text(String(subtitle[..<split.upperBound])).font(LWTheme.text(13))
            + Text(String(subtitle[split.upperBound...])).font(LWTheme.data(12))
    }
}

struct LWContextMenuSurface: View {
    let request: LWMenuRequest
    let presentationID: UUID
    @EnvironmentObject private var presenter: LWSheetPresenter
    @Environment(\.lwWideLayout) private var wide
    @Environment(\.lwReduceMotion) private var reduceMotion
    @Environment(\.lwCanvasSnapshotSource) private var snapshotSource
    @State private var submenu: LWMenuAction?
    @State private var submenuGroups: [LWMenuGroup] = []
    @State private var menuTrail: [(action: LWMenuAction, groups: [LWMenuGroup])] = []
    @State private var renderGeneration = 0
    @State private var settledGeneration: Int?
    @State private var backdropReady = false
    @State private var backdropTimedOut = false
    @State private var mobileMenuBounds = CGRect.zero
    @FocusState private var keyboardFocused: Bool

    private var ready: Bool { wide || snapshotSource == nil || backdropReady || backdropTimedOut }
    private var waitingForBackdrop: Bool { !wide && snapshotSource != nil && !backdropReady }
    private var entered: Bool { ready && settledGeneration == renderGeneration }
    private struct RevealEpoch: Hashable { let generation: Int; let ready: Bool }

    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: wide ? .topLeading : .center) {
                if !wide {
                    Group {
                        if let snapshotSource {
                            // Transparent blur edges would leak the sharp canvas below.
                            LWMenuBackdrop(source: snapshotSource) { backdropReady = true }
                                .blur(radius: 14, opaque: true).saturation(0.9)
                                .overlay { LWTheme.paper.opacity(0.55) }
                                .compositingGroup()
                        } else { LWTheme.paper.opacity(0.55) }
                    }.allowsHitTesting(false).accessibilityHidden(true)
                }
                Color.clear
                    .contentShape(Rectangle())
                    .onTapGesture(coordinateSpace: .global) { point in
                        guard presenter.identity == presentationID else { return }
                        // Transparent group gaps still belong to the menu.
                        guard wide || !mobileMenuBounds.contains(point) else { return }
                        presenter.dismissMenuFromBackdrop(wide: wide)
                    }
                if wide {
                    let origin = desktopOrigin(in: geometry)
                    desktopPanel(request.groups, open: submenu?.id)
                        .modifier(LWMenuDrop(shown: entered))
                        .id("parent-\(renderGeneration)")
                        .offset(x: origin.x, y: origin.y)
                    if submenu != nil {
                        desktopPanel(submenuGroups, open: nil)
                            .modifier(LWMenuDrop(shown: entered)).id("child-\(renderGeneration)")
                            .offset(x: origin.x + 232, y: origin.y + 4)
                    }
                } else {
                    ViewThatFits(in: .vertical) {
                        mobileMenu
                        ScrollView { mobileMenu.padding(.vertical, 1) }.scrollIndicators(.hidden)
                    }.frame(maxWidth: 300)
                        .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { mobileMenuBounds = $0 }
                        .padding(16)
                }
            }
            .compositingGroup()
            // The reference's .anim selector wins over .out; hold until removal.
            .opacity(wide || (ready && (reduceMotion ? presenter.visible : entered)) ? 1 : 0)
            .animation(wide || reduceMotion ? nil : LWMotion.out(LWMotion.base), value: settledGeneration)
            .focusable()
            .focusEffectDisabled()
            .focused($keyboardFocused)
            .onKeyPress(.escape) {
                guard presenter.kind == .menu, presenter.identity == presentationID else { return .ignored }
                presenter.dismiss()
                return .handled
            }
            .task(id: RevealEpoch(generation: renderGeneration, ready: ready)) {
                let generation = renderGeneration
                guard ready else { return }
                await Task.yield()
                guard !Task.isCancelled, ready, generation == renderGeneration,
                      presenter.revealMenu(requestID: request.id, presentationID: presentationID) else { return }
                if !wide && generation == 0 {
                    _ = UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
                }
                settledGeneration = generation
                if wide { keyboardFocused = true }
            }
            .task(id: waitingForBackdrop) {
                guard waitingForBackdrop else { return }
                // Keep actions usable if the platform cannot snapshot the canvas.
                let revealed = await presenter.revealMenuWithoutBackdrop(requestID: request.id, presentationID: presentationID)
                guard revealed, !Task.isCancelled, presenter.identity == presentationID, presenter.kind == .menu else { return }
                backdropTimedOut = true
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Context menu")
            .onDisappear { keyboardFocused = false }
            .background { Button("Close menu") { presenter.dismiss() }.keyboardShortcut(.cancelAction).hidden() }
        }
    }

    private var mobileMenu: some View {
        VStack(spacing: 10) {
            if let preview = request.preview {
                preview.padding(12).background(LWTheme.paper)
                    .background { Rectangle().fill(LWTheme.line).padding(-1) }
                    .background { Rectangle().fill(.black.opacity(0.35)).padding(20).blur(radius: 20).offset(y: 20) }
                    .modifier(LWMenuElementEntrance(lift: true, shown: entered)).id("preview-\(renderGeneration)")
            }
            VStack(spacing: 6) {
                if let level = menuTrail.last {
                    mobileGroup([LWMenuAction(title: "‹ " + level.action.title, action: goBack)], back: true)
                }
                ForEach(menuTrail.last?.groups ?? request.groups) { group in mobileGroup(group.actions) }
            }.modifier(LWMenuElementEntrance(lift: false, shown: entered)).id("groups-\(renderGeneration)")
        }
    }

    private func mobileGroup(_ actions: [LWMenuAction], back: Bool = false) -> some View {
        VStack(spacing: 1) {
            ForEach(actions) { item in
                Button {
                    if back { item.action() }
                    else if item.submenu != nil { openSubmenu(item) }
                    else { presenter.perform(menuAction: item, requestID: request.id) }
                } label: {
                    HStack(spacing: 12) {
                        Text(item.title).font(LWTheme.text(16)).lwLineBox(16, 22.4)
                        Spacer(minLength: 0)
                        if item.submenu != nil { Text("›").font(LWTheme.text(18)).accessibilityHidden(true) }
                        else if let icon = item.icon { LWIcon(icon, size: 18).accessibilityHidden(true) }
                    }
                    .padding(.horizontal, 14).frame(minHeight: 46)
                    .foregroundStyle(item.disabled ? LWTheme.tertiary : item.destructive ? LWTheme.danger : back ? LWTheme.secondary : LWTheme.ink)
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .buttonStyle(LWMenuRowStyle(background: LWTheme.paper, pressed: LWTheme.panel))
                .accessibilityLabel(item.title)
                .accessibilityHint(item.submenu == nil ? "" : "Opens submenu")
                .disabled(item.disabled)
            }
        }.background(LWTheme.line)
            .background { Rectangle().fill(LWTheme.line).padding(-1) }
    }

    private func desktopPanel(_ groups: [LWMenuGroup], open: UUID?) -> some View {
        VStack(spacing: 0) {
            ForEach(Array(groups.enumerated()), id: \.element.id) { index, group in
                if index > 0 { Rectangle().fill(LWTheme.line).frame(height: 1).padding(.horizontal, 6).padding(.vertical, 4) }
                ForEach(group.actions) { item in
                    LWDesktopMenuRow(item: item, open: open == item.id) {
                        if item.submenu != nil { openSubmenu(item) }
                        else { presenter.perform(menuAction: item, requestID: request.id) }
                    }
                }
            }
        }.padding(5).frame(width: 228).background(LWTheme.paper)
            .background { Rectangle().fill(LWTheme.line).padding(-1) }
            .background { Rectangle().fill(.black.opacity(0.35)).padding(12).blur(radius: 20).offset(y: 16) }
    }

    private func desktopOrigin(in geometry: GeometryProxy) -> CGPoint {
        let global = geometry.frame(in: .global).origin
        let anchor = request.anchor.map { CGPoint(x: $0.x - global.x, y: $0.y - global.y) }
            ?? CGPoint(x: geometry.size.width / 2, y: geometry.size.height / 2)
        // The reference reserves an adjacent panel and estimates30px rows at the edge.
        let actionCount: Int = request.groups.reduce(0) { $0 + $1.actions.count }
        let estimatedHeight = CGFloat(actionCount * 30 + request.groups.count * 9 + 10)
        return CGPoint(x: max(0, min(anchor.x, geometry.size.width - 470)), y: max(0, min(anchor.y, geometry.size.height - estimatedHeight - 8)))
    }

    private func openSubmenu(_ item: LWMenuAction) {
        guard let children = item.submenu else { return }
        let groups = children()
        var transaction = Transaction(); transaction.disablesAnimations = true
        withTransaction(transaction) {
            if wide { submenuGroups = groups; submenu = item }
            else { menuTrail.append((item, groups)) }
            // A different generation makes opacity zero in this same render;
            // the guarded task starts its fade in a separate transaction.
            renderGeneration += 1
        }
    }

    private func goBack() {
        guard !menuTrail.isEmpty else { return }
        var transaction = Transaction(); transaction.disablesAnimations = true
        withTransaction(transaction) {
            _ = menuTrail.popLast()
            renderGeneration += 1
        }
    }
}

private struct LWDesktopMenuRow: View {
    let item: LWMenuAction
    let open: Bool
    let action: () -> Void
    @State private var hovering = false
    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Text(item.title).font(LWTheme.text(13)).lwLineBox(13, 18.2)
                Spacer(minLength: 0)
                if item.submenu != nil { Text("▸").font(LWTheme.data(11)).foregroundStyle(LWTheme.secondary).accessibilityHidden(true) }
                else if let shortcut = item.shortcut { Text(shortcut).font(LWTheme.data(11)).foregroundStyle(LWTheme.secondary).accessibilityHidden(true) }
            }.padding(.horizontal, 10).frame(minHeight: 32)
                .foregroundStyle(item.disabled ? LWTheme.tertiary : item.destructive ? LWTheme.danger : LWTheme.ink)
        }.buttonStyle(LWMenuRowStyle(background: open || hovering ? LWTheme.panel2 : LWTheme.paper, pressed: LWTheme.panel2))
            .accessibilityLabel(item.title)
            .accessibilityHint(item.submenu == nil ? "" : "Opens submenu")
            .disabled(item.disabled).onHover { hovering = $0 }
    }
}

private struct LWMenuRowStyle: ButtonStyle {
    let background: Color
    let pressed: Color
    @Environment(\.lwReduceMotion) private var reduceMotion
    func makeBody(configuration: Configuration) -> some View {
        configuration.label.background(configuration.isPressed ? pressed : background)
            .contentShape(Rectangle())
            .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: configuration.isPressed)
    }
}

private struct LWMenuElementEntrance: ViewModifier {
    let lift: Bool
    let shown: Bool
    @Environment(\.lwReduceMotion) private var reduceMotion
    func body(content: Content) -> some View {
        content.compositingGroup()
            .opacity(shown || reduceMotion ? 1 : lift ? 0.4 : 0)
            .scaleEffect(shown || reduceMotion || !lift ? 1 : 0.94)
            .offset(y: shown || reduceMotion || lift ? 0 : 10)
            .animation(reduceMotion ? nil : LWMotion.springAnimation(LWMotion.slow).delay(lift ? 0 : 0.06), value: shown)
    }
}

private struct LWMenuDrop: ViewModifier {
    let shown: Bool
    @Environment(\.lwReduceMotion) private var reduceMotion
    func body(content: Content) -> some View {
        content.compositingGroup()
            .opacity(shown || reduceMotion ? 1 : 0)
            .scaleEffect(x: 1, y: shown || reduceMotion ? 1 : 0.96, anchor: .topLeading)
            .offset(y: shown || reduceMotion ? 0 : -6)
            .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: shown)
    }
}

#endif
