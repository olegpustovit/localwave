#!/usr/bin/env bash
# Give the counted release its own app sandbox before opening any Cocoa or playback state.
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ ${1:-} == smoke ]]; then
    export LOCALWAVE_SOAK_PLAYBACK_SECONDS=3 LOCALWAVE_SOAK_SCROLL_SECONDS=3 LOCALWAVE_SOAK_CHURN_SECONDS=3
    export LOCALWAVE_SOAK_SAMPLE_SECONDS=1 LOCALWAVE_SOAK_VERIFY_EVERY=10
fi

source=target/bundle/LocalWave.app
codesign --verify --deep --strict "$source"
mkdir -p target/soak
run=$(mktemp -d "$PWD/target/soak/run.XXXXXX")
app=$run/LocalWave.app
trap 'rm -rf "$app"' EXIT
ditto "$source" "$app"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier com.olegpustovit.localwave.soak' "$app/Contents/Info.plist"
codesign --force --sign - --entitlements localwave/mac.entitlements "$app"
codesign --verify --deep --strict "$app"

printf 'LocalWave soak samples: %s/samples.jsonl\n' "$run"
PXUI_FULL_DAMAGE=0 "$app/Contents/MacOS/localwave" --soak -ApplePersistenceIgnoreState YES \
    2> >(tee "$run/stderr.log" >&2) | tee "$run/samples.jsonl"
