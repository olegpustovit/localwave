#!/bin/bash
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
output_dir="${1:-$repo_dir/build/unsigned}"
mkdir -p "$output_dir"
output_dir="$(cd "$output_dir" && pwd)"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/localwave-ipa.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

xcodebuild -project "$repo_dir/localwave.xcodeproj" -scheme musicapp \
  -configuration Release -destination 'generic/platform=iOS' \
  -derivedDataPath "$work_dir/DerivedData" \
  CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO build

mkdir -p "$work_dir/package/Payload"
ditto --norsrc "$work_dir/DerivedData/Build/Products/Release-iphoneos/localwave.app" \
  "$work_dir/package/Payload/localwave.app"
ditto --norsrc -c -k --keepParent "$work_dir/package/Payload" "$output_dir/LocalWave-unsigned.ipa"
(cd "$output_dir" && shasum -a 256 LocalWave-unsigned.ipa > LocalWave-unsigned.ipa.sha256)
echo "Unsigned IPA: $output_dir/LocalWave-unsigned.ipa"
