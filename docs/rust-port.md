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
| B3a | Schedule plain-node loops through their retained owner | done | b4ec380 + pxui 48c55f8 |
| B4 | Framework scrub signal for seek/volume | done | 57cb610 |
| B5 | Framework virtual-list reveal | done | 6179982 |
| B6 | Framework completed file-drop events | done | 042b961 |
| B7 | Move profiling/lifetime checks; close boundary | done | 325d54a |
| P1 | End-to-end timing and LongScroll workload | done | d407e8a |
| P2 | Profile top frame costs | done | 360a5e3 |
| P3 | Isolate position ticks from stable content | done | d16e6a3 |
| P4 | Worker-prepared catalog remapping | done | 81ab1ca |
| P5 | Single controller-poll lock | done | 7bb51ac |
| P6 | Fix measured hot paths; pass phase gate | done | HEAD + pxui 520db20 |
| P6a | Bind fixed renderer state once per pass | done | 89ee53a + pxui d10fb8c |
| P6b | Worker-prepared text API and ownership | done | 8a934a3 + pxui 3231c8e |
| P6c | Bounded song-text delivery; remove LongScroll allocations | done | d67773d |
| P6d | Skip damage calculations inside an already covered view clip | done | 79cb6c6 + pxui 71dc0a1 |
| P6e | Reuse bounded fresh-row gaps before retained runs | done | ec54614 + pxui 796ba63 |
| P6f | Bound actual write ends when reusing retained storage | done | 962df25 + pxui 519e367 |
| P6g | Avoid recursive native background redraws under opaque Metal | done | 6614ab5 + pxui 8a9f05b |
| P6h | Borrow instances during retained-run compaction | done | 4f4ed00 + pxui 7556bee |
| P6i | Limit playback invalidation to affected song rows | done | 5f03419 |
| P6j | Borrow fresh instances during retained placement | done | ca17adc + pxui 766f6ff |
| P6k | Owner-approved M1 limits; gate frame work wall | done | d41fb42 + pxui 520db20 |
| F1 | Global shortcuts from one command table | done | af4b2e8 |
| F2 | Favorite commands, menu, key and heart | done | 504bf9f |
| F3 | Complete track menu and native reveal | done | 3a56ba9 |
| F4 | Playlist CRUD, membership and reorder | done | 72bb927 |
| F5 | Album/artist detail headers and actions | done | 1868431 |
| F6 | Occurrence-based multi-selection | done | 8ce3af5 |
| F7 | Drag songs to playlists and queue | done | 905149e |
| F8 | Metadata editor and metadata-only deletion | done | HEAD |
| F9 | Source observation and debounced rescan | done | 12c2408 + d7c9b87 |
| F10 | Onboarding, route empty states and Share | done | 92993cf |
| S | Sign, migrate, TestFlight and release macOS | todo | — |
| I1 | Framework gestures and device comparison | todo | — |
| I2 | iOS flows, physical install and shipping gates | todo | — |
| D | Resolve deferred accessibility/opacity decisions | todo | — |

## Latest performance

F-gate `make performance` (exit 0): eighteen signed workloads, 10,000 songs, 1440×872, 182 frames /
82 warm-up / 100 measured, both Rust counters. All eighteen pass with zero warm Rust allocations and
new layouts, and a passing frame-82 audit. Settled idle smoke passes (505.1 ms). The five new rows
drive album-detail and artist-detail scoped-table scrolls (album with real artwork), list menu and
New list dialog with bounded typing, metadata editor typing and a song carry over the table, a sidebar
list and Up next that Esc cancels. Typing is applied as the Intents a text field emits, because
TextInput events own Strings. Onboarding is not a workload: it needs an empty library, which
contradicts the 10,000-song fixture contract.
Source: `/tmp/localwave-fgate-performance.log`.

Gate (owner decision for this M1 Mac mini, 2026-11-15): build CPU p99 < 1,500 µs, frame CPU p99
< 2,500 µs, frame work wall p99 < 8,000 µs (half a 60 Hz vsync), zero warm allocations. Work wall is
the input-pump-to-draw-return span minus drawable/slot acquisition, which `pxui_host::Graphics`
reports. Build wall and work wall maxima are recorded, not gated.

