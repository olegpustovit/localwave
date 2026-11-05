import SwiftUI
import XCTest
@testable import localwave

@MainActor
final class SheetPresentationTests: XCTestCase {
    func testDialogKeyboardPlacementPreservesReferenceCenterUntilButtonsWouldBeCovered() {
        XCTAssertEqual(LWDialogLayout.keyboardOffset(canvasHeight: 778, panelHeight: 179, keyboardTop: nil), 0)
        XCTAssertEqual(LWDialogLayout.keyboardOffset(canvasHeight: 778, panelHeight: 179, keyboardTop: 508), 0)
        let shift = LWDialogLayout.keyboardOffset(canvasHeight: 778, panelHeight: 367, keyboardTop: 508)
        XCTAssertEqual(shift, -76.5)
        XCTAssertEqual((778 + 367) / 2 + shift, 508 - 12)
    }

    func testDismissNotifiesOnceAndClearsReducedMotionPresentation() async {
        let presenter = LWSheetPresenter()
        presenter.reduceMotion = true
        var dismissals = 0
        presenter.present(content: AnyView(Text("Add music"))) { dismissals += 1 }
        presenter.reveal()
        XCTAssertTrue(presenter.visible)
        presenter.dismiss()
        presenter.dismiss()
        for _ in 0..<20 where presenter.content != nil { await Task.yield() }
        XCTAssertEqual(dismissals, 1)
        XCTAssertNil(presenter.content)
        XCTAssertFalse(presenter.visible)
    }

    func testLateLayoutMeasurementCannotReopenDismissingSheet() {
        let presenter = LWSheetPresenter()
        presenter.present(content: AnyView(Text("Add music"))) { }
        presenter.reveal()
        presenter.dismiss()
        presenter.reveal()
        XCTAssertFalse(presenter.visible)
    }

    func testReplacingDismissingSheetCancelsStaleRemoval() async throws {
        let presenter = LWSheetPresenter()
        var oldDismissals = 0
        let first = presenter.present(content: AnyView(Text("First"))) { oldDismissals += 1 }
        presenter.reveal()
        presenter.dismiss()
        let second = presenter.present(content: AnyView(Text("Second"))) { }
        presenter.reveal()
        try await Task.sleep(for: .milliseconds(280))
        XCTAssertNotEqual(first, second)
        XCTAssertEqual(oldDismissals, 1)
        XCTAssertNotNil(presenter.content)
        XCTAssertTrue(presenter.visible)
        presenter.reduceMotion = true
        presenter.dismiss()
    }

    func testDialogCancelIsImmediateAndNotifiesOnce() {
        let presenter = LWSheetPresenter()
        var cancellations = 0
        presenter.present(dialog: LWDialogRequest(title: "Rename List", input: LWDialogInput(value: "Late drive"), primaryTitle: "Save", onSubmit: { _ in XCTFail("Cancellation must not submit") }, onCancel: { cancellations += 1 }))
        XCTAssertEqual(presenter.kind, .dialog)
        presenter.reveal()
        presenter.dismiss()
        XCTAssertNil(presenter.content)
        XCTAssertFalse(presenter.visible)
        presenter.dismiss()
        XCTAssertEqual(cancellations, 1)
    }

    func testSubmitCanReplaceDialogAndStaleSubmitCannotAffectReplacement() {
        let presenter = LWSheetPresenter()
        var values: [String] = []
        var cancellations = 0
        let replacement = LWDialogRequest(title: "Second", primaryTitle: "Done", onSubmit: { values.append($0) })
        let first = LWDialogRequest(title: "New List", input: LWDialogInput(), primaryTitle: "Create", onSubmit: {
            values.append($0)
            presenter.present(dialog: replacement)
        }, onCancel: { cancellations += 1 })
        presenter.present(dialog: first)
        presenter.submit(dialog: first, value: "")
        XCTAssertEqual(values, [""])
        XCTAssertEqual(cancellations, 0)
        XCTAssertNotNil(presenter.content)
        presenter.submit(dialog: first, value: "stale")
        XCTAssertEqual(values, [""])
        presenter.submit(dialog: replacement, value: "next")
        XCTAssertEqual(values, ["", "next"])
        XCTAssertNil(presenter.content)
    }

    func testConfirmationHasIndependentKindAndCancelDoesNotDelete() {
        let presenter = LWSheetPresenter()
        var deleted = false
        presenter.present(dialog: LWDialogRequest(title: "Delete “Late drive”?", primaryTitle: "Delete List", destructive: true, mobileConfirmation: true, onSubmit: { _ in deleted = true }))
        XCTAssertEqual(presenter.kind, .confirmation)
        presenter.dismiss()
        XCTAssertFalse(deleted)
        XCTAssertNil(presenter.content)
    }

    func testMenuActionClearsOwnershipBeforePresentingDialog() {
        let presenter = LWSheetPresenter()
        let dialog = LWDialogRequest(title: "New List", primaryTitle: "Create", onSubmit: { _ in })
        let action = LWMenuAction(title: "New list…", action: { presenter.present(dialog: dialog) })
        let menu = LWMenuRequest(groups: [LWMenuGroup([action])])
        presenter.present(menu: menu)
        presenter.perform(menuAction: action, requestID: menu.id)
        XCTAssertEqual(presenter.kind, .dialog)
        XCTAssertNotNil(presenter.content)
        presenter.perform(menuAction: action, requestID: menu.id)
        XCTAssertEqual(presenter.kind, .dialog)
        presenter.dismiss()
    }

