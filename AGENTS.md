# Development guide

LocalWave is an iOS MP3 player. Read [README.md](README.md) for the current build instructions and limits.

## Build and test

Open localwave.xcodeproj in Xcode 16.2 or newer (CI and TestFlight builds use Xcode 26.6). Select the shared musicapp scheme, which builds the localwave app. The app target requires iOS 18.0; use an iOS 18.2 or newer simulator for the project defaults and unit-test target. Targets retain Swift 5 language mode.

Product > Run builds the app. Product > Test uses localwave.xctestplan. The test targets retain the names musicappTests and musicappUITests. Existing cases in localwaveTests/Tests.swift use Swift Testing. Do not infer broad UI coverage from target names.

## Source map

- localwave/Sources/App contains the entry point and dependency container.
- Core contains shared helpers and constants.
- Domain contains models and repository/service protocols.
- Data contains SQLite repositories, file access, import and playback services.
- Features contains Library, Player, Playlists, Sync and shared SwiftUI components.

SQLite.swift is the Swift package dependency. Shared Domain, Core, Data and Player targets are declared in Package.swift; Xcode links those modules. Do not re-add their source files to the app target. Song and source-path search use FTS5; song queries include BM25 ranking. AVFoundation and MediaPlayer provide playback and remote controls. Folder access uses security-scoped bookmarks; selected cloud files may need downloading before playback.

The Rust migration follows `docs/rust-port-reference.html`; current qualification is in `docs/rust-port.md`. `crates/lw_core` holds models/storage, `lw_apple_sys` owns native FFI, `lw_ui` uses the public pxui wrapper, and `lw_bin` owns the macOS window/GPU and serialized library worker. `pxui/` is a pinned submodule; read its AGENTS.md before changing it. Use `make build` / `make bundle` for the Rust app and `cargo t -p lw_core -p lw_apple_sys -p lw_ui -p lw_bin` for bounded tests. Static SDL3 is built under `target/sdl`; only libpxui ships as a dylib. The app uses the platform's default executable stack. The terminal player and SwiftTUI dependency have been removed. Retain the shipping Swift app until the replacement passes the plan's gates.

Continue Rust work on `rust-port` in both repositories and commit completed milestones regularly. Publish the pxui commit before staging its pointer in LocalWave so every parent checkpoint is fetchable.

Keep reusable layout, widget-body/style, animation, text and rendering machinery in pxui behind its public API. LocalWave defines its screens, application-specific bodies, theme/motion values and state. `lw_ui` must not import private pxui engines or GPU backends. The host currently owns window/device setup, Metal frame acquisition/submission/presentation and application artwork texture lifetimes; it calls public `Renderer::prepare/draw/forget_image`, with no app shaders or drawing pipelines. This host boundary is not a claim that every Metal call lives in pxui.

Palette suggestions and ranked queue payloads are prepared on the library worker in the same SQLite snapshot as the underlying page. Keep its query separate from the page request, preserve page identity across palette-only query changes, and refresh both on actual playback starts. Native palette command/album/artist matching preserves accents; ordinary song matching uses localized standard containment. Most Played uses the top 24 positive history counts with stable library-order ties. The visible palette is not connected yet.

Rust performance is a required gate: zero Rust heap allocations across app + libpxui on warm input/build/draw frames, no rendering while idle, and UI frame builds under 1 ms at 10,000 songs. Use retained views, fixed-height virtualization, cached strings/widths, worker-side database/import/artwork work, bounded caches and queues, and fixed-capacity messages. No hot-frame formatting, sorting, decoding or fresh resource wrappers. Run `target/release/localwave --performance-check` after host/render changes; it fails on allocations or frame-build budget overruns. Keep its workload representative as real screens arrive. Native Metal/OS allocations are outside the Rust allocator counter and must be separately profiled; never label the whole process allocation-free based on that counter.

`make check` includes the host input allocation regression and prepares static SDL when needed. `library_bench`, `catalog_bench` and `cache_bench` examples exercise 10,000-song/path SQLite workloads outside the frame loop. Preserve bulk-read ordering and repeated queue occurrences while avoiding redundant model copies; avoid correlated per-song album scans and repeated UNINDEXED FTS identifier scans during batch import. Timing evidence must retain failures alongside passing runs.

Song FTS row IDs are cached on `LibraryRepository`; `data_version` refreshes the map after external commits. New same-connection FTS deletion paths must invalidate `song_fts`, and failed song/import transactions must discard it. Keep this cache off the UI thread and preserve arbitrary legacy FTS row IDs.

## Test budget (fast by default)

Default verification loop must stay fast. Budgets are hard limits; a change that breaks one must fix or move the slow test before merging.

| Loop | Command | Budget |
| --- | --- | --- |
| Shared (every change) | `swift test` | 30 s wall, warm build |
| iOS unit (app logic changes) | `xcodebuild ... -only-testing:musicappTests test` | 90 s wall, warm build |
| UI flows (redesign/UI changes only, nightly or manual) | full `musicapp` plan on a simulator | on demand; never required per PR |

Rules: no `sleep` or real-time waits in unit tests, use injected clocks or polling with short deadlines; no network; one shared fixture per suite; shared tests run in parallel (`swift test --parallel` when isolated); a unit test over 1 s needs a reason in a comment or moves to the UI plan. Pull-request CI runs only the first two rows; UI flows run through the manual **Verify** workflow. Release tags run `swift test` only so the TestFlight upload is not blocked on UI flows.

