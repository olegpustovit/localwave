#!/bin/bash
# macbench.sh <pxui worktree> [--no-default-features] : one release abi_bench at a time on this Mac (mkdir lock), 6 build jobs.
LOCK=/private/tmp/claude-501/dx-macbench.lock
cd "$1" || exit 2; shift
. /private/tmp/claude-501/-Users-snowbear-WORK-GIT-localwave/70341840-4a85-459b-b067-76591dd048ed/scratchpad/dx_impl/lock.sh; take_lock
trap 'rm -rf "$LOCK"' EXIT
CARGO_BUILD_JOBS=6 RAYS_SLOW=1 cargo t --release -p pxui_gallery "$@" abi_bench
