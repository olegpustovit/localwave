# Rust port status

## Steps

Follow the [implementation plan](https://artifacts.microapps.space/a/w474jy87pnxg#plan) in order.
DX rows follow the [API redesign plan](https://artifacts.microapps.space/a/mr55jcx8k90t#plan).
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
| B3a | Schedule plain-node loops through their retained owner | done | b4ec380 + pxui 48c55f8 |
| B4 | Framework scrub signal for seek/volume | done | 57cb610 |
| B5 | Framework virtual-list reveal | done | 6179982 |
| B6 | Framework completed file-drop events | done | 042b961 |
| B7 | Move profiling/lifetime checks; close boundary | done | 325d54a |
| P1 | End-to-end timing and LongScroll workload | done | d407e8a |
| P2 | Profile top frame costs | done | 360a5e3 |
| P3 | Isolate position ticks from stable content | done | d16e6a3 |
| P4 | Worker-prepared catalog remapping | done | 81ab1ca |
| P5 | Single controller-poll lock | done | 7bb51ac |
| P6 | Fix measured hot paths; pass phase gate | done | d41fb42 + pxui 520db20 |
| P6a | Bind fixed renderer state once per pass | done | 89ee53a + pxui d10fb8c |
| P6b | Worker-prepared text API and ownership | done | 8a934a3 + pxui 3231c8e |
| P6c | Bounded song-text delivery; remove LongScroll allocations | done | d67773d |
| P6d | Skip damage calculations inside an already covered view clip | done | 79cb6c6 + pxui 71dc0a1 |
| P6e | Reuse bounded fresh-row gaps before retained runs | done | ec54614 + pxui 796ba63 |
| P6f | Bound actual write ends when reusing retained storage | done | 962df25 + pxui 519e367 |
| P6g | Avoid recursive native background redraws under opaque Metal | done | 6614ab5 + pxui 8a9f05b |
| P6h | Borrow instances during retained-run compaction | done | 4f4ed00 + pxui 7556bee |
| P6i | Limit playback invalidation to affected song rows | done | 5f03419 |
| P6j | Borrow fresh instances during retained placement | done | ca17adc + pxui 766f6ff |
| P6k | Owner-approved M1 limits; gate frame work wall | done | d41fb42 + pxui 520db20 |
| F1 | Global shortcuts from one command table | done | af4b2e8 |
| F2 | Favorite commands, menu, key and heart | done | 504bf9f |
| F3 | Complete track menu and native reveal | done | 3a56ba9 |
| F4 | Playlist CRUD, membership and reorder | done | 72bb927 |
| F5 | Album/artist detail headers and actions | done | 1868431 |
| F6 | Occurrence-based multi-selection | done | 8ce3af5 |
| F7 | Drag songs to playlists and queue | done | 905149e |
| F8 | Metadata editor and metadata-only deletion | done | HEAD |
| F9 | Source observation and debounced rescan | done | 12c2408 + d7c9b87 |
| F10 | Onboarding, route empty states and Share | done | 92993cf |
| S | Sign, migrate, TestFlight and release macOS | doing | `make sign` (keychain access pending), `tools/notarize-macos.sh` unrun, `tools/migration-check.sh` passes on a container copy |
| I1 | Framework gestures and device comparison | doing | pxui e19dedb..62f1f94 (ABI 1.38-1.42: gestures, two-sided swipe, dismiss, touch drag; trace tests); device side-by-side pending |
| I2 | iOS flows, physical install and shipping gates | doing | Compact flows on simulator/macOS (swipe, long press, refresh, Now player, sheets, Select, menus, Files); 390×778 goldens; `make ios-build` passes; physical install, 120 Hz and device qualification pending |
| D | Resolve deferred accessibility/opacity decisions | doing | HEAD + pxui a0a0cb8 |
| DX0.1 | DX phase rows and ordering | done | aec5dce |
| DX0.2 | Allocation tripwire: warm_scope/take_reports | done | f126022 + pxui 53d48ba |
| DX0.3 | Measured uncovered workloads (PERF) | done | 8b62723 |
| DX0.4 | cargo t --doc in the runner | done | 8ef0591 + pxui 4804a4f |
| DX0.5 | Gate: author-shim rule | done | bde0f9b + pxui 2e6a4ec |
| DX0.6 | abi_bench page for authoring helpers | done | 51bd807 + pxui 88c61a7 |
| DX0.7 | verify_views in LocalWave UI tests | done | 9be4f52 |
| DX0.7a | Stale view `compact-library`: revision misses an input (found by DX0.7) | done | 736f07c |
| DX0.7b | Stale view `visual (kit button press scale)`: revision misses an input (found by DX0.7) | done | 49ae7ec |
| DX0.7c | Stale view `transport`: revision misses an input (found by DX0.7) | done | 5e77922 |
| DX0.7d | Stale view `sidebar`: revision misses an input (found by DX0.7) | done | 4fb54b0 |
| DX0.7e | Stale view `now-panel`: revision misses an input (found by DX0.7) | done | dfc7b42 |
| DX0.7f | Stale view `up-next`: revision misses an input (found by DX0.7) | done | 0754b76 |
| DX0.7g | pxui verify_views: replay transitions started in a body and nested-view draw segments under a11y (blocks DX0.7b/e) | done | 8c5158c + pxui 4670ad6 |
| DX1.1 | const fn builders on Node, Look, Edges, Font | done | 254361b + pxui 86c1601 |
| DX1.2 | Flags newtype; distinct FOCUSABLE names | done | 6f0fc01 + pxui 2f7a6f1 |
| DX1.3 | Key type; row/column/stack shims | done | f3b89c5 + pxui 64f2bba |
| DX1.4 | label and label_fmt | done | 25544b5 + pxui 8c3116b |
| DX1.5 | Response and pressable | done | 19cb7d8 + pxui ac973af |
| DX1.6 | view_of and hash_of | done | aebb68e + pxui 5e63962 |
| DX1.7 | local_bool/local_i64/local_f32 | done | 0cd0585 + pxui 770760a |
| DX1.8 | pxui::doc::frames and pxui::testing | done | c8ad0eb + pxui 9cd2dcb |
| DX1.9 | header.rs reference conversion (PERF) | done | 0292834 |
| DX1.10 | Text API matches the proposal: ui.label(text, Label) with const builders | done | 1cb17d5 + pxui 1089fe5 |
| DX1.11 | One shape per job: remove twins (signal_state, i64 flags, list forms, allocating text twins, _with pairs), set_colors pairs, local_* stale default | done | 8709c75 + pxui 5c16fdb |
| DX1.12 | Full public-surface audit against the proposal before the freeze (blocks DX7) | todo |  |
| DX2.1 | Borrowed key signals | done | d7aa1d9 + pxui 4e0de1f |
| DX2.2 | Borrowed text results | done | 1667f9e + pxui 5e0e4a3 |
| DX2.2a | Gallery and abi_bench build again after DX2.2 (shared pages.rs) | done | 5ed6818 + pxui 769f444 |
| DX2.3 | Borrowed inputs; gate rejects owned parameters | done | efe51b9 + pxui 550a4c9 |
| DX2.4 | _into forms for quads and rasterize_rgba | done | 1a95982 + pxui 932e9c5 |
| DX2.5 | Typed errors | done | b63c59e + pxui 850be9d |
| DX2.6 | One API shape: engine matches the wrapper; gallery compat deleted | done | 657e6ea + pxui 7c4e0db |
| DX3.1 | Theme row height outside panels | done | b89b7c5 + pxui edfae14 |
| DX3.2 | App-defined colour tokens | done | b1e9ef6 + pxui 74c723a |
| DX3.3 | Engine text element behind label | done | ca088b0 + pxui 065dc5e |
| DX3.4 | button_with and Response | done | 24be9be + pxui 9c84707 |
| DX3.5 | icon_button | done | c2bf971 + pxui 7891b2a |
| DX3.5a | abi_bench back within limits after DX3.2/3.4/1.10/3.5 (no limit or baseline changed) | done | 712627d + pxui b999ffd |
| DX3.6 | menu with MenuItem | done | ab60a7b + pxui 759c84a |
| DX3.7 | toggle_with, segmented_with, disabled scope | done | bf5780a + pxui 30260fb |
| DX3.8 | dialog_with | done | eb713db + pxui d45155b |
| DX3.9 | Per-call widget body variants | done | 1dbb163 + pxui ddb538f |
| DX3.10 | Label, Button and IconButton style values express LocalWave helpers (found by DX4.2–4.4) | done | 2a97383 + pxui 76d9c7d |
| DX3.11 | Menu, Toggle, Segmented and Dialog carry what app bodies need (found by DX4.5–4.7) | done | eaa73cd + pxui 95df44c |
| DX3.12 | Disabled widgets never click; Button label/layout/focus options; button_content gets the state (found by DX4.3/4.4) | done | 99ebbe1 + pxui 34c8e4f |
| DX3.12a | DX3.12 review follow-ups (legacy export marked, docs) | done | ae5b91a + pxui 456892b |
| DX4.1 | Palette to colour tokens | done | fbfe4be |
| DX4.2 | kit::text and settings::paragraph to label | done | 269af1c |
| DX4.3 | Buttons to button_with | done | 157587d |
| DX4.4 | Icon buttons to icon_button | done | 27c3e69 |
| DX4.5 | Menus to menu | todo |  |
| DX4.6 | Settings switch and chips to toggle_with/segmented_with | todo |  |
| DX4.7 | Confirmation sheets to dialog_with; field variant | todo |  |
| DX4.7a | Field body picks the Find structure by variant (part of DX4.7) | done | 3181aa4 |
| DX4.8 | Codemod A: Node literals to builders | done | 4091362 |
| DX4.9 | Codemod B: hashed view to view_of | done | ccbe842 |
| DX4.10 | Codemod C: node+within to containers | done | 51e19dc |
| DX4.11 | Hand conversion to pressable, one commit per file | todo |  |
| DX4.12 | LocalWave review cleanups: id keys, named styles, theme bound once, press state in button contents | done | 0172279 |
| DX4.13 | API-conformance check in make check (lw_ui/tests/api_conformance.rs, shrinking allowlist; empty when DX4 completes) | done | 73512bf |
| DX5.1 | Pinned glyph entries; label_digits | done | 9260c34 |
| DX5.2 | Debug report for prepared/requested style mismatch | done | f129f4e + pxui ef803a6 |
| DX5.3 | Viewport-distance eviction for list entries | done | de53440 + pxui 5340fe9 |
| DX5.4 | Engine prepare queue; fling exception ceiling | todo |  |
| DX5.5 | Reusable layout storage for fresh shapes | done | 71d78a2 + pxui 8bbb922 |
| DX5.6 | Remove remaining caller allocations in DX0.3 workloads | done | 39c4d96 (fling, reverse-scroll, scroll-jump, non-song-lists, search-typing, text-typing, resize, theme-switch gated at 0; first-navigation and first-menu-open stay measured: all their allocations are first-time shaping, glyph rasterising and node creation for never-drawn content) |
| DX-PERF2 | abi_bench: Overlays wrapper saving; stable small-page and start-up estimates (no limit changed) | done | eaa6541 + pxui 3cf7de3 |
| DX6.1 | Public API documentation section; ABI text only in docs/abi.md | done | 7765686 + pxui 629c3be |
| DX6.2 | One item per line in pxui and pxui_types | done | 8447600 + pxui 08d4efd |
| DX6.3 | gate -- docs with shrinking allowlist | done | 10dc9a5 + pxui 74a81f8 |
| DX6.0a | Doctests really run (rays-t ran none) and run split by module | done | 5123918 + pxui 6662f51 |
| DX6.0b | Fix the 13 doctest examples that never ran | done | ff1a73f + pxui 369fe6c |
| DX6.4 | Docs: model and frame | done | 3e582a1 + pxui 20e5661 |
| DX6.5 | Docs: nodes and state | done | 962be7c + pxui 4086ec0 |
| DX6.6 | Docs: views, overlays, animation | done | ca227ba + pxui 652ec53 |
| DX6.7 | Docs: Paint (1) | done | c164296 + pxui 1d8678e |
| DX6.8 | Docs: Paint (2) | done | 622d26c + pxui 5830fd4 |
| DX6.9 | Docs: keys and queries | done | 4c586e9 + pxui 61ffda6 |
| DX6.10 | Docs: gestures | done | a61b65a + pxui 07c3d9d |
| DX6.11 | Docs: text | done | ad2410d + pxui 6ab2bf7 |
| DX6.12 | Docs: kit widgets | todo |  |
| DX6.13 | Docs: options | todo |  |
| DX6.14 | Docs: inspector | todo |  |
| DX6.15 | Docs: images and renderer | done | 0d0cd58 + pxui 48be183 |
| DX6.16 | Docs: node.rs | todo |  |
| DX6.17 | Docs: layout and geometry | done | 0a5c610 + pxui 4c08a69 |
| DX6.18 | Docs: theme | done | c772eb9 + pxui 5b268d7 |
| DX6.19 | Docs: ui, a11y, draw | done | eb23944 + pxui 28893de |
| DX6.20 | Docs: pxui_types root | todo |  |
| DX6.21 | Docs: ffi.rs (1) | todo |  |
| DX6.22 | Docs: ffi.rs (2) | todo |  |
| DX6.23 | Docs: ffi.rs (3) | todo |  |
| DX6.24 | Docs: ffi.rs (4) | todo |  |
| DX6.25 | deny(missing_docs); allowlist deleted | todo |  |
| DX6.26 | pxui-app skill; README and specification fixes | done | 73512bf + pxui a50ab58 |
| DX7.1 | Generated layout manifest; C static asserts | done | 2c4bd61 + pxui 8b4dcae |
| DX7.1a | pxui.h compiles as C11/C++17 and abi-header compiles it | done | 58e0eb8 + pxui 6326d8c |
| DX7.2 | Size-prefix negative table | done | 9770273 + pxui 7a15162 |
| DX7.2a | Read PxuiGestureIn/PxuiSaved after their size check (found by DX7.2) | done | b399fd6 + pxui 61e0cf6 |
| DX7.3 | ABI policy in docs/abi.md | done | 337cd1f + pxui e387123 |
| DX7.4 | Minimum-size rule; abi_built; pxui_supports | todo |  |
| DX7.5 | Frozen identity hash; parent-scoped integer keys | done | ffe72c6 + pxui 6d85f9c |
| DX7.6 | Reserved surface id | done | 53377e7 + pxui e5be793 |
| DX7.7 | Record changes before freeze | todo |  |
| DX7.8 | Rays vocabulary to extension manifest | done | 7a93fa3 + pxui 3d16c6b |
| DX7.9 | Lint: records built from DEFAULT | todo |  |
| DX7.10 | Mach-O and ELF versions | todo |  |
| DX7.11 | Old-app fixture; abi-compat and abi-frozen gates | done | HEAD + pxui 6f2df11 |
| DX7.12 | Freeze 1.N (PERF) | todo |  |

## Latest performance

`make performance` with DX5.3 (kept views own their drawn text; pxui 5340fe9) (exit 0), lanes paused: twenty gated signed workloads pass with zero warm Rust allocations and no measured text layouts, idle-600 return now gated (74 -> 0). Measured: first navigation 1,244, first menu open 151 (DX5.6); the other eight measured workloads 0. Source: `/tmp/localwave-dx53b-performance.log`. Settled idle smoke passes (500.8 ms).

Gate (owner decision for this M1 Mac mini, 2026-11-15): build CPU p99 < 1,500 µs, frame CPU p99
< 2,500 µs, frame work wall p99 < 8,000 µs (half a 60 Hz vsync), zero warm allocations. Work wall is
the input-pump-to-draw-return span minus drawable/slot acquisition, which `pxui_host::Graphics`
reports. Build wall and work wall maxima are recorded, not gated.

Evidence for the decision: in a System Trace of LongScroll (`/tmp/localwave-p6-system-trace-long-scroll.trace`),
measured builds were never Blocked. All off-CPU time was Preempted (VM_compressor, WindowServer,
fseventsd, another app's display link, the GPU work loop), with load average 7–25 and 21 GB of
swap in use. 83% of build CPU ran on E cores. The same build costs a median 312 µs on P cores (max
547) and 896 µs on E cores (max 1,298). A foreground task-role experiment did not change residency.

Times are µs; CPU is p50/p99/max, build wall is p50/p95/p99/max, work wall is p50/p95/p99/max.

| Workload | Build CPU | Build wall | Frame CPU | Work wall | Alloc max | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| songs-original | 698/977/1023 | 781/1890/3885/- | 1129/1544/1579 | 1213/2784/4243/5408 | 0 | pass |
| find-filters-title-descending | 652/878/928 | 698/1144/1540/- | 1093/1355/1437 | 1022/1653/2402/2504 | 0 | pass |
| queue-occurrences | 749/948/979 | 829/2366/3077/- | 1249/1559/1637 | 1287/3331/3904/3984 | 0 | pass |
| queue-carry-scroll-cancel | 670/1043/1090 | 676/1250/2070/- | 1094/1710/1726 | 1032/2462/3269/3597 | 0 | pass |
| full-queue-playlist-editor | 434/537/541 | 451/1171/1594/- | 862/1045/1086 | 797/2191/2430/3910 | 0 | pass |
| albums-artwork | 666/1015/1034 | 720/1961/3085/- | 1293/1721/1787 | 1424/3962/4551/4919 | 0 | pass |
| waveform-scrub-and-scroll | 516/941/1030 | 586/1788/2677/- | 939/1506/1632 | 947/2922/3427/4932 | 0 | pass |
| settings-menus-appearance | 562/896/938 | 608/1548/2508/- | 982/1397/1404 | 1018/2913/3793/4574 | 0 | pass |
| files-indexed-directory | 873/1079/1106 | 994/2533/3196/- | 1317/1629/1707 | 1404/3114/3584/5476 | 0 | pass |
| external-drop-hover-and-cancel | 349/471/472 | 366/723/1226/- | 856/1139/1142 | 792/1524/3006/3375 | 0 | pass |
| artists-measured-columns | 695/911/1007 | 818/2419/3049/- | 1111/1354/1396 | 1228/3470/3865/4126 | 0 | pass |
| songs-playing-indicator | 787/1012/1039 | 902/2130/2968/- | 1239/1512/1548 | 1321/2743/3819/3836 | 0 | pass |
| songs-long-scroll | 833/1102/1145 | 908/2160/2624/- | 1271/1500/1546 | 1308/2877/4233/4757 | 0 | pass |
| album-detail-scoped-scroll | 533/671/846 | 556/1498/1613/- | 1010/1215/1373 | 1036/2161/3568/4080 | 0 | pass |
| artist-detail-scoped-scroll | 471/835/879 | 518/1506/2379/- | 924/1364/1368 | 994/2382/2814/3638 | 0 | pass |
| list-dialogs-menu-and-name | 506/1153/1309 | 626/2106/2658/- | 995/1678/1884 | 1203/2679/4070/6957 | 0 | pass |
| metadata-editor-typing | 923/1289/1317 | 1023/2349/2806/- | 1394/1895/1948 | 1558/3024/4072/4127 | 0 | pass |
| song-drag-carry-hover-cancel | 872/1125/1133 | 960/1953/2896/- | 1344/1665/1710 | 1415/2998/3314/3412 | 0 | pass |
| measured-fling | 580/826/910 | 615/1728/2087/- | 1024/1334/1530 | 1037/2535/2921/3328 | 0 | measured |
| measured-reverse-scroll | 809/1078/1087 | 900/2156/2775/- | 1297/1596/1609 | 1289/3179/4058/4135 | 0 | measured |
| measured-scroll-jump | 378/1391/1441 | 426/1452/1620/- | 845/1786/1859 | 836/2481/3278/3278 | 0 | measured |
| measured-non-song-lists | 466/1410/1449 | 746/2049/3103/- | 942/1851/1944 | 1262/2915/3751/6600 | 0 | measured |
| measured-search-typing | 519/656/658 | 560/1600/1879/- | 990/1270/1311 | 1014/2719/3593/3912 | 0 | measured |
| measured-first-navigation | 453/3233/3262 | 518/2202/3297/- | 963/3836/3982 | 970/2830/3866/4371 | 1244 | measured |
| measured-text-typing | 895/1111/1128 | 1003/2469/3512/- | 1347/1560/1580 | 1565/3378/4227/4273 | 0 | measured |
| measured-first-menu-open | 358/809/2261 | 363/1423/2108/- | 813/1215/2844 | 797/2443/3032/3327 | 151 | measured |
| measured-resize | 1240/1626/1653 | 1396/2419/3334/- | 1715/2158/2186 | 1921/3871/4882/5101 | 0 | measured |
| measured-theme-switch | 1270/1637/1683 | 1430/2563/3205/- | 1761/2281/2358 | 1835/3416/4646/6503 | 0 | measured |
| idle-600-return | 705/944/1014 | 782/2031/3736/- | 1162/1468/1481 | 1163/3217/4653/6084 | 0 | pass |
| clock-120 | 320/410/420 | 334/473/908/- | 760/1109/1113 | 700/1033/1499/1623 | 0 | pass |

### CPU profile

Separate signed `--performance-check --trace-frames` runs of the five P6a follow-up cases use the same
10,000-song, 1440×872, 182/82/100 fixture. All five fail the unchanged performance gate;
Rank by mean thread CPU over each run’s 100 measured frames, rather than wall time or an individual
outlier. Profiling is opt-in. LongScroll averages three P6j after runs; SongIndicators averages three
P6i after runs. Settings retains P6g, and the other three rows retain their P6a trace baseline. Settings additionally required input-pump attribution
outside the engine/output/draw phases; that work remains included in full-frame CPU.
The ABI 1.35 CPU clock sums every engine settling pass. Existing wall-phase times remain the
final pass. Output spans `platform.apply`; draw spans the public `Graphics::draw`, including
acquisition, preparation, recording, submission and image retirement. Other host/ABI/input work
remains in the full-frame total; these phase means do not exhaust it or measure GPU execution.
Raw CPU samples are retained with their corresponding wall/phase records before sorting copies.

| Workload | First CPU phase (µs) | Second | Third |
| --- | --- | --- | --- |
| queue-drag | draw 352.30 | build 206.62 | paint 175.45 |
| song-indicators | draw 318.05 | build 257.75 | paint 246.31 |
| waveform | draw 304.78 | paint 205.84 | build 175.21 |
| artwork | draw 448.75 | build 281.43 | paint 190.28 |
| long-scroll | draw 315.46 | build 272.14 | paint 255.85 |
| settings | draw 349.73 | paint 138.97 | build 99.37 |

Sources: `/tmp/localwave-p6-bindings-trace-{queue-drag,song-indicators,waveform,artwork,long-scroll}.log`.
SongIndicators uses `/tmp/localwave-p6-row-state-paired-{2,3,6}-after.log`.
LongScroll uses `/tmp/localwave-p6-place-paired-{2,3,6}-after.log`; all three also have zero
warm Rust allocations/layouts and passing frame-82 audits, while latency still fails. Settings
uses `/tmp/localwave-p6-background-settings-clean.log`, also zero/zero with a passing audit. Full phase
means, aggregate CPU/wall timings and failing exits are preserved in each corresponding commit. Earlier baselines remain in their corresponding commits.

Temporary signed host probes before P6j split draw CPU further. Means cover 100 measured frames;
acquisition includes drawable/command-buffer setup, encoding includes native pass creation and
renderer commands, and submission includes image retirement. They do not measure GPU execution.
The probes were removed before qualification.

| Workload | Acquire CPU (µs) | Prepare | Encode | Submit |
| --- | --- | --- | --- | --- |
| long-scroll | 128.16 | 9.21 | 144.91 | 40.78 |
| queue-sheet | 121.29 | 8.26 | 146.09 | 40.70 |
| artwork | 118.72 | 9.62 | 234.67 | 44.30 |
| settings | 143.06 | 10.62 | 143.94 | 47.03 |
| song-indicators | 126.69 | 8.91 | 142.88 | 43.00 |

A second probe separates pass creation/renderer encoding: LongScroll 73.13/62.64 µs and Artwork
82.23/184.42 µs. Sources: `/tmp/localwave-p6-gpu-cost-{long-scroll,queue-sheet,artwork,settings,song-indicators}.log`
and `/tmp/localwave-p6-gpu-encode-cost-{long-scroll,artwork}.log`. Subphase sums match outer draw CPU
within 5 µs; these diagnostic runs retain zero warm Rust allocations and their timing failures.

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
| P6f / 20×64 tiles, nineteen new rows/frame, mean paint, 100 samples | 53.90 | 72.66 | 0 |
| P6g / precreated native colors, mean setter CPU, 100 calls | 3.804 | 0.066 | — |
| P6g / signed Settings full-frame CPU p99 | 2334 | 1157 | 0 |
| P6h / 20×64 tiles, three new rows/frame, mean paint, 100 samples | 33.26 | 22.55 | 0 |
| P6h / 20×64 tiles, nineteen new rows/frame, mean paint, 100 samples | 70.53 | 45.85 | 0 |
| P6i / SongIndicators playback transitions, full-frame CPU maximum, 3×5 samples | 1862 | 1385 | 0 |
| P6j / 20×64 tiles, three new rows/frame, warm repeat mean paint, 100 samples | 20.78 | 18.69 | 0 |
| P6j / 20×64 tiles, nineteen new rows/frame, warm repeat mean paint, 100 samples | 45.35 | 40.23 | 0 |
| P6j / 20×64 unchanged freshly rebuilt tiles, warm repeat mean paint, 100 samples | 23.19 | 20.11 | 0 |

P6j's initial rebuild measurements and signed LongScroll comparisons remain mixed; its warm
microbenchmark repeats do not establish a controlled whole-app speedup. Failure evidence remains
in the commit. The unchanged-fresh case also checks no dirty uploads, exact tiles/geometry and
bounded storage without changing the existing scrolling cases' warm-up.

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
P6e reduces compactions for three new rows/frame from 100 to 50 per 100 frames. P6f checks
actual fresh write ends against the same budget instead of adding all fresh data to old storage;
this also reduces the 19-of-20 replacement case from 100 to 50 compactions. Largest retained
length stays 2,561 under its unchanged 2,690 bound (three-row case: 2,049 under 3,714).
The regression preserves exact row order/geometry, footer, dirty bounds and zero warm allocations.
Raw nineteen-row mean paint CPU is 53.90→72.66 µs in separate runs; three-row control is
31.12→42.82 µs. These measurements do not establish a CPU improvement. Failed baseline assertions
and slower timings remain in commit evidence. All signed latency gates still fail in the matrix.
P6f verification: 401 framework cases pass in 13.112 s; 209 LocalWave cases pass in 10.369 s,
zero skips, with one/five existing isolated contention retries. Dependency and diff checks pass.
The separate clean signed LongScroll trace also fails: build CPU 893/1166/1199 µs, wall maximum
3,886 µs, zero allocations/layouts and a passing frame-82 audit.

P6g found the Settings outliers in the input pump, not drawing: frames 99/139/179 spent
1,509/2,629/1,916 µs there. Sampling captured `setBackgroundColor:` recursively invalidating
native views, then AppKit display work during SDL pumping. Opaque, full-content Metal windows
now retain a window-owned dynamic fallback color; updates avoid that recursive invalidation.
The provider keeps no window/layer reference, uses a lock for concurrent color resolution and
releases at window teardown. Other window paths retain the Cocoa setter. No public UI ABI changed.
The native regression checks eight RGBA colors under both appearances, legacy frame-raster parity
after resizing, partial/transparent/non-Metal refusal, idempotence and weak teardown. The SDK test
covers real background updates, resize, dead tokens and zero warm Rust allocations. The native
microbenchmark excludes color creation and deferred redraw; allocation claims come from API/frame
checks. Full Settings p99 is 2,334→1,157 µs across signed matrices; these remain separate runs.
Final clean Settings: build CPU 340/628/631 µs, full-frame CPU 822/1117/1135 µs; wall maximum
2,816 µs still fails. Both Rust counters/layouts remain zero, with all eight themes exercised.
P6g verification: 440 framework cases pass in 18.471 s; 209 LocalWave cases pass in 16.710 s,
zero skips, with two/four existing isolated retries. Dependency/diff and bundle checks pass.
Temporary GPU/App probes were removed. Earlier timings/failures remain in commit evidence.

P6h follows a temporary signed LongScroll probe: mean paint CPU splits into visit/paint 143.84,
retained placement 131.89, view refresh 30.02 and settling 12.29 µs over 101 post-warm-up passes.
The probe and sample evidence are in `/tmp/localwave-p6-hot-{paint-probe.log,long-sample.txt}`;
they were removed before qualification. Compaction now compares borrowed 64-byte instances rather
than copying both into temporary Options. Equality, damage, dirty ranges, moves and ownership are
unchanged. The existing scrolling regression checks every row/tile, retained footer, geometry,
zero warm allocations, the same 50/100 compactions and bounded 2,049/2,561-instance storage.
Before/after: `/tmp/localwave-p6-merge-{before,borrowed}.log`; a warm repeat measures 20.82/45.49 µs.
Release test execution is 0.032/0.029/0.051 s; first release rebuilds take 53.18/50.81 s,
while the repeat builds in 0.07 s. No profile or test deadline changed.
The full signed matrix retains slower results too: LongScroll build CPU p99 1197→1217 µs,
full-frame p99 1760→1798 µs, compared with P6g. The separate clean P6h trace measures
778/1061/1204 µs build CPU and 1214/1577/1688 µs frame CPU; wall maximum 4,587 µs still fails.
Mean paint is 264.58 µs, but all phases vary between runs; the isolated compaction benchmark
supports the narrower improvement claim. All 13 allocation/layout/audit checks pass unchanged.
P6h verification: 440 framework cases pass in 25.623 s wall (19.353 s case sum);
209 LocalWave cases pass in 9.417 s wall (13.792 s case sum), zero skips. Five/four
existing cases pass on the runner’s isolated contention retry. Native pixels, retained geometry,
input, text, ownership, allocation, bundle and dependency/diff checks pass.

B3a corrects the scheduling predicate for a plain node inside a kept view: its retained instance
range and owner visibility remain meaningful even when it was not painted again this frame.
Visible loops keep requesting frames; clipped or transparent owners do not. Root-owned nodes keep
their existing behavior. The native regression checks every pixel at 1×/2×, one body build across
18 changing frames, and zero warm Rust allocations; it fails on the previous implementation.
441 framework cases pass in 26.859 s; 209 app cases pass in 14.257 s, zero skips, with six/fifteen
existing isolated contention retries. The new native case takes 0.613 s in the full suite.
B3a changes no renderer, layout, cache storage, ABI or application code.

The overscan-retention experiment was **rejected** after alternating baseline/candidate runs.
Fewer row builds traded against more retained paint and accessibility work. Phase means below
average three 100-frame runs; frame percentiles list each run separately, never an averaged percentile.

| LongScroll CPU (µs) | Baseline | Rejected candidate |
| --- | --- | --- |
| Build phase mean | 287.76 | 255.25 |
| Paint phase mean | 277.19 | 296.66 |
| Accessibility phase mean | 81.64 | 86.42 |
| Frame p50, each run | 1387 / 1260 / 1273 | 1310 / 1281 / 1326 |
| Frame p99, each run | 1796 / 1638 / 1651 | 1650 / 1664 / 1831 |

All six runs fail the unchanged gate, with zero warm allocations/layouts and ready audits.
Sources: `/tmp/localwave-p6-overscan-paired-{1,4,5}-baseline.log` and
`/tmp/localwave-p6-overscan-paired-{2,3,6}-candidate.log`. The 13-case candidate and restored
matrices, slower runs and prototype test failures remain in commit evidence. Cache warming,
ancestor bookkeeping, short-run padding and eager hit buffers are all reverted; only the independent
B3a scheduling correction is kept. Its clean LongScroll trace still fails: build CPU 790/1090/1336 µs,
frame CPU 1285/1577/1886 µs and build wall maximum 6,106 µs. No timing improvement is claimed for B3a.

P6i separates the shared catalog/geometry revision from each row’s current-song flag. Only its
current visual depends on play/pause; text stays cached. Track changes update the old/new current
rows, while width changes still invalidate all affected geometry. The public-API regression alternates
play/pause and current IDs across eight visible songs: eight row builds previously, now one/two,
zero copy builds on play/pause, correct accessible indicator placement and zero warm Rust allocations.
All eight song/table cases pass in 3.480 s; `make check` passes 210 cases in 13.827 s, zero skips,
with two existing isolated retries. The new case takes 0.031 s. No framework/ABI change is needed.

Alternating signed before/after runs retain unchanged clocks, wheel input and 182/82/100 frames.
All 15 predefined playback transitions (frames 101/121/141/161/181 across three runs) have build CPU
below 1 ms after the fix; their full-frame CPU maximum falls 1862→1385 µs. Four after-transition
frames are below the existing outlier-print threshold (both build wall and full-frame CPU <1000 µs);
all other transition records are retained. This does not replace whole-workload percentiles: before
build CPU p99 is 1259/1172/1077 µs, after is 1051/987/1155 µs. All six runs still fail the full gate.
Sources: `/tmp/localwave-p6-row-state-paired-{1,4,5}-before.log` and `{2,3,6}-after.log`;
`/tmp/localwave-p6-row-state-transitions.json` retains every logged transition and the bounded four.
The separate final LongScroll trace has build CPU 763/949/968 µs, frame CPU 1200/1455/1465 µs and
build wall maximum 3,809 µs (fail). Earlier slower runs remain recorded; native memory is separate.

P6 closes under the owner-approved M1 limits (P6k). Native/process allocation and real
device/provider behavior remain open.

## Open qualification

- Physical USB iPhone/iPad signing, install, launch and 120 Hz touch/scroll behavior.
- Real background audio, interruptions, route changes and system media controls on devices.
- Live iCloud/Files providers: original picker grants, downloads, offline recovery, account changes,
  moved/expired bookmarks and reconnect. Local URL/synthetic checks do not prove provider behavior.
- Actual sandbox Finder drops, native full-screen/Spaces gestures and supported macOS runtime checks.
- Interactive IME/candidate selection, dictation, floating keyboard avoidance and device editing feel.
- Real-user-container migration, distribution provisioning, TestFlight delivery and a release cycle.

## Known gaps

- Performance: all thirteen workloads pass the P6k limits. P-core builds stay near 300 µs;
  further engine CPU work (paint visit, placement, compaction, a11y emit) remains worthwhile.
  Cold glyphs, other scripts/native font discovery and native/process memory need separate evidence.
  A text/core edit check took 1.60 s, exceeding pxui's 1 s warm-edit target.
  P3 removes Now metadata string hashes and caches stable pages/rows across ticks; P6 hot-path work remains.
  Whole-framework Clippy has existing dependency/integration-test warnings; changed production,
  ABI/public tests and the full LocalWave workspace pass their scoped checks.
- Shared Swift checks fail two child source-notification tests, also in an isolated run on unchanged
  Swift sources. Six Swift/Rust exchange cases pass. Investigated in F9 on macOS 27.0.1: a file
  presenter gets coordinated writes reliably, but a directory presenter receives subitem
  notifications only intermittently (about 1 in 12 coordinated writes, in-process or from a child
  process, in /tmp, /Users/Shared and $HOME; registration delay and run loop servicing do not
  matter). An in-process directory presenter also makes the app's own coordinated reads under it
  wait for presenter registration (75 ms–4.9 s measured, 1–4 ms without), so macOS source
  observation uses FSEvents instead (started and released on its own queue; drop never blocks).
  Real FSEvents delivery is on demand only: this host's fseventsd ran at ~100% CPU with 5 GB
  resident and delivered in 2 of 6 standalone probes, with stream start taking 53 ms–5.3 s. iOS
  keeps an NSFilePresenter, unqualified until I2.
- F3: the submenu chevron and group dividers are absent.
- F-gate palette/Now/queue/transport: the AirPlay route button is deferred (iOS-only in Swift: an
  invisible image on macOS; needs AVRoutePickerView through lw_apple_sys on iOS). Swift's mini
  "Open now playing" card in the transport (panel hidden) is not ported; a Queue key shows beside the
  mode keys only while the panel is hidden, since Swift opens the sheet from the track menu, which Rust
  lacks. Desktop transport has no Stop key or volume meter (Swift: play key cancels loading, volume is
  ⌘↑/⌘↓); Intent::Stop stays for compact and media keys. Save as list is reached from the queue sheet
  header (secondary press or accessible activation), as Swift's context menu. The Now panel's Plays
  count refreshes on each recorded start; an empty-query palette lists commands statically, so its
  Rescan row appears when the worker's results arrive. Reference fixture seeds no play counts or
  album context, so Plays reads 0 and the sheet says "From Library".
- F4: the compact (narrow) layout has no New list button or list menu (⌘N and the track menu work);
  the sidebar list menu opens on secondary press only (no visible actions button); renaming the
  playing list leaves its queue context label stale until playback is reconfigured; Swift's list
  menu extras (Play, Shuffle, Duplicate, Keep offline) are not ported. Deleting a list returns to
  Songs because routes address lists by position.
- Sidebar parity (F-gate): Search, Sources (Reconnect badge, Browse intent), Add folder or files (picker), the six
  smart lists, New list last and the Offline meter follow Swift. Deferred: source rows show no selected state or scan
  progress/"scanning…" status, no source context menu or song-drop; the sidebar does not scale row heights to fit
  (the lists scroll instead); the offline-meter bytes use catalog availability, not a live provider probe; the
  Rust-only LOCALWAVE wordmark, Find/Files rows and Settings button are gone (use ⌘F, ⌘, the palette and source rows).
- Files parity (F-gate): source cards (name, path, "N songs", kept/on device/in cloud stats with bar, Swift's
  abbreviated relative scan age, expired-source Reconnect folder/Options, Offline space meter) and the folder page
  (Back, breadcrumb, "N songs · size · N offline", Play folder, Shuffle, Keep offline, legend, #/Name/Kind/Size/On
  device/Time table with greyed non-audio files, per-file availability glyphs) follow Swift; every label is formatted on
  the FilesWorker. Play/Shuffle send the page's saved songs as a Folder context, Keep offline is TogglePins; stats and
  glyphs come from worker probes and fall back to the stored availability when a file cannot be proven (Swift: dashed
  unknown). Goldens (before to after, mean abs diff / % px over 24): folders 21.4/19.7% to 23.0/15.5%, folder
  15.1/9.9% to 15.4/9.9%; content and numbers now equal Swift (the remainder is the sidebar, top spacing, glyph
  AA and Swift's DEBUG-only subtitle "Music" and "2m ago" strings: Rust shows the real path and Foundation's
  "2 min. ago"; the seed's Google Drive shows 22 on device because kept songs have real cached files).
  Deferred: no per-card scan progress/"Scanning" state (global progress bar only), no source-card context-menu
  extras (Play all, Keep everything offline, Add folder to queue), no row selection, double-click play, sorting
  headers, per-row context menu or drag,
  Play folder/Shuffle act on saved songs (Swift imports missing files first: use Import folder or the row's Import),
  the add-music sheet is two header buttons plus Refresh folders, the folder page keeps Rust-only Import folder and
  Folder options buttons (Rescan, Reconnect, Remove live in the options menu), the compact folder page reuses the
  table without Kind/Size and honours Show non-audio files only there, and button labels are left-aligned.
- F5: desktop album pages show the mono format line (first track, worker-prepared) and the underlined "Folder:" link (one `song_folders` query on the library worker; opens it in Files). Root-level albums link the source root; unindexed songs show no link; a narrow window keeps the format on the summary line (Swift wraps it). The artist album grid, 40 pt Back row and 52 pt Play/Shuffle/Keep offline keys are ported.
- Desktop parity: Folders now has only Swift's "+" (Add music sheet; Rescan is Settings' "Rescan all sources", now on desktop too). Desktop Find results are Swift's separate "Search / “query” / N songs · M albums · F folders" page when opened from the palette (`Intent::SearchResults`); ⌘F still opens the editable field + chips (Swift has no such page; it edits in the palette). Swift's desktop page lists no Albums/Artists/Folders sections, only the counts.
- F7: a song carry cannot edge-scroll Up next (pxui edge-scrolls only the carry's owner; queue
  occurrence carries transfer, song carries do not). Compact layouts have no Up next panel to drop on (the modal queue sheet hides the rows); the Up next target is the whole right panel with a signal outline; only song rows start a song drag (not albums, artists or Find rows); a list drop is AddToList with no confirmation notice.
- F8: the editor omits disc number (kept untouched) and has no compact-layout scrolling; Delete Album
  (Swift's album-cell menu) has no Rust album menu or command; deleting a queued song keeps its queue
  occurrence, as source removal does. Edit info and Delete act on the clicked song only.
- F9: folder observers start with the lazy Files worker (first Files use), not at app launch; moved-root
  notices only trigger a rescan, bookmark re-resolution handles the new location.
- F10/F-gate: onboarding matches Swift (matrix "00" counter shared with the Now clock, full-width Choose a
  folder, Pick some files, Google Drive disclosure) and persists `lwDidStart` through the preferences worker
  (set on Explore or once a source exists; an empty library no longer re-onboards). Still differs: "Try the
  demo library" (needs the 92-track bundled catalog and an isolated install; the link stays "Explore the
  library"), the folder glyph on the primary button, medium heading weight, the live song count
  (always "00" for an empty library). Golden diff 22.03 -> 2.15 mean, 10.8% -> 1.9% over 24. Empty library text replaces
  "No songs here yet." on Songs; compact Now omits its empty hint. Share… is in the track menu only (macOS,
  NSSharingServicePicker anchored at the click): Rust Files has no per-file menu, so Swift's Files Share and
  iOS UIActivityViewController remain. Share copies run on the LibraryWorker background slot (blocking
  playback preparation for large files); temp copies are swept after a day, not on picker completion.
- F1: Swift's Esc that cancels folder selection has no Rust counterpart (Rust Files has no selection mode).
  Return does not confirm the metadata editor or the Rust-only delete-song dialog (Swift's editor has
  no default action; its song delete has no dialog). Delete (Backspace) removes from a list route only.
- macOS parity gate: complete 1440×872 reference goldens, Swift shortcut parity and new-screen workloads.
  Goldens: `tools/reference-goldens.sh && tools/compare-goldens.py` (outputs and `index.html` in /tmp/localwave-goldens;
  needs the booted iOS 18 simulator and Screen Recording rights). Swift: DEBUG canvas hook at 1440×872; Rust:
  `--reference-screen` window via `screencapture` (pxui_host has no pixel readback); the window is 1440×(872+titlebar inset)
  and the inset is cropped, so both sides are the 1440×872 content area. The Rust seed mirrors LWReferenceFixture (play counts,
  last played, pins, waveforms, non-audio files, queue/context, paused at 1:41 on a real silent file). 17 of 19 screens
  compare (add: native picker; track: the anchored song-menu popup never paints in the capture). Latest (pxui 4dff60c,
  after sidebar/palette/Now/queue/Files/onboarding parity): mean abs diff 2.25-23.17, 2.1-15.6% of pixels over 24
  (onboarding 2.25, queue 2.59, find 6.24, artists 6.39, now 7.20, songs 8.52, commands 11.56, settings 14.73, folders
  23.17). Desktop parity step (docked Now panel on Folders/Settings, artist album grid + Back/play keys, Folders "+" and centred
  card buttons, Settings rescan row): folders 23.2 -> 8.9, artist 15.6 -> 7.0, folder 15.5 -> 9.9, settings 14.7 -> 8.1, album 9.9 -> 7.9
  (commands 11.5 and find-results 10.8 unchanged: the palette card sits 32 pt high only because the capture window includes the titlebar inset,
  find-results is Swift's separate results page). Remaining: Files fixture paths/"Music" DEBUG names, demo library; the rest is spacing and per-glyph antialiasing (CoreText-matching raster, see D).
- I2 mobile goldens: `MOBILE=1 tools/reference-goldens.sh && tools/compare-goldens.py /tmp/localwave-goldens-mobile` (Swift canvas 390×778 without
  LOCALWAVE_REFERENCE_DESKTOP; Rust: macOS window at 390×778 via LOCALWAVE_REFERENCE_MOBILE=1, which renders the compact layout; same
  titlebar crop; 17 screens, add/track skipped). Latest: mean abs diff 7.6-41.2 (onboarding 7.6, find 11.5, queue 16.6, find-results 18.5,
  lists 19.1, album 22.2, artists 23.2, list 23.3, settings 23.6, songs 24.5, favorites 25.1, folders 26.4, folder 28.5, commands 31.2,
  shortcuts 39.1, artist 41.2, now 7.7; the `now` capture opens the full-screen player). After the Songs/Artist/tab parity step: songs 19.9, artist 19.9 (was 24.5 / 41.2), artists 21.0, lists 18.6, find 11.2.
  After the phone parity step (Settings, Files, album/list leads): settings 18.2 (was 23.6), folder 26.8 (28.5), folders 25.4 (26.4), album 19.9 (22.2), favorites 23.1 (25.1), commands 28.6 (31.2), shortcuts 35.5 (39.1), artist 19.4; `list` now opens the same "Late drive" playlist on both sides (6a817d7): 21.6 phone, 8.6 desktop.
  Compact header metrics: there is no global offset (the phone content origin matches Swift: onboarding, album, list, favorites within 1 px); the library, Lists and Folders headings sat 4 pt low (title cap-top 18 vs 14 / 20 vs 15) and now match (songs 19.9 -> 15.1, artists 21.0 -> 18.8, lists 18.6 -> 18.5, folders 25.4 -> 25.3, commands 28.6 -> 27.5, shortcuts 35.5 -> 34.3); `list` now opens "Late drive" in Swift too (21.6).
  Compact Find, palette and shortcuts parity: Find has Swift's phone header (caption, 26 pt query field with 1.5 pt ink rule and a plate-less 44 pt clear target, chips, bold-filter hint or "Songs N" label), the palette/shortcuts cards sit at 12% of the window below the status inset, palette rows are 44 pt and its placeholder ellipsizes, shortcut labels wrap in the left half (find 11.2 -> 9.3, commands 27.5 -> 11.4, shortcuts 34.3 -> 12.8, find-results 18.2 -> 16.5). Open: the `cloud` find-results capture matches 0 songs in Rust (availability is not probed in the fixture; Swift shows 30), phone Find dropped the desktop Play all / Keep offline buttons (Swift's "Play results / Keep results offline" keys after the list are not ported; rows still play the result queue), no Albums/Artists/Folders result sections, and Swift wraps "Small player / full panel" after the slash where Rust wraps after "full".
  Select/menus/Files-tabs step: album 19.9 -> 15.9, favorites 23.1 -> 23.2, list 21.6 -> 21.7, folder 26.8 -> 26.6, folders 25.4 -> 25.3 (the fixture shows no extra rows for the new tabs; Swift's list summary reads "8 songs" without minutes, still "8 songs · 39 min" here).
  Gaps: Songs has no sort menu (the "Title ↓" label is static), no per-row dividers, no play-count glyph
  (Swift's compact row has none either), the A-Z index drags with a mouse only (touch has no press-and-drag signal; touch taps jump) and
  non-ASCII initials share "#"; the artist page lists every song (Swift prefixes 12); tab-bar cells lack Swift's boxed dividers/marker.
  Phone Files: the compact folder page (back/count, title + options, Play folder/Shuffle/Keep offline keys, All / Offline / In cloud tabs with worker-prepared counts and row lists, 56 pt rows with the availability glyph and "FLAC · size · time") replaces the desktop table; tapping a saved audio row plays it inside the folder's context (`Intent::PlayFile`, desktop rows still do not), tapping an unsaved one imports it, and each saved row has a "…" song menu. The Folders "+" opens an Add music modal (A folder / Some files). Source cards already show Swift's production strings (display names are DEBUG-fixture-only in Swift). Still missing on Files: Share is macOS-only as elsewhere, and the error text truncates. Phone Settings now keeps controls beside labels, adds Show non-audio files and Rescan all sources. Album/list pages lead with Swift's back row, Select ("Edit" on playlists) and "…" buttons, 150 pt artwork + title/artist/summary (album), title + summary (lists, Favorites), Play/Shuffle (albums add Keep offline). Select mode (`Intent::SelectMode`, reusing the F6 `marked` selection): checkboxes, tap toggles, Select all/Deselect all, "N selected", Done, and a bottom deck (Play next, Add to list, Offline, Queue or Remove in a playlist) that leaves the mode after acting; the "…" page menu (`Intent::Page`: Play, Shuffle, Play next/Add to list/Go to artist on albums, Add to queue, Keep offline, Rename/Delete on playlists) acts on the displayed collection in the controller; album rows show track numbers and "12.3 MB · in cloud". Select also covers Songs (Select beside the title sort), Artist, Find and Folder pages (folder marks index the shown rows; its deck acts through `PageAction` on the marked saved songs); playlist Edit shows a drag handle per row (`ui.carry` + the F4 `MoveListRow` drop target); the list menu has Duplicate (`PageAction::Duplicate` -> one `CreateList` "<name> copy"), the page Add to list leads with "New list…" (`NewPageList`/`CreatePageList`), album "Show in folder" uses `Request::Locate` on the FilesWorker, folder rows have a Open/Play/Shuffle folder menu (worker-prepared `Entry::songs`) and the "N non-audio files hidden · show" footer toggles Show non-audio files. Not ported: album source-folder footer, Keep offline on folder rows (Swift has none), drag auto-scroll, the numbered-row equalizer.
- S: Rust macOS signing, migration/release pipeline and a regression-free release cycle before Swift removal.
- I1–I2: compact swipe/long-press/refresh flows are in (pxui 1.38 `Ui::gesture`; Swift's swipe is Play next right /
  Add to queue or Remove left with a 88 pt commit, which Rust now matches on pxui 1.40 `swipe_actions` (Favorite lives in the long-press menu);
  the backdrop blur needs device qualification; refresh ends on the next catalog delivery or after 5 s; no a11y custom
  actions for the swipe, the row's actions button remains the alternative). The compact full-screen Now player follows WaveNowView (header with Playing from, art region,
  title/heart, 72 pt matrix clock with format summary and remaining time, waveform scrubber, Shuffle/Repeat/N up next, prev/play/next bar;
  Open/Close Now playing, slide-in via `motion::enter`). Drag-down dismiss runs on pxui 1.41 `GestureOpts::dismiss` (player root; the queue popup follows the finger via a state cell, because moving only a node inside a popup loses the touch). Deferred: true
  mini-player morph, AirPlay route picker and track-actions button in the header, medium title weight on the close glyph/key fills.
  Remaining: sheets,
  390×778 goldens and device/background/provider qualification before iOS shipping.
- D: owner policy: VoiceOver before the first Rust macOS release. pxui a0a0cb8 adds the macOS NSAccessibility bridge
  (one element per a11y node, sync on revision change, actions via inject_a11y; tree built only while VoiceOver runs or
  PXUI_ACCESSIBILITY=1); LocalWave enables it outside performance runs. Not yet qualified with a live VoiceOver session;
  missing: custom rotors, table/column semantics, live announcements, text value/selection setting, scroll-to-visible.
  Group opacity remains deferred. Antialiasing: pxui 4dff60c rasterises Apple text
  CoreText-style (unhinted, ppem-quantised origins, rounded baseline, 0.006·ppem stem growth, shader light-text growth;
  ABI 1.37); mean MAE vs CoreText 1×/2× Host Grotesk 1.86/0.99, Geist Mono 2.40/1.69 (pxui tools/glyph_ref.sh).
  Geist Mono 20 pt stays ~4 (layout drift, open). pxui static abi_bench pages meet the
  plan limits again (pxui 25873cc: Overlays 1.0101/1.011, Layout 1.0095/1.01). Decide accessibility release policy before S. Keep shipping Swift targets until their gates pass.
