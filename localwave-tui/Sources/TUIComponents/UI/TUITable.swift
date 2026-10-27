//
//  TUITable.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI

/// Column definition for TUITable
public struct TUITableColumn {
    public let title: String
    public let width: Int
    public let alignment: TUITableAlignment

    public init(title: String, width: Int, alignment: TUITableAlignment = .left) {
        self.title = title
        self.width = width
        self.alignment = alignment
    }
}

/// Text alignment for table columns
public enum TUITableAlignment {
    case left
    case right
    case center
}

/// Table view component for displaying multi-column data
///
/// Example usage:
/// ```swift
/// let columns = [
///     TUITableColumn(title: "Title", width: 30),
///     TUITableColumn(title: "Artist", width: 20),
///     TUITableColumn(title: "Duration", width: 8, alignment: .right)
/// ]
///
/// TUITable(
///     columns: columns,
///     items: songs,
///     state: listState,
///     rowContent: { song, isSelected in
///         [song.title, song.artist, formatDuration(song.duration)]
///     }
/// )
/// ```
public struct TUITable<Item>: View {
    let columns: [TUITableColumn]
    let items: [Item]
    @Binding var state: TUIListState
    let rowContent: (Item, Bool) -> [String]

    public init(
        columns: [TUITableColumn],
        items: [Item],
        state: Binding<TUIListState>,
        rowContent: @escaping (Item, Bool) -> [String]
    ) {
        self.columns = columns
        self.items = items
        self._state = state
        self.rowContent = rowContent
    }

    public var body: some View {
        VStack(spacing: 0) {
            // Header row
            headerRow

            // Divider
            dividerRow

            // Data rows
            if items.isEmpty {
                Text("(empty)")
                Spacer()
            } else {
                dataRows
                Spacer()
            }
        }
    }

    private var headerRow: some View {
        HStack(spacing: 1) {
            ForEach(Array(columns.enumerated()), id: \.offset) { _, column in
                Text(alignText(column.title, width: column.width, alignment: column.alignment))
            }
            Spacer()
        }
    }

    private var dividerRow: some View {
        HStack(spacing: 1) {
            ForEach(Array(columns.enumerated()), id: \.offset) { _, column in
                Text(String(repeating: "─", count: column.width))
            }
            Spacer()
        }
    }

    private var dataRows: some View {
        VStack(spacing: 0) {
            // Calculate visible range with bounds checking
            let visibleStart = min(state.scrollOffset, items.count)
            let visibleEnd = min(state.scrollOffset + state.effectiveVisibleHeight, items.count)

            // Only render if we have a valid range
            // ULTRA-DEFENSIVE: Verify all bounds before creating ForEach Range
            if visibleStart >= 0 &&
               visibleStart < items.count &&
               visibleEnd > 0 &&
               visibleEnd <= items.count &&
               visibleStart < visibleEnd {
                ForEach(visibleStart..<visibleEnd, id: \.self) { index in
                    let item = items[index]
                    let isSelected = index == state.selectedIndex
                    let values = rowContent(item, isSelected)

                    dataRow(values: values, isSelected: isSelected)
                }
            }
        }
    }

    private func dataRow(values: [String], isSelected: Bool) -> some View {
        HStack(spacing: 1) {
            // Selection indicator
            Text(isSelected ? ">" : " ")

            // Column values
            ForEach(Array(zip(columns.indices, columns)), id: \.0) { index, column in
                let value = index < values.count ? values[index] : ""
                Text(alignText(value, width: column.width, alignment: column.alignment))
            }

            Spacer()
        }
    }

    private func alignText(_ text: String, width: Int, alignment: TUITableAlignment) -> String {
        guard width > 0 else { return " " }
        let truncated = text.count > width ? String(text.prefix(max(0, width - 1))) + "…" : text
        let padding = max(0, width - truncated.count)

        switch alignment {
        case .left:
            return truncated + String(repeating: " ", count: padding)
        case .right:
            return String(repeating: " ", count: padding) + truncated
        case .center:
            let leftPad = padding / 2
            let rightPad = padding - leftPad
            return String(repeating: " ", count: leftPad) + truncated + String(repeating: " ", count: rightPad)
        }
    }
}

/// Simplified table for displaying key-value pairs
public struct TUIKeyValueTable: View {
    let pairs: [(key: String, value: String)]
    let keyWidth: Int
    let valueWidth: Int

    public init(
        pairs: [(key: String, value: String)],
        keyWidth: Int = 20,
        valueWidth: Int = 40
    ) {
        self.pairs = pairs
        self.keyWidth = keyWidth
        self.valueWidth = valueWidth
    }

    public var body: some View {
        VStack(spacing: 0) {
            ForEach(Array(pairs.enumerated()), id: \.offset) { _, pair in
                HStack {
                    Text(padRight(pair.key, width: keyWidth))
                    Text(": ")
                    Text(padRight(pair.value, width: valueWidth))
                    Spacer()
                }
            }
            Spacer()
        }
    }

    private func padRight(_ text: String, width: Int) -> String {
        guard width > 0 else { return " " }
        let truncated = text.count > width ? String(text.prefix(max(0, width - 1))) + "…" : text
        let padding = max(0, width - truncated.count)
        return truncated + String(repeating: " ", count: padding)
    }
}
