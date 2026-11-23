#!/bin/sh
# Give the regression its own launch identity so a crash cannot trigger AppKit restore prompts.
set -eu
cd "$(dirname "$0")/.."
source=target/bundle/LocalWave.app
codesign --verify --deep --strict "$source"
run=$(mktemp -d "$PWD/target/compositor-smoke.XXXXXX")
app=$run/LocalWave.app
trap 'rm -rf "$run"' EXIT
ditto "$source" "$app"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier com.olegpustovit.localwave.compositor-smoke.${run##*.}" "$app/Contents/Info.plist"
codesign --force --sign - --entitlements localwave/mac.entitlements "$app"
codesign --verify --deep --strict "$app"
PXUI_FRAME_REASONS=1 "$app/Contents/MacOS/localwave" --performance-check --performance-song-indicators
