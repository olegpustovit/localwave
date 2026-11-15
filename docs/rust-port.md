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
| P5 | Single controller-poll lock | done | HEAD |
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

P5 `make performance`: thirteen signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Frame spans cover input pump through
GPU draw return; build spans retain their earlier boundary. CPU p99 is gated for both; the existing
strict 1,000 µs build wall maximum is unchanged. All thirteen runs fail (make exits nonzero).
Bundle/signature/rpath/default-stack checks and settled idle smoke pass (506.2 ms).
Source: `/tmp/localwave-p5-performance.log`; prior P1–P4 failures remain in commit evidence.

The original twelve cases have zero measured Rust allocations and pass the frame-82 audit.
LongScroll still exposes 202 allocations/frame and six new text layouts at frame 82 and during
measurement; its audit fails. It advances rows 240→537 in all 100 measured frames. No added warm-up,
pre-shaping or repeated wheel cycle hides cold work. End-to-end CPU also fails its gate.
Wall spans are recorded without attributing their gaps. Native/process allocations and physical-device
behavior remain open. Waveform retains 200 bars/nine release seeks; indicator behavior is verified.

Times are µs; CPU is p50/p99/max, wall is p50/p95/p99/max. Layouts count cache-miss shaping work.

| Workload | Build CPU | Build wall | Frame CPU | Frame wall | Alloc max | Layouts max | Frame 82 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| songs-original | 761/1005/1062 | 842/3406/4179/5073 | 1214/1644/55289 | 16626/19727/30241/108335 | 0 | 0 | pass |
| find-filters-title-descending | 726/950/1005 | 785/2777/4339/4421 | 1223/1522/1597 | 16655/17894/20101/20242 | 0 | 0 | pass |
| queue-occurrences | 604/870/886 | 651/1629/3223/3630 | 1040/1417/1470 | 16670/18078/20725/20910 | 0 | 0 | pass |
| queue-carry-scroll-cancel | 503/885/1120 | 560/2493/3250/4680 | 949/1437/1566 | 16635/18992/20426/23976 | 0 | 0 | pass |
| full-queue-playlist-editor | 718/1104/1119 | 789/3169/4242/4813 | 1217/1631/1649 | 16600/19133/21127/21501 | 0 | 0 | pass |
| albums-artwork | 595/1198/1234 | 680/1422/3380/3516 | 1263/1988/2001 | 16610/19382/21277/22183 | 0 | 0 | pass |
| waveform-scrub-and-scroll | 415/854/977 | 429/1063/2961/2975 | 838/1383/1454 | 16645/19028/20573/20591 | 0 | 0 | pass |
| settings-menus-appearance | 285/576/619 | 315/1298/1828/2000 | 761/2503/2779 | 16625/19490/21388/21462 | 0 | 0 | pass |
| files-indexed-directory | 347/441/481 | 365/1481/2375/3293 | 871/1030/1038 | 16648/17823/21069/21395 | 0 | 0 | pass |
| external-drop-hover-and-cancel | 302/392/401 | 313/515/792/2366 | 786/1069/1073 | 16650/20441/23313/35632 | 0 | 0 | pass |
| artists-measured-columns | 587/964/997 | 644/1394/3623/4148 | 1050/1509/1522 | 16639/20672/21361/31567 | 0 | 0 | pass |
| songs-playing-indicator | 807/1016/1132 | 835/1239/2450/4074 | 1248/1537/1649 | 16624/20747/23788/26171 | 0 | 0 | pass |
| songs-long-scroll | 1030/1348/1404 | 1096/3643/5224/5407 | 1473/1843/1853 | 16628/18365/19934/21625 | 202 | 6 | fail (6 layouts) |

### CPU profile

Separate signed `--performance-check --trace-frames` runs of the five P5 follow-up cases use the same
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
| queue-drag | draw 320.85 | build 189.41 | paint 163.70 |
| song-indicators | paint 341.87 | draw 311.46 | build 248.73 |
| waveform | draw 336.53 | paint 203.11 | build 181.63 |
| artwork | draw 510.39 | build 282.26 | paint 186.87 |
| long-scroll | paint 541.70 | draw 303.16 | build 262.49 |

Sources: `/tmp/localwave-p5-trace-{queue-drag,song-indicators,waveform,artwork,long-scroll}.log`.
The corresponding full phase means, aggregate CPU/wall timings and failing exits are preserved in
P5 commit evidence. Earlier baselines remain in their corresponding commits.
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

P5 exchanges native events/readings, pending replies, output, catalog and changed status under
one inbox lock per native poll. Native operations and UI repair run unlocked. Their acknowledgements
and retired catalog occupy bounded local return slots until an immediately woken follow-up poll;
explicit media/audio-session command delivery retains its enqueue synchronization. New native readings
win over older acknowledgement snapshots. Retirement backpressure and worker-side final release,
including shutdown, remain intact. The focused reply/overflow/stop and catalog tests pass with the
full 206-case Rust suite. Packaging checks, scoped Clippy and the signed iOS simulator build pass;
the iOS build was not installed or launched.

P6 still needs framework draw and paint/text work, including LongScroll's fresh layouts and
allocations. The strict latency and cold-work failures remain open; these changes and CPU percentiles
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
