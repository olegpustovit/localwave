import UIKit

class AppDelegate: UIResponder, UIApplicationDelegate {
    #if targetEnvironment(macCatalyst)
    private var selectionCount = 0
    private var selectionObservation: NSObjectProtocol?

    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        selectionObservation = NotificationCenter.default.addObserver(forName: Notification.Name("LWSelectionChanged"), object: nil, queue: .main) { [weak self] notification in
            guard let count = notification.object as? Int else { return }
            self?.selectionCount = max(0, count)
            UIMenuSystem.main.setNeedsRevalidate()
        }
        return true
    }

    deinit {
        if let selectionObservation { NotificationCenter.default.removeObserver(selectionObservation) }
    }

    override func buildMenu(with builder: UIMenuBuilder) {
        guard builder.system == .main else { return }
        if #available(iOS 26.0, *) { builder.remove(menu: .newItem) }
        else { builder.remove(menu: .newScene) }
        let controls = UIMenu(title: "Controls", identifier: UIMenu.Identifier("localwave.controls"), children: [
            command("Play or pause", action: "playPause", input: " ", modifiers: []),
            command("Next song", action: "next", input: UIKeyCommand.inputRightArrow),
            command("Previous song", action: "previous", input: UIKeyCommand.inputLeftArrow),
            command("Shuffle", action: "shuffle", input: "s", modifiers: [.command, .shift]),
            command("Repeat", action: "repeat", input: "r", modifiers: [.command, .shift])
        ])
        builder.insertSibling(controls, afterMenu: .edit)
        builder.insertChild(UIMenu(title: "", options: .displayInline, children: [
            command("Albums", action: "Albums", input: "1"),
            command("Artists", action: "Artists", input: "2"),
            command("Songs", action: "Songs", input: "3"),
            command("Command palette…", action: "commands", input: "k"),
            command("Find", action: "Find", input: "f"),
            command("Toggle player panel", action: "panel", input: "p", modifiers: [.command, .shift])
        ]), atStartOfMenu: .view)
        builder.insertChild(UIMenu(title: "", options: .displayInline, children: [
            command("Add Folder…", action: "addFolder", input: "o"),
            command("Add Files…", action: "addFiles", input: "o", modifiers: [.command, .shift]),
            command("New List…", action: "New List", input: "n"),
            command("New List from Selection…", action: "New List from Selection", input: "n", modifiers: [.command, .shift]),
            command("Lists", action: "Lists", input: "l", modifiers: [.command, .shift])
        ]), atStartOfMenu: .file)
        builder.insertChild(UIMenu(title: "", options: .displayInline, children: [
            UICommand(title: "Keyboard Shortcuts", image: nil, action: #selector(runCommand(_:)), propertyList: "shortcuts")
        ]), atStartOfMenu: .help)
    }

    private func command(_ title: String, action: String, input: String, modifiers: UIKeyModifierFlags = .command) -> UIKeyCommand {
        UIKeyCommand(title: title, action: #selector(runCommand(_:)), input: input, modifierFlags: modifiers, propertyList: action)
    }

    override func validate(_ command: UICommand) {
        super.validate(command)
        guard command.propertyList as? String == "New List from Selection" else { return }
        if selectionCount == 0 { command.attributes.insert(.disabled) }
        else { command.attributes.remove(.disabled) }
    }

    @objc private func runCommand(_ command: UICommand) {
        guard let action = command.propertyList as? String else { return }
        guard action != "New List from Selection" || selectionCount > 0 else { return }
        NotificationCenter.default.post(name: Notification.Name("LWCommand"), object: action)
    }
    #endif
}
