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
| P3 | Isolate position ticks from stable content | done | HEAD |
| P4 | Worker-prepared catalog remapping | todo | — |
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

P3 `make performance`: thirteen signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Frame spans cover input pump through
GPU draw return; build spans retain their earlier boundary. CPU p99 is gated for both; the existing
strict 1,000 µs build wall maximum is unchanged. All thirteen runs fail (make exits nonzero).
Bundle/signature/rpath/default-stack checks and settled idle smoke pass (507.2 ms).
Source: `/tmp/localwave-p3-performance.log`; prior P1/P2 failures remain in commit evidence.

The original twelve cases have zero measured Rust allocations and pass the frame-82 audit.
LongScroll still exposes 202 allocations/frame and six new text layouts at frame 82 and during
measurement; its audit fails. It advances rows 240→537 in all 100 measured frames. No added warm-up,
pre-shaping or repeated wheel cycle hides cold work. End-to-end CPU also fails its gate.
Wall spans are recorded without attributing their gaps. Native/process allocations and physical-device
behavior remain open. Waveform retains 200 bars/nine release seeks; indicator behavior is verified.

Times are µs; CPU is p50/p99/max, wall is p50/p95/p99/max. Layouts count cache-miss shaping work.

| Workload | Build CPU | Build wall | Frame CPU | Frame wall | Alloc max | Layouts max | Frame 82 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| songs-original | 703/969/999 | 734/2646/3133/3494 | 1133/1458/1458 | 16677/18163/18761/34662 | 0 | 0 | pass |
| find-filters-title-descending | 697/904/1032 | 775/1984/5113/7522 | 1155/1450/1468 | 16662/18523/19579/21544 | 0 | 0 | pass |
| queue-occurrences | 619/843/845 | 711/1987/2637/3191 | 1069/1358/1432 | 16646/18864/19107/19335 | 0 | 0 | pass |
| queue-carry-scroll-cancel | 525/861/880 | 555/2022/2912/3441 | 979/1404/1450 | 16675/18532/18999/19152 | 0 | 0 | pass |
| full-queue-playlist-editor | 659/994/1230 | 823/2662/3677/5360 | 1123/1451/1549 | 16664/18876/20178/21821 | 0 | 0 | pass |
| albums-artwork | 612/1027/1069 | 648/1580/1871/2646 | 1327/1844/2158 | 16649/17516/18212/18332 | 0 | 0 | pass |
| waveform-scrub-and-scroll | 442/970/973 | 530/2238/2874/3646 | 902/1500/1512 | 16649/18740/20690/21319 | 0 | 0 | pass |
| settings-menus-appearance | 330/601/618 | 365/1105/1511/2737 | 818/2613/3111 | 16637/18173/22446/33116 | 0 | 0 | pass |
| files-indexed-directory | 354/431/433 | 366/997/1555/1628 | 869/1034/1039 | 16618/18923/21005/21638 | 0 | 0 | pass |
| external-drop-hover-and-cancel | 303/414/438 | 334/1896/2166/2774 | 743/952/953 | 16608/18815/20033/25142 | 0 | 0 | pass |
| artists-measured-columns | 646/1002/1025 | 670/2447/3146/4247 | 1076/1638/1644 | 16638/18671/19376/19537 | 0 | 0 | pass |
| songs-playing-indicator | 933/1271/1306 | 1186/2528/3736/4442 | 1446/1758/1789 | 16604/18438/19281/19530 | 0 | 0 | pass |
| songs-long-scroll | 1147/1520/1567 | 1456/3169/3544/3852 | 1588/2069/2200 | 16620/18761/18979/19059 | 202 | 6 | fail (6 layouts) |

### CPU profile

Separate signed `--performance-check --trace-frames` runs of the five P3 follow-up cases use the same
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
| queue-drag | draw 374.62 | build 202.70 | paint 171.15 |
| song-indicators | paint 352.54 | draw 319.87 | build 261.17 |
| waveform | draw 333.77 | paint 213.15 | build 185.57 |
| artwork | draw 545.98 | build 302.20 | paint 200.13 |
| long-scroll | paint 590.94 | draw 372.96 | build 296.88 |

Sources: `/tmp/localwave-p3-trace-{queue-drag,song-indicators,waveform,artwork,long-scroll}.log`.
The corresponding full phase means, aggregate CPU/wall timings and failing exits are preserved in
P3 commit evidence. The P2 baseline remains in its parent commit.
Separate runs do not establish a controlled speedup. Correlated records confirm the measured CPU
phases fit inside their parent spans.

P3 separates position from stable `PlayerView` data. Worker status delivery keeps its full revision;
a second stable revision excludes position-only updates. Desktop and compact libraries use stable
revision keys, and Now metadata uses numeric revision/catalog identity instead of hashing track
strings on each tick. Clocks and seek controls receive the separate position and still advance.
The 10,000-song regression checks 20 ticks at 50 ms: no desktop song-row or stable metadata rebuilds,
no compact-library rebuilds, live seek/clock values and zero Rust allocations across app and dylib.
It also checks that actual metadata/current-track/pause changes invalidate the retained views.
The worker test verifies all 20 ticks still publish while keeping the stable revision unchanged.

Signed fixtures advance the position on every frame; actual indicator pause/play transitions still
advance stable revision. No input, clock updates, frame counts, warm-up or assertions were omitted.
Waveform retains 200 bars/nine release-only seeks; indicators retain 58 playing/34 paused/57 changes.
Full Rust iOS simulator build/signing passes; this does not qualify device playback or provider behavior.

P4 is next: prepare catalog adoption remaps on the worker. P5 still addresses polling locks; these
engine profiles alone do not quantify either benefit. P6 should investigate framework draw and
paint/text first, including LongScroll's six fresh layouts and 202 allocations/frame. The strict
latency and cold-work failures remain open. Neither CPU percentiles nor cached tick tests replace
the unchanged signed workload gate.

## Open qualification

- Physical USB iPhone/iPad signing, install, launch and 120 Hz touch/scroll behavior.
- Real background audio, interruptions, route changes and system media controls on devices.
- Live iCloud/Files providers: original picker grants, downloads, offline recovery, account changes,
  moved/expired bookmarks and reconnect. Local URL/synthetic checks do not prove provider behavior.
- Actual sandbox Finder drops, native full-screen/Spaces gestures and supported macOS runtime checks.
- Interactive IME/candidate selection, dictation, floating keyboard avoidance and device editing feel.
- Real-user-container migration, distribution provisioning, TestFlight delivery and a release cycle.

## Known gaps

- Performance P4–P6 precedes new feature work.
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
