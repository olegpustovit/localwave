#!/bin/bash
# Capture the Rust app's 1440x872 goldens (--reference-screen writes its native 1x content-area PNG through pxui_host's
# Graphics::screenshot) into OUTDIR/rust. The Swift references in OUTDIR/reference come from the archived `swift` branch's
# tools/reference-goldens.sh (iOS simulator). DARK=1 dark appearance; MOBILE=1 390x778 compact window.
# Usage: tools/reference-goldens.sh [OUTDIR]; SKIP_BUILD=1 reuses the build, ONLY="a b" limits the screens. Then tools/compare-goldens.py OUTDIR.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ -n "${MOBILE:-}" ]; then OUT=${1:-/tmp/localwave-goldens-mobile}; else OUT=${1:-/tmp/localwave-goldens}; fi
# DARK=1: default OUTDIR suffixed -dark.
[ -z "${DARK:-}" ] || OUT=${1:-$OUT-dark}
RUST_SCREENS=${ONLY:-"songs artists album artist list favorites find find-results lists folders settings commands shortcuts now queue folder onboarding"}
mkdir -p "$OUT/rust"

if [ -z "${SKIP_BUILD:-}" ]; then
  make build PROFILE=debug  # libpxui as well: lw_bin alone leaves a stale dylib after pxui changes
  # target/ is shared with other worktrees, which overwrite target/debug/localwave; keep a private copy.
  mkdir -p "$OUT/bin" && cp target/debug/localwave target/debug/libpxui.2.dylib "$OUT/bin/"
fi

for s in $RUST_SCREENS; do
  rm -f "$OUT/rust/$s.png"
  env ${MOBILE:+LOCALWAVE_REFERENCE_MOBILE=1} ${DARK:+LOCALWAVE_REFERENCE_DARK=1} LOCALWAVE_REFERENCE_CAPTURE="$OUT/rust/$s.png" "$OUT/bin/localwave" --reference-screen "$s" > "$OUT/rust-$s.log" 2>&1 &
  pid=$!
  for _ in $(seq 240); do grep -q "READY\|LocalWave:" "$OUT/rust-$s.log" && break; sleep 0.5; done
  [ -f "$OUT/rust/$s.png" ] || echo "rust $s: $(cat "$OUT/rust-$s.log")" >&2
  kill -9 $pid 2>/dev/null || true; wait $pid 2>/dev/null || true  # SIGTERM can hang on the sheet screens
done
echo "captured into $OUT; run tools/compare-goldens.py $OUT"