Read the implementation before changing behavior or promising reliability. Run `swift test` for shared repository, FLAC parsing and recursive import checks. Run the Xcode test plan on a compatible simulator for app checks. Physical-device playback, bookmark recovery and iCloud behavior need device qualification. `scripts/build-unsigned-ipa.sh` produces an unsigned device IPA; CI and the manual release workflow use the same script.

Offline materialization and eviction run on the same `LibraryWorker`. Use the native `Preferences` setters and cache guard for pin/active/queued protection changes so retained snapshot generations remain valid; do not add direct defaults writes that bypass this contract.

Queue/settings preferences use native Swift-compatible keys. The pure core `PlaybackContext` covers all seven Swift cases; Foundation JSON conversion stays on the library worker. Commands and controller snapshots carry retained typed contexts. Preserve original saved context bytes until the context changes; unknown or invalid saved cases display Library without overwriting those bytes. Waveform persistence uses the existing separate database and 512-float JSON blobs; coordinated PCM decoding/versioning feeds the serialized library service and the transport. The controller sends 512 peaks inline in Status. Load only after playback preparation/control work, cancel for controls/browse/critical native events, and gate cancellation reset under the inbox mutex. Clear stale peaks when selection/catalog identity changes; use token changes to revalidate after materialization. Refresh must publish metadata invalidation before loading replacement peaks. Decode only confirmed provider content, retain cached peaks after audio eviction, and preserve the content-hash signature across LRU touches. Keep both actual `Caches/LocalWave/waveforms.sqlite` and legacy cache paths excluded from source scans, including WAL/SHM sidecars.

The native queue player belongs to the initial thread; prepare `player::Prepared` items on a worker and transfer ownership before installing them. Keep current plus two upcoming items, preserve buffered items during append, and service the main run loop in headless hosts. `localwave-playback` is a diagnostic local-file host; full application failure traversal, persistence and library materialization are still being integrated.

Playback preparation leases use the existing native cache-protection keys. Release a token only after the native item is removed; adopt a new active token or acknowledge native stop before releasing the active file. Six leases cover current/upcoming plus one replacement batch; apply backpressure beyond that. Queue/model mutations stay off the UI frame; the new `lw_core::player::Session` owns ticket validation and bounded occurrence-based file-failure traversal. Native `Started` events identify actual first playback of each physical item; deliver them to `record_playback_start` before retiring the lease. History snapshots stay on the worker; catalog/UI strings should be prepared there. Recency touches apply only to regular managed Music files and remain best effort. The GUI now uses `controller::Controller`: 32 bounded ordinary commands plus a reserved Pause/Stop slot, 128 critical events, one native operation/acknowledgement, coalesced snapshots, retained queue payloads and worker-created display strings. A worker error must wait for native stop acknowledgement before clearing leases. Queue/index/volume/history delivery is connected; import UI requests and the full reference UI remain unfinished.

The real SDL host retains its event buffer and the UI retains its held-key routing buffer. The main-thread `window_input` regression covers SDL pointer/buttons/modified keys/resize/wheel/pinch through app encoding and UI build with zero warm Rust allocations. Owned text/IME/drop/dialog payloads still allocate; this does not qualify all input or native memory. The signed-app performance workload now loads 10,000 songs on the worker and repeats table scrolling with changing clocks; its events are synthetic and scrubbing/menus/native allocations remain unqualified. Smoke/performance library setup must remain isolated from the user database/defaults.

Preserve logical queue selection across rapid commands while a native advance awaits acknowledgement. Volume/seek updates are coalesced and do not cancel file preparation; queue-changing commands still fence native operations and preparation. Keep interactive QoS on the main UI thread only, after the library worker is spawned. Catalog edit commands must invalidate cached current metadata.

UI catalogs are prepared on the library worker: one label arena, fixed-height virtual rows and retained album/artist/playlist orders and queue/context payloads. Frames borrow it and clone existing Arcs when starting playback. RefreshCatalog requests coalesce without fencing native preparation. Build songs and playlists in one read_snapshot transaction, retain the last good catalog on failure, and invalidate current metadata on success. The mailbox keeps at most one incoming and one retired catalog; superseded and retired catalogs drop on the worker outside its mutex. poll_with_catalog repairs route/selection identity before adoption without holding that mutex; do not consume catalogs in command delivery. Preserve retirement backpressure and wake a waiting replacement after draining. Import/edit commands must request refresh after their commits; source observation remains unwired. Keep clock digits on the cached glyph path, and preserve accessibility string capacity when pxui reuses virtual-row slots. Warm visited-row scrolling passes zero Rust allocations; newly shaped text is not covered by that claim. Preserve the strict 1 ms wall maximum: one passing run does not resolve recorded latency failures, and CPU time or passing percentiles do not replace its threshold.

Sidebar collections also use fixed-height virtualization. Derive explicit numeric node keys with `int_key`: 0 and 1 are reserved slot-table sentinels. Keep navigation scroll state scoped by route. pxui recycles empty text/name/value buffers and compacts retained draw runs within a budget proportional to live runs; preserve the long one-direction scroll regression when changing slot or draw storage. The app authors accessibility semantics but only assembles a tree in tests or when a bridge consumes it; native bridges remain deferred by the plan. Use `--performance-check --trace-frames` for optional Instruments frame intervals and phase timing; retain strict-gate failures and do not attribute wall/CPU gaps to scheduling without frame-specific evidence.

