# SwiftTUI Scrolling Test Infrastructure Plan

**Goal**: Build a fully reproducible virtual rendering environment for SwiftTUI where we can simulate keypresses, render to virtual buffers, and assert rendering results without running a real terminal.

**Problem Statement**: The list scrolling bug is complex and cannot be debugged effectively in a real terminal. We need a test harness that:
1. Renders to a virtual buffer instead of stdout
2. Simulates keyboard events (j/k/Ctrl+D/Ctrl+U)
3. Captures rendered output as 2D grid of cells
4. Allows assertions on rendered state
5. Enables step-by-step debugging of render pipeline

## Master Checklist

- [x] **Phase 1: Virtual Terminal Abstraction** (Foundation) ✅
  - [x] Task 1.1: Create VirtualTerminal protocol and implementation (~200 LOC) ✅
  - [x] Task 1.2: Implement VirtualRenderer that writes to buffer (~250 LOC) ✅
  - [x] Task 1.3: Create TerminalBuffer 2D grid with cell inspection (~150 LOC) ✅ (completed with 1.1)
  - [x] Task 1.4: Add escape sequence parser for assertions (~200 LOC) ✅ (completed with 1.1)

- [x] **Phase 2: Test Application Infrastructure** (Test Harness) ✅
  - [x] Task 2.1: Create TestableApplication with injectable dependencies (~250 LOC) ✅
  - [x] Task 2.2: Implement KeyEventSimulator for input injection (~200 LOC) ✅
  - [x] Task 2.3: Build RenderCapture system for buffer snapshots (~150 LOC) ✅ (completed with 2.1)
  - [x] Task 2.4: Add async/await test helpers for update cycles (~100 LOC) ✅

- [x] **Phase 3: Assertion Framework** (Verification) ✅
  - [x] Task 3.1: Create TerminalMatcher DSL for readable assertions (~405 LOC) ✅
  - [x] Task 3.2: Implement cell-by-cell diff visualization (~240 LOC) ✅
  - [x] Task 3.3: Add scrolling-specific assertion helpers (~310 LOC) ✅
  - [x] Task 3.4: Build render timeline debugger (~335 LOC) ✅

- [x] **Phase 4: Scrolling Test Suite** (Actual Tests) ✅
  - [x] Task 4.1: Test basic j/k navigation with 10 items (~390 LOC) ✅
  - [x] Task 4.2: Test Ctrl+D/Ctrl+U half-page scrolling (~430 LOC) ✅
  - [x] Task 4.3: Test viewport boundaries and edge cases (~440 LOC) ✅
  - [x] Task 4.4: Test list with 100+ items for performance (~460 LOC) ✅
  - [x] Task 4.5: Test ForEach identity-based diffing behavior (~450 LOC) ✅
  - [x] Task 4.6: Test zero-sized rect invalidation edge cases (~520 LOC) ✅

---

## Architecture Overview

### Current SwiftTUI Architecture

```
Application
    ├── Node (view tree)
    ├── Control (layout)
    ├── Window (layer hierarchy)
    └── Renderer (escape sequences → stdout)
```

### New Test Architecture

```
TestableApplication
    ├── Node (view tree)
    ├── Control (layout)
    ├── Window (layer hierarchy)
    └── VirtualRenderer (escape sequences → TerminalBuffer)
        └── TerminalBuffer (2D Cell array)
            └── Assertions & Diffing
```

**Key Innovation**: Replace `Renderer` with `VirtualRenderer` that implements same interface but writes to memory instead of stdout.

---

## Phase 1: Virtual Terminal Abstraction

### Task 1.1: Create VirtualTerminal Protocol

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/VirtualTerminal.swift`

**Problem**: Renderer is tightly coupled to stdout via `write()` syscalls. We need an abstraction.

**Solution**: Create protocol that both real terminal and virtual buffer can implement.

**Implementation** (~200 LOC):

```swift
// MARK: - Terminal Protocol

/// Abstraction over terminal output
protocol Terminal {
    /// Write string to terminal (escape sequences + text)
    func write(_ string: String)

    /// Flush any buffered output
    func flush()

    /// Get current terminal size
    var size: Size { get }

    /// Clear screen
    func clear()
}

// MARK: - Real Terminal Implementation

/// Real stdout-based terminal (production)
final class StdoutTerminal: Terminal {
    private var writeBuffer = ""
    private let bufferFlushThreshold = 16384

    func write(_ string: String) {
        writeBuffer += string
        if writeBuffer.count >= bufferFlushThreshold {
            flush()
        }
    }

    func flush() {
        guard !writeBuffer.isEmpty else { return }
        writeBuffer.withCString {
            _ = Foundation.write(STDOUT_FILENO, $0, strlen($0))
        }
        writeBuffer = ""
    }

    var size: Size {
        var wsz = winsize()
        guard ioctl(STDOUT_FILENO, UInt(TIOCGWINSZ), &wsz) == 0 else {
            return Size(width: Extended(80), height: Extended(24))
        }
        return Size(
            width: Extended(Int(wsz.ws_col)),
            height: Extended(Int(wsz.ws_row))
        )
    }

    func clear() {
        write("\u{1B}[2J\u{1B}[H")
        flush()
    }
}

// MARK: - Virtual Terminal Implementation

/// Virtual in-memory terminal (testing)
final class VirtualTerminal: Terminal {
    var buffer: TerminalBuffer
    var outputLog: [String] = []  // Log all writes for debugging

    init(width: Int, height: Int) {
        self.buffer = TerminalBuffer(width: width, height: height)
    }

    func write(_ string: String) {
        outputLog.append(string)
        buffer.processEscapeSequences(string)
    }

    func flush() {
        // No-op for virtual terminal, but kept for protocol compliance
    }

    var size: Size {
        Size(
            width: Extended(buffer.width),
            height: Extended(buffer.height)
        )
    }

    func clear() {
        buffer.clear()
        outputLog.append("[CLEAR]")
    }

    /// Get rendered content at position
    func cell(at position: Position) -> Cell? {
        buffer.cell(at: position)
    }

    /// Get entire row as string (for assertions)
    func row(_ line: Int) -> String {
        buffer.row(line)
    }

    /// Snapshot current buffer state
    func snapshot() -> BufferSnapshot {
        BufferSnapshot(buffer: buffer)
    }
}
```

**Expected Impact**: Enables testing without stdout, foundation for all test infrastructure.

---

### Task 1.2: Implement VirtualRenderer

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/VirtualRenderer.swift`

**Problem**: Current Renderer writes directly to stdout. Need version that writes to VirtualTerminal.

**Solution**: Subclass Renderer or create variant that uses Terminal protocol.

**Implementation** (~250 LOC):

```swift
/// Renderer that writes to virtual terminal buffer
final class VirtualRenderer {
    private let layer: Layer
    private let terminal: VirtualTerminal

    // Cache for avoiding redundant renders (same as Renderer)
    private var cache: [[Cell?]] = []
    private var currentForegroundColor: Color?
    private var currentBackgroundColor: Color?
    private var currentAttributes = CellAttributes()

    weak var application: TestableApplication?

    init(layer: Layer, terminal: VirtualTerminal) {
        self.layer = layer
        self.terminal = terminal
        self.cache = Array(
            repeating: Array(repeating: nil, count: terminal.buffer.width),
            count: terminal.buffer.height
        )
    }

    func setCache() {
        let width = terminal.buffer.width
        let height = terminal.buffer.height
        cache = Array(
            repeating: Array(repeating: nil, count: width),
            count: height
        )
    }

    /// Draw invalidated region (same logic as Renderer.update())
    func update() {
        if let invalidated = layer.invalidated {
            draw(rect: invalidated)
            terminal.flush()
            layer.invalidated = nil
        }
    }

    /// Draw entire layer
    func draw() {
        draw(rect: Rect(position: .zero, size: layer.frame.size))
        terminal.flush()
    }

    /// Draw specific area (same as Renderer.draw())
    private func draw(rect: Rect? = nil) {
        if rect == nil { layer.invalidated = nil }
        let rect = rect ?? Rect(position: .zero, size: layer.frame.size)

        guard rect.size.width > 0, rect.size.height > 0 else {
            return
        }

        let minLine = rect.minLine.intValue
        let maxLine = rect.maxLine.intValue

        guard minLine <= maxLine else { return }

        for line in minLine...maxLine {
            drawRow(line: line, startCol: rect.minColumn.intValue, endCol: rect.maxColumn.intValue)
        }
    }

    /// Draw single row (row-based rendering from Task 1.1)
    private func drawRow(line: Int, startCol: Int, endCol: Int) {
        guard startCol <= endCol else { return }

        var rowBuffer = ""
        var rowForegroundColor: Color? = currentForegroundColor
        var rowBackgroundColor: Color? = currentBackgroundColor
        var rowAttributes = currentAttributes

        // Move cursor to start of row
        rowBuffer += EscapeSequence.moveTo(Position(column: Extended(startCol), line: Extended(line)))

        for column in startCol...endCol {
            let position = Position(column: Extended(column), line: Extended(line))

            guard position.line >= 0, position.line < layer.frame.size.height,
                  position.column >= 0, position.column < layer.frame.size.width else {
                continue
            }

            guard let cell = layer.cell(at: position) else { continue }

            // Check cache to avoid redundant writes
            if line < cache.count && column < cache[line].count,
               cache[line][column] == cell {
                continue
            }

            // Update cache
            if line < cache.count && column < cache[line].count {
                cache[line][column] = cell
            }

            // Update colors only when they change
            if rowForegroundColor != cell.foregroundColor {
                rowBuffer += cell.foregroundColor.foregroundEscapeSequence
                rowForegroundColor = cell.foregroundColor
            }

            let bg = cell.backgroundColor ?? .default
            if rowBackgroundColor != bg {
                rowBuffer += bg.backgroundEscapeSequence
                rowBackgroundColor = bg
            }

            if rowAttributes != cell.attributes {
                rowBuffer += buildAttributeDelta(from: rowAttributes, to: cell.attributes)
                rowAttributes = cell.attributes
            }

            rowBuffer += String(cell.char)
        }

        currentForegroundColor = rowForegroundColor
        currentBackgroundColor = rowBackgroundColor
        currentAttributes = rowAttributes

        if !rowBuffer.isEmpty {
            terminal.write(rowBuffer)
        }
    }

    private func buildAttributeDelta(from: CellAttributes, to: CellAttributes) -> String {
        var result = ""
        if from.bold != to.bold {
            result += to.bold ? EscapeSequence.enableBold : EscapeSequence.disableBold
        }
        if from.italic != to.italic {
            result += to.italic ? EscapeSequence.enableItalic : EscapeSequence.disableItalic
        }
        if from.underline != to.underline {
            result += to.underline ? EscapeSequence.enableUnderline : EscapeSequence.disableUnderline
        }
        return result
    }

    func stop() {
        terminal.clear()
    }
}
```

