import SwiftUI
import UIKit

struct LWMenuButton<Label: View>: View {
    let request: () -> LWMenuRequest
    @ViewBuilder let label: () -> Label
    @EnvironmentObject private var presenter: LWSheetPresenter
    @State private var bounds = CGRect.zero
    var body: some View {
        Button {
            var menu = request()
            if menu.anchor == nil { menu.anchor = CGPoint(x: bounds.minX, y: bounds.maxY) }
            presenter.present(menu: menu)
        } label: { label() }
            .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { bounds = $0 }
    }
}

private struct LWContextMenuTrigger: ViewModifier {
    let enabled: Bool
    let request: () -> LWMenuRequest
    @EnvironmentObject private var presenter: LWSheetPresenter
    @Environment(\.lwWideLayout) private var wide
    @Environment(\.lwLibraryKeyboardCovered) private var keyboardCovered
    @State private var bounds = CGRect.zero
    func body(content: Content) -> some View {
        content.background {
            LWMenuTouchRegion(secondaryEnabled: wide && enabled, isAvailable: { enabled && presenter.content == nil && !keyboardCovered }) { point in
                guard enabled, presenter.content == nil, !keyboardCovered else { return }
                var menu = request()
                menu.anchor = point
                presenter.present(menu: menu)
            }.accessibilityHidden(true)
        }
        .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { bounds = $0 }
        .highPriorityGesture(LongPressGesture(minimumDuration: 0.45, maximumDistance: 8).onEnded { _ in
            guard enabled, !wide, presenter.content == nil, !keyboardCovered else { return }
            var menu = request()
            menu.anchor = CGPoint(x: bounds.midX, y: bounds.midY)
            presenter.present(menu: menu)
        }, including: !enabled || wide || keyboardCovered || presenter.content != nil ? .none : .all)
        .accessibilityAction(named: Text("Show actions")) { if enabled { presenter.present(menu: request()) } }
    }
}

extension View {
    func lwContextMenu(enabled: Bool = true, request: @escaping () -> LWMenuRequest) -> some View {
        modifier(LWContextMenuTrigger(enabled: enabled, request: request))
    }
}

private struct LWMenuTouchRegion: UIViewRepresentable {
    let secondaryEnabled: Bool
    let isAvailable: () -> Bool
    let activate: (CGPoint) -> Void
    func makeUIView(context: Context) -> Region { Region() }
    func updateUIView(_ view: Region, context: Context) {
        view.secondaryEnabled = secondaryEnabled
        view.isAvailable = isAvailable
        view.activate = activate
    }
    final class Region: UIView {
        var secondaryEnabled = false
        var isAvailable: () -> Bool = { false }
        var activate: ((CGPoint) -> Void)?
        private weak var registry: LWMenuTouchRegistry?
        override init(frame: CGRect) { super.init(frame: frame); isUserInteractionEnabled = false; backgroundColor = .clear }
        required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
        override func didMoveToWindow() {
            super.didMoveToWindow()
            registry?.remove(self)
            registry = window.map { LWMenuTouchRegistry.registry(for: $0) }
            registry?.add(self)
        }
        func contains(_ point: CGPoint, in window: UIWindow) -> Bool {
            guard self.window === window, !isHidden, alpha > 0, bounds.width > 0, bounds.height > 0,
                  bounds.contains(convert(point, from: window)) else { return false }
            var ancestor = superview
            while let view = ancestor {
                guard !view.isHidden, view.alpha > 0, view.isUserInteractionEnabled else { return false }
                if view.clipsToBounds && !view.bounds.contains(view.convert(point, from: window)) { return false }
                ancestor = view.superview
            }
            return true
        }
    }
}

/// UIKit routes Catalyst secondary clicks through its context interaction path.
/// One interaction serves the window regardless of lazy row count; it never
/// supplies a native menu because the shared presenter owns the rendered menu.
/// The transparent measurement views never intercept row taps, selection or dragging.
@MainActor
private final class LWMenuTouchRegistry: NSObject, UIContextMenuInteractionDelegate {
    private static let registries = NSMapTable<UIWindow, LWMenuTouchRegistry>.weakToStrongObjects()
    private weak var window: UIWindow?
    private var regions = NSHashTable<LWMenuTouchRegion.Region>.weakObjects()
    static func registry(for window: UIWindow) -> LWMenuTouchRegistry {
        if let existing = registries.object(forKey: window) { return existing }
        let registry = LWMenuTouchRegistry(window: window)
        registries.setObject(registry, forKey: window)
        return registry
    }
    private init(window: UIWindow) {
        self.window = window
        super.init()
        window.addInteraction(UIContextMenuInteraction(delegate: self))
    }
    func add(_ region: LWMenuTouchRegion.Region) { regions.add(region) }
    func remove(_ region: LWMenuTouchRegion.Region) { regions.remove(region) }
    private func region(at point: CGPoint) -> LWMenuTouchRegion.Region? {
        guard let window else { return nil }
        return regions.allObjects.filter { $0.secondaryEnabled && $0.isAvailable() && $0.contains(point, in: window) }
            .min { $0.bounds.width * $0.bounds.height < $1.bounds.width * $1.bounds.height }
    }
    func contextMenuInteraction(_ interaction: UIContextMenuInteraction, configurationForMenuAtLocation point: CGPoint) -> UIContextMenuConfiguration? {
        let candidate = region(at: point)
        candidate?.activate?(point)
        return nil
    }
}

