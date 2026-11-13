#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build -p lw_core --example database_exchange
cargo rustc -p lw_apple_sys --features interop-test --lib --crate-type cdylib
LOCALWAVE_RUST_EXCHANGE="$PWD/target/debug/examples/database_exchange" \
LOCALWAVE_RUST_BOOKMARKS="$PWD/target/debug/liblw_apple_sys.dylib" \
    swift test --filter RustCompatibilityTests
