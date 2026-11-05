# Native Mac list qualification

The native macOS app uses SwiftUI `Table` for Songs, with `Song.uniqueId` as its stable row identity. Display strings are prepared when a query changes, table sorting updates the row values, and the player transport observes playback ticks separately from the library view. Albums use a lazy grid; the app no longer routes Mac rendering through the iOS canvas host.

## Checks run

- Native macOS app build: passed with `xcodebuild -scheme musicapp -destination 'platform=macOS' build`.
- iOS unit tests: passed with 36 tests (the shared change is also covered by `swift test --parallel`).
- Manual smoke on the earlier native shell: imported the 92-track demo library and played a track. This did not measure a large collection.

## Measurements still needed

No Instruments recording was made for a 1,838-song window drag or a 10,000-song scroll in this qualification. Frame hitches, first-paint time and row body evaluations therefore have no measured baseline or pass result. The plan's 16.7 ms drag, 33 ms scroll and 200 ms first-paint gates remain unverified on a representative library and Mac.