Unchanged native polls must not wake the library worker. Notify for delivered events, changed snapshots, faults and operation acknowledgements; preserve separate command, stop and catalog-retirement notifications. Trace diagnostics retain correlated frame/CPU/phase samples in fixed storage and print outliers only after measurement, before sorting percentile copies. The 1,000 µs wall limit is unchanged. Recorded thread-state durations explain the specific preempted frames in docs/perf-rust-frame-correlation.json, not every ordinary failure. In trace analysis, the first thread column is the owner; later thread columns can identify a waker or preemptor. Overlap duration intervals rather than inferring state duration from adjacent context-switch records.

Reference motion tokens live in `lw_ui::motion`. pxui sampled curves retain uneven positions and overshoot in fixed arrays; stop them by duration, never by progress reaching 1. Keyframes rescale table/step/Bezier duration and use at most eight stack descriptors in the public wrapper. Preserve the browser/Swift vectors and static/dynamic zero-allocation regression. ABI 1.0 is unreleased: the expanded curve/node layouts require rebuilding the app and dylib together; update both snapshots and C layout assertions for an intentional layout change without weakening the ABI gate.

`Target::Scale` animates view contents through the existing transform table; scale-only runs must not rebuild or rewrite instance runs. Preserve independent axes, fixed fractional anchors, manual/nested transform composition and hit/accessibility geometry. Animated bounds invalidate the accessibility walk. Icon buttons keep their full semantic/hit rectangle and scale only a cached visual child to 0.96; start a new animation only when held state changes. Use the `ThemeSource::REDUCED_MOTION` flag for reduced-motion input (it overrides the parsed token). Compaction scratch/buffer capacity covers existing fresh views becoming retained together; keep the actual retained-length budget and zero-allocation regression intact.

Root and pxui Apple Silicon invocations share `target/` and must resolve the same `pxui/tools/cc-stack.sh` linker path; different linker configuration rebuilds unchanged shared dependencies. Root sets `PXUI_CONSUMER_DEFAULT_STACK=1`: LocalWave retains its default stack, while pxui-owned tools retain their legacy stack and other consumers are unchanged without the opt-out. The public wrapper tracks that environment value. Preserve the bundle verifier's `LC_MAIN.stacksize == 0` check; do not solve app stack problems by raising it.

Backdrop saturation defaults to 1 and is applied once after the blur's vertical filter unpremultiplies RGB, before image tint/opacity. Keep Metal/GLSL and checked-in SPIR-V in sync. The public setter is ABI 1.1; finite, nonnegative values only. Preserve full app/library warm blurred-draw allocation tests as well as state/frame tests.

`Renderer::resize` (ABI 1.2) prepares both blur scratch targets at startup/drawable resize, before recording commands. `Painter::blur_in` requires prepared capacity and returns a texture plus its occupied UV rectangle; always composite using both. Both downsample factors share the half-window allocations, with viewport/scissor limited to occupied pixels and second-pass filtering clamped to their texel centers. Retire replaced targets through the existing flight window. Prepared render targets retain their immutable attachments and restart only on the same command wrapper. Drop closed cached passes before retiring textures, even when an ended metadata clone survives. Preserve zero-allocation warm RGBA/BGRA blurred draws, rotating targets, multiple blur regions and changing sigma/scale. The renderer retains a boundary Vec containing cheap cloned texture handles, clears its handles after every layer and keeps capacity for the largest observed layer. Preserve the 64–128-image growth/shrink and large blurred-layer allocation/pixel regressions.

Renderer status exports must reject both UI frame callbacks and another renderer's split callback on the same UI. `Shared::current == null` fences the latter while the renderer owns the engine; checking only `in_frame` is insufficient. Preserve the nested-renderer refusal regression.

Foreign-image and blur-source wrappers use the full native handles/extent/format identity and explicitly destroy retired wrappers. Native Metal release draining must keep Vec capacity, preserve concurrently queued releases, and destroy native handles outside the queue lock. Rust allocation counters do not qualify native allocator behavior.

`TextStyle::features` (ABI 1.3) consumes the former reserved u32 at offset 28; bit 0 enables `tnum`, and unknown bits are rejected. Keep features in both the caller-keyed Style comparison and the core text-key hash, or mixed styles reshape every frame. Feature setup allocates only on a new layout; preserve the standalone/canvas/static/dynamic warm glyph and allocation regressions. The Inter fixture is test-only; production fonts remain Host Grotesk/Geist Mono.

Inline text (ABI 1.4) uses TextSpan (24 bytes) and a fixed eight-record TextSpans packet (200 bytes). Ranges are ordered, disjoint, nonempty UTF-8 byte ranges; validate lengths, flags and packet size before changing a node. Text nodes own their original accessibility label and retain optional span storage across rebuilds/slot reuse; plain text and patch_text clear active spans. Include active span fields in layout keys and comparisons. Keep plain Style small.

Span colour cuts use the existing caret geometry and retain original glyph shaping, bitmap coverage, subpixel bins and cache replay. Keep the Arabic ligature, mixed-bidi, CRLF, wrapping and ellipsis regressions. Backgrounds/rules stream from shaped runs without allocating; cosmic-text LayoutRun::highlight allocates a Vec and must stay out of warm drawing. Rules are 1.5 points at the line-box bottom, with independent colour. Glyphs/decorations share node/view opacity, clip and one instanced draw per layer. Native pixel and static/dynamic allocation checks qualify these paths; they do not qualify native allocation or every script.

