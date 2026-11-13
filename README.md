# LocalWave

A SwiftUI music player for MP3, FLAC and other supported audio files on iPhone, iPad and native macOS. Choose music from Files or Finder. Browse your library by artist, album or song, search it, and play tracks without an Apple Music subscription.

![LocalWave album library, source-folder import and player](https://olegpustovit.com/posts/why-i-built-a-native-mp3-player-in-swiftui/frame_source.webp)

*Historical screenshots from the [development article](https://olegpustovit.com/posts/why-i-built-a-native-mp3-player-in-swiftui/). The current instrument-style SwiftUI port follows [redesign version 4](https://artifacts.microapps.space/a/mh0ydns8mzyg), with bundled Host Grotesk/Geist Mono, Canvas matrix numerals and audio-derived waveforms.*

## Build and try it

Use a Mac with Xcode 16.2 or newer and the iOS SDK. The app target requires iOS 18.0; the project defaults and unit-test target use iOS 18.2. Choose an iOS 18.2 or newer simulator to use the checked-in test plan. The targets use Swift 5 language mode.

```sh
git clone https://github.com/olegpustovit/localwave.git
cd localwave
open localwave.xcodeproj
```

1. Select the shared **musicapp** scheme and an iOS 18.2 or newer simulator. The scheme builds the **localwave** app.
2. Let Xcode resolve the Swift packages. For a physical device, open the **localwave** target’s **Signing & Capabilities**, enable **Automatically manage signing**, and select your own team (a Personal Team works for development). Change **Bundle Identifier** to a unique reverse-DNS value such as `com.yourname.localwave`; the checked-in identifier belongs to the project and may not be available to your account. Simulator builds do not need a signing team.
3. Choose **Product > Run**. On a physical iPhone, enable **Settings > Privacy & Security > Developer Mode** if requested, restart, and confirm it. If iOS reports an untrusted developer, open **Settings > General > VPN & Device Management**, select your developer profile, and trust it. Keep the iPhone unlocked and accept the Mac’s pairing prompt. Personal Team builds expire and need to be re-signed according to your account’s limits.
4. Open **Folders**, tap **Add music**, and choose folders or audio files. Folders scan and import automatically; individually selected audio is copied into **Added Files**.
5. Open **Library**, choose a song, and use the bottom deck or Now playing panel. **Lists** holds playlists and automatic collections; **Find** searches across sources.
6. On Mac, select **My Mac** in the same **musicapp** scheme. The native macOS app uses a sidebar, sortable song table, folder browser and transport bar. Drop folders or audio files into the window, or use **Add Folder…** and **Add Audio Files…**. The app requires macOS 15 or newer.

Importing an iCloud source can require a network connection to download its files. Local playback depends on the selected files being available and accessible; this is not a promise that every cloud-backed file works offline.

An empty library offers **Try the demo library**: 92 real synthesized recordings across 11 albums, imported as one local source. Existing libraries offer **Explore the library** and are not changed by that action. The bundled audio/catalog adds 101.5 MiB to the app; choosing the demo creates its local source files. Regenerate or verify them with [generate_demo_library.py](scripts/generate_demo_library.py).

## What is in the source

The app includes folder import, metadata and artwork, artist/album/song views, playlists, a playback queue, shuffle/repeat controls and lock-screen remote commands. Background audio is declared in the app's configuration. SQLite FTS5 provides indexed song and path search. Find preserves library order among matching songs; repository queries also support BM25 ranking.

FLAC imports read Vorbis comments (including artist, title, album artist, year, track and disc) and embedded cover art. On iPhone, device volume is controlled with hardware buttons or Control Center. Favorites use the heart action. Kept-offline pins protect cached copies from the configurable LRU cache limit; releasing a pin keeps its copy until space is needed. System appearance, signal color and reduced motion are available in Settings.

M4A imports also read native album-artist, release-date, track and disc tags, including the binary track/disc fields.

Automated checks cover shared database upgrades, Favorites persistence, recursive local FLAC import and native decoding. Physical-device audio routes, iCloud downloads, and long-running background playback still require device testing. Known evicted iCloud items and recognized provider locations are indexed without reading audio; metadata is enriched from a verified materialized copy. Generic providers expose no universal public download status, so unconfirmed availability remains unknown and provider downloads show indeterminate progress.

The [redesign qualification record](docs/redesign-qualification.md) documents the earlier iOS/Catalyst shell; [Mac list qualification](docs/perf-mac-list.md) records the native Mac checks and outstanding performance measurements.

## Unsigned IPA

Download the unsigned device IPA and checksum from [Releases](https://github.com/olegpustovit/localwave/releases). It must be signed using your chosen iOS sideloading tool before installation; it cannot install directly. Your signing account and tool determine expiry and renewal requirements.

To produce the same package locally:

```sh
bash scripts/build-unsigned-ipa.sh /tmp/localwave-unsigned
```

Pull-request CI also uploads an unsigned IPA as a workflow artifact. Pushing a tag such as `v1.0.1` runs the **Release** workflow: it runs `swift test --parallel`, uploads a signed build to internal TestFlight, and publishes a Developer ID signed, notarized Mac DMG plus the unsigned IPA as a GitHub release (`bash scripts/build-mac-dmg.sh` builds the DMG locally). The DMG needs `APPLE_CERTIFICATE_BASE64`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_TEAM_ID` and `APPLE_APP_SPECIFIC_PASSWORD`. TestFlight needs `ASC_KEY_ID`, `ASC_ISSUER_ID` and `ASC_KEY_P8_BASE64` (an App Store Connect team API key with Admin access for cloud-managed distribution signing, base64-encoded). Signing uses automatic cloud-managed certificates; the tag sets the numeric marketing version (for example, `v1.0.0-beta.2` becomes `1.0.0`) and the workflow run number plus attempt sets the build number. The workflow can also be run manually with a marketing version to upload a TestFlight build without a tag, or against a tag to build all release assets. Signed DMGs are also retained as workflow artifacts; a single publishing job uploads the DMG and unsigned IPA to avoid concurrent release creation. See [release notes](docs/release-notes.md) for distribution limits.

## Rust migration

The [Rust port plan](https://artifacts.microapps.space/a/w474jy87pnxg) is being implemented in `crates/`, with pxui pinned as a submodule. The Swift app remains the shipping app until the replacement passes the plan's platform gates. The terminal player has been removed. The Rust window restores a paused queue and shows a worker-prepared library with sidebar navigation, virtual song rows, album/artist browsing, a Now panel and transport controls. Double-click a song to play. Find searches song text; click Title, Artist, Album or Time to sort, or # to restore library order. Find supports format tokens (flac/mp3/m4a/wav/aiff), fav/favorites, year:2019, unplayed, cloud and offline; the visible chips toggle common filters. Other Find result sections remain unfinished. Native font fallback now renders the tested multilingual tags and emoji; first-time font discovery may allocate. System appearance and reduced motion now update through native notifications, with cached themes and saved settings loaded on the worker. Use the Refresh library button to load database changes without restarting; current selection follows song/collection identity. Automatic source observation and the complete reference UI remain unfinished.

The Rust window supports resizing and native zoom/full-screen controls, with content behind its
transparent title bar. The Queue button opens the full occurrence-based queue editor; Save as
playlist saves its current order with duplicate songs included once. Album, Now panel and queue
artwork load from the existing library on a separate worker, with bounded thumbnail/texture caches.
Tracks without readable local cover files keep their placeholder.
Quit an already running Rust app before opening a rebuilt bundle so macOS launches the new binary.

The Rust checkpoint is on **rust-port**. It requires macOS 15+, Xcode with the macOS 26 SDK, Rust (the pinned toolchain is selected by rustup), CMake and pkg-config. On another Mac:

```sh
git clone --branch rust-port --recurse-submodules git@github.com:olegpustovit/localwave.git
cd localwave
make build                  # static SDL3, libpxui, Rust macOS app
make bundle                 # signed sandboxed target/bundle/LocalWave.app
open target/bundle/LocalWave.app
make install                # install to /Applications (APPDIR overrides)
cargo t -p lw_core          # Rust storage and model checks
cargo run -p lw_bin --bin localwave-playback -- --mute /absolute/path/audio.flac
make performance            # signed Songs + Find + Queue + drag + queue sheet + artwork: allocation/time/idle gates
```

Warm library/app builds have measured 0.14/0.04 seconds, and 117 bounded app checks 12.55 seconds. The regular 10,000-song UI workload has zero warm Rust allocations, but the strict 1 ms maximum still has recorded misses. Tested warm RGBA/BGRA blur draws and image layers through 128 boundaries also allocate zero Rust objects. Native memory remains unqualified. The rust-port branch pins the matching published pxui commit; clone with submodules, or run git submodule update --init --recursive after pulling. See [Rust port qualification](docs/rust-port.md) for the current gate status and limitations.

## Development

The app is in [localwave/Sources](localwave/Sources), organized into App, Core, Domain, Data and Features. [AGENTS.md](AGENTS.md) maps those layers. The shared scheme uses [localwave.xctestplan](localwave.xctestplan); run **Product > Test** in Xcode. Existing iOS Swift Testing cases are in [localwaveTests](localwaveTests). Run `swift test` for shared repository/import tests; pull-request CI runs the package tests and the iOS unit tests; the manual **Verify** workflow also runs the UI flows. See the test budget in [AGENTS.md](AGENTS.md).

Report a reproducible problem in [GitHub issues](https://github.com/olegpustovit/localwave/issues), including your iOS version and whether the source is local or iCloud-backed. Do not attach private music or library data.

## License

[MIT](LICENSE). SQLite.swift and bundled fonts retain their own licenses. Rust filename cleanup uses Unicode data under the [Unicode License v3](crates/lw_core/UNICODE-LICENSE.txt).
