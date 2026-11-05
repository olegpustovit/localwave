#!/bin/bash
# Builds the native Mac app, signs it with Developer ID, notarizes and staples a DMG.
# Env: SIGN_ID (Developer ID Application identity), VERSION (optional), NOTARY_PROFILE + optional NOTARY_KEYCHAIN
# (a notarytool keychain profile); without NOTARY_PROFILE notarization is skipped (local smoke only).
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
output_dir="${1:-$repo_dir/build/dmg}"
mkdir -p "$output_dir"
output_dir="$(cd "$output_dir" && pwd)"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/localwave-dmg.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT
: "${SIGN_ID:?SIGN_ID must name a Developer ID Application identity}"

settings=()
[ -z "${VERSION:-}" ] || settings+=(MARKETING_VERSION="$VERSION")
[ -z "${BUILD:-}" ] || settings+=(CURRENT_PROJECT_VERSION="$BUILD")

xcodebuild -project "$repo_dir/localwave.xcodeproj" -scheme musicapp \
  -configuration Release -destination 'platform=macOS' \
  -derivedDataPath "$work_dir/DerivedData" \
  CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO ${settings[@]+"${settings[@]}"} build

app="$work_dir/DerivedData/Build/Products/Release/localwave.app"
keychain=()
[ -z "${SIGN_KEYCHAIN:-}" ] || keychain=(--keychain "$SIGN_KEYCHAIN")
codesign --force --deep --timestamp --options runtime ${keychain[@]+"${keychain[@]}"} \
  --entitlements "$repo_dir/localwave/mac.entitlements" --sign "$SIGN_ID" "$app"
codesign --verify --deep --strict --verbose=2 "$app"

version="$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' "$app/Contents/Info.plist")"
dmg="$output_dir/LocalWave-$version.dmg"
rm -f "$dmg"
mkdir "$work_dir/dmg"
ditto --norsrc "$app" "$work_dir/dmg/LocalWave.app"
ln -s /Applications "$work_dir/dmg/Applications"
hdiutil create -quiet -volname LocalWave -srcfolder "$work_dir/dmg" -ov -format UDZO "$dmg"
codesign --force --timestamp ${keychain[@]+"${keychain[@]}"} --sign "$SIGN_ID" "$dmg"

if [ -n "${NOTARY_PROFILE:-}" ]; then
  notary=(--keychain-profile "$NOTARY_PROFILE")
  [ -z "${NOTARY_KEYCHAIN:-}" ] || notary+=(--keychain "$NOTARY_KEYCHAIN")
  xcrun notarytool submit "$dmg" "${notary[@]}" --wait
  xcrun stapler staple "$dmg"
  xcrun stapler validate "$dmg"
fi
(cd "$output_dir" && shasum -a 256 "$(basename "$dmg")" > "$(basename "$dmg").sha256")
echo "DMG: $dmg"
