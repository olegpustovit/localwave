#if os(iOS) || os(macOS)
import SwiftUI

@MainActor
final class LWSheetPresenter: ObservableObject {
    enum Kind { case sheet, dialog, confirmation, menu }
    @Published private(set) var content: AnyView?
    @Published private(set) var visible = false
    @Published private(set) var identity = UUID()
    @Published private(set) var kind = Kind.sheet
    var reduceMotion = false
    private var onDismiss: (() -> Void)?
    private var removal: Task<Void, Never>?
    private var requestID: UUID?

    @discardableResult
    func present(content: AnyView, onDismiss: @escaping () -> Void) -> UUID {
        install(content: content, kind: .sheet, onDismiss: onDismiss)
    }

    @discardableResult
    func present(dialog: LWDialogRequest) -> UUID {
        let identity = install(content: AnyView(LWDialogSurface(request: dialog)), kind: dialog.mobileConfirmation ? .confirmation : .dialog, onDismiss: dialog.onCancel)
        requestID = dialog.id
        return identity
    }

    @discardableResult
    func present(menu: LWMenuRequest) -> UUID {
        let presentationID = UUID()
        let identity = install(content: AnyView(LWContextMenuSurface(request: menu, presentationID: presentationID)), kind: .menu, presentationID: presentationID, onDismiss: {})
        requestID = menu.id
        return identity
    }

    private func install(content: AnyView, kind: Kind, presentationID: UUID = UUID(), onDismiss: @escaping () -> Void) -> UUID {
        removal?.cancel()
        removal = nil
        let previous = self.onDismiss
        self.onDismiss = nil
        previous?()
        identity = presentationID
        visible = false
        self.kind = kind
        requestID = nil
        self.content = content
        self.onDismiss = onDismiss
        return identity
    }

    func reveal() {
        guard content != nil, removal == nil else { return }
        visible = true
    }

    @discardableResult
    func revealMenu(requestID: UUID, presentationID: UUID) -> Bool {
        guard !Task.isCancelled, kind == .menu, self.requestID == requestID,
              identity == presentationID, content != nil, removal == nil else { return false }
        if !visible { visible = true }
        return true
    }

    func revealMenuWithoutBackdrop(requestID: UUID, presentationID: UUID) async -> Bool {
        do { try await Task.sleep(for: .seconds(LWMotion.slow)) }
        catch { return false }
        return revealMenu(requestID: requestID, presentationID: presentationID)
    }

    func dismiss() {
        if kind != .sheet { dismissImmediately(); return }
        dismiss(after: LWMotion.base)
    }

    func dismissMenuFromBackdrop(wide: Bool) {
        guard kind == .menu else { return }
        if wide { dismissImmediately() }
        else { dismiss(after: LWMotion.quick) }
    }

    func dismissImmediately() {
        removal?.cancel()
        removal = nil
        let completed = onDismiss
        onDismiss = nil
        content = nil
        requestID = nil
        kind = .sheet
        visible = false
        completed?()
    }

    func submit(dialog: LWDialogRequest, value: String) {
        guard kind == .dialog || kind == .confirmation, requestID == dialog.id, content != nil, removal == nil else { return }
        onDismiss = nil
        dismissImmediately()
        dialog.onSubmit(value)
    }

    func perform(menuAction: LWMenuAction, requestID: UUID) {
        guard kind == .menu, self.requestID == requestID, !menuAction.disabled, menuAction.submenu == nil, content != nil, removal == nil else { return }
        dismissImmediately()
        menuAction.action()
    }

    private func dismiss(after duration: Double) {
        guard content != nil, removal == nil else { return }
        visible = false
        let completed = onDismiss
        onDismiss = nil
        completed?()
        let current = identity
        removal = Task { [weak self] in
            if self?.reduceMotion != true { try? await Task.sleep(for: .seconds(duration)) }
            guard !Task.isCancelled, let self, identity == current else { return }
            content = nil
            requestID = nil
            kind = .sheet
            removal = nil
        }
    }
}

struct LWRootSheetHost: View {
    let presentsSheets: Bool
    @EnvironmentObject private var presenter: LWSheetPresenter
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var contentHeight: CGFloat = 0

    var body: some View {
        GeometryReader { bounds in
            // A deferred canvas update can briefly retain both hosts. Each
            // host must authorize its own content before mounting it.
            if let content = presenter.content, (presenter.kind == .sheet) == presentsSheets {
                if presenter.kind != .sheet {
                    content.frame(maxWidth: .infinity, maxHeight: .infinity)
                        .id(presenter.identity)
                        .accessibilityAddTraits(.isModal)
                        .accessibilityAction(.escape) { presenter.dismiss() }
                        .onAppear { presenter.reduceMotion = reduceMotion }
                        .onChange(of: reduceMotion) { _, value in presenter.reduceMotion = value }
                } else {
                let cap = max(0, bounds.size.height * 0.86 - 33)
                let height = contentHeight > 0 ? min(contentHeight, cap) : cap
                ZStack(alignment: .bottom) {
                    Color(red: 10 / 255, green: 10 / 255, blue: 10 / 255)
                        .opacity(presenter.visible ? 0.32 : 0)
                        .onTapGesture { presenter.dismiss() }
                        .animation(reduceMotion ? nil : presenter.visible ? LWMotion.out(LWMotion.base) : LWMotion.in(LWMotion.quick), value: presenter.visible)
                    VStack(spacing: 0) {
                        Rectangle().fill(LWTheme.line).frame(width: 36, height: 5).padding(.top, 8)
                        ScrollView {
                            content.frame(maxWidth: .infinity).fixedSize(horizontal: false, vertical: true)
                                .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { measured in
                                    contentHeight = measured
                                    guard measured > 0, !presenter.visible else { return }
                                    let current = presenter.identity
                                    Task { @MainActor in
                                        await Task.yield()
                                        if presenter.identity == current { presenter.reveal() }
                                    }
                                }
                        }.scrollIndicators(.hidden)
                            .frame(height: height)
                        Color.clear.frame(height: 20)
                    }
                    .background(LWTheme.paper)
                    .offset(y: presenter.visible ? 0 : height + 33)
                    .animation(reduceMotion ? nil : presenter.visible ? LWMotion.out() : LWMotion.in(), value: presenter.visible)
                    .accessibilityAddTraits(.isModal)

                }
                .id(presenter.identity)
                .onAppear { presenter.reduceMotion = reduceMotion }
                .onChange(of: reduceMotion) { _, value in presenter.reduceMotion = value }
                .accessibilityAction(.escape) { presenter.dismiss() }
                }
            }
        }
        .ignoresSafeArea()
        .allowsHitTesting(presenter.content != nil)
    }
}

private struct LWSheetModifier<SheetContent: View>: ViewModifier {
    @Binding var isPresented: Bool
    @ViewBuilder let sheetContent: () -> SheetContent
    @EnvironmentObject private var presenter: LWSheetPresenter
    @State private var identity: UUID?

    func body(content: Content) -> some View {
        content.onChange(of: isPresented, initial: true) { _, presented in
            if presented {
                identity = presenter.present(content: AnyView(sheetContent())) { isPresented = false }
            } else if identity == presenter.identity { presenter.dismiss() }
        }
        .onDisappear {
            if identity == presenter.identity { presenter.dismiss() }
        }
    }
}

extension View {
    func lwSheet<SheetContent: View>(isPresented: Binding<Bool>, @ViewBuilder content: @escaping () -> SheetContent) -> some View {
        modifier(LWSheetModifier(isPresented: isPresented, sheetContent: content))
    }
}

#endif