**Expected Impact**: Enables virtual rendering to memory buffer for tests.

---

### Task 1.3: Create TerminalBuffer 2D Grid

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/TerminalBuffer.swift`

**Problem**: Need 2D grid to store rendered cells for inspection.

**Solution**: Create buffer that processes escape sequences and maintains cell grid.

**Implementation** (~150 LOC):

```swift
/// 2D grid of cells representing terminal screen
struct TerminalBuffer {
    private(set) var width: Int
    private(set) var height: Int
    private var cells: [[Cell]]

    private var cursorLine = 0
    private var cursorColumn = 0
    private var currentFg: Color = .default
    private var currentBg: Color = .default
    private var currentAttrs = CellAttributes()

    init(width: Int, height: Int) {
        self.width = width
        self.height = height
        self.cells = Array(
            repeating: Array(
                repeating: Cell(char: " ", foregroundColor: .default, backgroundColor: .default),
                count: width
            ),
            count: height
        )
    }

    /// Get cell at position
    func cell(at position: Position) -> Cell? {
        let line = position.line.intValue
        let col = position.column.intValue

        guard line >= 0, line < height, col >= 0, col < width else {
            return nil
        }

        return cells[line][col]
    }

    /// Get entire row as string
    func row(_ line: Int) -> String {
        guard line >= 0, line < height else { return "" }
        return cells[line].map { String($0.char) }.joined()
    }

    /// Set cell at position
    mutating func setCell(_ cell: Cell, at position: Position) {
        let line = position.line.intValue
        let col = position.column.intValue

        guard line >= 0, line < height, col >= 0, col < width else {
            return
        }

        cells[line][col] = cell
    }

    /// Clear entire buffer
    mutating func clear() {
        cells = Array(
            repeating: Array(
                repeating: Cell(char: " ", foregroundColor: .default, backgroundColor: .default),
                count: width
            ),
            count: height
        )
        cursorLine = 0
        cursorColumn = 0
    }

    /// Process escape sequences and update buffer
    mutating func processEscapeSequences(_ string: String) {
        var i = string.startIndex

        while i < string.endIndex {
            let char = string[i]

            if char == "\u{1B}" {
                // Parse escape sequence
                i = processEscapeSequence(string, from: i)
            } else if char == "\n" {
                cursorLine += 1
                cursorColumn = 0
            } else if char == "\r" {
                cursorColumn = 0
            } else {
                // Regular character - write to buffer at cursor position
                let cell = Cell(
                    char: char,
                    foregroundColor: currentFg,
                    backgroundColor: currentBg,
                    attributes: currentAttrs
                )
                setCell(cell, at: Position(column: Extended(cursorColumn), line: Extended(cursorLine)))
                cursorColumn += 1
            }

            i = string.index(after: i)
        }
    }

    /// Parse and apply escape sequence, return next index
    private mutating func processEscapeSequence(_ string: String, from start: String.Index) -> String.Index {
        // Simplified escape sequence parser for testing
        // Full implementation would handle all SGR codes, cursor movement, etc.
        var i = string.index(after: start)

        guard i < string.endIndex, string[i] == "[" else {
            return i
        }

        i = string.index(after: i)
        var params = ""

        while i < string.endIndex {
            let char = string[i]
            if char.isNumber || char == ";" {
                params += String(char)
                i = string.index(after: i)
            } else {
                // Command character
                applyEscapeCommand(command: char, params: params)
                return i
            }
        }

        return i
    }

    private mutating func applyEscapeCommand(command: Character, params: String) {
        let parts = params.split(separator: ";").compactMap { Int($0) }

        switch command {
        case "H", "f":  // Cursor position
            if parts.count >= 2 {
                cursorLine = parts[0] - 1  // 1-indexed to 0-indexed
                cursorColumn = parts[1] - 1
            }
        case "m":  // SGR (Select Graphic Rendition)
            for code in parts {
                applyGraphicRendition(code)
            }
        default:
            break
        }
    }

    private mutating func applyGraphicRendition(_ code: Int) {
        switch code {
        case 0:  // Reset
            currentFg = .default
            currentBg = .default
            currentAttrs = CellAttributes()
        case 1:  // Bold
            currentAttrs.bold = true
        case 3:  // Italic
            currentAttrs.italic = true
        case 4:  // Underline
            currentAttrs.underline = true
        default:
            break
        }
    }
}

/// Snapshot of buffer state for comparison
struct BufferSnapshot {
    let cells: [[Cell]]
    let width: Int
    let height: Int

    init(buffer: TerminalBuffer) {
        self.width = buffer.width
        self.height = buffer.height
        self.cells = (0..<buffer.height).map { line in
            (0..<buffer.width).map { col in
                buffer.cell(at: Position(column: Extended(col), line: Extended(line))) ??
                Cell(char: " ", foregroundColor: .default, backgroundColor: .default)
            }
        }
    }
}
```

**Expected Impact**: Foundation for buffer inspection and assertions.

---

### Task 1.4: Add Escape Sequence Parser for Assertions

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/EscapeSequenceParser.swift`

**Problem**: Need to verify correct escape sequences are generated (colors, cursor movement).

**Solution**: Parser that extracts structured data from escape sequences.

**Implementation** (~200 LOC):

```swift
/// Parsed escape sequence for testing
enum ParsedSequence: Equatable {
    case cursorMove(line: Int, column: Int)
    case setForeground(color: Color)
    case setBackground(color: Color)
    case setAttribute(bold: Bool?, italic: Bool?, underline: Bool?)
    case clearScreen
    case text(String)
}

/// Parser for escape sequences in test output
struct EscapeSequenceParser {
    /// Parse string into sequence of commands
    static func parse(_ string: String) -> [ParsedSequence] {
        var result: [ParsedSequence] = []
        var i = string.startIndex

        while i < string.endIndex {
            if string[i] == "\u{1B}" {
                let (sequence, nextIndex) = parseEscapeSequence(string, from: i)
                if let seq = sequence {
                    result.append(seq)
                }
                i = nextIndex
            } else {
                // Collect text until next escape sequence
                var text = ""
                while i < string.endIndex && string[i] != "\u{1B}" {
                    text += String(string[i])
                    i = string.index(after: i)
                }
                if !text.isEmpty {
                    result.append(.text(text))
                }
            }
        }

        return result
    }

    private static func parseEscapeSequence(_ string: String, from start: String.Index) -> (ParsedSequence?, String.Index) {
        var i = string.index(after: start)

        guard i < string.endIndex, string[i] == "[" else {
            return (nil, i)
        }

        i = string.index(after: i)
        var params = ""

        while i < string.endIndex {
            let char = string[i]
            if char.isNumber || char == ";" {
                params += String(char)
                i = string.index(after: i)
            } else {
                // Command character
                let sequence = interpretSequence(command: char, params: params)
                return (sequence, string.index(after: i))
            }
        }

        return (nil, i)
    }

    private static func interpretSequence(command: Character, params: String) -> ParsedSequence? {
        let parts = params.split(separator: ";").compactMap { Int($0) }

        switch command {
        case "H", "f":  // Cursor position
            if parts.count >= 2 {
                return .cursorMove(line: parts[0], column: parts[1])
            }
        case "J":  // Clear screen
            return .clearScreen
        case "m":  // SGR
            return interpretSGR(parts)
        default:
            return nil
        }

        return nil
    }

    private static func interpretSGR(_ codes: [Int]) -> ParsedSequence? {
        // Simplified SGR interpretation
        for code in codes {
            switch code {
            case 1:
                return .setAttribute(bold: true, italic: nil, underline: nil)
            case 3:
                return .setAttribute(bold: nil, italic: true, underline: nil)
            case 4:
                return .setAttribute(bold: nil, italic: nil, underline: true)
            default:
                continue
            }
        }
        return nil
    }
}
```

