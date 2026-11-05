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

SQLite.swift and the pinned SwiftTUI fork are package dependencies. Shared Domain, Core, Data and Player targets are declared in Package.swift; Xcode links those modules. Do not re-add their source files to the app target. Song and source-path search use FTS5; song queries include BM25 ranking. AVFoundation and MediaPlayer provide playback and remote controls. Folder access uses security-scoped bookmarks; selected cloud files may need downloading before playback.

## Test budget (fast by default)

Default verification loop must stay fast. Budgets are hard limits; a change that breaks one must fix or move the slow test before merging.

| Loop | Command | Budget |
| --- | --- | --- |
| Shared (every change) | `swift test` | 30 s wall, warm build |
| iOS unit (app logic changes) | `xcodebuild ... -only-testing:musicappTests test` | 90 s wall, warm build |
| UI flows (redesign/UI changes only, nightly or manual) | full `musicapp` plan on a simulator | on demand; never required per PR |

Rules: no `sleep` or real-time waits in unit tests, use injected clocks or polling with short deadlines; no network; one shared fixture per suite; shared tests run in parallel (`swift test --parallel` when isolated); a unit test over 1 s needs a reason in a comment or moves to the UI plan. Pull-request CI runs only the first two rows; UI flows run through the manual **Verify** workflow. Release tags run `swift test` only so the TestFlight upload is not blocked on UI flows.

Read the implementation before changing behavior or promising reliability. Run `swift test` for shared repository, FLAC parsing and recursive import checks, and `swift build -c release` for the macOS terminal player. Run the Xcode test plan on a compatible simulator for app checks. Physical-device playback, bookmark recovery and iCloud behavior need device qualification. `scripts/build-unsigned-ipa.sh` produces an unsigned device IPA; CI and the manual release workflow use the same script.
