#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
products=${1:-target/release}
app=target/bundle/LocalWave.app
frameworks=$app/Contents/Frameworks
identity=${CODESIGN_IDENTITY:--}
entitlements=${ENTITLEMENTS:-localwave/mac.entitlements}
test -f "$products/localwave"
test -f "$products/libpxui.2.dylib"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$frameworks"
cp "$products/localwave" "$app/Contents/MacOS/localwave"
cp -L "$products/libpxui.2.dylib" "$frameworks/libpxui.2.dylib"
cp packaging/Info.plist "$app/Contents/Info.plist"
# Store builds set MARKETING_VERSION / BUILD_NUMBER; development bundles keep the plist's values.
if [ -n "${MARKETING_VERSION:-}" ]; then /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $MARKETING_VERSION" "$app/Contents/Info.plist"; fi
if [ -n "${BUILD_NUMBER:-}" ]; then /usr/libexec/PlistBuddy -c "Set :CFBundleVersion $BUILD_NUMBER" "$app/Contents/Info.plist"; fi
# The macOS icon comes from the same 1024-point artwork as iOS.
iconset=$(mktemp -d)/AppIcon.iconset
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    sips -z $size $size localwave/Assets.xcassets/AppIcon.appiconset/AppIcon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    sips -z $((size * 2)) $((size * 2)) localwave/Assets.xcassets/AppIcon.appiconset/AppIcon.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"
# Swift's bundled demo library (onboarding's "Try the demo library")
cp -R localwave/DemoLibrary "$app/Contents/Resources/DemoLibrary"
if [ -n "${PROVISIONING_PROFILE:-}" ]; then
    cp "$PROVISIONING_PROFILE" "$app/Contents/embedded.provisionprofile"
fi

for binary in "$app/Contents/MacOS/localwave" "$frameworks/libpxui.2.dylib"; do
    codesign --remove-signature "$binary" 2>/dev/null || true
    otool -l "$binary" | awk '/cmd LC_RPATH/{f=1} f && / path /{print $2; f=0}' |
        while IFS= read -r path; do install_name_tool -delete_rpath "$path" "$binary"; done
done
install_name_tool -id @rpath/libpxui.2.dylib "$frameworks/libpxui.2.dylib"
install_name_tool -add_rpath @executable_path/../Frameworks "$app/Contents/MacOS/localwave"

# A store bundle must not quietly depend on a developer's Homebrew installation.
for binary in "$app/Contents/MacOS/localwave" "$frameworks/libpxui.2.dylib"; do
    otool -L "$binary" | tail -n +2 | awk '{print $1}' | while IFS= read -r dependency; do
        case "$dependency" in
            /usr/lib/*|/System/Library/*|@rpath/libpxui.2.dylib) ;;
            *) echo "Unbundled dependency: $dependency in $binary" >&2; exit 1 ;;
        esac
    done
done
set --
if [ "$identity" != - ]; then set -- --options runtime; fi
codesign --force --sign "$identity" "$@" "$frameworks/libpxui.2.dylib"
codesign --force --sign "$identity" "$@" --entitlements "$entitlements" "$app"
codesign --verify --deep --strict --verbose=2 "$app"
echo "$app"
