# Development guide

## Build and test

Read [README.md](README.md) for setup. Rust: `make build`, `make bundle`, `make check`;
`make performance` verifies signed release workloads. `make ios-build` / `make ios-run` build
and run the full Rust iOS app. Use focused `cargo t -p <crate> [filter]` while iterating.
The runner enforces 5 seconds per case and 60 seconds per run; never raise deadlines to hide failures.
Reuse shared `target/`, static SDL under `target/sdl` and the same `pxui/tools/cc-stack.sh`
linker in both workspaces. Keep the default executable stack and `PXUI_CONSUMER_DEFAULT_STACK=1`.

Swift: Xcode 16.2+ (CI uses 26.6), shared `musicapp` scheme, `localwave.xctestplan`, Swift 5
language mode, iOS 18 app floor and iOS 18.2+ test simulator. Tests retain `musicappTests` /
`musicappUITests` names; target names do not imply broad UI coverage. Shared cases use Swift Testing.

| Loop | Command | Warm wall budget |
| --- | --- | --- |
| Rust | `cargo t -p lw_sqlite -p lw_core -p lw_apple_sys -p lw_ui -p lw_bin` | 60 s/run, 5 s/case |
| Swift shared | `swift test --parallel` | 30 s |
| iOS logic changes | `xcodebuild … -only-testing:musicappTests test` | 90 s |
| UI flows | Full simulator plan, manual Verify workflow | On demand |

Unit tests use injected clocks or short-deadline polling, no sleeps/network, and shared fixtures.
A case exceeding one second needs a reason comment or belongs in UI verification. Fix or move
slow cases before merging. PR CI runs shared/unit checks; release tags run Swift shared checks.
Use `scripts/build-unsigned-ipa.sh` for the shipping unsigned device IPA.

## Source map

- `lw_sqlite`: system SQLite FFI, statement lifetimes and transaction rollback; no Apple dependency.
- `lw_core`: models, SQLite repositories, FTS5/BM25, import, cache and playback policy.
- `lw_apple_sys`: native Apple FFI, bookmarks, playback, preferences and coordinated file services.
- `lw_ui`: screens, app bodies, theme/motion tokens and navigation state through public pxui.
- `lw_bin`: executable, controller and library/files/artwork workers; GPU policy adapter.
- `pxui`: pinned framework submodule; read its AGENTS.md before editing it.
- `localwave/Sources`: shipping Swift App, Core, Domain, Data and Features; shared modules live
  in Package.swift and must not be re-added to the Xcode app target.

## Boundary

LocalWave owns music policy, screens, application bodies, decoded RGBA and source/size selection.
pxui owns reusable layout, widgets, editing, animation, text, rendering and touch physics.
`pxui_host::Graphics` owns GPU setup, frames, uploads, bounded caches and GPU-safe retirement;
`pxui_sdl::Chrome` owns generic native chrome. Never restore app-owned frame/texture machinery.

Allowed production dependencies (plus std):

- `lw_sqlite`: std only, linked to system sqlite3.
- `lw_core`: `lw_sqlite`; Apple import fixtures use `lw_apple_sys` only in dev-dependencies.
- `lw_apple_sys`: `file_access_sys`, `lw_sqlite` (existing public re-export); `cc` for native builds.
- `lw_ui`: `lw_core`, public `pxui`, `input`; no private engine/GPU imports or unsafe code.
- `lw_bin`: application crates including `lw_sqlite`, `pxui`, `pxui_sdl`, `pxui_host`, `input`, `mem_sys`.
  Existing GPU test-only dependencies are removed by B7; add none to production.

## Performance contract

Warm input/build/draw frames allocate zero Rust heap objects across app and libpxui; idle draws
zero frames. Keep UI frame builds below 1 ms at 10,000 songs. Preserve existing wall assertions
until the plan's explicit performance step changes them; CPU time never silently replaces wall time.
No per-frame formatting, sorting, string hashing, decoding or fresh resource wrappers. Use retained
views, virtualization, cached strings/widths, fixed-capacity messages and bounded caches/queues.
Workers own DB, filesystem and decoding work. Production fonts are Host Grotesk/Geist Mono.

Run performance checks after host/render changes and every PERF step. Preserve representative
workloads, failure evidence and allocator coverage on both sides of the ABI. Native/process
allocations are separate and require profiling; zero warm Rust allocations does not qualify them.
Never drop input or extend warm-up merely to pass an allocation check. Device/provider behavior
requires direct qualification, not synthetic or local-file fixtures.

## Threading

The initial thread owns UI, native player, window and GPU operations. Transfer worker-prepared
native items before installation; headless hosts service the main run loop. No I/O in frames.

- `LibraryWorker`: serialized database, playback preparation, preferences, waveforms and offline cache.
- `FilesWorker`: lazy independent library connection for browsing, scans, import and source mutations.
- `ArtworkWorker`: persistent bounded thumbnail decoding and worker-side pixel retirement.

Specific lifetime, cancellation, transaction and handoff contracts live in module `//!` comments.
Keep current library/player snapshots immutable across delivery and retire replaced storage on workers.

## Data compatibility

Preserve Swift-compatible SQLite schemas/date encodings, defaults keys, bookmarks, hashes and
source/account identity. Keep queue occurrence order, Favorites and playlist membership intact.
Foundation context conversion and native preferences belong on workers. Preserve unknown saved
context bytes until edited. Source access uses security-scoped grants and coordinated reads;
never rebuild an iOS provider capability from a portable path. Never delete original audio during
cache eviction, unpin or source removal. Validate complete external requests before side effects.
Keep shipping Swift apps until replacement platform/distribution gates pass; Rust development
packages use isolated containers. Smoke/performance fixtures never touch normal data or media sessions.

## Process

The [implementation plan](https://artifacts.microapps.space/a/w474jy87pnxg#plan) defines ordered
steps; [docs/rust-port.md](docs/rust-port.md) is the status. Finish C before B, B before P, P before F;
then follow shipping/iOS gates. Stop at a blocked step rather than jumping phases.

One step per commit; subjects include its ID. For framework steps publish pxui on `rust-port`
before staging its pointer in LocalWave, also on `rust-port`; check remote fetchability.
Before committing, require the bounded Rust suite and `make check` green. PERF steps also run
`make performance` and replace the status performance table. Record evidence in commit messages;
status holds one row per step, never checkpoint journals. ABI history lives in `pxui/docs/abi.md`.

Refactors move code verbatim and adjust paths/visibility only. Record discovered bugs for separate
fixes. Feature behavior precedes pixel polish. Read implementations/callers before changing behavior,
finish affected callers/tests/fixtures/config/exports, preserve trust-boundary validation, errors,
accessibility and ownership. Nontrivial logic needs a focused runnable check. No new dependencies,
abstractions or flexibility without a demonstrated requirement. Deferred platform checks stay explicit.

File and function length are review signals, not hard limits. Split when ownership, responsibility
or readability benefits; keep cohesive functions and tests together. Do not add line-count gates
or extract helpers solely to meet numeric caps. Performance and test-time gates still apply.
