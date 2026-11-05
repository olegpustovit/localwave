#if os(iOS)
import SwiftUI
import UIKit

private struct LWCanvasSnapshotSourceKey: EnvironmentKey {
    static let defaultValue: LWCanvasSnapshotSource? = nil
}

extension EnvironmentValues {
    var lwCanvasSnapshotSource: LWCanvasSnapshotSource? {
        get { self[LWCanvasSnapshotSourceKey.self] }
        set { self[LWCanvasSnapshotSourceKey.self] = newValue }
    }
}

/// The canvas has one live hosting tree. The menu reads a noninteractive visual
/// proxy of that tree; it never mounts a second copy of the canvas content.
@MainActor
final class LWCanvasSnapshotSource: ObservableObject {
    private(set) weak var view: UIView?

    fileprivate func attach(_ view: UIView) { self.view = view }
    fileprivate func detach(_ view: UIView) {
        if self.view === view { self.view = nil }
    }
}

struct LWCanvasHost<Content: View>: UIViewControllerRepresentable {
    let source: LWCanvasSnapshotSource
    private let content: Content

    init(source: LWCanvasSnapshotSource, @ViewBuilder content: () -> Content) {
        self.source = source
        self.content = content()
    }

    func makeCoordinator() -> Coordinator { Coordinator(source: source) }

    func makeUIViewController(context: Context) -> UIHostingController<AnyView> {
        let store = CanvasContent(AnyView(content.environment(\.self, context.environment)))
        context.coordinator.content = store
        let host = UIHostingController(rootView: AnyView(CanvasRoot(content: store)))
        host.safeAreaRegions = []
        host.view.backgroundColor = .clear
        context.coordinator.attach(source: source, view: host.view)
        return host
    }

    func updateUIViewController(_ host: UIHostingController<AnyView>, context: Context) {
        // rootView assignment drops the caller's animation transaction. Publish
        // into the retained SwiftUI graph so matched geometry keeps that timing.
        context.coordinator.update(AnyView(content.environment(\.self, context.environment)), transaction: context.transaction)
        context.coordinator.attach(source: source, view: host.view)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiViewController: UIHostingController<AnyView>, context: Context) -> CGSize? {
        guard let width = proposal.width, let height = proposal.height, width.isFinite, height.isFinite else { return nil }
        return CGSize(width: width, height: height)
    }

    static func dismantleUIViewController(_ host: UIHostingController<AnyView>, coordinator: Coordinator) {
        coordinator.dismantle()
    }

    @MainActor final class Coordinator {
        fileprivate var source: LWCanvasSnapshotSource
        fileprivate var content: CanvasContent?
        private weak var view: UIView?
        private var revision = 0
        init(source: LWCanvasSnapshotSource) { self.source = source }
        fileprivate func attach(source: LWCanvasSnapshotSource, view: UIView) {
            if self.source !== source, let previous = self.view { self.source.detach(previous) }
            self.source = source
            self.view = view
            source.attach(view)
        }

        fileprivate func update(_ view: AnyView, transaction: Transaction) {
            revision += 1
            let current = revision
            // SwiftUI forbids publishing during the representable update pass.
            DispatchQueue.main.async { [weak self] in
                guard let self, revision == current else { return }
                withTransaction(transaction) { self.content?.view = view }
            }
        }

        fileprivate func dismantle() {
            revision += 1
            if let view { source.detach(view) }
            view = nil
            content = nil
        }
    }

    @MainActor fileprivate final class CanvasContent: ObservableObject {
        @Published var view: AnyView
        init(_ view: AnyView) { self.view = view }
    }

    private struct CanvasRoot: View {
        @ObservedObject var content: CanvasContent
        var body: some View { content.view }
    }
}

/// Mount only while the menu backdrop is needed. Blur, saturation, paper tint
/// and the single outer opacity/compositing group belong to the menu surface.
struct LWMenuBackdrop: UIViewRepresentable {
    let source: LWCanvasSnapshotSource
    var onFirstFrame: () -> Void = {}

