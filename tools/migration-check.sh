#!/bin/sh
# Copies the Swift macOS app's data (read-only access to the source) into /tmp and runs the Rust
# migration check against the COPY. Prints counts and pass/fail only. Never launches either app.
# Usage: tools/migration-check.sh [container-data-dir]   (default: the shipping sandbox container)
set -eu
cd "$(dirname "$0")/.."
source=${1:-$HOME/Library/Containers/com.olegpustovit.localwave/Data}
domain=com.olegpustovit.localwave
test -f "$source/Documents/musicApp29.sqlite" || { echo "No Swift container at the given location"; exit 2; }
copy=/tmp/localwave-migration-$(date +%Y%m%d%H%M%S)
mkdir -p "$copy/Documents" "$copy/prefs"
for file in "$source"/Documents/musicApp29.sqlite*; do cp "$file" "$copy/Documents/"; done
cp -R "$source/Documents/CoverArt" "$copy/Documents/" 2>/dev/null || true
cp "$source/Library/Preferences/$domain.plist" "$copy/prefs/"
echo "copy: $copy (songs' audio files are not copied)"
echo "artwork files copied: $(find "$copy/Documents/CoverArt" -type f 2>/dev/null | wc -l | tr -d ' ')"
echo "defaults keys: $(python3 -I -c 'import plistlib,sys; print(" ".join(sorted(plistlib.load(open(sys.argv[1],"rb")))))' "$copy/prefs/$domain.plist" | tr ' ' '\n' | grep -v '^[0-9A-Fa-f-]\{8,\}' | grep -Ei '^(lw|current|player)|Queue|Volume' | tr '\n' ' ')"
cargo build -q -p lw_core --example migration_check
check=target/debug/examples/migration_check
snapshot() { sqlite3 "$copy/Documents/musicApp29.sqlite" .dump | shasum | cut -c1-12; }
prefs_before=$(shasum "$copy/prefs/$domain.plist" | cut -c1-12)
before=$(snapshot)
"$check" "$copy"
first=$(snapshot)
"$check" "$copy" >/dev/null
second=$(snapshot)
echo "database dump before/after first run: $([ "$before" = "$first" ] && echo identical || echo "changed (additive Rust migrations only)")"
[ "$first" = "$second" ] && echo "PASS idempotent: second run left the database dump unchanged" || { echo "FAIL second run changed the database"; exit 1; }
[ "$prefs_before" = "$(shasum "$copy/prefs/$domain.plist" | cut -c1-12)" ] && echo "PASS preferences plist byte-identical after runs" || { echo "FAIL preferences changed"; exit 1; }
