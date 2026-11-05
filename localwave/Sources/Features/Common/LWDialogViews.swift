#if os(iOS)
import SwiftUI
import UIKit

enum LWDialogLayout {
    static func keyboardOffset(canvasHeight: CGFloat, panelHeight: CGFloat, keyboardTop: CGFloat?) -> CGFloat {
        guard let keyboardTop, panelHeight > 0 else { return 0 }
        return min(0, keyboardTop - 12 - (canvasHeight + panelHeight) / 2)
    }
}

struct LWDialogSurface: View {
    let request: LWDialogRequest
    @EnvironmentObject private var presenter: LWSheetPresenter
    @Environment(\.lwWideLayout) private var wide
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var text: String
    @State private var focused = false
    @State private var measuredHeight: CGFloat = 0
    @State private var keyboardTop: CGFloat?
    @ScaledMetric private var inputHeight: CGFloat = 36
    @ScaledMetric private var mobileButtonHeight: CGFloat = 46

    init(request: LWDialogRequest) {
        self.request = request
        _text = State(initialValue: request.input?.value ?? "")
    }

    private var actionSheet: Bool { request.mobileConfirmation && !wide }
    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: actionSheet ? .bottom : wide ? .top : .center) {
                Color(red: 10 / 255, green: 10 / 255, blue: 10 / 255)
                    .opacity(presenter.visible ? (actionSheet ? 0.32 : wide ? 0.12 : 0.28) : 0)
                    .contentShape(Rectangle())
                    .onTapGesture { if actionSheet { presenter.dismiss() } }
                    .animation(reduceMotion ? nil : LWMotion.out(actionSheet ? LWMotion.base : LWMotion.quick), value: presenter.visible)
                Group {
                    if actionSheet { confirmation }
                    else { dialog }
                }
                .frame(width: actionSheet ? max(0, geometry.size.width - 16) : min(wide ? 420 : 280, max(0, geometry.size.width - 32)))
                .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { height in
                    measuredHeight = height
                    guard height > 0, !presenter.visible else { return }
                    let identity = presenter.identity
                    Task { @MainActor in
                        await Task.yield()
                        guard presenter.identity == identity else { return }
                        presenter.reveal()
                    }
                }
                .scaleEffect(reduceMotion || presenter.visible || actionSheet || wide ? 1 : 0.94)
                .opacity(reduceMotion || presenter.visible || actionSheet || wide ? 1 : 0.4)
                .offset(y: reduceMotion || presenter.visible ? 0 : actionSheet ? measuredHeight : wide ? -measuredHeight : 0)
                .animation(reduceMotion ? nil : actionSheet ? LWMotion.out(LWMotion.slow) : wide ? LWMotion.out(LWMotion.base) : LWMotion.springAnimation(LWMotion.base), value: presenter.visible)
                .padding(.top, wide ? 28 : 0)
                .padding(.bottom, actionSheet ? 24 : 0)
                .offset(y: wide || actionSheet ? 0 : LWDialogLayout.keyboardOffset(canvasHeight: geometry.size.height, panelHeight: measuredHeight, keyboardTop: keyboardTop.map { $0 - geometry.frame(in: .global).minY }))
            }
            .onReceive(NotificationCenter.default.publisher(for: UIResponder.keyboardWillChangeFrameNotification)) { notification in
                guard let frame = notification.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect,
                      let window = UIApplication.shared.connectedScenes.compactMap({ $0 as? UIWindowScene }).flatMap(\.windows).first(where: \.isKeyWindow) else { return }
                keyboardTop = window.convert(frame, from: window.screen.coordinateSpace).minY
            }
            .onReceive(NotificationCenter.default.publisher(for: UIResponder.keyboardWillHideNotification)) { _ in keyboardTop = nil }
            .accessibilityElement(children: .contain)
            .accessibilityLabel(request.title)
            .background {
                Button("Cancel") { presenter.dismiss() }.keyboardShortcut(.cancelAction).hidden()
                Button(request.primaryTitle, action: submit).keyboardShortcut(.defaultAction).hidden()
            }
        }
    }

    private var dialog: some View {
        VStack(spacing: 0) {
            VStack(alignment: wide ? .leading : .center, spacing: 0) {
                Text(request.title).font(LWTheme.text(17).weight(.medium)).lwLineBox(17, 23.8)
                    .frame(maxWidth: .infinity, alignment: wide ? .leading : .center)
                    .accessibilityAddTraits(.isHeader)
                if let message = request.message {
                    Text(message).font(LWTheme.text(13)).lwLineBox(13, 18.2).foregroundStyle(LWTheme.secondary)
                        .padding(.top, 4).frame(maxWidth: .infinity, alignment: wide ? .leading : .center)
                }
                if let input = request.input {
                    LWDialogTextField(text: $text, focused: $focused, input: input, label: input.placeholder.isEmpty ? request.title : input.placeholder, submit: submit)
                        .padding(.horizontal, 10).frame(height: inputHeight)
                        .background(LWTheme.key)
                        .overlay { Rectangle().strokeBorder(LWTheme.line, lineWidth: 1) }
                        .background { if focused { Rectangle().fill(LWTheme.signal).padding(-2) } }
                        .padding(.top, 14)
                }
            }
            .multilineTextAlignment(wide ? .leading : .center)
            .padding(.horizontal, wide ? 20 : 16)
            .padding(.top, 20)
            if wide {
                HStack(spacing: 8) {
                    Spacer(minLength: 0)
                    dialogButton("Cancel", primary: false) { presenter.dismiss() }
                    dialogButton(request.primaryTitle, primary: true, action: submit)
                }.padding(.top, 18).padding(.horizontal, 20).padding(.bottom, 16)
            } else {
                HStack(spacing: 0) {
                    dialogButton("Cancel", primary: false) { presenter.dismiss() }
                    dialogButton(request.primaryTitle, primary: true, action: submit)
                        .overlay(alignment: .leading) { Rectangle().fill(LWTheme.line).frame(width: 1) }
                }
                .frame(height: mobileButtonHeight)
                .overlay(alignment: .top) { Rectangle().fill(LWTheme.line).frame(height: 1).offset(y: -1) }
                .padding(.top, 17)
            }
        }
        .background(LWTheme.paper)
        .background { Rectangle().fill(LWTheme.line).padding(-1) }
        .background { Rectangle().fill(.black.opacity(0.35)).padding(20).blur(radius: 30).offset(y: 30) }
    }

    private func dialogButton(_ title: String, primary: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title).font(LWTheme.text(wide ? 13 : 16).weight(primary ? .medium : .regular))
                .lwLineBox(wide ? 13 : 16, wide ? 18.2 : 22.4)
                .modifier(LWDialogLabelWidth(title: title, medium: primary, enabled: wide))
                .padding(.horizontal, wide ? 14 : 0)
                .padding(.leading, !wide && primary ? 1 : 0)
                .frame(maxWidth: wide ? nil : .infinity, minHeight: wide ? 30 : 46)
                .foregroundStyle(primary && request.destructive ? (wide ? Color.white : LWTheme.danger) : wide && primary ? LWTheme.onInk : LWTheme.ink)
                .background(wide ? primary ? request.destructive ? LWTheme.danger : LWTheme.ink : LWTheme.key2 : LWTheme.paper)
        }.buttonStyle(LWIconButtonStyle())
    }

    private var confirmation: some View {
        VStack(spacing: 8) {
            VStack(spacing: 1) {
                VStack(spacing: 2) {
                    Text(request.title).lwLineBox(13, 18.2)
                    if let message = request.message { Text(message).lwLineBox(13, 18.2) }
                }.font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary)
                    .multilineTextAlignment(.center).padding(12)
                    .frame(maxWidth: .infinity, minHeight: 56).background(LWTheme.paper)
                Button(action: submit) {
                    Text(request.primaryTitle).font(LWTheme.text(18)).lwLineBox(18, 25.2)
                        .foregroundStyle(request.destructive ? LWTheme.danger : LWTheme.ink)
                        .frame(maxWidth: .infinity, minHeight: 56).background(LWTheme.paper)
                }.buttonStyle(LWIconButtonStyle())
            }.background(LWTheme.line)
            Button { presenter.dismiss() } label: {
                Text("Cancel").font(LWTheme.text(18).weight(.medium)).lwLineBox(18, 25.2)
                    .frame(maxWidth: .infinity, minHeight: 56).background(LWTheme.paper)
            }.buttonStyle(LWIconButtonStyle())
        }
    }

    private func submit() { presenter.submit(dialog: request, value: text) }
}