**Expected Impact**: Enables deep assertions on rendering output beyond just cell content.

---

## Phase 2: Test Application Infrastructure

### Task 2.1: Create TestableApplication

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/TestableApplication.swift`

**Problem**: Current Application is tightly coupled to real stdin/stdout. Need testable version.

**Solution**: Create TestableApplication with dependency injection for terminal and input.

**Implementation** (~250 LOC):

```swift
/// Testable version of Application with injectable dependencies
public final class TestableApplication {
    private let node: Node
    private let window: Window
    private let control: Control
    private let virtualRenderer: VirtualRenderer
    private let terminal: VirtualTerminal

    private var invalidatedNodes: [Node] = []
    private var updateScheduled = false
    private var lastLayoutSize: Size?
    private var needsLayout = true

    // Expose for testing
    public var currentBuffer: TerminalBuffer {
        terminal.buffer
    }

    public init<V: View>(rootView: V, width: Int = 80, height: Int = 24) {
        // Create virtual terminal
        terminal = VirtualTerminal(width: width, height: height)

        // Build view tree
        node = Node(view: VStack(content: rootView).view)
        node.build()

        control = node.control!

        window = Window()
        window.addControl(control)
        window.firstResponder = control.firstSelectableElement
        window.firstResponder?.becomeFirstResponder()

        // Use virtual renderer instead of real renderer
        virtualRenderer = VirtualRenderer(layer: window.layer, terminal: terminal)
        window.layer.renderer = virtualRenderer

        node.application = self
        virtualRenderer.application = self

        // Initial layout and render
        updateWindowSize()
        control.layout(size: window.layer.frame.size)
        virtualRenderer.draw()
    }

    /// Simulate terminal resize
    public func resize(width: Int, height: Int) {
        terminal.buffer = TerminalBuffer(width: width, height: height)
        virtualRenderer.setCache()
        window.layer.frame.size = Size(width: Extended(width), height: Extended(height))
        needsLayout = true
        scheduleUpdate()
        processUpdate()  // Process immediately for testing
    }

    /// Inject keyboard input
    public func sendKey(_ char: Character) {
        // Simulate key press handling
        window.firstResponder?.handleEvent(char)

        // Process onKeyPress controls (with caching from Task 6.5)
        let onKeyPressControls = window.controls.flattenAndKeepOnlyOnKeyPressControl()
        for control in onKeyPressControls {
            if control.keyPress == char {
                control.action()
            }
        }

        // Process any scheduled updates immediately for testing
        if updateScheduled {
            processUpdate()
        }
    }

    /// Send multiple keys in sequence
    public func sendKeys(_ keys: String) {
        for char in keys {
            sendKey(char)
        }
    }

    /// Manually trigger update cycle (for testing async updates)
    public func processUpdate() {
        update()
    }

    /// Schedule update (same as Application)
    func scheduleUpdate() {
        guard !updateScheduled else { return }
        updateScheduled = true
        // In tests, we process updates synchronously via processUpdate()
    }

    func invalidateNode(_ node: Node) {
        if !invalidatedNodes.contains(where: { $0 === node }) {
            invalidatedNodes.append(node)
        }
        scheduleUpdate()
    }

    private func update() {
        updateScheduled = false

        // Update invalidated nodes
        for node in invalidatedNodes {
            node.update(using: node.view)
        }
        invalidatedNodes.removeAll()

        // Layout if needed
        let currentSize = window.layer.frame.size
        if needsLayout || lastLayoutSize != currentSize {
            control.layout(size: currentSize)
            lastLayoutSize = currentSize
            needsLayout = false
        }

        // Render
        virtualRenderer.update()
    }

    private func updateWindowSize() {
        window.layer.frame.size = terminal.size
        virtualRenderer.setCache()
    }

    /// Get rendered content at position (for assertions)
    public func cell(at position: Position) -> Cell? {
        terminal.cell(at: position)
    }

    /// Get rendered row as string (for assertions)
    public func row(_ line: Int) -> String {
        terminal.row(line)
    }

    /// Snapshot current buffer state
    public func snapshot() -> BufferSnapshot {
        terminal.snapshot()
    }

    /// Get all escape sequences written (for debugging)
    public var outputLog: [String] {
        terminal.outputLog
    }
}
```

**Expected Impact**: Core test infrastructure, enables all subsequent test writing.

---

### Task 2.2: Implement KeyEventSimulator

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/KeyEventSimulator.swift`

**Problem**: Need to simulate complex keyboard sequences like Ctrl+D, arrow keys, etc.

**Solution**: Helper that generates correct character sequences for special keys.

**Implementation** (~200 LOC):

```swift
/// Simulator for keyboard events in tests
public struct KeyEventSimulator {
    private let app: TestableApplication

    public init(app: TestableApplication) {
        self.app = app
    }

    // MARK: - Basic Keys

    public func press(_ char: Character) {
        app.sendKey(char)
    }

    public func type(_ string: String) {
        app.sendKeys(string)
    }

    // MARK: - Navigation Keys

    public func pressJ() {
        app.sendKey("j")
    }

    public func pressK() {
        app.sendKey("k")
    }

    public func pressH() {
        app.sendKey("h")
    }

    public func pressL() {
        app.sendKey("l")
    }

    // MARK: - Control Keys

    public func pressCtrlD() {
        // Ctrl+D is ASCII 0x04
        app.sendKey(Character(UnicodeScalar(0x04)))
    }

    public func pressCtrlU() {
        // Ctrl+U is ASCII 0x15
        app.sendKey(Character(UnicodeScalar(0x15)))
    }

    public func pressCtrlF() {
        // Ctrl+F is ASCII 0x06
        app.sendKey(Character(UnicodeScalar(0x06)))
    }

    public func pressCtrlB() {
        // Ctrl+B is ASCII 0x02
        app.sendKey(Character(UnicodeScalar(0x02)))
    }

    // MARK: - Arrow Keys

    public func pressArrowUp() {
        // Arrow up is ESC [ A
        app.sendKeys("\u{1B}[A")
    }

    public func pressArrowDown() {
        // Arrow down is ESC [ B
        app.sendKeys("\u{1B}[B")
    }

    public func pressArrowLeft() {
        // Arrow left is ESC [ D
        app.sendKeys("\u{1B}[D")
    }

    public func pressArrowRight() {
        // Arrow right is ESC [ C
        app.sendKeys("\u{1B}[C")
    }

    // MARK: - Special Keys

    public func pressEnter() {
        app.sendKey("\n")
    }

    public func pressTab() {
        app.sendKey("\t")
    }

    public func pressEscape() {
        app.sendKey("\u{1B}")
    }

    public func pressSpace() {
        app.sendKey(" ")
    }

    // MARK: - Compound Actions

    /// Hold key down (simulate key repeat)
    public func hold(_ action: () -> Void, times: Int) {
        for _ in 0..<times {
            action()
        }
    }

    /// Scroll down by count
    public func scrollDown(by count: Int) {
        hold(pressJ, times: count)
    }

    /// Scroll up by count
    public func scrollUp(by count: Int) {
        hold(pressK, times: count)
    }

    /// Half-page down
    public func halfPageDown() {
        pressCtrlD()
    }

    /// Half-page up
    public func halfPageUp() {
        pressCtrlU()
    }
}
```

**Expected Impact**: Clean, readable test code for keyboard interactions.

---

### Task 2.3: Build RenderCapture System

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/RenderCapture.swift`

**Problem**: Need to capture and compare buffer states over time for debugging.

**Solution**: Snapshot system with diff capabilities.

**Implementation** (~150 LOC):

```swift
/// Captures and compares buffer states
public final class RenderCapture {
    private var snapshots: [(label: String, snapshot: BufferSnapshot)] = []
    private let app: TestableApplication

