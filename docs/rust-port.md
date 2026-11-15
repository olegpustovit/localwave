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
| C2 | Labels, module description and test contention | done | HEAD |
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

C1.4c `make performance`: twelve signed workloads, 10,000 songs, 1440×872,
182 frames / 82 warm-up / 100 measured, both Rust counters. Bundle signature/dependency/default-stack
checks and settled idle smoke pass (503.7 ms). Source: `/tmp/localwave-c14c-performance.log`.
All strict 1,000 µs wall maxima fail, so `make performance` returns nonzero. These are failures,
not passes; native/process allocation behavior remains unqualified. Previous measurements live in git history.

| Workload | CPU p50/p99/max µs | Wall p50/p95/p99/max µs | Warm Rust allocations |
| --- | --- | --- | --- |
| songs-original | 759/1007/1051 | 869/2790/3912/5783 | 0 |
| find-filters-title-descending | 703/906/943 | 793/2268/2883/3104 | 0 |
| queue-occurrences | 626/851/883 | 707/2501/2976/3607 | 0 |
| queue-carry-scroll-cancel | 516/947/1072 | 562/1617/2455/2531 | 0 |
| full-queue-playlist-editor | 578/943/999 | 627/1970/2901/5120 | 0 |
| albums-artwork | 582/1023/1107 | 654/1983/3643/3660 | 0 |
| waveform-scrub-and-scroll | 387/1035/1067 | 566/2147/2853/2964 | 0 |
| settings-menus-appearance | 314/607/721 | 402/1390/2064/3461 | 0 |
| files-indexed-directory | 365/510/519 | 457/2267/3003/3076 | 0 |
| external-drop-hover-and-cancel | 301/421/425 | 369/1259/1780/2203 | 0 |
| artists-measured-columns | 773/1039/1096 | 1182/3421/3943/4179 | 0 |
| songs-playing-indicator | 844/1111/1182 | 1125/3070/3326/3669 | 0 |

## Open qualification

- Physical USB iPhone/iPad signing, install, launch and 120 Hz touch/scroll behavior.
- Real background audio, interruptions, route changes and system media controls on devices.
- Live iCloud/Files providers: original picker grants, downloads, offline recovery, account changes,
  moved/expired bookmarks and reconnect. Local URL/synthetic checks do not prove provider behavior.
- Actual sandbox Finder drops, native full-screen/Spaces gestures and supported macOS runtime checks.
- Interactive IME/candidate selection, dictation, floating keyboard avoidance and device editing feel.
- Real-user-container migration, distribution provisioning, TestFlight delivery and a release cycle.

## Known gaps

- Boundary B1–B7 and performance P1–P6 precede new feature work.
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
