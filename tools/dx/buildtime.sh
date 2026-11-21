#!/bin/bash
# Build-time meter (U0.3): tools/dx/buildtime.sh [--compare <rev>] [--rounds N]
#
#  a  clean pxui workspace debug build              (budget 30 s)
#  b  warm cargo check -p pxui_core after a touch    (budget < 1 s)
#  c  clean cargo build -p pxui_dylib
#  d  clean cargo build -p lw_ui, dependencies prebuilt
#  e  warm cargo check -p lw_ui after touching crates/lw_ui/src/shell/sidebar.rs
#  f  warm cargo build -p lw_bin after the same touch
#
# Runs N (default 3) alternating rounds (HEAD, then <rev>, per round, so machine noise hits both) and reports
# each number's minimum. With --compare it exits 1 when a HEAD number exceeds <rev>'s by more than
# max(5%, 0.3 s). Target dirs: target/buildtime-a (this tree) and target/buildtime-b (<rev>), deleted at the
# end (owner sign-off 4). <rev> is checked out in a temporary worktree with its pxui pointer (cloned from the
# local pxui checkout, no network) and a symlink to target/sdl; the worktree is removed at the end.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
REV=""
ROUNDS=3
while [ $# -gt 0 ]; do
  case "$1" in
    --compare) REV=$2; shift 2 ;;
    --rounds) ROUNDS=$2; shift 2 ;;
    *) echo "usage: $0 [--compare <rev>] [--rounds N]" >&2; exit 2 ;;
  esac
done
unset CARGO_BUILD_JOBS CARGO_TARGET_DIR
TA="$ROOT/target/buildtime-a"
TB="$ROOT/target/buildtime-b"
WT=""
OUT=$(mktemp -d)
cleanup() {
  rm -rf "$TA" "$TB" "$OUT"
  if [ -n "$WT" ]; then git -C "$ROOT" worktree remove --force "$WT" 2>/dev/null || rm -rf "$WT"; git -C "$ROOT" worktree prune; fi
}
trap cleanup EXIT

if [ -n "$REV" ]; then
  WT=$(mktemp -d "${TMPDIR:-/tmp}/buildtime-wt.XXXXXX")
  rmdir "$WT"
  git -C "$ROOT" worktree add -q --detach "$WT" "$REV"
  touch "$WT/.metadata_never_index"
  git -C "$WT" submodule init -q pxui
  git -C "$WT" config submodule.pxui.url "$ROOT/pxui"
  git -C "$WT" -c protocol.file.allow=always submodule update -q pxui
  mkdir -p "$WT/target"
  ln -s "$ROOT/target/sdl" "$WT/target/sdl"
fi

# rec <id> <label> <dir> <target dir> <cargo args...>: appends "<id> <wall seconds>" to $OUT/<label>.
rec() {
  local id=$1 label=$2 dir=$3 target=$4 start end
  shift 4
  start=$(python3 -c 'import time; print(time.monotonic())')
  if ! (cd "$dir" && CARGO_TARGET_DIR=$target cargo "$@") >"$OUT/log" 2>&1; then
    cat "$OUT/log" >&2; echo "buildtime: failed in $dir: cargo $*" >&2; exit 1
  fi
  end=$(python3 -c 'import time; print(time.monotonic())')
  python3 -c "print('$id', f'{$end - $start:.2f}')" >>"$OUT/$label"
}

# quiet <dir> <target dir> <cargo args...>: an untimed warm-up step.
quiet() {
  local dir=$1 target=$2
  shift 2
  (cd "$dir" && CARGO_TARGET_DIR=$target cargo "$@") >"$OUT/log" 2>&1 || { cat "$OUT/log" >&2; exit 1; }
}

# measure <tree> <target root> <label>: one round of a..f.
measure() {
  local tree=$1 px=$2/px lw=$2/lw label=$3 library_name
  rm -rf "$px"
  rec a "$label" "$tree/pxui" "$px" build -q --workspace
  quiet "$tree/pxui" "$px" check -q -p pxui_core
  touch "$tree/pxui/crates/private/pxui_core/src/lib.rs"
  rec b "$label" "$tree/pxui" "$px" check -q -p pxui_core
  rm -rf "$px"
  rec c "$label" "$tree/pxui" "$px" build -q -p pxui_dylib
  rm -rf "$px"
  # lw_bin links the dynamic libpxui from its target dir, as `make lib` provides it.
  quiet "$tree" "$lw" build -q --manifest-path pxui/Cargo.toml -p pxui_dylib
  # The comparison tree may link an older major; stage the install name its own library declares.
  library_name=$(otool -D "$lw/debug/libpxui.dylib" | tail -n 1)
  ln -sfn libpxui.dylib "$lw/debug/${library_name##*/}"
  quiet "$tree" "$lw" build -q -p lw_ui
  quiet "$tree" "$lw" clean -q -p lw_ui
  rec d "$label" "$tree" "$lw" build -q -p lw_ui
  quiet "$tree" "$lw" check -q -p lw_ui
  touch "$tree/crates/lw_ui/src/shell/sidebar.rs"
  rec e "$label" "$tree" "$lw" check -q -p lw_ui
  quiet "$tree" "$lw" build -q -p lw_bin
  touch "$tree/crates/lw_ui/src/shell/sidebar.rs"
  rec f "$label" "$tree" "$lw" build -q -p lw_bin
}

for round in $(seq 1 "$ROUNDS"); do
  echo "buildtime: round $round/$ROUNDS HEAD" >&2
  measure "$ROOT" "$TA" head
  if [ -n "$REV" ]; then
    echo "buildtime: round $round/$ROUNDS $REV" >&2
    measure "$WT" "$TB" rev
  fi
done

python3 - "$OUT" "${REV:-}" <<'EOF'
import os, sys
out, rev = sys.argv[1], sys.argv[2]
names = {"a": "clean pxui workspace build", "b": "warm check pxui_core", "c": "clean build pxui_dylib",
         "d": "clean build lw_ui (deps prebuilt)", "e": "warm check lw_ui", "f": "warm build lw_bin"}
budget = {"a": 30.0, "b": 1.0}
def load(label):
    best = {}
    path = os.path.join(out, label)
    for line in open(path) if os.path.exists(path) else []:
        k, v = line.split()
        best[k] = min(best.get(k, float("inf")), float(v))
    return best
head, base = load("head"), load("rev")
over = 0
for k in "abcdef":
    line = f"{k} {names[k]:<34} HEAD {head[k]:7.2f} s"
    if k in budget and head[k] >= budget[k]:
        line += f"  (over the {budget[k]:g} s budget)"
    if rev:
        allow = max(base[k] * 0.05, 0.3)
        line += f"  {rev} {base[k]:7.2f} s  delta {head[k] - base[k]:+.2f} s"
        if head[k] - base[k] > allow:
            line += f"  OVER allowance {allow:.2f} s"
            over += 1
    print(line)
sys.exit(1 if over else 0)
EOF