Use the shared ABI read_style helper: validate the leading size before copying a TextStyle, including legacy measure/quads/raster/canvas entries. Span packets check count before records, and read only active records and flagged colours. Preserve actual short-prefix descriptor regressions. Rich-node centering uses fractional font points directly; the 20.25-point node/canvas comparison catches premature integer rounding. Center the whole wrapped block using its line count and line height; baseline measurement uses the current layout width. Preserve the three-line node/canvas geometry regression.

TextStyle.ellipsis (ABI 1.5) accepts NONE=0, END=1, MIDDLE=2, HEAD=3; reject all other values. Map modes through the existing shaper and include them in layout comparisons/content keys. FIT_TEXT must preserve explicit modes. Preserve full accessibility labels and the whole-block height limit across explicit paragraphs. Keep mode-specific raster/node/canvas/golden and warm allocation checks; hanging blanks and font overhangs must not be mistaken for a new layout clipping requirement.

Native font discovery (ABI 1.6) lives in font_sys, an open std-only crate. CoreText queries happen only for missing shaped clusters; file/coverage caching and reshaping stay in pxui_text. Inspect outline/bitmap table compatibility as well as character coverage: hvgl-only faces can shape but cannot rasterize in Swash. Require actual raster/native-pixel tests. Reuse fontdb mappings; do not copy large system font files or scan directories. Keep explicit-font constructors deterministic; the app enables discovery before its first Ui frame. Reject late Ui enabling, invalidate standalone layouts once and preserve idempotent enabling. Keep mapped paths bounded to 128 and negative clusters to 512×256 bytes; clearing negatives must permit later cold retries. Second atlas pages and native allocation/font parity remain separate requirements.

Glyph atlas pages (ABI 1.7) are bounded to two fixed pages per mask/colour kind. Second mirrors/textures allocate only on first use; preserve old handles/UVs and in-flight/retained shelf protection. Atlas codes 2/3 and GLYPH_PAGE_2 select second pages without a draw break. Prepare must upload each newly active full mirror on the app's command buffer before partial uploads/draws; reset initialization after upload failure. Preserve CPU/public/native page-growth, eviction and warm allocation regressions. Bundle checks require the embedded dylib ID @rpath/libpxui.1.dylib and the app's sole rpath @executable_path/../Frameworks.

FrameIn ABI 1.8 appends appearance/reduced_motion after the frozen 48-byte prefix; keep the legacy allocation test, partial-tail refusal, larger-descriptor bounds and matching static/dynamic validation. Keep the old layout row frozen and record complete versions separately. Host reduced motion overrides scoped themes, snaps active keyframes and restores authored flags when disabled. Parsed root themes use use_theme between frames, preserving owner/callback guards. The main-thread Environment caches AppKit KVO/workspace notifications, wakes only on changed facts, and removes observers before the SDL window dies. Do not poll OS/defaults in frames; saved settings come through worker-owned controller status. Cache both parsed themes, update native background only on reported change, and preserve cached-read/theme/native-pixel/allocation regressions.

ABI 1.9 exposes retained view opacity/scale/clip and measured virtual lists. Opacity is per instance; clip coordinates are window points and NULL clears the clip. Scale accepts finite zero/negative values and resets earlier rotation/fixed-pivot transforms. Equal control values must preserve transform-table storage and retained instances. Measured lists require an active frame, count <= 1,000,000, a finite positive estimate and finite estimated total; row callbacks use only their current borrowed UI handle and may repeat while measurements settle. Preserve static/dynamic validation, callback recovery, hit geometry, native pixels, draw counts and warm allocation checks.

Browse delivery uses one coalesced fixed-capacity request slot, separate from the 32 playback commands
and reserved Pause/Stop slot. New browse requests do not fence preparation. Drive native preparation
and acknowledgements before sorting; command revisions and preparation identities must never decrease
when a reserved control follows a newer coalesced ordinary control. Preserve the full-queue/latest-browse
and continuous browse/seek/volume native regression.

Catalog views share an immutable `Arc<Library>` label arena and ID index. Find uses distinct FTS
membership in original library order; column string orders use worker-owned native localized natural
keys, with song ID ties ascending and unknown durations last in either direction. Establish a SQLite
read snapshot before reading data_version; refresh after same-connection edits explicitly. Find must
hide results for an older query, retain editor focus through worker replies and reset result scrolling
for a new query. Query packets hold 1,024 UTF-8 bytes; reject oversized edits visibly without truncation.
The focused scroll regression includes app and dylib Rust allocations. `make performance` runs both
Songs and prepared filtered-Find/title-descending workloads even if one fails. Native memory, provider
qualification, other Find result sections and the full reference UI remain unqualified.

Find tokens live in `lw_core::query::SongQuery`; only its text goes to FTS. All free words also use
native localized containment across title/artist/album/album artist. Preserve invalid year: handling,
conjunctive token semantics and legacy format precedence (present extension, relative path, local
path, bookmark; a present empty value stops fallback). Album artist/year/format presence stay in the
shared catalog; hydrate file facts only for survivors that need legacy inference or availability.
Read song/source facts in one snapshot, then probe resource metadata after ending that transaction.
Cache each source grant or failure once per request. Copied state and stored sourceAvailability are
not proof of current availability; probes must not read audio or request downloads. Preserve path
containment, balanced grants and the isolated resource regression. Provider/device tests remain open.

