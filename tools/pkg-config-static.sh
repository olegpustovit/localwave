#!/bin/sh
set -eu
flags=$(pkg-config --static "$@")
# sdl3_sys propagates framework libraries, while raw -Wl arguments stop at its rlib.
printf '%s\n' "$flags" | sed -E 's/-Wl,-(weak_)?framework,([^ ]+)/-framework \2/g'