    func testDisabledMenuActionAndSubmenuDoNotDismissOrRun() {
        let presenter = LWSheetPresenter()
        var calls = 0
        let disabled = LWMenuAction(title: "Unavailable", disabled: true, action: { calls += 1 })
        let submenu = LWMenuAction(title: "Add to list", submenu: { calls += 1; return [] })
        let menu = LWMenuRequest(groups: [LWMenuGroup([disabled, submenu])])
        presenter.present(menu: menu)
        presenter.perform(menuAction: disabled, requestID: menu.id)
        presenter.perform(menuAction: submenu, requestID: menu.id)
        XCTAssertEqual(calls, 0)
        XCTAssertNotNil(presenter.content)
        presenter.dismiss()
    }

    func testReplacingFadingMenuCannotRemoveNewSheet() async throws {
        let presenter = LWSheetPresenter()
        presenter.present(menu: LWMenuRequest(groups: []))
        presenter.reveal()
        presenter.dismissMenuFromBackdrop(wide: false)
        XCTAssertFalse(presenter.visible)
        presenter.present(content: AnyView(Text("Queue"))) {}
        presenter.reveal()
        try await Task.sleep(for: .milliseconds(180))
        XCTAssertEqual(presenter.kind, .sheet)
        XCTAssertTrue(presenter.visible)
        XCTAssertNotNil(presenter.content)
        presenter.reduceMotion = true
        presenter.dismiss()
    }

    func testMenuRevealRejectsCancelledTaskWithoutShowingSamePresentation() async {
        let presenter = LWSheetPresenter()
        let request = LWMenuRequest(groups: [])
        let identity = presenter.present(menu: request)
        var continuation: CheckedContinuation<Void, Never>?
        let reveal = Task { @MainActor in
            await withCheckedContinuation { continuation = $0 }
            return presenter.revealMenu(requestID: request.id, presentationID: identity)
        }
        while continuation == nil { await Task.yield() }
        reveal.cancel()
        continuation?.resume()
        let shown = await reveal.value
        XCTAssertFalse(shown)
        XCTAssertFalse(presenter.visible)
        XCTAssertEqual(presenter.identity, identity)
        XCTAssertNotNil(presenter.content)
        presenter.dismiss()
    }

    func testRepeatedMenuRenderRevealKeepsOwnershipAndRejectsDismissal() {
        let presenter = LWSheetPresenter()
        let request = LWMenuRequest(groups: [])
        let identity = presenter.present(menu: request)
        XCTAssertTrue(presenter.revealMenu(requestID: request.id, presentationID: identity))
        XCTAssertTrue(presenter.revealMenu(requestID: request.id, presentationID: identity))
        XCTAssertEqual(presenter.identity, identity)
        XCTAssertTrue(presenter.visible)
        presenter.dismissMenuFromBackdrop(wide: false)
        XCTAssertFalse(presenter.revealMenu(requestID: request.id, presentationID: identity))
        XCTAssertFalse(presenter.visible)
        presenter.dismissImmediately()
    }

    func testStaleMenuRevealCannotRevealReplacementDialog() {
        let presenter = LWSheetPresenter()
        let request = LWMenuRequest(groups: [])
        let identity = presenter.present(menu: request)
        presenter.present(dialog: LWDialogRequest(title: "New List", primaryTitle: "Create", onSubmit: { _ in }))
        XCTAssertFalse(presenter.revealMenu(requestID: request.id, presentationID: identity))
        XCTAssertEqual(presenter.kind, .dialog)
        XCTAssertFalse(presenter.visible)
        presenter.dismiss()
    }

    func testReusedMenuRequestStillRequiresItsNewPresentationIdentity() {
        let presenter = LWSheetPresenter()
        let request = LWMenuRequest(groups: [])
        let original = presenter.present(menu: request)
        let replacement = presenter.present(menu: request)
        XCTAssertFalse(presenter.revealMenu(requestID: request.id, presentationID: original))
        XCTAssertFalse(presenter.visible)
        XCTAssertTrue(presenter.revealMenu(requestID: request.id, presentationID: replacement))
        XCTAssertEqual(presenter.identity, replacement)
        presenter.dismiss()
    }

    func testUnavailableBackdropEventuallyRevealsCurrentMenu() async {
        let presenter = LWSheetPresenter()
        let request = LWMenuRequest(groups: [])
        let identity = presenter.present(menu: request)
        XCTAssertFalse(presenter.visible)
        let revealed = await presenter.revealMenuWithoutBackdrop(requestID: request.id, presentationID: identity)
        XCTAssertTrue(revealed)
        XCTAssertTrue(presenter.visible)
        presenter.dismissImmediately()
    }

    func testCancelledBackdropWaitCannotRevealMenu() async {
        let presenter = LWSheetPresenter()
        let request = LWMenuRequest(groups: [])
        let identity = presenter.present(menu: request)
        let reveal = Task { await presenter.revealMenuWithoutBackdrop(requestID: request.id, presentationID: identity) }
        reveal.cancel()
        let revealed = await reveal.value
        XCTAssertFalse(revealed)
        XCTAssertFalse(presenter.visible)
        presenter.dismissImmediately()
    }

    func testBackdropWaitCannotRevealReplacementDialog() async {
        let presenter = LWSheetPresenter()
        let request = LWMenuRequest(groups: [])
        let identity = presenter.present(menu: request)
        let reveal = Task { await presenter.revealMenuWithoutBackdrop(requestID: request.id, presentationID: identity) }
        presenter.present(dialog: LWDialogRequest(title: "New List", primaryTitle: "Create", onSubmit: { _ in }))
        let revealed = await reveal.value
        XCTAssertFalse(revealed)
        XCTAssertFalse(presenter.visible)
        XCTAssertEqual(presenter.kind, .dialog)
        presenter.dismissImmediately()
    }
}
