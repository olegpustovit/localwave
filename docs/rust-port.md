# Rust port status

## Steps

Follow the [implementation plan](https://artifacts.microapps.space/a/w474jy87pnxg#plan) in order.
The next unfinished step is the first row marked todo/doing. `HEAD` identifies this status
checkpoint; the next step replaces it with its fixed commit hash. Evidence lives in commit messages.

| ID | Title | State | Commit |
| --- | --- | --- | --- |
| C0.1 | Durable rules; contracts beside code | done | bd31587 |
| C0.2 | One-page status | done | ee9f024 |
| C1.0 | Lossless manifest-driven Rust split tool | done | 4e9ceec + 2627081 + bc03270 |
| C1.1 | Move lw_ui lib tests out | done | 26817c6 |
| C1.2 | Split shell into screen modules | done | 9e52567 |
| C1.3 | Split song table, rows, menus and tests | done | 76a5c71 |
| C1.4 | Extract performance harness (a–c) | done | bf16b90 |
| C1.4a | Parse one Workload enum | done | e93d3da |
| C1.4b | Extract performance setup | done | 7da3d02 |
| C1.4c | Extract driver/report and production App | done | bf16b90 |
| C1.5 | Split controller and command families | done | 8a0a83e |
| C1.6 | Reasonable module boundaries; no line-count gate | done | d85b8f8 |
| C2 | Labels, module description and test contention | done | 0ea2899 |
| B1 | Extract SQLite; remove core Apple dependency | done | 77e86c6 |
| B2 | Framework text identity and pixel rounding | done | 5426f94 |
| B2a | Keyless measurement and bounded layout cache | done | 7ad09dc |
| B2b | Pixel Fit layout and public fast identities | done | 5426f94 |
| B3 | Repeating retained keyframes and indicators | done | f7c573a |
| B4 | Framework scrub signal for seek/volume | done | 57cb610 |
| B5 | Framework virtual-list reveal | done | 6179982 |
| B6 | Framework completed file-drop events | done | 042b961 |
| B7 | Move profiling/lifetime checks; close boundary | done | 325d54a |
| P1 | End-to-end timing and LongScroll workload | done | d407e8a |
| P2 | Profile top frame costs | done | 360a5e3 |
| P3 | Isolate position ticks from stable content | done | d16e6a3 |
| P4 | Worker-prepared catalog remapping | done | 81ab1ca |
| P5 | Single controller-poll lock | done | 7bb51ac |
| P6 | Fix measured hot paths; pass phase gate | doing | HEAD + pxui 8a9f05b |
| P6a | Bind fixed renderer state once per pass | done | 89ee53a + pxui d10fb8c |
| P6b | Worker-prepared text API and ownership | done | 8a934a3 + pxui 3231c8e |
| P6c | Bounded song-text delivery; remove LongScroll allocations | done | d67773d |
| P6d | Skip damage calculations inside an already covered view clip | done | 79cb6c6 + pxui 71dc0a1 |
| P6e | Reuse bounded fresh-row gaps before retained runs | done | ec54614 + pxui 796ba63 |
| P6f | Bound actual write ends when reusing retained storage | done | 962df25 + pxui 519e367 |
| P6g | Avoid recursive native background redraws under opaque Metal | done | HEAD + pxui 8a9f05b |
| F1 | Global shortcuts from one command table | todo | — |
| F2 | Favorite commands, menu, key and heart | todo | — |
| F3 | Complete track menu and native reveal | todo | — |
| F4 | Playlist CRUD, membership and reorder | todo | — |
| F5 | Album/artist detail headers and actions | todo | — |
| F6 | Occurrence-based multi-selection | todo | — |
| F7 | Drag songs to playlists and queue | todo | — |
| F8 | Metadata editor and metadata-only deletion | todo | — |
| F9 | Source observation and debounced rescan | todo | — |
| F10 | Onboarding, route empty states and Share | todo | — |
| S | Sign, migrate, TestFlight and release macOS | todo | — |
| I1 | Framework gestures and device comparison | todo | — |
| I2 | iOS flows, physical install and shipping gates | todo | — |
| D | Resolve deferred accessibility/opacity decisions | todo | — |

## Latest performance

P6g native background updates `make performance`: thirteen signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Frame spans cover input pump through
GPU draw return; build spans retain their earlier boundary. CPU p99 is gated for both; the existing
strict 1,000 µs build wall maximum is unchanged. All thirteen runs fail (make exits nonzero).
Bundle/signature/rpath/default-stack checks and settled idle smoke pass (508.5 ms).
Source: `/tmp/localwave-p6-background-performance.log`; prior P1–P6f failures remain in commit evidence.

All thirteen cases have zero measured Rust allocations/new UI text layouts and pass the frame-82
audit. LongScroll formerly exposed 202 allocations/frame and six new layouts; bounded worker
delivery removes both. It still advances rows 240→537 in all 100 measured frames. The original
182/82/100 frames and wheel trace are unchanged. End-to-end CPU also fails its gate.
Wall spans are recorded without attributing their gaps. Native/process allocations and physical-device
behavior remain open. Waveform retains 200 bars/nine release seeks; indicator behavior is verified.

Times are µs; CPU is p50/p99/max, wall is p50/p95/p99/max. Layouts count cache-miss shaping work.

| Workload | Build CPU | Build wall | Frame CPU | Frame wall | Alloc max | Layouts max | Frame 82 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| songs-original | 682/978/1014 | 757/2984/4976/6746 | 1170/1551/1571 | 16640/17768/18830/19836 | 0 | 0 | pass |
| find-filters-title-descending | 616/845/894 | 689/2641/3468/3603 | 1065/1370/1430 | 16639/18715/20414/25194 | 0 | 0 | pass |
| queue-occurrences | 646/897/1014 | 708/2969/5058/6229 | 1112/1429/1432 | 16621/18265/21558/32826 | 0 | 0 | pass |
| queue-carry-scroll-cancel | 516/915/951 | 540/2299/3678/3802 | 974/1433/1461 | 16634/18490/19597/20834 | 0 | 0 | pass |
| full-queue-playlist-editor | 670/1096/1184 | 821/3157/3800/3978 | 1155/1644/1646 | 16624/19410/20235/20248 | 0 | 0 | pass |
| albums-artwork | 611/1247/1423 | 642/1539/2974/3404 | 1198/1885/2311 | 16654/17552/18723/20138 | 0 | 0 | pass |
| waveform-scrub-and-scroll | 351/850/858 | 516/2315/3114/4486 | 868/1358/1359 | 16636/18721/19273/20451 | 0 | 0 | pass |
| settings-menus-appearance | 326/596/626 | 392/1228/2957/3435 | 767/1157/1227 | 16631/18298/19967/20086 | 0 | 0 | pass |
| files-indexed-directory | 347/487/500 | 384/2577/2793/3095 | 809/1059/1102 | 16662/18441/19831/20151 | 0 | 0 | pass |
| external-drop-hover-and-cancel | 314/466/518 | 359/1699/3454/3736 | 782/1099/1155 | 16618/18543/20724/20845 | 0 | 0 | pass |
| artists-measured-columns | 667/997/1039 | 790/2750/4044/5228 | 1115/1463/1501 | 16673/19891/21300/23677 | 0 | 0 | pass |
| songs-playing-indicator | 783/1253/1655 | 1078/3587/4455/5837 | 1245/1828/2293 | 16649/18322/20877/21139 | 0 | 0 | pass |
| songs-long-scroll | 848/1197/1276 | 1083/3705/5497/5706 | 1328/1760/1760 | 16666/20120/22018/22344 | 0 | 0 | pass |

### CPU profile

Separate signed `--performance-check --trace-frames` runs of the five P6a follow-up cases use the same
10,000-song, 1440×872, 182/82/100 fixture. All five fail the unchanged performance gate;
Rank by mean thread CPU over the 100 measured frames, rather than wall time or an individual
outlier. Profiling is opt-in. LongScroll retains the P6f trace, Settings uses P6g, and the other
four rows retain their P6a trace baseline. Settings additionally required input-pump attribution
outside the engine/output/draw phases; that work remains included in full-frame CPU.
The ABI 1.35 CPU clock sums every engine settling pass. Existing wall-phase times remain the
final pass. Output spans `platform.apply`; draw spans the public `Graphics::draw`, including
acquisition, preparation, recording, submission and image retirement. Other host/ABI/input work
remains in the full-frame total; these phase means do not exhaust it or measure GPU execution.
Raw CPU samples are retained with their corresponding wall/phase records before sorting copies.

| Workload | First CPU phase (µs) | Second | Third |
| --- | --- | --- | --- |
| queue-drag | draw 352.30 | build 206.62 | paint 175.45 |
| song-indicators | paint 376.62 | draw 350.42 | build 300.93 |
| waveform | draw 304.78 | paint 205.84 | build 175.21 |
| artwork | draw 448.75 | build 281.43 | paint 190.28 |
| long-scroll | draw 368.13 | build 319.65 | paint 313.18 |
| settings | draw 349.73 | paint 138.97 | build 99.37 |

Sources: `/tmp/localwave-p6-bindings-trace-{queue-drag,song-indicators,waveform,artwork,long-scroll}.log`.
The LongScroll trace is `/tmp/localwave-p6-budget-long-trace.log`; it also has zero
warm Rust allocations/layouts and a passing frame-82 audit, while latency still fails. Settings
uses `/tmp/localwave-p6-background-settings-clean.log`, also zero/zero with a passing audit. Full phase
means, aggregate CPU/wall timings and failing exits are preserved in each corresponding commit. Earlier baselines remain in their corresponding commits.
Separate runs do not establish a controlled speedup. Correlated records confirm the measured CPU
phases fit inside their parent spans.

P6c prepares at most 256 surrounding song rows from actual measured-list callbacks. One persistent
worker has one coalesced catalog request, 64 deliveries and bounded retirement slots. The public
pxui API owns shaping/cache adoption; the app selects music sources and sizes. Cancelled/superseded
catalogs and replaced/rejected layouts retire on the worker outside its mutex. Stale fonts trigger
a fresh fork; compact views cancel the desktop window. Shared artist/album/duration deliveries are
deduplicated. Late preparation uses normal rendering, preserving immediate input and complete rows.
The existing body styles/values are shared verbatim; numeric wrapping, current weight, album
numbers and display-density rounding preserve exact normal draw instances. Adoption and requests
are inside the measured build. Cold glyphs and worker/native/process allocations remain separate
from the two warm UI-thread Rust counters.

| P6 fix / measured scope | Before CPU (µs) | After CPU (µs) | Warm Rust allocations |
| --- | --- | --- | --- |
| P6a / 128 alternating images, mean draw, 20 samples | 189.14 | 84.88 | 0 |
| P6b / 64 fresh labels, main-thread batch | 713.38 | 134.92 | 4,043 → 0 |
| P6c / signed LongScroll build, p50/p99/max | 1144/1549/1610 | 919/1243/1263 | 202 → 0 |
| P6c / signed LongScroll full frame, p50/p99/max | 1589/2056/2170 | 1395/1729/1735 | 202 → 0 |
| P6d / 100 glyph rows with a covering background, mean paint, 28 samples | 256.59 | 129.65 | 0 |
| P6e / 20×64 tiles, three new rows/frame, mean paint, 100 samples | 52.31 | 32.04 | 0 |
| P6f / 20×64 tiles, nineteen new rows/frame, mean paint, 100 samples | 53.90 | 72.66 | 0 |
| P6g / precreated native colors, mean setter CPU, 100 calls | 3.804 | 0.066 | — |
| P6g / signed Settings full-frame CPU p99 | 2334 | 1157 | 0 |

P6c also removes six new UI layouts/frame and changes the frame-82 audit from fail to pass.
Before: `/tmp/localwave-p6-prepared-performance.log`; after:
`/tmp/localwave-p6-song-text-performance.log`. Its first isolated run, before shared-label delivery
deduplication, measured 1037/1226/1259 µs build CPU with zero allocations/layouts. The final independent
trace measured 989/1293/1316 µs, also zero allocations/layouts; its latency failure remains recorded.
The final rebuilt binary again reports zero allocations/layouts and a passing audit, with
1002/1265/1272 µs build CPU and a failing 4,679 µs wall maximum (`/tmp/localwave-p6-song-text-final-long.log`).
Separate-run timing variation never replaces a threshold. No warm-up extension, all-library
pre-shaping or dropped input was used.

P6a sources: `/tmp/localwave-p6-bindings-{baseline,final}-images.log`; its full-suite mean was
81.30 µs. P6b source: `/tmp/localwave-p6-prepared-final-focused.log`; separate worker CPU was
707.42 µs, so it demonstrates shifting work, not total CPU reduction. Its earlier isolated
main/after/worker values were 717.58/136.96/711.04 µs and full-suite values 548.04/102.83/569.79 µs.
Pragmasevka microbenchmarks do not represent production-font full frames. ABI ownership, descriptor
validation and glyph parity are covered by the 397-case core/text/ABI/static suite and focused
static/dynamic tests in the P6b commit. ABI history lives in `pxui/docs/abi.md`.

P6c verification: all 209 LocalWave cases pass in 15.700 s, zero skips. Checks cover exact cell instances,
bounded queue fill before any poll, allocation-free adoption/coalescing, stale-font retries,
resize/cancellation and worker-side catalog retirement. Production Clippy and dependency checks pass.
Signed bundle/signature/rpath/default-stack checks pass. The aggregate framework gate retains
existing shape-policy violations in unchanged code; earlier failures remain in their commits.

P6d skips instance damage work once the complete transformed view clip is covered, preserving
uploads and exact instances. P6e leaves room for another fresh batch before each retained run;
its total extra padding is at most the scratch instance count, so actual retained length stays
inside the existing `2 * packed` budget and existing reserved capacity. No new state or ABI.
P6e reduces compactions for three new rows/frame from 100 to 50 per 100 frames. P6f checks
actual fresh write ends against the same budget instead of adding all fresh data to old storage;
this also reduces the 19-of-20 replacement case from 100 to 50 compactions. Largest retained
length stays 2,561 under its unchanged 2,690 bound (three-row case: 2,049 under 3,714).
The regression preserves exact row order/geometry, footer, dirty bounds and zero warm allocations.
Raw nineteen-row mean paint CPU is 53.90→72.66 µs in separate runs; three-row control is
31.12→42.82 µs. These measurements do not establish a CPU improvement. Failed baseline assertions
and slower timings remain in commit evidence. All signed latency gates still fail in the matrix.
P6f verification: 401 framework cases pass in 13.112 s; 209 LocalWave cases pass in 10.369 s,
zero skips, with one/five existing isolated contention retries. Dependency and diff checks pass.
The separate clean signed LongScroll trace also fails: build CPU 893/1166/1199 µs, wall maximum
3,886 µs, zero allocations/layouts and a passing frame-82 audit.

P6g found the Settings outliers in the input pump, not drawing: frames 99/139/179 spent
1,509/2,629/1,916 µs there. Sampling captured `setBackgroundColor:` recursively invalidating
native views, then AppKit display work during SDL pumping. Opaque, full-content Metal windows
now retain a window-owned dynamic fallback color; updates avoid that recursive invalidation.
The provider keeps no window/layer reference, uses a lock for concurrent color resolution and
releases at window teardown. Other window paths retain the Cocoa setter. No public UI ABI changed.
The native regression checks eight RGBA colors under both appearances, legacy frame-raster parity
after resizing, partial/transparent/non-Metal refusal, idempotence and weak teardown. The SDK test
covers real background updates, resize, dead tokens and zero warm Rust allocations. The native
microbenchmark excludes color creation and deferred redraw; allocation claims come from API/frame
checks. Full Settings p99 is 2,334→1,157 µs across signed matrices; these remain separate runs.
Final clean Settings: build CPU 340/628/631 µs, full-frame CPU 822/1117/1135 µs; wall maximum
2,816 µs still fails. Both Rust counters/layouts remain zero, with all eight themes exercised.
P6g verification: 440 framework cases pass in 18.471 s; 209 LocalWave cases pass in 16.710 s,
zero skips, with two/four existing isolated retries. Dependency/diff and bundle checks pass.
Temporary GPU/App probes were removed. Earlier timings/failures remain in commit evidence.

P6 remains open: allocation/layout misses are fixed in the measured matrix, but CPU/wall latency
still fails the full gate. Native/process allocation and real device/provider behavior remain open.

## Open qualification

- Physical USB iPhone/iPad signing, install, launch and 120 Hz touch/scroll behavior.
- Real background audio, interruptions, route changes and system media controls on devices.
- Live iCloud/Files providers: original picker grants, downloads, offline recovery, account changes,
  moved/expired bookmarks and reconnect. Local URL/synthetic checks do not prove provider behavior.
- Actual sandbox Finder drops, native full-screen/Spaces gestures and supported macOS runtime checks.
- Interactive IME/candidate selection, dictation, floating keyboard avoidance and device editing feel.
- Real-user-container migration, distribution provisioning, TestFlight delivery and a release cycle.

## Known gaps

- Performance P6 precedes new feature work.
  All thirteen workloads now have zero measured warm UI-thread Rust allocations/new layouts,
  including LongScroll, and pass their frame-82 audit. CPU/wall latency still exceeds its gates.
  Cold glyphs, other scripts/native font discovery and native/process memory need separate evidence.
  A text/core edit check took 1.60 s, exceeding pxui's 1 s warm-edit target.
  P3 removes Now metadata string hashes and caches stable pages/rows across ticks; P6 hot-path work remains.
  Whole-framework Clippy has existing dependency/integration-test warnings; changed production,
  ABI/public tests and the full LocalWave workspace pass their scoped checks.
- Framework `repaint_node` resets accumulated damage for each between-frame patch; multiple patches
  before one draw need a separate correctness fix. Dirty upload ranges still accumulate.
- Shared Swift checks fail two child source-notification tests, also in an isolated run on unchanged
  Swift sources. Six Swift/Rust exchange cases pass; native notification delivery remains unresolved.
- F1: global Space/S/R/I/L, open/find/new-list, navigation and seek shortcuts.
- F2: favorite toggle entry points and retained favorite burst.
- F3: Play next, playlist submenu, album/artist routing, native reveal, per-song offline/retry,
  contextual removal and right-click track menus.
- F4: create/rename/delete playlists, membership editing and drag reorder; preserve active playback.
- F5: album/artist artwork headers, prepared summaries and Play/Shuffle actions.
- F6–F7: multi-select, range/select-all behavior and song drops to playlists/queue.
- F8: metadata editor, transactional song/FTS updates and deletion without removing original files.
- F9: native source observation and worker-debounced automatic rescans.
- F10: onboarding, route-specific empty states and native sharing.
- macOS parity gate: complete 1440×872 reference goldens, Swift shortcut parity and new-screen workloads.
- S: Rust macOS signing, migration/release pipeline and a regression-free release cycle before Swift removal.
- I1–I2: remaining framework gestures, swipe/long-press/refresh app flows, player morph/sheets,
  390×778 goldens and device/background/provider qualification before iOS shipping.
- D: VoiceOver bridges, group opacity and CoreText-exact antialiasing remain deferred;
  decide accessibility release policy before S. Keep shipping Swift targets until their gates pass.