    public init(app: TestableApplication) {
        self.app = app
    }

    /// Capture current state with label
    public func capture(_ label: String) {
        let snapshot = app.snapshot()
        snapshots.append((label, snapshot))
    }

    /// Get snapshot by label
    public func snapshot(labeled: String) -> BufferSnapshot? {
        snapshots.first { $0.label == labeled }?.snapshot
    }

    /// Get latest snapshot
    public var latest: BufferSnapshot? {
        snapshots.last?.snapshot
    }

    /// Clear all captures
    public func clear() {
        snapshots.removeAll()
    }

    /// Diff two snapshots
    public func diff(from: String, to: String) -> BufferDiff? {
        guard let fromSnapshot = snapshot(labeled: from),
              let toSnapshot = snapshot(labeled: to) else {
            return nil
        }

        return BufferDiff(from: fromSnapshot, to: toSnapshot)
    }

    /// Print all snapshots for debugging
    public func printAll() {
        for (label, snapshot) in snapshots {
            print("=== \(label) ===")
            printSnapshot(snapshot)
            print("")
        }
    }

    private func printSnapshot(_ snapshot: BufferSnapshot) {
        for line in 0..<snapshot.height {
            var row = ""
            for col in 0..<snapshot.width {
                if col < snapshot.cells[line].count {
                    row += String(snapshot.cells[line][col].char)
                }
            }
            print(row)
        }
    }
}

/// Difference between two buffer snapshots
public struct BufferDiff {
    let changedCells: [(position: Position, from: Cell, to: Cell)]

    init(from: BufferSnapshot, to: BufferSnapshot) {
        var changes: [(Position, Cell, Cell)] = []

        let height = min(from.height, to.height)
        let width = min(from.width, to.width)

        for line in 0..<height {
            for col in 0..<width {
                let fromCell = from.cells[line][col]
                let toCell = to.cells[line][col]

                if fromCell != toCell {
                    changes.append((
                        Position(column: Extended(col), line: Extended(line)),
                        fromCell,
                        toCell
                    ))
                }
            }
        }

        self.changedCells = changes
    }

    public var isEmpty: Bool {
        changedCells.isEmpty
    }

    public func printDiff() {
        print("Buffer Diff: \(changedCells.count) changes")
        for (pos, from, to) in changedCells.prefix(20) {  // Limit output
            print("  [\(pos.line),\(pos.column)]: '\(from.char)' → '\(to.char)'")
        }
        if changedCells.count > 20 {
            print("  ... and \(changedCells.count - 20) more")
        }
    }
}
```

**Expected Impact**: Powerful debugging tool for understanding rendering changes.

---

### Task 2.4: Add Async/Await Test Helpers

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/AsyncHelpers.swift`

**Problem**: Some updates may be async, need helpers for waiting and assertions.

**Solution**: Helpers for async test code.

**Implementation** (~100 LOC):

```swift
/// Helpers for async testing
public struct AsyncTestHelpers {
    /// Wait for condition with timeout
    public static func wait(
        for condition: () -> Bool,
        timeout: TimeInterval = 1.0,
        message: String = "Condition not met"
    ) throws {
        let deadline = Date().addingTimeInterval(timeout)

        while !condition() {
            if Date() > deadline {
                throw TestError.timeout(message)
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.01))
        }
    }

    /// Wait for buffer to contain text
    public static func waitForText(
        in app: TestableApplication,
        at line: Int,
        contains text: String,
        timeout: TimeInterval = 1.0
    ) throws {
        try wait(
            for: { app.row(line).contains(text) },
            timeout: timeout,
            message: "Text '\(text)' not found in line \(line)"
        )
    }

    /// Wait for cell to have specific character
    public static func waitForCell(
        in app: TestableApplication,
        at position: Position,
        equals char: Character,
        timeout: TimeInterval = 1.0
    ) throws {
        try wait(
            for: { app.cell(at: position)?.char == char },
            timeout: timeout,
            message: "Cell at \(position) does not equal '\(char)'"
        )
    }
}

public enum TestError: Error {
    case timeout(String)
}
```

**Expected Impact**: Enables robust async testing when needed.

---

## Phase 3: Assertion Framework

### Task 3.1: Create TerminalMatcher DSL

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/TerminalMatcher.swift`

**Problem**: Need readable, expressive assertions for terminal content.

**Solution**: Fluent DSL for terminal assertions.

**Implementation** (~250 LOC):

```swift
/// Fluent assertion DSL for terminal testing
public struct TerminalMatcher {
    private let app: TestableApplication

    public init(app: TestableApplication) {
        self.app = app
    }

    // MARK: - Line Assertions

    @discardableResult
    public func line(_ line: Int) -> LineMatcher {
        LineMatcher(app: app, line: line)
    }

    @discardableResult
    public func assertLine(_ line: Int, contains text: String, file: StaticString = #file, lineNum: UInt = #line) -> Self {
        let actual = app.row(line)
        assert(actual.contains(text),
               "Line \(line) does not contain '\(text)'\nActual: '\(actual)'",
               file: file, line: lineNum)
        return self
    }

    @discardableResult
    public func assertLine(_ line: Int, equals text: String, file: StaticString = #file, lineNum: UInt = #line) -> Self {
        let actual = app.row(line).trimmingCharacters(in: .whitespaces)
        let expected = text.trimmingCharacters(in: .whitespaces)
        assert(actual == expected,
               "Line \(line) does not equal expected\nExpected: '\(expected)'\nActual:   '\(actual)'",
               file: file, line: lineNum)
        return self
    }

    @discardableResult
    public func assertLine(_ line: Int, startsWith prefix: String, file: StaticString = #file, lineNum: UInt = #line) -> Self {
        let actual = app.row(line)
        assert(actual.hasPrefix(prefix),
               "Line \(line) does not start with '\(prefix)'\nActual: '\(actual)'",
               file: file, line: lineNum)
        return self
    }

    // MARK: - Cell Assertions

    @discardableResult
    public func assertCell(at position: Position, equals char: Character, file: StaticString = #file, lineNum: UInt = #line) -> Self {
        guard let cell = app.cell(at: position) else {
            assertionFailure("No cell at position \(position)", file: file, line: lineNum)
            return self
        }

        assert(cell.char == char,
               "Cell at \(position) does not equal '\(char)'\nActual: '\(cell.char)'",
               file: file, line: lineNum)
        return self
    }

    @discardableResult
    public func assertCell(at position: Position, hasColor color: Color, file: StaticString = #file, lineNum: UInt = #line) -> Self {
        guard let cell = app.cell(at: position) else {
            assertionFailure("No cell at position \(position)", file: file, line: lineNum)
            return self
        }

        assert(cell.foregroundColor == color,
               "Cell at \(position) does not have color \(color)\nActual: \(cell.foregroundColor)",
               file: file, line: lineNum)
        return self
    }

    // MARK: - Range Assertions

    @discardableResult
    public func assertRange(lines: ClosedRange<Int>, contains text: String, file: StaticString = #file, lineNum: UInt = #line) -> Self {
        let combined = lines.map { app.row($0) }.joined(separator: "\n")
        assert(combined.contains(text),
               "Lines \(lines) do not contain '\(text)'",
               file: file, line: lineNum)
        return self
    }

    // MARK: - List Item Assertions (Scrolling-specific)

    @discardableResult
    public func assertVisibleItems(_ items: [String], startingAt line: Int, file: StaticString = #file, lineNum: UInt = #line) -> Self {
        for (index, item) in items.enumerated() {
            let actual = app.row(line + index)
            assert(actual.contains(item),
                   "Line \(line + index) does not contain '\(item)'\nActual: '\(actual)'",
                   file: file, line: lineNum)
        }
        return self
    }

    private func assert(_ condition: Bool, _ message: String, file: StaticString, line: UInt) {
        if !condition {
            XCTFail(message, file: file, line: line)
        }
    }
}

/// Line-specific matcher
public struct LineMatcher {
    private let app: TestableApplication
    private let line: Int

    init(app: TestableApplication, line: Int) {
        self.app = app
        self.line = line
    }

    @discardableResult
    public func contains(_ text: String, file: StaticString = #file, lineNum: UInt = #line) -> Self {
        let actual = app.row(line)
        XCTAssertTrue(actual.contains(text),
                     "Line \(line) does not contain '\(text)'\nActual: '\(actual)'",
                     file: file, line: lineNum)
        return self
    }

    @discardableResult
    public func equals(_ text: String, file: StaticString = #file, lineNum: UInt = #line) -> Self {
        let actual = app.row(line).trimmingCharacters(in: .whitespaces)
        let expected = text.trimmingCharacters(in: .whitespaces)
        XCTAssertEqual(actual, expected, "Line \(line) mismatch", file: file, line: lineNum)
        return self
    }

