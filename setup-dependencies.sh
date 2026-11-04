#!/bin/bash
# Setup local dependencies for LocalWave development
# This script clones SwiftTUI locally so you can modify the library directly

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SWIFTTUI_DIR="$SCRIPT_DIR/SwiftTUI"

echo "🔧 Setting up LocalWave dependencies..."
echo ""

# Check if SwiftTUI directory already exists
if [ -d "$SWIFTTUI_DIR" ]; then
    echo "✅ SwiftTUI directory already exists at: $SWIFTTUI_DIR"
    echo "   Skipping clone. If you want to re-clone, delete the directory first:"
    echo "   rm -rf SwiftTUI"
else
    echo "📦 Cloning SwiftTUI..."
    git clone https://github.com/nexo-tech/SwiftTUI.git "$SWIFTTUI_DIR"
    echo "✅ SwiftTUI cloned successfully"
fi

echo ""
echo "📝 Updating Package.swift to use local SwiftTUI..."

# The Package.swift is already configured to use local SwiftTUI
# Just verify it exists
if grep -q "path: \"SwiftTUI\"" "$SCRIPT_DIR/Package.swift"; then
    echo "✅ Package.swift already configured for local SwiftTUI"
else
    echo "⚠️  Warning: Package.swift might not be configured for local SwiftTUI"
    echo "   Check the SwiftTUI dependency in Package.swift"
fi

echo ""
echo "🧹 Cleaning build cache..."
rm -rf .build .swiftpm/xcode

echo ""
echo "🔨 Building to verify setup..."
swift build --target localwave-tui

echo ""
echo "✅ Dependencies setup complete!"
echo ""
echo "📂 SwiftTUI location: $SWIFTTUI_DIR"
echo ""
echo "You can now:"
echo "  - Modify SwiftTUI directly in: ./SwiftTUI/"
echo "  - Changes will be picked up on next build"
echo "  - Build: swift build --target localwave-tui"
echo "  - Run: .build/debug/localwave-tui"
echo ""
echo "To switch back to remote SwiftTUI, edit Package.swift and change:"
echo "  .package(path: \"SwiftTUI\")"
echo "to:"
echo "  .package(url: \"https://github.com/nexo-tech/SwiftTUI.git\", branch: \"onKeyPress\")"
