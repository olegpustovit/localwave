import SwiftUI
import UIKit
import XCTest
@testable import localwave

@MainActor
final class CanvasHostTests: XCTestCase {
    func testParentAndEnvironmentUpdatesRetainOneStatefulNativeCanvas() async throws {
        let source = LWCanvasSnapshotSource()
        let model = CanvasProbeModel()
        let recorder = CanvasProbeRecorder()
        let host = UIHostingController(rootView: CanvasProbeParent(source: source, model: model, recorder: recorder))
        host.safeAreaRegions = []
        let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 390, height: 778))
        window.rootViewController = host
        window.isHidden = false
        defer { window.isHidden = true; window.rootViewController = nil }
        host.view.layoutIfNeeded()
        try await waitUntil { recorder.revision == 0 && recorder.field != nil && source.view != nil }
        let originalField = try XCTUnwrap(recorder.field)
        let originalSource = try XCTUnwrap(source.view)
        let originalState = try XCTUnwrap(recorder.stateID)
        XCTAssertEqual(recorder.mounts, 1)

        // A real editing event changes child @State. Replacing a hosting root or
        // mounting a second canvas must not reset this native field or its text.
        originalField.text = "Edited in the live canvas"
        originalField.sendActions(for: .editingChanged)
        try await waitUntil { recorder.text == "Edited in the live canvas" }
        for revision in 1...3 {
            model.revision = revision
            model.marker = "environment-\(revision)"
            host.view.layoutIfNeeded()
        }
        model.dynamicType = .accessibility3
        model.reduceMotion = true
        model.inherited = CanvasInheritedObject("replacement inherited object")
        host.view.layoutIfNeeded()
        try await waitUntil {
            recorder.revision == 3 && recorder.marker == "environment-3" &&
            recorder.dynamicType == .accessibility3 && recorder.reduceMotion &&
            recorder.inherited === model.inherited
        }
        XCTAssertTrue(recorder.field === originalField)
        XCTAssertTrue(source.view === originalSource)
        XCTAssertEqual(recorder.stateID, originalState)
        XCTAssertEqual(recorder.mounts, 1)
        XCTAssertEqual(originalField.text, "Edited in the live canvas")
    }

    func testUnmountClearsSourceAndPendingContentCannotResurrectCanvas() async throws {
        let source = LWCanvasSnapshotSource()
        let model = CanvasProbeModel()
        let recorder = CanvasProbeRecorder()
        let host = UIHostingController(rootView: CanvasProbeParent(source: source, model: model, recorder: recorder))
        host.safeAreaRegions = []
        let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 390, height: 778))
        window.rootViewController = host
        window.isHidden = false
        defer { window.isHidden = true; window.rootViewController = nil }
        host.view.layoutIfNeeded()
        try await waitUntil { source.view != nil && recorder.mounts == 1 }
        // Queue a content update and remove its owner before deferred publication.
        model.revision = 99
        host.view.layoutIfNeeded()
        model.showCanvas = false
        host.view.layoutIfNeeded()
        try await waitUntil { source.view == nil }
        let mountsAtRemoval = recorder.mounts
        model.revision = 100
        model.marker = "after removal"
        for _ in 0..<3 {
            await nextMainQueueTurn()
            host.view.layoutIfNeeded()
        }
        XCTAssertNil(source.view)
        XCTAssertEqual(recorder.mounts, mountsAtRemoval)
    }

    func testConcurrentScopedHostsMountExactlyOneDialogInputAndOneSheetContent() async throws {
        for wide in [false, true] {
            let presenter = LWSheetPresenter()
            let source = LWCanvasSnapshotSource()
            let mounts = CanvasSheetMountRecorder()
            let host = UIHostingController(rootView: ZStack {
                // Both observers stay mounted, including the deferred canvas
                // observer that previously also created the external dialog.
                LWCanvasHost(source: source) { LWRootSheetHost(presentsSheets: true) }
                LWRootSheetHost(presentsSheets: false)
            }.frame(width: 390, height: 778).environmentObject(presenter).environment(\.lwWideLayout, wide).environment(\.lwReduceMotion, true))
            host.safeAreaRegions = []
            let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
            let previousKeyWindow = scene.windows.first(where: \.isKeyWindow)
            let window = UIWindow(windowScene: scene)
            window.frame = CGRect(x: 0, y: 0, width: 390, height: 778)
            window.rootViewController = host
            window.makeKeyAndVisible()
            defer {
                presenter.dismissImmediately()
                window.isHidden = true
                window.rootViewController = nil
                previousKeyWindow?.makeKeyAndVisible()
            }
            host.view.layoutIfNeeded()
            try await waitUntil { source.view != nil }
            presenter.present(dialog: LWDialogRequest(title: "New List", input: LWDialogInput(placeholder: "List name"), primaryTitle: "Create", onSubmit: { _ in }))
            host.view.layoutIfNeeded()
            try await waitUntil { !self.textFields(in: host.view).isEmpty }
            // Drain both graphs' queued updates before counting actual UIKit
            // fields. Checking presenter.kind alone would miss double mounting.
            for _ in 0..<3 { await nextMainQueueTurn(); host.view.layoutIfNeeded() }
            let fields = textFields(in: host.view)
            XCTAssertEqual(fields.count, 1, "Only the external host may mount the input dialog (wide=\(wide))")
            XCTAssertEqual(fields.first?.placeholder, "List name")
            XCTAssertLessThanOrEqual(fields.filter(\.isFirstResponder).count, 1)

            presenter.present(content: AnyView(CanvasSheetMountProbe(recorder: mounts).frame(height: 44))) {}
            host.view.layoutIfNeeded()
            try await waitUntil { mounts.count > 0 && self.textFields(in: host.view).isEmpty }
            for _ in 0..<3 { await nextMainQueueTurn(); host.view.layoutIfNeeded() }
            XCTAssertEqual(mounts.count, 1, "Only the canvas host may mount an ordinary sheet (wide=\(wide))")
            presenter.dismissImmediately()
            host.view.layoutIfNeeded()
            try await waitUntil { self.textFields(in: host.view).isEmpty }
        }
    }

    private func textFields(in view: UIView) -> [UITextField] {
        (view as? UITextField).map { [$0] } ?? view.subviews.flatMap { textFields(in: $0) }
    }

    private func waitUntil(_ condition: @escaping @MainActor () -> Bool, file: StaticString = #filePath, line: UInt = #line) async throws {
        let deadline = ContinuousClock.now + .seconds(3)
        while !condition(), ContinuousClock.now < deadline { try await Task.sleep(for: .milliseconds(10)) }
        XCTAssertTrue(condition(), "Canvas did not reach its expected committed state", file: file, line: line)
    }

    private func nextMainQueueTurn() async {
        await withCheckedContinuation { continuation in DispatchQueue.main.async { continuation.resume() } }
    }
}

