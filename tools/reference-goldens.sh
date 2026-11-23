#!/bin/bash
# Capture 1440x872 goldens: Swift reference fixture (iOS simulator canvas, DEBUG capture hook) and the
# Rust app (--reference-screen writes its native 1× content-area PNG through pxui_host's Graphics::screenshot).
# DARK=1 compares the dark appearance. MOBILE=1: 390x778 canvas (Swift without LOCALWAVE_REFERENCE_DESKTOP; Rust compact window), default OUTDIR /tmp/localwave-goldens-mobile.
# Usage: tools/reference-goldens.sh [OUTDIR]; SKIP_BUILD=1 reuses builds, SKIP_SWIFT=1 skips the Swift captures, SWIFT_ONLY="a b" limits them, ONLY="a b" limits the Rust screens. Then tools/compare-goldens.py OUTDIR.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ -n "${MOBILE:-}" ]; then OUT=${1:-/tmp/localwave-goldens-mobile}; W=390 H=778 DESKTOP=; else OUT=${1:-/tmp/localwave-goldens}; W=1440 H=872 DESKTOP=1; fi
# DARK=1: both sides in dark appearance (the simulator's UI style; Swift's fixture follows System), default OUTDIR suffixed -dark.
[ -z "${DARK:-}" ] || OUT=${1:-$OUT-dark}
SIM=${SIMULATOR:-booted}
BUNDLE=com.olegpustovit.localwave
# Swift screens; add (picker sheet) and track (anchored popup never paints in the Rust capture) are captured, not compared.
SWIFT_SCREENS=${SWIFT_ONLY:-"songs artists album artist list favorites find find-results lists folders settings commands shortcuts now queue add folder track onboarding"}
RUST_SCREENS=${ONLY:-"songs artists album artist list favorites find find-results lists folders settings commands shortcuts now queue folder onboarding"}
mkdir -p "$OUT/reference" "$OUT/rust"

# xcodebuild may rewrite the tracked Package.resolved; always put it back.
RESOLVED=localwave.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved
trap 'git checkout -q -- "$RESOLVED" 2>/dev/null || true' EXIT
if [ -z "${SKIP_BUILD:-}" ]; then
  [ -n "${SKIP_SWIFT:-}" ] || xcodebuild -project localwave.xcodeproj -scheme musicapp -destination "platform=iOS Simulator,id=$([ "$SIM" != booted ] && echo "$SIM" || xcrun simctl list devices booted -j | python3 -c 'import json,sys;print(next(d["udid"] for k,v in json.load(sys.stdin)["devices"].items() if "iOS" in k for d in v))')" \
    -derivedDataPath "$OUT/dd" CODE_SIGNING_ALLOWED=NO build > "$OUT/xcodebuild.log" 2>&1 || { tail -20 "$OUT/xcodebuild.log"; exit 1; }
  make build PROFILE=debug  # libpxui as well: lw_bin alone leaves a stale dylib after pxui changes
  # target/ is shared with other worktrees, which overwrite target/debug/localwave; keep a private copy.
  mkdir -p "$OUT/bin" && cp target/debug/localwave target/debug/libpxui.2.dylib "$OUT/bin/"
fi
[ -n "${SKIP_SWIFT:-}" ] || xcrun simctl ui "$SIM" appearance "$([ -n "${DARK:-}" ] && echo dark || echo light)"
[ -n "${SKIP_SWIFT:-}" ] || xcrun simctl install "$SIM" "$(ls -d "$OUT"/dd/Build/Products/Debug-iphonesimulator/*.app | head -1)"

for s in $([ -n "${SKIP_SWIFT:-}" ] || echo $SWIFT_SCREENS); do
  rm -f "$OUT/reference/$s.png"
  SIMCTL_CHILD_LOCALWAVE_REFERENCE_FIXTURE=1 SIMCTL_CHILD_LOCALWAVE_REFERENCE_DESKTOP=$DESKTOP \
  SIMCTL_CHILD_LOCALWAVE_UI_TEST_ID=$(uuidgen) SIMCTL_CHILD_LOCALWAVE_REFERENCE_SCREEN=$s \
  SIMCTL_CHILD_LOCALWAVE_REFERENCE_CAPTURE="$OUT/reference/$s.png" \
    xcrun simctl launch --terminate-running-process "$SIM" $BUNDLE > /dev/null
  for _ in $(seq 60); do [ -f "$OUT/reference/$s.png" ] && break; sleep 1; done
  [ -f "$OUT/reference/$s.png" ] || echo "swift $s: no capture" >&2
  xcrun simctl terminate "$SIM" $BUNDLE || true
done

for s in $RUST_SCREENS; do
  rm -f "$OUT/rust/$s.png"
  env ${MOBILE:+LOCALWAVE_REFERENCE_MOBILE=1} ${DARK:+LOCALWAVE_REFERENCE_DARK=1} LOCALWAVE_REFERENCE_CAPTURE="$OUT/rust/$s.png" "$OUT/bin/localwave" --reference-screen "$s" > "$OUT/rust-$s.log" 2>&1 &
  pid=$!
  for _ in $(seq 240); do grep -q "READY\|LocalWave:" "$OUT/rust-$s.log" && break; sleep 0.5; done
  [ -f "$OUT/rust/$s.png" ] || echo "rust $s: $(cat "$OUT/rust-$s.log")" >&2
  kill -9 $pid 2>/dev/null || true; wait $pid 2>/dev/null || true  # SIGTERM can hang on the sheet screens
done
echo "captured into $OUT; run tools/compare-goldens.py $OUT"
