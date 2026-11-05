#!/bin/bash
# Map prerelease tags to Apple's numeric marketing version, with monotonic CI builds.
set -euo pipefail
version="${RELEASE_VERSION:-1.0.0}"
if [[ "${GITHUB_REF_TYPE:-}" == tag ]]; then
  version="${GITHUB_REF_NAME#v}"
  version="${version%%[-+]*}"
fi
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  echo "::error::Release version must contain three numeric components" >&2; exit 1;
}
run="${GITHUB_RUN_NUMBER:?Missing GitHub run number}"
attempt="${GITHUB_RUN_ATTEMPT:?Missing GitHub run attempt}"
# Keep within Apple's four-digit major and two-digit minor build components.
[[ "$run" =~ ^[1-9][0-9]{0,3}$ && "$attempt" =~ ^[1-9][0-9]?$ ]] || {
  echo "::error::CI build exceeds the supported bundle version range" >&2; exit 1;
}
printf 'version=%s\nbuild=%s.%s\n' "$version" "$run" "$attempt"