Evidence for the decision: in a System Trace of LongScroll (`/tmp/localwave-p6-system-trace-long-scroll.trace`),
measured builds were never Blocked. All off-CPU time was Preempted (VM_compressor, WindowServer,
fseventsd, another app's display link, the GPU work loop), with load average 7–25 and 21 GB of
swap in use. 83% of build CPU ran on E cores. The same build costs a median 312 µs on P cores (max
547) and 896 µs on E cores (max 1,298). A foreground task-role experiment did not change residency.

Times are µs; CPU is p50/p99/max, build wall is p50/p95/p99/max, work wall is p50/p95/p99/max.

| Workload | Build CPU | Build wall | Frame CPU | Work wall | Alloc max | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| songs-original | 680/869/978 | 729/2422/3777/4527 | 1144/1389/1446 | 1147/3209/4379/4958 | 0 | pass |
| find-filters-title-descending | 693/957/984 | 804/3618/3959/4283 | 1186/1521/1526 | 1276/4474/4824/6057 | 0 | pass |
| queue-occurrences | 643/847/939 | 747/3033/3545/3857 | 1120/1477/1483 | 1186/3582/4714/5285 | 0 | pass |
| queue-carry-scroll-cancel | 564/974/1025 | 637/3161/3623/4252 | 1051/1501/1605 | 1133/3856/4770/5102 | 0 | pass |
| full-queue-playlist-editor | 708/1061/1069 | 873/2848/3972/5754 | 1242/1572/1639 | 1357/4147/6394/6537 | 0 | pass |
| albums-artwork | 638/1088/1128 | 710/2747/4898/5533 | 1229/1761/1850 | 1291/3929/5304/5972 | 0 | pass |
| waveform-scrub-and-scroll | 432/858/925 | 506/1995/3211/3895 | 917/1450/1454 | 881/3519/3958/4629 | 0 | pass |
| settings-menus-appearance | 311/510/534 | 385/2140/2645/2921 | 719/1009/1030 | 753/2501/3183/3290 | 0 | pass |
| files-indexed-directory | 351/483/492 | 374/636/1158/2327 | 820/1156/1176 | 729/1189/1500/2704 | 0 | pass |
| external-drop-hover-and-cancel | 341/451/460 | 370/1251/2824/3099 | 806/1041/1082 | 767/2715/4104/4130 | 0 | pass |
| artists-measured-columns | 641/982/1009 | 718/2880/3200/4227 | 1110/1625/1651 | 1124/3721/4263/4504 | 0 | pass |
| songs-playing-indicator | 783/1084/1096 | 874/3244/4944/5755 | 1279/1631/1643 | 1399/4624/5435/6030 | 0 | pass |
| songs-long-scroll | 831/1232/1266 | 897/3768/4277/4782 | 1263/1731/1754 | 1379/4461/5819/6438 | 0 | pass |
| album-detail-scoped-scroll | 536/758/785 | 572/2448/3906/4107 | 1037/1329/1369 | 1022/3708/4929/5407 | 0 | pass |
| artist-detail-scoped-scroll | 516/684/723 | 545/1232/3800/6386 | 934/1133/1169 | 874/2583/4055/6611 | 0 | pass |
| list-dialogs-menu-and-name | 425/1064/1118 | 466/1189/2031/3055 | 976/1650/1658 | 850/1835/3273/4925 | 0 | pass |
| metadata-editor-typing | 748/1231/1289 | 882/3047/4007/4407 | 1182/1693/1723 | 1359/4244/5240/5452 | 0 | pass |
| song-drag-carry-hover-cancel | 600/1335/1367 | 654/3143/4920/4946 | 1065/1827/1854 | 1047/4319/5188/5319 | 0 | pass |

### CPU profile

Separate signed `--performance-check --trace-frames` runs of the five P6a follow-up cases use the same
10,000-song, 1440×872, 182/82/100 fixture. All five fail the unchanged performance gate;
Rank by mean thread CPU over each run’s 100 measured frames, rather than wall time or an individual
outlier. Profiling is opt-in. LongScroll averages three P6j after runs; SongIndicators averages three
P6i after runs. Settings retains P6g, and the other three rows retain their P6a trace baseline. Settings additionally required input-pump attribution
outside the engine/output/draw phases; that work remains included in full-frame CPU.
The ABI 1.35 CPU clock sums every engine settling pass. Existing wall-phase times remain the
final pass. Output spans `platform.apply`; draw spans the public `Graphics::draw`, including
acquisition, preparation, recording, submission and image retirement. Other host/ABI/input work
remains in the full-frame total; these phase means do not exhaust it or measure GPU execution.
Raw CPU samples are retained with their corresponding wall/phase records before sorting copies.

| Workload | First CPU phase (µs) | Second | Third |
| --- | --- | --- | --- |
| queue-drag | draw 352.30 | build 206.62 | paint 175.45 |
| song-indicators | draw 318.05 | build 257.75 | paint 246.31 |
| waveform | draw 304.78 | paint 205.84 | build 175.21 |
| artwork | draw 448.75 | build 281.43 | paint 190.28 |
| long-scroll | draw 315.46 | build 272.14 | paint 255.85 |
| settings | draw 349.73 | paint 138.97 | build 99.37 |

Sources: `/tmp/localwave-p6-bindings-trace-{queue-drag,song-indicators,waveform,artwork,long-scroll}.log`.
SongIndicators uses `/tmp/localwave-p6-row-state-paired-{2,3,6}-after.log`.
LongScroll uses `/tmp/localwave-p6-place-paired-{2,3,6}-after.log`; all three also have zero
warm Rust allocations/layouts and passing frame-82 audits, while latency still fails. Settings
uses `/tmp/localwave-p6-background-settings-clean.log`, also zero/zero with a passing audit. Full phase
means, aggregate CPU/wall timings and failing exits are preserved in each corresponding commit. Earlier baselines remain in their corresponding commits.

Temporary signed host probes before P6j split draw CPU further. Means cover 100 measured frames;
acquisition includes drawable/command-buffer setup, encoding includes native pass creation and
renderer commands, and submission includes image retirement. They do not measure GPU execution.
The probes were removed before qualification.

| Workload | Acquire CPU (µs) | Prepare | Encode | Submit |
| --- | --- | --- | --- | --- |
| long-scroll | 128.16 | 9.21 | 144.91 | 40.78 |
| queue-sheet | 121.29 | 8.26 | 146.09 | 40.70 |
| artwork | 118.72 | 9.62 | 234.67 | 44.30 |
| settings | 143.06 | 10.62 | 143.94 | 47.03 |
| song-indicators | 126.69 | 8.91 | 142.88 | 43.00 |

A second probe separates pass creation/renderer encoding: LongScroll 73.13/62.64 µs and Artwork
82.23/184.42 µs. Sources: `/tmp/localwave-p6-gpu-cost-{long-scroll,queue-sheet,artwork,settings,song-indicators}.log`
and `/tmp/localwave-p6-gpu-encode-cost-{long-scroll,artwork}.log`. Subphase sums match outer draw CPU
within 5 µs; these diagnostic runs retain zero warm Rust allocations and their timing failures.

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
| P6h / 20×64 tiles, three new rows/frame, mean paint, 100 samples | 33.26 | 22.55 | 0 |
| P6h / 20×64 tiles, nineteen new rows/frame, mean paint, 100 samples | 70.53 | 45.85 | 0 |
| P6i / SongIndicators playback transitions, full-frame CPU maximum, 3×5 samples | 1862 | 1385 | 0 |
| P6j / 20×64 tiles, three new rows/frame, warm repeat mean paint, 100 samples | 20.78 | 18.69 | 0 |
| P6j / 20×64 tiles, nineteen new rows/frame, warm repeat mean paint, 100 samples | 45.35 | 40.23 | 0 |
| P6j / 20×64 unchanged freshly rebuilt tiles, warm repeat mean paint, 100 samples | 23.19 | 20.11 | 0 |

P6j's initial rebuild measurements and signed LongScroll comparisons remain mixed; its warm
microbenchmark repeats do not establish a controlled whole-app speedup. Failure evidence remains
in the commit. The unchanged-fresh case also checks no dirty uploads, exact tiles/geometry and
bounded storage without changing the existing scrolling cases' warm-up.

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

P6h follows a temporary signed LongScroll probe: mean paint CPU splits into visit/paint 143.84,
retained placement 131.89, view refresh 30.02 and settling 12.29 µs over 101 post-warm-up passes.
The probe and sample evidence are in `/tmp/localwave-p6-hot-{paint-probe.log,long-sample.txt}`;
they were removed before qualification. Compaction now compares borrowed 64-byte instances rather
than copying both into temporary Options. Equality, damage, dirty ranges, moves and ownership are
unchanged. The existing scrolling regression checks every row/tile, retained footer, geometry,
zero warm allocations, the same 50/100 compactions and bounded 2,049/2,561-instance storage.
Before/after: `/tmp/localwave-p6-merge-{before,borrowed}.log`; a warm repeat measures 20.82/45.49 µs.
Release test execution is 0.032/0.029/0.051 s; first release rebuilds take 53.18/50.81 s,
while the repeat builds in 0.07 s. No profile or test deadline changed.
The full signed matrix retains slower results too: LongScroll build CPU p99 1197→1217 µs,
full-frame p99 1760→1798 µs, compared with P6g. The separate clean P6h trace measures
778/1061/1204 µs build CPU and 1214/1577/1688 µs frame CPU; wall maximum 4,587 µs still fails.
Mean paint is 264.58 µs, but all phases vary between runs; the isolated compaction benchmark
supports the narrower improvement claim. All 13 allocation/layout/audit checks pass unchanged.
P6h verification: 440 framework cases pass in 25.623 s wall (19.353 s case sum);
209 LocalWave cases pass in 9.417 s wall (13.792 s case sum), zero skips. Five/four
existing cases pass on the runner’s isolated contention retry. Native pixels, retained geometry,
input, text, ownership, allocation, bundle and dependency/diff checks pass.

B3a corrects the scheduling predicate for a plain node inside a kept view: its retained instance
range and owner visibility remain meaningful even when it was not painted again this frame.
Visible loops keep requesting frames; clipped or transparent owners do not. Root-owned nodes keep
their existing behavior. The native regression checks every pixel at 1×/2×, one body build across
18 changing frames, and zero warm Rust allocations; it fails on the previous implementation.
441 framework cases pass in 26.859 s; 209 app cases pass in 14.257 s, zero skips, with six/fifteen
existing isolated contention retries. The new native case takes 0.613 s in the full suite.
B3a changes no renderer, layout, cache storage, ABI or application code.

The overscan-retention experiment was **rejected** after alternating baseline/candidate runs.
Fewer row builds traded against more retained paint and accessibility work. Phase means below
average three 100-frame runs; frame percentiles list each run separately, never an averaged percentile.

| LongScroll CPU (µs) | Baseline | Rejected candidate |
| --- | --- | --- |
| Build phase mean | 287.76 | 255.25 |
| Paint phase mean | 277.19 | 296.66 |
| Accessibility phase mean | 81.64 | 86.42 |
| Frame p50, each run | 1387 / 1260 / 1273 | 1310 / 1281 / 1326 |
| Frame p99, each run | 1796 / 1638 / 1651 | 1650 / 1664 / 1831 |

All six runs fail the unchanged gate, with zero warm allocations/layouts and ready audits.
Sources: `/tmp/localwave-p6-overscan-paired-{1,4,5}-baseline.log` and
`/tmp/localwave-p6-overscan-paired-{2,3,6}-candidate.log`. The 13-case candidate and restored
matrices, slower runs and prototype test failures remain in commit evidence. Cache warming,
ancestor bookkeeping, short-run padding and eager hit buffers are all reverted; only the independent
B3a scheduling correction is kept. Its clean LongScroll trace still fails: build CPU 790/1090/1336 µs,
frame CPU 1285/1577/1886 µs and build wall maximum 6,106 µs. No timing improvement is claimed for B3a.

P6i separates the shared catalog/geometry revision from each row’s current-song flag. Only its
current visual depends on play/pause; text stays cached. Track changes update the old/new current
rows, while width changes still invalidate all affected geometry. The public-API regression alternates
play/pause and current IDs across eight visible songs: eight row builds previously, now one/two,
zero copy builds on play/pause, correct accessible indicator placement and zero warm Rust allocations.
All eight song/table cases pass in 3.480 s; `make check` passes 210 cases in 13.827 s, zero skips,
with two existing isolated retries. The new case takes 0.031 s. No framework/ABI change is needed.

Alternating signed before/after runs retain unchanged clocks, wheel input and 182/82/100 frames.
All 15 predefined playback transitions (frames 101/121/141/161/181 across three runs) have build CPU
below 1 ms after the fix; their full-frame CPU maximum falls 1862→1385 µs. Four after-transition
frames are below the existing outlier-print threshold (both build wall and full-frame CPU <1000 µs);
all other transition records are retained. This does not replace whole-workload percentiles: before
build CPU p99 is 1259/1172/1077 µs, after is 1051/987/1155 µs. All six runs still fail the full gate.
Sources: `/tmp/localwave-p6-row-state-paired-{1,4,5}-before.log` and `{2,3,6}-after.log`;
`/tmp/localwave-p6-row-state-transitions.json` retains every logged transition and the bounded four.
The separate final LongScroll trace has build CPU 763/949/968 µs, frame CPU 1200/1455/1465 µs and
build wall maximum 3,809 µs (fail). Earlier slower runs remain recorded; native memory is separate.

P6 closes under the owner-approved M1 limits (P6k). Native/process allocation and real
device/provider behavior remain open.

## Open qualification

- Physical USB iPhone/iPad signing, install, launch and 120 Hz touch/scroll behavior.
- Real background audio, interruptions, route changes and system media controls on devices.
- Live iCloud/Files providers: original picker grants, downloads, offline recovery, account changes,
  moved/expired bookmarks and reconnect. Local URL/synthetic checks do not prove provider behavior.
- Actual sandbox Finder drops, native full-screen/Spaces gestures and supported macOS runtime checks.
- Interactive IME/candidate selection, dictation, floating keyboard avoidance and device editing feel.
- Real-user-container migration, distribution provisioning, TestFlight delivery and a release cycle.

## Known gaps

- Performance: all thirteen workloads pass the P6k limits. P-core builds stay near 300 µs;
  further engine CPU work (paint visit, placement, compaction, a11y emit) remains worthwhile.
  Cold glyphs, other scripts/native font discovery and native/process memory need separate evidence.
  A text/core edit check took 1.60 s, exceeding pxui's 1 s warm-edit target.
  P3 removes Now metadata string hashes and caches stable pages/rows across ticks; P6 hot-path work remains.
  Whole-framework Clippy has existing dependency/integration-test warnings; changed production,
  ABI/public tests and the full LocalWave workspace pass their scoped checks.
- Framework `repaint_node` resets accumulated damage for each between-frame patch; multiple patches
  before one draw need a separate correctness fix. Dirty upload ranges still accumulate.
- Shared Swift checks fail two child source-notification tests, also in an isolated run on unchanged
  Swift sources. Six Swift/Rust exchange cases pass. Investigated in F9 on macOS 27.0.1: a file
  presenter gets coordinated writes reliably, but a directory presenter receives subitem
  notifications only intermittently (about 1 in 12 coordinated writes, in-process or from a child
  process, in /tmp, /Users/Shared and $HOME; registration delay and run loop servicing do not
  matter). An in-process directory presenter also makes the app's own coordinated reads under it
  wait for presenter registration (75 ms–4.9 s measured, 1–4 ms without), so macOS source
  observation uses FSEvents instead (started and released on its own queue; drop never blocks).
  Real FSEvents delivery is on demand only: this host's fseventsd ran at ~100% CPU with 5 GB
  resident and delivered in 2 of 6 standalone probes, with stream start taking 53 ms–5.3 s. iOS
  keeps an NSFilePresenter, unqualified until I2.
- F3: the submenu chevron and group dividers are absent.
- F4: the compact (narrow) layout has no New list button or list menu (⌘N and the track menu work);
  the sidebar list menu opens on secondary press only (no visible actions button); renaming the
  playing list leaves its queue context label stale until playback is reconfigured; Swift's list
  menu extras (Play, Shuffle, Duplicate, Keep offline) are not ported. Deleting a list returns to
  Songs because routes address lists by position.
- F5: album format/folder lines, artist album grid and responsive big play keys remain Swift-only.
- F7: compact layouts have no Up next panel to drop on (the modal queue sheet hides the rows); the Up next target is the whole right panel with a signal outline; only song rows start a song drag (not albums, artists or Find rows); a list drop is AddToList with no confirmation notice.
- F8: the editor omits disc number (kept untouched) and has no compact-layout scrolling; Delete Album
  (Swift's album-cell menu) has no Rust album menu or command; deleting a queued song keeps its queue
  occurrence, as source removal does. Edit info and Delete act on the clicked song only.
- F9: folder observers start with the lazy Files worker (first Files use), not at app launch; moved-root
  notices only trigger a rescan, bookmark re-resolution handles the new location.
- F10: onboarding omits the animated MatrixText counter (static "00"), the iOS Google Drive disclosure and
  the demo-library installer ("Explore the library" only leaves onboarding; the gate is in-memory, not
  Swift's persisted `didStart`, so an empty library re-onboards on relaunch). Empty library text replaces
  "No songs here yet." on Songs; compact Now omits its empty hint. Share… is in the track menu only (macOS,
  NSSharingServicePicker anchored at the click): Rust Files has no per-file menu, so Swift's Files Share and
  iOS UIActivityViewController remain. Share copies run on the LibraryWorker background slot (blocking
  playback preparation for large files); temp copies are swept after a day, not on picker completion.
- F1: Swift's Esc that cancels folder selection has no Rust counterpart (Rust Files has no selection mode).
  Return does not confirm the metadata editor or the Rust-only delete-song dialog (Swift's editor has
  no default action; its song delete has no dialog). Delete (Backspace) removes from a list route only.
- macOS parity gate: complete 1440×872 reference goldens, Swift shortcut parity and new-screen workloads.
- S: Rust macOS signing, migration/release pipeline and a regression-free release cycle before Swift removal.
- I1–I2: remaining framework gestures, swipe/long-press/refresh app flows, player morph/sheets,
  390×778 goldens and device/background/provider qualification before iOS shipping.
- D: VoiceOver bridges, group opacity and CoreText-exact antialiasing remain deferred;
  decide accessibility release policy before S. Keep shipping Swift targets until their gates pass.