    func makeCoordinator() -> Coordinator { Coordinator(source: source, onFirstFrame: onFirstFrame) }

    func makeUIView(context: Context) -> BackdropView {
        let view = BackdropView()
        context.coordinator.container = view
        view.windowChanged = { [weak coordinator = context.coordinator] in coordinator?.windowChanged() }
        return view
    }

    func updateUIView(_ view: BackdropView, context: Context) {
        context.coordinator.update(source: source, onFirstFrame: onFirstFrame)
    }

    static func dismantleUIView(_ view: BackdropView, coordinator: Coordinator) {
        view.windowChanged = nil
        coordinator.stop()
    }

    @MainActor final class BackdropView: UIView {
        fileprivate var windowChanged: (() -> Void)?
        override init(frame: CGRect) {
            super.init(frame: frame)
            backgroundColor = .clear
            isUserInteractionEnabled = false
            isAccessibilityElement = false
            accessibilityElementsHidden = true
            clipsToBounds = true
        }
        required init?(coder: NSCoder) { fatalError("init(coder:) is unavailable") }
        override func didMoveToWindow() {
            super.didMoveToWindow()
            windowChanged?()
        }
    }

    @MainActor final class Coordinator: NSObject {
        fileprivate weak var container: BackdropView?
        private var source: LWCanvasSnapshotSource
        private var onFirstFrame: () -> Void
        private var displayLink: CADisplayLink?
        private var generation = UUID()
        private var deliveredFirstFrame = false
        private var active = false
        private var frame: UIView?

        init(source: LWCanvasSnapshotSource, onFirstFrame: @escaping () -> Void) {
            self.source = source
            self.onFirstFrame = onFirstFrame
        }

        fileprivate func update(source: LWCanvasSnapshotSource, onFirstFrame: @escaping () -> Void) {
            self.onFirstFrame = onFirstFrame
            if self.source !== source {
                self.source = source
                generation = UUID()
                deliveredFirstFrame = false
                frame?.removeFromSuperview()
                frame = nil
            }
        }

        fileprivate func windowChanged() {
            guard container?.window != nil else { stop(); return }
            guard !active else { return }
            active = true
            generation = UUID()
            deliveredFirstFrame = false
            let link = CADisplayLink(target: DisplayLinkTarget(self), selector: #selector(DisplayLinkTarget.tick))
            link.preferredFramesPerSecond = 60
            displayLink = link
            link.add(to: .main, forMode: .common)
        }

        fileprivate func stop() {
            active = false
            generation = UUID()
            displayLink?.invalidate()
            displayLink = nil
            frame?.removeFromSuperview()
            frame = nil
        }

        fileprivate func capture() {
            guard active, let container, let window = container.window,
                  let source = source.view, source.window === window,
                  !source.bounds.isEmpty, !container.bounds.isEmpty else {
                frame?.removeFromSuperview()
                frame = nil
                return
            }
            // Snapshot only the owned canvas, never the window or menu overlay.
            // A missing frame is retried on the next tick without blocking reveal.
            guard let copy = source.snapshotView(afterScreenUpdates: false) else { return }
            copy.isUserInteractionEnabled = false
            copy.isAccessibilityElement = false
            copy.accessibilityElementsHidden = true
            copy.frame = source.convert(source.bounds, to: container)
            container.addSubview(copy)
            frame?.removeFromSuperview()
            frame = copy
            guard !deliveredFirstFrame else { return }
            deliveredFirstFrame = true
            let current = generation
            DispatchQueue.main.async { [weak self] in
                guard let self, active, generation == current, container.window != nil else { return }
                onFirstFrame()
            }
        }

        deinit { displayLink?.invalidate() }
    }

    /// CADisplayLink retains its target. Keep the coordinator weak so removal
    /// does not leave a run-loop-owned snapshot driver alive.
    @MainActor private final class DisplayLinkTarget: NSObject {
        weak var coordinator: Coordinator?
        init(_ coordinator: Coordinator) { self.coordinator = coordinator }
        @objc func tick() { coordinator?.capture() }
    }
}

#endif
