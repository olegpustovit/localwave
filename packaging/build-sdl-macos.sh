#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
version=3.4.18
sha=9c75cf16330322c217dedd2e0609f1124f1b54b8633e763467b4684d0f4334a3
archive="target/downloads/SDL3-$version.tar.gz"
mkdir -p target/downloads
if [ ! -f "$archive" ]; then
    curl --fail --location --retry 2 "https://www.libsdl.org/release/SDL3-$version.tar.gz" -o "$archive.part"
    mv "$archive.part" "$archive"
fi
echo "$sha  $archive" | shasum -a 256 --check
if [ ! -d "target/SDL3-$version" ]; then tar -xzf "$archive" -C target; fi
cmake -S "target/SDL3-$version" -B target/sdl-build \
    -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$PWD/target/sdl" \
    -DCMAKE_OSX_DEPLOYMENT_TARGET=15.0 -DCMAKE_INSTALL_LIBDIR=lib \
    -DSDL_SHARED=OFF -DSDL_STATIC=ON -DSDL_TESTS=OFF -DSDL_TEST_LIBRARY=OFF \
    -DSDL_EXAMPLES=OFF
cmake --build target/sdl-build --parallel "$(sysctl -n hw.ncpu)"
cmake --install target/sdl-build