    @discardableResult
    public func isEmpty(file: StaticString = #file, lineNum: UInt = #line) -> Self {
        let actual = app.row(line).trimmingCharacters(in: .whitespaces)
        XCTAssertTrue(actual.isEmpty, "Line \(line) is not empty: '\(actual)'", file: file, line: lineNum)
        return self
    }
}
```

**Expected Impact**: Tests become highly readable, easy to write and maintain.

**✅ COMPLETED** - Implemented with 405 LOC
- **Files Created**:
  - `TerminalMatcher.swift` (405 LOC): Complete fluent DSL implementation
  - `TerminalMatcherTests.swift` (479 LOC): 36 comprehensive tests
  - `RenderDebugTests.swift` (79 LOC): Debugging helpers
- **Critical Fixes**:
  1. Fixed escape sequence parsing in TerminalBuffer.swift (private sequences like `\e[?25l`)
  2. Fixed virtual terminal rendering in TestableApplication.swift (invalidation + update)
  3. Updated 4 test files to work with corrected rendering
- **Test Results**: All 351 tests passing (36 TerminalMatcher tests, 1 AsyncHelper test disabled)
- **API Features**:
  - Line assertions: contains, equals, startsWith, endsWith, isEmpty
  - Cell assertions: equals, hasColor, hasBackgroundColor, exists
  - Range assertions: contains, isEmpty
  - List assertions: assertVisibleItems, assertItemsInOrder
  - Buffer state: assertBufferSize, assertBufferChanged, assertBufferMatches
  - LineMatcher: Fluent chaining for line-specific assertions

---

### Task 3.2: Implement Cell-by-Cell Diff Visualization

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/BufferVisualizer.swift`

**Problem**: When tests fail, need to see exactly what's different between expected and actual.

**Solution**: Visual diff tool that highlights differences.

**Implementation** (~200 LOC):

```swift
/// Visualize buffer differences for debugging
public struct BufferVisualizer {
    /// Print buffer with highlighted differences
    public static func visualizeDiff(from: BufferSnapshot, to: BufferSnapshot) {
        print("=== BUFFER DIFF ===")

        let height = max(from.height, to.height)
        let width = max(from.width, to.width)

        for line in 0..<height {
            var fromRow = ""
            var toRow = ""
            var diffMarkers = ""

            for col in 0..<width {
                let fromChar = from.cells[safe: line]?[safe: col]?.char ?? " "
                let toChar = to.cells[safe: line]?[safe: col]?.char ?? " "

                fromRow += String(fromChar)
                toRow += String(toChar)

                if fromChar != toChar {
                    diffMarkers += "^"
                } else {
                    diffMarkers += " "
                }
            }

            if !diffMarkers.trimmingCharacters(in: .whitespaces).isEmpty {
                print("\(String(format: "%3d", line))│ FROM: \(fromRow)")
                print("   │ TO:   \(toRow)")
                print("   │ DIFF: \(diffMarkers)")
                print("   │")
            }
        }
    }

    /// Print buffer with visible control characters
    public static func visualizeRaw(snapshot: BufferSnapshot) {
        print("=== RAW BUFFER ===")
        for line in 0..<snapshot.height {
            var row = String(format: "%3d│ ", line)
            for col in 0..<snapshot.width {
                if let cell = snapshot.cells[safe: line]?[safe: col] {
                    let char = cell.char
                    if char == " " {
                        row += "·"
                    } else if char.isWhitespace {
                        row += "░"
                    } else {
                        row += String(char)
                    }
                }
            }
            print(row)
        }
    }

    /// Visualize with color attributes
    public static func visualizeWithColors(app: TestableApplication, lines: Range<Int>) {
        print("=== BUFFER WITH COLORS ===")
        for line in lines {
            let row = app.row(line)
            print("\(String(format: "%3d", line))│ \(row)")

            // Show color info for first few cells
            var colorInfo = "   │ "
            for col in 0..<min(10, app.currentBuffer.width) {
                if let cell = app.cell(at: Position(column: Extended(col), line: Extended(line))) {
                    let fg = colorCode(cell.foregroundColor)
                    let bg = colorCode(cell.backgroundColor ?? .default)
                    colorInfo += "[\(fg):\(bg)]"
                }
            }
            print(colorInfo)
        }
    }

    private static func colorCode(_ color: Color) -> String {
        switch color {
        case .default: return "def"
        case .black: return "blk"
        case .red: return "red"
        case .green: return "grn"
        case .yellow: return "yel"
        case .blue: return "blu"
        case .magenta: return "mag"
        case .cyan: return "cyn"
        case .white: return "wht"
        default: return "???"
        }
    }
}

extension Array {
    subscript(safe index: Int) -> Element? {
        return indices.contains(index) ? self[index] : nil
    }
}
```

**Expected Impact**: Dramatically speeds up debugging when tests fail.

**✅ COMPLETED** - Implemented with 240 LOC
- **Files Created**:
  - `BufferVisualizer.swift` (240 LOC): Complete diff visualization implementation
  - `BufferVisualizerTests.swift` (321 LOC): 21 comprehensive tests
- **Implementation Details**:
  1. `BufferDiff` struct: Tracks changed cells between two snapshots
  2. `visualizeDiff()`: Side-by-side diff with change markers
  3. `visualizeRaw()`: Raw buffer view with visible whitespace
  4. `visualizeWithColors()`: Shows color attributes for debugging
  5. `visualize()`: Basic buffer visualization with line numbers
  6. `visualizeSideBySide()`: Horizontal comparison view
  7. `visualizeChangesWithContext()`: Shows changes with surrounding context
- **Test Results**: All 374 tests passing
- **API Features**:
  - Cell-by-cell comparison with position tracking
  - Multiple visualization modes for different debugging needs
  - Context-aware change display
  - Performance-optimized for large buffers
  - Safe array subscripting to handle edge cases

---

### Task 3.3: Add Scrolling-Specific Assertion Helpers

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/ScrollingMatchers.swift`

**Problem**: Need assertions specific to scrolling behavior.

**Solution**: Specialized matchers for scroll state.

**Implementation** (~150 LOC):

```swift
/// Assertions specific to scrolling lists
public struct ScrollingMatcher {
    private let app: TestableApplication

    public init(app: TestableApplication) {
        self.app = app
    }

    /// Assert that specific items are visible in viewport
    @discardableResult
    public func assertVisibleItems(
        _ items: [String],
        startLine: Int,
        file: StaticString = #file,
        line: UInt = #line
    ) -> Self {
        for (index, item) in items.enumerated() {
            let lineNum = startLine + index
            let actual = app.row(lineNum)
            XCTAssertTrue(
                actual.contains(item),
                "Line \(lineNum) should contain '\(item)'\nActual: '\(actual)'",
                file: file,
                line: line
            )
        }
        return self
    }

    /// Assert items are NOT visible
    @discardableResult
    public func assertNotVisible(
        _ items: [String],
        in lines: ClosedRange<Int>,
        file: StaticString = #file,
        line: UInt = #line
    ) -> Self {
        let content = lines.map { app.row($0) }.joined(separator: "\n")
        for item in items {
            XCTAssertFalse(
                content.contains(item),
                "Item '\(item)' should NOT be visible in lines \(lines)",
                file: file,
                line: line
            )
        }
        return self
    }

    /// Assert selection indicator is at specific line
    @discardableResult
    public func assertSelection(
        at line: Int,
        indicator: String = ">",
        file: StaticString = #file,
        lineNum: UInt = #line
    ) -> Self {
        let actual = app.row(line)
        XCTAssertTrue(
            actual.hasPrefix(indicator) || actual.contains(indicator),
            "Line \(line) should have selection indicator '\(indicator)'\nActual: '\(actual)'",
            file: file,
            line: lineNum
        )
        return self
    }

    /// Assert viewport shows items in correct order
    @discardableResult
    public func assertSequence(
        _ items: [String],
        startLine: Int,
        file: StaticString = #file,
        line: UInt = #line
    ) -> Self {
        for (index, item) in items.enumerated() {
            let lineNum = startLine + index
            let actual = app.row(lineNum)

            // Check this item is present
            XCTAssertTrue(
                actual.contains(item),
                "Line \(lineNum) should contain '\(item)' in sequence\nActual: '\(actual)'",
                file: file,
                line: line
            )

            // Check next item is NOT on this line (unless it's the last item)
            if index < items.count - 1 {
                let nextItem = items[index + 1]
                XCTAssertFalse(
                    actual.contains(nextItem),
                    "Line \(lineNum) should not contain next item '\(nextItem)'",
                    file: file,
                    line: line
                )
            }
        }
        return self
    }
}
```

**Expected Impact**: Makes scrolling tests extremely clear and maintainable.

---

### Task 3.4: Build Render Timeline Debugger

**File**: `SwiftTUI/Sources/SwiftTUI/Testing/RenderTimeline.swift`

**Problem**: Need to see how buffer evolves over multiple render cycles.

**Solution**: Timeline that captures every render step.

**Implementation** (~200 LOC):

```swift
/// Captures render timeline for debugging
public final class RenderTimeline {
    private var entries: [TimelineEntry] = []
    private let app: TestableApplication

    public init(app: TestableApplication) {
        self.app = app
    }

    /// Record an action and capture state
    public func record(action: String, execute: () -> Void) {
        let beforeSnapshot = app.snapshot()

        execute()

        let afterSnapshot = app.snapshot()
        let diff = BufferDiff(from: beforeSnapshot, to: afterSnapshot)

        entries.append(TimelineEntry(
            action: action,
            before: beforeSnapshot,
            after: afterSnapshot,
            diff: diff,
            timestamp: Date()
        ))
    }

    /// Print full timeline
    public func print() {
        Swift.print("=== RENDER TIMELINE ===")
        Swift.print("\(entries.count) steps recorded\n")

        for (index, entry) in entries.enumerated() {
            Swift.print("Step \(index + 1): \(entry.action)")
            Swift.print("  Changes: \(entry.diff.changedCells.count) cells")

            if !entry.diff.isEmpty {
                Swift.print("  First few changes:")
                for (pos, from, to) in entry.diff.changedCells.prefix(5) {
                    Swift.print("    [\(pos.line),\(pos.column)]: '\(from.char)' → '\(to.char)'")
                }
            }
            Swift.print("")
        }
    }

    /// Print specific step in detail
    public func printStep(_ index: Int) {
        guard index < entries.count else { return }
        let entry = entries[index]

        Swift.print("=== STEP \(index + 1): \(entry.action) ===")
        Swift.print("\nBEFORE:")
        printSnapshot(entry.before)

        Swift.print("\nAFTER:")
        printSnapshot(entry.after)

        if !entry.diff.isEmpty {
            Swift.print("\nCHANGES:")
            entry.diff.printDiff()
        }
    }

    /// Get entry at index
    public func entry(_ index: Int) -> TimelineEntry? {
        guard index < entries.count else { return nil }
        return entries[index]
    }

    /// Compare two steps
    public func compare(from: Int, to: Int) {
        guard from < entries.count, to < entries.count else { return }

        let fromSnapshot = entries[from].after
        let toSnapshot = entries[to].after

        BufferVisualizer.visualizeDiff(from: fromSnapshot, to: toSnapshot)
    }

    private func printSnapshot(_ snapshot: BufferSnapshot) {
        for line in 0..<snapshot.height {
            var row = String(format: "%3d│ ", line)
            for col in 0..<snapshot.width {
                if col < snapshot.cells[line].count {
                    row += String(snapshot.cells[line][col].char)
                }
            }
            Swift.print(row)
        }
    }
}

public struct TimelineEntry {
    public let action: String
    public let before: BufferSnapshot
    public let after: BufferSnapshot
    public let diff: BufferDiff
    public let timestamp: Date
}
```

**Expected Impact**: Powerful tool for understanding complex multi-step rendering bugs.

---

## Phase 4: Scrolling Test Suite

### Task 4.1: Test Basic j/k Navigation

**File**: `SwiftTUI/Tests/SwiftTUITests/ScrollingNavigationTests.swift`

**Problem**: Need to verify basic scrolling works correctly.

**Solution**: Comprehensive tests for j/k navigation.

**Implementation** (~150 LOC):

```swift
import XCTest
@testable import SwiftTUI

final class ScrollingNavigationTests: XCTestCase {

    func testBasicJNavigationMoves DownOneItem() {
        // Setup: List with 10 items, terminal height 10
        let items = (1...10).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 40,
            height: 10
        )

        let matcher = TerminalMatcher(app: app)
        let keys = KeyEventSimulator(app: app)

        // Initial state: Item 1 selected
        matcher.assertLine(0, contains: "> Item 1")

        // Press j
        keys.pressJ()

        // Assert: Item 2 now selected
        matcher.assertLine(1, contains: "> Item 2")
        matcher.assertLine(0, contains: "  Item 1")  // No longer selected
    }

    func testKNavigationMovesUpOneItem() {
        let items = (1...10).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 40,
            height: 10
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)

        // Move to item 3 first
        keys.pressJ()
        keys.pressJ()

        matcher.assertLine(2, contains: "> Item 3")

        // Press k
        keys.pressK()

        // Assert: Back to item 2
        matcher.assertLine(1, contains: "> Item 2")
    }

    func testNavigationAtTopBoundary() {
        let items = (1...10).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 40,
            height: 10
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)

        // At top, pressing k should not move
        matcher.assertLine(0, contains: "> Item 1")

        keys.pressK()

        matcher.assertLine(0, contains: "> Item 1")  // Still at top
    }

    func testNavigationAtBottomBoundary() {
        let items = (1...5).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 40,
            height: 10
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)

        // Move to bottom
        keys.scrollDown(by: 4)

        matcher.assertLine(4, contains: "> Item 5")

        // Press j should not move past bottom
        keys.pressJ()

        matcher.assertLine(4, contains: "> Item 5")  // Still at bottom
    }

    func testHoldingJScrollsSmoo thly() {
        let items = (1...20).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 40,
            height: 8
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)
        let capture = RenderCapture(app: app)

        // Hold j 10 times
        for i in 1...10 {
            keys.pressJ()
            capture.capture("After j press \(i)")
        }

        // Should now be at item 11
        matcher.assertLine(7, contains: "> Item 11")  // Bottom of viewport

        // Verify no crashes or render glitches
        XCTAssertNotNil(app.cell(at: Position(column: 0, line: 0)))
    }

    // Helper to create test list view
    private func makeTestList(items: [String]) -> some View {
        TestListView(items: items)
    }
}

