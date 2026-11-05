import SwiftUI

struct LWDialogInput {
    var value = ""
    var placeholder = ""
    var selectAll = true
}

struct LWDialogRequest {
    let id = UUID()
    let title: String
    var message: String? = nil
    var input: LWDialogInput? = nil
    let primaryTitle: String
    var destructive = false
    var mobileConfirmation = false
    let onSubmit: (String) -> Void
    var onCancel: () -> Void = {}
}

struct LWMenuAction: Identifiable {
    let id = UUID()
    let title: String
    var icon: String? = nil
    var shortcut: String? = nil
    var destructive = false
    var disabled = false
    var submenu: (() -> [LWMenuGroup])? = nil
    var action: () -> Void = {}

    init(title: String, icon: String? = nil, shortcut: String? = nil, destructive: Bool = false, disabled: Bool = false, action: @escaping () -> Void = {}) {
        self.title = title
        self.icon = icon
        self.shortcut = shortcut
        self.destructive = destructive
        self.disabled = disabled
        self.action = action
    }

    init(title: String, icon: String? = nil, shortcut: String? = nil, destructive: Bool = false, disabled: Bool = false, submenu: @escaping () -> [LWMenuGroup]) {
        self.title = title
        self.icon = icon
        self.shortcut = shortcut
        self.destructive = destructive
        self.disabled = disabled
        self.submenu = submenu
    }
}

struct LWMenuGroup: Identifiable {
    let id = UUID()
    let actions: [LWMenuAction]
    init(_ actions: [LWMenuAction]) { self.actions = actions }
}

struct LWMenuRequest {
    let id = UUID()
    let groups: [LWMenuGroup]
    var preview: AnyView? = nil
    /// Global view coordinates; the host converts these into its own canvas.
    var anchor: CGPoint? = nil
}

enum LWPresentationNotice {
    static func show(_ text: String) {
        NotificationCenter.default.post(name: Notification.Name("LWShowNotice"), object: text)
    }
}
