# LocalWave

A Rust music player (on the pxui UI framework) for MP3, FLAC and other supported audio files on iPhone, iPad and native macOS. Choose music from Files or Finder. Browse your library by artist, album or song, search it, and play tracks without an Apple Music subscription.

![LocalWave album library, source-folder import and player](https://olegpustovit.com/posts/why-i-built-a-native-mp3-player-in-swiftui/frame_source.webp)

*Historical screenshots of the former SwiftUI app from the [development article](https://olegpustovit.com/posts/why-i-built-a-native-mp3-player-in-swiftui/). The Rust app keeps that app's instrument-style design, [redesign version 4](https://artifacts.microapps.space/a/mh0ydns8mzyg), with bundled Host Grotesk/Geist Mono, Canvas matrix numerals and audio-derived waveforms.*

## Build and try it

The project is Rust only; build it as described under [Rust app](#rust-app) and [Rust iOS development](#rust-ios-development). The former SwiftUI app (Xcode project, Swift tests, TestFlight/DMG release workflow) is archived on the [`swift` branch](https://github.com/olegpustovit/localwave/tree/swift).

Importing an iCloud source can require a network connection to download its files. Local playback depends on the selected files being available and accessible; this is not a promise that every cloud-backed file works offline.

An empty library offers **Try the demo library**: 92 real synthesized recordings across 11 albums, imported as one local source. Existing libraries offer **Explore the library** and are not changed by that action. The bundled audio/catalog adds 101.5 MiB to the app; choosing the demo creates its local source files. Regenerate or verify them with [generate_demo_library.py](scripts/generate_demo_library.py).

## What is in the source

The app includes folder import, metadata and artwork, artist/album/song views, playlists, a playback queue, shuffle/repeat controls and lock-screen remote commands. Background audio is declared in the app's configuration. SQLite FTS5 provides indexed song and path search. Find preserves library order among matching songs; repository queries also support BM25 ranking.

FLAC imports read Vorbis comments (including artist, title, album artist, year, track and disc) and embedded cover art. On iPhone, device volume is controlled with hardware buttons or Control Center. Favorites use the heart action. Kept-offline pins protect cached copies from the configurable LRU cache limit; releasing a pin keeps its copy until space is needed. System appearance, signal color and reduced motion are available in Settings.

M4A imports also read native album-artist, release-date, track and disc tags, including the binary track/disc fields.

Automated checks cover shared database upgrades, Favorites persistence, recursive local FLAC import and native decoding. Physical-device audio routes, iCloud downloads, and long-running background playback still require device testing. Known evicted iCloud items and recognized provider locations are indexed without reading audio; metadata is enriched from a verified materialized copy. Generic providers expose no universal public download status, so unconfirmed availability remains unknown and provider downloads show indeterminate progress.

The [redesign qualification record](docs/redesign-qualification.md) documents the earlier iOS/Catalyst shell; [Mac list qualification](docs/perf-mac-list.md) records the native Mac checks and outstanding performance measurements.

## Rust app

The [Rust port plan](https://artifacts.microapps.space/a/w474jy87pnxg) is being implemented in `crates/`, with pxui pinned as a submodule. The SwiftUI app it replaces is archived on the `swift` branch. The terminal player has been removed. The Rust window restores a paused queue and shows a worker-prepared library with sidebar navigation, virtual song rows, album/artist browsing, a Now panel and transport controls. Double-click a song to play. Find searches song text; click Title, Artist, Album or Time to sort, or # to restore library order. Find supports format tokens (flac/mp3/m4a/wav/aiff), fav/favorites, year:2019, unplayed, cloud and offline; the visible chips toggle common filters. Other Find result sections remain unfinished. Native font fallback now renders the tested multilingual tags and emoji; first-time font discovery may allocate. System appearance and reduced motion now update through native notifications, with cached themes and saved settings loaded on the worker. Use the Refresh library button to load database changes without restarting; current selection follows song/collection identity. Automatic source observation and the complete reference UI remain unfinished.

The Rust window supports resizing and native zoom/full-screen controls, with content behind its
transparent title bar. The Queue button opens the full occurrence-based queue editor; Save as
playlist saves its current order with duplicate songs included once. Album, Now panel and queue
artwork load from the existing library on a separate worker, with bounded thumbnail/texture caches.
Image identity, GPU wrapper reuse and completed-frame release live in pxui itself. ABI 1.16 adds a shared frame clock and correct retained-view entrance transforms; held-key queries and fractional waveform fills remain available.
Tracks without readable local cover files keep their placeholder. The transport displays cached/native-decoded waveform peaks; drag to preview and release to seek, or use Left/Right for five seconds and Home/End for the track bounds.
Press **⌘K** or **/** outside a text field for Search and commands. Search results include songs, albums, artists and indexed folders; use ↑/↓ and Return, or click a result. Song results play from the complete ranked search queue. Commands control playback, shuffle/repeat, network simulation, appearance, the player panel and settings. **?** opens keyboard help; Escape, focus loss or an outside click dismisses these panels. Add music opens Files with separate folder/audio picker choices; the native mixed picker, query highlights and some global shortcuts listed in the reference help remain unfinished.

The Rust app now publishes track metadata and worker-decoded covers to macOS Now Playing, with play/pause, toggle, stop, next/previous and position commands routed through the same controller. Physical media-key and Control Center qualification remains open. Playback failures now update saved file state while preserving newer cache/import records; unreadable files retain their paths and are skipped with a notice.
Open Settings from the sidebar or with ⌘, to change download-ahead, simulated offline mode, cache limit and appearance. Right-click the cache choices for 2/32 GB, or appearance for signal colors and reduced motion. About & Privacy opens from Settings.
Open **Files → Add folder** to choose an original music folder. A separate worker indexes and recursively imports its audio, with progress and Cancel. **Add audio files** copies selected tracks into **Added Files**, opens that source and starts the selected tracks in picker order, deduplicated by song identity. Identical files reuse their original name; differing name collisions get a UUID prefix. These private copies play directly and are excluded from cache eviction. Browse indexed subfolders, import individual audio files or whole folders, and use **Rescan source** to refresh the entire source. **Folder options** (also on right-click) offers **Reconnect folder…** and confirmed **Remove source**. Reconnect keeps source/song IDs and Favorites; removal deletes the source's library records and playlist memberships while leaving files on disk. Cancel active source work before reconnecting or removing. Swift account identities and legacy folder rows are preserved; stale account/root actions are rejected. Drop folders and audio files from Finder anywhere in the window. The drop opens Files with progress and Cancel; audio joins Added Files, folders stay in place, and imported tracks play in selection order (selected files first, then naturally sorted folder tracks). Empty folders leave playback unchanged. Wait or cancel active source work before another drop. Automatic source observation remains unfinished. Actual sandbox picker/drop grants and live provider/account transitions still need qualification.
Quit an already running Rust app before opening a rebuilt bundle so macOS launches the new binary.

Find's **Play all** plays its full result set. **Keep offline** queues eligible results on the
library worker; **Kept offline** releases their pins while preserving existing copies until normal
cache eviction. The toolbar uses the SwiftUI button geometry and background animations. The
command palette's worker-side suggestions and ranked playback payloads are prepared, including
owned folders and the top-24 Most played collection. The visible palette and scrolling search
header still need porting; full visual parity is unfinished.

The Rust shell uses the framework's `pxui_host` for window/GPU integration, presentation and image
texture lifetime. LocalWave supplies decoded covers and source/layout policy; it contains no
production Metal or texture management code. The shared app callback runs on macOS and
experimentally on iOS, with lifecycle and appearance monitoring owned by pxui. LocalWave's shared native services and
controller library now compile for iOS device/simulator, with an isolated simulator artwork/
bookmark/callback check. The controller now connects worker-side audio-session activation,
interruptions, route loss and media-service reset; actual device/background behavior remains
unqualified. The full Rust executable now builds for iOS device/simulator and passes an isolated
simulator launch/draw/idle/shutdown check. Below 620 points the UI switches to phone navigation,
virtual lists, a mini-player and an expanded player; macOS can resize down to 320 points. Native
tap/drag/scroll gestures use pxui's shared widget and scroll engine. Long press, swipe actions,
interactive back and physical gesture feel remain unfinished or unqualified. pxui's shared
`content_rect()` follows safe areas and animated docked-keyboard presentation geometry; the compact
shell, command search and playlist naming use it. Repeated show/hide and refocus preserve zero warm
Rust allocations in the simulator probe. Find disables autocorrection/capitalization and uses Done
to dismiss input; command search uses the Search key. Native UIKit fixtures now cover marked text,
Unicode selection, commit/cancel, deletion and Done through pxui. Interactive IME candidates,
microphone dictation and floating-keyboard field avoidance remain unqualified. Native iOS document pickers now transfer their original URL capabilities
to the import worker; real provider grants still need qualification. The full-app signing/install command is implemented;
its simulator path is qualified, while the physical USB path still needs a connected iPhone/iPad.

The app is on **dev**. It requires macOS 15+, Xcode with the macOS 26 SDK, Rust (the pinned toolchain is selected by rustup), CMake and pkg-config. On another Mac:

```sh
git clone --recurse-submodules git@github.com:olegpustovit/localwave.git
cd localwave
make build                  # static SDL3, libpxui, Rust macOS app
make bundle                 # signed sandboxed target/bundle/LocalWave.app
open target/bundle/LocalWave.app
make install                # install to /Applications (APPDIR overrides)
make sign                   # Developer ID (team 7ZH3FYV964) hardened-runtime build, verifies signature + entitlements
xcrun notarytool store-credentials <profile> --apple-id <id> --team-id 7ZH3FYV964   # one time
NOTARY_PROFILE=<profile> make notarize   # submit, staple, spctl
tools/migration-check.sh    # copy the Swift container to /tmp and run the Rust migration check on the copy
cargo t -p lw_sqlite -p lw_core # Rust SQLite, storage and model checks
cargo run -p lw_bin --bin localwave-playback -- --mute /absolute/path/audio.flac
make performance            # twelve signed workloads: allocation/time/idle gates
```

The adaptive checkpoint passes 192 bounded app checks; timings and native qualification are recorded in [the port status](docs/rust-port.md). Songs and Artists use worker-prepared SwiftUI desktop columns, adaptive headings, wrapped numeric cells and measured virtual rows, and shared labels use explicit pxui line boxes with checked CoreText baselines at 1×/2×. Albums has the measured Artist/Year/Size toolbar, correct gallery spacing and functional Shuffle all. Album orders and header summaries are prepared on the worker. Intrinsic text widths and wrapped player bylines match measured native SwiftUI components; artist/album links open their library contexts. Albums and Artists scroll their page headings with the rows; Artists pins its column heading, and overlay scroll layout preserves the content width. Page entrances, changing matrix digits, playing bars and file-state pops animate through retained views and respect reduced motion. Find now displays worker-prepared matching song/album/folder counts; its offline-action service is implemented, with the remaining Find toolbar binding still pending. Full screenshot parity and the remaining reference layouts are unfinished.

All twelve signed 10,000-song workloads record zero warm Rust allocations, including the 10,000-artist table and 10,000-path Files directory. Playing and paused song bars are covered while scrolling. The artwork gallery also has zero warm requests. The strict 1 ms wall maximum still fails; zero allocations refers to warm Rust UI frames, and native/process memory remains unqualified. The dev branch pins the matching published pxui commit (pxui `dev`); clone with submodules, or run `git submodule update --init --recursive` after pulling. See [Rust port status](docs/rust-port.md) for measured results and remaining work.

## Rust iOS development

The full Rust app installs as **LocalWave Dev**, using a separate bundle ID and library so an
installed Swift app stays available. This is experimental: native file picking is connected, while
real provider permissions, touch behavior, keyboard adaptation and phone visual parity remain to qualify.

```sh
make ios-run                            # paired USB iPhone/iPad: build, sign, install, launch
make ios-run IOS_DEVICE=simulator       # explicitly use one booted iPhone/iPad simulator
make ios-build                          # unsigned arm64 device app, without installing
make ios-build IOS_SDK=iphonesimulator  # signed simulator app, without installing
```

Use a Mac with full Xcode selected, Rust/rustup, Python 3, CMake and pkg-config; simulator runs
require Apple Silicon (the produced packages are arm64). Devices require iOS 18 or newer, Developer Mode and trust for this Mac.
Add your Apple account in Xcode Settings → Accounts before the first device run. The command
finds a development team from valid signing identities and lets
[Xcode manage development signing and profiles](https://developer.apple.com/help/account/provisioning-profiles/create-a-development-provisioning-profile).
Apple account setup, keychain unlocks and device trust still require the device/account owner.

If several devices or teams are available, set `IOS_DEVICE=<UDID>` or `IOS_TEAM_ID=<team ID>`.
`IOS_BUNDLE_ID` can select another development identifier; the Swift app's identifier is
refused. iOS targets build release (debug builds rebuild every retained view each frame); `make ios-build PROFILE=debug` selects debug.
Rust/SDL caches stay in `target/`; Xcode packaging uses `target/ios`. Device builds without
signing go into `target/ios/Build/Products/Release-iphoneos-unsigned/LocalWave.app`.

The command builds the actual `lw_bin` executable with static pxui/SDL, compiles the existing app
icon/privacy resources and verifies executable UUID, architecture, platform, deployment floor,
system-only dependencies and signatures before installation. The simulator build/install/launch
path is verified; automatic provisioning, installation and launch on physical hardware remain
unqualified because no iPhone/iPad is connected. An unsigned device build alone is not installable.

## Development

The Rust crates are in [crates](crates) with the pxui framework in the [pxui](pxui) submodule; [AGENTS.md](AGENTS.md) maps them and sets the test budget. Run `make check` before committing; pull-request CI runs the Rust checks.

On macOS, `make soak-smoke` runs three seconds each of playback, scrolling, and resize/theme/menu churn after each phase warms up. `make soak` uses the same checks with longer configurable phases. Both use a separately signed sandbox, a temporary 10,000-song catalog, and a silent repeat-one track. The soak requires promoted playback draws through the pxui compositor. The Metal oracle compares the damage-limited base canvas with a full redraw of the same base.

Set `LOCALWAVE_SOAK_PLAYBACK_SECONDS`, `LOCALWAVE_SOAK_SCROLL_SECONDS`, or `LOCALWAVE_SOAK_CHURN_SECONDS` to change the phase lengths. `LOCALWAVE_SOAK_SAMPLE_SECONDS` defaults to 60 seconds and `LOCALWAVE_SOAK_VERIFY_EVERY` defaults to every 60 presented frames. The runner keeps JSON samples in `target/soak/`. Each phase requires zero warm frame allocations across the app and pxui, byte-exact Metal canvas checks, and RSS and physical footprint ranges within 64 MiB after warm-up. CPU and work-wall p99 are reported without gating. Timing buckets are 100 µs wide through 100 ms and coarser above it.

Report a reproducible problem in [GitHub issues](https://github.com/olegpustovit/localwave/issues), including your iOS version and whether the source is local or iCloud-backed. Do not attach private music or library data.

## License

[MIT](LICENSE). Bundled fonts retain their own licenses. Rust filename cleanup uses Unicode data under the [Unicode License v3](crates/lw_core/UNICODE-LICENSE.txt).
