#!/bin/bash
# Debug TUI with LLDB
# Usage: ./debug-tui.sh

set -e

echo "Building TUI..."
swift build --product localwave-tui

echo ""
echo "Starting LLDB debugger..."
echo "Useful commands:"
echo "  (lldb) run          - Start the TUI"
echo "  (lldb) b main       - Set breakpoint at main"
echo "  (lldb) b <file>:<line> - Set breakpoint at specific location"
echo "  (lldb) c            - Continue execution"
echo "  (lldb) quit         - Exit debugger"
echo ""

lldb .build/debug/localwave-tui
