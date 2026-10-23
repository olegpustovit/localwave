# Development guide

LocalWave is an iOS MP3 player. Read [README.md](README.md) for the current build instructions and limits.

## Build and test

Open localwave.xcodeproj in Xcode 16.2 or newer. Select the shared musicapp scheme, which builds the localwave app. The app target requires iOS 18.0; use an iOS 18.2 or newer simulator for the project defaults and unit-test target. Targets retain Swift 5 language mode.

Product > Run builds the app. Product > Test uses localwave.xctestplan. The test targets retain the names musicappTests and musicappUITests. Existing cases in localwaveTests/Tests.swift use Swift Testing. Do not infer broad UI coverage from target names.

## Source map

- localwave/Sources/App contains the entry point and dependency container.
- Core contains shared helpers and constants.
- Domain contains models and repository/service protocols.
- Data contains SQLite repositories, file access, import and playback services.
- Features contains Library, Player, Playlists, Sync and shared SwiftUI components.

SQLite.swift is the external package dependency. Song and source-path search use FTS5; song queries include BM25 ranking. AVFoundation and MediaPlayer provide playback and remote controls. Folder access uses security-scoped bookmarks; selected cloud files may need downloading before playback.

Read the implementation before changing behavior or promising reliability. No native build, device playback, bookmark recovery or iCloud test was completed during the Linux documentation review.
