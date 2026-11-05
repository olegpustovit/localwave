#if os(iOS) || os(macOS)
import Foundation
import LocalWaveDomain

struct LWRootNavigationFrame: Identifiable {
    let id: UUID
    var section: String
    var playbackDestination: PlaybackContext?
    var libraryPath: [PlaybackContext]?
    var libraryBoundaryDepth: Int
    var searchQuery: String?
    var searchShowAll = false
    var folderState: WaveFolderNavigationState?
    var filesState: WaveFilesNavigationState?
    var includesFolderAncestors = false
    var showsSettings: Bool
    var sidebarRoute: String?

    init(id: UUID = UUID(), section: String, context: PlaybackContext? = nil, query: String? = nil, showAll: Bool = false) {
        self.id = id
        self.section = section
        playbackDestination = context
        searchQuery = query
        searchShowAll = showAll
        showsSettings = section == "Settings"
        switch context {
        case .album?, .artist?: sidebarRoute = nil
        case .playlist(let id, _)?: sidebarRoute = "List:\(id)"
        case .automatic(let name)?: sidebarRoute = name
        default: sidebarRoute = ["Find", "Lists", "Settings"].contains(section) ? nil : section
        }
        if section.hasPrefix("List:") { libraryBoundaryDepth = 1 }
        else {
            switch context {
            case .album?, .artist?, .playlist?, .automatic?: libraryBoundaryDepth = 1
            default: libraryBoundaryDepth = 0
            }
        }
    }
}

/// Child stacks own their own Back steps; crossing a surface adds one root frame.
struct LWRootNavigationHistory {
    private(set) var current: LWRootNavigationFrame
    private(set) var previous: [LWRootNavigationFrame] = []

    init(initial: LWRootNavigationFrame = LWRootNavigationFrame(section: "Albums")) { current = initial }
    var canGoBack: Bool { !previous.isEmpty }

    mutating func update(_ id: UUID, _ change: (inout LWRootNavigationFrame) -> Void) {
        guard current.id == id else { return }
        change(&current)
    }

    mutating func push(_ next: LWRootNavigationFrame) {
        previous.append(current)
        current = next
    }

    mutating func replace(_ next: LWRootNavigationFrame) { current = next }
    mutating func reset(_ next: LWRootNavigationFrame) { previous = []; current = next }

    @discardableResult mutating func back() -> LWRootNavigationFrame? {
        guard let restored = previous.popLast() else { return nil }
        current = restored
        return restored
    }

    mutating func reconcile(sourceIDs: Set<Int64>, listIDs: Set<Int64>) {
        previous = previous.compactMap { frame in
            guard !Self.missingSource(frame, sourceIDs: sourceIDs) else { return nil }
            return Self.removingDeletedLists(frame, listIDs: listIDs)
        }
        if Self.missingSource(current, sourceIDs: sourceIDs) {
            current = previous.popLast() ?? LWRootNavigationFrame(section: "Folders")
        }
        current = Self.removingDeletedLists(current, listIDs: listIDs)
    }

    private static func missingSource(_ frame: LWRootNavigationFrame, sourceIDs: Set<Int64>) -> Bool {
        if case .folder(let id, _, _)? = frame.playbackDestination { return !sourceIDs.contains(id) }
        if frame.section.hasPrefix("Source:"), let id = Int64(frame.section.dropFirst(7)) { return !sourceIDs.contains(id) }
        return false
    }

    private static func removingDeletedLists(_ frame: LWRootNavigationFrame, listIDs: Set<Int64>) -> LWRootNavigationFrame {
        var result = frame
        let missingRoot: Bool
        if case .playlist(let id, _)? = frame.playbackDestination { missingRoot = !listIDs.contains(id) }
        else if frame.section.hasPrefix("List:"), let id = Int64(frame.section.dropFirst(5)) { missingRoot = !listIDs.contains(id) }
        else { missingRoot = false }
        if missingRoot {
            result = LWRootNavigationFrame(section: "Lists")
        } else if let path = frame.libraryPath, let index = path.firstIndex(where: {
            if case .playlist(let id, _) = $0 { return !listIDs.contains(id) }
            return false
        }) {
            result.libraryPath = Array(path.prefix(index))
            result.libraryBoundaryDepth = min(result.libraryBoundaryDepth, index)
        }
        return result
    }
}

#endif