Filter chips use bounded UTF-8 packets, keep user text casing and reject an overflowing addition
without losing the accepted query. Keep their wrapping, checked semantics and warm allocation test.
A successful actual playback start invalidates active Unplayed results; do not poll defaults in a
frame. Overview browse packets clear stale Find projections when navigating to Albums or Artists.
The signed Find workload uses `Track m4a year:2019 unplayed`, with all 10,000 fixture songs dated 2019;
it must still match 10,000 rows and obey the unchanged warm allocation and wall-time gates.

The shell reads `Ui::signal_state` (ABI 1.10) because it consumes no owned key vectors. Preserve the
keyboard-chip warm regression, including Space press/release with modifiers. PXUI retains routed
signal/modifier storage and encodes borrowed events into its existing ABI chunks; `signal` must
still return every ordered key/text/IME event. Owned key-vector reads and text payloads remain
separate allocation paths. No input event may be dropped to meet the numeric allocation gate.

Queue drags transfer pointer capture to the stable list with ABI 1.12 `transfer_carry`; source rows
may be virtualized away. Preserve wheel/edge scrolling, exact occurrence targets, cancellation and
no activation on release. The queue regression scrolls 30 rows while holding duplicate-song
occurrences, checks the original grip disappears, and counts warm Rust allocations with live clocks.
A11y IDs are stable while a node remains live; refresh the identity probe after virtualization.

`make performance` runs Songs, filtered Find, Queue, queue carry/scroll/cancel, the full queue
sheet with its playlist editor, artwork and waveform scrub/scroll, even if an earlier workload fails. All load a valid cached 512-peak fixture through the worker; the scrub case checks 200 actual bars, retained capture and nine release-only seek commits. Qualification requires a 1440x872 logical window. Every performance frame advances
PlayerView revision along with position; changing the value alone can leave the Now panel cached.
The 40-frame carry trace repeats through two warm cycles before measurement, asserts retained
payload identity/capture, and cancels before release; any resulting application intent is an error.
The trace must not edit the fixture queue or touch the normal database/defaults. Preserve the exact
1,000 µs wall maximum, both Rust allocator counters, correlated diagnostics and recorded failures.

Enable SDL resizing before configuring AppKit full-size content. Window chrome uses the live Metal
layer token and measures contentLayoutRect only at startup/native size changes; never query it in
UI builds. Keep interactive content below its measured top inset, collapse the tall Now panel in
short windows, and preserve native zoom/full-screen eligibility. Do not replace the native controls
or raise the default stack. Actual full-screen/Spaces gestures and native allocation remain separate
qualification from style, button-enabled and geometry tests.

Full queue editing resolves occurrence IDs on the worker and preserves the current physical item
when its index moves. Up next retains its stricter upcoming-only boundary. Saving a queue creates
the playlist and its deduplicated first-occurrence membership in one database transaction; failures
must retain the queue, playback and Clear/Undo. Save commands do not fence preparation. Publish the
refreshed catalog after success; playlist names use the existing bounded UTF-8 packet and reject
blank or oversized input visibly. Artwork delivery uses a separate persistent worker with coalesced eight-key requests and a preallocated eight-result queue, keeping ImageIO work off playback and UI threads. Decode ahead without waiting for a frame per cover; bound queued decoded RGBA to 8 MiB. Poll drains a fixed batch without allocating, and old pixels retire on the worker outside its mutex. Upload every result in order and reject all already-seen revisions, including earlier entries in a replayed batch. Catalog-scoped cover IDs provide constant-time lookups; refresh/size-density changes advance epochs and reject stale results. Retained image views include the cache revision. Texture retirement follows completed GPU serials, never the next UI frame. Negative results remain cached until refresh. The 1,024-entry texture cache has a 128 MiB resident texel budget; requested sizes account for density and visible gallery size, and remember requested quality even for smaller source images. Cold decoding/upload/resource creation allocates. Pending cover candidates survive retained frames and are resolved against final node geometry with two image heights of vertical prefetch; only eight keys cross the worker mailbox. Use display-sized thumbnails. After removing the one-cover handoff bottleneck, three consecutive signed scrolling gallery runs pass the Rust allocation/request gates, with last uploads at frames 21/25/21 before the unchanged frame-82 warm-up boundary. Preserve the deterministic eight-covers-before-any-poll regression and full-batch native pixel/replay/allocation checks. Cold work allocates; strict wall latency and native memory remain open. Native decoding rejects axes above 32,768 or more than 64 Mi source pixels before thumbnail creation. Generic identity interning, wrapper reuse and explicit completed-serial release belong to pxui (ABI 1.13). Call Renderer::forget_image on every sampling renderer after its final sampling serial completes and before destroying/reusing the native image. Ignore stale retained bindings on draw segments with no IMAGE instances. Retirement scans only actual pending entries; never submit an extra frame merely to report completion. Native allocations and real provider/full-screen behavior remain unqualified.


Waveform bars use ABI 1.14 `Paint::fill_fractional`: preserve the reference 200 max-pooled bars,
quarter-step widths, bottom baseline and signal cursor. Keep held preview separate from release
seeking, retain five-second keyboard/accessibility adjustment and clamp against finite duration.
The shared shape writer rejects invalid/collapsed bounds; matching Metal/GLSL pixel-area coverage
and validated SPIR-V preserve thin-bar weight at 1x/2x without extra draws or warm Rust allocations.
The ABI output dirty-span copy reserves from existing engine dirty capacities. A later partition
of warmed instances must not allocate in `Ui::output`; preserve its static/dynamic regression.
`--trace-frames` records fixed-size host/build/after-build allocation samples and GPU/output-getter
subphases, printing after measurement. Keep the strict 1,000 microsecond wall gate and all failures.
Waveform reveal/stagger, native memory and platform/device qualification remain open.


