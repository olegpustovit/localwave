#!/bin/bash
# Builds and signs LocalWave.app with the Developer ID Application identity of $TEAM_ID (hardened
# runtime), then verifies the signature, entitlements and Gatekeeper assessment (unnotarized until
# tools/notarize-macos.sh runs). A locked login keychain fails at codesign: run
#   security unlock-keychain ~/Library/Keychains/login.keychain-db
set -eu
cd "$(dirname "$0")/.."
team=${TEAM_ID:-7ZH3FYV964}
products=${1:-target/release}
identity=$(security find-identity -v -p codesigning | awk -v team="($team)" '/Developer ID Application/ && index($0, team) { print $2; exit }')
test -n "$identity" || { echo "No Developer ID Application identity for team $team in the keychain" >&2; exit 1; }
CODESIGN_IDENTITY=$identity packaging/bundle-macos.sh "$products"
app=target/bundle/LocalWave.app
codesign --verify --deep --strict --verbose=2 "$app"
codesign -dvv "$app" 2>&1 | grep -E 'Identifier|flags|TeamIdentifier|Authority=Developer ID Application' | sed 's/^Authority=Developer ID Application:.*/Authority=Developer ID Application (name omitted)/'
# The shipping Swift macOS target signs with the same file, so equality is the entitlement/sandbox parity check.
codesign -d --entitlements - --xml "$app" 2>/dev/null > target/bundle/signed.entitlements
plutil -convert xml1 -o target/bundle/expected.entitlements localwave/mac.entitlements
diff <(plutil -p target/bundle/signed.entitlements) <(plutil -p target/bundle/expected.entitlements) && echo "entitlements match localwave/mac.entitlements (app-sandbox, user-selected read-only, app-scope bookmarks)"
spctl -a -t exec -vv "$app" || echo "spctl rejects the app until it is notarized (expected before tools/notarize-macos.sh)"
