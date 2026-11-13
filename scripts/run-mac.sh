#!/bin/bash
# Builds the Mac app (incremental) signed with your Apple Development identity, then runs it in the foreground (logs in this terminal and build/mac-run.log) or installs it.
# CONFIG=Debug|Release (default Debug); INSTALL=1 copies to /Applications instead of running; SIGN_IDENTITY overrides the identity.
# Every config uses the same bundle id, sandbox container and signature, so they share one database and security-scoped bookmarks stay valid.
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
config="${CONFIG:-Debug}"
derived="$repo_dir/build/mac-run-$config"
identity="${SIGN_IDENTITY:-$(security find-identity -v -p codesigning | grep -oE '"Apple Development[^"]*"' | head -1 | tr -d '"')}"
[ -n "$identity" ] || { echo "error: no Apple Development identity; set SIGN_IDENTITY" >&2; exit 1; }
mkdir -p "$repo_dir/build"

xcodebuild -project "$repo_dir/localwave.xcodeproj" -scheme musicapp \
  -configuration "$config" -destination 'platform=macOS' -derivedDataPath "$derived" \
  CODE_SIGN_IDENTITY="$identity" CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM= -quiet build

app="$derived/Build/Products/$config/LocalWave.app"
# Two copies running on one SQLite file is asking for trouble: quit any running LocalWave first.
pkill -x LocalWave && sleep 0.5 || true

if [ "${INSTALL:-}" = 1 ]; then
  dest=/Applications; [ -w "$dest" ] || dest="$HOME/Applications"
  mkdir -p "$dest"; rm -rf "$dest/LocalWave.app"; ditto "$app" "$dest/LocalWave.app"
  codesign --verify --strict "$dest/LocalWave.app"
  echo "Installed $dest/LocalWave.app"
  exit 0
fi
exec "$app/Contents/MacOS/LocalWave" 2>&1 | tee "$repo_dir/build/mac-run.log"
