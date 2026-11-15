#!/bin/bash
# Capture 1440x872 goldens: Swift reference fixture (iOS simulator canvas, DEBUG capture hook) and the
# Rust app (--reference-screen, real window via screencapture: pxui_host has no pixel readback).
# Usage: tools/reference-goldens.sh [OUTDIR]; SKIP_BUILD=1 reuses builds, SKIP_SWIFT=1 skips the Swift captures, ONLY="a b" limits the Rust screens. Then tools/compare-goldens.py OUTDIR.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=${1:-/tmp/localwave-goldens}
SIM=${SIMULATOR:-booted}
BUNDLE=com.olegpustovit.localwave
# Swift screens; Rust has no equivalent of add, folder, track and onboarding (captured, not compared).
SWIFT_SCREENS="songs artists album artist list favorites find find-results lists folders settings commands shortcuts now queue add folder track onboarding"
RUST_SCREENS=${ONLY:-"songs artists album artist list favorites find find-results lists folders settings commands shortcuts now queue"}
mkdir -p "$OUT/reference" "$OUT/rust"

if [ -z "${SKIP_BUILD:-}" ]; then
  xcodebuild -project localwave.xcodeproj -scheme musicapp -destination "platform=iOS Simulator,id=$(xcrun simctl list devices booted -j | python3 -c 'import json,sys;print(next(d["udid"] for v in json.load(sys.stdin)["devices"].values() for d in v))')" \
    -derivedDataPath "$OUT/dd" CODE_SIGNING_ALLOWED=NO build > "$OUT/xcodebuild.log" 2>&1 || { tail -20 "$OUT/xcodebuild.log"; exit 1; }
  cargo build -p lw_bin
  # target/ is shared with other worktrees, which overwrite target/debug/localwave; keep a private copy.
  mkdir -p "$OUT/bin" && cp target/debug/localwave target/debug/libpxui.1.dylib "$OUT/bin/"
fi
xcrun simctl install "$SIM" "$(ls -d "$OUT"/dd/Build/Products/Debug-iphonesimulator/*.app | head -1)"

for s in $([ -n "${SKIP_SWIFT:-}" ] || echo $SWIFT_SCREENS); do
  rm -f "$OUT/reference/$s.png"
  SIMCTL_CHILD_LOCALWAVE_REFERENCE_FIXTURE=1 SIMCTL_CHILD_LOCALWAVE_REFERENCE_DESKTOP=1 \
  SIMCTL_CHILD_LOCALWAVE_UI_TEST_ID=$(uuidgen) SIMCTL_CHILD_LOCALWAVE_REFERENCE_SCREEN=$s \
  SIMCTL_CHILD_LOCALWAVE_REFERENCE_CAPTURE="$OUT/reference/$s.png" \
    xcrun simctl launch --terminate-running-process "$SIM" $BUNDLE > /dev/null
  for _ in $(seq 60); do [ -f "$OUT/reference/$s.png" ] && break; sleep 1; done
  [ -f "$OUT/reference/$s.png" ] || echo "swift $s: no capture" >&2
  xcrun simctl terminate "$SIM" $BUNDLE || true
done

window_id() { # 1440-wide window of pid $1
  osascript -l JavaScript -e "ObjC.import('CoreGraphics');var l=ObjC.deepUnwrap(ObjC.castRefToObject(\$.CGWindowListCopyWindowInfo(\$.kCGWindowListOptionAll,0)));l.filter(w=>w.kCGWindowOwnerPID==$1&&w.kCGWindowBounds.Width==1440).map(w=>w.kCGWindowNumber).join('')"
}
for s in $RUST_SCREENS; do
  rm -f "$OUT/rust/$s.png"
  "$OUT/bin/localwave" --reference-screen "$s" > "$OUT/rust-$s.log" 2>&1 &
  pid=$!
  for _ in $(seq 240); do grep -q "READY\|LocalWave:" "$OUT/rust-$s.log" && break; sleep 0.5; done
  if grep -q READY "$OUT/rust-$s.log"; then
    screencapture -x -o -l "$(window_id $pid)" "$OUT/rust/$s.png" || echo "rust $s: capture failed" >&2
  else
    echo "rust $s: $(cat "$OUT/rust-$s.log")" >&2
  fi
  kill -9 $pid 2>/dev/null || true; wait $pid 2>/dev/null || true  # SIGTERM can hang on the sheet screens
done
echo "captured into $OUT; run tools/compare-goldens.py $OUT"
