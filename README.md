# LocalWave

An iOS music player for MP3, FLAC and other supported audio files you select from Files or iCloud Drive. Browse your library by artist, album or song, search it, and play tracks without an Apple Music subscription.

![LocalWave album library, source-folder import and player](https://olegpustovit.com/posts/why-i-built-a-native-mp3-player-in-swiftui/frame_source.webp)

*Existing app screenshots from the [development article](https://olegpustovit.com/posts/why-i-built-a-native-mp3-player-in-swiftui/). They were not recaptured for this review.*

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
4. Open **Sync**, tap **+**, and select a folder containing audio files you have permission to use.
5. Tap **Import all** in the source browser to import the displayed files and folders recursively, or select items and tap **Import N items**.
6. Open **Library**, choose a song, and use the mini player or full player to control playback.

Importing an iCloud source can require a network connection to download its files. Local playback depends on the selected files being available and accessible; this is not a promise that every cloud-backed file works offline.

## What is in the source

The app includes folder import, metadata and artwork, artist/album/song views, playlists, a playback queue, shuffle/repeat controls and lock-screen remote commands. Background audio is declared in the app's configuration. SQLite FTS5 provides indexed song and path search, with BM25 ranking in song queries.

FLAC imports read Vorbis comments (including artist, title, album artist, year, track and disc) and embedded cover art. Device volume is controlled with the iPhone’s hardware buttons or Control Center. Favorite tracks can be marked with the star in Songs; playlist views show the saved favorite state.

Automated checks cover shared database upgrades, Favorites persistence, recursive local FLAC import and native decoding. Physical-device audio routes, iCloud downloads, and long-running background playback still require device testing.

## Unsigned IPA

Download the unsigned device IPA and checksum from [Releases](https://github.com/olegpustovit/localwave/releases). It must be signed using your chosen iOS sideloading tool before installation; it cannot install directly. Your signing account and tool determine expiry and renewal requirements.

To produce the same package locally:

```sh
bash scripts/build-unsigned-ipa.sh /tmp/localwave-unsigned
```

Pull-request CI also uploads an unsigned IPA as a workflow artifact. The manual **Unsigned release** workflow publishes a tagged commit as a release. See [release notes](docs/unsigned-release-notes.md) for distribution limits.

## macOS terminal player

```sh
swift build -c release
.build/release/localwave-tui
```

The TUI uses the same repositories and import service as iOS, with its own database under `~/Library/Application Support/localwave` (or `$XDG_DATA_HOME/localwave`). Add and scan a source under Sync, then browse/import it. Use Ctrl+G/A/B/L/P/R for Sync/Artists/Albums/Songs/Playlists/Player, j/k to navigate, Space to play, p to add a song to a playlist, and Ctrl+X to quit. `?` shows global help. The checked-in SwiftTUI revision is fetched automatically; a separate local checkout is not required. See [TUI.md](TUI.md).

## Development

The app is in [localwave/Sources](localwave/Sources), organized into App, Core, Domain, Data and Features. [CLAUDE.md](CLAUDE.md) maps those layers. The shared scheme uses [localwave.xctestplan](localwave.xctestplan); run **Product > Test** in Xcode. Existing iOS Swift Testing cases are in [localwaveTests](localwaveTests). Run `swift test` for shared repository/import tests; CI runs both the package tests and the iOS test plan.

Report a reproducible problem in [GitHub issues](https://github.com/olegpustovit/localwave/issues), including your iOS version and whether the source is local or iCloud-backed. Do not attach private music or library data.

## License

[MIT](LICENSE). SQLite.swift and bundled fonts retain their own licenses.
