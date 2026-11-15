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
| P6 | Fix measured hot paths; pass phase gate | doing | HEAD + pxui 796ba63 |
| P6a | Bind fixed renderer state once per pass | done | 89ee53a + pxui d10fb8c |
| P6b | Worker-prepared text API and ownership | done | 8a934a3 + pxui 3231c8e |
| P6c | Bounded song-text delivery; remove LongScroll allocations | done | d67773d |
| P6d | Skip damage calculations inside an already covered view clip | done | 79cb6c6 + pxui 71dc0a1 |
| P6e | Reuse bounded fresh-row gaps before retained runs | done | HEAD + pxui 796ba63 |
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

P6e bounded fresh-gap reuse `make performance`: thirteen signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Frame spans cover input pump through
GPU draw return; build spans retain their earlier boundary. CPU p99 is gated for both; the existing
strict 1,000 µs build wall maximum is unchanged. All thirteen runs fail (make exits nonzero).
Bundle/signature/rpath/default-stack checks and settled idle smoke pass (509.3 ms).
Source: `/tmp/localwave-p6-gap-performance.log`; prior P1–P6d failures remain in commit evidence.

All thirteen cases have zero measured Rust allocations/new UI text layouts and pass the frame-82
audit. LongScroll formerly exposed 202 allocations/frame and six new layouts; bounded worker
delivery removes both. It still advances rows 240→537 in all 100 measured frames. The original
182/82/100 frames and wheel trace are unchanged. End-to-end CPU also fails its gate.
Wall spans are recorded without attributing their gaps. Native/process allocations and physical-device
behavior remain open. Waveform retains 200 bars/nine release seeks; indicator behavior is verified.

Times are µs; CPU is p50/p99/max, wall is p50/p95/p99/max. Layouts count cache-miss shaping work.

| Workload | Build CPU | Build wall | Frame CPU | Frame wall | Alloc max | Layouts max | Frame 82 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| songs-original | 649/956/1003 | 733/1601/2073/2641 | 1115/1428/1519 | 16632/17700/18753/21445 | 0 | 0 | pass |
| find-filters-title-descending | 617/1180/1203 | 686/2659/3447/3745 | 1029/1771/1866 | 16645/17956/20039/30881 | 0 | 0 | pass |
| queue-occurrences | 643/959/1002 | 726/3876/4049/5628 | 1147/1468/1536 | 16617/18367/19280/22544 | 0 | 0 | pass |
| queue-carry-scroll-cancel | 524/883/905 | 608/1753/3101/3573 | 965/1498/1509 | 16645/19863/20636/21203 | 0 | 0 | pass |
| full-queue-playlist-editor | 705/1152/1164 | 853/2704/3206/3522 | 1257/1729/1737 | 16596/18226/20495/20507 | 0 | 0 | pass |
| albums-artwork | 664/1083/1128 | 782/3375/4045/4328 | 1290/1775/1779 | 16592/18932/19495/19899 | 0 | 0 | pass |
| waveform-scrub-and-scroll | 348/900/901 | 454/1730/3241/3949 | 834/1414/1519 | 16618/19071/20177/32775 | 0 | 0 | pass |
| settings-menus-appearance | 312/522/564 | 344/1805/3469/3498 | 729/2632/2859 | 16650/18719/20187/20261 | 0 | 0 | pass |
| files-indexed-directory | 341/482/486 | 366/1254/2956/3183 | 761/1019/1057 | 16632/19342/20194/20357 | 0 | 0 | pass |
| external-drop-hover-and-cancel | 307/405/424 | 314/989/2623/2733 | 776/1013/1053 | 16671/19678/21481/24285 | 0 | 0 | pass |
| artists-measured-columns | 653/971/990 | 745/3219/3745/3824 | 1133/1512/1636 | 16652/18798/20886/21549 | 0 | 0 | pass |
| songs-playing-indicator | 771/1283/1307 | 842/2981/4721/5138 | 1221/1715/1811 | 16649/19621/20750/20902 | 0 | 0 | pass |
| songs-long-scroll | 927/1269/1329 | 1227/3820/5388/6456 | 1423/1876/1956 | 16560/18501/19362/19742 | 0 | 0 | pass |

### CPU profile

Separate signed `--performance-check --trace-frames` runs of the five P6a follow-up cases use the same
10,000-song, 1440×872, 182/82/100 fixture. All five fail the unchanged performance gate;
Rank by mean thread CPU over the 100 measured frames, rather than wall time or an individual
outlier. Profiling is opt-in. LongScroll below uses the current P6e trace; the other four rows
retain their P6a trace baseline.
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
| long-scroll | draw 373.13 | paint 342.66 | build 319.06 |

Sources: `/tmp/localwave-p6-bindings-trace-{queue-drag,song-indicators,waveform,artwork,long-scroll}.log`.
The current LongScroll trace is `/tmp/localwave-p6-gap-long-trace.log`; it also has zero
warm Rust allocations/layouts and a passing frame-82 audit, while latency still fails. Full phase
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
The streaming regression reduces compactions from 100 to 50 per 100 frames; largest retained
storage grows from 1,857 to 2,049 instances inside the same bound. A 19-of-20 replacement case
retains its 2,690-instance bound (largest 2,561) and still compacts every frame under the existing
budget check. Both preserve row order, geometry, footer, dirty bounds and zero warm allocations.
The final focused three-row mean is 37.15 µs; full-suite mean is 42.75 µs. These separate runs
do not establish an end-to-end speedup. All signed gates still fail as recorded in the matrix.
Temporary placement diagnostics were removed. Evidence and earlier failures live in commits.
P6e verification: 401 framework cases pass in 12.524 s; 209 LocalWave cases pass in 10.900 s,
zero skips, with two/six existing isolated contention retries respectively. Dependency and diff
checks pass. The separate signed LongScroll trace also fails: build CPU 947/1351/1364 µs,
wall maximum 4,753 µs, zero allocations/layouts and a passing frame-82 audit.

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
