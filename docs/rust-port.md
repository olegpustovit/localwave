# Rust port implementation and qualification

The port is **in progress**. The Rust app now opens the library worker, restores a paused queue and connects library browsing and transport controls to native playback. The complete reference UI and platform qualification remain unfinished. The Swift iOS and Mac app remains the shipping product. Do not remove its targets or switch release distribution before the platform gates below pass.

## SwiftUI scrolling sections and retained geometry checkpoint

Albums and Artists now use one scroll region for the page heading and rows. The Albums title,
Artist/Year/Size toolbar and gallery scroll together. Artists keeps its page title in the scroll
content while its **37.3-point** opaque column heading pins above the rows. Blank heading padding
blocks row clicks underneath; wheel input still reaches the containing scroll region. Overlay
scroll layout reserves no gutter, preserving the full content width and rightmost gallery column.

The measured initial positions remain **172.6 points** for album artwork and **142.9 points** for
the first artist row (105.6-point lead-in plus heading). Gallery spacing preserves fractional row
strides. The album header rule now spans the full content width, matching the native reference.
The Artists Refresh control, absent in SwiftUI, has been removed. A pending artist projection
keeps the page title and original 32-point loading inset until the prepared table arrives.

pxui **ABI 1.20** adds `Ui::section_list` / `pxui_ui_section_list`, reusing existing sticky,
blocking and scroll layout. It builds only visible fixed-height rows plus bounded overscan,
with separate lead-in, heading and trailing extents; there is no storage proportional to song
count. C descriptor validation, sentinel callbacks and borrowed-handle guards match the public
static/dynamic wrapper. Existing row identities, retained artwork and entrance motion remain.
The page heading has its own retained view, independent of playback clock/artwork revisions.

The app's new scroll assertions exposed stale geometry when an entire heading left the clip.
pxui now updates culled retained-view geometry without adding offscreen draw runs, and composes
its rectangle/accessibility queries. Accessibility visibility includes the composed clip; clip
bounds participate in retained tree identity. Indexed and flat cached headings, including an
explicit clip ancestor, pass accessible scroll-out/return with zero warm Rust allocations.
The first attempted fix visited all unclipped children, exceeding the existing **1,024-instance**
long-scroll storage bound. That implementation was replaced; the original bound now passes.
Its failing evidence remains `/tmp/localwave-section-sdk-matrix.log`, followed by the focused
passing `/tmp/localwave-section-budget-regression.log`.

### Checks

- **336 engine/text/ABI/type checks pass**, **17.048 s wall / 14.020 s summed**, zero skips:
  `/tmp/localwave-section-sdk-visibility-final-matrix.log`. The full static wrapper passes
  **62 checks**, **6.343 s wall / 1.963 s summed**:
  `/tmp/localwave-section-sdk-static-matrix.log`.
- Native Metal **1×/2×** pixels pass for scrolling cached lead-in, opaque pinned heading, rows
  below it and the rightmost content pixel. The case asserts one draw and zero warm Rust UI
  build/draw allocations: **4.457 s wall / 0.153 s case**, zero skips,
  `/tmp/localwave-section-native-cached-pixels.log`. Fixture/prepare allocations are outside
  this focused counter; the signed app matrix below retains both full warm UI counters.
- Descriptor validation passes short/null/invalid flags, reserved bytes, text, count and geometry
  before invoking builders or changing output. Callback scope restores correctly:
  `/tmp/localwave-section-abi-validation.log`, **6.192 s wall / 0.031 s case**.
  Type layout, dependencies, C header, dynamic exports and append-only snapshot gates pass.
  Gate logs are `/tmp/localwave-section-gate-{snapshot-write,deps,header,exports,snapshot}.log`.
- **172 app checks pass**, **16.726 s wall / 16.390 s summed**, zero skips:
  `/tmp/localwave-section-app-matrix.log`. Artist and 10,000-album scroll checks assert exact
  heading/toolbar/row positions, pinning, virtualization and zero warm allocations. The loading
  transition uses the same `ShellState::refresh` adoption contract as the host; the first test
  omitted that call and retained the old page, recorded in
  `/tmp/localwave-section-app-final-matrix.log`. Corrected UI checks pass **44 cases** in
  **5.887 s wall / 1.483 s summed**: `/tmp/localwave-section-app-ui-final.log`.
- App Clippy passes all four packages/all targets/native interop with warnings denied in
  **3.59 s**, `/tmp/localwave-section-app-clippy.log`. Formatting and whitespace checks pass.
  pxui `5998153` was published before the parent pointer.

### Rejected offscreen build-cache experiment

The first signed section-list run is retained in `/tmp/localwave-section-performance-final.log`:
all eleven workloads have zero warm Rust allocations and all eleven strict wall maxima fail.
Its library/app builds take **25.84 / 13.01 s**, idle is **509.6 ms**, and the gallery has
**19 image quads / 4,800,512 texture bytes / zero warm requests**. Artists draws **23–31 rows**
while its first artist ranges from **1–13**.

pxui `680c9d5` then tried keeping a clean build after its culled paint run was discarded. That
avoids rebuilding fully offscreen headings, but a returning view has no valid paint and requests
an additional whole-frame settling build. Although shared/static/native correctness checks and
warm allocation checks passed, the signed workload became slower: Artists CPU p50/p95/p99/max
changed from **581/889/991/997 µs** to **1125/1422/1491/1564 µs**. This comparison does not qualify
an exact speed ratio or explain every wall outlier. The experiment is **reverted** by `afdbe68`;
final engine runtime code matches `5998153`. A new assertion pins **one frame build** when an
indexed cached heading returns. The existing flat-view/clipping/storage/allocation checks remain.

The rejected run remains `/tmp/localwave-section-retained-build-performance.log`, with zero
warm Rust allocations in all eleven workloads and every strict wall maximum failing. Its passing
checks are retained as `/tmp/localwave-section-retained-build-{sdk-matrix,static,pixels}.log` and
`/tmp/localwave-section-retained-build-app-{matrix,clippy}.log`; they do not establish speed.
Final single-build qualification and the signed run are recorded below. The first implementation
and rejected experiment are not presented as passing latency gates.

The final pxui pin is `dce8d2d`, published before staging. Its new single-build assertion passes
dynamically in **10.329 s wall / 0.027 s case**; both static section cases pass in **3.938 s wall /
0.058 s summed**. Native 1×/2× pixels pass in **1.513 s wall / 0.137 s case**. Final app UI checks
pass **44 cases in 1.077 s wall / 1.511 s summed**, zero skips. Logs:
`/tmp/localwave-section-single-build-{sdk,static,pixels,ui}.log`. The final runtime code is the
same as the 336-check/172-check matrices above; the follow-up adds regression coverage and
backs out the rejected experiment. No latency or allocation assertion was weakened.

### Final signed bundle

`/tmp/localwave-section-one-pass-performance.log` records the final reverted-runtime bundle.
Library/app release builds take **22.48 / 0.12 s**. Signature, macOS 15 minimum, default stack,
bundled dylib identity and portable rpath checks pass. Startup draws **27** times and then stays
idle for **509.0 ms** without builds or draws. The running normal GUI remains PID 69331.

All **eleven unchanged workloads** retain **182 frames / 82 warm-up / 100 measured frames**, with
**zero warm Rust allocations** across app and libpxui. Artwork retains **19 image quads** and
**zero warm requests**. Artists renders **23–31 of 10,000 rows**, scrolling first artist **1–13**.
Every strict **1,000 µs wall maximum fails**; `make performance` returns nonzero. CPU timings are
reported separately and do not replace the wall gate. Native/process memory remains unqualified.

| Workload | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- |
| songs-original | 3871 / 625 / 2387 / 3296 | 846 / 592 / 755 / 834 |
| find-filters-title-descending | 3114 / 614 / 1942 / 2715 | 876 / 580 / 698 / 730 |
| queue-occurrences | 2345 / 635 / 1829 / 2090 | 788 / 598 / 714 / 770 |
| queue-carry-scroll-cancel | 3874 / 533 / 2177 / 2372 | 2081 / 499 / 834 / 2039 |
| full-queue-playlist-editor | 3416 / 748 / 2041 / 2643 | 957 / 609 / 870 / 957 |
| albums-artwork | 2180 / 644 / 1839 / 2100 | 1293 / 578 / 1155 / 1227 |
| waveform-scrub-and-scroll | 3341 / 459 / 1538 / 3298 | 751 / 417 / 669 / 731 |
| settings-menus-appearance | 1804 / 357 / 1289 / 1638 | 538 / 284 / 463 / 512 |
| files-indexed-directory | 1393 / 323 / 1108 / 1341 | 416 / 307 / 380 / 416 |
| external-drop-hover-and-cancel | 1512 / 295 / 523 / 1494 | 363 / 274 / 331 / 356 |
| artists-measured-columns | 2497 / 725 / 2257 / 2436 | 1000 / 645 / 917 / 982 |

Full screenshot parity, Songs/Find/detail scrolling and sizing, remaining reference views,
strict wall latency, native/process memory and Mac/iOS/device/distribution qualification remain
unfinished. No Swift/device suite was repeated. The normal GUI PID 69331 and its library/defaults
are preserved.

## SwiftUI album toolbar and native component sizing checkpoint

Albums now has the reference's Artist/Year/Size toolbar and functional Shuffle all, placing the
first gallery row at **y=172.6** before titlebar insets. Controls are **30 points high**, with
**8-point gaps**, **52/47/44/100-point widths**, and a **1.5-point fractional selection border**.
The header's bottom line is inset as in SwiftUI, followed by its one-point clearance. Album
names use the 14-point medium face. Plain album cards no longer acquire an extra hover fill or
Refresh button that is absent from the reference.

All three album orders are prepared with the catalog on the worker; rendering borrows indices.
Artist strips exact leading `The ` as Swift does, Year uses the first ordered track's release
year, and Size preserves Swift's existing Added/latest-created ordering despite its label.
Changing sort retains album identity, survives navigation and does not restart page entrances.
The header's formatted size and active-account source count are prepared on the worker. Source
count uses a scoped SQL COUNT rather than loading folder paths/bookmarks. Account identity joins
SQLite version in the catalog reuse key, so source labels cannot reuse the previous account's
projection.

Shuffle all sends the retained library song Arc through the controller mailbox. The worker
copies and shuffles it, including its first item and repeated occurrences, then uses the existing
Configure/lease/playback/persistence path. Ordinary Shuffle remains the current queue mode.
Empty selection is rejected. The native integration uses muted real audio and checks persisted
queue order, context, first selection, volume and unchanged catalog input.

### Native comparison and shared pxui fixes

A separate AppKit/SwiftUI process measured the album header and LazyVGrid at content widths
**888/728/472**, using the actual `LWCSSLineBox.swift`, bundled fonts and matching view structure.
The actual private `PlayerBylineFlow` definition was also measured at **100/279/400**. These are
component comparisons, not a capture of the complete shipping application. Temporary harnesses
are `/tmp/localwave-measure-album-grid.swift` and `/tmp/localwave-measure-byline.swift`; results
are `/tmp/localwave-native-album-grid-geometry.log` and `/tmp/localwave-native-byline-geometry.log`.

Native Text intrinsic widths ceil to physical pixels; raw CoreText widths do not. Artist's toolbar width
first failed at **51.915** versus native **52**. Shared fit labels now apply that native rounding
using cached measurements and pxui's integer rendering density (ABI 1.19). A second native
capture with displayScale=2 gives Year **46.5** rather than **47** and The National **80.5** rather
than **81**; both 1×/2× variants are checked, including warm density changes. References are
`/tmp/localwave-native-album-grid-scale2-geometry.log` and
`/tmp/localwave-native-byline-scale2-geometry.log`. The byline comparison also failed at a **74-point** wrapped album
versus native **78**: cosmic-text omits the line-breaking separator's advance from its fitting
width. pxui **ABI 1.18** adds opt-in `WRAP_WORD_SPACES` sizing. It reuses cached runs and scratch
storage to include blank advances between visible soft wraps. Glyph positions, ordinary wrapping
and hidden paragraphs are unchanged. The native byline geometry now passes all three widths.
The earlier failing logs remain `/tmp/localwave-album-toolbar-native-width-red.log` and
`/tmp/localwave-byline-native-geometry-test.log`.

pxui also exposes analytic quadratic and bounded four-piece cubic painting through its existing
instance writer and shader pipeline. The actual shuffle SVG geometry replaces the old crossed
lines in both toolbar and transport. It uses **20 retained analytic instances**, no tessellation
buffer or added draw pipeline. Sampled cubic deviation stays below **0.04 physical pixel** at
16/20 points and 1×/2×; large arbitrary cubic paths still need adaptive splitting.

### Checks

- **328 engine/text/ABI checks pass**, **33.665 s wall / 13.915 s summed**, zero skips:
  `/tmp/localwave-album-layout-sdk-matrix.log`. The static wrapper passes **59 checks** in
  **6.003 s wall / 2.047 s summed**: `/tmp/localwave-album-layout-sdk-static.log`.
- Native renderer scene/golden passes with **zero per-channel differences**, **5.077 s wall /
  0.224 s case**: `/tmp/localwave-album-layout-native-curves.log`. No fixture was refreshed.
- Dependencies, ABI header, exports and append-only snapshot gates pass. The optional whole-SDK
  shape gate still reports existing generic/macro/lifetime violations outside the changed
  code: `/tmp/localwave-album-layout-gate-shape.log`. pxui `6d6a148` was pushed before its parent pin.
- **171 app checks pass**, **24.835 s wall / 16.238 s summed**, zero skips:
  `/tmp/localwave-album-layout-app-matrix.log`. This includes exact native toolbar/byline bounds,
  three desktop gallery widths/row stride, warm sorting/vector allocations and the native queue
  check. The focused native Shuffle all check takes **0.616 s** after compilation:
  `/tmp/localwave-album-shuffle-native-test.log`.

- The density follow-up passes public dynamic/static query, cached-view and warm-allocation
  checks: **12.603 / 4.445 s wall**, **0.032 / 0.032 s case**. Logs:
  `/tmp/localwave-density-sdk-test.log` and `/tmp/localwave-density-sdk-static.log`.
  The focused finite-width bound check passes in **7.054 s wall / 0.030 s case**:
  `/tmp/localwave-curves-width-bound-test.log`; analytic widths exceeding the instance's f16 range
  are refused before emission.
- After the 2× correction, all **43 UI cases pass in 6.677 s wall / 1.538 s summed**:
  `/tmp/localwave-album-layout-density-ui-matrix.log`. Toolbar and byline checks alternate 1×/2×,
  asserting the actual native bounds, navigation and zero warm allocations.
- Final app Clippy passes with native interop/all targets/warnings denied in **1.79 s**:
  `/tmp/localwave-album-layout-density-clippy.log`. Dependency/header/export/append-only ABI gates
  pass for ABI 1.19. Published pxui follow-ups are `d52b687` and `87c30bc`.

The full port remains unfinished. The library header's scrolling/pinning behaviour, remaining
song/detail/compact layouts, whole-screen captures, native memory, strict wall latency and
Mac/iOS/device/distribution qualification remain open. No Swift or device suite was repeated.
The normal GUI PID 69331 and its library/defaults are preserved.

App Clippy passes all four packages/all targets with native interop and warnings denied in
**3.21 s**: `/tmp/localwave-album-layout-clippy.log`.

### Final signed performance

`/tmp/localwave-album-layout-performance-final.log` records the ABI 1.19 bundle built in
**22.99 / 11.84 s** for library/app, including the density changes. Portable dylib ID/rpath,
macOS 15 minimum, sandbox, default executable stack and signature checks pass. Startup draws
**32** times, then the existing idle check observes **508.2 ms** without builds/draws.
All **eleven unchanged workloads** retain 182/82/100 total/warm/measured frames and record
**zero warm Rust allocations** across app and libpxui. The artwork gallery retains 19 image
quads and **zero warm requests**. Artists still renders 23–27 of 10,000 rows and scrolls first
artist 1–17. Every strict **1,000 µs wall maximum fails**, and `make performance` returns
nonzero. No limit, workload or warm-up was weakened. The earlier pre-density run is retained
as `/tmp/localwave-album-layout-performance-matrix.log`; the rerun followed actual source changes.

| Workload | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- |
| songs-original | 4635 / 637 / 1453 / 2784 | 777 / 595 / 686 / 766 |
| find-filters-title-descending | 2508 / 664 / 2006 / 2117 | 742 / 590 / 699 / 735 |
| queue-occurrences | 2205 / 646 / 1690 / 2128 | 795 / 601 / 717 / 763 |
| queue-carry-scroll-cancel | 4502 / 492 / 2163 / 3056 | 2062 / 463 / 755 / 2003 |
| full-queue-playlist-editor | 3725 / 775 / 2211 / 3439 | 1004 / 668 / 950 / 990 |
| albums-artwork | 2643 / 660 / 1863 / 2218 | 1136 / 597 / 922 / 1001 |
| waveform-scrub-and-scroll | 2571 / 470 / 1658 / 2119 | 748 / 407 / 679 / 745 |
| settings-menus-appearance | 2107 / 298 / 745 / 1600 | 598 / 276 / 464 / 577 |
| files-indexed-directory | 1503 / 338 / 938 / 1303 | 407 / 314 / 383 / 406 |
| external-drop-hover-and-cancel | 2893 / 302 / 1010 / 1986 | 391 / 275 / 345 / 370 |
| artists-measured-columns | 3381 / 834 / 2473 / 2932 | 982 / 724 / 901 / 955 |

## SwiftUI artist table and text layout checkpoint

pxui **ABI 1.17** adds an explicit `Ui::text_line_box` mode. It applies the same primary-font
rounded ascent/descent and floored half-leading as Swift's `LWCSSLineBox`, preserving legacy
text placement for other consumers. Fit text now supplies intrinsic width/height in this mode;
wrapped Grow text settles its final height in the same frame, and empty text retains one line.
Shared labels, headings, Settings paragraphs and the player title use this API. Direct glyph
comparisons at 1×/2× check the captured CoreText baselines for 34/19/14/12-point Host Grotesk and
12-point Geist Mono. Nine named-face width references agree within 0.01 point.

Artists now uses the desktop reference table: four font-measured proportional columns, 37.3-point
header, 38-point rows, 28-point artwork, correct 32/8/20-point insets and a 140 ms hover. Distinct
album summaries, rounded minutes and preferred widths are prepared once on the catalog worker
after its SQLite read snapshot; catalog clones share the result. Rendering borrows only virtual
rows. Existing entrance motion applies to the first rows. An isolated eleventh signed workload
scrolls **10,000 prepared artists** without changing the other ten fixtures or their limits.

The player byline reproduces `PlayerBylineFlow` with a fixed five-item stack array, measured
15-point spaces/strut and padded 14-point links aligned to their first baseline. It wraps long
names, trims whitespace at row starts and routes artist/album links to the matching context.
Natural and constrained measurement keys include distinct width identities, preventing repeated
reshaping. Measurement happens inside the retained metadata view, independent of clock ticks.
The danger colour now matches the reference's constant #D21A1A in both appearances.

Accessible scrolling exposed an unrelated generic allocation in pxui's action bridge. The
bridge now validates the entire bounded batch before translating in stack chunks into its
retained queue. Input order, numeric payloads and invalid-tail atomicity are preserved. The
public static/dynamic regression injects 65 actions, exercising multiple chunks.

### Checks

- **326 pxui/text/core/ABI cases pass in 20.440 s wall / 14.235 s summed**, zero skips:
  `/tmp/localwave-linebox-sdk-matrix.log`. Static line-box resize/validation/warm-allocation case
  passes in **5.176 s wall / 0.044 s case**: `/tmp/localwave-linebox-static.log`.
- Dependency, header, export and append-only ABI snapshot gates pass. pxui commits `94ddcb3`
  (accessible input) and `692b06f` (line boxes) were pushed before the parent pin.
- **165 app cases pass in 16.650 s wall / 13.835 s summed**, zero skips:
  `/tmp/localwave-artists-linebox-app-matrix.log`. This includes artist virtualization, 140 ms
  hover, accessible scrolling/navigation, CoreText glyph baselines and byline wrapping/links.
- App Clippy passes all four packages/all targets with native interop and warnings denied in
  **2.18 s**: `/tmp/localwave-artists-linebox-clippy.log`.
- Release library/app build in **23.00 / 11.32 s**, including recompilation. The sandbox bundle
  is rebuilt and signed: `/tmp/localwave-artists-linebox-bundle.log`. Normal GUI PID 69331 and
  its library/defaults remain untouched.
- Focused signed Artists renders **23–27 rows**, scrolls first row **1–17**, and records **zero
  warm Rust allocations**. Its strict 1,000 µs wall maximum fails at **3,299 µs**:
  `/tmp/localwave-artists-linebox-performance-focused.log`. Earlier failures are retained;
  the byline's new width-identity regression first failed with 62 warm allocations, then passed
  after fixing the keys. No guard, warm-up or budget was relaxed.

These checks qualify the listed dimensions/behaviours, not whole-screen screenshot equality.
Responsive compact navigation, remaining library/detail layouts, other reference flows, native
memory and Mac/iOS/device distribution qualification remain unfinished. No Swift/device suite
was repeated for these Rust-only changes. The full signed results follow below.

### Signed eleven-workload results

`/tmp/localwave-artists-linebox-performance-matrix.log` records the final full run. All eleven
workloads retain **182 frames / 82 warm-up / 100 measured frames** and record **zero warm
Rust allocations** across the app and libpxui counters. Artwork records **zero warm requests**.
Artists renders **23–27 rows** and moves first visible artist **1–17**. Settings applies all
45 actions/all eight themes; Files draws 14 of 10,000 indexed rows; drop draws its overlay on
50 measured frames. The signed bundle checks pass portable rpath/dylib identity, sandbox,
macOS 15 minimum and default executable stack. Startup draws 31 times, then remains idle for
**501.3 ms**. Warm lib/app builds take **0.15 / 0.05 s**.

Every unchanged **1,000 µs wall maximum fails**, so `make performance` correctly returns nonzero.
These failures remain open and are not replaced by the CPU numbers or percentiles below.

| Workload | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- |
| songs-original | 3394 / 671 / 1870 / 2651 | 840 / 603 / 730 / 810 |
| find-filters-title-descending | 2790 / 642 / 2202 / 2391 | 781 / 593 / 721 / 769 |
| queue-occurrences | 2863 / 649 / 1937 / 2171 | 774 / 599 / 719 / 772 |
| queue-carry-scroll-cancel | 2422 / 487 / 1884 / 2362 | 2078 / 466 / 753 / 2055 |
| full-queue-playlist-editor | 2831 / 793 / 2112 / 2718 | 1051 / 633 / 907 / 991 |
| albums-artwork | 2903 / 575 / 1781 / 2196 | 992 / 540 / 900 / 936 |
| waveform-scrub-and-scroll | 2518 / 470 / 1328 / 2329 | 766 / 381 / 717 / 751 |
| settings-menus-appearance | 2371 / 317 / 829 / 1654 | 569 / 279 / 464 / 555 |
| files-indexed-directory | 2010 / 320 / 938 / 1390 | 397 / 306 / 353 / 382 |
| external-drop-hover-and-cancel | 1391 / 281 / 1129 / 1362 | 370 / 268 / 323 / 331 |
| artists-measured-columns | 2723 / 735 / 1924 / 2263 | 928 / 692 / 837 / 885 |

## SwiftUI motion and first layout checkpoint

This begins the requested SwiftUI visual/motion correction. **Full pixel parity is not qualified.**
The actual source references are `ThemeProvider.swift` (`LWAppear`, `MatrixText`),
`MainTabView.swift`, `WaveLibraryViews.swift` and `WavePlayerViews.swift`.

pxui **ABI 1.16** exposes the existing host frame clock. Offset/opacity animations on retained
views previously affected the root node's own paint rather than its cached contents; now they
compose in the view table with nested hit/accessibility geometry. Equal-time keyframes merge
channels before interpolation, allowing a fade and movement to share their curve. Cached instances
remain unchanged. Completing keyframes reserves its return pool during cold creation. Reopening
nested view kinds reuses their warmed node/hit/name storage rather than assigning unlike buffers
by LIFO. This fixes allocations exposed by resize and transient-view animation checks.

LocalWave now starts page entrances on navigation, staggers the first library rows by 18 ms with
a 10-point rise, and applies the player panel's 32-point entrance and delayed information/clock
entrances. Ordinary catalog/player updates retain their navigation generation. Animations respect
reduced motion, use the original curves and stop requesting frames after settlement. The library
and panel are clipped to their viewports.

The elapsed matrix now matches Swift's glyph bitmaps, one-column colon, zero-padded minutes,
43-point height, exact intrinsic width and 0.78-unit fractional dots. Unchanged characters remain
lit; changed characters reveal in two steps over 140 ms, staggered by 16 ms per row. Seven shared
fading rows bound animation storage even when several digits carry simultaneously. No clock
formatting, heap strings or font reshaping is introduced on those animation ticks.

The first layout corrections use the 32-point library header inset, 34-point medium heading with
-0.85 tracking, reference subtitle sizing, adaptive square album cards (150-point minimum,
20-point column gap, 28-point row gap), and 14/13-point card labels. Album captions include the
year, prepared once on the worker. The player artwork is 200×200 at its reference origin, with
corrected information/byline and clock placement. Geometry checks cover three desktop widths.

### Evidence

- Deterministic pre-fix retained entrance fails at its initial child position:
  `/tmp/localwave-motion-engine-red.log`. Core and public static/dynamic cases now check motion,
  opacity, geometry, unchanged instances, no rebuilds, warm allocations and eventual idle.
- **295 engine/ABI checks pass in 15.295 s wall / 12.409 s summed**, zero skips:
  `/tmp/localwave-motion-engine-final.log`. The static wrapper's **55 checks pass in 5.536 s**:
  `/tmp/localwave-motion-engine-static-final.log`. Dependency, header, export and append-only ABI
  snapshot gates pass. Child published first, then the parent pin is checked against its remote.
- **159 app checks pass in 11.058 s wall / 14.681 s summed**, zero skips:
  `/tmp/localwave-motion-prefetch-final-tests.log`. Clock coverage includes minute/hour and multi-digit
  carries, 43-point glyph geometry, final idle and unchanged zero-allocation assertions.
- App Clippy passes all four packages/all targets with native interop and warnings denied in
  **2.51 s**: `/tmp/localwave-motion-prefetch-clippy.log`. Child-wide Clippy is blocked by pre-existing
  warnings in `cfmt`, `native_layer_token_sys`, and `font_sys`; the failed log is retained as
  `/tmp/localwave-motion-engine-clippy.log`.
- Existing gesture/idle geometry checks use injected clock settlement within their original
  number of setup frames. Fractional point differences are compared within 0.001 point; strict
  performance thresholds and their warm-up remain unchanged. Earlier failing logs remain under
  `/tmp/localwave-motion-ui-*.log`.

### Artwork prefetch discovered during signed qualification

A quiet signed gallery run exposed **12 warm allocations / two warm requests**; another trace
kept four requests returning every scroll cycle without completing. Logs remain
`/tmp/localwave-motion-performance-final-quiet.log` and
`/tmp/localwave-motion-gallery-late-trace.log`. The larger adaptive cards exposed a real worker
policy bug: an empty/replaced visible packet discarded already-selected prefetch and rejected
its completed decode. Brief visibility could cancel the same covers on every cycle.

The existing application artwork worker now prioritizes new visible keys and retains unfinished
selected keys in spare slots of the same eight-key packet. Completed same-epoch decodes are
published even after a brief viewport change. Explicit cancellation remains separate and sticky
until consumed; epoch changes discard old work. The eight-result / 8 MiB decoded bound is
unchanged, with no additional threads or unbounded queue. A deterministic callback pauses after
the first thumbnail, replaces visibility with an empty packet, then requires all eight selected
results before any poll. It fails before the fix (**1.025 s case**) and passes after it; existing
cancel/restart and batch-retirement cases also pass. Logs:
`/tmp/localwave-motion-prefetch-red.log`, `/tmp/localwave-motion-prefetch-fixed.log`.

Three signed scrolling gallery runs now all have **zero warm allocations and zero warm requests**,
19 image quads and 5,386,240 texture bytes. Last uploads occur at frames **64 / 23 / 24**, before
the unchanged frame-82 boundary. Logs: `/tmp/localwave-motion-prefetch-gallery-{1,2,3}.log`.
Every run still fails the strict wall maximum (**1,739 / 2,297 / 2,538 µs**).

The signed idle check now waits for startup motion to finish before its existing 250 ms quiet
settling and full 500 ms no-build/no-draw observation. Its five-second total timeout is unchanged;
injected-clock tests still reject extra idle frames and endless animation. The queue-drag workload
uses visible accessibility handles so moving/new layout geometry cannot invalidate fixed click
coordinates. It retains the same occurrence/capture/cross-page-scroll/cancel assertions, 182 total
frames, frame-82 warm-up, 100 measured frames and both Rust allocator counters.

### Final signed checkpoint

`/tmp/localwave-motion-prefetch-performance-final.log` records the final warm lib/app builds in
**0.67 / 0.06 s**. The signed sandbox bundle passes minimum OS, default executable stack, bundled
dylib ID/rpath and launch checks. Startup animates for 32 actual draws, then the full idle check
observes **510.0 ms** with no builds/draws. All **ten workloads have zero warm Rust allocations**;
gallery also has zero warm requests. The strict **1,000 µs wall maximum fails in every workload**
and `make performance` correctly returns nonzero. No warm-up or performance limit was relaxed.

| Workload | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- |
| songs-original | 3358 / 597 / 1720 / 2087 | 713 / 557 / 672 / 712 |
| find-filters-title-descending | 3125 / 611 / 1757 / 2431 | 799 / 569 / 682 / 779 |
| queue-occurrences | 2534 / 620 / 1642 / 2363 | 925 / 571 / 676 / 791 |
| queue-carry-scroll-cancel | 2587 / 492 / 2134 / 2574 | 1995 / 471 / 789 / 1833 |
| full-queue-playlist-editor | 2518 / 708 / 2163 / 2468 | 1047 / 592 / 866 / 956 |
| albums-artwork | 2976 / 609 / 1976 / 2582 | 996 / 546 / 897 / 953 |
| waveform-scrub-and-scroll | 1987 / 550 / 1667 / 1978 | 677 / 379 / 629 / 662 |
| settings-menus-appearance | 2370 / 321 / 1335 / 2058 | 516 / 271 / 464 / 502 |
| files-indexed-directory | 3050 / 346 / 1415 / 2387 | 427 / 308 / 385 / 408 |
| external-drop-hover-and-cancel | 3140 / 296 / 1079 / 2931 | 379 / 272 / 345 / 378 |

Earlier smoke/startup and stale-coordinate failures remain in `/tmp/localwave-motion-performance.log`,
`/tmp/localwave-motion-performance-idle-fixed.log` and `/tmp/localwave-motion-performance-final.log`.
The intermediate ten-workload run `/tmp/localwave-motion-performance-qualified.log` overlapped the
app test matrix and is retained as contended evidence; the final run above had no simultaneous test
runner. None of these logs establishes strict wall latency or native/process allocation freedom.

Still required: matched native SwiftUI/Rust captures and motion samples, desktop table column
metrics, complete album/artist/list detail layouts, compact navigation/layout, all overlays,
waveform/queue motion and hardware/native-memory qualification. The original live GUI, database
and preferences remain untouched. The strict 1,000 µs wall gate remains open.

## Finder drop checkpoint

