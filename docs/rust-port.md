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
| P6 | Fix measured hot paths; pass phase gate | doing | HEAD + pxui a7be2aa |
| P6a | Bind fixed renderer state once per pass | done | 89ee53a + pxui d10fb8c |
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

P6 text-cost checkpoint `make performance`: thirteen signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Frame spans cover input pump through
GPU draw return; build spans retain their earlier boundary. CPU p99 is gated for both; the existing
strict 1,000 µs build wall maximum is unchanged. All thirteen runs fail (make exits nonzero).
Bundle/signature/rpath/default-stack checks and settled idle smoke pass (504.2 ms).
Source: `/tmp/localwave-p6-text-costs-performance.log`; prior P1–P6a failures remain in commit evidence.

The original twelve cases have zero measured Rust allocations and pass the frame-82 audit.
LongScroll still exposes 202 allocations/frame and six new text layouts at frame 82 and during
measurement; its audit fails. It advances rows 240→537 in all 100 measured frames. No added warm-up,
pre-shaping or repeated wheel cycle hides cold work. End-to-end CPU also fails its gate.
Wall spans are recorded without attributing their gaps. Native/process allocations and physical-device
behavior remain open. Waveform retains 200 bars/nine release seeks; indicator behavior is verified.

Times are µs; CPU is p50/p99/max, wall is p50/p95/p99/max. Layouts count cache-miss shaping work.

| Workload | Build CPU | Build wall | Frame CPU | Frame wall | Alloc max | Layouts max | Frame 82 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| songs-original | 765/1028/1088 | 875/2731/4072/4513 | 1218/1588/1737 | 16611/18540/22534/33835 | 0 | 0 | pass |
| find-filters-title-descending | 649/864/972 | 706/3237/4299/4553 | 1071/1342/1421 | 16656/18578/20119/21263 | 0 | 0 | pass |
| queue-occurrences | 677/884/886 | 736/3287/3818/4158 | 1144/1436/1442 | 16654/18013/18635/18697 | 0 | 0 | pass |
| queue-carry-scroll-cancel | 522/912/916 | 543/1382/2035/4182 | 988/1468/1549 | 16652/19078/19737/22164 | 0 | 0 | pass |
| full-queue-playlist-editor | 734/1121/1149 | 903/3716/4412/4854 | 1257/1732/1760 | 16560/18119/18954/23460 | 0 | 0 | pass |
| albums-artwork | 611/1142/1282 | 664/2140/3585/3919 | 1186/1801/2051 | 16676/18983/19984/20297 | 0 | 0 | pass |
| waveform-scrub-and-scroll | 507/944/948 | 616/2951/3340/3843 | 1047/1427/1464 | 16620/18239/20195/20522 | 0 | 0 | pass |
| settings-menus-appearance | 317/529/565 | 338/693/1625/1817 | 792/2762/2905 | 16618/17408/18029/19734 | 0 | 0 | pass |
| files-indexed-directory | 328/451/465 | 340/1692/3001/3403 | 779/959/1348 | 16628/17337/18386/18439 | 0 | 0 | pass |
| external-drop-hover-and-cancel | 278/361/388 | 298/1937/2477/3230 | 693/858/868 | 16576/18968/19934/20180 | 0 | 0 | pass |
| artists-measured-columns | 684/1007/1099 | 775/2497/3681/3847 | 1128/1482/1666 | 16640/18450/19987/20354 | 0 | 0 | pass |
| songs-playing-indicator | 870/1222/1358 | 1072/3918/4179/4322 | 1314/1853/1946 | 16637/17995/19921/19921 | 0 | 0 | pass |
| songs-long-scroll | 1082/1438/1560 | 1175/2210/4443/5098 | 1529/1934/2046 | 16649/18627/20478/23300 | 202 | 6 | fail (6 layouts) |

### CPU profile

Separate signed `--performance-check --trace-frames` runs of the five P6a follow-up cases use the same
10,000-song, 1440×872, 182/82/100 fixture. All five fail the unchanged performance gate;
LongScroll also retains its allocation/cold-layout failure. Rank by mean thread CPU over the
100 measured frames, rather than wall time or an individual outlier. Profiling is opt-in.
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
| long-scroll | paint 551.28 | draw 319.50 | build 266.48 |

Sources: `/tmp/localwave-p6-bindings-trace-{queue-drag,song-indicators,waveform,artwork,long-scroll}.log`.
The corresponding full phase means, aggregate CPU/wall timings and failing exits are preserved in
P6a commit evidence. Earlier baselines remain in their corresponding commits.
Separate runs do not establish a controlled speedup. Correlated records confirm the measured CPU
phases fit inside their parent spans.

P6a reuses pipeline/atlas/sampler bindings across retained image segments within a pass. Each
segment still binds its current globals, instances and image; every split callback restores full
state, including callbacks retaining the same pass. No state cache crosses a draw call. All 45 GPU
cases pass, including exact pixels, blur splits, draw counts, dirty bytes and warm Rust allocations.
The original seven gallery growth/shrink frames and warm assertions remain; twenty additional
128-image measurements use the same isolated test command before and after the production edit.

| P6 fix / benchmark | Before mean draw CPU (µs) | After | Warm Rust allocations |
| --- | --- | --- | --- |
| P6a / 128 alternating images, 20 samples | 189.14 | 84.88 | 0 |

Sources: `/tmp/localwave-p6-bindings-{baseline,final}-images.log`. The full-suite run measured
81.30 µs separately. These microbenchmarks do not qualify full-frame latency or native allocations.
The 206-case LocalWave suite, dependency gate and signed iOS simulator build also pass;
iOS was built only. Strict renderer Clippy still reports existing warnings; its scoped check passes.

The current checkpoint adds fast measurement regressions without production changes. A 100-row,
20-sample cached-glyph case verifies identical instance bytes and zero warm Rust allocations.
Separate 64-label tests measure first layouts and assert unchanged metrics/no allocations on hits:
Pragmasevka title layouts averaged 7.66 µs with 1,611 total allocations; tabular-number layouts
averaged 3.73 µs with 2,359. These fixture measurements do not represent Host Grotesk/Geist Mono
frame totals. All 239 core/text cases and 206 LocalWave cases pass within their budgets.

The P6a CPU phase profile above still applies to the unchanged production code.

P6 remains open: fresh text shaping/layout allocations and latency still fail the full gate.
The microbenchmarks, cached-hit checks and CPU percentiles do not replace that gate.

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
  LongScroll now exposes cold text shaping and 202 measured Rust allocations/frame; its warm-up
  audit fails. Twelve visited-row workloads remain zero. End-to-end CPU also exceeds the new gate.
  The full strict latency gate still fails; cold shaping and native memory need evidence.
  A text/core edit check took 1.60 s, exceeding pxui's 1 s warm-edit target.
  P3 removes Now metadata string hashes and caches stable pages/rows across ticks; P6 hot-path work remains.
  Whole-framework Clippy has existing dependency/integration-test warnings; changed production,
  ABI/public tests and the full LocalWave workspace pass their scoped checks.
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
