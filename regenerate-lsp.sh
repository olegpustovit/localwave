#!/bin/bash
# Regenerate SourceKit-LSP index for VSCode
# Run this after adding/removing files or when autocomplete stops working

set -e

echo "🧹 Cleaning old build artifacts..."
rm -rf .build .swiftpm/xcode

echo "🔨 Building Swift Package..."
swift build --build-tests

echo "✅ SourceKit-LSP index regenerated!"
echo ""
echo "📝 Next steps:"
echo "  1. Restart VSCode (Cmd+Q, then reopen)"
echo "  2. Or reload window: Cmd+Shift+P → 'Developer: Reload Window'"
echo ""
echo "LSP should now work properly with autocomplete, go-to-definition, etc."
