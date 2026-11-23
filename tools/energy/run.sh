#!/bin/sh
# Run a signed LocalWave release in a separate macOS application sandbox.
set -eu
cd "$(dirname "$0")/../.."
case "${1-}" in playing-list|idle|pointer|hidden) scenario=$1;;
    *) echo "usage: tools/energy/run.sh <playing-list|idle|pointer|hidden> [seconds]" >&2; exit 2;;
esac
seconds=${2:-5}
case "$seconds" in ''|*[!0-9]*) exit 2;; esac
[ "$#" -le 2 ] && [ "$seconds" -ge 5 ] && [ "$seconds" -le 120 ] || exit 2
source=target/bundle/LocalWave.app
[ -x "$source/Contents/MacOS/localwave" ] || { echo "Build the signed release with make bundle first" >&2; exit 1; }
codesign --verify --deep --strict "$source"
mkdir -p target/energy
run=$(mktemp -d "$PWD/target/energy/$scenario.XXXXXX")
app=$run/LocalWave.app
trap 'rm -rf "$app"' EXIT
ditto "$source" "$app"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier com.olegpustovit.localwave.energy' "$app/Contents/Info.plist"
codesign --force --sign - --entitlements localwave/mac.entitlements "$app"
codesign --verify --deep --strict "$app"
python3 tools/energy/measure.py "$app" "$scenario" "$seconds"
