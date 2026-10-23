# LocalWave

An iOS music player for MP3 files you select from Files or iCloud Drive. Browse your library by artist, album or song, search it, and play tracks without an Apple Music subscription.

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
2. Let Xcode resolve SQLite.swift. For a physical device, select your signing team in the app target.
3. Choose **Product > Run**.
4. Open **Sync**, tap **+**, and select a folder containing MP3 files you have permission to use.
5. Select files or folders in the source browser and tap **Import N items**, where N is the selection count.
6. Open **Library**, choose a song, and use the mini player or full player to control playback.

Importing an iCloud source can require a network connection to download its files. Local playback depends on the selected files being available and accessible; this is not a promise that every cloud-backed file works offline.

## What is in the source

The app includes folder import, metadata and artwork, artist/album/song views, playlists, a playback queue, shuffle/repeat controls and lock-screen remote commands. Background audio is declared in the app's configuration. SQLite FTS5 provides indexed song and path search, with BM25 ranking in song queries.

These are source-backed capabilities, not a fresh device test. Bookmark recovery, iCloud downloads, long-running playback and library changes were not qualified on the Linux review host. No current native build or test pass is claimed.

## Development

The app is in [localwave/Sources](localwave/Sources), organized into App, Core, Domain, Data and Features. [CLAUDE.md](CLAUDE.md) maps those layers. The shared scheme uses [localwave.xctestplan](localwave.xctestplan); run **Product > Test** in Xcode. Existing Swift Testing cases are in [localwaveTests/Tests.swift](localwaveTests/Tests.swift).

Report a reproducible problem in [GitHub issues](https://github.com/olegpustovit/localwave/issues), including your iOS version and whether the source is local or iCloud-backed. Do not attach private music or library data.

## License

[MIT](LICENSE). SQLite.swift and bundled fonts retain their own licenses.
