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
| P1 | End-to-end timing and LongScroll workload | done | HEAD |
| P2 | Profile top frame costs | todo | — |
| P3 | Isolate position ticks from stable content | todo | — |
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

P1 `make performance`: thirteen signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Frame spans cover input pump through
GPU draw return; build spans retain their earlier boundary. CPU p99 is gated for both; the existing
strict 1,000 µs build wall maximum is unchanged. All thirteen runs fail (make exits nonzero).
Bundle/signature/rpath/default-stack checks and settled idle smoke pass (502.0 ms).
Source: `/tmp/localwave-p1-performance-final.log`; prior failures remain in commit evidence.

The original twelve cases have zero measured Rust allocations and pass the frame-82 audit.
LongScroll exposes 202 allocations/frame and six new text layouts at frame 82 and during measurement;
its audit fails. It advances rows 240→537 in all 100 measured frames, without revisiting a wheel cycle.
No additional warm-up or pre-shaping hides this cold work. Its accessibility tree is consumed to
verify row order. End-to-end CPU exposes work omitted by build-only timing; wall spans are recorded
without attributing their gaps. Native/process allocations and physical-device behavior remain open.
Waveform retains 200 bars/nine release seeks; the indicator validates 58 playing/34 paused/57 changes.

Times are µs; CPU is p50/p99/max, wall is p50/p95/p99/max. Layouts count cache-miss shaping work.

| Workload | Build CPU | Build wall | Frame CPU | Frame wall | Alloc max | Layouts max | Frame 82 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| songs-original | 715/939/965 | 754/1887/2653/2691 | 1139/1673/1720 | 16643/18677/19162/20215 | 0 | 0 | pass |
| find-filters-title-descending | 747/937/969 | 811/1640/3477/3556 | 1312/1577/1682 | 16680/19167/19898/20282 | 0 | 0 | pass |
| queue-occurrences | 674/864/908 | 735/1382/2365/2966 | 1174/1533/1545 | 16649/18699/19123/19299 | 0 | 0 | pass |
| queue-carry-scroll-cancel | 363/817/838 | 374/954/1435/1649 | 861/1336/1361 | 16657/18947/19640/33296 | 0 | 0 | pass |
| full-queue-playlist-editor | 643/976/1100 | 759/1854/2182/2712 | 1122/1450/1460 | 16639/17707/18308/18668 | 0 | 0 | pass |
| albums-artwork | 648/1133/1144 | 739/1919/2712/3185 | 1305/1958/1980 | 16601/18920/19153/19230 | 0 | 0 | pass |
| waveform-scrub-and-scroll | 468/953/1054 | 494/1598/2798/11077 | 895/1442/1544 | 16664/18447/18957/19117 | 0 | 0 | pass |
| settings-menus-appearance | 308/583/588 | 353/1067/1497/1840 | 798/2740/2849 | 16618/18142/18870/19816 | 0 | 0 | pass |
| files-indexed-directory | 309/396/399 | 326/1633/1849/2062 | 722/827/853 | 16637/18412/18671/19065 | 0 | 0 | pass |
| external-drop-hover-and-cancel | 291/370/392 | 307/1032/1430/1672 | 718/849/858 | 16646/18652/18928/21060 | 0 | 0 | pass |
| artists-measured-columns | 676/952/1054 | 752/2295/2636/2986 | 1112/1561/1632 | 16663/18222/18872/21327 | 0 | 0 | pass |
| songs-playing-indicator | 850/1146/1198 | 944/2337/2840/2847 | 1310/1693/1795 | 16606/18246/19297/19705 | 0 | 0 | pass |
| songs-long-scroll | 1182/1409/1510 | 1327/2745/3479/3769 | 1727/2009/2029 | 16606/18087/18783/19012 | 202 | 6 | fail (6 layouts) |

## Open qualification

- Physical USB iPhone/iPad signing, install, launch and 120 Hz touch/scroll behavior.
- Real background audio, interruptions, route changes and system media controls on devices.
- Live iCloud/Files providers: original picker grants, downloads, offline recovery, account changes,
  moved/expired bookmarks and reconnect. Local URL/synthetic checks do not prove provider behavior.
- Actual sandbox Finder drops, native full-screen/Spaces gestures and supported macOS runtime checks.
- Interactive IME/candidate selection, dictation, floating keyboard avoidance and device editing feel.
- Real-user-container migration, distribution provisioning, TestFlight delivery and a release cycle.

## Known gaps

- Performance P2–P6 precedes new feature work.
  LongScroll now exposes cold text shaping and 202 measured Rust allocations/frame; its warm-up
  audit fails. Twelve visited-row workloads remain zero. End-to-end CPU also exceeds the new gate.
  The full strict latency gate still fails; cold shaping and native memory need evidence.
  A text/core edit check took 1.60 s, exceeding pxui's 1 s warm-edit target.
  P3 still needs to remove stable metadata/state string hashing and position-driven rebuilds.
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
