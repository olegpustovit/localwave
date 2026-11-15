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
| P6 | Fix measured hot paths; pass phase gate | doing | — |
| P6a | Bind fixed renderer state once per pass | done | HEAD + pxui d10fb8c |
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

P6a `make performance`: thirteen signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Frame spans cover input pump through
GPU draw return; build spans retain their earlier boundary. CPU p99 is gated for both; the existing
strict 1,000 µs build wall maximum is unchanged. All thirteen runs fail (make exits nonzero).
Bundle/signature/rpath/default-stack checks and settled idle smoke pass (503.2 ms).
Source: `/tmp/localwave-p6-bindings-performance.log`; prior P1–P5 failures remain in commit evidence.

The original twelve cases have zero measured Rust allocations and pass the frame-82 audit.
LongScroll still exposes 202 allocations/frame and six new text layouts at frame 82 and during
measurement; its audit fails. It advances rows 240→537 in all 100 measured frames. No added warm-up,
pre-shaping or repeated wheel cycle hides cold work. End-to-end CPU also fails its gate.
Wall spans are recorded without attributing their gaps. Native/process allocations and physical-device
behavior remain open. Waveform retains 200 bars/nine release seeks; indicator behavior is verified.

Times are µs; CPU is p50/p99/max, wall is p50/p95/p99/max. Layouts count cache-miss shaping work.

| Workload | Build CPU | Build wall | Frame CPU | Frame wall | Alloc max | Layouts max | Frame 82 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| songs-original | 747/1027/1056 | 834/3470/4645/5313 | 1233/1646/1681 | 16635/18718/19863/20420 | 0 | 0 | pass |
| find-filters-title-descending | 676/874/893 | 742/2822/3312/4328 | 1108/1322/1373 | 16679/18007/19868/20497 | 0 | 0 | pass |
| queue-occurrences | 616/817/898 | 649/1616/2640/3886 | 1088/1369/45145 | 16666/19329/27466/75827 | 0 | 0 | pass |
| queue-carry-scroll-cancel | 531/993/1028 | 576/2258/3819/4529 | 983/1485/1507 | 16644/18342/20559/20605 | 0 | 0 | pass |
| full-queue-playlist-editor | 711/1081/1107 | 831/2867/3583/3721 | 1241/1586/1631 | 16627/17904/18732/18755 | 0 | 0 | pass |
| albums-artwork | 622/1072/1111 | 644/3158/3620/3903 | 1196/1672/1931 | 16645/18692/19690/19811 | 0 | 0 | pass |
| waveform-scrub-and-scroll | 339/946/1093 | 492/2579/3228/4039 | 836/1482/1623 | 16659/18250/20147/21057 | 0 | 0 | pass |
| settings-menus-appearance | 288/531/546 | 315/1489/2849/4387 | 685/2484/2512 | 16657/19682/20449/20776 | 0 | 0 | pass |
| files-indexed-directory | 333/559/588 | 347/817/3143/3601 | 766/1120/1137 | 16644/17901/20297/20495 | 0 | 0 | pass |
| external-drop-hover-and-cancel | 296/400/419 | 314/836/1246/1273 | 753/973/1019 | 16662/18976/20420/20692 | 0 | 0 | pass |
| artists-measured-columns | 712/1031/1087 | 793/3514/3989/4118 | 1164/1571/1657 | 16621/19085/20505/20621 | 0 | 0 | pass |
| songs-playing-indicator | 806/1098/1269 | 888/3457/3904/4719 | 1239/1653/1703 | 16655/18262/20507/20846 | 0 | 0 | pass |
| songs-long-scroll | 1066/1268/1348 | 1128/3827/4657/4819 | 1510/1745/1790 | 16654/18094/20297/24549 | 202 | 6 | fail (6 layouts) |

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

P6 remains open after the binding fix: paint/text work still includes LongScroll's fresh layouts
and allocations. The strict latency and cold-work failures remain open; these changes and CPU percentiles
do not replace the signed workload gate.

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