The GUI opts into `controller::Controller::enable_media_controls`; normal hosts publish through
MediaPlayer, while smoke/performance/controller checks pass false and never register or replace a
system media session. Native handlers enqueue at most 32 ordered commands under a lock and wake the
host. A full queue returns failure; accepted heads stay until the existing worker mailbox accepts
them. Resolve Toggle on the worker before committing a pending selection, so rapid presses do not
read a stale UI intent. Remove each exact target and reject late callbacks before stopping the player.

Now Playing publication stays in controller polling, outside drawing. Reuse retained metadata Arcs
and let the system extrapolate ordinary playback ticks, with 250 ms drift correction; paused seeks,
identity, metadata, duration and state changes publish. macOS needs explicit playbackState as well
as dictionary playback rate. Read no files or tags there. Covers reuse the existing artwork worker's
RGBA batch and epoch/path/revision checks, and retain just the native Now Playing image. Give a
pending current cover first place in the same bounded request packet so a full gallery cannot starve
it. Cold native image/dictionary creation is outside Rust allocator qualification. Physical keys,
Control Center, native memory and iOS interruption/session qualification remain open.


Playback file-state writes compare the saved Song's ID, key, path hash, dates, state, cached path,
source/path/bookmark, size and availability in one guarded UPDATE. Keep metadata, paths, favorites,
playlist membership and FTS unchanged. Native failures must match a leased token, generation and
occurrence; never fetch the newest row and mark it on behalf of an old native item. The six existing
worker-side leases retain their prepared Song snapshot. Update only its state after our successful
failure/recovery write, so the same physical item can fail again after recovery.

Do not mark cancellation, backpressure, invalid requests, offline simulation or typed SQLite errors
as file failures. Preserve a cache-commit database error through fallback selection; the controller
reports a session fault without traversing the queue. A rejected failure-state write must preserve
the original file error and cache protection cleanup. Already-local Failed files remain Failed
through preparation and recover on actual native Started; verified materialization retains its
existing Copied semantics. Coalesce file-state/cache changes through take_playback_file_changes
and the controller's existing worker catalog refresh. Compare the prepared canonical path and import
version against the row before leasing: a replacement retries without opening old bytes for a new
row. Native decode/provider/device failures and native memory still require platform qualification.


Settings sends typed deltas to the serialized controller, applies them against its latest saved
settings and persists through the existing native keys. Do not send stale full Settings snapshots
from frames. Appearance/signal selection uses eight pre-parsed themes; never parse in a frame.
Cache usage and protected trimming run on the library worker after preparation work. Preserve the
Added Files exclusion and pin/active/queued protection; reuse unchanged summary Arcs.

Settings/About preserve library route, sort and selection. Exclude playback clock revisions from
the retained Settings page; use content-specific measurement keys for recurring label variants.
Dismiss custom popup acceptance after Ui::popup returns. Command-comma uses the raw scalar held-key
query; routed key vectors hide focused modifiers and can allocate. Preserve immediate overlapping
menu presses, narrow wrapped paragraphs, keyboard activation and warm cache-limit/menu checks.
`make performance` also runs Settings with 45 real input actions and all eight themes, with the same
182 frames / 82 warm-up / 100 measured frames and strict allocation/wall-time limits. Rescan all
sources remains with Files/source integration; do not expose a disconnected action.

Files source ownership uses the Swift secure-keyed-archive/base64 iCloud identity and the shared
masked FNV hash; only an absent native identity maps to account -1. Archive/read errors must remain
errors, never silently switch accounts. Read identity only on the library worker. Scope source
listing and current selection to that user; revalidate the expected account before selection and
let the repository transaction reject foreign sources. Preserve the Swift-generated fixture,
signed-out/resumed account selection and stale/foreign rejection checks. Files UI/pickers/rescan
and real account/provider transitions remain unqualified.

Files uses its own lazy LibraryWorker/SQLite connection so enumeration/import cannot occupy the
playback worker. Keep one coalesced browse request, one bounded mutation, explicit cancellation,
worker-formatted progress and immutable row snapshots retired on the worker. Add-folder registration
validates its bookmark and generated-path exclusions before atomically saving the grant and current
selection; preserve the selected path's Swift identity when bookmark resolution changes /var aliases.
Use the existing asynchronous SDL folder picker, match its exact request ID once and retain the
account captured when it opened. SDL completion must wake the idle host for every outcome.
Rescan from a nested directory indexes/imports the whole source. Cancelled scans keep the previous
index; completed per-file imports remain. Completion/cancellation invalidates the controller catalog.
The ninth signed workload uses 10,000 indexed fixture paths and verifies actual virtualized draws;
preserve the existing eight workloads and unchanged allocation/wall-time/warm-up gates. Automatic
observation and real sandbox/provider/device qualification remain unfinished;
do not present these as completed by the picker workflows.

Wrapped paragraph measurement keys include their text, as recurring busy/ready cache summaries
must retain separate layouts. Keep the Settings regression alternating storage_busy within its
original 18 samples/round and unchanged warm-up; a coalesced real worker reply can otherwise hide
the collision. Hidden Files progress must not change the library draw revision or force frames;
catalog changes from completed imports still pass through the existing controller refresh.

