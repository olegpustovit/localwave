#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
products=${1:-target/release}
app=target/bundle/LocalWave.app
frameworks=$app/Contents/Frameworks
identity=${CODESIGN_IDENTITY:--}
test -f "$products/localwave"
test -f "$products/libpxui.1.dylib"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$frameworks"
cp "$products/localwave" "$app/Contents/MacOS/localwave"
cp -L "$products/libpxui.1.dylib" "$frameworks/libpxui.1.dylib"
cp packaging/Info.plist "$app/Contents/Info.plist"
if [ -n "${PROVISIONING_PROFILE:-}" ]; then
    cp "$PROVISIONING_PROFILE" "$app/Contents/embedded.provisionprofile"
fi

for binary in "$app/Contents/MacOS/localwave" "$frameworks/libpxui.1.dylib"; do
    codesign --remove-signature "$binary" 2>/dev/null || true
    otool -l "$binary" | awk '/cmd LC_RPATH/{f=1} f && / path /{print $2; f=0}' |
        while IFS= read -r path; do install_name_tool -delete_rpath "$path" "$binary"; done
done
install_name_tool -id @rpath/libpxui.1.dylib "$frameworks/libpxui.1.dylib"
install_name_tool -add_rpath @executable_path/../Frameworks "$app/Contents/MacOS/localwave"

# A store bundle must not quietly depend on a developer's Homebrew installation.
for binary in "$app/Contents/MacOS/localwave" "$frameworks/libpxui.1.dylib"; do
    otool -L "$binary" | tail -n +2 | awk '{print $1}' | while IFS= read -r dependency; do
        case "$dependency" in
            /usr/lib/*|/System/Library/*|@rpath/libpxui.1.dylib) ;;
            *) echo "Unbundled dependency: $dependency in $binary" >&2; exit 1 ;;
        esac
    done
done
set --
if [ "$identity" != - ]; then set -- --options runtime; fi
codesign --force --sign "$identity" "$@" "$frameworks/libpxui.1.dylib"
codesign --force --sign "$identity" "$@" --entitlements localwave/mac.entitlements "$app"
codesign --verify --deep --strict --verbose=2 "$app"
echo "$app"
