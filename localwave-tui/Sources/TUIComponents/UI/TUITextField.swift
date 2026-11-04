//
//  TUITextField.swift
//  localwave-tui
//
//  Created by Claude Code on 04.10.2025.
//

import SwiftTUI
import Combine

/// State management for TUI text input
public struct TUITextFieldState {
    public var text: String
    public var cursorPosition: Int
    public var isFocused: Bool

    public init(text: String = "", cursorPosition: Int = 0, isFocused: Bool = false) {
        self.text = text
        self.cursorPosition = min(cursorPosition, text.count)
        self.isFocused = isFocused
    }

    /// Insert character at cursor position
    public mutating func insertCharacter(_ char: Character) {
        let index = text.index(text.startIndex, offsetBy: cursorPosition)
        text.insert(char, at: index)
        cursorPosition += 1
    }

    /// Delete character before cursor (backspace)
    public mutating func deleteBackward() {
        guard cursorPosition > 0 else { return }
        let index = text.index(text.startIndex, offsetBy: cursorPosition - 1)
        text.remove(at: index)
        cursorPosition -= 1
    }

    /// Move cursor left
    public mutating func moveCursorLeft() {
        cursorPosition = max(0, cursorPosition - 1)
    }

    /// Move cursor right
    public mutating func moveCursorRight() {
        cursorPosition = min(text.count, cursorPosition + 1)
    }

    /// Move cursor to start
    public mutating func moveCursorToStart() {
        cursorPosition = 0
    }

    /// Move cursor to end
    public mutating func moveCursorToEnd() {
        cursorPosition = text.count
    }

    /// Clear all text
    public mutating func clear() {
        text = ""
        cursorPosition = 0
    }
}

/// Text input field with cursor support
///
/// Example usage:
/// ```swift
/// TUITextField(
///     state: textFieldState,
///     placeholder: "Search...",
///     width: 40
/// )
/// .onKeyPress("a") { if textFieldState.isFocused { textFieldState.insertCharacter("a") } }
/// // ... handle other keys
/// ```
public struct TUITextField: View {
    @Binding var state: TUITextFieldState
    let placeholder: String
    let width: Int
    let label: String?

    public init(
        state: Binding<TUITextFieldState>,
        placeholder: String = "",
        width: Int = 40,
        label: String? = nil
    ) {
        self._state = state
        self.placeholder = placeholder
        self.width = width
        self.label = label
    }

    public var body: some View {
        VStack(spacing: 0) {
            if let label = label {
                HStack {
                    Text(label)
                    Spacer()
                }
            }

            HStack {
                Text(renderTextField())
                Spacer()
            }
        }
    }

    private func renderTextField() -> String {
        let displayText = state.text.isEmpty ? placeholder : state.text
        let textWithCursor: String

        if state.isFocused {
            // Insert cursor character at cursor position
            if state.cursorPosition >= displayText.count {
                textWithCursor = displayText + "┃"
            } else {
                let beforeCursor = String(displayText.prefix(state.cursorPosition))
                let atCursor = String(displayText[displayText.index(displayText.startIndex, offsetBy: state.cursorPosition)])
                let afterCursor = String(displayText.suffix(from: displayText.index(displayText.startIndex, offsetBy: state.cursorPosition + 1)))
                textWithCursor = beforeCursor + "┃" + atCursor + afterCursor
            }
        } else {
            textWithCursor = displayText
        }

        // Pad or truncate to width
        if textWithCursor.count > width {
            return String(textWithCursor.prefix(width - 1)) + "…"
        } else {
            return textWithCursor + String(repeating: " ", count: width - textWithCursor.count)
        }
    }
}

/// Search field with icon
///
/// Example usage:
/// ```swift
/// TUISearchField(
///     state: searchState,
///     width: 50
/// )
/// ```
public struct TUISearchField: View {
    @Binding var state: TUITextFieldState
    let width: Int

    public init(state: Binding<TUITextFieldState>, width: Int = 40) {
        self._state = state
        self.width = width
    }

    public var body: some View {
        HStack {
            Text("🔍 ")
            TUITextField(
                state: $state,
                placeholder: "Type to search...",
                width: width - 3
            )
            Spacer()
        }
    }
}

/// Simple prompt/input dialog
///
/// Example usage:
/// ```swift
/// TUIPrompt(
///     title: "Enter playlist name",
///     state: inputState,
///     onSubmit: { name in
///         // Handle submission
///     },
///     onCancel: {
///         // Handle cancellation
///     }
/// )
/// ```
public struct TUIPrompt: View {
    let title: String
    @Binding var state: TUITextFieldState
    let onSubmit: (String) -> Void
    let onCancel: () -> Void

    public init(
        title: String,
        state: Binding<TUITextFieldState>,
        onSubmit: @escaping (String) -> Void,
        onCancel: @escaping () -> Void
    ) {
        self.title = title
        self._state = state
        self.onSubmit = onSubmit
        self.onCancel = onCancel
    }

    public var body: some View {
        VStack(spacing: 1) {
            Spacer()

            // Border top
            HStack {
                Text("╭" + String(repeating: "─", count: 50) + "╮")
                Spacer()
            }

            // Title
            HStack {
                Text("│ ")
                Text(title)
                Spacer()
                Text(" │")
            }

            // Divider
            HStack {
                Text("├" + String(repeating: "─", count: 50) + "┤")
                Spacer()
            }

            // Input field
            HStack {
                Text("│ ")
                TUITextField(state: $state, width: 46)
                Text(" │")
                Spacer()
            }

            // Border bottom
            HStack {
                Text("╰" + String(repeating: "─", count: 50) + "╯")
                Spacer()
            }

            // Help text
            HStack {
                Text("[Enter] Submit  [Esc] Cancel")
                Spacer()
            }

            Spacer()
        }
    }
}

/// Extension for text field keyboard handling
public extension TUITextFieldState {
    /// Handle printable character input
    mutating func handleCharacter(_ char: Character) {
        guard isFocused else { return }
        if char.isLetter || char.isNumber || char.isWhitespace || char.isPunctuation {
            insertCharacter(char)
        }
    }

    /// Handle backspace key
    mutating func handleBackspace() {
        guard isFocused else { return }
        deleteBackward()
    }

    /// Handle arrow keys
    mutating func handleLeftArrow() {
        guard isFocused else { return }
        moveCursorLeft()
    }

    mutating func handleRightArrow() {
        guard isFocused else { return }
        moveCursorRight()
    }

    /// Handle home/end
    mutating func handleHome() {
        guard isFocused else { return }
        moveCursorToStart()
    }

    mutating func handleEnd() {
        guard isFocused else { return }
        moveCursorToEnd()
    }
}