Finder drops now accept mixed original folders and supported audio anywhere in the window. The
host accumulates file events until SDL's completion event, across pump boundaries, and retains the
known account from first hover/file. A first drop opens the lazy Files worker and resolves the
actual current identity there. The [SDL event definitions](https://wiki.libsdl.org/SDL3/SDL_EventType)
define completion of a drop set; the pinned Cocoa source also sends completion on drag exit.
Keyboard focus loss dismisses the hover without discarding files Finder may still deliver.
Invalid/oversized batches fail together; an occupied Files mutation slot reports a visible retry
message. There is no unbounded import queue.

The existing selection service now handles picker audio and mixed drops. It validates the entire
batch, including folder grants/generated-path exclusions, before copying. Audio copies first;
selected folders and then Added Files pass through the same registration, scan and import path.
Playback follows Swift: selected audio first in its supplied order, then each selected folder's
localized natural relative-path order, with shared song keys deduplicated. A minimal SQL join
returns current indexed song IDs/keys/paths once per folder, excluding stale and foreign paths
without hydrating tags/bookmarks or doing a queue-building query per track. Cancelling a scan
retains its previous complete index; completed imports/private files remain.

Accepted drops open Files so progress and Cancel are available. Successful drop results carry
the retained **Dropped music** context through the existing snapshot/controller handoff. Empty
folders navigate Files without clearing the current queue or starting playback. Stale accounts
are rejected. The passive **Drop to play** overlay retains the Swift instructions and dashed
border, stays outside layout/hit handling and has accessible instructions on the window node.
Warm overlay/cancel cycles at both minimum and full width allocate no Rust memory.

### SDL source ownership correction

Checking the actual pinned SDL 3.4.18 path exposed a native lifetime error: SDL creates temporary
drop strings and reclaims them on a later pump, but the bridge also freed the source after copying.
The bridge now owns only its copied payload. The regression uses a valid static borrowed event
string, avoiding reliance on private SDL symbols and deterministically detecting an accidental
source free. It still verifies that later source mutation cannot change the retained Rust path.
The pre-fix check aborts with **SIGABRT** in **4.629 s**; after the fix all **19 SDL/host checks pass
in 6.722 s wall / 2.516 s summed**, zero skips. The dependency gate passes. Logs:
`/tmp/localwave-drop-sdl-owned-red.log`, `/tmp/localwave-drop-sdl-fixed.log`, and
`/tmp/localwave-drop-pxui-deps.log`. This is published in pxui before the parent pin; ABI 1.15 is
unchanged because the correction is inside its statically linked SDL bridge.

The tenth signed workload uses actual external hover/completion events and checks the drawn
overlay rectangle on every frame. Exactly 50 of the 100 measured frames must contain the overlay;
empty completion cycles must submit no import jobs or application actions. It preserves the
1440×872 window, 10,000 songs, 182 total / 82 warm / 100 measured frames and both allocator counters.
The original nine workloads and the strict 1,000 µs wall maximum remain intact.

### Controller fairness and checkpoint evidence

The final native smoke exposed a real scheduling bug: continuous volume/seek delivery could consume
all worker turns before upcoming-item preparation. Controls now alternate with ready preparation,
and volume and seek alternate with each other. Stop/reset/queue fencing retain priority; deferred
or cancelled preparation gives controls their next turn. A deterministic regression delivers changed
controls after every acknowledgement: the old scheduler prepares only one of three items; the fix
prepares all three while delivering both control kinds. No waits, deadlines or assertions were relaxed.
The failed full run remains `/tmp/localwave-drop-final-border-tests.log`; deterministic red/green
are `/tmp/localwave-control-fairness-red.log` and `/tmp/localwave-control-fairness-fixed.log`.

All **155 checks pass in 16.190 s wall / 14.658 s summed**, zero skips, in
`/tmp/localwave-drop-and-control-final-tests.log`. The native controller takes **1.972 s**.
All four packages/all targets pass Clippy with native interop enabled and warnings denied in
**2.28 s** (`/tmp/localwave-drop-motion-clippy.log`). The overlay uses the reference six-point
dash and six-point gap, checked at both window sizes.

The final signed run (`/tmp/localwave-drop-and-control-final-performance.log`) passes bundle
minimum OS/default stack/dylib/rpath and launch checks; smoke draws once and idles **500.6 ms**.
All **ten workloads have zero warm Rust allocations**. Every strict **1,000 µs wall maximum
still fails** and `make performance` returns nonzero; warm-up and thresholds are unchanged.

| Workload | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- |
| Songs | 3086 / 479 / 2134 / 2561 | 628 / 432 / 558 / 586 |
| Find | 2112 / 424 / 1144 / 1961 | 520 / 403 / 481 / 520 |
| Queue | 2009 / 511 / 1620 / 1792 | 615 / 456 / 550 / 587 |
| Queue drag | 2696 / 354 / 1655 / 2419 | 1800 / 331 / 638 / 1647 |
| Queue sheet | 2486 / 617 / 1950 / 2386 | 805 / 501 / 706 / 762 |
| Gallery | 2305 / 502 / 1616 / 2221 | 747 / 467 / 574 / 664 |
| Waveform | 2118 / 407 / 1543 / 2074 | 596 / 382 / 514 / 565 |
| Settings | 2653 / 289 / 1282 / 2378 | 524 / 271 / 450 / 509 |
| Files | 2101 / 320 / 411 / 618 | 372 / 302 / 347 / 360 |
| Drop | 1944 / 189 / 710 / 1825 | 231 / 181 / 217 / 225 |

Live Finder sandbox extensions, picker/provider/account transitions, native memory, device behavior,
source observation and the full port remain unqualified or unfinished. Native event injection and
real worker imports do not establish those platform capabilities. The existing normal GUI, database
and defaults were preserved throughout this work.

## Audio selection checkpoint

Files now exposes **Add audio files** through SDL's asynchronous multiple-file picker. Matching
request IDs are consumed once, cancellation stays separate from errors, and the captured account
is revalidated by the Files worker. The worker validates the entire selection before copying,
then holds each resolved security scope through the existing coordinated bounded-memory stream.
Original filenames and extension case are retained. Identical named content is reused; different
content gets an exclusive UUID-prefixed destination. Symlinks are not followed when comparing
existing destinations. Cancellation, callback failure and source-version changes remove staging
files; completed private files and original audio are retained. Named copies skip unused hashing.

The existing source registration, scan and import services populate **Added Files**. Selection
results preserve picker order and deduplicate the repository's shared song identities. Those
identities can share a final source path, matching Swift's compacted lookup behavior. Private
paths are attached under the cache guard, so prepared playback opens Added Files directly rather
than making another managed Music copy. Favorites and playlist membership survive re-selection.
The worker publishes retained song/context payloads through its existing snapshot retirement;
the host retries autoplay delivery until the bounded controller accepts it. Account changes clear
pending selection payloads. Files navigates to the imported source; its toolbar still fits the
minimum 640-point window.

The real import check exposed a shared path-parser bug: `URLByStandardizingPath` rewrote
`/private/var` to `/var` after registration, changing indexed parent keys relative to the saved
root. The parser now uses `standardizedURL`, retaining that spelling while normalizing dot paths.
Regression checks cover both path forms and their scan identities. Earlier failed import/compiler
logs remain under `/tmp/localwave-audio-selection-*` and `/tmp/localwave-audio-picker-tests*`.

### Evidence and limits

- **150 checks pass in 19.626 s wall / 13.061 s summed**, zero skips. Fourteen cases were retried
  alone after exceeding their recorded contention baseline. Slowest case: native controller
  **2.554 s**, within the five-second case limit. The real selected-audio service takes **0.22 s**,
  the complete Files actor check **0.33 s**, and the UI action/scroll/minimum-window check **0.02 s**.
  Log: `/tmp/localwave-audio-picker-final-tests.log`. The earlier 150-case checkpoint passes in
  **15.172 s**: `/tmp/localwave-audio-picker-checkpoint-tests.log`.
- All four packages/all targets pass Clippy with interop enabled and warnings denied in
  **2.13 s**. Formatting, whitespace and the published pxui pin checks pass.
  Log: `/tmp/localwave-audio-picker-final-clippy.log`.
- `make performance` rebuilds libpxui/app in **0.70 / 20.97 s**. The signed sandbox bundle passes
  minimum OS, default executable stack, bundled dylib/rpath and launch checks. Smoke draws once,
  then remains idle **508.8 ms**. Log: `/tmp/localwave-audio-picker-performance.log`.
- All nine signed workloads retain **zero warm Rust allocations** with the unchanged 10,000-song,
  182-frame / 82-warm / 100-measured setup. Every strict **1,000 µs wall maximum still fails**;
  `make performance` correctly returns nonzero. CPU figures do not substitute for that gate.

| Workload | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- |
| Songs | 3217 / 570 / 2196 / 3188 | 575 / 445 / 535 / 569 |
| Find | 2572 / 438 / 1689 / 2423 | 575 / 408 / 511 / 543 |
| Queue | 2454 / 487 / 1571 / 2016 | 567 / 447 / 531 / 554 |
| Queue drag | 2052 / 355 / 1375 / 1889 | 1727 / 341 / 590 / 1716 |
| Queue sheet | 3385 / 658 / 2027 / 2734 | 989 / 506 / 756 / 838 |
| Gallery | 1558 / 477 / 956 / 1500 | 768 / 440 / 564 / 617 |
| Waveform | 1981 / 397 / 1713 / 1970 | 549 / 386 / 488 / 524 |
| Settings | 2316 / 302 / 1657 / 2262 | 549 / 278 / 483 / 509 |
| Files | 2656 / 335 / 1836 / 2486 | 445 / 310 / 391 / 425 |

Gallery retains 18 image quads, 3,756,032 texture bytes and zero warm requests. Waveform keeps
200 bars and nine scrub commits; Settings makes 45 actions across eight themes; Files retains
10,000 indexed paths and 14 drawn row surfaces. Cold selection/import/copy and native allocations
are outside warm Rust qualification. Live signed picker grants, cloud providers, account changes,
native memory and devices remain unqualified. Live external drop grants, observation and the full port remain
unqualified or unfinished; the later Finder drop checkpoint adds the input/import flow. The existing normal GUI was preserved throughout this checkpoint.

Source: [LocalWave Rust Port, version 2](https://artifacts.microapps.space/a/w474jy87pnxg), saved verbatim as [rust-port-reference.html](rust-port-reference.html). The reference file SHA-256 is `32178251cf3c64b313a073277eeda58c1b60a897df229aeeefba9aafff9c1301`. Baselines: LocalWave `af73743`; pxui `59d6f4540bc9eb7bffe981af0578cd286920f55c`.

## Build and checks

```sh
git submodule update --init pxui
make build                         # release (PROFILE=debug for development)
make bundle                        # target/bundle/LocalWave.app
open target/bundle/LocalWave.app
make performance                   # real signed Metal frames, allocation/timing/idle gates
tools/check-macos-bundle.sh         # dependencies, signature, entitlements, real Metal window
cargo t -p lw_core -p lw_apple_sys -p lw_ui -p lw_bin
cargo run -p lw_bin --bin localwave-playback -- --mute /absolute/path/to/audio.flac
cargo run --locked -p lw_core --example library_bench
cargo run --locked -p lw_core --example catalog_bench
cargo run --locked -p lw_core --example cache_bench
tools/check-database-compatibility.sh
tools/check-pxui-remote.sh --self-test
tools/check-pxui-remote.sh
swift test --parallel
```

Rust 1.99.1 is pinned. Xcode, CMake, pkg-config and curl are required. Static SDL3 3.4.18 is downloaded from libsdl.org with a pinned SHA-256 and built under `target/sdl`; warm builds reuse it. Only `libpxui.1.dylib` is bundled as a non-system dylib. The executable owns SDL, Metal, the frame loop and renderer lifetime; app views use the public pxui wrapper. Root and pxui share one linker wrapper to preserve the shared build cache; LocalWave explicitly opts out of its legacy stack override, and the bundle verifier requires the default executable stack. Native and Rust builds explicitly target macOS 15.0; the bundle verifier checks both Mach-O minimums and the plist. Running on macOS 15 remains unqualified. Run formatters with explicit app package names (`cargo fmt -p lw_core -p lw_apple_sys -p lw_ui -p lw_bin`): `--all` also formats pxui path dependencies, which must retain their upstream formatting.

`make install APPDIR='/Applications/LocalWave Rust Port'` installs the scaffold without replacing the shipping app. Installation refuses to overwrite an existing app. The default signature is ad hoc, with sandbox, user-selected read-only files and app-scoped bookmark entitlements. Set `CODESIGN_IDENTITY` to the shipping app's signing identity for a real migration build; same-team sandbox/bookmark continuity has not been qualified. `PROVISIONING_PROFILE` embeds a supplied Mac profile. Store submission, profile validation, versioning and distribution remain P5 work.

## Gate status

| Phase | Current implementation | Acceptance still required |
| --- | --- | --- |
| P0: scaffold/bundle | Four-crate Rust workspace, pinned pxui submodule, static SDL3, dynamic pxui, sandbox bundle, remote-pointer check, Rust CI job, VS Code build/debug tasks. Terminal app, target, SwiftTUI dependency and obsolete terminal instructions removed. | Developer/team signature and provisioning qualification remain. Local ad hoc scaffold checks pass; see evidence below. |
| P1: core | Song model and raw states, masked UTF-8 FNV identity, Swift-compatible scalar lowercase, complete song hydration, additive song migrations, transactional song upsert and FTS updates, BM25 prefix search, Favorites, guarded cache updates, native FLAC comments/pictures, occurrence-based queue/reorder/shuffle/repeat/Clear/Undo and two-item prequeue selection. User/source/path/playlist repositories, transactional metadata/FTS removal, guarded bookmark/root replacement and source selection, SQLite.swift-compatible UTC text dates, indexed path rescans, album grouping without correlated rescans, and duplicate-preserving bulk reads without redundant Song copies. System SQLite through safe FFI. Indexed recursive import selection, transactional song/FTS/digest commits, pure import policy with progress/cancellation, cloud placeholders, verified-cache enrichment/invalidation, unchanged-file skipping and Unicode track-prefix cleanup. Thread-confined application scan/import service, generated-path exclusions, native bookmark/pin/cache-proof preferences, atomic scan publication and legacy-song relocation. LRU cache policy and a serialized application copy/eviction service with native coordinated copies and pin/playback protection. Native queue/volume/settings persistence, typed Swift-compatible playback contexts, duplicate-aware queue restoration, isolated UI-test suites and validated waveform JSON/SQLite storage. The application waveform service uses coordinated PCM extraction and Swift-compatible source signatures. | Remaining import/source command integration and source observation; port coverage from all 18 SwiftPM test files. Fixture exchange does not qualify a full user library. |
| P2: Apple playback | Native Foundation URL normalization and security-scoped bookmark creation/resolution/renewal, balanced RAII access grants, core relative-path containment checks, worker-only availability probes and coordinated AVFoundation tag/measurement reads, streaming SHA-256, cancellation and post-read version checks exist. The core FLAC parser runs inside the same coordinated grant. Coordinated offline copies stream and hash with cancellation, version guards and network simulation checks. Native waveform extraction uses a fixed 64 KiB PCM buffer and checks source versions before/during/after the coordinated read. Worker-prepared AVPlayerItems feed a main-thread AVQueuePlayer with two upcoming items, incremental refill, bounded events and a headless local-file host. A pure Rust transport tracks playback intent, stale-load generations, completion and interruption intent. A queue session now adds preparation tickets, queue edits, repeat/shuffle, bounded occurrence-based failure traversal and prequeue reconciliation. Library preparation uses coordinated materialization and explicit cache leases through native retirement. Native one-shot start events feed a worker history service with Swift-compatible play counts/dates and safe managed-cache recency updates. The GUI now uses a bounded application controller for worker-owned queue operations, cancellation, native installation/retirement acknowledgements, history and queue/index/volume persistence. The GUI publishes Now Playing metadata/artwork and routes seven bounded MediaPlayer commands through that controller. Guarded preparation/native failure writes, actual-start recovery and worker catalog invalidation now preserve newer cache/import records. | Complete library/UI integration and native decoder/provider failure qualification, physical media-key/Control Center qualification, interruption handling, coordinated playback access, provider observation, pickers/share; sandboxed same-team upgrade and device bookmark qualification. |
| P3: pxui fixes | Local Metal compatibility fixes, themes/fonts, thread-safe wake, retained input/text/draw storage and reserved-key guards. Fixes 1–2: inline sampled/stepped curves, fixed-capacity keyframes, anchored independent-axis view scaling, nested hit/accessibility geometry and static/dynamic allocator checks. Scale-only runs update the existing transform table without rewriting instances. Fix 3 exposes backdrop saturation and resize-time scratch preparation through the ABI and Metal/Vulkan shaders, with native Metal readbacks, reusable prepared passes, retained large-layer boundaries and zero warm Rust allocations in tested RGBA/BGRA blurred draws. Fix 5 adds validated OpenType tabular-number flags with independent cached proportional/tabular layouts and static/dynamic warm allocation checks. Fix 6 adds eight inline spans on text nodes/canvases, preserving accessibility labels, ligature coverage and warm allocation-free drawing. Fix 7 exposes head/middle ellipsis with cache, accessible-label, raster and native golden/allocation regressions. Fix 8 includes Apple native font discovery on missing clusters, file mapping, compatibility checks and two lazy fixed pages each for mask/colour glyphs, with stable handles/UVs and warm script/emoji pixel/allocation tests. Fix 9 adds versioned frame appearance/motion inputs, cached theme selection and Apple notification delivery with running-motion cancellation. Fix 10 exposes retained view opacity/scale/clip and measured virtual lists with static/dynamic and native pixel/allocation regressions. All current engine stages are published/pinned on rust-port. ABI 1.13 adds shared bounded image identities, retained GPU wrappers and completed-serial release; ABI 1.14 adds fractional fills and output partition capacity reuse. | Finish fix 3 platform qualification, qualify macOS 15 runtime, then fix 4 if measured necessary, finish fix 8 platform/font parity and remaining appearance/platform qualification; gallery LocalWave theme. Image atlas and group opacity require the plan's measurements before deciding whether needed. |
| P4: Mac UI | Worker-prepared sidebar collections, virtual 38-point song rows, album/artist navigation, selectable rows and double-click/Return playback, 319-point Now panel and 71-point transport with cached clocks, seek/volume, repeat/shuffle and panel controls. An immutable worker-published Up next panel supports occurrence-based play/remove, cross-page drag reordering and edge scrolling, Clear/Undo and shared cursor-only snapshots. Queue/context payloads are retained on the worker. Manual refresh reads a consistent catalog, preserves selection/navigation identity and retires replaced snapshots on the worker. Worker-backed token-filtered Find with wrapping chips and Title/Artist/Album/Time sorting share immutable labels. Full queue editing and Save as playlist, native resize/zoom/full-size transparent title bar, worker-loaded Album/Now/queue covers and cached/native-decoded transport waveform with release-only scrubbing are implemented. Settings/About and appearance/cache controls are connected. Files adds a native asynchronous folder picker, separate scan/import worker, account-scoped source/directory browsing, recursive import, rescan and cancellation, plus options, reconnect and confirmed source removal. Audio selections copy safely into Added Files, open that source and autoplay the retained selection without a second managed-cache copy. Finder drops support mixed audio/folders, a passive overlay, cancellable import and ordered playback. Warm repeated 10,000-song scroll frames and clocks allocate no Rust objects. After removing the one-result artwork bottleneck, three signed gallery runs also have zero warm allocations and pending requests. | Waveform reveal/stagger, Find album/artist/folder result sections, remaining columns, multiselection/edit/import and remaining Settings actions, other presenters, shortcuts, song/playlist drag/drop, live external drop grant qualification, automatic source observation, 1440×872 goldens and the strict 10,000-song timing gate. |
| P5: ship Mac | Shipping Swift release pipeline retained. | Rust TestFlight/distribution pipeline, one release cycle without reported regressions, then remove Swift Mac target. |
| P6: pxui iOS | No changes. | Static SDL device/simulator spike, Metal/host/touch port, safe areas/keyboard, display link/background lifecycle, Dynamic Type, framework/xcframework packaging and every device feel/performance gate. |
| P7: iOS UI/ship | Shipping Swift iOS app retained. | Complete narrow UI, native pickers/share/route overlay, 390×778 goldens and physical-device playback/bookmark/cloud qualification; only then delete Xcode and Swift sources. |

Accessibility names are set as views are created. Native macOS/iOS accessibility bridges are explicitly deferred by the supplied plan; the Rust scaffold does not provide VoiceOver parity. The shipping Swift app must remain available during migration.

## Current evidence — 13 November 2026

- `make bundle`: release app builds, signs and passes `codesign --verify --deep --strict`. `otool -L` shows `@rpath/libpxui.1.dylib`, system libraries/frameworks, and no SDL/Homebrew dylib. The bundle contains one framework dylib. Its three required sandbox entitlements are true.
- The sandboxed app executed its real SDL/Metal smoke path from the build bundle and through LaunchServices from `/Applications/LocalWave Rust Port/LocalWave.app`. Both reported `frames=1 draws=1 size=1440x872` and exited successfully after the idle observation interval. This proves the P0 window, not music playback or a 10,000-song frame budget.
- All **457 bounded Rust checks pass**: 106 text/static-wrapper/ABI/type cases in **4.222 seconds wall** (4.155 summed), 41 dynamic-wrapper cases in **4.320 seconds** (1.272 summed), 229 core/Metal cases in **6.816 seconds** (9.859 summed), and 81 app cases in **9.459 seconds** (7.435 summed), zero skips. New coverage includes real appearance KVO, isolated workspace notifications/teardown, cached reads/themes, legacy frame allocations, active motion cancellation, retained-view invalidation and native pixels. The latest signed 10,000-song workload **fails** the unchanged 1 ms wall maximum at **2,314 µs**, with zero warm Rust allocations across 182 frames/draws (wall p50/p95/p99 275/712/1,527 µs; CPU p50/p95/p99/max 263/321/340/364 µs). The previous 854 µs pass and earlier failures remain evidence; the cause of the wall/CPU gap is unproven. Cold shaping/page growth, unfinished interactions, native memory and other platform/runtime qualification remain open. Logs: `/tmp/localwave-rust-system-preferences-{static-complete,dynamic,render-core-final,app-final,performance}.log`.
- `tools/check-database-compatibility.sh` passed both cases in 0.815 seconds after its build: the actual Swift repositories create a temporary `musicApp29.sqlite`; Rust reads/updates it and inserts a song; Swift reads both, verifies FTS and retains playlist membership, Favorites, measurements and bookmark bytes. It also exchanges source/path records, playlist order and millisecond UTC Date values. In the same XCTest process, Swift creates a real security-scoped folder bookmark, renames the physical folder, and Rust resolves and reads it; Swift resolves a bookmark created by Rust too. A separate executable cannot generally consume app-scoped bookmarks from another app identity; this test does not qualify a signed Swift-to-Rust app upgrade. Fourteen filename-title vectors also check ASCII, Arabic, Devanagari/fullwidth decimal prefixes, Roman/superscript exclusions, whitespace, hidden names and extension cleanup against the actual Swift importer. Fourteen cross-language identity vectors include Greek, Turkish, decomposed/composed accents, Arabic, CJK, emoji, punctuation and path normalization. Greek sigma exposed a real lowercase incompatibility, fixed with scalar lowercase and pinned by a Rust regression.
- The native FLAC fixture's tags and every truncated prefix are checked. A separate constructed picture verifies embedded artwork; the native fixture itself has no picture block. SQLite tests verify embedded NUL text, empty blobs, typed reads, finite numeric binds, parameter counts, column bounds, rollback, multiple-statement rejection and read-only errors. Source/playlist tests inject write failures and verify rollback includes metadata and FTS. Physical relative-path checks reject escaping and dangling symlinks and follow a renamed root.
- Native recursive import reads real FLAC and M4A fixtures after renaming a bookmarked physical source folder. It verifies tags, measurements, digest persistence and a second import that skips unchanged bytes. A deliberate write inside the coordinated callback is rejected before publishing metadata/digest. This exposed NSURL resource-attribute caching; the post-read probe now clears it. Native metadata snapshots own length-delimited UTF-8, preserving an embedded NUL in a real modified M4A tag. Callback errors propagate, and callback panics unwind only after returning from Objective-C.
- Import policy tests use injected availability/cache proofs to verify cloud placeholders enrich the same ID, retain Favorites/playlist membership and invalidate stale cache metadata without deleting files. Cancellation during progress and after a read leaves song/digest/FTS uncommitted. Invalid import dates are rejected. Native provider probes and preference cache proofs now feed the application adapter; real cloud-provider qualification and source observation remain outstanding.
- A worker-only native folder enumerator now borrows the source's security-scoped grant, retains registered-root identities, skips hidden/package contents and explicit generated paths, and rejects dangling or escaping symlinks. Its real-folder regression passes, including an original `Collection/Music` folder alongside generated Music paths. Pruning is limited to directories: calling Foundation's `skipDescendants` on file leaves could omit the next sibling folder. A separate repository regression verifies complete snapshot publication preserves existing hashes/sizes and original creation dates, removes obsolete FTS rows, rejects stale/duplicate snapshots and rolls back failed publication. These components now feed the application service described below. Logs: `/tmp/localwave-rust-source-scan-fixed.log`, `/tmp/localwave-rust-source-scan-tests.log` (the latter contains the initial, subsequently fixed native failure).
- Indexed recursive selection handles a 1,300-directory graph containing a cycle, overlapping selections across the parameter-batch boundary and shared path hashes in separate sources. Injected digest-write failures roll back song and FTS writes; stale path removal cannot resurrect imported rows.
- `lw_bin::library::LibraryWorker` owns SQLite, native preferences and one live grant per resolved source. Construction on the main thread is rejected, and its native handles cannot move between threads. It reads legacy defaults bookmarks, persists renewal, and atomically couples moved-root updates with legacy song location migration; stale updates and injected write failures preserve both. A real-file test scans/cancels/renames, imports FLAC with a constructed picture plus M4A with native artwork, preserves song IDs/Favorites/playlist membership, queues a pinned song for copy and skips both files on unchanged reimport. Artwork uses the shipping Mac signature check, SHA-256 names and atomic writes; repeated artwork does not create another file. This is an application service, not yet a launched command worker or an interactive music UI. Log: `/tmp/localwave-rust-worker-final-tests.log`.
- Native defaults preserve the existing decimal bookmark keys, string song IDs under `lwPinnedSongs`, and `lwCacheSourceVersions` records with canonical cache paths and optional source modification dates/sizes. Each import retains one immutable snapshot with set/dictionary lookups, rather than rebuilding the full pin set for every file. Tests check cross-handle persistence, snapshot consistency, matching/mismatching/absent cache proofs and strict local file URL decoding (including encoded-NUL rejection). The application worker test also creates a local provider-shaped path classified by the real native probe as unknown: import defers without opening it, a recorded matching local cache enriches the same placeholder, and a mismatching version invalidates metadata and queues a pinned copy without deleting the cache file. This does not emulate a live provider or qualify iCloud. Offline copy/eviction now uses the same preference schema. Queue IDs/index, optional volume, opaque Swift Codable playback-context bytes and the seven settings now round-trip through native defaults and the application worker. Typed context interpretation and basic playback integration are now implemented; see the later controller and context checkpoints. These workers allocate; they stay outside the zero-allocation frame contract.
- The `rust-port` branch pins the engine patches published on PXUI’s `rust-port` branch, including the three Metal compatibility files. The remote-pointer check verifies that the exact staged engine commit is fetchable before publishing LocalWave. Its self-test accepts published commits and rejects unpushed ones. Continue work on `rust-port` in both repositories and publish the submodule before its parent pointer.
- Native offline copies use one 256 KiB streaming buffer, SHA-256 names, exclusive atomic promotion and source size/modification checks. Tests verify all chunks against a fixed digest, eight concurrent identical copies, cancellation before/during copy, callback errors/panics, source mutation, missing input, corrupt/symlink destinations and a mid-copy network simulation change. Completed duplicates are byte-checked; partial files are removed on errors. The local provider-shaped fixture is rejected before destination creation during simulation and succeeds after simulation is disabled; this does not qualify real provider downloads. Log: `/tmp/localwave-rust-offline-copy-tests.log`.
- The application copy queue processes explicit pending requests and retries failed requests only when pinned. Its real-file check verifies persisted source-version proofs, content deduplication, cancellation retaining the pending state, error/retry states, and both source-relative and legacy bookmark access. Core LRU policy drives eviction of regular files directly inside the managed Music directory; symlinks, partial files, subdirectories and original files outside it are preserved. Any pinned owner, the active song, active cache path or queued path protects a file, including files not yet in the database; protected data may exceed the quota. A recursive native lock serializes protection checks/deletion with preference writes. Protection snapshots refresh on native setter generation changes rather than rebuilding all sets for every file. Copy and eviction requests must remain on the single library worker; parallel materialization is not implemented. Log: `/tmp/localwave-rust-offline-final-tests.log`.
- A development-profile, in-memory 10,000-song cache benchmark measured owner projection at **1.60 ms / 10,014 Rust allocations**, versus full song hydration at **20.83 ms / 60,322 allocations** for the same fixture (six nonempty strings per complete song, one per owner). Pure LRU sorting/removal policy took **0.73 ms / zero Rust allocations**. Under-quota checks skip owner hydration entirely. Filesystem/provider latency and native allocations are outside this benchmark. Log: `/tmp/localwave-rust-offline-cache-bench.log`.
- The later full Swift shared run still failed the same two unchanged file-presenter notification cases (48 scheduled cases, warm build 1.74 seconds); this is not a passing shared-suite result. The dedicated Swift/Rust database/bookmark exchange passed separately in 0.815 seconds. Logs: `/tmp/localwave-rust-offline-swift-shared.log`, `/tmp/localwave-rust-offline-exchange.log`.
- Initial Swift shared run: 46 existing tests, two failures in unchanged file-presenter notification cases (`testNonAudioMetadataChangeReachesSourceWatcher`, `testCoordinatedChildWriteNotifiesAndInvalidationStopsDelivery`). They timed out waiting for OS callbacks. No passing full shared-suite claim is made. The cold build took 79.44 seconds; the 30-second shared budget applies to warm builds.
- The retained Swift macOS app builds successfully with `xcodebuild -project localwave.xcodeproj -scheme musicapp -destination platform=macOS CODE_SIGNING_ALLOWED=NO build`. App-package Rust Clippy checks pass with warnings denied. Two existing generated Metal enum static-assert availability warnings appear with the macOS 15 deployment target; generator cleanup remains P3 work.
- A warm full Swift shared recheck again reported the same two file-presenter timeouts; the Rust exchange test is skipped in the ordinary run and passes when invoked by its dedicated gate.
- CI configuration has been added; no remote CI run or release has been performed for this port.

## Required performance contract

Warm frames must allocate **zero Rust heap objects across the app and libpxui**, build UI in **less than 1 ms**, and render **zero frames while idle**. The full acceptance workload remains a 10,000-song table, waveform scrubbing and menus. Database/import/artwork work stays on workers; strings, column widths, visible-row data, queues and caches must have explicit lifetimes and bounds. Do not move allocation into a helper or across the dynamic-library boundary to make a counter pass.

`make performance` builds/signs the release app, verifies its sandbox/dependencies, checks a half-second idle interval, then runs 182 actual Metal frames (two geometry frames, 80 warm wheel frames, 100 counted). It fails if any warm input/build/draw iteration allocates, any UI build reaches 1 ms, or the frame count is incomplete. It reports wall-time and thread CPU-time distributions and counts both Rust allocator copies. Synthetic pointer/wheel events now scroll the actual 10,000-song table with changing clocks; real SDL input has a separate bounded integration check. Scrub/menu traces and first-time text shaping remain unqualified. The wall timer encloses the UI build, with the thread-CPU probe outside that timed interval. Native Metal/OS allocations are outside this Rust counter; Instruments is still required for process/native memory qualification.

The host now retains one render-pass descriptor, renders directly into BGRA drawables, and uses three bounded submission slots. It no longer creates Rust command/encoder/attachment wrappers or an offscreen copy per frame, and no longer synchronously waits for every submission. A slot waits before reuse; shutdown waits for all work before releasing renderer resources. This leaves the same OGPU-owned device/queue and public pxui renderer boundary.

An earlier signed-bundle run passed with **120 draws, 0 warm Rust allocations**, wall build **33 µs median / 210 µs maximum**. A subsequent run correctly **failed** the strict wall-time gate: **37 µs median / 207 µs p95 / 650 µs p99 / 1,671 µs maximum**, while thread CPU peaked at **86 µs**; allocations remained zero. The elapsed/CPU gap is consistent with scheduling or blocking, but has not been traced. Earlier exploratory wall maxima were 1.371–2.356 ms too. The threshold remains strict; one passing run does not establish a stable timing guarantee. These are scaffold measurements, not evidence for the still-unimplemented music screens. Failure log: `/tmp/localwave-rust-core-final-performance.log`.

After retaining host input buffers and moving the CPU-clock probe outside the UI wall timer, the signed pointer/wheel workload passed: **120 draws, 0 warm Rust allocations**, UI wall build **35 µs median / 186 µs p95 / 326 µs p99 / 350 µs maximum**, thread CPU **35 µs median / 56 µs maximum**. The independent idle check passed with one initial frame. This single run does not resolve the earlier scheduling/blocking question. Log: `/tmp/localwave-rust-performance-input-gate.log`.

The later macOS-15-targeted signed run **failed** the wall gate again: **120 draws, 0 warm Rust allocations**, wall UI build **41 µs median / 256 µs p95 / 858 µs p99 / 1,587 µs maximum**, thread CPU **70 µs maximum**. This failure remains unresolved; no passing claim is made for stable wall latency. Log: `/tmp/localwave-rust-import-performance.log`. The independently rerun bundle check passed minimum-OS load commands, signature, sandbox/dependencies and one initial draw with **501.4 ms** of observed idle time. The smoke timer now starts after the first draw and observes a full half-second after settling; an injected-clock test proves slow startup cannot consume that interval. It also rejects idle redraws and a missing first draw. Log: `/tmp/localwave-rust-import-bundle.log`.

Worker benchmarks use actual system SQLite with in-memory fixtures: 10,000 source paths initially indexed in **161.83 ms**, changed path IDs rescanned in **582.23 ms**, with stable database IDs and exact FTS counts. For 10,000 songs across 100 albums, album lookup took **0.96 ms**, compared with **47.87 ms** for the previous correlated query with the same album index; a bulk read took **28.63 ms** and **50,322 Rust allocations** (five owned strings per fixture song plus query/result buffers), FTS **1.01 ms**, and adding 10,000 playlist members **339.30 ms**. Earlier exploratory comparisons of 4.36–7.87 seconds lacked that index and are not a valid production baseline. These worker operations allocate; they must stay outside frame construction. These are development-profile, single-run figures, not disk/provider or end-to-end budgets. Logs: `/tmp/localwave-rust-catalog-bench.log`, `/tmp/localwave-rust-library-bench.log`.

Host event encoding retains dialog text buffers when event counts fall and reuses their storage for subsequent results. A bounded test cycles empty, pointer/key and multilingual dialog-result batches with zero warm encoding allocations; CI checks it after the static SDL build. First-seen payload sizes and native/SDL event creation are not covered by that isolated assertion.

Local logs for this checkpoint are under `/tmp/localwave-rust-*.log`. They are development evidence, not committed artifacts. Re-run the commands above for current results.

After adding the application import service, the release app rebuilt successfully (10.38 seconds for the app Cargo build). The signed bundle passed minimum-OS commands, signature/dependency/sandbox checks and one initial draw with **509.1 ms** of observed idle time. Logs: `/tmp/localwave-rust-worker-bundle-build.log`, `/tmp/localwave-rust-worker-bundle-check.log`. The strict frame latency benchmark was not rerun; its latest recorded failure above remains unresolved.

## Next implementation work

Finish P1 against the live Swift repositories and fixtures, next covering source observation, guarded failure-state persistence and the remaining catalog/import/waveform command delivery. The native queue player is now connected to library materialization, failure traversal, queue edits and persistence through the application controller. The headless native host now plays supplied local-file queues; it is separate from that application controller. The application scan/import service now uses the native IO adapter; it is tested outside the app frame loop and is not yet connected to UI actions. Song tag updates now reuse a rowid map instead of scanning UNINDEXED FTS identifiers for every song; source-path rescans already use batched rowid lookup. Source/path persistence is connected to the application scan/import service; user/playlist actions and the UI bridge remain to be wired. The queue session and library preparation service are exercised together against the native engine in an isolated integration fixture; the GUI now uses that controller for transport; library, waveform and full queue presenters remain to be connected. Preserve the schema's existing date encodings: songs use numeric epoch values while source/playlist tables use SQLite.swift's text Date values. Do not assume all dates are numeric. Apple normalization stays in the native boundary because Foundation's file URL canonicalization can change UTF-8 spelling. Core hashes the normalized URL supplied by that boundary.

Then implement and test the P2 shim/state machine before wiring real actions into P4. Keep every remaining gate in the reference in scope, including device-only gates and the Mac TestFlight release cycle. Neither a clean build nor the scaffold allocator check establishes completion of the port. The core now has an additional zero-allocation regression that advances/prequeues a 10,000-song queue 10,000 times; this is not a 10,000-row UI test. Ordered bulk song reads use batches of 250 parameters instead of one query per song, with a 1,001-row batch-boundary/duplicate/missing-ID regression.

The offline-cache checkpoint also rebuilt and signed successfully (12.73 seconds for the app Cargo build). Its bundle verifier passed minimum OS, signature/dependency/sandbox checks and a single initial frame followed by **502.6 ms** idle. Logs: `/tmp/localwave-rust-offline-bundle-build.log`, `/tmp/localwave-rust-offline-bundle-check.log`. This does not change the unresolved frame latency result or qualify the unimplemented music screens.

The preferences/waveform checkpoint adds native Swift-compatible settings and player-state storage, including UI-test UUID suite isolation. Queue restoration preserves the selected duplicate occurrence when earlier songs are deleted. Settings/player writes validate values before changing defaults; muted volume remains distinct from a missing preference, and Swift Codable context bytes remain opaque and byte-preserved. The application worker round-trip also checks rejection without partial writes. Native defaults and database operations allocate on workers; they are not frame-loop APIs.

The waveform repository reads/writes the existing separate SQLite table and 512-float JSON blobs. Corrupt or oversized peak rows are discarded; decoding a fixed array passes 100 iterations with zero Rust heap allocations. At that checkpoint audio extraction, source signatures and application/UI integration remained outstanding; the later decoder checkpoint below adds the worker service, with UI requests still unwired. The dedicated Swift/Rust exchange now passes **three cases in 0.867 seconds**, including actual Swift JSONEncoder/JSONDecoder waveform values (negative zero, minimum subnormal and scientific magnitudes), duplicate queue IDs, Int64 bounds, muted volume, settings and opaque Codable context bytes. Log: `/tmp/localwave-rust-profile-exchange.log`.

Generated-path exclusions now include the actual `Caches/LocalWave/waveforms.sqlite` and its sidecars, retaining the legacy root-cache exclusions. The Swift scanner and fingerprint walker skip descendants only for excluded directories, preserving sibling folders after excluded files. The three focused exclusion checks pass, including a real scan with waveform sidecars and adjacent original audio. Log: `/tmp/localwave-rust-waveform-exclusion-tests.log`.

The earlier preferences checkpoint shared run scheduled 50 cases and failed the two prior OS file-presenter notification cases plus `testRootOnlySongPlaysFromCoordinatedCacheWithoutChildBookmark` (preload timeout); the shared gate is not passing. Log: `/tmp/localwave-rust-profile-swift-shared.log`. iOS unit verification was stopped at the 90-second limit while building assets, before running tests; no simulator-pass claim is made. Log: `/tmp/localwave-rust-profile-ios-unit.log`.

The playback preload case passed alone in 1.071 seconds after the concurrent builds finished. This isolated result does not resolve its full-suite failure or the watcher timeouts. Log: `/tmp/localwave-rust-profile-playback-recheck.log`.

The release app rebuilt in **13.27 seconds** and its signed bundle passed minimum OS, signature/dependency/sandbox checks and a single draw followed by **506.9 ms** idle. App-package Clippy with the interop feature, formatting and whitespace checks pass. Logs: `/tmp/localwave-rust-profile-bundle-build.log`, `/tmp/localwave-rust-profile-bundle-check.log`, `/tmp/localwave-rust-profile-final-clippy.log`. The strict frame-latency gate was not rerun and its earlier failure remains unresolved.

## Waveform decoder checkpoint

`lw_apple_sys::waveform` now performs AVAssetReader extraction inside a coordinated worker read. It loads audio tracks/duration with a bounded wait, checks cancellation through loading and PCM chunks, folds absolute peaks across all channels using sample timestamps, and normalizes the same 512 buckets as Swift. One fixed 64 KiB scratch buffer replaces the Swift decoder's per-sample-buffer array. Native AVFoundation allocations are not counted by the Rust allocator and still require Instruments qualification.

Source signatures preserve Swift's Foundation path spelling, shortest Double text (including signed, padded exponents), optional NSNumber size spelling and SHA-256. Managed content-addressed Music files use their hash rather than modification date, preserving waveform versions after LRU touches. Unconfirmed provider files are rejected before opening audio. Files outside the known folder list remain readable when they are not provider files, matching import policy. Before/after coordinated reads check file size, modification date and file identity; the expected resource version is checked again on the actual coordinated URL. Tests trigger cancellation after nonzero PCM has been accumulated and mutate an actively read file; neither returns completed peaks.

`LibraryWorker::waveform_peaks` lazily opens the existing `Caches/LocalWave/waveforms.sqlite`, resolves source-relative or legacy bookmarked input, reuses matching rows and persists successful extraction. Missing or unavailable audio retains existing cached peaks. Its real-file fixture checks decoded peaks, repeated cache hits, changed audio, corrupt rows, LRU touches, eviction/relaunch, both bookmark forms, cancellation and provider deferral. Requests are serialized with the library service; eight repeated viewers reuse one stored result. The application command bridge and concurrent viewer request delivery have not been implemented, so this is not a concurrent UI qualification.

The dedicated Swift/Rust check compares signatures and all decoded peaks against the shipping Swift decoder, using generated stereo CAF at four modification dates, managed content-addressed files, AAC and FLAC. The ordinary Rust checks cover fixed peak JSON, native PCM and application persistence; the latest 47-case run took 7.323 seconds wall, with the slowest case 0.400 seconds. Logs: `/tmp/localwave-rust-waveform-final-tests.log`, `/tmp/localwave-rust-waveform-final-exchange.log`.

The safe decoder retains the probed source path and local-copy status with its version, so extraction cannot accidentally use a different path with the same size/date. After version preparation, the successful native decode wrapper passes a zero-Rust-allocation assertion; Foundation/AVFoundation memory and database/signature work remain outside that measurement.

The final waveform interop run passed all four cases in **1.410 seconds** after building, including every CAF/AAC/FLAC peak comparison. Clippy with the interop feature, package formatting and whitespace checks pass. The release app build took **12.15 seconds** and its bundle signature verifies. Logs: `/tmp/localwave-rust-waveform-final-exchange.log`, `/tmp/localwave-rust-waveform-final-clippy.log`, `/tmp/localwave-rust-waveform-bundle-build.log`. The full Swift shared suite and strict UI timing gate were not rerun during this decoder checkpoint; their prior failures remain unresolved, and iOS device/native-memory qualification remains outstanding.

The final signed-bundle verifier also passed minimum OS, dependency/sandbox checks and the real SDL/Metal launch: one draw followed by **504.7 ms** idle. The window remains the scaffold; this does not qualify waveform UI or sandboxed playback. Log: `/tmp/localwave-rust-waveform-bundle-check.log`.

## Native queue and transport checkpoint

`lw_apple_sys::player` prepares playable assets on a worker, rejecting cancellation, non-regular files and unconfirmed provider content before reading audio. Prepared items move once into the main-thread AVQueuePlayer. Distinct occurrence IDs survive repeated file paths; a separate token identifies each physical player item. Current plus two upcoming items form the native queue. Incremental append preserves an already-buffered next item; replacing future items checks the current token/generation before mutation. Status/completion/failure callbacks use a bounded 64-event ring; overflow pauses playback and returns an error. Retired items remain observed until their late completion/failure can be delivered, with an explicit six-wrapper ceiling. Polling snapshots/events passes 10,000 iterations with zero Rust allocations; native AVFoundation memory remains unqualified.

`lw_core::playback::Transport` owns intent, preparation generations, transport state, measured progress, completion/replay state and interruption resume guards. Pure tests cover stale preparation/activation/completion after pause, stop or replacement; manual pause/stop/output loss during interruption; session failure; final metadata retention/replay; invalid measurements and 10,000 allocation-free updates. This is the transport portion of the full player state machine. Application queue traversal after bad files, download-ahead, library/bookmark materialization, persistence, native session events, Now Playing and media keys remain to be integrated.

`localwave-playback` plays local file arguments through the native engine with one bounded preparation worker. It initially installs three items, then refills without replacing the buffered next item. A bad file stops this diagnostic host with an error; this does not implement the application's required skip-and-notice policy. `--mute` preserves the real audio clock at zero volume. `--smoke-test` checks initial prequeue, manual advancement, refill, stale-generation rejection, future-item removal, pause/volume/invalid-seek/stop and zero-allocation polling without waiting on playback. The GUI remains the empty window scaffold.

The bounded native integration check launches that real main-thread executable and passes. A separate manual audio-clock check played **five 250 ms silent WAV occurrences**, reported each token in order, and reached natural completion in **1.688 seconds** wall, with the last track's 0.250-second elapsed value retained. This verifies live queue operation, not audible gap measurement, background/device routes or iOS session handling. Logs: `/tmp/localwave-rust-player-headless-tests.log`, `/tmp/localwave-rust-player-clock-check.log`, `/tmp/localwave-rust-player-final-tests.log`.

The integration target exposed a pxui test-runner bug: normal binaries built for integration tests have `target.test=true` but `profile.test=false`. The runner previously attempted to invoke the normal player with `--list`. It now excludes those non-test artifacts; five parser regressions pass in 2.159 seconds, and the dependency gate passes. This is an additional local pxui patch alongside the three Metal files; it has not been published/pinned, so clean-clone/remote CI reproducibility remains unresolved. Log: `/tmp/localwave-rust-player-runner-tests.log`.

The later mixed FLAC/AAC/FLAC native queue also reached natural completion, reporting all three tokens in **1.849 seconds** wall and retaining the last measured 0.163-second track duration. This was muted and did not measure audible gaps. The latest 52-case bounded Rust run took **4.260 seconds** wall; its slowest case was 0.277 seconds. App-package Clippy with interop enabled, formatting and whitespace checks pass. Logs: `/tmp/localwave-rust-player-mixed-clock-check.log`, `/tmp/localwave-rust-player-final-clippy.log`. The full Swift shared suite, strict frame-timing gate and iOS/device qualification remain unresolved.

The release GUI build took **12.62 seconds**; the signed bundle passed minimum OS, dependency/sandbox checks and one draw followed by **500.7 ms** idle. Building the separate release headless executable took **5.22 seconds**, and its native smoke check passed with `[1, 2, 3]` initially queued and zero Rust polling allocations. The Swift/Rust exchange passed four cases in **1.135 seconds**. Logs: `/tmp/localwave-rust-player-bundle-build.log`, `/tmp/localwave-rust-player-bundle-check.log`, `/tmp/localwave-rust-player-release-build.log`, `/tmp/localwave-rust-player-release-smoke.log`, `/tmp/localwave-rust-player-exchange.log`. GUI bundle builds now select only the GUI binary, avoiding an unnecessary diagnostic CLI rebuild.

The runner correction is committed locally in pxui as `c36a1589e5c5dc776ffe30c78b620fa66edd7ae5`; the three Metal compatibility files remain uncommitted. No patch was pushed and the parent index still pins baseline `59d6f4540bc9eb7bffe981af0578cd286920f55c`. The remote-pointer check passes for that indexed baseline only; it does not prove this working tree is reproducible from a clean clone. Publishing the patches and updating the parent pointer remain required before remote CI can exercise the new integration target correctly.

## Library preparation and queue-policy checkpoint

`lw_core::player::Session` now combines the occurrence queue and transport with unique preparation tickets. It retains unchanged upcoming items, rejects stale preparation after pause/stop/queue edits or session failure, supports repeat/shuffle/reorder/remove/Clear/Undo, and counts the start of each physical item once for later history integration. Bad current files traverse each occurrence at most once, including duplicate song IDs and repeat-all/one queues. Failed upcoming items leave current playback alone; terminal bad items retain an error, while later good items continue. An already-running preparation can become the next current item without restarting its IO. Session/activation errors do not advance the song. Queue/model mutations belong off the UI frame; the ten-thousand advance/refill regression asserts zero Rust allocations for routine playback transitions.

`LibraryWorker::prepare_playback` uses a readable private copy or creates a content-addressed coordinated copy before native preparation. Relative source access and legacy child bookmarks are both supported; a legacy bookmark can recover when the source-relative file has disappeared. A stale/unconfirmed cached path can fall back to a valid local source without opening the provider file. Cache saves use the existing state/path comparison, preserving Favorites, playlist membership and current metadata. Bookmark renewal now compares expected bytes before updating, preserving a concurrent reconnect. The same materialization code serves the offline copy queue.

Each prepared playback token holds a cache lease. Active, queued and pending copies use the existing native protection keys, so eviction retains shared files until every duplicate occurrence retires. The active token cannot be released before adoption of another item or a native stop acknowledgement. Six leases bound current/upcoming plus one replacement batch; additional preparation applies backpressure. The caller must release rejected preparations and retired items, and clear the final leases only after stopping native playback. The later controller checkpoint below adds application command/lease enforcement; native session lifecycle delivery remains unfinished.

An isolated main-thread integration fixture now creates a real SQLite library and folder bookmark, skips a missing occurrence, materializes one file for three duplicate occurrences, fills the native queue, advances to the second occurrence and tests zero-quota eviction before/after retirement. The file remains protected while any duplicate is queued and becomes evictable after native stop and final release. The original paused check has now been extended to actual muted starts, pause/resume and persistent history updates on the library worker; it still does not qualify the complete GUI controller. Native KVO now uses the actual change payload, preserving transient item identities rather than reading a newer `currentItem` value from a delayed callback.

The latest bounded Rust run passes **60 cases in 4.991 seconds**, with the slowest case **0.203 seconds**. Six new queue-policy regressions cover failure bounds, final bad prequeues, in-flight promotion, stale edits, session/interruption guards and allocation-free ten-thousand-track advancement/refill. The real library preparation check additionally covers copy/bookmark paths, private Added Files, Favorites/membership, duplicate leases, corrupt input/cancellation cleanup, provider simulation, valid-source fallback and lease backpressure. Swift/Rust interoperability passes four cases in **1.056 seconds**. A fresh muted five-clip audio-clock run completed in **0.884 seconds** wall, preserving all five reported tokens and the last 0.100-second duration. Logs: `/tmp/localwave-rust-session-final-tests.log`, `/tmp/localwave-rust-session-exchange.log`, `/tmp/localwave-rust-session-clock-check.log`, `/tmp/localwave-rust-library-native-tests.log`.

The session checkpoint rebuilt the release GUI app in **13.25 seconds**. Its signed bundle passed minimum OS, signature/dependency/sandbox checks and one draw followed by **507.7 ms** idle. The separate release headless build took **7.74 seconds**, and its real library/native integration check passed. App-package Clippy with interop enabled, formatting and whitespace checks pass. Logs: `/tmp/localwave-rust-session-bundle-build.log`, `/tmp/localwave-rust-session-bundle-check.log`, `/tmp/localwave-rust-session-release-build.log`, `/tmp/localwave-rust-session-release-library.log`, `/tmp/localwave-rust-session-final-clippy.log`.

The full Swift shared recheck completed in **27.975 seconds** (warm build 1.41 seconds), within its 30-second budget, but still failed the two unchanged OS notification cases: `testNonAudioMetadataChangeReachesSourceWatcher` and `testCoordinatedChildWriteNotifiesAndInvalidationStopsDelivery`. The playback preload case passed in this run. The shared gate is still failing; the strict UI latency result, device/native-memory gates and pxui publication also remain unresolved. Log: `/tmp/localwave-rust-session-swift-shared.log`.

## Actual playback history checkpoint

The native queue now reports one `Started` event per physical item after it is ready and the engine is actually playing. Preparation, prequeueing and pause/resume do not count additional starts. The diagnostic host verifies that start events match every occurrence reaching natural completion; a real three-item, 80 ms WAV integration check covers short transitions. Its approximately one-second case includes AVQueuePlayer process/audio-clock startup, so it is intentionally an integration check rather than an injected-clock unit test.

`LibraryWorker::record_playback_start` deduplicates by leased physical token, ignores retired tokens and preserves separate counts for duplicate queue occurrences and repeat-one loads. It updates the shipping `lwPlayCounts` and `lwLastPlayed` dictionaries without replacing other songs. Immutable worker-owned history snapshots support repeated catalog lookups without copying defaults for each song; 10,000 lookups assert zero Rust allocations. These native lookups are worker IO, not a frame API, and Foundation allocations are outside that Rust count. The later controller checkpoint below connects start-event delivery to history; library-list refresh remains unfinished.

Managed `Documents/Music` recency updates are best effort after counting, so an IO error cannot prevent playback or cause a duplicate count on retry. Directory-relative, no-follow file descriptors restrict touches to regular immediate cache children and preserve access time. Added Files, external originals, symlink leaves/directories and subdirectories are not touched. Cache leases remain held while these changes run; waveform content signatures continue to use the existing content-addressed cache names.

Swift seeds real native defaults with 64-bit song keys and fractional dates, Rust reads them and records new starts, and Swift verifies counts/dates and untouched entries. The full explicit Swift/Rust exchange now passes five cases in **1.431 seconds**. The library/native integration records two actual duplicate starts, verifies pause/resume and duplicate acknowledgements do not count again, and retains the shared file through eviction until retirement.

The bounded app Rust suite passes **63 checks in 8.956 seconds**; app-package Clippy with interop enabled, formatting and whitespace checks pass. The release GUI build took **14.71 seconds**, its signed bundle passed dependency/sandbox/minimum-OS checks and one draw followed by **502.1 ms** idle, and the diagnostic release build took **6.05 seconds**. Logs: `/tmp/localwave-rust-history-final-tests.log`, `/tmp/localwave-rust-history-exchange.log`, `/tmp/localwave-rust-history-final-clippy.log`, `/tmp/localwave-rust-history-bundle-build.log`, `/tmp/localwave-rust-history-bundle-check.log`, `/tmp/localwave-rust-history-release-build.log`.

The complete application command/event controller, typed playback context, source observation, system media controls and P3–P7 remain unfinished. The two failing Swift source-watcher checks and earlier strict UI wall-time failure remain unresolved; neither was rerun or reclassified by this checkpoint. Device/cloud/native-memory and clean-clone pxui publication gates remain unqualified.

The final history recheck passes **63 Rust checks in 7.082 seconds** (slowest 0.814 seconds) and **five Swift/Rust exchange cases in 1.199 seconds**, including saturated/negative legacy count handling and preserved cache access time. The release library/native check passes with `actual_starts=2 history_plays=2` in **1.301 seconds**. Logs: `/tmp/localwave-rust-history-verified-tests.log`, `/tmp/localwave-rust-history-exchange-final.log`, `/tmp/localwave-rust-history-release-library.log`.

## Application playback bridge checkpoint

`lw_bin::controller::Controller` is now used by the GUI and the native integration host. A single library worker owns SQLite, bookmarks, materialization, history and `Session` queue mutations. The main thread owns AVQueuePlayer and acknowledges every native operation. Its mailbox holds 32 ordinary commands plus a reserved Pause/Stop slot, 128 critical events, one native operation and one acknowledgement; snapshots coalesce instead of growing a message stream. Immutable queue payloads and worker-created title/error strings cross as retained `Arc` values. The UI performs no database/file work or queue sorting.

New commands cancel preparation and invalidate pending native operations before they can play. Pause takes effect on the native engine immediately; installation/rejection acknowledgements determine when a preparation lease can retire. Critical start events are delivered before newer snapshots release their files. A worker failure requests a native stop and waits for its acknowledgement before clearing cache protection. Shutdown likewise stops native playback before joining the worker. Empty/paused players use host/native wakes, with a 50 ms progress sample only while playback is wanted.

The controller restores duplicate queue occurrences and the selected index without autoplay. Queue edits, repeat/shuffle, Clear/Undo, seeking and volume flow through the worker. Index and volume changes write only their existing preference keys, preserving the queue/context bytes and avoiding a full queue rewrite on each natural advance. Native prefix trimming preserves already buffered upcoming items when possible. A pure regression covers a native item becoming current during a reordered queue, and a late completion refines the final measured duration after the empty-queue observation. When download-ahead is disabled, only confirmed local input is prepared in advance; unconfirmed provider files defer until selected.

The GUI has minimal transport controls and cached title/notice content. This is an integration shell; the reference sidebar/table/grids/Now/queue/folders/settings, presenters, shortcuts, drag/drop and goldens remain P4 work. A stronger warm UI check initially exposed **99–100 allocations over 100 state changes**. Dynamic text now uses stable keyed nodes and retained text storage; warm light/dark label and notice changes assert zero Rust allocations. Public accessibility input verifies all five transport intents, without claiming a native VoiceOver bridge. Logs retaining the failures: `/tmp/localwave-rust-controller-edge-tests.log`, `/tmp/localwave-rust-controller-edge-fixed-tests.log`; passing UI checks: `/tmp/localwave-rust-controller-ui-zero.log`, `/tmp/localwave-rust-controller-ui-actions.log`.

The real main-thread controller fixture creates an isolated bookmarked library, restores a paused duplicate queue, skips missing audio with a notice, materializes and prequeues, reorders without replacing the current item, advances a buffered occurrence, tests Clear/Undo/repeat/paused seek, and checks queue/index/muted-volume/context persistence and actual-start history. It checks 1,000 warm controller polls with zero Rust allocations, 32-command backpressure while initialization is held by a channel, rejection of a newer stop, failed initialization, and a cache file disappearing while native items still hold it. Failure shutdown stops native playback before protection clears and retains the original source file. The release fixture passes in **1.045 seconds**, counting three actual starts across the normal and injected-failure scenarios. Log: `/tmp/localwave-rust-controller-release-smoke.log`.

Swift/Rust interoperability passes five cases in **1.158 seconds**, including independent index/volume preference updates preserving duplicate IDs and context bytes. App-package Clippy with interop enabled, formatting and whitespace checks pass. The first release GUI build took **13.98 seconds** and the final host rebuild **8.51 seconds**; the diagnostic release build took **7.68 seconds**. The signed bundle passes minimum OS, dependency/signature/sandbox checks, with **one draw and 510.0 ms idle**. Smoke/performance runs use isolated databases and preference suites under the native temporary directory, so qualification does not open the user's real library. Logs: `/tmp/localwave-rust-controller-exchange.log`, `/tmp/localwave-rust-controller-verified-clippy.log`, `/tmp/localwave-rust-controller-verified-bundle-build.log`, `/tmp/localwave-rust-controller-bundle-check.log`, `/tmp/localwave-rust-controller-release-build.log`.

The strict signed-app timing check still **fails**: 120 draws, **zero warm Rust allocations**, wall build **42 µs median / 212 µs p95 / 696 µs p99 / 1,138 µs maximum**; thread CPU max **66 µs**. The elapsed/CPU gap is not traced, and CPU time does not replace the wall gate. This is the minimal transport shell with synthetic pointer/scroll input, not the required 10,000-song table/scrub/menu workload. Reading the real SDL path also confirms that `poll_coalesced`/`coalesce` still create temporary vectors for actual OS input; those buffers must be retained before claiming a zero-allocation real-input loop. Log: `/tmp/localwave-rust-controller-performance.log`.

The full Swift shared run reached its **30-second wall cutoff** and exited 124; it is still a failed gate. The previously failing source-watcher cases remain unresolved, and this timeout is not a completed suite result. No iOS/device/cloud/native-memory qualification was performed. Log: `/tmp/localwave-rust-controller-swift-shared.log`.

The SDL wake patch and real-window regression are committed locally in pxui as `f624e59a48213a33f8bbcef501789f6cfd03f780`; its focused `cargo t -p pxui_sdl window_visibility` passes in **5.439 seconds** and the dependency gate passes. The test-runner commit and three uncommitted Metal compatibility changes are still local too. The parent index still points to published baseline `59d6f4540bc9eb7bffe981af0578cd286920f55c`; publishing/pinning the needed patches remains required for clean-clone/CI reproducibility. Logs: `/tmp/localwave-rust-controller-sdl-tests.log`, `/tmp/localwave-rust-controller-deps.log`.

Rapid-command qualification additionally caught and fixed two Next requests collapsing into one buffered advance. The controller now commits the pending logical selection before later Next/Previous/Pause/Stop commands, and a stale native advance falls back to that selection without autoplay after pause. A repeat-one restart followed immediately by a volume command is covered too. Pause/Stop can use a reserved mailbox slot even when 32 ordinary commands are queued; Stop retains priority over a later Pause. The integration check covers these cases and counts four actual starts across its normal/repeat/failure scenarios. The failing rapid-click case remains in `/tmp/localwave-rust-controller-rapid-next-before.log`; focused corrected checks are in `/tmp/localwave-rust-controller-rapid-next-after.log` and `/tmp/localwave-rust-controller-ordered-repeat-tests.log`.

Known bridge limits are recorded in code: every command currently fences preparation, so continuous volume/seek controls need separate handling when those UI controls are connected; current-title metadata also needs invalidation when catalog edit commands are added. These do not remove the corresponding full-app requirements.

The final rapid-command/priority changes pass **66 bounded Rust checks in 3.589 seconds** warm (5.218 seconds summed case time; slowest 0.684 seconds), and Clippy with interop enabled passes. Logs: `/tmp/localwave-rust-controller-complete-tests.log`, `/tmp/localwave-rust-controller-complete-clippy.log`.

The real audio-clock fixture also caught AVQueuePlayer remaining at rate 1 with an empty queue after natural completion. The controller now explicitly pauses that empty engine, retains the final duration/elapsed value and records the final actual start. Its full normal/repeat/failure/completion flow now counts five starts. Failure evidence is retained in `/tmp/localwave-rust-controller-natural-before.log`; the focused fix passes in `/tmp/localwave-rust-controller-natural-after.log` (1.781 seconds for the real playback integration case). This deliberately uses the audio clock and remains bounded by the existing 5-second case / 4-second fixture deadlines.

The final lifecycle checkpoint passes **66 cases in 13.616 seconds** after rebuilding; the longest real audio-clock integration case is **1.686 seconds**. Clippy with interop enabled, formatting and whitespace checks pass. The final GUI release build took **9.44 seconds**, the signed bundle again passes with one draw and **502.6 ms** idle, and the diagnostic release build took **7.95 seconds**. The completed release controller fixture passes with `history_plays=5 natural_end_paused=true rapid_next_ordered=true stop_priority=true`. Logs: `/tmp/localwave-rust-controller-lifecycle-tests.log`, `/tmp/localwave-rust-controller-lifecycle-clippy.log`, `/tmp/localwave-rust-controller-lifecycle-bundle.log`, `/tmp/localwave-rust-controller-lifecycle-bundle-check.log`, `/tmp/localwave-rust-controller-lifecycle-release-build.log`, `/tmp/localwave-rust-controller-lifecycle-release-smoke.log`. The earlier timing failure and Swift shared timeout are still failed gates.

### Real SDL input allocation checkpoint

The real SDL numeric-event regression initially measured **700 Rust allocations over 100 warm pumps**. `poll_coalesced_into` now streams into the window's retained event buffer, coalescing motion and both resize kinds with fixed local slots. Modified-key flags use the existing normalized bitmask instead of a vector. Stable-event boundaries and resize-kind ordering remain covered. The allocating convenience `poll_coalesced` remains available. Failure evidence: `/tmp/localwave-rust-input-before.log`.

The wider app regression then caught **200 allocations over 100 frames** in UI key routing, even after the SDL pump itself passed. The UI now retains its key-state buffer and applies each event in place. The harness-free `window_input` test runs on the initial thread and checks SDL pumping, app wire encoding and UI construction separately. Its real queued pointer, button, modified-key, resize, wheel and pinch events pass with **zero warm Rust allocations across app and libpxui**, including changing play/notice state. This does not draw the GPU, exercise the unfinished 10,000-song screens, cover owned text/IME/drop/dialog payloads, or measure native allocations. Failure and fix logs: `/tmp/localwave-rust-input-app-tests-fixed.log`, `/tmp/localwave-rust-input-app-stages.log`, `/tmp/localwave-rust-input-app-after.log`.

The complete app check passes **67 cases in 5.094 seconds wall**, with 6.355 seconds summed case time and the real audio-clock controller case at 1.788 seconds. The SDL/input checks pass **31 cases in 7.750 seconds wall**, including retained-buffer domain validation, coalescing boundaries and the real native pump. Logs: `/tmp/localwave-rust-input-complete-tests.log`, `/tmp/localwave-rust-input-upstream-complete.log`. The previous strict frame-time failure and Swift shared timeout remain failed gates.

The focused core keyboard regression passes (33.878 seconds with an initial dependency build; 4.078 seconds on the following run). Clippy across app targets with interop enabled passes after scoping the shared harness's registry-signature lint allowance to the test module. The pxui dependency gate and whitespace checks pass. The input/routing fixes are committed locally as `49e50fb`; the three preexisting Metal changes remain uncommitted, and the parent index still pins the published baseline. Clean-clone publication remains unfinished. Logs: `/tmp/localwave-rust-input-core-keyboard.log`, `/tmp/localwave-rust-input-core-keyboard-warm.log`, `/tmp/localwave-rust-input-clippy-fixed.log`, `/tmp/localwave-rust-input-deps.log`.

The release library and app rebuilds took **16.38 seconds** and **6.47 seconds**. The signed bundle passes its minimum-OS/dependency/signature/sandbox checks and draws once before **506.9 ms idle**. One post-change performance run passes: **120 draws, zero warm Rust allocations, 383 µs maximum UI wall build** (24 µs median, 83 µs p95, 146 µs p99; CPU maximum 60 µs). This remains the minimal synthetic pointer/scroll shell; it does not replace real input/draw or full 10,000-song qualification, resolve the earlier elapsed-time spikes, or fix the Swift shared timeout. Logs: `/tmp/localwave-rust-input-bundle.log`, `/tmp/localwave-rust-input-bundle-check.log`, `/tmp/localwave-rust-input-performance.log`.

### Song FTS update checkpoint

The 10,000-song worker benchmark now includes changing every title through the production `upsert_song` transaction. Before the fix this took **10,358.78 ms**, because each update scanned the whole UNINDEXED `songId` column. The same development-profile workload now takes **1,318.54 ms** (about **7.9× faster**), with exact search counts and playlist membership checked. These are single in-memory worker runs, not disk/import or UI latency guarantees. Logs: `/tmp/localwave-rust-fts-before.log`, `/tmp/localwave-rust-fts-after.log`.

`LibraryRepository` loads the existing FTS row IDs once, preserves arbitrary legacy row IDs and duplicate entries, and addresses updates by rowid. It repairs a missing search entry when saving metadata. The map remains on the worker, with bucket capacity bounded by the largest FTS table seen by that repository. Every transaction checks [SQLite's connection-local `data_version`](https://www.sqlite.org/pragma.html#pragma_data_version) for other writers; deletions and failed transactions invalidate it. No database schema or Swift identity changes are required. The focused real two-connection regression covers row-ID replacement/reuse, duplicates, missing rows and album deletion. Existing import failure tests cover rollback of metadata, digest and search together.

At this FTS checkpoint, all **68 Rust checks pass in 18.869 seconds wall** after rebuilding (6.251 seconds summed case time), and the actual Swift/Rust exchange passes **five cases in 0.932 seconds**. Clippy with interop enabled, app formatting and whitespace checks pass. The app formatter now skips its included upstream window harness, preventing unrelated submodule formatting changes. The earlier full Swift shared timeout and full UI/device/native-memory qualification remain unresolved. Logs: `/tmp/localwave-rust-fts-complete-tests.log`, `/tmp/localwave-rust-fts-focused.log`, `/tmp/localwave-rust-fts-exchange.log`, `/tmp/localwave-rust-fts-clippy.log`.

### Typed playback context checkpoint

The pure core model now covers Library, album, artist, playlist, folder with optional path ID, automatic and selection contexts. The native Foundation boundary reads and writes Swift's synthesized associated-enum JSON shape with length-delimited UTF-8 and signed 64-bit IDs. Queue configuration carries a retained typed context; the worker serializes it before changing the queue, restores it into snapshots, and the shell displays its borrowed title. Index/volume updates preserve the original persisted bytes. Unknown or invalid saved contexts display Library without rewriting those bytes. The application worker checks every model/codec conversion, including missing folder IDs and unknown future cases.

The actual Swift `JSONEncoder` → Rust typed fields → JSON → Swift `JSONDecoder` exchange covers all seven cases, embedded NUL, quoting, Unicode, optional path IDs and Int64 extremes. Adversarial numeric spellings initially caught a mismatch at `9223372036854775807.0`; the boundary guard now rejects it as the current Swift decoder does. The initial failure is retained. All **six exchange cases pass in 0.888 seconds**. Logs: `/tmp/localwave-rust-context-exchange.log`, `/tmp/localwave-rust-context-exchange-fixed.log`.

All **69 bounded Rust checks pass in 6.357 seconds wall** warm (6.055 seconds summed case time), including real controller restoration/persistence and zero warm allocations while the context/transport/notice labels alternate in both themes and through real numeric SDL input. Clippy with interop enabled, formatting and whitespace checks pass. The release app rebuild takes **12.53 seconds**, the signed bundle passes with one draw and **503.6 ms idle**, and the rebuilt release controller fixture passes with five actual starts, natural completion paused and zero warm poll allocations. Logs: `/tmp/localwave-rust-context-ui-complete-tests.log`, `/tmp/localwave-rust-context-clippy.log`, `/tmp/localwave-rust-context-bundle.log`, `/tmp/localwave-rust-context-bundle-check.log`, `/tmp/localwave-rust-context-release-build.log`, `/tmp/localwave-rust-context-release-smoke.log`.

The latest strict signed-app frame gate **fails**: 120 draws, zero warm Rust allocations, wall build **40 µs median / 249 µs p95 / 1,137 µs p99 / 1,649 µs maximum**, thread CPU maximum **62 µs**. The elapsed-time spikes remain untraced; CPU time does not replace the wall threshold, and the workload remains the minimal synthetic shell. The full Swift shared suite now completes in **17.789 seconds wall** (53 scheduled cases), but the same two source-watcher notification cases fail; the six explicit interop cases are skipped in the ordinary suite and pass separately above. No iOS/device/native-memory qualification is claimed. Logs: `/tmp/localwave-rust-context-performance.log`, `/tmp/localwave-rust-context-swift-shared.log`.


### Reference shell and 10,000-song workload checkpoint

The placeholder window has been replaced by the wide reference layout: 231-point sidebar, flexible library, 319-point Now panel and 71-point transport. SQLite and label/group preparation remain on the serialized worker. Song labels occupy one string arena; album/artist/playlist orders retain song IDs and typed contexts. Album/artist navigation, collections, selection, double-click/Return playback, play/pause/previous/next/stop, repeat/shuffle, panel visibility and continuous seek/volume are connected. The Now panel shows current metadata and a matrix clock. Artwork remains a placeholder and the scrubber is a progress line; neither is presented as completed artwork/waveform delivery. Search, sort, editing/import, sources, full queue/settings and other reference interactions remain open. Catalog refresh is deliberately unwired until worker-side replacement/retirement is implemented.

Clock changes initially caused **4,611 allocations per 100 frames** because whole time strings were shaped each tick. Numeric clocks now reuse individual font glyphs without frame-side heap formatting. The 10,000-song wheel trace then caught **69 allocations over 80 warm frames**: recycled row slots discarded accessibility strings. pxui now retains those string capacities, with its existing scrolling regression extended to cover both names and values. A remaining single growth of the hit-record vector was traced to the fixture sending its first wheel event before geometry existed; the fixture now settles geometry before replaying its identical warm and measured cycles. Assertions remain zero. The pxui fix is committed locally as `46f8112`; the three pre-existing Metal files remain uncommitted, no patches were pushed and the parent index still pins the published baseline. Logs retain the failures and stack trace: `/tmp/localwave-rust-shell-ui-tests.log`, `/tmp/localwave-rust-shell-virtual-tests.log`, `/tmp/localwave-rust-shell-allocation-trace.log`.

Seek/volume envelopes now coalesce within the existing 32-command bound and retain a separate preparation identity. They cannot invalidate an already prepared native item; queue/transport changes still cancel preparation and reject stale native operations. Repeated unchanged volume values avoid defaults writes. The actual controller fixture continuously sends seek/volume during current/upcoming preparation and still checks failure skipping, rapid Next order, Stop priority, persistence, five physical starts and native-retirement leases. Current artist/album strings are prepared alongside the title. Catalog editing must invalidate this metadata cache when connected.

All **73 bounded Rust checks pass in 17.224 seconds wall** after rebuilding (7.888 seconds summed case time). The runner retried the native metadata and resource-scan cases alone after contention; both passed, and their initial results remain in the log. Five focused UI checks pass in **2.127 seconds**, including public accessibility seek/volume/mode/panel actions and the 10,000-song virtual-row/clock allocation check. The pxui scrolling regression and dependency gate pass; Clippy with warnings denied, formatting and whitespace checks pass. Logs: `/tmp/localwave-rust-shell-qos-complete-tests.log`, `/tmp/localwave-rust-shell-controls-tests.log`, `/tmp/localwave-rust-shell-pxui-test.log`, `/tmp/localwave-rust-shell-pxui-deps.log`, `/tmp/localwave-rust-shell-qualified-clippy.log`.

The release bundle builds and signs, and the new window was captured and inspected at `/tmp/localwave-rust-shell-preview.png`. Its isolated native smoke reports **one frame, one draw and 500.5 ms idle**. The actual normal app was already running as PID 69331 when opening was reported to fail; its LocalWave window was brought forward. It was left running, so quit/reopen it to use the rebuilt executable. A startup-only worker snapshot, screenshot and smoke do not qualify the full reference UI or a signed upgrade.

The signed performance workload now seeds an isolated 10,000-song SQLite database on the worker, scrolls the actual table and changes clocks. Two initial geometry frames and two 40-frame wheel cycles precede 100 measured frames. **All recorded runs fail the strict 1 ms wall maximum**, despite zero Rust allocations:

| Version | Median | p95 | p99 | Maximum | Thread CPU maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Initial 10k shell | 328 µs | 1,035 µs | 1,669 µs | 2,885 µs | 379 µs |
| Interactive UI QoS, first run | 281 µs | 465 µs | 806 µs | 1,925 µs | 374 µs |
| Final bundle, same QoS with sidebar border removed | 358 µs | 1,681 µs | 2,773 µs | 2,806 µs | 408 µs |

A real System Trace was recorded before changing QoS: `/tmp/localwave-rust-shell-system.trace` (208 MiB); its exported context switches are `/tmp/localwave-rust-shell-context-switches.xml`. It records 444.690 ms Running and 46.237 ms Runnable at default priority, compared with 9.015 ms Running at interactive priority. The host now requests interactive QoS for the main thread after spawning the library worker, and a regression rejects attempts from worker threads. The first QoS run had lower percentiles, but the final run did not sustain that improvement. These runs do not establish the cause of every build spike; frame-specific signpost correlation and native allocation profiling remain. Do not substitute CPU time or percentiles for the maximum gate. Logs: `/tmp/localwave-rust-shell-performance.log`, `/tmp/localwave-rust-shell-qos-performance.log`, `/tmp/localwave-rust-shell-final-performance.log`, `/tmp/localwave-rust-shell-system-trace.log`.

Warm visited-row scrolling is covered; first-time text shaping, all 10,000 rows, scrub/menu traces, native Metal/OS memory, macOS 15, iOS and device/cloud behavior remain unqualified. The previous full Swift run still has two source-watcher notification failures; it was not rerun for these Rust-only UI changes.

Final bundle validation after the visual adjustment: release application rebuild **7.20 seconds**, strict codesign/dependency/minimum-OS checks pass, and native smoke reports one frame/one draw with **502.0 ms idle**. No timing retry is discarded. Logs: `/tmp/localwave-rust-shell-final-bundle.log`, `/tmp/localwave-rust-shell-final-bundle-check.log`.


### Scroll storage, navigation and frame correlation checkpoint

Retained draw runs previously appended new rows during one-direction scrolling without reclaiming old holes. pxui now compacts within a budget proportional to live runs, with a small minimum budget, and reserves its existing compaction scratch before reuse. The extended nested-row regression travels for 600 frames; all 570 measured frames allocate zero Rust objects and retained instance storage stays bounded. Recycled empty slots also donate existing text, accessibility-name and value buffers when a target needs capacity; nonempty content is never taken. A separate regression checks donor content and accessibility values. These changes are committed locally as `cf48169`.

A per-row retained-view experiment was removed after comparison: with tree assembly and phase timing enabled, its median UI CPU/paint times were 326/164 µs, versus 296/121 µs for flat virtual rows. Pane-level caching remains. These are single comparable instrumented runs, not a stable latency claim. Logs: `/tmp/localwave-rust-retained-phases.log`, `/tmp/localwave-rust-flat-rows-phases.log`.

Route changes now create separate library scroll scopes, and Return plays the selected song even while the library view is retained. A new regression checks row displacement, selection, Return, panel resizing and Favorites navigation. It exposed raw integer sidebar keys 0/1 colliding with slot-table sentinels and corrupting accessibility parents. The app uses generated keys; pxui now rejects the reserved values at both ABI entries before node/grid materialization, and lookup treats them as absent. The C header and wrapper document the constraint; the shared byte-boundary regression checks both entries. This guard is committed locally as `b08f1de`. Sidebar collections now use the existing fixed-height grid instead of building every playlist. Its 10,000-collection regression checks bounded visible nodes, actual scrolling and zero allocations over 80 warm measured frames.

The earlier `/tmp/localwave-rust-frame-final-tests.log` scheduled 75 cases but contains the navigation failure; its total is not a passing-suite result. After the fixes, the authoritative app run passes **76 cases in 15.670 seconds wall**, with **6.796 seconds summed case time**. Four native IO checks were retried alone after contention and passed; their initial results remain in the log. All **57 focused pxui window checks pass in 27.890 seconds wall** including rebuilding, with **2.176 seconds summed case time**. The focused ABI byte-boundary check passes. The pxui dependency gate, app Clippy with interop enabled and warnings denied, app formatting and whitespace checks pass. Logs: `/tmp/localwave-rust-scroll-storage-complete-tests.log`, `/tmp/localwave-rust-reserved-keys-core-fixed.log`, `/tmp/localwave-rust-node-key-abi.log`, `/tmp/localwave-rust-reserved-keys-deps.log`, `/tmp/localwave-rust-scroll-storage-clippy-fixed.log`.

The host no longer assembles an unconsumed accessibility tree in production, because native bridges are explicitly deferred by the supplied plan. Views still author roles, names and actions; UI and SDL-input regressions keep tree assembly enabled. Earlier timing comparisons with tree assembly enabled remain above; the current production workload omits that work. This does not provide native VoiceOver support.

`--trace-frames` adds optional native signpost intervals with frame ID, measured wall/CPU time and requested QoS, plus seven UI phase medians. Normal frames do not invoke those intervals. A counting-allocator check covers warm interval scopes. A real System Trace records 182 paired intervals in `/tmp/localwave-rust-frame-metrics.trace`; frame 137 reports wall/CPU **1,785/354 µs**, and frame 138 **1,253/312 µs**. The initial context-switch adjacency analysis classified most of both spans as Running. The later direct thread-state duration export below corrects that classification and identifies preemption. A CPU/wall gap alone does not establish a cause; the corrected durations are required evidence. Export and analysis: `/tmp/localwave-rust-frame-metrics.xml`, `/tmp/localwave-rust-frame-metrics-analysis.log`. This diagnostic run also fails the strict wall gate. Native allocation profiling remains outstanding.

The latest release app rebuild takes **8.00 seconds** and the signed bundle passes minimum-OS load commands, dependencies, signature, sandbox and one draw followed by **510.0 ms idle**. Its separate performance run draws all **182 frames for 10,000 songs** with **zero warm Rust allocations**, but **fails** the unchanged 1 ms maximum: wall **283 µs median / 1,261 µs p95 / 1,682 µs p99 / 1,992 µs maximum**; thread CPU **272 µs median / 378 µs maximum**. The preceding bundle's failed **2,614 µs** maximum remains recorded in `/tmp/localwave-rust-frame-final-performance.log`; no retry was discarded. Latest logs: `/tmp/localwave-rust-scroll-storage-bundle.log`, `/tmp/localwave-rust-scroll-storage-bundle-check.log`, `/tmp/localwave-rust-scroll-storage-performance.log`.

The normal application process was left running; quit/reopen it to load the rebuilt bundle. Local pxui commits remain unpublished and the parent index still pins the old published baseline, so the current integration is not yet reproducible from a clean remote clone. The full Swift suite was not rerun for these Rust-only changes and retains its two source-watcher failures. Full Mac interactions, P3 fixes, iOS/device/cloud/release checks, first-seen text and process/native memory remain unfinished. No stable sub-millisecond frame guarantee is claimed.


### Sampled motion checkpoint — P3 fix 1

pxui now supports `Curve::Table` and `Curve::Steps`, committed locally as `3d805aa`. A table stores at most 16 positions and values inline, preserving uneven CSS sample times and overshoot. Values alone at uniform intervals cannot reproduce the reference's 12 uneven points to 1e-6. Bezier/table/step runs now finish by elapsed duration; progress crossing 1 during a spring overshoot cannot retire the animation early. Keyframe segments supply their duration to all three finite-duration curve types. The public wrapper's temporary keyframe Vec is replaced by eight stack descriptors, retaining the engine's existing limit.

`lw_ui::motion` carries all reference durations, easing control points, the actual 12-point spring and end steps with the Swift one-microsecond scheduled-boundary tolerance. Its ported `MotionTokenTests` vectors pass, including the 1.043 overshoot, the 0.75 interpolation sample and Date-subtraction boundary cases. A 10,001-point sweep also compares f32 evaluation against the double-precision piecewise reference within **1e-6** with zero Rust allocations. These are motion primitives; button/dialog/presenter integration still needs scale animation and the remaining UI work.

The public wrapper regression exercises rendered positions across the library boundary, overshoot, table/step keyframe duration rescaling, unchanged retained-view build counts, frame idling and zero warm app/library allocations. It passes with both dynamic and static linking. Core reduced-motion coverage now includes both new curves. Descriptor and raw-node tests reject malformed counts, endpoint/order errors, nonfinite active values/durations and unknown tags; the trusted static node entries share parameter validation. Grid declaration errors return before reading an unwritten output handle. The full ABI run also exposed the prior reserved-key guard's panic fixture still using key 1; it now uses a valid key 7 and retains every panic-containment assertion. The initial failure remains in `/tmp/localwave-rust-curve-abi-tests.log`.

Inline tables expand the unreleased C layouts: Curve **140 bytes**, optional Curve **144**, curve descriptor **160**, keyframe descriptor **344**, Node **600**. Existing Node field offsets remain unchanged. Both layout snapshots are intentionally refreshed under the header's existing unreleased-ABI policy; the append-only gate itself is unchanged. C static assertions and compilation, all four Rust layout checks, ABI-header/snapshot and dependency gates pass. The app and dylib must rebuild together. No dependency or build-profile changes were added.

Verification:

- **77 app cases pass** in **38.278 seconds wall** including the first rebuild (6.472 seconds summed case time), and again in **33.461 seconds** including native/shared rebuilding (5.692 seconds summed case time). These are not claimed as fully warm-cache loop measurements. Both retain native contention retries; no case fails. Logs: `/tmp/localwave-rust-curves-app-complete.log`, `/tmp/localwave-rust-curves-app-warm.log`.
- **58 dynamic-wrapper/ABI/type checks pass in 1.207 seconds**, and **28 static-wrapper checks pass in 21.853 seconds** including rebuilding. Logs: `/tmp/localwave-rust-curve-public-abi-complete.log`, `/tmp/localwave-rust-curves-static-complete.log`.
- **57 focused core window checks pass in 5.937 seconds**, and **14 styling/reduced-motion checks pass in 0.359 seconds**. Logs: `/tmp/localwave-rust-curve-core-window.log`, `/tmp/localwave-rust-curve-reduced-motion-styling.log`.
- App Clippy with interop and warnings denied, app formatting, whitespace, C header compilation and pxui gates pass. A cached core check takes **0.16 seconds**; the earlier dependency-rebuilding checks took 8.75/8.88 seconds and are not warm-check evidence. Logs: `/tmp/localwave-rust-curves-clippy.log`, `/tmp/localwave-rust-curve-abi-header.log`, `/tmp/localwave-rust-curve-abi-snapshot.log`, `/tmp/localwave-rust-curves-deps.log`, `/tmp/localwave-rust-curves-native-warm-check.log`.

Release rebuilding takes **18.23 seconds for libpxui** and **6.19 seconds for the app**. The signed bundle passes minimum-OS/dependency/signature/sandbox checks and one draw followed by **502.3 ms idle**. The unchanged 10,000-song gate draws **182 frames**, with **zero warm Rust allocations**, and **fails** the strict maximum: wall **282 µs median / 1,998 µs p95 / 2,296 µs p99 / 2,847 µs maximum**; CPU **268 µs median / 405 µs maximum**. This is not a stable sub-millisecond result, and earlier failures remain recorded. Logs: `/tmp/localwave-rust-curves-bundle.log`, `/tmp/localwave-rust-curves-bundle-check.log`, `/tmp/localwave-rust-curves-performance.log`.

Local patches are not published and the parent index still pins the old remote baseline; clean-clone/CI reproducibility remains unresolved. The already-running normal app was left untouched and needs quit/reopen to use this ABI/bundle. The full Swift/iOS suites and native allocation/device/cloud checks were not rerun for this Rust-only stage; the two known Swift source-watcher failures remain. P3 scale/blur/text/OS-input/export work, full Mac/iOS UI and release/device gates remain in scope.


### Anchored scale checkpoint — P3 fix 2

Committed locally in pxui as `3eb84ae`: `Target::Scale { x, y, anchor }` and its public keyframe equivalent cross the existing four-float descriptor as target kind 3. No C layout or function change is needed. Scaling requires a view with its own painter entry, composes after the manual transform and before enclosing transforms, and fixes a fractional anchor for each segment. Zero collapses contents; negative factors reflect them. Ordinary nodes, flat views and stale handles return false. Resting scale/anchor data lives per view, not per song/node.

Scale-only runs update the existing transform table and preserve instance bytes and retained-view build counts. Core coverage compares painter corners, reported rectangles and accessibility bounds through independent axes, corner anchoring, nested views, zero/reflected scales, inverse hit testing and manual rotation/scale composition. The initial test caught stale accessibility geometry during a retained animation; changed transform tables now invalidate its walk. The dynamic/static public regression covers sampled overshoot, continuous retargeting, stepped keyframes, frame idling, unchanged instance bytes and zero warm allocations. The reduced-motion check asserts the actual ThemeSource flag and samples at the start time, so elapsed time cannot create a false pass. Logs retain the original accessibility failure: `/tmp/localwave-rust-scale-core-test.log`; fixed result: `/tmp/localwave-rust-scale-core-test-fixed.log`.

App icon buttons now keep their original hit/semantic rectangle while a cached visual child scales to **0.96** using the reference spring over **0.08 seconds** on pointer press and returns over **0.24 seconds** on release. Only held-state transitions start animations. The regression releases outside the shrunk visual but inside the original button, verifies the click and unchanged accessible bounds, counts zero warm allocations and verifies immediate reduced-motion rendering. This connects pointer press feedback; full keyboard/presenter interactions and native OS reduce-motion delivery remain unfinished.

Introducing visual child views exposed one allocation in the original warm light/dark shell check. A native stack trace identified compaction move scratch growing by 108 bytes; its source instrumentation was removed after diagnosis. A stronger 12-view regression rebuilds nine existing children before retaining all of them together. It exposed three allocations, then two after fixing only move scratch. Moves now reserve for the known view count; both instance and compaction buffers reserve potential run padding for existing fresh views too. The actual retained-length/compaction budget and all original zero assertions are unchanged. The regression passes with its original 10-frame warmup. Failure and trace logs: `/tmp/localwave-rust-scale-app-ui.log`, `/tmp/localwave-rust-scale-shell-allocation-trace.log`, `/tmp/localwave-rust-scale-more-kept-before.log`, `/tmp/localwave-rust-scale-more-kept-after.log`. Passing buffer result: `/tmp/localwave-rust-scale-more-kept-both-buffers.log`.

Verification of the final stage:

- **78 app checks pass in 41.420 seconds wall**, including rebuilding, with **8.512 seconds summed case time**. Native cases and the sidebar check were retried alone after contention and passed; their initial output remains. The app's nine UI checks also passed separately in 1.752 seconds before the final scale-only instance optimization; the final whole-app run covers that optimization. Logs: `/tmp/localwave-rust-scale-app-complete.log`, `/tmp/localwave-rust-scale-ui-qualified.log`.
- **60 dynamic-wrapper/ABI/type checks pass in 30.752 seconds** including rebuilding (1.665 seconds summed cases), **29 static-wrapper checks in 6.964 seconds**, **58 focused core window checks in 6.971 seconds**, **15 styling checks in 0.383 seconds**, and **four accessibility checks in 0.275 seconds**. Logs: `/tmp/localwave-rust-scale-public-abi.log`, `/tmp/localwave-rust-scale-static.log`, `/tmp/localwave-rust-scale-window-complete.log`, `/tmp/localwave-rust-scale-styling-complete.log`, `/tmp/localwave-rust-scale-core-a11y.log`.
- App Clippy with interop/warnings denied, app formatting, whitespace, C header compilation and ABI-header/snapshot/dependency gates pass. Logs: `/tmp/localwave-rust-scale-clippy.log`, `/tmp/localwave-rust-scale-abi-header.log`, `/tmp/localwave-rust-scale-abi-snapshot.log`, `/tmp/localwave-rust-scale-deps.log`.

Release rebuilding takes **18.70 seconds for libpxui** and **7.74 seconds for the app**. The signed bundle passes minimum-OS/dependency/signature/sandbox checks and one draw followed by **504.6 ms idle**. One new signed 10,000-song scroll/clock run **passes** the unchanged gate: **182 draws**, **zero warm Rust allocations**, wall **290 µs median / 469 µs p95 / 602 µs p99 / 749 µs maximum**; CPU **273 µs median / 395 µs maximum**. This single run does not explain or erase the preceding **2,847 µs** failure or establish stable sub-millisecond latency. The workload does not yet qualify animated-button GPU traces, scrub/menu interactions, first-seen text or native process allocations. Logs: `/tmp/localwave-rust-scale-bundle.log`, `/tmp/localwave-rust-scale-bundle-check.log`, `/tmp/localwave-rust-scale-performance.log`.

Switching app/submodule debug invocations repeatedly rebuilt shared dependencies during this stage; fast build-cache behavior remains unresolved and no warm-loop speed guarantee is made. Local pxui commits remain unpublished, the parent index still pins the remote baseline, and the three older Metal files remain uncommitted. The normal GUI process was left running; quit/reopen to load the rebuilt app. Swift/iOS/device/cloud/release checks were not rerun, and the two known Swift source-watcher failures remain. P3 fixes 3–10, complete Mac/iOS interactions and all remaining platform/release gates remain in scope.


## Shared build-cache checkpoint — 13 November 2026

Switching between the app and pxui had changed Cargo's linker configuration under shared artifact names: repeated checks rebuilt unchanged dependencies in 7.13–9.49 seconds. Both workspaces now resolve the same canonical `tools/cc-stack.sh` linker. Root opts out of its legacy stack setting through `PXUI_CONSUMER_DEFAULT_STACK=1`; pxui-owned tools retain their existing 256 MiB stack, and consumers that do not opt out retain their previous behavior. The public wrapper tracks the environment value so changing it relinks consumers. No profiles, Rust flags, toolchains or target directories were changed, and no clean was run. Local pxui commit: `9c23fed`.

- Root → pxui → root checks take **0.14 / 0.13 / 0.03 seconds**, all shared dependencies fresh. A final root check after the explicit opt-out takes 0.14 seconds. Logs: `/tmp/localwave-cache-root-shared-linker.log`, `/tmp/localwave-cache-child-shared-linker.log`, `/tmp/localwave-cache-root-shared-return.log`, `/tmp/localwave-cache-root-optout-check.log`.
- **25 runner tests pass in 3.300 seconds** (5.814 seconds summed), including mocked compiler argv/exit propagation, legacy consumers, opted-out consumers, owned tools, dylibs, path boundaries, spaces and a filename containing the literal `-dynamiclib`. The initial root `-p rays-t` attempt was unsupported by the runner's package dispatcher; the final suite ran from pxui using the existing shared target directory. Logs: `/tmp/localwave-cache-legacy-runner-tests.log`, `/tmp/localwave-cache-linker-regression.log`.
- Final app → pxui core → app verification passes: **78 app checks**, **58 core/window checks in 2.811 seconds** (0.06-second build), then **78 app checks in 4.618 seconds** (0.04-second build, 5.903 seconds summed cases). Native contention retries are retained and pass; no deadlines or assertions changed. App Clippy with interop and warnings denied passes in 0.99 seconds; the pxui dependency gate passes. Logs: `/tmp/localwave-cache-app-optout.log`, `/tmp/localwave-cache-core-optout-switch.log`, `/tmp/localwave-cache-app-final-warm.log`, `/tmp/localwave-cache-final-clippy.log`, `/tmp/localwave-cache-optout-deps.log`.
- The one-time shared linker change rebuilt the release library/app in 35.18/14.68 seconds; the final build-script change relinked the app in 8.44 seconds. The final **warm bundle builds take 0.14/0.03 seconds** and signing passes. Mach-O inspection verifies the app stack size is **0** and the runner retains **268,435,456 bytes**. The signed verifier passes minimum OS, dependency, entitlement and stack checks; smoke renders one frame/draw and remains idle for **500.7 ms**. Logs: `/tmp/localwave-cache-final-bundle.log`, `/tmp/localwave-cache-final-bundle-warm.log`, `/tmp/localwave-cache-final-bundle-check.log`.
- An earlier signed frame run passes at **545 µs maximum wall build** with zero warm Rust allocations. The final relinked bundle instead **fails at 1,980 µs** (wall p50/p95/p99 271/841/1,243 µs; CPU p50/p95/p99/max 261/306/348/357 µs), still with **zero warm Rust allocations across 182 frames/draws**. This preserves the strict gate and adds failure evidence; scheduling is not established as the cause. Logs: `/tmp/localwave-cache-performance.log`, `/tmp/localwave-cache-final-performance.log`.

Local pxui commits are unpublished and the parent index still pins the old remote baseline; clean-clone/CI reproducibility remains unresolved. The three earlier Metal changes remain uncommitted. The running normal GUI was left untouched. Swift/iOS/device/cloud/native-memory checks were not rerun; the two known Swift source-watcher failures remain. P3 fixes 3–10, complete Mac/iOS interactions, stable latency and platform/release gates remain required.


## Backdrop saturation checkpoint — 13 November 2026

P3 fix 3 is partially implemented: `Ui::view_backdrop_saturation` carries one finite, nonnegative float per retained view through `BlurOut`, the C export/header and the static/dynamic public wrapper. Default 1 preserves existing color, 0 produces grayscale, and values above 1 increase saturation. Metal and GLSL apply it once, after unpremultiplying the vertical blur result, using the existing unused uniform component; tint/opacity still apply when compositing. Alpha and transparent-edge behavior are preserved, with no additional pass or texture. The checked-in SPIR-V was regenerated and validated for Vulkan 1.1. The additive export bumps the ABI minor to **1.1**; no public layouts changed, and the snapshot gate is unchanged. Local pxui commit: `c6867e3`.

- **22 renderer cases pass in 4.460 seconds** (1.372 seconds summed), including existing goldens, translucent blur edges, both downsample modes, saturation 0/0.9/1/1.5 with alpha 128, invalid-value rejection, and a native Metal pixel readback through the public view API at saturation 0.9. Log: `/tmp/localwave-rust-blur-saturation-render-complete.log`.
- **30 static wrapper cases pass in 1.892 seconds** (0.845 summed) and **29 dynamic wrapper cases pass in 3.990 seconds** (0.806 summed). The new saturation case checks invalid-value errors and zero warm Rust allocations for UI state updates/frame construction across the library boundary. The initial test compile used an unavailable size variant/private Look import; the corrected test uses the existing public types. Logs: `/tmp/localwave-rust-blur-saturation-wrapper-static-complete.log`, `/tmp/localwave-rust-blur-saturation-wrapper-dynamic.log`, `/tmp/localwave-rust-blur-saturation-wrapper-static.log`.
- The core retained-blur record regression passes in 4.543 seconds including rebuilding (0.027-second case), covering default/persisted saturation, offset updates and rejected nonfinite/negative values. **78 app cases pass in 13.090 seconds** including rebuilding, while the release build also ran (10.295 seconds summed). No `FAILED` cases appear in these final logs. Logs: `/tmp/localwave-rust-blur-saturation-core.log`, `/tmp/localwave-rust-blur-saturation-app-complete.log`.
- ABI header/snapshot and dependency gates, C header compilation, app formatting/whitespace and app Clippy with interop/warnings denied pass (Clippy 0.89 seconds). Log: `/tmp/localwave-rust-blur-saturation-clippy.log`. The initial parallel invocations of the same gate executable raced over Cargo's executable link and one never launched; sequential ABI verification passes without changing the gate. Logs: `/tmp/localwave-rust-blur-saturation-header.log`, `/tmp/localwave-rust-blur-saturation-abi.log`, `/tmp/localwave-rust-blur-saturation-abi-sequential.log`, `/tmp/localwave-rust-blur-saturation-deps.log`.
- Release library/app builds take **22.04/6.20 seconds** after the code changes; the signed bundle verifier passes and smoke reports one frame/draw with **509.0 ms idle**. The rebuilt signed 10,000-song workload still has **zero warm Rust allocations** but **fails at 2,411 µs maximum wall build** (wall p50/p95/p99 288/552/1,353 µs; CPU p50/p95/p99/max 276/316/338/374 µs). This is additional failure evidence, not a stable latency claim. Logs: `/tmp/localwave-rust-blur-saturation-bundle.log`, `/tmp/localwave-rust-blur-saturation-bundle-check.log`, `/tmp/localwave-rust-blur-saturation-performance.log`.

Blur drawing still creates scratch textures on demand, temporary vectors and GPU wrappers; the zero-allocation UI-state test does **not** qualify that path. Fix 3 still requires resize-time target preparation and a measured allocation-free blurred layer. Vulkan shader validation is not a Vulkan runtime test. Native allocations, macOS 15, Intel, Swift/iOS/device/cloud and release qualification were not rerun. Local patches remain unpublished and the parent pin remains unchanged; the normal GUI process was left running.


## Blur target preparation checkpoint — 13 November 2026

P3 fix 3 now prepares both scratch textures at startup/drawable resize through `Renderer::resize` and `Painter::prepare_blur`, before recording commands. Both downsample factors reuse two RGBA8 allocations at half-window capacity. A blur draws only its occupied rectangle through viewport/scissor, returns that rectangle's UVs with its texture, and rejects absent/insufficient capacity instead of creating textures during drawing. The second filter clamps to the occupied texel centers so stale pixels outside it cannot bleed into animated/smaller regions. Half-texel UVs are calculated once on the CPU and passed in the existing uniform. Metal/GLSL and validated SPIR-V remain parallel. The additive resize export raises the ABI minor to **1.2**, without public layout changes. Local pxui checkpoint: `7358eac`.

The app calls resize only when drawable dimensions change, before beginning its native frame; minimized zero extents skip drawing. Replaced targets retire after the existing frames-in-flight window. New preparation failure paths release textures/pipeline/sampler already created. A blur recording failure also releases its wrapped source before returning, preventing the new missing-capacity refusal from leaking a GPU child. The ABI keeps split points in reusable storage and merges blur boundaries into the existing small stack break array, removing temporary blur/break vectors for layers within its existing 64-break stack capacity. Larger layers still have the existing heap fallback and remain part of the allocation work.

- **25 native Metal renderer cases pass** after the final half-texel change in **6.941 seconds** including rebuilding alongside the release build (4.278 seconds summed). Before that uniform-only change, the same 25 cases pass in **4.332 seconds** (1.746 summed). New cases check texture identity across region/sigma changes, odd and one-pixel regions, changing colors to detect stale-edge sampling, zero-allocation unchanged-capacity preparation, replaced-target retirement, missing/oversized capacity, zero extents, the shared export's existing callback guard and failed-blur teardown without leaked children. Existing blur color/alpha checks and scene goldens pass. Logs: `/tmp/localwave-rust-blur-preallocated-render-half-texel.log`, `/tmp/localwave-rust-blur-preallocated-render-cleanup-qualified.log`.
- The initial resize test expected a custom callback error string, but the shared export guard rejects renderer calls before the function runs. The corrected assertion checks that existing guard's message; the redundant in-function check was removed. The earlier failures remain in `/tmp/localwave-rust-blur-preallocated-render-final.log` and `/tmp/localwave-rust-blur-preallocated-resize-fixed.log`; the subsequent 24/25-case runs pass.
- **78 app checks pass in 11.327 seconds** including rebuilding, and the final warm run passes in **6.420 seconds** (0.05-second build; 7.338 seconds summed). **29 dynamic wrapper cases pass in 3.129 seconds** (0.821 summed) and **30 static cases in 3.487 seconds** (0.878 summed). ABI header/snapshot/dependency gates, C header compilation, app formatting and app Clippy with interop/warnings denied pass. Logs: `/tmp/localwave-rust-blur-preallocated-app-complete.log`, `/tmp/localwave-rust-blur-preallocated-app-final.log`, `/tmp/localwave-rust-blur-preallocated-wrapper-dynamic.log`, `/tmp/localwave-rust-blur-preallocated-wrapper-static.log`, `/tmp/localwave-rust-blur-preallocated-clippy.log`, `/tmp/localwave-rust-blur-preallocated-header.log`, `/tmp/localwave-rust-blur-preallocated-abi.log`, `/tmp/localwave-rust-blur-preallocated-deps.log`.
- A temporary five-frame allocation probe, removed before final qualification, measures **106 library + 13 host Rust allocations on the first blurred draw**, then **69 library + 13 host allocations on each of four warm draws**. Host counting excludes the library recorder call and includes only the callback's host pass teardown/recreation. This is a current failure baseline, not a zero-allocation blurred-layer pass; GPU command/pass/texture wrappers and larger break storage still need reuse. Log: `/tmp/localwave-rust-blur-preallocated-allocation-probe.log`.
- Initial release library/app builds take **15.24/6.88 seconds**; the final uniform/shader update takes **16.98/0.04 seconds**. The signed bundle passes minimum OS, dependency, entitlement and default-stack checks; final smoke draws one frame and remains idle for **501.4 ms**. An earlier signed regular UI workload passes at **639 µs maximum wall build** with zero warm Rust allocations. The final relinked bundle instead **fails at 1,742 µs** (wall p50/p95/p99 327/665/1,257 µs; CPU p50/p95/p99/max 288/358/399/445 µs), still with **zero warm Rust allocations across 182 frames/draws**. No thresholds changed and both results remain recorded. Logs: `/tmp/localwave-rust-blur-preallocated-bundle.log`, `/tmp/localwave-rust-blur-preallocated-final-bundle.log`, `/tmp/localwave-rust-blur-preallocated-final-bundle-check.log`, `/tmp/localwave-rust-blur-preallocated-performance.log`, `/tmp/localwave-rust-blur-preallocated-final-performance.log`.

Fix 3 remains incomplete until the whole blurred layer passes the allocation gate. The regular UI workload currently contains no blurred menu, so its allocation result cannot qualify that path. Native memory and Vulkan runtime remain unqualified. The normal GUI process was left running. Local patches are unpublished and the parent pin remains unchanged; the three earlier Metal edits remain separate. Swift/iOS/device/cloud, native Mac 15/Intel and release gates were not rerun; the two known Swift source-watcher failures remain.


### Resize callback guard follow-up

Review found that renderer status exports checked `in_frame` but could still enter a second renderer during another renderer's split callback on the same UI. `Shared::current` is null while a renderer owns the engine; the common guard now checks that fence too. A safe Rust regression creates two renderers on one UI and verifies the nested resize returns the existing state error while the outer blur continues to produce the expected pixels and both renderers tear down cleanly. Rejections use the existing shared state/poison error handling. This adds no new callback state or allocation.

After this change, **26 native Metal renderer cases pass in 5.365 seconds** (1.685 summed), **32 ABI/type cases pass in 5.493 seconds** (0.936 summed), and **78 app checks pass in 10.798 seconds** alongside the release build (9.359 summed; 0.06-second warm build). Final Clippy passes in 0.25 seconds, the dependency gate passes, formatting/whitespace pass, and the C header compiles. Logs: `/tmp/localwave-rust-blur-preallocated-callback-guard.log`, `/tmp/localwave-rust-blur-preallocated-abi-types.log`, `/tmp/localwave-rust-blur-preallocated-guard-app.log`, `/tmp/localwave-rust-blur-preallocated-guard-clippy.log`, `/tmp/localwave-rust-blur-preallocated-guard-deps.log`.

The guard rebuild takes **19.43/0.04 seconds** for library/app; signed verification passes and smoke remains idle for **501.6 ms** after one frame/draw. The latest signed performance run still has **zero warm Rust allocations** but **fails at 1,229 µs maximum wall build** (wall p50/p95/p99 284/641/1,188 µs; CPU p50/p95/p99/max 269/340/360/386 µs). Earlier 639 µs passing and 1,742 µs failing runs remain recorded. The pre-guard no-op bundle also measured **0.14/0.04-second library/app builds**. Logs: `/tmp/localwave-rust-blur-preallocated-guard-bundle.log`, `/tmp/localwave-rust-blur-preallocated-guard-bundle-check.log`, `/tmp/localwave-rust-blur-preallocated-guard-performance.log`, `/tmp/localwave-rust-blur-preallocated-warm-bundle.log`.

The 69-library/13-host warm blur allocation probe predates this guard-only change and is retained as the next failure baseline. Full blurred-layer allocation removal, stable strict latency and all remaining platform/release gates remain open.


### Reusable GPU passes and complete warm blurred draws

Prepared render targets validate/lower immutable attachment configuration once. Closed cached passes restart only on their original command wrapper, with per-call state/device/attachment-liveness validation; other drivers retain the existing creation fallback. Dropping prepared storage releases closed native passes even if an ended metadata clone survives. Metal retains checked native pass/encoder wrappers and rebinds the next encoder through owning `metal_sys` code. Vulkan retains attachment setup and converts up to its existing eight native attachments on the stack. Painter teardown drops cached passes before textures. No public ABI change (still 1.2). Local pxui checkpoint: `4ef7147`.

The renderer now reuses blur commands and foreign source wrappers; identity includes both native handles, extent and format. Foreign-image and blur-source maps have separate borrows and explicitly destroy wrappers after the existing flight window. Reopened host passes retain matching command/encoder pairs. Metal release draining preserves Vec capacity, merges concurrently queued releases, and performs native destruction outside its mutex.

Allocation investigation retained every failing checkpoint: **69 library + 13 host** warm allocations initially; wrapper reuse lowered library allocations to **26**; prepared passes reached **zero library**; the extended complete draw still allocated **one** object until release-queue capacity was retained. A temporary native backtrace identified that last allocation; all tracing was removed afterward. Logs: `/tmp/localwave-rust-blur-wrappers-baseline.log`, `-cached.log`, `-prepared.log`, `-full-draw-fixed.log`, `-last-allocation.log` (same prefix).

The final fixture counts UI property updates, host pass splitting/reopening and the full renderer call together, without subtracting host allocations. Six RGBA frames rotate three target colors and two blur regions with changing scale/sigma, checking both pixel regions every frame and requiring zero allocations after the three targets warm. A separate native BGRA target checks the app's real format, saturated pixels, command completion errors and zero warm allocations. It exposed BGRA import rejection in core and Metal; imported textures now use the general validated texture profile. Ordinary OGPU texture creation intentionally retains its existing format restriction.

- **29 renderer cases pass in 2.168 seconds**, 1.936 summed; **64 backend cases pass in 4.364 seconds**, 1.516 summed; **33 ABI/type cases pass in 3.517 seconds**, 0.891 summed. Prepared-target tests cover open-pass refusal, command/device mismatches, stale attachments and teardown; foreign-image tests cover wrapper retirement without destroying the caller's original texture. Logs: `/tmp/localwave-rust-blur-wrappers-render-bgra-final.log`, `-backends-final.log`, `-abi-types.log`.
- **78 app cases pass in 9.391 seconds**, 8.807 summed, with release compilation active; **30 static wrapper cases pass in 22.961 seconds** including rebuilding, 0.916 summed; **29 dynamic cases pass in 2.621 seconds**, 0.780 summed. All final logs report zero skipped cases. App Clippy with interop/warnings denied passes in 1.85 seconds. Formatting, whitespace, dependency, ABI snapshot/header/export gates pass. Logs: `/tmp/localwave-rust-blur-wrappers-app-bgra-final.log`, `-wrapper-static.log`, `-wrapper-dynamic.log`, `-clippy-final.log`, `-deps-final.log`, `-header.log`, `-abi-snapshot.log`, `-exports.log`.
- Native Linux cross-checking was attempted, but `x86_64-linux-gnu-g++` is missing (`/tmp/localwave-rust-blur-wrappers-linux-check.log`). A Rust-only `cargo check -p ogpu_vulkan` passes in 3.53 seconds with module platform guards temporarily enabled for type checking on macOS; both files were restored byte-for-byte. This does not compile/link Vulkan native C++ or qualify Linux runtime. Log: `/tmp/localwave-rust-blur-wrappers-vulkan-rust-check.log`.
- The final signed bundle builds, verifies minimum OS/default stack and stays idle **509.9 ms** after one frame/draw. The final app release build takes **9.63 seconds**. Signed performance still fails at **4,427 µs maximum wall build**, with zero warm Rust allocations across 182 frames/draws; full timing distributions are recorded above. Logs: `/tmp/localwave-rust-blur-wrappers-bgra-bundle.log`, `-bundle-check.log`, `-performance.log`.

This proves zero warm Rust allocations for the tested blur paths, not every layer or native allocation. The existing >64-boundary heap fallback remains next. Vulkan runtime, native memory, strict stable latency, complete Mac/iOS UI and release/device/cloud gates remain open. Local pxui patches are unpublished and the parent pin still names the remote baseline; three pre-existing Metal compatibility edits remain separate. The normal GUI process was left running and requires quit/reopen to load the new bundle. Swift/iOS checks were not rerun; the two known source-watcher failures remain.


### Large-layer boundaries without a per-draw heap fallback

Local pxui checkpoint: `2512c17`. The preceding reusable-pass checkpoint is `4ef7147`; neither is published.

The old stack array allocated a fresh Vec whenever a layer exceeded 64 image/blur boundaries. A native 96-image regression reproduces exactly **one warm Rust allocation** in `/tmp/localwave-rust-large-breaks-baseline-canvas.log`. The earlier widget version did not reach 64 boundaries because layout clipped its rows; the corrected fixture paints every rectangle explicitly and asserts the boundary count.

Private painter breaks now contain cheap cloned Texture handles instead of frame-borrowed references. The renderer keeps one Vec sized for the largest observed layer, reserves once for all texture/blur boundaries and clears its handles after each layer (including split/draw errors). This removes both stack/fallback paths without lifetime casts or public ABI changes. Existing direct painter callers use nonallocating Texture clones. The flight-window source caches still own retirement; scratch storage keeps no texture handles after a layer finishes.

The image regression varies **65/128/96/128/65/64/96 boundaries**, requires zero warm draw allocations, verifies every red/blue image pixel and pins one draw per alternating texture segment. The large-blur regression adds 96 transparent alternating image boundaries to the existing two-blur, three-target fixture; it preserves changing sigma/scale, both readbacks, full host/library allocation counting and clean teardown. Both cases pass.

- **31 native Metal renderer cases pass in 6.087 seconds**, 2.805 summed. **13 GPU parity cases pass in 5.096 seconds**, 1.354 summed, including rounded clips, retained bytes, widget draw counts and existing scene goldens. **78 app checks pass in 6.564 seconds**, 8.167 summed. Logs: `/tmp/localwave-rust-large-breaks-render.log`, `-parity.log`, `-app.log`.
- **30 static wrapper cases pass in 3.662 seconds**, 0.906 summed; **29 dynamic wrapper cases pass in 0.911 seconds**, 0.881 summed. Dependency, ABI header/snapshot, formatting and whitespace gates pass; Clippy with all app targets, `lw_apple_sys/interop-test` and warnings denied passes in **0.16 seconds**. The first Clippy invocation named a nonexistent feature and was corrected without source changes; both logs remain. Logs: `/tmp/localwave-rust-large-breaks-wrapper-static.log`, `-wrapper-dynamic.log`, `-deps.log`, `-header.log`, `-abi.log`, `-clippy.log`, `-clippy-fixed.log`.
- The signed release library/app builds take **18.80/0.15 seconds**. Bundle verification passes minimum OS/default stack; smoke records one frame/draw and **500.4 ms** idle. The strict signed 10,000-song performance run still **fails at 2,889 µs maximum wall build**, with zero warm Rust allocations across 182 frames/draws; distributions are recorded above. The preceding 4,427 µs failure is retained. Logs: `/tmp/localwave-rust-large-breaks-bundle.log`, `-bundle-check.log`, `-performance.log`.

The >64-boundary allocation limit is removed. Warm allocation claims remain limited to the tested workloads; native allocators, Vulkan runtime, cold text/new scripts and complete interactive menu/scrubbing workloads require their own qualification. The 1 ms maximum remains unchanged and unresolved. Local patches are unpublished; the normal GUI remains running on its older mapped build. Swift/iOS/device/cloud/release checks were not rerun, and the two known Swift source-watcher failures remain.

The final documentation-only painter rebuild takes **15.77/0.05 seconds** for library/app; signed verification passes and smoke stays idle **503.3 ms** after one frame/draw. Logs: `/tmp/localwave-rust-large-breaks-final-bundle.log`, `-final-bundle-check.log`.

The final relinked signed performance run still **fails at 1,728 µs maximum wall build** (wall p50/p95/p99 282/590/1,607 µs; CPU p50/p95/p99/max 269/324/337/337 µs), with **zero warm Rust allocations across 182 frames/draws**. The earlier 2,889 and 4,427 µs failures remain recorded. No thresholds changed. Log: `/tmp/localwave-rust-large-breaks-final-performance.log`.


### OpenType tabular digits and feature-aware layout caches (ABI 1.3)

Local pxui checkpoint: `8784e43` (unpublished).

P3 fix 5 now carries a validated feature bitset through public TextStyle, the shared ABI decoder and cosmic-text's existing shaping path. Bit 0 enables `tnum`; default 0 preserves font defaults, and unknown bits are rejected. The former reserved u32 becomes `features`, retaining its byte offset 28, total C size 36 and tracking offset 32. Numeric layout snapshots remain unchanged; the append-only snapshot only advances its minor version to 3. Named source initializers must rename the former reserved field; zero-initialized binary callers keep their default behavior. No dependency or profile changes.

Both caller-keyed Style equality and the core content-derived text hash include feature flags. Cold shaping may allocate its OpenType feature list; warm measurement and glyph replay return through existing caches before feature-list construction. Removing the feature from the core hash deliberately reproduces **88 allocations per warm mixed-style canvas frame**; restoring it passes. The source was restored byte-for-byte after that experiment. The original implementation first rejects the new bit with argument error -1; both failing logs remain. Logs: `/tmp/localwave-rust-text-features-baseline.log`, `-missing-cache-key.log`.

The Inter fixture proves a real proportional-to-tabular digit substitution and equal advances for all ten digits. It is copied from cosmic-text 0.19.0's test fonts, with its SIL license and recorded SHA-256 in pxui/docs/abi-baseline.md; it is compiled only into tests. Production Host Grotesk/Geist Mono are unchanged. Host Grotesk's checked-in GSUB table has no tnum lookup, so it would not establish that the new flag reaches shaping.

- **80 text/static-wrapper/ABI/type cases pass in 7.818 seconds**, 2.925 summed; **31 dynamic wrapper cases pass in 3.220 seconds**, 0.899 summed. Cases cover all digit advances, same-key feature toggles, invalid flags and zero warm measurement/glyph/canvas allocations across the library boundary. Logs: `/tmp/localwave-rust-text-features-static-text-abi.log`, `-wrapper-dynamic.log`, `-focused.log`, `-canvas.log`.
- **44 renderer/parity cases pass in 8.179 seconds**, 5.151 summed, preserving existing goldens, large-layer and RGBA/BGRA blur allocation checks. **78 app cases pass in 8.528 seconds**, 7.387 summed. No skipped cases or failing final cases appear. App Clippy with all targets, interop and warnings denied passes in **1.27 seconds**. Logs: `/tmp/localwave-rust-text-features-render-parity.log`, `-app.log`, `-clippy.log`.
- C header compilation asserts style size 36, feature offset 28 and tracking offset 32. Header/snapshot/export/dependency gates and app formatting/whitespace pass. The release library/app builds take **20.56/6.60 seconds**. Signed bundle verification passes minimum OS/default stack; smoke draws once and stays idle **509.8 ms**. Logs: `/tmp/localwave-rust-text-features-abi-write.log`, `-header.log`, `-exports.log`, `-deps.log`, `-bundle.log`, `-bundle-check.log`.

The normal GUI remains running on its older mapped build. Local pxui changes are unpublished; the parent pin and the three pre-existing Metal compatibility files remain separate. The strict 1 ms wall maximum, native memory, full Mac/iOS UI, remaining renderer fixes and device/cloud/release qualification remain open. Swift/iOS checks were not rerun; the two known source-watcher failures remain.

The signed 10,000-song run after this text change still **fails at 1,943 µs maximum wall build**, with **zero warm Rust allocations across 182 frames/draws** (wall p50/p95/p99 279/488/1,789 µs; CPU p50/p95/p99/max 261/301/339/365 µs). The earlier 1,728 µs failure and passing runs remain recorded; the strict 1 ms maximum is unchanged. Log: `/tmp/localwave-rust-text-features-performance.log`.


### Inline weights, colours, backgrounds and rules (ABI 1.4)

P3 fix 6 now exposes a fixed eight-span packet through ordinary text nodes, canvas text and standalone/UI measurement. TextSpan has UTF-8 byte start/end, weight (0 inherits), flags and independent foreground/background/underline colours. TextSpan/TextSpans are **24/200 bytes**; the existing TextStyle and glyph/instance layouts remain unchanged. The header and wrapper add four functions, the append-only snapshots add the two types, and the ABI minor becomes **1.4**. The private text crate reuses the public plain span records through one internal dependency; no external dependency/profile/target-directory change.

Plain Style and plain nodes avoid inline eight-record arrays. Rich layouts and nodes allocate optional fixed span storage once and retain/recycle it; cached comparisons/keys include active span fields. Plain text and patch_text clear active styles. The source string remains the complete accessibility label. A node's explicit style controls font/wrapping/ellipsis/start alignment; span colours/decorations follow its normal opacity and clip.

Weight spans use existing shaping. Colour boundaries cut the original bitmap using the text provider's caret geometry, preserving Arabic ligatures and font context rather than reshaping separate pieces. Cached glyphs retain cuts/colours and replay identically to fresh walks at fractional origins and 1/1.5/2x scales. Backgrounds and independent **1.5-point bottom rules** stream from shaped runs, including disjoint mixed-bidi ranges, without allocating decoration vectors. All output remains in the existing instanced layer: native cases pin **one draw per layer**. No shader, extra render pass or GPU resource is added.

- **88 static text/wrapper/ABI/type cases pass in 16.790 seconds** including rebuilding (3.357 summed); **33 dynamic wrapper cases pass in 3.409 seconds**, 0.985 summed. Zero warm allocations cover complete rich/plain node rebuilds, changing opacity, accessible labels, mixed span colours, glyph replay and decorations. Tests cover all eight spans, invalid UTF-8/ranges/flags, null/short/oversized packets, same-key changes, CRLF, wrapping, ellipsis, bidi and fractional origins/scales. The focused weight check verifies the same glyph-cache entries as a whole bold layout. Logs: `/tmp/localwave-rust-inline-qualified-static.log`, `-qualified-dynamic.log`, `-weight.log`. Earlier static run: 88 cases, 3.736 seconds; contention retries are retained and pass.
- **46 native renderer/parity cases pass in 7.953 seconds**, 3.923 summed, preserving existing goldens, large-layer and RGBA/BGRA blur gates. New native readbacks verify red glyphs over a translucent blue background, a separately green rule, and partial Arabic-ligature coverage matching the original within the existing two-level tolerance. Both paths have zero warm draw allocations. The earlier native background test sampled the antialiased edge and expected interior coverage; it now samples 1.25 points inside, without changing renderer behaviour. Logs: `/tmp/localwave-rust-inline-qualified-render.log`, `-gpu.log`, `-gpu-pixels.log`, `-gpu-corrected.log`.
- **78 app cases pass in 10.172 seconds**, 7.912 summed. App Clippy with all targets, interop and warnings denied passes in **3.08 seconds**. Header/snapshot/export/dependency, C size/offset assertions, app formatting and whitespace checks pass. Initial release library/app builds take **19.63/6.08 seconds**; the final source cleanup builds in **22.36/5.40 seconds**. Signed verification passes minimum OS/default stack and smoke stays idle **508.2 ms** after one frame/draw. Logs: `/tmp/localwave-rust-inline-app.log`, `-clippy.log`, `-header.log`, `-abi-final.log`, `-exports.log`, `-deps.log`, `-bundle.log`, `-qualified-bundle.log`, `-qualified-bundle-check.log`.

Inter-Regular's default fi sequence did not form the assumed ligature, so that initial fixture assertion failed. The corrected test uses the dependency's real Noto Sans Arabic lam-alef ligature; its unchanged test-only font and SIL license are included with a recorded SHA-256 in pxui/docs/abi-baseline.md. No production font changes. Logs: `/tmp/localwave-rust-inline-text-tests.log`, `-text-tests-arabic.log`.

This establishes the tested span paths, not complete script/font/platform or application parity. Local pxui patches are unpublished, the parent pin remains at the remote baseline, and the three earlier Metal compatibility edits remain separate. The normal GUI remains on its older mapped build until quit/reopen. Strict stable latency, native memory, remaining renderer/UI features and Mac/iOS device/cloud/release qualification remain open. Swift/iOS checks were not rerun; the two known source-watcher failures remain.


### Inline descriptor validation and fractional font alignment

Review found that text styles were copied before their size field was validated. A shared read_style now checks the prefix first for both existing and new measurement/quads/raster/canvas paths. Span packets check size/count before reading records, reject unknown flags and read only active scalar fields and flagged colours into a canonical packet. Tests include null pointers, actual eight-byte short styles, invalid-count/empty headers, and a weight-only active prefix without colour bytes. Invalid descriptors return before changing node/layout output.

Rich node centering also keeps fractional font points instead of rounding them to an integer first. A 20.25-point node and canvas regression compares matching glyph rectangles, UVs and colours. This retains integer-size rendering and baseline alignment.

- **90 text/static-wrapper/ABI/type cases pass in 5.855 seconds**, 3.636 summed; **34 dynamic wrapper cases pass in 3.943 seconds**, 0.947 summed; **46 renderer/parity cases pass in 5.389 seconds**, 3.013 summed; **78 app cases pass in 9.130 seconds**, 8.150 summed. Logs: `/tmp/localwave-rust-inline-final-static.log`, `-final-dynamic.log`, `-final-render.log`, `-final-app-corrected.log`, `-descriptor-guard-fixed.log`, `-fractional.log`. An initial let-else parenthesis compile error was corrected and its log retained; the root app runner also failed to launch with an unnecessary target-directory environment override, then passed with the documented ordinary root command.
- The final signed library/app builds take **18.39/0.04 seconds**. Bundle checks pass minimum OS/default stack and smoke stays idle **500.4 ms** after one frame/draw. Logs: `/tmp/localwave-rust-inline-final-bundle.log`, `-final-bundle-check.log`. No profiles, toolchains, thresholds or target-directory layout changed.
- Before these final guards, the signed 10,000-song run still **failed at 2,219 µs maximum wall build**, with zero warm Rust allocations across 182 frames/draws (wall p50/p95/p99 280/1,035/2,064 µs; CPU p50/p95/p99/max 271/323/371/399 µs). Log: `/tmp/localwave-rust-inline-performance.log`. Earlier 1,943 µs and other failures remain retained; the strict 1 ms maximum stays open.


### Final inline span checkpoint

Rich nodes now center the entire wrapped block using fractional font size and actual line count/height; baseline layout uses the current measured width. A three-line node/canvas comparison pins identical glyph rectangles, UVs and colours alongside the 20.25-point case. Local pxui checkpoint: `1516f2d`. This completes the tested P3 fix 6 implementation; broader script/font/platform qualification remains open.

The final runs pass **91 static**, **35 dynamic**, **46 renderer/parity** and **78 app cases** (250 total), with zero skips. Wall times are **4.802/3.828/8.564/15.331 seconds**; summed case times are **3.671/0.985/2.848/10.681 seconds**. The app run overlaps release compilation and retains a native SDL contention retry. App Clippy passes in **0.19 seconds** with warnings denied; the two existing native Metal availability warnings remain. ABI header/snapshot/export/dependency gates, app formatting and whitespace checks pass. Signed library/app builds take **22.22/0.03 seconds**; bundle checks pass and smoke observes **509.9 ms** idle after one frame/draw. Logs: `/tmp/localwave-rust-inline-complete-{static,dynamic,render,app,clippy,bundle,bundle-check,abi-header,abi-snapshot,abi-exports,deps}.log`.

The descriptor/fractional checkpoint's signed workload failed at **1,412 µs** (wall p50/p95/p99 303/829/1,237 µs; CPU p50/p95/p99/max 293/349/359/364 µs). The final wrapped-block checkpoint still **fails at 3,091 µs** (wall 290/540/1,302 µs; CPU 281/346/362/374 µs), with **zero warm Rust allocations across all 182 frames/draws**. Logs: `/tmp/localwave-rust-inline-final-performance.log`, `-complete-performance.log`. Earlier failures remain; the strict 1 ms threshold is unchanged. Native allocations, stable latency, full UI and Mac/iOS device/cloud/release gates remain unqualified.


### Head and middle ellipsis (ABI 1.5)

Local pxui checkpoint: `bdc6852`. P3 fix 7 adds values to the existing TextStyle.ellipsis byte: NONE=0, END=1, MIDDLE=2, HEAD=3. The original 0/1 behaviour and 36-byte C layout remain; unknown values are refused by the shared decoder for every measurement/raster/quads/node/canvas entry. The private Style uses a typed mode and delegates to cosmic-text 0.19's existing End/Middle/Start implementations. Layout cache comparisons and the core content key include the mode. FIT_TEXT supplies END only when no explicit mode exists. Accessible labels remain the complete original text. No exports, external dependencies, shader/resource changes or public layout changes.

The new multi-paragraph regression exposed an existing bug: the provider applied its line limit per paragraph, producing four visible lines for a two-line limit. The buffer now also receives the whole-block height limit for ellipsis, including the default one-line case. CRLF, empty/fitting text, word/glyph wrapping, Unicode paths and UTF-8 hit indices pass. An initial overly broad width assertion counted hanging blanks; readback showed ink fitting while the provider's advance included the trailing blank. Font overhangs retain their natural bounds. No renderer clipping or tolerance was changed.

- **96 static text/wrapper/ABI/type cases pass in 3.033 seconds**, 3.585 summed; **37 dynamic cases pass in 7.269 seconds**, 1.064 summed; **47 native renderer/parity cases pass in 4.028 seconds**, 3.684 summed; **7 core text cases pass in 7.656 seconds**, 0.233 summed; **78 app cases pass in 15.816 seconds**, 10.557 summed, with release compilation active. All zero skips. Logs: `/tmp/localwave-rust-ellipsis-{static,dynamic,render-complete,core-text,app}.log`. The earlier name-filtered renderer run passed 28 cases but excluded shared renderer cases; the final explicit test-target run above covers all 47.
- Native readbacks verify original red/blue endpoints for each mode, one instanced draw per layer and zero warm Rust draw allocations. A new 640×320 @2x golden is generated, visually inspected and replayed without update mode. Wrapper checks compare node/canvas rectangles/UVs/colours, retain full accessible labels, count zero warm allocations while modes alternate, and compare standalone/UI measurement/raster output. Logs: `/tmp/localwave-rust-ellipsis-{focused,focused-limit,width-diagnostic,focused-visible,focused-final,wrapper,gpu-write,gpu-corrected}.log`. The first native test compile used the wrong Size re-export; corrected to the existing ui::Size with no API change.
- ABI header/snapshot/exports/dependency gates pass. C compilation pins mode values, style size 36 and offsets ellipsis=27/features=28/tracking=32. Formatting, whitespace and app Clippy with all targets/interop/warnings denied pass; Clippy takes **5.45 seconds**. Signed release library/app builds take **25.96/7.60 seconds**; bundle verification passes and smoke observes **509.8 ms** idle after one frame/draw. Logs: `/tmp/localwave-rust-ellipsis-{abi-header,abi-snapshot,exports,deps,clippy,bundle,bundle-check}.log`.
- The latest signed performance workload still **fails at 1,823 µs maximum wall build**, with **zero warm Rust allocations across 182 frames/draws** (wall p50/p95/p99 318/700/946 µs; CPU p50/p95/p99/max 295/345/365/389 µs). Log: `/tmp/localwave-rust-ellipsis-performance.log`. The preceding 3,091 µs failure and earlier results remain recorded. The strict 1 ms maximum remains unchanged; no scheduling cause has been established.

Local patches remain unpublished and the parent submodule pin stays at the remote baseline. Normal GUI PID 69331 remains on its older mapped build. Native memory, stable latency, remaining renderer/full UI, iOS/device/cloud and release gates remain open. Swift/iOS checks were not rerun; the two known source-watcher failures remain.


### Latency trace attempt after ellipsis

A `System Trace` capture was launched against a separate `--performance-check --trace-frames` process with a 20-second recording limit. Instruments stalled in startup: recorder PID 79906 remained live, target PID 79918 stayed stopped before main, and the target output file remained empty after more than a minute. Developer mode reports enabled. SIGINT did not finish startup; SIGTERM stopped the recorder and its target, confirmed by process inspection. Normal GUI PID 69331 remained alive and untouched. No trace-based scheduling, blocking or native-allocation claim follows from this attempt. Logs: `/tmp/localwave-rust-ellipsis-system-record.log`, `-system-diagnostics.log`, `-traced-performance.log`; template/help output is retained separately. The independent signed 1,823 µs failure remains the current performance result. Use a working profiler capture or correlate individual slow-frame phases before claiming a cause.


### Apple system font discovery (ABI 1.6; P3 fix 8 in progress)

Local pxui checkpoint: `4699263`. A new std-only open font_sys crate owns CoreText/CoreFoundation FFI and releases every native Create/Copy result. The text engine queries it only for missing shaped UTF-8 clusters. It first tries CoreText's preferred substitute/cascade for the font system's locale, then a charset-constrained OS font-registry match when that cascade has no rasterizable face. It checks coverage, standard outline/bitmap tables, URL type and an absolute local path; LastResort is refused. There is no font-directory scan or font registration. Existing fontdb memory mapping loads the chosen file/collection; its coverage/fallback caches resolve subsequent characters in that script without another native request when the loaded face covers them. No registry dependency is added.

Initial CJK shaping succeeded but produced no raster pixels: the selected PingFangUI.ttc exposes hvgl outlines, unsupported by Swash. Both the UI and a Helvetica cascade initially selected that file. The compatibility/table check plus OS registry match fixes the real failure; tests now require nonzero glyph IDs, actual raster output and native pixels. An alternate compatible face can differ from SwiftUI's preferred system face, so exact font/script/reference parity remains unqualified. Current validation covers CJK, Cyrillic, Arabic, command symbols, emoji and a joined emoji sequence; it does not establish complete Unicode coverage.

Text keeps optional native discovery state. Successful font paths are loaded once (at most 128 files, documented in code); negative clusters are cached up to 512 entries of at most 256 bytes. Clearing negative storage permits later cold retries without growing the cache. Missing clusters in one layout are deduplicated. New files trigger reshaping before measurement/glyph output. Warm unchanged layouts return before discovery, and cached glyphs reuse their atlas entries. A system-only shaper bootstraps a real font before the first shape. These first-sighting paths allocate and may map files; they are not warm-frame guarantees.

Two additive C exports and safe wrapper methods enable discovery: pxui_shaper_system_fonts and pxui_text_system_fonts / Text::enable_system_fonts and Ui::enable_system_fonts. Ui enabling is rejected after the first frame to avoid stale retained output; standalone enabling invalidates earlier missing-font layouts. Repeated early enabling is idempotent. Explicit-font constructors stay deterministic. The provider is Apple-only; other platforms return the existing state error and keep explicit fonts. The app enables it before its first frame. ABI minor becomes 1.6, with no existing layout/signature changes.

- **102 static text/wrapper/native-font/ABI/type cases pass in 3.271 seconds**, 3.802 summed; **39 dynamic wrapper cases pass in 3.313 seconds**, 1.174 summed; **48 Metal renderer/parity cases pass in 16.971 seconds**, 6.145 summed, overlapping release compilation; **7 core text cases pass in 11.659 seconds**, 0.234 summed, including rebuild/lock wait; **78 app cases pass in 9.589 seconds**, 8.193 summed. All report zero skips. The initial static/rebuild run passes in 28.752 seconds, 4.903 summed. Logs: `/tmp/localwave-rust-system-font-{static,static-warm,dynamic,render,core-text,app-recheck}.log`.
- Native tests require actual black script coverage and yellow colour-emoji pixels, **one draw per layer**, and **zero warm Rust allocations for UI build and draw**. Public wrapper checks retain complete accessible labels, compare standalone/UI metrics, rasterize after enabling, preserve warm measurement/node/canvas storage, and refuse late enabling. Native system-only bootstrap and unsupported-cluster cache bounds pass. Earlier failing CJK shaping/raster logs remain: `/tmp/localwave-rust-system-font-{native,raster-diagnostic,non-ui,cascade,cascade-diagnostic,registry,wrapper,gpu}.log`.
- Header/snapshot/export/dependency/tier gates, formatting, whitespace and app Clippy with all targets/interop/warnings denied pass. Clippy takes **1.09 seconds**. A core check overlapping tests waits on the build lock and takes 10.96 seconds; the quiet warm check takes **0.12 seconds**. The first root app runner bootstrap fails before execution with ENOENT during concurrent Cargo work, then the ordinary root command passes after the other builds finish. The launch failure is retained without attributing its cause. Logs: `/tmp/localwave-rust-system-font-{abi-write,abi-header,abi-snapshot,exports,deps,tiers,clippy,core-warm-check,core-quiet-check,app,app-recheck}.log`.
- Signed release library/app builds take **23.50/8.04 seconds**. Bundle verification passes signatures, dependency/sandbox, minimum OS and default stack checks; smoke draws once and observes **500.4 ms** idle. The signed 10,000-song run still **fails at 1,614 µs maximum wall build**, with **zero warm Rust allocations across 182 frames/draws** (wall p50/p95/p99 332/597/1,187 µs; CPU p50/p95/p99/max 329/358/384/391 µs). Logs: `/tmp/localwave-rust-system-font-{bundle,bundle-check,performance}.log`. The prior 1,823 µs and other failures remain; no thresholds changed.

P3 fix 8 remains incomplete until second atlas pages and font/platform/reference qualification pass. Native allocations, strict stable latency, remaining renderer/full UI, iOS/device/cloud and release gates remain open. Swift/iOS were not rerun; the known source-watcher failures remain. The three prior Metal compatibility edits and parent pin remain separate; normal GUI PID 69331 stays on its older mapped build. No patches were pushed.


### Second glyph atlas pages and portable test branch (ABI 1.7)

PXUI commit `55ba80a` adds two fixed pages per mask/colour kind; `9e61a50` separately records the existing macOS Metal compatibility changes. Both are published on PXUI `rust-port`. LocalWave `rust-port` pins the matching engine commit so another machine can fetch the entire checkpoint. All current workspace changes, including the terminal target removal and Swift source-scan fixes, are included in the parent checkpoint. The shipping Swift UI is retained.

The extra CPU mirrors and GPU textures allocate on first use. Default CPU mirrors have a 12 MiB ceiling across all four pages; layouts and font mappings retain their existing separate bounds. Existing texture handles and UVs remain stable. Allocation tries fresh space on both pages before safe shelf reuse/retirement. Eight shelf-mark words preserve the prior per-page collision limit and protect retained/in-flight output. Atlas codes 0/1 remain the first mask/colour pages, 2/3 name the second; querying an unprepared second GPU page returns the existing state error. One existing instance flag selects the page without increasing instance size or adding a draw. New GPU pages receive a complete zero-padded mirror in the app command buffer even when they activate after earlier prepares; failed uploads force a full retry.

All 450 bounded Rust cases listed above pass. Shader compilation/SPIR-V validation, C header, ABI header/snapshot/export, dependency/tier, app formatting/Clippy and whitespace checks pass. A broader engine Clippy attempt fails existing chunks_exact/type-complexity/range-loop and open-crate lints; it is not a passing engine lint result. The churn fixture uses two smaller pages with the same total capacity as before, preserving its eviction, in-flight texel and miss-rate assertions.

The signed library/app release builds take **21.92/5.99 seconds**. Bundle verification enforces both Mach-O minimums, the default stack, dylib ID **@rpath/libpxui.1.dylib**, the sole app rpath **@executable_path/../Frameworks**, signatures and sandbox entitlements. Smoke passes after one frame/draw and **504.7 ms** idle. The latest strict workload passes at **854 µs** maximum with zero warm Rust allocations; earlier timing failures remain recorded. Logs are under `/tmp/localwave-rust-atlas-pages-*`.

P3 fix 8 still needs complete font/reference and platform qualification. Native memory, stable latency, remaining engine fixes/full UI, iOS/device/cloud and release gates remain open. Swift/iOS were not rerun; the known source-watcher failures remain. The normal GUI stays on its older mapped build until the user quits and reopens it.

The same signed bundle was copied to an unrelated temporary directory and passed signature/rpath verification plus its real SDL/Metal smoke launch (508.7 ms idle). macOS emitted a nonfatal sandbox-extension diagnostic for that temporary path; this is portability evidence, not file-access or upgrade qualification. Log: `/tmp/localwave-rust-atlas-pages-relocated-bundle.log`. The exact CI app Clippy command with interop enabled also passes in 1.67 seconds; the broader engine lint failures remain separate.


### Live host appearance and reduced motion (ABI 1.8)

The complete FrameIn now appends appearance (unknown/light/dark) and reduced_motion (0/1) after its frozen 48-byte prefix. Actual old allocations execute through the C export with default host facts; partial tails are refused without over-reading; larger descriptors are read only through known fields. Rust/static and dynamic paths validate the same codes. The append-only snapshot preserves the original prefix row and adds the 56-byte FrameInV1_8 row. New system_appearance and use_theme exports expose the last host appearance and select a previously parsed root theme between frames.

Changed host facts invalidate retained views once; unchanged values keep them. OS reduced motion wins over scoped/root themes while preserving authored flags for restoration. Enabling it snaps active tween/keyframe runs to their final pose, suppresses new explicit node transitions and removes their repaint deadlines. Tests pin warm allocations, final geometry, stable cached views and immediate native pixels with one draw.

The main-thread native Environment observes [AppKit effectiveAppearance](https://developer.apple.com/documentation/appkit/nsapplication/effectiveappearance) and [NSWorkspace accessibility display changes](https://developer.apple.com/documentation/appkit/nsworkspace/accessibilitydisplayoptionsdidchangenotification). Its notification center is the workspace's own, as required by AppKit. Only startup/changes read OS properties; the frame loop borrows cached scalars. Changed values wake SDL once, duplicate notifications do not wake, and weak queued callbacks/observer removal protect teardown. The native regression uses actual per-process AppKit appearance changes and an isolated workspace notification source; it does not modify global preferences or the ordinary GUI. Full user-driven OS/upgrade qualification remains open.

The library worker loads saved settings into controller status; the app combines the saved motion restriction with the OS signal and honours saved System/Light/Dark selection. Two parsed themes remain cached. Native window backgrounds update only when the engine reports a changed background. A controller integration assertion checks saved Dark/reduced-motion settings reach the main-thread status without main-thread defaults reads. Settings editing and refresh remain part of the unfinished full UI.

All **457 Rust checks** above pass. Updated controller fixture: **5.669 seconds wall / 2.055 case**. Header/snapshot/export/dependency/tier gates, explicit C descriptor offsets/sizes, all-target Gallery/ABI consumer checks, app Clippy/formatting and whitespace pass. Warm core check: **0.14 seconds**. Broader engine Clippy was not rerun; the prior baseline failures remain.

Release library/app builds take **21.69/12.60 seconds**, then **0.13/7.30 seconds** for the final library/app rebuild after native background delivery. Signed bundle checks pass and smoke records one frame/draw then **503.1 ms** idle. The latest strict performance run **fails at 2,314 µs wall maximum**, with **zero warm Rust allocations** across 182 frames/draws; CPU maximum is 364 µs. The previous 854 µs pass and earlier failures remain recorded. This does not qualify stable latency or establish a cause for the wall/CPU gap. Logs: `/tmp/localwave-rust-system-preferences-*`.

PXUI checkpoint `87e413c` is published on rust-port before updating the parent pin. The final app rerun passes all 81 cases in **9.459 seconds wall / 7.435 summed**, including the saved-settings assertion. Native allocations, macOS 15/Vulkan runtime, complete reference UI, iOS/device/cloud and release qualification remain outstanding. Swift/iOS were not rerun; their earlier source-watcher failures remain. Continue on rust-port, committing milestones and publishing PXUI before its parent pointer.

### Retained view controls and measured lists (ABI 1.9)

P3 fix 10 now exposes the existing core view opacity, uniform scale, clip and measured virtual-list operations through the C ABI and public Rust wrapper. These are four additive exports with no existing layout/signature changes. Opacity is per instance; clipping uses window points with an optional rectangle. Finite zero/negative scales are supported and reset earlier manual rotation/fixed pivots. Repeated equal controls preserve table storage and retained instances, avoiding redundant geometry work. Measured lists reuse the existing cache and callback guard, validate row/estimate limits before invoking rows, and build only virtual rows while settling actual heights.

Static/dynamic and C regressions cover validation, borrowed row handles, callback panic recovery, empty lists, scrolling, stable instances, hit clips and zero warm Rust allocations. Native Metal readbacks check nested alpha/scaling/clipping and settled variable-height rows, including draw counts and warm build/draw allocations. All **468 Rust checks** pass with zero skips: 111 static/text/ABI/type cases in **5.574 seconds wall / 4.030 summed**, 44 dynamic cases in **3.119/1.326**, 232 core/Metal cases in **9.415/10.244**, and 81 app cases in **11.931/8.530**. Header/snapshot/export/dependency/tier gates, Gallery/ABI consumer checks, app Clippy/formatting and whitespace pass. Quiet warm core check: **0.11 seconds**. Earlier engine Clippy baseline failures remain.

Release library/app builds take **19.46/6.86 seconds**. Signed bundle verification passes, including the embedded `@rpath/libpxui.1.dylib`, sole app rpath `@executable_path/../Frameworks`, default stack and macOS 15 minimum; smoke observes one frame/draw then **508.6 ms** idle. The ordinary signed 10,000-song workload passes the unchanged 1 ms maximum at **613 µs**, with **zero warm Rust allocations** across 182 frames/draws. Wall p50/p95/p99: **293/472/601 µs**; CPU p50/p95/p99/max: **286/353/369/372 µs**. Prior failed runs remain recorded; one pass does not qualify stable latency or prove these changes caused the improvement.

A separate bounded Time Profiler/Points of Interest recording yielded no application output and an incomplete trace. Export fails with **Document Missing Template Error**, exit 10. No recorder or isolated profiling target remains; the ordinary GUI was preserved. There are no usable samples to explain earlier outliers. Logs: `/tmp/localwave-rust-view-controls-*`; incomplete trace: `/tmp/localwave-time-profile-4y3_tn49/Run.trace`.

PXUI checkpoint `05069a5` is published on rust-port before updating the parent pin. The full reference UI, native memory and stable latency, macOS 15/Vulkan runtime, iOS/device/cloud and release qualification remain unfinished. Swift/iOS were not rerun; earlier source-watcher failures remain. Continue on rust-port and publish the engine before the parent pointer.

### Worker catalog refresh and view identity

The library heading now has an accessible Refresh library button. Its bounded controller command coalesces pending refreshes without fencing or cancelling native playback preparation. The worker reads songs, playlists and membership in one deferred SQLite read transaction; formatting, grouping and sorting stay there. RAII rollback preserves connection reuse on errors. Import/edit commands must request this refresh after committing when they are connected; automatic source observation remains unfinished.

The mailbox holds at most one incoming and one retired catalog. The main thread repairs view state while borrowing both snapshots, outside the mailbox mutex, then transfers the old owner into the retirement slot. A pending retirement applies backpressure to adoption. The worker drops superseded/retired catalogs outside the mutex and wakes the host if a replacement was waiting. This qualifies live snapshot exchange; final application shutdown/native memory remain separate qualification. The current-song metadata cache invalidates on a successful refresh. A failed read keeps the last good catalog and playback session, reports a retry message, and recovers on a later request without clearing an unrelated playback notice.

Route repair uses album/artist context and persistent playlist IDs; selection follows song ID and repeated playlist occurrence. Missing collections return to their parent and clear selection. Worker-prepared identity keys also scope scroll state, so inserting an earlier album or renaming/reordering playlists cannot attach another collection's scroll position. The retained sidebar/library rebuild on adoption, while ordinary frames continue borrowing prepared labels and orders.

All **85 app checks pass**, zero skips, in **8.454 seconds wall / 6.693 summed**. Regressions cover a concurrent WAL commit during a consistent read, rollback/nested-read refusal, stale labels, identity repair, duplicate selection, actual scroll retention, callback execution outside the mutex, bounded retirement backpressure/final worker ownership and zero warm handoff allocations. The production native controller test passes in **1.530 seconds**, including 100 refresh requests, metadata/Favorites updates, failed-read recovery, unchanged queue/transport and refreshes during current/upcoming preparation. Initial fixture failures used a rollback journal for the concurrent writer and the metadata upsert instead of the dedicated Favorites setter; corrected tests retain those repository contracts. The handoff check warms the same notification primitive that production poll has already used. Logs: `/tmp/localwave-rust-catalog-refresh-*`.

App Clippy with interop/warnings denied passes in **0.78 seconds**; formatting, whitespace and published-engine-pointer checks pass. No PXUI source or ABI changes were needed. Initial/final release library/app builds take **0.15/7.23** and **0.13/5.53 seconds**. Signed bundle minimum OS/default stack/dylib ID/relative rpath/entitlement checks pass; final smoke observes one frame/draw then **506.8 ms** idle.

Both ordinary signed timing runs **fail** the unchanged 1 ms maximum, with **zero warm Rust allocations** across 182 frames/draws: **3,652 µs** before the scroll-identity correction and **3,843 µs** afterward. Final wall p50/p95/p99: **300/793/2,310 µs**; CPU p50/p95/p99/max: **286/341/358/372 µs**. Earlier passes and failures remain recorded; CPU values do not substitute for the wall gate and the gap does not establish a cause. Full reference screens, automatic source observation, native memory/stable latency, macOS 15/Vulkan runtime, iOS/device/cloud and release qualification remain open. Swift/iOS were not rerun; their earlier failures remain recorded.

### Frame preemption evidence and worker notifications

A completed System Trace attached to an isolated signed app at `36d86db` captures 182 frame intervals and all 100 warm frames. Direct thread-state duration records explain its two failing frames. A fresh duration export also corrects the earlier context-switch adjacency analysis of the historical trace:

| Capture/frame | Build wall / CPU (µs) | Preempted interval time (µs) | Recorded preemptors |
| --- | --- | --- | --- |
| New / 84 | 1,939 / 381 | 1,561.750 | Kernel memory compressor |
| New / 99 | 1,397 / 341 | 1,058.959 | Instruments, WindowServer, kernel memory compressor |
| Historical / 137 | 1,785 / 354 | 1,452.084 | AppleCLPC, kernel memory compressor, audio thread |
| Historical / 138 | 1,253 / 312 | 944.542 | Kernel memory compressor, mds |

The earlier claim that the historical gaps were mostly Running is superseded. Analysis must use the first thread column as the owner; later columns can name a waker/preemptor. Intersect explicit thread-state durations with the paired signposts. Running, Interrupted and Preempted partition each reported interval within 0.02 µs. Intervals include clock/signpost overhead outside the measured build. The new two failing intervals contain no recorded main-thread memory faults and only two short thread-selfusage calls. This supports preemption attribution for these specific samples; it does not explain every uninstrumented failure, remove profiler overhead, or qualify native memory. Compact evidence, source paths and export hashes: [perf-rust-frame-correlation.json](perf-rust-frame-correlation.json). The completed trace is 170 MiB; its original export remains outside Git.

The same baseline records **99 library-worker Runnable transitions caused by the main thread** between frames 83 and 182. Unchanged polling previously notified the worker even though its wait predicate had no new work. Native inbox updates now request notification only for a delivered batch, changed snapshot or fault; native operation acknowledgements still notify, as do commands, stop and catalog retirement. The bounded queue refusal still pauses playback before returning an error. A regression checks 1,000 unchanged updates with no notification request or warm Rust allocation, changed facts, faults and overflow refusal. Existing real native/controller cases retain event delivery, preparation, acknowledgements, refresh and shutdown coverage.

Diagnostic frame timings now stay together in one fixed array. `--performance-check --trace-frames` prints failing frame IDs with their own CPU and seven phase wall durations after measurement, before sorting percentile copies. A boundary regression pins 999/1,000/1,500 µs and correlation; the shared wall limit remains exactly **1,000 µs**. Phase wall durations can include preemption and are not CPU attribution.

All **87 app checks pass**, zero skips, in **8.367 seconds wall / 8.284 summed**. App Clippy with interop/warnings denied passes in **1.51 seconds**; explicit formatting and whitespace pass. Release library/app builds take **0.22/7.76 seconds**. Signed bundle checks pass; smoke observes one frame/draw then **508.4 ms** idle. PXUI source, ABI and pin remain unchanged.

The ordinary signed run still **fails at 2,932 µs maximum**, with **zero warm Rust allocations** across 182 frames/draws (wall p50/p95/p99 **320/2,140/2,760 µs**, CPU p50/p95/p99/max **311/374/394/410 µs**). A separate diagnostic run without Instruments passes at **863 µs**, also with zero warm Rust allocations (wall **285/533/860**, CPU **277/334/367/425 µs**). These are different runs; the passing diagnostic does not replace the ordinary failure or establish an improvement in latency.

A follow-up System Trace never reported ready. Its isolated target and owned recorder were stopped; it provides **no after-change wake-count measurement**. An earlier attach attempt ran but exceeded its save deadline and left an unexportable trace; the successful capture was allowed to finish saving. Neither incomplete capture is passing evidence. Logs: `/tmp/localwave-rust-latency-*`. Native allocations, stable ordinary latency, full reference screens and remaining macOS/iOS/device/release qualification remain open. Swift/iOS were not rerun, and the existing ordinary GUI process was preserved.

## Find, column sorting and focused editor checkpoint

PXUI `116f822` is published on `rust-port`. The app now offers song-text Find and sortable Title,
Artist, Album and Time headings; `#` restores original order. String sorting uses Apple's
`localizedStandardCompare` on worker-owned immutable NSStrings, including numeric names such as
Track 2 before Track 10. Equal keys use ascending song IDs in either direction; invalid/missing
numeric durations remain last in both directions. Find takes distinct FTS membership, removes
orphan matches and preserves the original library order unless a column is selected. The existing
ranked repository search is unchanged. Full Find tokens/chips, matching albums/artists/folders,
other columns and the rest of P4 remain unfinished.

Prepared views share one immutable label arena and ID map through `Arc<Library>`. Each projection
owns only its order/context/count. A read transaction establishes its SQLite snapshot before reading
`data_version`; external edits rebuild the arena, while unchanged searches and sorts reuse it.
Same-connection edits still require explicit refresh. The existing bounded incoming/retirement
mailbox keeps large catalog drops on the worker. Selection follows song ID plus duplicate occurrence;
playback starts from the displayed order. New search queries hide stale rows, reset result scrolling
and retain text focus across asynchronous results. Query packets accept up to 1,024 UTF-8 bytes;
oversized edits are rejected visibly and the field returns to the accepted query.

Browse requests have a separate single coalesced inbox slot, so they neither fill the 32 ordinary
playback commands nor consume the reserved Pause/Stop slot. Worker revisions are monotonic even
when a reserved control follows a newer coalesced volume command. Preparation depends on its own
identity rather than every benign command revision. Native preparation/acknowledgements run before
catalog sorting. A gated native regression fills all ordinary slots, reserves Stop, replaces an
older volume update and sends 100 browse requests before releasing the worker; the latest result
arrives and playback preparation also survives continuous browse/seek/volume/refresh traffic.

The first focused-scroll check caught three Rust allocations per frame in the text editor. PXUI
now reuses its edit buffer, copies paint text/composition into retained frame-arena ranges and
borrows the unchanged caret prefix. A second regression with active selection caught cosmic-text's
allocating highlight iterator. Selection now streams the same shaped caret ranges used for inline
decorations. Focus still rebuilds for caret and IME output; it is not skipped to pass the counter.
Core, static and dynamic checks cover selection/composition, UTF-8 commit, resize and warm zero
Rust allocations. Core coverage also forces arena growth after capturing text ranges. Native 1x/2x
readbacks verify selection/composition changes pixels and survives the next frame at the same draw
count. Changed text/IME events and other editor widgets remain separate allocation paths.

Qualification for this checkpoint:

- All **94 application checks pass**, **15.728 s wall / 9.182 s summed cases**, including the held
  SQLite version snapshot, shared-arena sort/external-edit checks, focused 10,000-song Find scrolling,
  selection/Enter order and the bounded native controller. Clippy with warnings denied passes in
  **1.18 s**. The earlier route-scroll failure and first focused allocation failures remain in logs.
- The core/text/render matrix schedules **261 cases in 17.314 s**; its only failure is layout scaling
  at 140→269 ns/node while a compiler job was active. An isolated follow-up passes at 143→161 ns/node
  (**1.451 s wall / 1.026 s case**). This retains the failed run and does not establish stable timing.
  All **85 static wrapper/ABI/type cases** pass in **7.644 s**, and **45 dynamic wrapper cases** pass
  in **4.167 s**. The additional native focused-field pixel case passes in **2.741 s** including build
  (**0.132 s case**). The dependency gate passes; no public ABI/layout changes were made.
- Initial structural core check took **2.67 s**; the later core/text edit check took **1.72 s**, quiet
  check **0.02 s**, and a core-only edit check **1.49 s**. The required sub-second check after an edit
  is still open; a quiet check does not satisfy it. No profile, target directory, linker or stack
  workaround was introduced.
- Release library/app rebuilds take **23.44/17.69 s**. Bundle verification passes macOS 15 minimums,
  default stack, embedded `@rpath/libpxui.1.dylib` and sole app rpath
  `@executable_path/../Frameworks`; the signed smoke draws once then observes **508.8 ms idle**.

The signed performance runs both **fail the unchanged 1 ms wall maximum**; each draws all 182
frames and records **zero warm Rust allocations**:

| Workload | Wall max / p50 / p95 / p99 (µs) | CPU max / p50 / p95 / p99 (µs) |
| --- | --- | --- |
| Songs, original order | 2,185 / 345 / 1,133 / 1,847 | 603 / 335 / 528 / 595 |
| Find, Title descending | 2,365 / 350 / 1,659 / 2,204 | 480 / 337 / 425 / 470 |

These CPU numbers do not replace the wall gate or establish the cause of this run's slow frames.
Prior failed and passing runs remain retained. Logs: `/tmp/localwave-rust-browse-performance-songs.log`
and `/tmp/localwave-rust-browse-performance-find.log`.

`make performance` now runs both the original Songs view and a worker-prepared Find/title-descending
view over the isolated 10,000-song fixture, even if the first fails. Both keep 82 warmup and 100
measured frames and the same strict 1,000 µs maximum. Invalid standalone `--performance-find` exits with the expected requires-performance-check error
before opening a window or library (`/tmp/localwave-rust-browse-invalid-find-flag.log`). This workload measures warm result scrolling; typing,
sorting latency, scrubbing/menus, native allocations and device/distribution qualification remain
outside it. Swift/iOS checks were not rerun for this Rust checkpoint; their recorded limits remain.

Logs: `/tmp/localwave-rust-browse-{root-matrix,engine-matrix,engine-scaling-alone,static-abi,dynamic,
editor-native-pixels,clippy-final,bundle,bundle-check}.log`; focused editor allocation failures are
retained in `find-ui-compiled`, `editor-public-static` and `editor-regression-counted` logs with the
same prefix. The normal GUI process remains untouched.

## Find filters and live Unplayed results checkpoint

Find now parses the reference tokens: `cloud`, `offline`, `fav`/`favorites`, `unplayed`, `flac`, `mp3`,
`m4a`, `wav`, `aiff` and `year:<integer>`. Every token is conjunctive. Only normalized free text goes
to FTS; its membership still preserves original library order. Free terms also use Apple's localized
containment across title, artist, album and album artist, including case/diacritic matching. Every
`year:` word is excluded from FTS; an invalid integer still matches that literal word in metadata,
as in Swift. Blank Find displays no songs; the pure predicate itself remains unrestricted for a
blank query, matching the Swift helper contract.

The catalog now retains album artist, release year and whether an extension was present. Metadata
filters reuse that shared arena and cached playback-history snapshot. File locations are bulk-read
only for surviving rows that need availability or legacy format inference; source identities come
from the same read transaction. Resource probes run after the transaction ends, without opening an
audio stream or requesting downloads. Each source, including a failed lookup, is resolved at most once per request.
An explicit format precedes relative-path, local-path and bookmark inference; a present empty
extension/path stops fallback. Current resource metadata, rather than Copied state or persisted
sourceAvailability, decides Cloud/Offline. Source-relative containment and bookmark lifetimes stay
validated. Native NSString temporaries are confined to autorelease pools on the worker; this does
not qualify native process allocation or large provider-library probe latency.

The visible `flac`, `cloud`, `offline`, `year:2019` and `unplayed` chips wrap, expose checked toggle
semantics and preserve the user's remaining text/case. Removing a chip removes all case-insensitive
occurrences. Additions use the existing 1,024-byte UTF-8 packet; overflow preserves the accepted query
and shows the error. Warm activation/query updates allocate no Rust objects. An actual native start
now rebuilds an active Unplayed projection automatically. Entering Albums/Artists sends an Overview
packet that clears the old projection, ending that Find subscription.

Qualification:

- All **97 bounded application checks pass** in **10.881 s wall / 8.860 s summed cases**. The native
  controller case takes **1.741 s**, including a real muted start that removes its song from Unplayed
  without another browse request, exact first-start/history counts and subsequent queue/retirement
  checks. The new start changes the later exact history expectations from 3/4/5 to 4/5/6; the initial
  stale-fixture failure is retained rather than loosening those assertions. The preceding complete
  passing matrix took **16.021 s**; its results are also retained.
- The worker regression covers compound text/tokens, album-artist accents, saved history, real cache,
  source and legacy-bookmark metadata, extension precedence/empty values, missing/inaccessible files,
  invalid legacy relative paths and unknown availability. Metadata-only queries assert no file
  hydration. It passes in **0.278 s** after building. Its first fixture incorrectly used the guarded
  write API for invalid paths; the corrected fixture uses explicit legacy SQL rows while preserving
  production validation.
- The two focused Find UI checks pass in **4.616 s wall / 0.087 s cases**, including the existing
  focused 10,000-row scrolling check and new chip/overflow checks. Chips remain within bounds and
  disjoint at 960, 650 and 390 points; 390-point wrapping is not iOS qualification. Native localized
  containment is also checked with composed/decomposed café text. Native bridges remain deferred.

The signed filtered-Find workload now searches `Track m4a year:2019 unplayed`, sorts Title descending
and requires all 10,000 songs; the isolated seed sets their release year to 2019. Songs and Find keep
the same 82 warmup/100 measured frames and strict 1,000 µs maximum. Filter parsing, SQLite and file
work remain outside frame construction. Cold probe latency, real provider downloads/observation,
other Find result sections, full P4 goldens/shortcuts, native allocations and iOS/distribution gates
remain unfinished. Swift/iOS suites were not rerun for this Rust-only checkpoint.

Logs use `/tmp/localwave-rust-find-filters-`: `worker` (first fixture failure), `worker-fixed`,
`parser`, `ui-tests`, `live-history` (old count failure), `live-history-fixed`, `matrix` (old count
failure), `matrix-final`, `matrix-overview` and `clippy-overview`. The first parser run was **31.333 s**
including native/test bootstrap (**0.030 s case**); this is not a warm focused-loop timing claim.

### Keyboard-chip allocation correction (PXUI 1.10)

Keyboard activation then found **four Rust allocations per warm Space activation**. PXUI now
reuses signal accumulator and per-event modifier buffers at their observed peaks, and its existing
signal ABI encodes borrowed core events into retained chunks. The new Rust `signal_state` method
returns the same scalars without copying an unused owned key vector. The shell uses it throughout.
The existing `signal` API still delivers complete ordered key/text/IME events. ABI minor 1.10
advances for the public Rust addition; no C signature or wire layout changes.

PXUI `a459f01` is published before this app pin. The corrected app chip check passes at zero warm
Rust allocations (**4.513 s including build / 0.031 s case**), alternating accessibility activation
and actual Space press/release. Chips expose both checkable and checked states. Core plus native
render checks pass **235 cases in 13.419 s wall / 12.111 summed cases**. Static wrapper/ABI/type
checks pass **86 cases in 7.386 s / 2.634 s**; a subsequent static extension verifies UTF-8 text/IME
encoding (**5.353 s / 0.028 s**). The final dynamic wrapper suite passes **46 cases in 5.461 s /
1.584 s**, including that text/IME check. Dependency, C header, snapshot and dynamic-export gates
pass. The full 97-case app run above uses this corrected library. Prior 13.765- and 16.021-second
passing matrices remain recorded, together with the original four-allocation keyboard failure.

This qualifies numeric keyboard routing and state reads. It does not qualify all owned text/input,
legacy owned key-vector reads, native process allocations or stable wall latency. The prior
sub-second edited-core check requirement remains open; the initial multi-crate core/ABI/wrapper
check here took **2.87 s**, without profile, target-directory, linker or stack changes. Logs use
`/tmp/localwave-rust-find-filters-signal-` for `check`, `core`, `engine`, `static`, `static-text`,
`dynamic` and the library builds; `/tmp/localwave-rust-find-filters-keyboard{,-fixed}.log` retains the
failure and fix. Final app logs are `matrix-signal` and `clippy-signal` with the same base prefix.

### Final signed bundle and performance

The rebuilt PXUI library/app take **29.94/15.32 s**. `make performance` exits successfully: the
bundle verifier confirms signatures/entitlements, macOS 15 minimums, default stack, embedded
`@rpath/libpxui.1.dylib` and sole app rpath `@executable_path/../Frameworks`. Smoke records one
frame/draw then **506.7 ms idle**. Each performance workload draws all **182 frames** and records
**zero warm Rust allocations** across app plus libpxui:

| Workload | Wall max / p50 / p95 / p99 (µs) | CPU max / p50 / p95 / p99 (µs) |
| --- | --- | --- |
| Songs, original order | 162 / 143 / 155 / 160 | 160 / 143 / 155 / 159 |
| Find, format/year/Unplayed, Title descending | 371 / 140 / 155 / 318 | 276 / 141 / 154 / 266 |

Both pass the unchanged 1,000 µs wall maximum in this run. Earlier 2,185/2,365 µs failures and all
prior timing evidence remain recorded; this pass does not establish stable latency or prove the
cause of the difference. Native process allocation, cold provider probing, full P4 and platform
qualification remain open. Final app Clippy with warnings denied passes in **1.56 s**; formatting
and whitespace checks pass. No Swift/iOS suites or native process-memory profile were run.

Log: `/tmp/localwave-rust-find-filters-performance-final.log`. The earlier pre-keyboard-correction
bundle build remains at `/tmp/localwave-rust-find-filters-bundle.log` (**0.45/20.12 s**); it is not
the final runtime artifact. The normal GUI was preserved; all qualification processes and databases
are isolated.

A subsequent no-change release library/app build takes **0.14/0.05 s**, reusing the shared target
and linker configuration (`/tmp/localwave-rust-find-filters-warm-build.log`). This does not establish
a clean-build or edited-core budget.


## Up next panel and occurrence-based drag checkpoint

PXUI **bb219f5** is published on `rust-port` before the parent pin. The Now panel now displays
Up next with context/count, accessible play/remove controls, a drag grip and insertion line,
Clear and Undo. Each row uses its queue occurrence identity, including duplicate songs; the drag
label displays the track title. The worker resolves moves against the current queue, refuses
played/current/stale/no-op targets and reuses existing native prequeue reconciliation. Playing a
row selects its occurrence. Clear/Undo continue using the controller's persisted canonical queue.

`lw_ui::queue::Snapshot` holds one immutable entry array, cursor, context, worker-formatted labels
and Undo state. Structural commands copy the entries on the worker; cursor changes share the same
array. Queue and library projections use the existing one-incoming/one-retired catalog mailbox,
so final drops of large arrays happen on the worker. A queue-only catalog adoption leaves library
selection/navigation untouched. The nested queue view stays cached during clock ticks.

The initial 10,000-entry UI check caught 125 rows from an incorrect five-column initial estimate.
PXUI now invalidates enclosing caches when a grid estimate changes and propagates size reflow
through nested cached views within the existing two settling passes. The regression covers initial
layout and a parent-only resize. The playback notice keeps its existing fixed-height space so
appearing/disappearing notices do not change the queue viewport.

The new UI regression uses 10,000 duplicate-song occurrences, checks virtualization, stable row
identity after reorder, exact play/remove/Clear/Undo intents and cancellation by Escape, pointer
cancellation and focus loss. Repeated real pointer gestures with changing clocks assert **zero warm
Rust allocations across app and dylib**. A retirement test tracks the final owner of a 10,000-entry
array through the bounded worker slot. The native controller smoke now checks restored duplicate
occurrences, occurrence-based moves, shared cursor snapshots and Clear/Undo order while preserving
prepared AVQueuePlayer items.

Final bounded app checks pass **100 cases in 10.052 s wall / 9.083 s summed**; the native controller
case takes **2.246 s**. Clippy with warnings denied passes in **2.38 s**. Formatting and whitespace
checks pass. Logs: `/tmp/localwave-rust-queue-final-tests-v2.log` and
`/tmp/localwave-rust-queue-clippy.log`. An earlier passing 100-case matrix (12.538 s) remains at
`/tmp/localwave-rust-queue-final-tests.log`, before making the floating drag label show the title.

PXUI's final core/ABI/dynamic matrix passes **278 cases in 15.101 s / 10.470 s**, and the static
wrapper passes **48 cases in 2.538 s / 1.404 s**. The whole warm gesture and key replacement/
cancellation tests are allocation-free; a raw C regression retains UTF-8 pointers through
replacement, cancellation and 128 reads until the next frame. Owned payload APIs still copy. The
ABI minor is **1.11** with unchanged C signatures/layouts. Dependency/header/snapshot/export gates
and the published-submodule pointer check pass. Earlier scaling failures remain recorded in PXUI's
qualification notes; the final matrix passes, without claiming stable timing or a proven cause.

`make performance` adds a separate queue-scroll workload: 10,000 restored entries / 9,999 upcoming
rows, isolated defaults/database, paused and silent. Songs and filtered Find retain their original
workloads. Each still requires 82 warmup frames, 100 measured frames, zero warm Rust allocations
and a maximum build strictly below 1,000 microseconds. The initial run passes Songs at **180 µs**
and fails Find at **2,385 µs** (both zero warm allocations); Queue rejects startup before frames
because status and queue catalogs use separate bounded handoffs. Qualification startup now waits
for the queue projection as well as initialized status before checking the fixture. That initial
failure remains at `/tmp/localwave-rust-queue-performance.log`.

The final `make performance` run **passes all three workloads**. The no-change library build takes
**0.22 s**, the updated app **8.36 s**. Bundle verification confirms the embedded
`@rpath/libpxui.1.dylib`, sole app rpath `@executable_path/../Frameworks`, signatures/entitlements,
macOS 15 minimums and default stack. Smoke draws once and observes **508.1 ms idle**. Each workload
completes **182 frames/draws with zero warm Rust allocations**:

| Workload | Wall max / p50 / p95 / p99 (µs) | CPU max / p50 / p95 / p99 (µs) |
| --- | --- | --- |
| Songs, original | 186 / 139 / 162 / 170 | 166 / 140 / 156 / 162 |
| Find, filters and Title descending | 403 / 133 / 153 / 400 | 305 / 133 / 153 / 302 |
| Queue, 9,999 upcoming occurrences | 395 / 168 / 185 / 199 | 379 / 169 / 185 / 200 |

Log: `/tmp/localwave-rust-queue-performance-final.log`. Earlier 2,385 µs and other misses remain;
a pass does not establish stable latency or explain earlier failures. The initial changed PXUI/app
release builds took **22.24/8.95 s**; the final library build above reuses those artifacts.

This is a queue-panel checkpoint. Cross-page drags still cancel when their source row is
virtualized away (documented `shortcut:`); list-owned capture, the full queue sheet/Save as list,
artwork/waveform delivery and the rest of P4 remain. Native process memory, stable latency,
macOS 15/Vulkan, release/distribution and iOS/device/cloud gates remain open. Swift/iOS suites
were not rerun; their earlier source-observer failures remain recorded. The normal GUI is
preserved and all qualification apps/data are isolated.


## Cross-page queue dragging and clock benchmark correction

PXUI **6130090** is published on `rust-port` before this parent pin. The queue panel transfers
pointer capture to its stable scroll list through additive ABI **1.12** `transfer_carry`. Source
rows can now leave virtualization while their occurrence/title payload remains held. Both wheel
scrolling and pointer-edge scrolling work; the prior cross-page `shortcut:` is removed. Edge
movement is time-based, bounded to 50 ms per step, and retains subpixel travel in two UI-level f64
remainders rather than adding storage to every slot. Engine checks cover both axes and RTL, including
one-microsecond input steps at 100,000-point offsets in 100,000-cell virtual grids.

Unrelated or retired owners cannot take capture; key carries retain press-to-drop behavior. A
release or explicit cancellation never clicks the capture owner. Focus loss, pointer cancellation
and removing the owning container still end the carry; removal also removes the floating label in
that same frame. Captured press modifiers retain their values while their vectors reuse capacity.

The app's repeated 10,000-occurrence regression holds a duplicate-song occurrence through a
30-row scroll, verifies the original grip actually disappears, drops before the exact target, and
returns to the original page with clocks advancing. Warm gestures allocate zero Rust objects across
app and library. After virtualization the test reacquires its visible accessibility identity before
checking stability under reorder: retired node IDs are not a lifetime promise across slot reuse.

This stronger trace found a repeated floating-label allocation in `Ui::text`. A temporary allocator
backtrace isolated the path: retired slots retained nonempty strings which the existing empty-buffer
donor path could not reuse until those slots were reassigned. Retiring now clears text/name/value
contents while retaining their capacity. The warm assertion passes, and the temporary allocator
instrumentation is removed. Development failures and the backtrace remain in
`/tmp/localwave-rust-carry-owner-{ui,ui-diagnostic,allocation-trace}.log`.

### Qualification

Core/ABI/dynamic checks pass **281 cases in 8.870 s wall / 12.050 s summed**; the four-direction
100,000-cell edge/precision/allocation case takes **0.575 s**. The static wrapper passes **49 cases
in 2.168 s / 1.760 s**. Raw C validation includes null UI/box/output and retired handles. Dependency,
header, snapshot and dynamic-export gates pass. The initial C fixture's wrong expectation for an
unknown slot is corrected to the existing stale-handle contract; the production validation was not
weakened. Final focused app cross-page checks pass in **3.636 s / 0.046 s**.

The first complete app matrix passes **101 cases in 14.692 s / 9.420 s**; native controller smoke
is **1.955 s**. A small event-trace regression pins the exact pointer/Escape sequence, repeats it
and verifies retained event capacity. Clippy with warnings denied passes in **1.48 s**; formatting
and whitespace checks pass. Logs use `/tmp/localwave-rust-carry-owner-` with `engine-final`,
`static-matrix`, `abi-fixed`, `ui-final`, `app-tests`, `clippy` and `gate-*` suffixes.

### Corrected signed performance workload

Audit found the earlier signed harness changed `PlayerView.position` without advancing its revision.
The cached Now panel could therefore keep its clock during Songs/Find scrolling. Earlier recorded
passes remain evidence for their actual retained-panel workload; they do not fully qualify clock
rebuilds. The harness now advances the revision every performance frame. This corrects earlier
"changing clocks" claims for signed runs; the headless UI regressions already advanced revision.

`make performance` now runs four workloads even after a failure. The new native trace holds the
first upcoming occurrence through a 30-row wheel scroll, checks payload identity and capture, then
cancels with Escape before releasing and returning to the source page. Any application intent fails
the trace, so it does not edit the fixture queue. Qualification now requires a 1440x872 logical
window, keeping fixed event coordinates and the measured scene consistent. All four workloads
retain 82 warmup frames, 100 measured frames, zero warm Rust allocations and the unchanged maximum
strictly below **1,000 µs**.

The corrected `make performance` run **passes all four workloads**, each completing **182 frames/
draws with zero warm Rust allocations across app and dylib**:

| Workload | Wall max / p50 / p95 / p99 (µs) | CPU max / p50 / p95 / p99 (µs) |
| --- | --- | --- |
| Songs, original | 184 / 159 / 176 / 183 | 184 / 160 / 176 / 183 |
| Find, filters and Title descending | 419 / 167 / 198 / 365 | 398 / 168 / 181 / 367 |
| Queue, 9,999 upcoming occurrences | 545 / 190 / 213 / 491 | 472 / 190 / 211 / 433 |
| Queue carry, cross-page wheel scroll and cancel | 378 / 154 / 234 / 309 | 379 / 154 / 234 / 289 |

The changed release library/app builds take **21.28/10.99 s**. Bundle checks pass signatures,
entitlements, macOS 15 minimums, default stack, bundled dylib ID and sole portable rpath. Smoke draws
once then observes **507.3 ms idle**. Log: `/tmp/localwave-rust-carry-owner-performance.log`.
Previous timing failures remain; this pass does not establish stable latency or their cause. The
native carry workload qualifies wheel scrolling; pointer-edge motion has the separate engine
allocation/geometry checks above. Native process memory and broader platform qualification remain
open. The ordinary GUI is preserved; all qualification data/apps are isolated. No Swift/iOS or
physical-device suites were rerun. Queue sheet/Save as list and the remaining P4/P5/P6/P7 work are
still required for the full port.


A final app rerun after the window-size guard passes **101 cases in 11.254 s wall / 8.981 s
summed**, including native controller smoke in **3.351 s**. Final Clippy with warnings denied passes
in **1.30 s**. Formatting/whitespace and the exact published PXUI pin checks pass. The standalone
`--performance-queue-drag` flag returns its expected requires-performance-check error before any
window/library setup. Final logs: `/tmp/localwave-rust-carry-owner-app-final.log`,
`/tmp/localwave-rust-carry-owner-clippy-final.log` and `/tmp/localwave-rust-carry-owner-remote.log`.
## Full queue editor and macOS window chrome

PXUI **25b43a6** is published on `rust-port` before the parent pin. Its ABI remains 1.12:
the changes reuse internal storage without adding exports. Modal input filtering retains its
signal map in place, and fitted plain labels use the painter's existing string buffer instead of
formatting a new ellipsis string on every repaint. The legacy cut, tiny-width fallback, UTF-8
boundaries and ASCII caps match complete plain-label draw instances across the regression vectors.

The desktop Queue button opens a virtualized full queue sheet. Current and played occurrences can
be selected and reordered; moving the current occurrence preserves its physical native item.
The Now panel keeps its upcoming-only drag boundary. The sheet distinguishes the current occurrence,
prevents removing it, supports cross-page carries, and retains its header/editor/content between
changes. It also fits the 390x778 headless layout fixture; this is not iOS platform qualification.

Save as playlist uses the existing 1,024-byte UTF-8 packet, rejects blank/oversized names, and retains
the accepted name for retry. The worker creates the list and all membership in one transaction,
deduplicating songs in first-occurrence queue order, matching the current Swift action. Success
refreshes the catalog; failure does not stop playback or change the queue. Saving does not fence
preparation or discard Clear/Undo. The isolated native controller fixture checks those invariants,
failed saves and moving the active occurrence while paused.

The macOS host now enables SDL resizing explicitly. It configures AppKit full-size content behind
the transparent title bar, preserves the native traffic lights and full-screen eligibility, and
measures the title-bar clearance from `contentLayoutRect`. Chrome reads occur at startup/size changes;
frames use the cached inset. The tall Now panel collapses in short windows, and the native content
minimum is 640x480. The main-thread fixture checks 800x600, 1000x720 and 640x480 against both SDL
and actual native view bounds, zoom-button enablement, full-size/full-screen style, and stale-token
refusal after destroying its own hidden window. Actual full-screen/Spaces gestures remain unqualified.
The normal GUI process **69331** was left running; quit/reopen to load the new executable.

Verification:

- PXUI core/ABI/dynamic: **283 cases**, **8.480 s wall / 11.632 s summed** on the final recheck.
  Static wrapper: **49 cases**, **4.490 / 1.658 s**. The deps gate and diff checks passed.
  An earlier matrix failed `layout_scaling` at 139 vs 212 ns/node (the unchanged ratio limit is 1.5).
  The failure is retained at `/tmp/localwave-rust-modal-child-final.log`; the successful recheck is
  `/tmp/localwave-rust-modal-child-recheck.log`. A recheck does not prove stable latency.
- App/core/native/UI: **107 cases**, **14.040 s wall / 10.081 s summed**, exit 0; the controller case
  took **2.385 s**. Log: `/tmp/localwave-rust-full-queue-window-verified.log`.
  Final Clippy with `-D warnings` passed in **2.24 s**; formatting/diff checks passed.
- The full queue regression covers 10,000 duplicate occurrences, one-column virtualization,
  current protection, cross-page drag identity, Create/Cancel/name validation, 390x778 resize,
  and zero warm app+dylib Rust allocations while scrolling with live clocks and the editor open.
  It caught the initially missing semantic role on the current indicator; the indicator is now
  an authored Image. Native accessibility bridges remain a separate deferred requirement.

`make performance` now also runs the full queue sheet with the playlist editor open, after the four
existing workloads, even if an earlier case fails. The carry coordinates include measured native
title-bar clearance, and their bounded trace test covers multiple inset values. Every measured
frame still advances the clock revision and the 1,000 µs wall maximum remains unchanged.

The first signed run passed packaging/signature/rpath/default-stack checks and **502.9 ms** idle,
with zero warm Rust allocations in all five workloads, but **failed the time gate in every workload**:

| Workload | Wall max / p50 / p95 / p99 µs | CPU max / p50 / p95 / p99 µs |
| --- | --- | --- |
| Songs | 2893 / 430 / 1540 / 2458 | 542 / 409 / 495 / 542 |
| Find | 2332 / 362 / 951 / 1720 | 490 / 360 / 459 / 484 |
| Queue | 2600 / 475 / 1811 / 2302 | 605 / 433 / 565 / 596 |
| Carry / wheel / cancel | 1952 / 343 / 1299 / 1821 | 612 / 330 / 453 / 499 |
| Full queue / playlist editor | 2963 / 642 / 1510 / 2333 | 770 / 526 / 700 / 744 |

Log: `/tmp/localwave-rust-full-queue-window-performance.log`. A Songs trace also failed with a
**2437 µs** wall maximum and **620 µs** CPU maximum; medians by phase were 15/129/12/39/175/0/9 µs
(begin/build/intrinsic/arrange/paint/a11y/finish). Log:
`/tmp/localwave-rust-full-queue-window-trace.log`. These measurements do not identify the cause of
the wall/CPU gap. Further runs and intermediate failing current-indicator checks are retained in
the `/tmp/localwave-rust-full-queue-*` logs. No test threshold was relaxed.

A rebuilt source (with the retained sheet body and corrected current-indicator role) is
recorded at `/tmp/localwave-rust-full-queue-window-performance-verified.log`. It passed signature,
rpath, default-stack and **500.1 ms** idle checks. Every workload again measured zero warm Rust
allocations across 182 frames/draws, but the command exited nonzero: four time gates still failed.

| Final workload | Wall max / p50 / p95 / p99 µs | CPU max / p50 / p95 / p99 µs |
| --- | --- | --- |
| Songs | 2621 / 393 / 1344 / 2047 | 621 / 385 / 512 / 536 |
| Find | 1896 / 340 / 1570 / 1855 | 510 / 340 / 452 / 503 |
| Queue | 2155 / 498 / 1821 / 2150 | 629 / 444 / 558 / 590 |
| Carry / wheel / cancel | 218 / 138 / 192 / 214 | 219 / 138 / 193 / 215 |
| Full queue / playlist editor | 3229 / 509 / 2019 / 3188 | 796 / 460 / 698 / 778 |

Unchanged library/rebuilt app release build times in that run were **0.69 / 8.63 s**. The one passing
carry workload does not resolve the other misses or prove native allocator behavior.

The last run after widening the count for the maximum queue size is
`/tmp/localwave-rust-full-queue-window-release-final.log`: **0.18 / 9.60 s** library/app builds,
valid bundle checks, **508.4 ms** idle, and zero warm Rust allocations in all five workloads.
Four time gates passed; Songs still failed, so `make performance` correctly returned nonzero.

| Last workload | Wall max / p50 / p95 / p99 µs | CPU max / p50 / p95 / p99 µs |
| --- | --- | --- |
| Songs | 1935 / 172 / 429 / 1085 | 428 / 171 / 380 / 424 |
| Find | 192 / 159 / 172 / 183 | 176 / 159 / 172 / 173 |
| Queue | 443 / 186 / 210 / 366 | 398 / 185 / 208 / 240 |
| Carry / wheel / cancel | 584 / 145 / 205 / 257 | 351 / 146 / 205 / 257 |
| Full queue / playlist editor | 292 / 204 / 263 / 284 | 285 / 205 / 262 / 273 |

The last complete app matrix after the count-width change and explicit Cancel-button check passed
**107 cases in 12.006 s wall / 8.755 s summed**, exit 0; controller **2.346 s**. Log:
`/tmp/localwave-rust-full-queue-window-commit-tests.log`. The final 20 UI cases also passed separately
in **5.425 / 0.891 s**. The normal process 69331 remained alive throughout qualification.

This checkpoint does not qualify the strict latency gate, native allocator behavior, artwork
delivery, physical-device/provider behavior, iOS, distribution signing or the complete reference UI.


## Artwork delivery checkpoint (2026-11-13)

Existing catalog cover paths now reach the Album grid, Now panel and both queue views. Import
already publishes managed covers; this change reads those files and legacy absolute cover paths.
Missing/unreadable files retain the placeholder until Refresh retries them. No network lookup or
cover extraction during a frame was added. The previously pushed native resizing, green zoom/full-screen
eligibility and transparent full-size title bar remain in place; an already running binary must be
quit before opening the rebuilt bundle.

ImageIO decodes bounded, orientation-correct thumbnails on a separate persistent worker. It checks
local availability, coordinates the file read, limits encoded files to 64 MiB, validates size/mtime
before publication, and produces sRGB straight-alpha RGBA. Relative managed paths reject parent and
symlink escapes. The worker has eight coalesced keys, one incoming result, one retained result and
one retirement slot; old pixel buffers are released on that worker. Playback uses its existing worker.

The catalog deduplicates cover strings once and exposes catalog-scoped numeric IDs. UI lookups use
a preallocated ID map; the texture cache holds at most 1,024 entries and 128 MiB of resident RGBA texels.
Thumbnail requests follow density and a conservative visible-grid budget, capped at 512 pixels.
Size/density changes retire the old working set and advance an epoch; stale replies are ignored.
Missing results are negatively cached. GPU textures remain owned until their last submitted frame
has completed. Retained view revisions prevent reusing descriptors after eviction. The cache size
bounds logical texture payload, not total Metal/native memory or in-flight retired payloads.

Checks so far:

- Native fixtures verify exact color rows, transparency/unpremultiplication, EXIF orientation,
  downsampling, malformed data and encoded-size limits. Worker checks cover managed/legacy paths,
  escape rejection, coalescing, retirement, unchanged polls and cancellation/restart.
- The hidden real-window check decodes on the worker, uploads to Metal, reads exact pixels back,
  asserts actual album/player image draws and zero warm app+dylib Rust allocations, then exercises
  retirement through actual completed GPU fences. An unknown-duration playback accessibility bug
  exposed by that test is fixed: the clock remains valid and seeking is disabled until length is known.
- The full matrix passed **117 cases in 13.743 s wall / 11.683 s summed**, exit 0. The native image
  case took **0.662 s** in that run. Earlier complete 116-case run: **19.576 / 12.314 s**.
  Logs: `/tmp/localwave-rust-artwork-{final-tests,app-tests-v2}.log`.
- Clippy with `-D warnings` passed in **13.08 s** before the final diagnostic assertions.
  Source formatting passed. No pxui source or ABI change was needed.

The first six-workload signed run passed bundle signature, macOS minimum, default stack, dylib ID,
rpath and **500.4 ms** idle checks. The first five workloads allocated zero warm Rust objects but
all missed the unchanged 1,000 µs wall maximum (2,600 / 2,688 / 2,512 / 5,720 / 3,412 µs).
The new gallery workload **failed**, with **75** Rust allocations maximum, **2,434 µs** wall maximum
and **30,691,328 resident texture bytes**. Log: `/tmp/localwave-rust-artwork-performance.log`.

The explicit gallery diagnostic also fails: **18 minimum image quads**, **4 pending image requests
maximum** during counted frames, **75 allocations**, **392 µs wall maximum**, CPU maximum **393 µs**.
It is drawing real images but still reaches first-use covers during measurement; the benchmark now
rejects that condition explicitly. Log: `/tmp/localwave-rust-artwork-gallery-trace.log`. This run
was concurrent with the bounded test matrix and is allocation/loading evidence, not isolated latency
qualification. Warm single-working-set rendering passes; the scrolling gallery allocation gate,
strict stable latency, native memory, real provider/device behavior and the full reference UI remain
open. No threshold was relaxed. The normal process 69331 and its database/defaults were untouched.

Final checkpoint recheck: **117 cases, 12.546 s wall / 10.306 s summed**, exit 0; native artwork
case **0.602 s**. The actor regression now also verifies all eight results arrive and a cancelled
identical request can restart even when coalescing overwrites the cancellation. Clippy with
`-D warnings` passed in **1.75 s**. Logs: `/tmp/localwave-rust-artwork-commit-{tests,clippy}.log`.
Final release rebuild: **0.61 / 9.77 s** library/app; signature checks passed. The rebuilt bundle
again verifies macOS minimum, default stack, embedded dylib/rpath and **502.7 ms** idle with one
frame/draw. Logs: `/tmp/localwave-rust-artwork-commit-bundle{,-check}.log`.


## Shared pxui image cache and loading checkpoint (2026-11-13)

PXUI **9e5d072** is published on `rust-port` before the parent pin. ABI **1.13** adds
`Renderer::forget_image(image, completed)` and its C export. The engine interns full native image
identities in a preallocated O(1) lookup cache, bounded to 1,024 entries and 128 MiB estimated texels
(one oversized image can stand alone). GPU wrappers survive gallery revisits while their image
identity remains live. Retained draw segments without IMAGE instances do not import or bind stale
image descriptors. Actual completed serials fence wrapper release; the host releases the source
texture afterwards. Every sampling renderer must forget the image before native handle destruction
or reuse. No extra GPU frame is needed to report completion. These fixes are shared by pxui apps.
The bounds describe logical texels, not total native memory.

LocalWave keeps music-specific file lookup and decoding on its artwork worker. Up to 1,024 pending
node candidates survive retained frames; after final layout, only visible/nearby nodes enter the
eight-key wire packet, with two image heights of vertical prefetch. Retired nodes are pruned.
Gallery/Now/queue requests use their actual 160/184/30-point display sizes and density, capped at
512 pixels. Empty packets genuinely cancel obsolete requests. GPU retirement now scans actual
pending textures instead of 3,072 empty slots. ImageIO rejects source axes above 32,768 or products
above 64 Mi pixels before thumbnail creation, alongside the existing 64 MiB encoded-file bound.

Verification on this source:

- **119 app checks pass in 28.777 s wall / 13.045 s summed**, zero skips. Slowest case 2.772 s;
  the native decode/upload/pixel/warm-allocation/fence case takes 1.697 s.
  Log: `/tmp/localwave-rust-image-release-app-tests.log`.
- **328 pxui checks pass in 17.200 s wall / 15.804 s summed**, zero skips; **48 dynamic checks
  pass in 5.292 / 1.798 s**. A real 128-image gallery test repeatedly revisits four pages, reads
  every cell's exact pixels, asserts zero warm Rust build/draw allocations, rejects incomplete
  release, and releases completed images without another command buffer.
  Logs: `/tmp/localwave-rust-image-release-{child-tests,dynamic-tests}.log`.
- Dependency, C header, ABI snapshot and exported-symbol gates pass. App Clippy with `-D warnings`
  passes in 11.01 s. Optional child Clippy remains blocked by existing warnings in unchanged code;
  broad/scoped failure logs are retained. No unrelated lint cleanup was added.
- The isolated prefetch gallery run now records **zero warm Rust allocations, zero pending requests,
  at least 18 actual image quads**, and **3,756,032 resident texture bytes**, down from the earlier
  30,691,328-byte trace. All 42 required covers arrive before warm-up ends. Wall max/p50/p95/p99
  **3,398/455/1,795/2,484 µs** and CPU **647/435/521/616 µs** still fail the unchanged 1,000 µs
  wall maximum. Log: `/tmp/localwave-rust-image-prefetch-performance.log`. Earlier 75-allocation
  and late-request failures remain recorded above. This is Rust allocation evidence, not a claim
  about native allocations or stable latency.

The full reference UI, native memory, Vulkan image-path qualification, macOS 15 runtime and iOS/
provider/device/distribution gates remain open. The normal running app and its library/defaults
were left untouched. Repeated broad verification during this cache change took too long; use
focused checks while editing and one complete matrix per checkpoint.

### Final signed bundle and retained failure

The final release library/app rebuild takes **19.80/18.19 s**. Bundle signatures, macOS 15 minimum,
default executable stack, embedded dylib ID/rpath and smoke pass; smoke draws once and remains idle
**504.7 ms**. The unchanged six-workload gate returns nonzero. All workloads complete 182 frames:

| Workload | Warm Rust alloc max | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- | --- |
| Songs | 0 | 2556/440/1541/2261 | 603/397/483/549 |
| Find | 0 | 2207/412/1384/1879 | 475/384/438/456 |
| Queue | 0 | 1842/434/1273/1776 | 515/409/483/504 |
| Queue drag | 0 | 2203/377/1597/2163 | 566/341/479/548 |
| Queue/playlist sheet | 0 | 4039/639/2505/3020 | 834/481/718/785 |
| Gallery | 12 | 2568/474/1598/2204 | 583/430/553/582 |

Gallery still draws at least 18 image quads and uses 3,756,032 resident texture bytes, but records
one pending request after warm-up. The earlier zero-allocation/zero-request gallery run does not
qualify reliably completed loading. This remains an open application loading/performance defect;
shared warmed wrapper reuse passes its focused native regression. All six workloads also miss the
strict 1,000 µs wall maximum. No warm-up duration, allocation gate or timing threshold was relaxed,
and no rerun was made to replace this failure. Log: `/tmp/localwave-rust-image-release-performance.log`.


## Remove the one-cover-per-frame handoff (2026-11-13)

The late-cover failure above was left open too early. The worker could publish only one completed
thumbnail and had to wait for UI adoption before decoding the next. This serialized a grid of covers
through repeated frame handoffs even when ImageIO could decode faster. The pre-change trace finished
its uploads at frame **63**; the preceding complete run crossed frame 82 and recorded one late request
and 12 allocations. Both results remain retained.

The same worker now decodes ahead into a preallocated queue of at most **eight** results and
**8 MiB** decoded RGBA. A UI poll drains a fixed batch and the GPU host uploads every new result in
order. Already queued keys are skipped when requests coalesce. Monotonic revision rejection prevents
replaying an older entry of the retained batch. Replaced pixel buffers move in fixed retirement
batches and are released on the worker outside the mutex; retirement re-wakes a consumer when another
batch is waiting. Request coalescing, stale-epoch rejection, negative caching, public pxui image
caching and actual completed-GPU serial release remain in use. No decoder threads or dependencies
were added. Incoming/current/retiring batches each hold at most 8 MiB of raw pixels; one in-progress
thumbnail can add at most 4 MiB. These are payload bounds, not total native memory limits.

Evidence for the final source:

- A deterministic regression waits for **all eight requested covers to finish before any UI poll**,
  then asserts one allocation-free batch adoption, bounded payload, replacement and off-thread
  retirement. The old one-result pipeline cannot satisfy that condition.
- The real Metal check requests eight covers, waits for all eight completions before polling, uploads
  the batch, reads every texture's exact pixels, verifies allocation-free replay/build/draw, rejects
  stale batch revival after invalidation, and checks actual completed-fence retirement.
- **120 app checks pass in 11.050 s wall / 11.041 s summed**, zero skips. The native artwork case
  takes **0.601 s**; controller **2.147 s**. Focused four-case artwork run: **5.821 / 1.595 s**
  including compilation. Full suite was run once for this checkpoint.
  Logs: `/tmp/localwave-artwork-batch-{all-tests,integration-fixed}.log`.
- App Clippy with `-D warnings` passes in **10.32 s**. Formatting and diff whitespace checks pass.
  No pxui source changed, so its ABI/dependency suites were not repeated.
- Release library/app builds take **0.28/7.61 s**. Signatures, macOS minimum, default stack,
  bundled dylib ID/rpath and smoke pass; one draw then **500.7 ms** idle.
  Logs: `/tmp/localwave-artwork-batch-{clippy,bundle,bundle-check}.log`.

Three predetermined, sequential signed gallery runs use the same **182 frames**, **82 warm-up
frames**, **100 measured frames**, actual images and unchanged **1,000 µs** wall limit:

| Run | Last cover upload frame | Warm requests / Rust allocations | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- | --- | --- |
| 1 | 21 | 0 / 0 | 2227/448/1527/1916 | 566/437/541/566 |
| 2 | 25 | 0 / 0 | 2022/505/1537/1833 | 637/444/591/629 |
| 3 | 21 | 0 / 0 | 2727/471/1270/2103 | 627/431/553/574 |

All three retain at least **18 actual image quads** and **3,756,032 resident texture bytes**.
All pass the artwork working-set/allocation conditions and all still return exit 1 for the strict
wall-time gate. No retries were used to replace a failed result, and no threshold, warm-up duration
or workload was weakened. Logs: `/tmp/localwave-artwork-batch-gallery-{1,2,3}.log`; pre-change trace:
`/tmp/localwave-artwork-late-reproduction.log`. This verifies the bounded delivery fix for the tested
local gallery. Cold decode/upload still allocates. Provider/device behavior, native memory, stable
latency and the full port remain unfinished. Normal GUI PID 69331 and its library/defaults were
left untouched.


## Transport waveform and shared fractional fills (2026-11-13)

The Mac transport now displays the existing 512-peak waveform cache/native PCM service. The library
worker prioritizes playback/control work, loads only the current song, yields decoding for controls,
browse or critical native events, and uses a separate cancellation flag from file preparation.
Cancellation reset is gated under the inbox mutex so queued controls cannot be lost. Status carries
the array inline: UI delivery, clone and repaint need no new heap-owned waveform object. Selection
and catalog changes clear stale peaks; a token change revalidates after materialization. A refresh
publishes metadata invalidation before loading replacement peaks, including while restored paused.
Decode/cache errors leave the seekable baseline and preserve playback.

The transport max-pools into the reference's **200 bars**, with quarter-step width, 42% ink,
bottom baseline/progress and signal cursor. Pointer hold previews, primary-button release commits;
Left/Right adjust five seconds, Home/End seek to bounds, and accessibility retains finite duration
and five-second adjustment. Missing/unknown duration remains a progress role with seeking disabled.
Waveform reveal/stagger and the broader narrow/iOS interface remain unfinished.

PXUI **d611cee** is published on `rust-port` before the parent pin. ABI **1.14** adds
`Paint::fill_fractional` through the existing public span writer. The shared helper preserves finite,
increasing bounds. RECT flag AREA uses matching Metal/GLSL pixel-area coverage, with derivatives
calculated before discard, so a half-pixel bar retains half a pixel's weight at 1x/2x. SPIR-V is
rebuilt and validated. The ordinary pixel-snapped fill uses its existing mode. No C layout changes,
extra draw calls, dependencies or per-frame threads were added.

The first signed run found **one warm allocation in queue dragging at frame 84**. Optional fixed
trace samples identify host/build/after-build counts, four GPU recording stages and output getter
counts without hot-loop printing. On the failing frame, build/GPU stages were zero and `Ui::output`
allocated once. Its ABI dirty-span buffer had grown for a more fragmented update although the
engine's buffer already had capacity. It now reserves from those existing capacities. The direct
regression fails before and passes after the fix, returning all sixteen disjoint ranges. This fix
lives in pxui for every consumer. Earlier failures remain in
`/tmp/localwave-waveform-performance.log` and `/tmp/localwave-waveform-queue-{allocation,gpu,output}-trace.log`.

### Checks

- Final **123 app checks pass in 15.080 s wall / 12.756 s summed**, zero skips. Controller **2.347 s**;
  real artwork/Metal/combined-waveform case **1.366 s**. Existing numeric SDL input, queue drag and
  theme allocation cases now include actual waveform bars. Log: `/tmp/localwave-waveform-final-app-tests.log`.
- The waveform widget check verifies 200 bar extents, max-pooling, clamped held preview, release-only
  seek, keyboard focus and zero warm app+dylib Rust allocations. The controller check verifies paused
  native decode/cache delivery, replacement after catalog refresh and clearing on an empty queue;
  existing playback/history/prequeue/lease checks remain. A pure check verifies cancellation gating
  and 1,000 allocation-free inline-snapshot clones. The event trace check asserts its full 120-frame
  press/move/release/wheel sequence without allocations.
- **335 pxui static/core/ABI/renderer checks pass in 18.577 / 16.488 s**, and **50 dynamic checks
  in 5.472 / 1.859 s**, zero skips. The native 200-thin-bar case reads every bar at both densities,
  checks total covered weight, one instanced draw and zero warmed Rust build/draw allocations.
  Logs: `/tmp/localwave-waveform-pxui-final-tests.log`, `/tmp/localwave-waveform-verified-dynamic-tests.log`.
- Dependency, C-header, ABI snapshot, dynamic-export and shader validation gates pass. The initially
  failed layout-scaling check is retained; its existing 1.5x ratio is now measured in the runner's
  serial group. Its classification check passes; no timeout or numeric limit changed. Optional
  broad child Clippy remains unqualified due to the existing warnings recorded above.

### Final signed performance run

Library/app release builds take **23.52/10.06 s**. Bundle signature, macOS minimum, default stack,
embedded dylib ID/rpath and smoke checks pass; smoke draws once and then remains idle **507.6 ms**.
All seven workloads load a valid cached **512-peak fixture with its audio copy absent** through the
worker, exercise **182 frames** with the existing **82 warm-up / 100 measured** split, and retain at
least **200 actual waveform bars**. The dedicated scrub case asserts held capture and **nine actual
release commits through the UI/controller handoff**; real audio seek is qualified separately by the
native controller case. The original scroll and queue traces remain in the suite.

| Workload | Warm Rust alloc max | Wall max/p50/p95/p99 microseconds | CPU max/p50/p95/p99 microseconds |
| --- | --- | --- | --- |
| songs-original | 0 | 2195/460/1654/2026 | 630/421/515/551 |
| find-filters-title-descending | 0 | 2365/449/1382/2157 | 532/406/494/529 |
| queue-occurrences | 0 | 2612/494/1839/2469 | 587/463/554/571 |
| queue-carry-scroll-cancel | 0 | 2255/393/1588/1870 | 631/344/528/575 |
| full-queue-playlist-editor | 0 | 2721/432/2129/2232 | 812/426/709/773 |
| albums-artwork | 0 | 2676/471/1894/2666 | 636/448/561/621 |
| waveform-scrub-and-scroll | 0 | 2363/401/1467/2330 | 585/373/466/535 |

The gallery retains **18 minimum actual image quads**, **3,756,032 texture bytes**, and **zero late
requests**. Every workload has **zero warm Rust allocations**, including the previously failing
queue drag. All seven still miss the unchanged **1,000 microsecond wall maximum**, so `make performance`
correctly returns nonzero. The final run is retained as recorded; warm-up duration, allocation checks and timing limits stayed strict. Log: `/tmp/localwave-waveform-fixed-performance.log`.

Stable wall latency, native process memory, waveform reveal, full reference goldens/screens,
macOS 15/Vulkan runtime and iOS/provider/device/distribution qualification remain open. Swift
suites were not repeated for this Rust-only checkpoint. Normal GUI PID 69331 and its library/defaults
were left untouched. The overall port remains in progress.

Final app Clippy with `-D warnings` passes in **2.50 s**; formatting and whitespace checks pass.
`--performance-waveform` without `--performance-check` returns its expected argument error before
opening a window. Logs: `/tmp/localwave-waveform-final-clippy.log`,
`/tmp/localwave-waveform-invalid-flag.log`.


## macOS Now Playing and bounded remote controls (2026-11-13)

The normal GUI now registers MediaPlayer Play, Pause, Toggle, Stop, Next, Previous and Seek handlers.
Callbacks can run away from the main thread: they only validate and append to a fixed 32-event
native queue, then wake the host. They return failure on overflow or an invalid event and report no
actionable item for unavailable controls. The main thread peeks without consuming; a busy worker
leaves that accepted head in place until the existing controller mailbox accepts it. Native callbacks
never call the player or touch the library. Teardown closes the queue, removes each exact target,
disables its command and clears Now Playing; already captured callbacks safely reject a dead owner.

Toggle is resolved against transport intent on the serialized worker, before pending-selection
reconciliation. The real controller fixture sends two rapid toggles from restored pause and checks
that they return to pause without starting a physical item. Previous playback/history/prequeue,
stop priority, waveform and cache-lease assertions remain.

Publication runs in controller polling outside UI drawing. Length-delimited UTF-8 metadata retains
embedded NUL and emoji. Title/artist/album, duration, elapsed time, rate and explicit macOS
playbackState are published for actual playback state; Stop/failure/empty state clears the item.
Ordinary ticks use system extrapolation with 250 ms drift correction, and small paused seeks publish.
The clock policy is checked with injected `Instant` values, including duration clamping and track,
metadata and pause transitions. Retained metadata Arcs require no warm Rust allocation. See Apple's
[playbackState contract](https://developer.apple.com/documentation/mediaplayer/mpnowplayinginfocenter/playbackstate)
and [elapsed time contract](https://developer.apple.com/documentation/mediaplayer/mpnowplayinginfocenter).

Now Playing covers share the existing artwork worker and its eight-result RGBA batch. No file read,
decode, new worker or separate GPU texture is added to media publication. The native center retains one sRGB
image, copied only when a new matching result arrives. Identity/catalog epochs clear old covers;
stale paths, epochs and already-consumed revisions cannot replace them. A pending current cover
occupies the first slot of the existing eight-key request so busy gallery requests cannot starve it.
Metadata/clock changes preserve the existing native artwork. Cold native image and dictionary
creation still allocates outside the Rust counters.

Smoke, performance and controller fixtures use the same local queue and metadata sink with system
publication/registration disabled. Tests use isolated native center/command doubles; they never send
media keys to another application or replace normal GUI PID 69331's session/defaults/library. Real
physical keys, Control Center eligibility and appearance, native memory, interruption handling,
macOS 15 runtime and iOS audio/session/device qualification remain open. This is implementation and
isolated integration evidence, not completion of P2 or the overall port.

### Checks

- First complete matrix: **128 pass / 17.461 s wall / 12.735 s summed**, zero skips.
  `/tmp/localwave-media-app-tests.log`.
- Final implementation matrix: **128 pass / 25.684 s wall / 13.855 s summed**, zero skips. The
  runner automatically retried four existing audio/collation cases after contention; those attempts
  remain in the log. Controller **3.470 s**, artwork/Metal **1.422 s**. The five new cases cover
  off-main construction rejection, deterministic clock policy, command mapping, native callback/
  metadata/artwork/teardown and real worker-mailbox backpressure. `/tmp/localwave-media-final-app-tests.log`.
- Native callback checks use concurrent background producers, all seven handler blocks, repeatable
  peek/ack, full-queue rejection, invalid seek/metadata, UTF-8 and exact RGBA bytes. Main-thread bridge
  checks preserve native artwork across clock changes, reject old epochs and count **zero warm Rust
  allocations** during 1,000 unchanged polls and playing polls.
- Clippy with `-D warnings` passes in **12.66 s**. Existing pxui native SDK-availability/linker warnings
  remain. Formatting and whitespace pass. `/tmp/localwave-media-clippy.log`.
- `make performance` release library/app builds **0.24 / 19.03 s**. Signing, sandbox entitlements,
  macOS minimum load commands, default executable stack and portable embedded dylib/rpath pass.
  The signed smoke draws once then remains idle **506.3 ms**. `/tmp/localwave-media-performance.log`.

### Signed performance with the isolated media bridge enabled

All seven unchanged workloads draw **182 frames**, load **512 waveform peaks** and retain at least
**200 bars**. All have **zero warm Rust allocations**. Every strict **1,000 microsecond wall maximum
still fails**, so `make performance` exits nonzero. No warm-up, workload or threshold was weakened.

| Workload | Warm Rust allocations | Wall max/p50/p95/p99 (µs) | CPU max/p50/p95/p99 (µs) |
| --- | ---: | --- | --- |
| songs-original | 0 | 2023/430/1486/1946 | 538/408/490/517 |
| find-filters-title-descending | 0 | 2567/413/1029/2094 | 625/394/460/500 |
| queue-occurrences | 0 | 3695/483/1585/3075 | 662/442/573/628 |
| queue-carry-scroll-cancel | 0 | 1357/360/862/1189 | 636/339/483/575 |
| full-queue-playlist-editor | 0 | 1683/539/1110/1492 | 801/480/724/739 |
| albums-artwork | 0 | 2312/517/1342/2121 | 678/474/603/641 |
| waveform-scrub-and-scroll | 0 | 2005/403/1321/1508 | 518/377/467/497 |

Gallery still has **18 minimum actual image quads**, **3,756,032 texture bytes** and **zero late
requests**. The scrub workload commits **nine** completed seeks. The full port, stable strict
latency, native memory, full Mac UI, real media-key delivery and iOS/device/cloud/distribution gates
remain unfinished. Swift/platform suites were not repeated for this Rust-only checkpoint.

Final host review moves media cover adoption after artwork density/cache configuration, before the
view build and request packet, so a resize uses the newly established epoch. The final signed run
above includes that ordering and retains all seven timing failures. The first matrix remains in
`/tmp/localwave-media-performance.log`; the final matrix is
`/tmp/localwave-media-final-performance.log`. Final library/app builds are **0.58 / 8.65 s**, smoke
idle **501.6 ms**. Different timing samples do not establish a latency improvement or its cause.
Final Clippy passes **1.63 s**; the final focused native media check passes **5.468 s wall / 0.077 s
case**, including its rebuild. Logs: `/tmp/localwave-media-final-clippy.log`,
`/tmp/localwave-media-final-focused.log`. Playing clock policy uses injected times in the pure check;
the native warm loop has no wall-clock-sensitive count assertion. The existing GUI remains alive.


## Guarded playback file failures and recovery (2026-11-13)

Unreadable/missing files previously advanced the queue without saving their failure state. The
library service now records eligible preparation errors, and the controller accepts native item
failures only when token, generation and occurrence match one of its six retained playback leases.
Each lease retains the Song snapshot associated with the prepared physical file on the worker.
The repository changes only `fileState`, using one UPDATE guarded by ID, song key, path hash,
created/updated dates, previous state, local path, source ID, relative path, bookmark bytes, size and
source availability. A newer cache, import/access record, deleted row or replayed event cannot be
overwritten. Paths and bytes, metadata, favorites, playlist membership and FTS remain intact.

An already-local Failed file retains that state through successful preparation. Its actual Started
event records history once and attempts guarded recovery to Copied; the lease advances its expected
state after our own successful writes so a recovered physical item can fail again. Ordinary verified
materialization retains its existing Copied behavior. Recovery/failure writes are best effort: a
state-write error is logged while preserving the file/decoder error, native playback and lease
cleanup. No automatic file deletion or byte repair is introduced.

Cancellation, bounded-mailbox backpressure, invalid requests and offline simulation do not persist
failure markers. Typed SQLite errors stay identifiable across materialization fallback. A database
read/commit error produces a session fault at the current queue index instead of treating successive
songs as broken. An injected rejected cache-pointer commit verifies that an unavailable alternate
source cannot mask that database error. Before leasing, preparation checks its canonical path and
original import version against the reread row; a replacement requests a retry and catalog refresh
before any native item opens old bytes for a new record.

Changed file states/cache associations coalesce into the existing worker catalog refresh, after
native event delivery and before catalog publication. Database/file work and the new snapshots stay
off the UI thread. Native operations keep priority over catalog loading. The real controller fixture
now checks the missing song reaches the catalog as Failed, and a deliberately unavailable songs
table leaves a Session fault at index zero, no native item and no file-failure traversal. Its existing
six-start/history, waveform, queue, cache-lease, rapid-control and zero-warm-poll-allocation checks
remain.

### Checks

- **131 bounded app checks pass / 17.661 s wall / 11.595 s summed**, zero skips. The existing runner
  automatically retries two pre-existing audio cases after contention; both attempts remain in
  `/tmp/localwave-failure-state-verified-tests.log`. Controller **2.548 s**, real artwork/Metal
  **1.193 s**. Earlier complete matrices remain in `/tmp/localwave-failure-state-tests.log` and
  `/tmp/localwave-failure-state-final-tests.log`.
- Two new repository cases check preserved metadata/favorites/playlist/FTS, duplicate events,
  recovery, read-only errors and **22 stale-version combinations** (eleven guarded fields for
  failure and recovery). A new pure classification case checks cancellation/backpressure/offline,
  invalid input, typed SQL errors and genuine file errors.
- The existing worker fixture additionally checks corrupted bytes remain intact; cancellation and
  cloud/offline states are untouched; actual-start recovery followed by another native failure;
  an old token cannot mark a replacement pointer; replacement during copy-to-lease; rejected cache
  commits keep their SQL identity; and a rejected failure-state write keeps the decode error and
  leaves no playback/cache lease. Focused log: `/tmp/localwave-failure-state-focused.log`; final
  expanded coverage is in the complete verified matrix.
- Final Clippy with `-D warnings` passes **2.02 s**. Formatting and whitespace checks pass. The first
  Clippy attempt rejected two unnecessary test-only Song clones; those are now borrowed slices.
  Logs: `/tmp/localwave-failure-state-clippy.log`, `/tmp/localwave-failure-state-verified-clippy.log`.

No dependency, schema, public pxui ABI or shipping Swift/iOS change is needed. Pxui remains at
published `d611cee` (ABI 1.14). Swift/device/provider/native-memory suites are not repeated. The
seven signed frame workloads were not repeated for this worker/repository-only change; their
previous zero warm Rust allocations and strict **1 ms wall maximum failures** remain the current
frame evidence in `/tmp/localwave-media-final-performance.log`. This does not resolve stable
latency, real media-key delivery, the full Mac UI or iOS/device/distribution qualification. The
normal GUI process and its database/defaults are preserved; the overall port remains in progress.


The updated signed bundle passes dependency, sandbox-entitlement, macOS 15 minimum, default stack,
embedded dylib ID and portable rpath checks. It draws once and stays idle **507.8 ms**. Release
library/app builds take **0.64 / 10.16 s**. Logs: `/tmp/localwave-failure-state-bundle.log`,
`/tmp/localwave-failure-state-bundle-check.log`. Normal GUI PID 69331 remains alive on its mapped
older build, with its library and defaults untouched.


## Settings, appearance and protected offline space (2026-11-13)

The macOS shell now opens Settings from its sidebar or Command-comma. Back/Escape restore the
existing library route, sort and selection. Keyboard activation stays in Settings instead of
playing the selected library row. Escape closes a Settings menu before About or the page, while
the queue sheet retains modal priority. Settings and About use wrapped, explicitly measured
paragraphs at narrow widths; the retained page does not rebuild for ordinary player clock ticks.

Typed deltas enter the existing bounded controller mailbox and apply against the worker's latest
Settings. Rapid toggles do not overwrite one another with a stale full snapshot. Persistence uses
the existing Swift-compatible native keys. Download-ahead/offline-policy changes fence preparation
without interrupting the current native player; appearance/signal/reduced-motion/cache/UI policy
changes do not fence it. Save/read failures produce a settings notice.

The visible controls cover download-ahead, simulated no-network mode, 4/8/16 GB and System/Light/
Dark. Right-click menus expose 2/32 GB, four signal colours and reduced motion. All **eight**
appearance/signal themes are parsed before frames; live changes only select a cached theme. The
previous host ignored the persisted signal setting; it now uses the reference light/dark blue,
red, green and monochrome values. Show-non-audio remains hidden in the desktop layout, matching
Swift's wide layout, while its saved delta remains supported.

Offline-space summaries and trimming run on the serialized library worker after pending
preparation work. The existing cache service preserves pin/active/queued protection and never
removes Added Files. Unchanged summaries keep their Arc identity. The isolated controller fixture
uses a sparse 3,000,000,000-byte old cache entry and an Added Files sentinel, applies rapid paired
toggles and saved deltas, then verifies eviction, preserved queue/native state, the sentinel and
reopened native preferences. No normal app data or defaults are used.

About & Privacy offers the existing fixed privacy-policy and support URLs through an initial-
thread NSWorkspace bridge; worker-thread attempts are rejected. Browser launch itself is not
exercised by these checks. The macOS data-removal text does not claim deleting an app bundle
automatically removes its data. Source rescan belongs to the still-unfinished Files/source
integration and is not exposed as a disconnected button. Exact nested-menu/reference-motion
parity, screenshots/goldens and full Settings qualification remain open.

### Shared fixes and allocation evidence

PXUI **f19637b**, published on `rust-port` before the parent pin, includes ABI **1.15** raw held-
key queries and the follow-up shared fixes. Event-only input retains modifiers across frames;
focused and modal ownership does not hide this scalar query or consume presses. Popup filtering
recycles discarded keyboard buffers. Dismissed popup roots, including attached roots, cannot
intercept a press over their previous bounds on the next frame. The app dismisses only after
the popup call returns. Two signed preflights caught immediate-menu presses being swallowed
at frames 6 and 10; both failed logs remain under `/tmp/localwave-settings-performance-*.log`.

A measurement-cache key collision was in **LocalWave's caller**: recurring 2/32 GB detail text
shared a key and reshaped each change. Each variant now keeps its own measured layout. The
remaining four allocations were traced to **pxui's SlotTable**, which grew due to tombstones
with no increase in its live working set. It now reuses tombstones and grows from actual live
load; cleanup reuses its spare arrays. Recycled text buffers choose the smallest sufficient
donor, including newly assigned identities with oversized empty buffers. Existing identities
keep their constant-time capacity check. A rejected broad scan slowed layout; the final path
restricts it to new identities and insufficient buffers. Temporary allocator tracing is removed.

The original signed eight-workload run records **10 Settings allocations**, 45 actual changes
and all eight themes. Its diagnostic follow-up identifies recurring detail-layout work and a
later four-allocation growth event. These failures remain retained; the budgets, frame counts
and assertions were not relaxed. Logs: `/tmp/localwave-settings-final-performance.log`,
`/tmp/localwave-settings-allocation-trace.log`,
`/tmp/localwave-settings-allocation-backtrace8.log`. A registered 100-frame core case varies
60 node identities and short/medium/long labels beside 100 fixed nodes and requires zero warm
allocations. The app regression alternates cache limits and both menus while advancing clock
digits within the same warm-up.

### Checkpoint checks

- **136 app checks pass in 17.314 s wall / 14.371 s summed**, zero skips. Controller **2.867 s**,
  native artwork/Metal **1.733 s**. `/tmp/localwave-settings-checkpoint-app-tests.log`.
- **251 core/dynamic-wrapper checks pass in 14.206 / 11.458 s**, zero skips; all 52 dynamic
  public checks pass. `/tmp/localwave-settings-pxui-checkpoint-tests.log`. Earlier static/core/
  text/ABI/renderer matrix: 365 passes in 22.674 s, before the follow-up hit/table fixes.
- C header, ABI snapshot, exported-symbol and dependency gates pass. Final app Clippy with
  `-D warnings` passes in **1.84 s**. Formatting/diff checks pass.
- Earlier scaling-ratio failures, the intermediate 5-second scan regression and the subsequent
  passing check remain documented in pxui's qualification record. No timing pass establishes
  stable latency or changes the strict signed-app wall maximum.

Large cache/provider-maintenance latency, native process memory, actual browser/media-key delivery,
Files/import/source observation, full reference screens/motion and macOS 15/iOS/device/distribution
qualification remain open. Swift/iOS suites were not repeated for these Rust-only changes. The
normal GUI process and its database/defaults were preserved.


### Final signed bundle and unchanged strict gate

The final bundle passes signature, macOS minimum, default executable stack, embedded dylib ID
and portable rpath checks. Smoke builds/draws once and remains idle **508.6 ms**. Each workload
uses the same 1440×872 window, 10,000 songs, **182 frames/draws**, **82 warm-up frames** and
**100 measured frames**. All eight have **zero warm Rust allocations**; every strict
**1,000 microsecond wall maximum still fails**. `make performance` returns nonzero as required.

| Workload | Warm Rust alloc max | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- | --- |
| songs-original | 0 | 2343/503/1818/2312 | 642/451/555/579 |
| find-filters-title-descending | 0 | 3236/478/1958/3094 | 581/436/534/554 |
| queue-occurrences | 0 | 2827/558/2149/2582 | 674/496/613/668 |
| queue-carry-scroll-cancel | 0 | 2714/419/2299/2367 | 1811/375/647/1783 |
| full-queue-playlist-editor | 0 | 4228/741/2265/2722 | 866/552/796/861 |
| albums-artwork | 0 | 5270/520/1601/3672 | 727/484/600/653 |
| waveform-scrub-and-scroll | 0 | 4348/446/1609/3088 | 564/394/531/562 |
| settings-menus-appearance | 0 | 2684/334/1380/2463 | 619/294/506/602 |

Settings verifies **45 exact intended input actions** and all **eight themes**, with no swallowed
presses. Gallery retains **18 minimum actual image quads**, **3,756,032 resident texel bytes**,
and **zero warm cover requests**. All workloads render **200 waveform bars**; the scrub case
commits exactly **nine** release-only seeks. The ordinary seven workloads retain their original
traces. No warm-up count, allocator check, time threshold or frame count changed.

Log: `/tmp/localwave-settings-checkpoint-performance.log`. The earlier 10-allocation Settings
run and all strict-latency failures remain retained. This is Rust allocation and tested local-
gallery evidence, not native memory, stable wall latency or real-provider/device qualification.

## Files prerequisite: Swift-compatible source accounts

The native worker bridge now reads `ubiquityIdentityToken` and uses the same secure keyed
archive and base64 encoding as `DefaultICloudProvider`. The existing masked FNV hash reopens
the Swift account; only an absent identity selects the signed-out account (`-1`). Native read
and archive failures remain errors. The initial-thread guard rejects the call before querying
Foundation. No identity is queried from frame construction or logged.

`LibraryWorker::current_sources` lists that account's sources and selects its first source
only when it has no current selection. `select_source` rereads account identity and rejects
an action from a stale account snapshot before changing selection. The existing repository
transaction rejects sources owned by another account. Signing out and returning preserve
the cloud account's current source and the other accounts' rows.

The committed fixture contains a fake NSString token, generated by
`tools/generate-cloud-identity-fixture.swift`; native archive bytes and Rust account hashing
must match its Swift output. Checks cover empty lists, signed-out/resumed selection, foreign
source rejection, stale-account rejection and unchanged selection after rejection.

Checkpoint verification:

- **139 checks pass**, zero skips. Initial matrix **13.140 s wall / 14.508 s summed**;
  final matrix after guarding native identity-read exceptions **28.704 / 14.395 s**,
  including native rebuild. Slowest final case **3.782 s**.
- App Clippy with interop and `-D warnings` passes in **15.22 s**. Formatting, whitespace
  and published pxui pin checks pass; pxui remains `f19637b` with no engine changes.
- Final release library/app builds **0.15 / 23.94 s**. Signed bundle checks pass for
  dependencies, minimum OS, default stack, dylib ID, portable rpath and sandbox entitlements.
  Native smoke observes one frame/draw and **507.7 ms** idle. The running normal GUI is kept.

Logs: `/tmp/localwave-source-account-checkpoint-final-tests.log`,
`/tmp/localwave-source-account-checkpoint-clippy.log`,
`/tmp/localwave-source-account-checkpoint-final-bundle.log`,
`/tmp/localwave-source-account-checkpoint-final-bundle-check.log`.

These are worker services; **Files UI, folder/audio pickers and rescan are not connected yet**.
Live iCloud identity/account transitions, provider access and same-team migration remain
unqualified. The bundle retains the shipping Mac sandbox entitlement set. No render path
changed, so the eight-workload performance matrix was not repeated; its strict **1,000 µs
wall maximum remains failed**, and native memory and full Mac/iOS parity remain open.

## Files: folder picker, browsing and background import

The Mac sidebar now opens **Files**. **Add folder** reuses the host's asynchronous SDL/AppKit
picker, matches its completion ID exactly once and keeps the account captured when the picker
opened. Cancellation and failure are distinct outcomes. Bookmark creation, validation, source
registration, recursive indexing and audio import run on a separate lazy library worker and
SQLite connection, leaving playback's worker available. The source grant and current selection
commit in one transaction; injected selection failures roll back both new rows and bookmark
renewal. Generated cache roots, nonfolders, relative paths and cancelled registration are rejected.

An initial regression found that resolving a fresh bookmark could change a `/var` spelling to
`/private/var`, creating a second row for the selected Swift source. Registration now derives
identity from the selected path and preserves the existing file-URL spelling. The final check
reuses the old row and retains its creation date and bookmark preference key. The failed run
remains in `/tmp/localwave-files-registration-tests.log`.

Source/directory labels and natural directory-first ordering are prepared on the worker. The UI
borrows immutable snapshots, virtualizes directory rows and retires replaced storage on the worker.
Browse requests coalesce independently of the one bounded mutation; a second mutation returns
backpressure. Progress is formatted on the worker, at most every 100 ms. There is no periodic UI
polling timer. **Import folder**, individual indexed-file **Import**, **Rescan source** and **Cancel**
are connected. Rescan from a nested page refreshes and imports the whole source. Cancelling a scan
preserves the previous complete index; completed per-file imports remain. Final/cancelled work
invalidates the existing controller catalog. Source removal/reconnect and observation remain open.

The picker regression also exposed a shared SDL bug: callbacks copied their result but did not
wake an idle host. Published pxui commit **`c50bf14`** release-publishes the outcome before calling
the existing wake. The pre-fix zero-time wait check fails; selection, cancellation and failure
checks now pass. No UI ABI or frame-memory change was needed. The parent pins the published commit.

### Checks and remaining qualification

- **146 app checks pass in 15.144 s wall / 14.594 s summed**, zero skips. The runner retried
  **three existing cases** alone after contention; those retries remain in the log. The new
  real worker case recursively imports local FLAC, browses indexed children, rescans from a
  nested folder and rejects stale accounts/generated roots. The 10,000-row UI case routes
  folder/import/rescan/cancel actions, disables duplicate work and records zero warm allocations.
- Native SDL delivery covers copied Unicode paths, exact owner/ID delivery and one-shot
  cancellation/failure. The initial-thread window case is **1.194 s** including AppKit startup;
  it does not open a real picker. Slowest full-matrix case is the controller at **4.087 s**.
- SDL/pxui_sdl: **19 checks pass in 7.228 / 2.909 s**, with native window/input cases; dependency
  and diff gates pass. App Clippy with interop and warnings denied passes in **1.90 s**.
- Release library/app builds **0.17 / 10.60 s**. Signed dependency, sandbox-entitlement,
  macOS minimum, default-stack, dylib-ID and portable-rpath checks pass. Smoke draws once
  and stays idle **502.8 ms**. The normal GUI process and its database/defaults are preserved.

Logs: `/tmp/localwave-files-fixed-storage-tests.log`,
`/tmp/localwave-files-fixed-storage-clippy.log`,
`/tmp/localwave-files-dialog-wake-before.log`, `/tmp/localwave-files-dialog-host-tests.log`,
`/tmp/localwave-files-fixed-storage-performance.log`.

The actual signed sandbox picker grant, live iCloud/provider/account changes, moved/expired
folder reconnection and device behavior remain unqualified. Audio-file selection into Added Files,
source reconnect/removal, multiselection/search/filtering, automatic observation, exact reference
layout/motion, large-import maintenance latency, native memory and Mac/iOS platform/distribution
gates remain unfinished. Swift/iOS suites were not repeated for these Rust-only changes.

### Nine signed workloads; strict wall gate still fails

An intermediate signed rerun records **32 warm Settings allocations** in
`/tmp/localwave-files-publish-performance.log`; a traced follow-up records zero and is retained
separately in `/tmp/localwave-files-settings-allocation-trace.log`. The deterministic regression
then alternates `storage_busy` inside the original 18 samples per round, without extending warm-up:
it fails with **378 warm allocations**, including repeated 10/32-allocation transitions. Log:
`/tmp/localwave-files-settings-storage-before.log`.

The shared **LocalWave paragraph helper** gave the busy text and ready cache summary one measurement
key. It now includes text content in the key, preserving both layouts in pxui's existing swept cache.
The strengthened regression and final signed Settings run both have zero warm allocations. This
is an application caller fix; no temporary allocator probes or new engine ABI were added.
The earlier signed nine-workload run and all strict-latency failures remain recorded. Hidden Files
progress also no longer forces a library rebuild: the Files revision only participates when its
page is visible, with an unchanged-draw-revision regression. Completed imports still refresh songs.

Files adds a worker-prepared **10,000-path** directory to the existing eight signed workloads.
Every run retains the 1440×872 window, 10,000 songs, **182 frames/draws**, **82 warm-up frames**,
**100 measured frames** and both Rust allocation counters. All nine have **zero warm Rust
allocations**. Every strict **1,000 µs maximum wall time fails**; `make performance` returns nonzero.

| Workload | Warm Rust alloc max | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- | --- |
| songs-original | 0 | 4066/536/1666/2520 | 672/454/576/613 |
| find-filters-title-descending | 0 | 2537/519/1915/2437 | 577/439/547/571 |
| queue-occurrences | 0 | 3414/608/2175/3098 | 667/476/609/650 |
| queue-carry-scroll-cancel | 0 | 2538/443/2243/2387 | 1963/369/774/1762 |
| full-queue-playlist-editor | 0 | 4260/689/2166/3179 | 944/571/821/939 |
| albums-artwork | 0 | 2112/515/1713/2037 | 801/480/609/676 |
| waveform-scrub-and-scroll | 0 | 3340/423/1220/2392 | 607/382/563/587 |
| settings-menus-appearance | 0 | 2091/417/1583/2044 | 553/291/496/543 |
| files-indexed-directory | 0 | 3351/385/1581/2204 | 432/312/413/429 |

Files draws **14 actual row action surfaces** from the indexed directory throughout measurement.
All workloads keep **200 waveform bars**; scrubbing commits exactly **nine** release-only seeks.
Settings makes **45 exact input actions**, visiting all **eight themes**. Gallery retains at least
**18 image quads**, **3,756,032 texel bytes** and **zero warm cover requests**. The original traces
and all earlier allocation/latency failures remain recorded; no gate or warm-up was relaxed.

## Source actions prerequisite: guarded migration and native retention

Reconnect/removal worker services validate the captured account/source/root, validate a new
bookmark and generated-path exclusions for reconnect, and gather legacy identities from indexed,
registered, canonical and resolved paths. Migration and root replacement/deletion now share a
transaction guarded by the source owner, directory/hash, bookmark, error and update stamp.
Injected deletion failure rolls back migrated legacy songs, FTS and playlist membership together.
Reconnect retains source/song IDs, creation dates, Favorites and membership; removal follows Swift's
existing metadata policy and leaves original/cache files untouched. These services are not exposed
as UI actions yet; the Files options/confirmation/picker flow is the next step.

The native retention check also found that a catalog refresh cleared current metadata after its
row was removed. The controller now reads the matching current physical token/occurrence's leased
Song when the row is absent. The real paused AVPlayerItem keeps its token, occurrence, title/artist
and cache protection after source removal; protection clears after controller/native retirement.
No playback queue reset or original-file access is required for an already prepared cached item.

The focused source matrix passes **13 checks in 8.401 s wall / 1.228 s summed**, including the
new atomic migration and main-thread native checks. Native retention takes **0.185 s** including
its harness. `/tmp/localwave-source-actions-foundation-fixed-tests.log`. The first native check
used the uncanonicalized `/var` test path against the intentionally canonical cache-protection
keys; the corrected check uses the actual canonical path, preserving the equality contract.
Its initial log remains `/tmp/localwave-source-actions-foundation-tests.log`. Frame construction
has not changed and the signed performance matrix was not repeated for this prerequisite.

## Files source options, reconnect and confirmed removal

Source cards and directory headers now expose **Folder options**; source cards also accept
right-click. The menu connects Open, Rescan, Reconnect and Remove. Reconnect uses the existing
asynchronous folder picker with a typed purpose retaining the original account/source/root.
Its exact completion ID is consumed once. Both the UI and worker reject a stale root/account.
Source mutations remain bounded to one job; reconnect/removal are disabled until active work
finishes or is cancelled. Browsing requests still share the Files worker with scans/imports.
Immediate cross-source reads and queued edits during large operations remain work to finish.

Removal opens an explicit dialog naming the source and explaining that songs leave the library
while files remain on disk. Cancel closes it; Remove enqueues the guarded transaction. The
destructive menu/action uses the reference light/dark danger colours. Popup acceptance dismisses
after its callback returns. A changed root disables the old confirmation instead of repointing it.

Root replacement now reparents indexed top-level rows in that same transaction, preserving a
usable previous index before the replacement rescan commits. Child identities remain from the
previous complete scan until the existing atomic scan swap. The real worker check moves a folder,
converts its song to a legacy source-less row, reconnects, and verifies the same source/song IDs,
Favorite and playlist membership. Old-root and foreign-account removal attempts are rejected;
confirmed current-root removal leaves the moved FLAC files and foreign source/song intact.

### Checkpoint evidence

- **148 app checks pass in 11.293 s wall / 13.443 s summed**, zero skips. The runner retries
  **eleven cases** alone when they exceed their recorded baseline; these remain in the log.
  Slowest final case is the existing native controller at **2.136 s**.
- The expanded UI case exercises options, reconnect intent, removal confirmation/cancel/submit,
  busy and changed-root refusal, plus 10,000-row virtualization and scrolling. Reopened source
  options and dialogs have **zero warm Rust allocations**. Focused UI case **0.053 s** including
  its harness; expanded real worker case **0.330 s**. Native cache/metadata retention **0.07 s**
  in the final main-thread matrix. Picker delivery additionally preserves the reconnect purpose.
- App Clippy with interop and `-D warnings` passes in **2.69 s**; formatting and whitespace checks
  pass. pxui remains the published `c50bf14`; no engine/ABI change was required.
- Release library/app builds **0.20 / 10.58 s**. Signed bundle checks pass for dependencies,
  sandbox entitlements, minimum OS, default stack, dylib identity and portable rpath. Smoke
  draws once and stays idle **502.5 ms**. The normal GUI process and its library are preserved.

Logs: `/tmp/localwave-source-actions-ui-tests.log`,
`/tmp/localwave-source-actions-worker-tests.log`,
`/tmp/localwave-source-actions-checkpoint-tests.log`,
`/tmp/localwave-source-actions-checkpoint-clippy.log`,
`/tmp/localwave-source-actions-checkpoint-performance.log`.

The nine signed workloads retain the original **182 frames / 82 warm-up / 100 measured frames**,
1440×872 window, 10,000 songs and both Rust allocation counters. All nine have **zero warm Rust
allocations**; every strict **1,000 µs wall maximum still fails**. `make performance` returns nonzero.

| Workload | Warm Rust alloc max | Wall max/p50/p95/p99 µs | CPU max/p50/p95/p99 µs |
| --- | --- | --- | --- |
| songs-original | 0 | 3175/443/1458/1967 | 568/422/507/552 |
| find-filters-title-descending | 0 | 2572/436/1649/2072 | 559/409/531/554 |
| queue-occurrences | 0 | 2503/489/1728/2233 | 620/461/568/586 |
| queue-carry-scroll-cancel | 0 | 2836/379/1736/2011 | 1753/346/562/1734 |
| full-queue-playlist-editor | 0 | 2754/629/1856/2241 | 770/529/712/761 |
| albums-artwork | 0 | 2089/470/1305/1699 | 760/454/598/667 |
| waveform-scrub-and-scroll | 0 | 2288/415/1225/1943 | 551/384/495/534 |
| settings-menus-appearance | 0 | 2181/298/1264/2000 | 580/265/478/507 |
| files-indexed-directory | 0 | 2676/319/1030/1732 | 404/307/371/399 |

Files retains **14 drawn rows** from its 10,000-path fixture. Gallery retains **18 image quads**,
**3,756,032 texel bytes** and no warm cover requests. All workloads keep **200 waveform bars**;
scrub commits **nine** release-only seeks, and Settings records **45 actions / eight themes**.
The signed Files case scrolls the directory; the unit case separately qualifies options/dialog
allocation. Exact reference pixels/motion, actual sandbox picker grants, live providers/accounts,
playing-device behavior, native memory, audio-file selection, observation, source multiselection/
filters, full Mac/iOS parity and distribution remain unfinished. Swift/iOS suites were not repeated.
