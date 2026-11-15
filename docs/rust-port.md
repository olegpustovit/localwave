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
| C1.4 | Extract performance harness (a–c) | doing | 71d32b2 |
| C1.4a | Parse one Workload enum | done | e93d3da |
| C1.4b | Extract performance setup | done | HEAD |
| C1.4c | Extract driver/report and production App | todo | — |
| C1.5 | Split controller and command families | todo | — |
| C1.6 | Enforce Rust production file-size limit | todo | — |
| C2 | Labels, module description and test contention | todo | — |
| B1 | Extract SQLite; remove core Apple dependency | todo | — |
| B2 | Framework text identity and pixel rounding | todo | — |
| B3 | Repeating retained keyframes and indicators | todo | — |
| B4 | Framework scrub signal for seek/volume | todo | — |
| B5 | Framework virtual-list reveal | todo | — |
| B6 | Framework completed file-drop events | todo | — |
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

C1.4b `make performance`: twelve signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Bundle signature/dependency/default-stack
checks and settled idle smoke pass (508.3 ms). Source: `/tmp/localwave-c14b-performance.log`.
All strict 1,000 µs wall maxima fail, so `make performance` returns nonzero. These are failures,
not passes; native/process allocation behavior remains unqualified. Previous measurements live in git history.

| Workload | CPU p50/p99/max µs | Wall p50/p95/p99/max µs | Warm Rust allocations |
| --- | --- | --- | --- |
| songs-original | 759/1001/1031 | 883/2649/3349/3567 | 0 |
| find-filters-title-descending | 682/898/912 | 745/1809/3479/7867 | 0 |
| queue-occurrences | 656/892/1211 | 718/2658/2969/3324 | 0 |
| queue-carry-scroll-cancel | 548/877/892 | 598/1839/2928/4445 | 0 |
| full-queue-playlist-editor | 720/1140/1170 | 869/2029/2277/2576 | 0 |
| albums-artwork | 578/1119/1166 | 614/1666/2179/3339 | 0 |
| waveform-scrub-and-scroll | 383/877/908 | 519/1863/2534/2834 | 0 |
| settings-menus-appearance | 300/525/595 | 327/920/2214/2285 | 0 |
| files-indexed-directory | 348/495/512 | 382/1811/2392/2395 | 0 |
| external-drop-hover-and-cancel | 133/351/383 | 133/473/1302/1833 | 0 |
| artists-measured-columns | 713/1081/1096 | 843/2382/2949/4189 | 0 |
| songs-playing-indicator | 977/1311/1324 | 1381/3140/3562/3732 | 0 |

## Open qualification

- Physical USB iPhone/iPad signing, install, launch and 120 Hz touch/scroll behavior.
- Real background audio, interruptions, route changes and system media controls on devices.
- Live iCloud/Files providers: original picker grants, downloads, offline recovery, account changes,
  moved/expired bookmarks and reconnect. Local URL/synthetic checks do not prove provider behavior.
- Actual sandbox Finder drops, native full-screen/Spaces gestures and supported macOS runtime checks.
- Interactive IME/candidate selection, dictation, floating keyboard avoidance and device editing feel.
- Real-user-container migration, distribution provisioning, TestFlight delivery and a release cycle.

## Known gaps

- Cleanup C1.0–C2, boundary B1–B7 and performance P1–P6 precede new feature work.
  The full strict latency gate still fails; cold shaping, native memory and build budgets need evidence.
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