// Test view that mimics TUIList behavior
struct TestListView: View {
    let items: [String]
    @State private var selectedIndex = 0
    @State private var scrollOffset = 0

    var body: some View {
        VStack(spacing: 0) {
            let visibleStart = scrollOffset
            let visibleEnd = min(scrollOffset + 8, items.count)

            if visibleStart < items.count && visibleEnd > visibleStart {
                ForEach(visibleStart..<visibleEnd, id: \.self) { index in
                    Text("\(index == selectedIndex ? ">" : " ") \(items[index])")
                }
            }
        }
        .onKeyPress("j") {
            if selectedIndex < items.count - 1 {
                selectedIndex += 1
                if selectedIndex >= scrollOffset + 8 {
                    scrollOffset += 1
                }
            }
        }
        .onKeyPress("k") {
            if selectedIndex > 0 {
                selectedIndex -= 1
                if selectedIndex < scrollOffset {
                    scrollOffset -= 1
                }
            }
        }
    }
}
```

**Expected Impact**: Core scrolling functionality verified to work correctly.

---

### Task 4.2: Test Ctrl+D/Ctrl+U Half-Page Scrolling

**File**: `SwiftTUI/Tests/SwiftTUITests/HalfPageScrollTests.swift`

**Problem**: The reported bug is specifically with Ctrl+D scrolling.

**Solution**: Exhaustive tests for half-page scrolling.

**Implementation** (~200 LOC):

```swift
import XCTest
@testable import SwiftTUI

final class HalfPageScrollTests: XCTestCase {