private struct CanvasProbeMarkerKey: EnvironmentKey { static let defaultValue = "missing environment" }
private extension EnvironmentValues {
    var canvasProbeMarker: String {
        get { self[CanvasProbeMarkerKey.self] }
        set { self[CanvasProbeMarkerKey.self] = newValue }
    }
}

@MainActor private final class CanvasInheritedObject: ObservableObject {
    let value: String
    init(_ value: String) { self.value = value }
}
@MainActor private final class CanvasProbeModel: ObservableObject {
    @Published var revision = 0
    @Published var marker = "initial environment"
    @Published var dynamicType = DynamicTypeSize.large
    @Published var reduceMotion = false
    @Published var showCanvas = true
    @Published var inherited = CanvasInheritedObject("initial inherited object")
}
@MainActor private final class CanvasProbeRecorder {
    weak var field: UITextField?
    weak var inherited: CanvasInheritedObject?
    var mounts = 0
    var revision: Int?
    var stateID: UUID?
    var text = ""
    var marker = ""
    var dynamicType = DynamicTypeSize.large
    var reduceMotion = false
}

private struct CanvasProbeParent: View {
    let source: LWCanvasSnapshotSource
    @ObservedObject var model: CanvasProbeModel
    let recorder: CanvasProbeRecorder
    var body: some View {
        Group {
            if model.showCanvas {
                LWCanvasHost(source: source) { CanvasProbeContent(revision: model.revision, recorder: recorder) }
                    .environment(\.canvasProbeMarker, model.marker)
                    .environment(\.dynamicTypeSize, model.dynamicType)
                    .environment(\.lwReduceMotion, model.reduceMotion)
                    .environmentObject(model.inherited)
            } else { Color.clear }
        }.frame(width: 390, height: 778)
    }
}

private struct CanvasProbeContent: View {
    let revision: Int
    let recorder: CanvasProbeRecorder
    @State private var stateID = UUID()
    @State private var text = "Original live text"
    var body: some View {
        NavigationStack {
            ScrollView { CanvasProbeField(revision: revision, stateID: stateID, text: $text, recorder: recorder).frame(height: 44) }
        }
    }
}

private struct CanvasProbeField: UIViewRepresentable {
    let revision: Int
    let stateID: UUID
    @Binding var text: String
    let recorder: CanvasProbeRecorder
    @Environment(\.canvasProbeMarker) private var marker
    @Environment(\.dynamicTypeSize) private var dynamicType
    @Environment(\.lwReduceMotion) private var reduceMotion
    @EnvironmentObject private var inherited: CanvasInheritedObject
    func makeCoordinator() -> Coordinator { Coordinator(text: $text) }
    func makeUIView(context: Context) -> UITextField {
        let field = UITextField()
        field.addTarget(context.coordinator, action: #selector(Coordinator.edited(_:)), for: .editingChanged)
        recorder.mounts += 1
        recorder.field = field
        return field
    }
    func updateUIView(_ field: UITextField, context: Context) {
        context.coordinator.text = $text
        if field.text != text { field.text = text }
        recorder.revision = revision
        recorder.stateID = stateID
        recorder.text = text
        recorder.marker = marker
        recorder.dynamicType = dynamicType
        recorder.reduceMotion = reduceMotion
        recorder.inherited = inherited
    }
    final class Coordinator: NSObject {
        var text: Binding<String>
        init(text: Binding<String>) { self.text = text }
        @objc func edited(_ field: UITextField) { text.wrappedValue = field.text ?? "" }
    }
}

@MainActor private final class CanvasSheetMountRecorder { var count = 0 }
private struct CanvasSheetMountProbe: UIViewRepresentable {
    let recorder: CanvasSheetMountRecorder
    func makeUIView(context: Context) -> UILabel {
        recorder.count += 1
        let label = UILabel()
        label.text = "Ordinary sheet content"
        return label
    }
    func updateUIView(_ view: UILabel, context: Context) {}
}
