#!/bin/sh
# Mac App Store / TestFlight package for the shipping bundle ID: release bundle signed with Apple Distribution,
# the Mac App Store provisioning profile embedded, the sandbox entitlements plus the store's identifiers, and a
# .pkg signed with the installer identity. UPLOAD=1 sends it to App Store Connect with the API key.
set -eu
cd "$(dirname "$0")/.."
: "${MARKETING_VERSION:?set the marketing version}" "${BUILD_NUMBER:?set a build number above the latest uploaded}"
: "${PROVISIONING_PROFILE:?set the Mac App Store provisioning profile for com.olegpustovit.localwave}"
team=7ZH3FYV964
bundle_id=com.olegpustovit.localwave
app_identity=${APP_IDENTITY:-Apple Distribution}
installer_identity=${INSTALLER_IDENTITY:-3rd Party Mac Developer Installer}
entitlements=$(mktemp -d)/store.entitlements
cp localwave/mac.entitlements "$entitlements"
/usr/libexec/PlistBuddy -c "Add :com.apple.application-identifier string $team.$bundle_id" \
    -c "Add :com.apple.developer.team-identifier string $team" "$entitlements"
CODESIGN_IDENTITY=$app_identity ENTITLEMENTS=$entitlements PROVISIONING_PROFILE=$PROVISIONING_PROFILE make bundle PROFILE=release
app=target/bundle/LocalWave.app
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app/Contents/Info.plist")" = "$bundle_id"
signed_entitlements=$(codesign -d --entitlements - --xml "$app" 2>/dev/null)
case "$signed_entitlements" in *"$team.$bundle_id"*) ;; *) echo "Store identifiers missing from the signature" >&2; exit 1 ;; esac
codesign --verify --strict "$app/Contents/Frameworks/libpxui.2.dylib"
pkg=target/bundle/LocalWave-$MARKETING_VERSION-$BUILD_NUMBER.pkg
productbuild --component "$app" /Applications --sign "$installer_identity" "$pkg"
pkgutil --check-signature "$pkg"
if [ "${UPLOAD:-0}" = 1 ]; then
    : "${ASC_KEY_ID:?}" "${ASC_ISSUER_ID:?}"
    # The app record is matched by its App Store Connect ID: macOS was added to an existing iOS app.
    xcrun altool --upload-app -f "$pkg" -t macos --apple-id "${ASC_APP_ID:-6830623498}" --apiKey "$ASC_KEY_ID" --apiIssuer "$ASC_ISSUER_ID"
fi
echo "$pkg"
