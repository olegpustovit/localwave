#!/bin/bash
# usage: remote.sh <pxui worktree dir> <command...>   |   remote.sh --sync-only <dir>
set -euo pipefail
HOST=snowbear@gigacomp
SYNC_ONLY=0
if [ "${1:-}" = "--sync-only" ]; then SYNC_ONLY=1; shift; fi
DIR=${1:?usage: remote.sh [--sync-only] <worktree dir> <command...>}; shift
DIR=$(cd "$DIR" && pwd); BASE=$(echo "$DIR" | sed "s#^/Users/snowbear/WORK/GIT/##; s#[/ ]#_#g")
"/private/tmp/claude-501/-Users-snowbear-WORK-GIT-localwave/70341840-4a85-459b-b067-76591dd048ed/scratchpad/dx_impl/remote_gc.sh" >/dev/null 2>&1 || true
USED=$(ssh -q $HOST "du -s ~/pxui-dx 2>/dev/null | cut -f1" || echo 0)
if [ "${USED:-0}" -gt $((120*1024*1024)) ]; then echo "remote.sh: ~/pxui-dx on gigacomp is over 120 GB; delete finished lanes' target dirs first (ssh snowbear@gigacomp 'du -sh ~/pxui-dx/*')" >&2; exit 3; fi
ssh -q $HOST "mkdir -p ~/pxui-dx/$BASE"
rsync -az --delete --exclude target/ --exclude .git/ --exclude .DS_Store -e "ssh -q" "$DIR/" "$HOST:pxui-dx/$BASE/"
[ $SYNC_ONLY = 1 ] && exit 0
[ $# -gt 0 ] || { echo "no command" >&2; exit 2; }
CMD=$(printf '%q ' "$@")
# script goes over stdin to bash -l (the remote login shell is fish)
ssh -q $HOST 'bash -l -s' <<REMOTE
set -e
export CARGO_TARGET_DIR=\$HOME/pxui-dx/target-$BASE
export XDG_RUNTIME_DIR=/mnt/wslg/runtime-dir WAYLAND_DISPLAY=wayland-0
export CARGO_TERM_COLOR=never
cd \$HOME/pxui-dx/$BASE
$CMD
REMOTE
