# LocalWave development in VS Code

Install the recommended Swift, Rust Analyzer and CodeLLDB extensions. The Swift app remains available through Xcode while the Rust port progresses.

The default build task runs `make build PROFILE=debug`, which builds static SDL3, libpxui and the Rust executable. F5 starts **Debug LocalWave (Rust)**. `rust-toolchain.toml` pins the compiler. Use `cargo t -p lw_core -p lw_apple_sys -p lw_ui` for bounded Rust checks.

The `swift-build-all` and `swift-test` tasks build and check the existing shared Swift modules. SourceKit-LSP uses the Swift package. If its index needs refreshing, run `swift build --build-tests`, then reload the window.

Build and qualification details are in [the Rust port record](../docs/rust-port.md).