Source reconnect/removal migrate legacy song hashes from the registered, canonical and resolved
root spellings before changing ownership. Keep that migration in the same guarded transaction as
the root update or deletion. Revalidate the account/root captured by an action; the transaction
also checks owner, root, bookmark, error and update stamp. Never delete original or cache files
when removing a source. Current native items retain their cached-file lease and metadata from the
leased Song after the library row disappears; release cache protection only after native retirement.

Folder options are available on source cards and directory headers, with right-click support on
cards. Capture account/source/root in the menu, confirmation and reconnect picker purpose; reject
stale confirmations both in the UI and worker. Reconnect reuses the existing native folder dialog
with its own typed purpose, exact request ID and one-shot completion. Removal requires the explicit
dialog action. Mutations share the bounded Files slot; cancel work before reconnect/remove.
When a root changes, reparent its indexed top-level rows in the same transaction so the previous
complete index stays browsable until rescan commits. Preserve the real moved-folder/legacy/Favorite/
membership/foreign-source checks, warm options/dialog allocation case and native cache-lease test.

Audio selections use SDL OpenFiles with exact one-shot request IDs and the account captured when
opened. The Files worker validates the whole selection, holds each resolved grant only for its
coordinated copy, and uses the same bounded stream/cancellation/version checks as offline copies.
Keep original filenames, identical-content reuse and exclusive UUID-prefixed collision promotion;
never overwrite a different track or leave staging files. Named copies skip unused SHA computation.
Register/index/import Added Files through the existing source services. Attach selected private
files as Copied under the cache guard so playback does not duplicate them into managed Music.
Preserve the repository's shared-song-key behavior, selection order and Favorites/membership.
The retained selection crosses the existing snapshot retirement path; retry controller delivery
until accepted and clear it on account changes. No picker or import filesystem work belongs in a
frame. file_path must preserve /var and /private/var spellings while normalizing dot components;
rewriting one breaks registered source-root identity against its indexed children's parent keys.

Finder drops accumulate through SDL FileDragEnded, even when file events span host pumps. Capture
the known account on first hover/file; if Files has never opened, resolve the current identity on
its lazy worker rather than guessing an account. Keyboard focus loss only dismisses the visual
hover because Finder may still deliver its files. Submit one bounded mutation per complete set;
reject invalid/oversized batches as a whole and report busy delivery visibly. Preserve empty/cancel
cycles with zero warm Rust allocations. Native SDL delivery checks own their copied Unicode paths.

Mixed selections validate every path and folder exclusion before copying; copy selected audio
first, import selected folders and then Added Files, and build playback in Swift order: audio
selection order, then localized natural relative-path order per selected folder, deduplicated by
song key. indexed_source_songs joins the current index in one minimal SQL projection per source;
do not hydrate full Song rows or issue one queue-building lookup per folder track. Empty-folder
success navigates Files without clearing playback. Imports expose progress/Cancel on Files.

The passive external-drop overlay draws through the existing root canvas with accessible
instructions. It must not shift layout, capture internal queue drags or force idle redraws.
make performance retains the nine existing workloads and adds --performance-drop: 182 frames,
82 warm-up, 100 measured, 50 actual measured overlay quads and no import jobs. Keep both allocator
counters and the strict 1,000-microsecond wall maximum unchanged. Synthetic hover/cancel checks do
not qualify real Finder sandbox grants, provider downloads, native memory or platform devices.

Continuous volume/seek delivery must alternate with ready current/upcoming preparation on the
controller worker. Alternate both control kinds when pending; preserve reset/stop/queue fencing
priority. Deferred, failed or cancelled preparation must allow ordinary controls to progress too.
Keep the deterministic acknowledgement-driven starvation regression and the real native smoke;
never hide a preparation starvation failure with a longer timeout.

SwiftUI motion uses retained-view table transforms from pxui ABI 1.16, never per-frame reshaping
or rebuilding for entrance ticks. Same-time opacity/offset keyframes compose as one pose. Navigation
starts page and bounded first-row entrances; ordinary query/catalog/player updates must not restart
them. Respect reduced motion and stop repaint requests after settlement. Keep the library/panel clip
regions at their viewport boundaries. The matrix clock uses Swift's exact glyphs, padded minutes,
one-column colon, 43-point height and 0.78-unit fractional dots. Only changed digits fade, in two
steps over 140 ms with 16 ms row stagger. Share seven fading rows across digits so carries do not
increase animation capacity. Preserve minute/hour/carry, idle and warm allocation regressions.
Album cards use the Swift 150-point adaptive minimum, 20-point column gap, square artwork and
28-point row gap. Prepare captions (including year) on the library worker. Fractional point positions
need subpixel comparisons; do not weaken the strict signed-app wall/per-frame allocation gates.

Viewport request changes do not cancel selected artwork prefetch. On the existing worker, merge
new visible keys first and unfinished selected keys into spare slots of the same eight-key packet;
retain the eight-result / 8 MiB decoded bound. Publish completed same-epoch work even if its view
has briefly left the prefetch margin. Explicit cancel is sticky until the worker consumes it and
still clears selected work; catalog epochs reject old results. Preserve the deterministic paused-
after-first-thumbnail regression and three signed scrolling runs, with the original frame-82
warm-up, zero warm requests/allocations and all strict wall failures retained.


