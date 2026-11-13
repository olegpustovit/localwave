#!/bin/sh
# Refuses a pxui/ submodule commit that is not on the PXUI remote: a LocalWave commit pointing at a PXUI
# commit nobody else can fetch is unbuildable for everyone else. Run by `cargo run -p gate -- pxui|all`.
#   LOCALWAVE_OFFLINE=1 tools/check-pxui-remote.sh   skip (prints that nothing was verified)
#   tools/check-pxui-remote.sh --self-test      the check against two throwaway repositories
set -eu

check() { # check <localwave-root>
    root=$1
    [ -e "$root/pxui/.git" ] || { echo "pxui remote check: no pxui/ submodule, nothing to check"; return 0; }
    sha=$(git -C "$root" rev-parse :pxui) || { echo "pxui remote check: pxui/ is not recorded in the index" >&2; return 1; }
    if [ "${LOCALWAVE_OFFLINE:-}" = 1 ]; then
        echo "pxui remote check SKIPPED (LOCALWAVE_OFFLINE=1): $sha is NOT verified against the PXUI remote"
        return 0
    fi
    git -C "$root/pxui" cat-file -e "$sha^{commit}" 2>/dev/null ||
        { echo "pxui remote check: $sha is not in pxui/ (git submodule update --init pxui)" >&2; return 1; }
    git -C "$root/pxui" fetch --quiet origin ||
        { echo "pxui remote check: cannot reach the PXUI remote; set LOCALWAVE_OFFLINE=1 to skip when offline" >&2; return 1; }
    [ -n "$(git -C "$root/pxui" for-each-ref --contains "$sha" --format=x refs/remotes/origin)" ] ||
        { echo "pxui remote check: $sha is not on any PXUI remote branch; push pxui/ first, then commit the pointer" >&2; return 1; }
    echo "pxui remote check: $sha is on the PXUI remote"
}

if [ "${1:-}" = --self-test ]; then
    t=$(mktemp -d); trap 'rm -rf "$t"' EXIT
    g() { git -c user.name=t -c user.email=t@t -c protocol.file.allow=always "$@"; }
    g init -q "$t/remote"; g -C "$t/remote" commit -q --allow-empty -m one
    g init -q "$t/localwave"; g -C "$t/localwave" submodule add -q "$t/remote" pxui
    check "$t/localwave" >/dev/null || { echo "self-test: pushed commit refused" >&2; exit 1; }
    g -C "$t/localwave/pxui" commit -q --allow-empty -m local; g -C "$t/localwave" add pxui
    if check "$t/localwave" >/dev/null 2>&1; then echo "self-test: unpushed commit accepted" >&2; exit 1; fi
    echo "self-test ok"; exit 0
fi
check "$(cd "$(dirname "$0")/.." && pwd)"