    func testCtrlDScrollsHalfPageDown() {
        let items = (1...50).map { "Song \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 80,
            height: 24
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = ScrollingMatcher(app: app)
        let timeline = RenderTimeline(app: app)

        // Initial state
        timeline.record(action: "Initial render") {}

        matcher.assertVisibleItems(
            ["Song 1", "Song 2", "Song 3"],
            startLine: 0
        )

        // Press Ctrl+D (half page = 12 lines)
        timeline.record(action: "Ctrl+D") {
            keys.pressCtrlD()
        }

        // Should jump to song ~13 (depending on viewport calculation)
        matcher.assertVisibleItems(
            ["Song 13", "Song 14", "Song 15"],
            startLine: 12
        )

        // Verify old items not visible
        matcher.assertNotVisible(
            ["Song 1", "Song 2", "Song 3"],
            in: 0...5
        )

        // Debug output if test fails
        if XCTCurrentContext.runActivity(named: "Debug Timeline").isFailed {
            timeline.print()
        }
    }

    func testCtrlUScrollsHalfPageUp() {
        let items = (1...50).map { "Song \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 80,
            height: 24
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = ScrollingMatcher(app: app)

        // Scroll down first
        keys.pressCtrlD()
        keys.pressCtrlD()

        // Verify we're at song ~25
        matcher.assertVisibleItems(["Song 25", "Song 26"], startLine: 24)

        // Press Ctrl+U
        keys.pressCtrlU()

        // Should be back to song ~13
        matcher.assertVisibleItems(["Song 13", "Song 14"], startLine: 12)
    }

    func testRepeatedCtrlDDoesNotCrash() {
        let items = (1...100).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 80,
            height: 20
        )

        let keys = KeyEventSimulator(app: app)
        let capture = RenderCapture(app: app)

        // Hammer Ctrl+D 20 times
        for i in 1...20 {
            capture.capture("Before Ctrl+D \(i)")
            keys.pressCtrlD()
            capture.capture("After Ctrl+D \(i)")
        }

        // Should reach near end without crash
        let finalRow = app.row(19)
        XCTAssertFalse(finalRow.isEmpty, "Buffer should have content")

        // Verify no Range crashes occurred
        XCTAssertNotNil(app.cell(at: Position(column: 0, line: 0)))
    }

    func testCtrlDWithZeroSizedRectsDoesNotCrash() {
        // This specifically tests the bug we fixed
        let items = (1...50).map { "Song \($0): \($0 % 2 == 0 ? "Even" : "Odd")" }
        let app = TestableApplication(
            rootView: TestListViewWithEmptyRows(items: items),
            width: 60,
            height: 20
        )

        let keys = KeyEventSimulator(app: app)

        // This should trigger zero-sized rect invalidation
        keys.pressCtrlD()

        // Should not crash
        XCTAssertNotNil(app.cell(at: Position(column: 0, line: 0)))

        // Verify content updated
        let row = app.row(10)
        XCTAssertFalse(row.isEmpty)
    }

    func testScrollingMaintainsForEachIdentity() {
        // Test that ForEach correctly updates when scrollOffset changes
        let items = (1...50).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 80,
            height: 10
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)

        // Initial: should show items 1-10
        matcher.assertLine(0, contains: "Item 1")
        matcher.assertLine(9, contains: "Item 10")

        // Ctrl+D: should show items 6-15
        keys.pressCtrlD()

        // THIS IS THE KEY TEST: verify ForEach rebuilt with new range
        matcher.assertLine(0, contains: "Item 6")
        matcher.assertLine(9, contains: "Item 15")

        // Verify old items truly gone
        for line in 0..<10 {
            let row = app.row(line)
            XCTAssertFalse(row.contains("Item 1"), "Item 1 should not be visible at line \(line)")
            XCTAssertFalse(row.contains("Item 2"), "Item 2 should not be visible at line \(line)")
        }
    }

    private func makeTestList(items: [String]) -> some View {
        TestScrollingListView(items: items)
    }
}

struct TestScrollingListView: View {
    let items: [String]
    @State private var selectedIndex = 0
    @State private var scrollOffset = 0

    var body: some View {
        VStack(spacing: 0) {
            let visibleHeight = 20
            let visibleStart = min(scrollOffset, items.count)
            let visibleEnd = min(scrollOffset + visibleHeight, items.count)

            if visibleStart < visibleEnd {
                ForEach(visibleStart..<visibleEnd, id: \.self) { index in
                    Text("\(index == selectedIndex ? ">" : " ") \(items[index])")
                }
            }
        }
        .onKeyPress(Character(UnicodeScalar(0x04))) {  // Ctrl+D
            let pageSize = 10
            selectedIndex = min(selectedIndex + pageSize, items.count - 1)
            if selectedIndex >= scrollOffset + 20 {
                scrollOffset = max(0, min(selectedIndex - 19, items.count - 20))
            }
        }
    }
}
```

**Expected Impact**: Reproduces and verifies fix for the exact bug reported.

---

### Task 4.3: Test Viewport Boundaries

**File**: `SwiftTUI/Tests/SwiftTUITests/ViewportBoundaryTests.swift`

**Problem**: Edge cases at viewport boundaries often cause bugs.

**Solution**: Comprehensive boundary testing.

**Implementation** (~200 LOC):

```swift
import XCTest
@testable import SwiftTUI

final class ViewportBoundaryTests: XCTestCase {

    func testViewportWithExactlyEnoughItems() {
        // 10 items, 10 line viewport - perfect fit
        let items = (1...10).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 40,
            height: 10
        )

        let matcher = TerminalMatcher(app: app)

        // All items should be visible
        for i in 1...10 {
            matcher.assertLine(i - 1, contains: "Item \(i)")
        }
    }

    func testViewportWithFewerItemsThanHeight() {
        // 5 items, 10 line viewport
        let items = (1...5).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 40,
            height: 10
        )

        let matcher = TerminalMatcher(app: app)

        // First 5 lines have items
        for i in 1...5 {
            matcher.assertLine(i - 1, contains: "Item \(i)")
        }

        // Remaining lines should be empty or have spacer
        matcher.assertLine(6, equals: "")
    }

    func testViewportWithOneItem() {
        // Edge case: single item
        let items = ["Only Item"]
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 40,
            height: 10
        )

        let matcher = TerminalMatcher(app: app)

        matcher.assertLine(0, contains: "Only Item")

        // No crash when trying to scroll
        let keys = KeyEventSimulator(app: app)
        keys.pressJ()
        keys.pressK()
        keys.pressCtrlD()

        // Still showing same item
        matcher.assertLine(0, contains: "Only Item")
    }

    func testEmptyList() {
        // Edge case: no items
        let items: [String] = []
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 40,
            height: 10
        )

        let matcher = TerminalMatcher(app: app)

        // Should show empty state or blank
        let row = app.row(0)
        XCTAssertTrue(row.isEmpty || row.contains("empty"))

        // No crash when trying to scroll
        let keys = KeyEventSimulator(app: app)
        keys.pressJ()
        keys.pressCtrlD()

        XCTAssertNotNil(app.cell(at: Position(column: 0, line: 0)))
    }

    func testScrollingNearEndBoundary() {
        // 25 items, 10 line viewport
        let items = (1...25).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 40,
            height: 10
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)

        // Scroll to near end
        keys.scrollDown(by: 23)

        // Should show last items without crash
        matcher.assertLine(9, contains: "Item 25")

        // Try to scroll past end
        keys.pressJ()
        keys.pressCtrlD()

        // Should stay at end
        matcher.assertLine(9, contains: "Item 25")
    }

    private func makeTestList(items: [String]) -> some View {
        TestListView(items: items)
    }
}
```

**Expected Impact**: Ensures robustness at all edge cases.

---

### Task 4.4: Test Large Lists (100+ items)

**File**: `SwiftTUI/Tests/SwiftTUITests/LargeListTests.swift`

**Problem**: Performance and correctness with large datasets.

**Solution**: Tests with realistic large lists.

**Implementation** (~150 LOC):

```swift
import XCTest
@testable import SwiftTUI

final class LargeListTests: XCTestCase {

    func testScrollingThrough100Items() {
        let items = (1...100).map { "Item \(String(format: "%03d", $0))" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 80,
            height: 20
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)

        // Scroll to middle
        for _ in 0..<50 {
            keys.pressJ()
        }

        matcher.assertLine(19, contains: "Item 051")

        // Scroll to end
        for _ in 0..<49 {
            keys.pressJ()
        }

        matcher.assertLine(19, contains: "Item 100")
    }

    func testRapidScrollingPerformance() {
        let items = (1...200).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 80,
            height: 24
        )

        let keys = KeyEventSimulator(app: app)

        measure {
            // Rapid scrolling
            for _ in 0..<100 {
                keys.pressJ()
            }
        }

        // Should complete without timeout
        XCTAssertNotNil(app.cell(at: Position(column: 0, line: 0)))
    }

    func testVirtualizationOnlyRendersVisibleItems() {
        // This test verifies that only visible items are in the buffer
        let items = (1...1000).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: makeTestList(items: items),
            width: 80,
            height: 20
        )

        let matcher = TerminalMatcher(app: app)

        // Should show items 1-20
        matcher.assertLine(0, contains: "Item 1")
        matcher.assertLine(19, contains: "Item 20")

        // Items beyond viewport should NOT be rendered
        for line in 0..<20 {
            let row = app.row(line)
            XCTAssertFalse(row.contains("Item 100"), "Item 100 should not be rendered yet")
        }
    }

    private func makeTestList(items: [String]) -> some View {
        TestListView(items: items)
    }
}
```

**Expected Impact**: Verifies performance and virtualization work correctly.

---

### Task 4.5: Test ForEach Identity-Based Diffing

**File**: `SwiftTUI/Tests/SwiftTUITests/ForEachDiffingTests.swift`

**Problem**: ForEach's identity-based diffing causes rendering bugs when indices overlap.

**Solution**: Targeted tests for ForEach behavior.

**Implementation** (~250 LOC):

```swift
import XCTest
@testable import SwiftTUI

final class ForEachDiffingTests: XCTestCase {

    func testForEachRebuildsWithNewRange() {
        // Core issue: ForEach sees overlapping indices and updates instead of rebuilding
        let items = (0...49).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: TestForEachView(items: items),
            width: 80,
            height: 10
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)
        let capture = RenderCapture(app: app)