private struct LWDialogTextField: UIViewRepresentable {
    @Binding var text: String
    @Binding var focused: Bool
    let input: LWDialogInput
    let label: String
    let submit: () -> Void
    func makeCoordinator() -> Coordinator { Coordinator(parent: self) }
    func makeUIView(context: Context) -> Field {
        let field = Field()
        field.selectsAll = input.selectAll
        field.delegate = context.coordinator
        field.addTarget(context.coordinator, action: #selector(Coordinator.changed(_:)), for: .editingChanged)
        field.text = text
        field.placeholder = input.placeholder
        field.accessibilityLabel = label
        let font = UIFontMetrics.default.scaledFont(for: UIFont(name: "HostGrotesk-Regular", size: 15) ?? .systemFont(ofSize: 15))
        field.font = font
        field.attributedPlaceholder = NSAttributedString(string: input.placeholder, attributes: [.foregroundColor: UIColor(white: 117 / 255, alpha: 1), .font: font])
        field.adjustsFontForContentSizeCategory = true
        field.textColor = UIColor(LWTheme.ink)
        field.tintColor = UIColor(LWTheme.signal)
        field.autocorrectionType = .no
        field.returnKeyType = .done
        field.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        return field
    }
    func updateUIView(_ field: Field, context: Context) {
        context.coordinator.parent = self
        if field.text != text { field.text = text }
        field.textColor = UIColor(LWTheme.ink)
        field.tintColor = UIColor(LWTheme.signal)
    }
    final class Field: UITextField {
        var selectsAll = true
        private var activated = false
        override func didMoveToWindow() {
            super.didMoveToWindow()
            guard window != nil, !activated else { return }
            activated = true
            DispatchQueue.main.async { [weak self] in
                guard let self, window != nil else { return }
                becomeFirstResponder()
                if selectsAll { selectAll(nil) }
            }
        }
    }
    final class Coordinator: NSObject, UITextFieldDelegate {
        var parent: LWDialogTextField
        init(parent: LWDialogTextField) { self.parent = parent }
        @objc func changed(_ field: UITextField) { parent.text = field.text ?? "" }
        func textFieldDidBeginEditing(_ textField: UITextField) { parent.focused = true }
        func textFieldDidEndEditing(_ textField: UITextField) { parent.focused = false }
        func textFieldShouldReturn(_ textField: UITextField) -> Bool { parent.submit(); return false }
    }
}

#endif
