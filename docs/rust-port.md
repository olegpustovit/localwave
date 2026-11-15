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
| B6 | Framework completed file-drop events | done | HEAD |
| B7 | Move profiling/lifetime checks; close boundary | todo | — |
| P1 | End-to-end timing and LongScroll workload | todo | — |
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

B6 `make performance`: twelve signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Bundle signature/dependency/default-stack
checks and settled idle smoke pass (507.3 ms). Source: `/tmp/localwave-b6-performance.log`.
12 strict 1,000 µs wall maxima fail, so `make performance` returns nonzero. The full latency
gate remains unmet; no causal speedup is claimed. Native/process allocations remain unqualified.
Waveform scrubbing retains 200 bars and exactly nine release commits; the indicator validates
58 visible playing / 34 paused / 57 changing frames. Previous measurements remain in git history.

| Workload | CPU p50/p99/max µs | Wall p50/p95/p99/max µs | Warm Rust allocations |
| --- | --- | --- | --- |
| songs-original | 708/909/954 | 807/2542/3006/4131 | 0 |
| find-filters-title-descending | 738/924/960 | 879/2916/3374/3734 | 0 |
| queue-occurrences | 629/857/918 | 707/2110/3310/3343 | 0 |
| queue-carry-scroll-cancel | 541/1113/1149 | 592/1964/2254/2528 | 0 |
| full-queue-playlist-editor | 635/923/953 | 750/1904/2238/2250 | 0 |
| albums-artwork | 689/1165/1206 | 831/2186/2859/3080 | 0 |
| waveform-scrub-and-scroll | 446/963/973 | 528/1535/2706/2769 | 0 |
| settings-menus-appearance | 291/548/578 | 329/1224/1908/2686 | 0 |
| files-indexed-directory | 314/426/470 | 334/1152/1680/2122 | 0 |
| external-drop-hover-and-cancel | 310/416/419 | 334/1501/1972/2165 | 0 |
| artists-measured-columns | 706/967/1020 | 762/2363/2777/3894 | 0 |
| songs-playing-indicator | 885/1128/1209 | 1044/2787/3069/3871 | 0 |

## Open qualification

- Physical USB iPhone/iPad signing, install, launch and 120 Hz touch/scroll behavior.
- Real background audio, interruptions, route changes and system media controls on devices.
- Live iCloud/Files providers: original picker grants, downloads, offline recovery, account changes,
  moved/expired bookmarks and reconnect. Local URL/synthetic checks do not prove provider behavior.
- Actual sandbox Finder drops, native full-screen/Spaces gestures and supported macOS runtime checks.
- Interactive IME/candidate selection, dictation, floating keyboard avoidance and device editing feel.
- Real-user-container migration, distribution provisioning, TestFlight delivery and a release cycle.

## Known gaps

- Boundary B7 and performance P1–P6 precede new feature work.
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