Artists uses the SwiftUI desktop table: 37.3-point header, 38-point rows, 28-point artwork,
12-point artwork/text gap, 32/8/20-point column insets and 140 ms hover. Prepare distinct album
summaries, rounded minutes and bundled-font preferred column widths on the catalog worker after
leaving its SQLite snapshot; share the result across catalog clones. Scale four widths and borrow
visible rows on the UI thread. Preserve the 10,000-artist signed workload in addition to all ten
existing workloads, with the same 182/82/100 frames, both allocation counters and strict wall gate.

Shared UI labels use pxui ABI 1.17 explicit line boxes. Preserve the actual bundled CoreText
width/baseline references and 1x/2x glyph-position checks. The player title wraps intrinsically;
its byline follows Swift's 15-point strut and baseline-aligned 14-point padded links, trims leading
wrapped whitespace and measures only in its retained metadata view. Natural and constrained
measurement identities must differ by width; reusing one identity causes repeated shaping.
Preserve accessible artist/album routing, empty-line height and warm resize allocation checks.


Albums has SwiftUI's 30-point Artist/Year/Size toolbar, 8-point gaps, fractional 1.5-point
selection border, 14-point bottom padding and one-point header clearance. The desktop first
artwork starts at y=172.6 with no window inset; preserve actual native SwiftUI control widths
52/47/44/100 and the fractional grid row stride across desktop widths. Prepare all three album
orders and size/source labels on the worker; Year uses the first ordered track, Size follows
Swift's Added ordering, and Artist strips only exact "The ". Sorting retains album identity and
navigation generations. Intrinsic SwiftUI Text widths ceil to physical pixels using Ui::density (ABI 1.19); explicit CoreText column
widths remain fractional. The byline uses pxui ABI 1.18 WRAP_WORD_SPACES to retain the logical
advance of omitted separators in fitting widths; preserve the native 100/279/400-point references.
Shuffle all hands the library Arc to the controller and shuffles its copy there, including the
first item and duplicate occurrences, through the existing Configure/lease/persistence path.
Keep ordinary Shuffle semantics separate. Preserve muted native queue/persistence and warm UI
checks. Reference shuffle paths use bounded analytic curves in the existing pipeline, with
less than 0.04-pixel cubic approximation deviation at 16/20 points and 1x/2x.


Albums uses pxui ABI 1.20 section_list; Songs and Artists use ABI 1.21 section_list_measured: page headings scroll with the content,
Artists column headings pin, and overlay thumbs reserve no content gutter. Retain the page
heading separately from changing playback/artwork revisions. Gallery and table rows keep their
existing identities/entrances, overscan and worker-prepared data. Preserve the full-width gallery
rule, 172.6/105.6-point lead-ins, 37.3-point normal heading, fractional row stride, and warm
accessible scrolling checks. Numeric cells and headings can wrap; use measured rows and baseline
alignment rather than forced truncation or fixed 38-point rows. Detail/Find headers and full
screenshot parity remain unfinished.

Prepare song column preferences, natural numeric widths, pins, action labels and per-song occurrence
keys on the library worker through prepare_tables. Frames select prepared maxima in bounded work;
never scan a collection or construct a text engine on the UI thread. Mixed formats compare original
case but render lowercase. Songs use stable occurrence scopes across sorting, and duplicate queue
entries keep distinct visual state. Playing bars preserve cached glyphs and request frames only
while visible/playing; reduced motion and pause settle. File-state pops use the shared 240 ms spring
without initial animation. Preserve native 1x/2x wrapped baselines, responsive thresholds, popup
staleness guards and warm allocation checks. Performance adds active indicators as a twelfth
workload; retain every original workload, both counters and the strict 1,000 us wall maximum.

Find summaries are prepared on the library worker. Keep song membership, current-account folder
counts and blocked-source metadata in one read snapshot; probe files afterward and reuse observed
availability. Count all owned indexed directory matches in one SQL query, excluding files/foreign
accounts. Album counts use native album/album-artist identity and localized text words. Frames
borrow the summary and retained eligible IDs; never scan sources or format summaries in a frame.

Bulk offline toggles use Command::TogglePins and LibraryWorker::toggle_pins. Validate the complete
selection, deduplicate IDs, recheck blocked/local state, preserve copied rows, and roll back failed
copy-request batches before changing preferences. Preferences::set_pins writes lwPinnedSongs and
advances cache protection once for the whole selection, under the existing recursive cache guard.
Background copies share the inbox-locked cancellation gate with waveforms, after playback work.
Resume persisted work at startup and retry on explicit RefreshCatalog; failures must settle without
a tight retry loop. Never start/pause playback for a pin action or delete original files on unpin.
Preserve the native copy/resume, snapshot-generation, rollback and blocked-source regressions.

Find's Play all/Keep offline toolbar uses SwiftUI's 30-point buttons, 8-point gap, 10-point side
padding, 16/18-point SVG icons and 15-point bottom padding including its full-width one-point rule.
Measure 13-point Host labels with distinct identities and ceil to physical pixels. These buttons
animate only their retained hover/press backgrounds for 140 ms; do not apply the key style's scale
effect. Borrow the prepared eligible IDs, clone their Arc into TogglePins, and leave copies and
preferences on the worker. Disable stale-query/empty/blocked actions. ABI 1.22 exposes accessible
text values and draws play as one analytic triangle; preserve native 1x/2x geometry, UTF-8 guards,
glyph retention, idling and zero warm allocations. The legacy Find input remains until its command
palette and scrolling query hero are implemented; the toolbar alone does not qualify Find parity.
