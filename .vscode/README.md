# VSCode Setup for LocalWave TUI Development

## Required Extensions

Install these VSCode extensions:
1. **Swift Language Support** (`sswg.swift-lang`)
2. **CodeLLDB** (`vadimcn.vscode-lldb`) - for debugging

VSCode will prompt you to install these when you open the project.

## Debug Configurations

Three debug configurations are available in the Debug panel (⇧⌘D):

### 1. Debug TUI (Recommended)
- Runs TUI in VSCode's integrated terminal
- Full debugging support (breakpoints, step through, inspect variables)
- Press **F5** to start

### 2. Run TUI (No Debug)
- Faster startup, no debugger attached
- Good for quick testing
- Press **^F5** (Ctrl+F5)

### 3. Debug TUI (External Terminal)
- Runs in macOS Terminal.app
- Better terminal compatibility
- Use if integrated terminal has display issues

## How to Debug

1. **Set breakpoints**: Click in the left margin next to line numbers
2. **Start debugging**: Press **F5** or select "Debug TUI" from Debug panel
3. **Debug controls**:
   - Continue: F5
   - Step Over: F10
   - Step Into: F11
   - Step Out: ⇧F11
   - Restart: ⇧⌘F5
   - Stop: ⇧F5

4. **Inspect variables**: Hover over variables or check the Variables panel

## Build Tasks

Run tasks via **⇧⌘B** or Command Palette (⇧⌘P):
- `swift-build-tui` - Build only TUI (default)
- `swift-build-all` - Build all targets
- `swift-clean` - Clean build artifacts
- `swift-test` - Run tests

## Keyboard Shortcuts

- **F5** - Start debugging
- **^F5** - Run without debugging
- **⇧⌘B** - Build
- **⇧⌘D** - Open Debug panel
- **⌘K ⌘I** - Show hover info (type definitions, documentation)

## Tips

- The integrated terminal supports the TUI's interactive features
- If the TUI display looks broken, try "Debug TUI (External Terminal)"
- Build output appears in the Terminal panel
- LLDB console is available in the Debug Console panel

## Language Server (Autocomplete & Navigation)

This project uses **SourceKit-LSP** (not xcode-build-server) for Swift language support.

### If autocomplete/go-to-definition stops working:

1. Run the regeneration script:
   ```bash
   ./regenerate-lsp.sh
   ```

2. Restart VSCode (⌘Q, then reopen)

### Manual regeneration:
```bash
rm -rf .build .swiftpm/xcode
swift build --build-tests
```

Then reload VSCode window: ⇧⌘P → "Developer: Reload Window"

## Troubleshooting

**Problem**: "CodeLLDB is not available"
**Solution**: Install the CodeLLDB extension

**Problem**: TUI display is garbled
**Solution**: Use "Debug TUI (External Terminal)" configuration

**Problem**: Breakpoints not working
**Solution**: Make sure you're using "Debug TUI", not "Run TUI (No Debug)"

**Problem**: Autocomplete not working / "Cannot find type in scope"
**Solution**: Run `./regenerate-lsp.sh` and restart VSCode

**Problem**: xcode-build-server errors
**Solution**: Ignore them - this project uses Swift Package Manager with SourceKit-LSP, not xcode-build-server
