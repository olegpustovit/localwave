#if os(iOS)
import Foundation
import SwiftUI

/// Ordered persisted-song IDs, distinct from queue-entry UUIDs.
enum LWSongDragPayload {
    private static let singlePrefix = "localwave-song:"
    private static let groupPrefix = "localwave-songs:"

    static func encode(_ ids: [Int64]) -> String {
        var seen = Set<Int64>()
        let ordered = ids.filter { $0 > 0 && seen.insert($0).inserted }
        let prefix = ordered.count == 1 ? singlePrefix : groupPrefix
        return prefix + ordered.map(String.init).joined(separator: ",")
    }

    static func decode(_ values: [String]) -> [Int64] {
        var seen = Set<Int64>()
        var ordered: [Int64] = []
        for value in values {
            let raw: Substring
            let single: Bool
            if value.hasPrefix(singlePrefix) { raw = value.dropFirst(singlePrefix.count); single = true }
            else if value.hasPrefix(groupPrefix) { raw = value.dropFirst(groupPrefix.count); single = false }
            else { continue }
            let parts = raw.split(separator: ",", omittingEmptySubsequences: false)
            let ids = parts.compactMap { Int64($0) }
            guard ids.count == parts.count, !ids.isEmpty, ids.allSatisfy({ $0 > 0 }), !single || ids.count == 1 else { continue }
            for id in ids where seen.insert(id).inserted { ordered.append(id) }
        }
        return ordered
    }
}

/// Resolve selection only when native dragging begins, never during View body evaluation.
struct LWOrderedSongDrag: ViewModifier {
    let title: String
    let ids: () -> [Int64]
    @State private var count = 1

    func body(content: Content) -> some View {
        content.onDrag {
            let ordered = ids()
            count = ordered.count
            return NSItemProvider(object: LWSongDragPayload.encode(ordered) as NSString)
        } preview: {
            Text(count > 1 ? "\(count) songs" : title)
                .font(LWTheme.text(13).weight(.medium)).lwLineBox(13, 18.2)
                .padding(.vertical, 6).padding(.horizontal, 10)
                .foregroundStyle(LWTheme.onInk).background(LWTheme.ink)
        }
    }
}

#endif
