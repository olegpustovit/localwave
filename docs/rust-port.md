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
| P4 | Worker-prepared catalog remapping | done | HEAD |
| P5 | Single controller-poll lock | todo | — |
| P6 | Fix measured hot paths; pass phase gate | todo | — |
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

P4 `make performance`: thirteen signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Frame spans cover input pump through
GPU draw return; build spans retain their earlier boundary. CPU p99 is gated for both; the existing
strict 1,000 µs build wall maximum is unchanged. All thirteen runs fail (make exits nonzero).
Bundle/signature/rpath/default-stack checks and settled idle smoke pass (500.3 ms).
Source: `/tmp/localwave-p4-performance.log`; prior P1–P3 failures remain in commit evidence.

The original twelve cases have zero measured Rust allocations and pass the frame-82 audit.
LongScroll still exposes 202 allocations/frame and six new text layouts at frame 82 and during
measurement; its audit fails. It advances rows 240→537 in all 100 measured frames. No added warm-up,
pre-shaping or repeated wheel cycle hides cold work. End-to-end CPU also fails its gate.
Wall spans are recorded without attributing their gaps. Native/process allocations and physical-device
behavior remain open. Waveform retains 200 bars/nine release seeks; indicator behavior is verified.

Times are µs; CPU is p50/p99/max, wall is p50/p95/p99/max. Layouts count cache-miss shaping work.

| Workload | Build CPU | Build wall | Frame CPU | Frame wall | Alloc max | Layouts max | Frame 82 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| songs-original | 790/1148/1180 | 862/2535/2670/3082 | 1290/1684/1757 | 16645/17798/18242/18770 | 0 | 0 | pass |
| find-filters-title-descending | 699/921/990 | 787/2515/2938/3113 | 1141/1416/1580 | 16628/17772/18727/26384 | 0 | 0 | pass |
| queue-occurrences | 561/839/874 | 648/1932/2780/2987 | 1014/1414/1499 | 16683/19041/21576/31982 | 0 | 0 | pass |
| queue-carry-scroll-cancel | 496/879/1028 | 527/1479/2458/4927 | 931/1355/1508 | 16669/19427/20575/20632 | 0 | 0 | pass |
| full-queue-playlist-editor | 671/1089/1093 | 762/2577/3182/3376 | 1136/1634/1733 | 16600/19191/19385/20431 | 0 | 0 | pass |
| albums-artwork | 598/985/1142 | 718/1573/2034/2506 | 1258/1728/1814 | 16651/18828/19903/20831 | 0 | 0 | pass |
| waveform-scrub-and-scroll | 420/989/1084 | 577/2504/3457/3904 | 907/1495/1580 | 16624/18718/19831/23346 | 0 | 0 | pass |
| settings-menus-appearance | 282/546/550 | 311/853/2277/2336 | 696/2901/2904 | 16604/18125/19177/19184 | 0 | 0 | pass |
| files-indexed-directory | 317/459/482 | 334/1410/2424/2540 | 738/914/992 | 16643/18449/19209/19240 | 0 | 0 | pass |
| external-drop-hover-and-cancel | 298/397/405 | 312/1270/2178/2301 | 766/951/987 | 16574/18804/19974/34480 | 0 | 0 | pass |
| artists-measured-columns | 655/1008/1054 | 730/2041/2581/3248 | 1124/1552/1624 | 16628/18815/20196/21266 | 0 | 0 | pass |
| songs-playing-indicator | 827/1212/1308 | 917/2408/2818/3650 | 1252/1673/1710 | 16611/18789/19948/20038 | 0 | 0 | pass |
| songs-long-scroll | 1224/1507/1587 | 1443/2728/3502/4480 | 1722/2108/2118 | 16582/17659/18580/19155 | 202 | 6 | fail (6 layouts) |

### CPU profile

Separate signed `--performance-check --trace-frames` runs of the five P4 follow-up cases use the same
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
| queue-drag | draw 314.78 | build 192.52 | paint 170.75 |
| song-indicators | paint 375.28 | draw 357.51 | build 277.59 |
| waveform | draw 378.47 | paint 224.03 | build 202.75 |
| artwork | draw 578.35 | build 320.42 | paint 204.38 |
| long-scroll | paint 646.45 | draw 377.11 | build 322.01 |

Sources: `/tmp/localwave-p4-trace-{queue-drag,song-indicators,waveform,artwork,long-scroll}.log`.
The corresponding full phase means, aggregate CPU/wall timings and failing exits are preserved in
P4 commit evidence. Earlier baselines remain in their corresponding commits.
Separate runs do not establish a controlled speedup. Correlated records confirm the measured CPU
phases fit inside their parent spans.

Stable `PlayerView` data remains separate from position. Worker delivery revisions still advance
on position ticks; stable revisions leave desktop/compact library and metadata views cached.
The 20-tick/10,000-song regression keeps seek/clock values live with no row rebuilds or warm Rust
allocations, then verifies that real metadata/current-track/pause changes invalidate retained views.
Signed fixtures still advance position each frame; waveform retains 200 bars/nine release-only seeks
and indicators retain 58 playing/34 paused/57 changing frames. No warm-up/input/assertions were omitted.

P4 prepares route and repeated-song occurrence remaps on the library worker. UI refresh validates
source/target identities and indexes prepared arrays; a 10,000-song/album/artist test bounds adoption
to three mapping probes with zero Rust allocations. Playlist renames, disappearing routes, duplicate
occurrences, sorted views, Find query identity and collection scroll positions remain covered.
Byline artist/album routes are prepared per song ID, including album-artist grouping; stale displayed
metadata cannot activate a mismatched group. Byline fitting/native geometry and warm checks still pass.

The mailbox retains a shared owner of the displayed catalog and reserves the next base before UI
repair, outside the repair callback's lock scope. Worker publication rechecks that base and reuses
prepared remaps for an unchanged coalesced page. Deterministic tests cover superseded deliveries and
publication during repair, plus existing retirement backpressure and worker-side final release.
Remaps use weak source/target identities so old label arenas are not retained by a snapshot chain.
Bulk preparation and superseded/retired storage release stay outside the mailbox lock on the worker.

P5 is next: reduce controller polling locks. P6 still needs framework draw and paint/text work,
including LongScroll's fresh layouts and allocations. The strict latency and cold-work failures
remain open; constant-time adoption and CPU percentiles do not replace the signed workload gate.

## Open qualification

- Physical USB iPhone/iPad signing, install, launch and 120 Hz touch/scroll behavior.
- Real background audio, interruptions, route changes and system media controls on devices.
- Live iCloud/Files providers: original picker grants, downloads, offline recovery, account changes,
  moved/expired bookmarks and reconnect. Local URL/synthetic checks do not prove provider behavior.
- Actual sandbox Finder drops, native full-screen/Spaces gestures and supported macOS runtime checks.
- Interactive IME/candidate selection, dictation, floating keyboard avoidance and device editing feel.
- Real-user-container migration, distribution provisioning, TestFlight delivery and a release cycle.

## Known gaps

- Performance P5–P6 precedes new feature work.
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
