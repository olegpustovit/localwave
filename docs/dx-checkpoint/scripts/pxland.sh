#!/bin/bash
# pxland.sh BRANCH... : cherry-pick onto pxui rust-port, run standing checks (lanes deprioritized), push.
set -e
cd /Users/snowbear/WORK/GIT/localwave/pxui
git checkout -q rust-port
for b in "$@"; do git cherry-pick $b >/dev/null; done
. /private/tmp/claude-501/-Users-snowbear-WORK-GIT-localwave/70341840-4a85-459b-b067-76591dd048ed/scratchpad/dx_impl/lock.sh; take_lock
pkill -STOP -f "dx-wt|debugserver" || true; trap 'pkill -CONT -f "dx-wt|debugserver"; rm -rf "$LOCK" 2>/dev/null' EXIT
PK=""; for c in ${PKGS:-pxui_core pxui pxui_abi}; do PK="$PK -p $c"; done; cargo t $PK > /tmp/pxland_t.log 2>&1 || cargo t $PK > /tmp/pxland_t.log 2>&1 || { grep -E "failed \(exit|panicked at|exceeded" /tmp/pxland_t.log | tail -5; echo "PXUI T FAILED"; exit 1; }
cargo check -q --workspace --tests > /tmp/pxland_ws.log 2>&1 || { grep -E "^error" -A6 /tmp/pxland_ws.log | head -20; echo "WORKSPACE TESTS DO NOT COMPILE"; exit 1; }
for f in '' --no-default-features; do cargo check -q -p abi_bench -p pxui_gallery --tests $f > /tmp/pxland_gal.log 2>&1 || { tail -8 /tmp/pxland_gal.log; echo GALLERY BUILD FAILED; exit 1; }; done
if [ "${GAL:-0}" = 1 ]; then cargo t -p pxui_gallery > /tmp/pxland_galt.log 2>&1 || cargo t -p pxui_gallery > /tmp/pxland_galt.log 2>&1 || { grep -E 'failed \(exit|exceeded' /tmp/pxland_galt.log | tail -5; echo GALLERY TESTS FAILED; exit 1; }; fi
if [ "${BENCH:-0}" = 1 ]; then for f in '' --no-default-features; do RAYS_SLOW=1 cargo t --release -p pxui_gallery $f abi_bench > /tmp/pxland_bench.log 2>&1 || RAYS_SLOW=1 cargo t --release -p pxui_gallery $f abi_bench > /tmp/pxland_bench.log 2>&1 || { grep -E 'over [0-9.]+%|above|failed \(exit' /tmp/pxland_bench.log | tail -6; echo "ABI_BENCH FAILED ($f)"; exit 1; }; done; fi
if [ "${FULL:-0}" = 1 ]; then cargo t -p pxui --doc > /tmp/pxland_doc.log 2>&1 || { tail -5 /tmp/pxland_doc.log; echo DOC FAILED; exit 1; }; fi
for g in deps shape abi-header abi-snapshot ${FULLGATES:-}; do cargo run -q -p gate -- $g > /tmp/pxland_g.log 2>&1 || { tail -5 /tmp/pxland_g.log; echo "GATE $g FAILED"; exit 1; }; done
git diff --check HEAD~$# 
{ pkill -CONT -f "dx-wt|debugserver" || true; }; git push -q origin rust-port && git fetch -q origin && git branch -r --contains HEAD | grep -q origin/rust-port && echo "PUSHED"
git log --oneline -$#
/private/tmp/claude-501/-Users-snowbear-WORK-GIT-localwave/70341840-4a85-459b-b067-76591dd048ed/scratchpad/dx_impl/remote_gc.sh "$@" || true