        // Initial: ForEach(0..<10)
        capture.capture("Initial")
        matcher.assertLine(0, contains: "Item 0")
        matcher.assertLine(9, contains: "Item 9")

        // Scroll: ForEach(5..<15) - overlaps with previous range!
        capture.capture("Before scroll")
        keys.sendKeys("SCROLL")  // Custom command that changes offset
        capture.capture("After scroll")

        // CRITICAL: Should show NEW items, not updated old items
        matcher.assertLine(0, contains: "Item 5")
        matcher.assertLine(9, contains: "Item 14")

        // Debug if fails
        if XCTCurrentContext.runActivity(named: "Debug").isFailed {
            capture.printAll()
        }
    }

    func testForEachWithExplicitIDsRebuilds() {
        // Test with explicit IDs that force rebuild
        struct ItemWithID: Identifiable {
            let id: String
            let text: String
        }

        let items = (0...49).map { ItemWithID(id: "item-\($0)", text: "Item \($0)") }
        let app = TestableApplication(
            rootView: TestForEachIDView(items: items),
            width: 80,
            height: 10
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)

        matcher.assertLine(0, contains: "Item 0")

        keys.sendKeys("SCROLL")

        // With explicit IDs, should correctly rebuild
        matcher.assertLine(0, contains: "Item 5")
    }

    func testCompositeIDsForceCorrectRendering() {
        // Solution: use composite ID that includes scroll offset
        let items = (0...49).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: TestForEachCompositeIDView(items: items),
            width: 80,
            height: 10
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)

        matcher.assertLine(0, contains: "Item 0")

        keys.sendKeys("SCROLL")

        // Composite IDs should force rebuild
        matcher.assertLine(0, contains: "Item 5")
    }

    func testArrayWrapperForcesRebuild() {
        // Alternative solution: wrap range in array
        let items = (0...49).map { "Item \($0)" }
        let app = TestableApplication(
            rootView: TestForEachArrayView(items: items),
            width: 80,
            height: 10
        )

        let keys = KeyEventSimulator(app: app)
        let matcher = TerminalMatcher(app: app)

        matcher.assertLine(0, contains: "Item 0")

        keys.sendKeys("SCROLL")

        // Array wrapper should force rebuild
        matcher.assertLine(0, contains: "Item 5")
    }
}

// Test views with different ForEach strategies

struct TestForEachView: View {
    let items: [String]
    @State private var offset = 0

    var body: some View {
        VStack(spacing: 0) {
            let start = offset
            let end = min(offset + 10, items.count)

            ForEach(start..<end, id: \.self) { index in
                Text(items[index])
            }
        }
        .onKeyPress("S") { offset = 5 }
    }
}

struct TestForEachCompositeIDView: View {
    let items: [String]
    @State private var offset = 0

    var body: some View {
        VStack(spacing: 0) {
            let start = offset
            let end = min(offset + 10, items.count)

            ForEach(start..<end, id: \.self) { index in
                Text(items[index])
                    .id("\(offset)-\(index)")  // Composite ID!
            }
        }
        .onKeyPress("S") { offset = 5 }
    }
}

struct TestForEachArrayView: View {
    let items: [String]
    @State private var offset = 0

    var body: some View {
        VStack(spacing: 0) {
            let start = offset
            let end = min(offset + 10, items.count)
            let indices = Array(start..<end)  // Array wrapper!

            ForEach(indices, id: \.self) { index in
                Text(items[index])
            }
        }
        .onKeyPress("S") { offset = 5 }
    }
}
```

**Expected Impact**: Identifies and tests solutions for the ForEach diffing bug.

---

### Task 4.6: Test Zero-Sized Rect Invalidation

**File**: `SwiftTUI/Tests/SwiftTUITests/ZeroSizedRectTests.swift`

**Problem**: Zero-sized rects cause Range crashes.

**Solution**: Specific tests for this edge case.

**Implementation** (~150 LOC):

```swift
import XCTest
@testable import SwiftTUI

final class ZeroSizedRectTests: XCTestCase {

    func testEmptyTextDoesNotCrashRenderer() {
        // Empty Text creates zero-sized rect
        let app = TestableApplication(
            rootView: VStack {
                Text("Before")
                Text("")  // Zero-sized!
                Text("After")
            },
            width: 40,
            height: 10
        )

        let matcher = TerminalMatcher(app: app)

        // Should render without crash
        matcher.assertLine(0, contains: "Before")
        matcher.assertLine(1, contains: "After")
    }

    func testZeroWidthColumnDoesNotCrash() {
        let app = TestableApplication(
            rootView: HStack {
                Text("Left")
                Text("").frame(width: 0)  // Explicit zero width
                Text("Right")
            },
            width: 40,
            height: 10
        )

        // Should not crash
        XCTAssertNotNil(app.cell(at: Position(column: 0, line: 0)))
    }

    func testInvalidationWithZeroSizedRect() {
        // Directly test Layer invalidation
        let layer = Layer()
        layer.frame = Rect(position: .zero, size: Size(width: 80, height: 24))

        // Create zero-sized rect (height = 0)
        let zeroRect = Rect(
            position: Position(column: 0, line: 10),
            size: Size(width: 10, height: 0)
        )

        // Should not crash
        layer.invalidate(rect: zeroRect)

        XCTAssertNotNil(layer.invalidated)
    }

    func testSpatialIndexWithZeroSizedFrame() {
        let layer = Layer()
        layer.frame = Rect(position: .zero, size: Size(width: 80, height: 24))

        let child = Layer()
        child.frame = Rect(
            position: Position(column: 10, line: 10),
            size: Size(width: 0, height: 0)  // Zero-sized!
        )

        // Should not crash when building spatial index
        layer.addLayer(child, at: 0)

        // Query should work
        let cell = layer.cell(at: Position(column: 15, line: 15))
        XCTAssertNotNil(cell)  // Should return something, even if nil
    }
}
```

**Expected Impact**: Ensures zero-sized rects are handled gracefully everywhere.

---

## Success Criteria

After implementing this plan, we will have:

1. ✅ **Virtual test environment** - Can test SwiftTUI rendering without real terminal
2. ✅ **Reproducible bug** - Can write test that fails before fix, passes after
3. ✅ **Visual debugging** - Can see exactly what's rendered at each step
4. ✅ **Regression prevention** - Tests catch scrolling bugs in future
5. ✅ **Fast iteration** - Run tests in <1 second vs manual terminal testing

## Implementation Timeline

- **Week 1**: Phase 1 - Virtual Terminal Abstraction (Foundation)
- **Week 2**: Phase 2 - Test Application Infrastructure (Harness)
- **Week 3**: Phase 3 - Assertion Framework (Verification)
- **Week 4**: Phase 4 - Scrolling Test Suite (Actual Tests)

## Files to Create

### SwiftTUI Testing Infrastructure
- `SwiftTUI/Sources/SwiftTUI/Testing/VirtualTerminal.swift` (~200 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/VirtualRenderer.swift` (~250 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/TerminalBuffer.swift` (~150 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/EscapeSequenceParser.swift` (~200 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/TestableApplication.swift` (~250 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/KeyEventSimulator.swift` (~200 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/RenderCapture.swift` (~150 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/AsyncHelpers.swift` (~100 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/TerminalMatcher.swift` (~250 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/BufferVisualizer.swift` (~200 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/ScrollingMatchers.swift` (~150 LOC)
- `SwiftTUI/Sources/SwiftTUI/Testing/RenderTimeline.swift` (~200 LOC)

### Test Files
- `SwiftTUI/Tests/SwiftTUITests/ScrollingNavigationTests.swift` (~150 LOC)
- `SwiftTUI/Tests/SwiftTUITests/HalfPageScrollTests.swift` (~200 LOC)
- `SwiftTUI/Tests/SwiftTUITests/ViewportBoundaryTests.swift` (~200 LOC)
- `SwiftTUI/Tests/SwiftTUITests/LargeListTests.swift` (~150 LOC)
- `SwiftTUI/Tests/SwiftTUITests/ForEachDiffingTests.swift` (~250 LOC)
- `SwiftTUI/Tests/SwiftTUITests/ZeroSizedRectTests.swift` (~150 LOC)

**Total**: ~3,150 LOC across 18 files

---

## Conclusion

This plan provides a **comprehensive, reproducible test infrastructure** for SwiftTUI that will:

1. **Enable debugging** the current scrolling bug with full visibility into rendering
2. **Prevent regressions** by capturing correct behavior in tests
3. **Speed up development** by removing need for manual terminal testing
4. **Document behavior** through executable test specifications

The virtual terminal abstraction is the key innovation - it allows SwiftTUI to render to memory instead of stdout, making every aspect of rendering inspectable and testable.

**Next step**: Begin with Phase 1, Task 1.1 to create the VirtualTerminal protocol and foundation.
