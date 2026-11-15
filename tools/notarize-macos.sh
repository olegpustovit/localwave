#!/bin/sh
# Notarizes and staples the signed app from tools/sign-macos.sh. One-time credential setup (prompts
# for an app-specific password; never stored in the repo):
#   xcrun notarytool store-credentials "$NOTARY_PROFILE" --apple-id <id> --team-id 7ZH3FYV964
set -eu
cd "$(dirname "$0")/.."
: "${NOTARY_PROFILE:?Set NOTARY_PROFILE to the notarytool keychain profile name}"
app=target/bundle/LocalWave.app
zip=target/bundle/LocalWave-notarize.zip
codesign --verify --deep --strict "$app"
rm -f "$zip"
ditto -c -k --keepParent "$app" "$zip"
xcrun notarytool submit "$zip" --keychain-profile "$NOTARY_PROFILE" --wait
xcrun stapler staple "$app"
xcrun stapler validate "$app"
spctl -a -t exec -vv "$app"
