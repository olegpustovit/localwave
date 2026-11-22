# Rust port status

## Steps

Follow the [implementation plan](https://artifacts.microapps.space/a/w474jy87pnxg#plan) in order.
DX rows follow the [API redesign plan](https://artifacts.microapps.space/a/mr55jcx8k90t#plan); its open rows are
superseded by U rows, which follow the [pxui Unification Plan](https://artifacts.microapps.space/a/nnhryen1cd09#plan).
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
| DX1.12 | Full public-surface audit against the proposal before the freeze (blocks DX7) | superseded by U1a–U1c, U2a–U2b, U6.2–U6.3 |  |
| DX1.12F2 | Audit batch F2: Keyframes/Repeat/Keyframe constructors, gesture option builders, borrowed carry/drop, Ui::scroll, Ui::set_view | done | 2f5ac3d |
| DX1.12F2a | pxui_render GPU tests use set_view; ellipsis golden follows DX-BUG1; landing compiles every workspace test target | done | fdf646d + pxui e659280 |
| DX1.12E | Audit batch E: text inputs — TextField style, one keyed text_field, key-first field widgets with response structs, call-scoped callbacks, zero warm allocations | done | c6cd355 + pxui cf953b3 |
| DX1.12A | Audit batch A: Label is the one text-style input (TextStyle opaque, from resolve), enums for wrap/ellipsis/align, label exports resolve in-call | done | 1efcf4d + pxui 763df85 |
| DX1.12F1 | Audit batch F1: old-shape widgets key-first with Copy styles and Response; keyed size-prefixed C records (no in-band key encoding); one checkbox/table/picker; overlays share Overlay | done | 0cd725a + pxui b6958651 |
| DX1.12G | Audit batch G: body vocabulary follows the public calls; sealed per-kind body config wrappers | done | 7c9c670 + pxui 98ca495 |
| DX1.12C | Audit batch C: geometry/colour/theme/layout/draw/a11y value types, one const shape each | done | bb8df23 + pxui 8d0e1f4 |
| DX1.12B | Audit batch B: paint Rect + Fill/Stroke, Shapes private, images by id with checked exports (ABI 1.70) | done | 02fc990 + pxui 11a546d |
| DX1.12D | Audit batch D: one container/view/state shape, Chosen, borrowed looks, Handle (ABI 1.72 ui_view_keyed) | done | HEAD + pxui d6b4182 |
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
| DX3.13 | Menu rows keyed by id/position, chosen_id, safe body accessors, toggle stored value, segment container (found by DX4.5–4.7) | done | c4d4617 + pxui e0c424d |
| DX3.14 | Looks name colour tokens (const app looks follow the theme); Icon::draw(f, word) (found by the DX4 review) | done | cdd976b + pxui cac8615 |
| DX3.15 | pressable takes a Pressable style (per-state looks, enabled, tab_stop, selected) and gives f the state; Response::secondary (found by DX4.11) | done | a7f9acd |
| DX3.16 | Menu::max_h, Pressable::focus_on_click, segmented_with(Option) -> SegmentedResponse with a radio-group keyboard model (found by DX4.5/4.6) | done | 1b2be80 + pxui 0968a85 |
| DX3.17 | Segmented::focus_on_click; segment bodies see the caller's stored choice (found by DX4.6) | done | 07012b0 + pxui 0aef02c |
| DX3.18 | ABI 1.71: Button icon/detail/accessory + label styles, IconButton::selected, DialogButton::detail (gaps G2, G3, G13; G11 deferred) | done | 3d7799f + pxui 28fc92d |
| DX4.1 | Palette to colour tokens | done | fbfe4be |
| DX4.2 | kit::text and settings::paragraph to label | done | 269af1c |
| DX4.3 | Buttons to button_with | done | 157587d |
| DX4.4 | Icon buttons to icon_button | done | 27c3e69 |
| DX4.5 | Menus to menu | done | cede8a0 |
| DX4.6 | Settings switch and chips to toggle_with/segmented_with | done | 435e02a |
| DX4.7 | Confirmation sheets to dialog_with; field variant | done | efa00fc |
| DX4.7a | Field body picks the Find structure by variant (part of DX4.7) | done | 3181aa4 |
| DX4.8 | Codemod A: Node literals to builders | done | 4091362 |
| DX4.9 | Codemod B: hashed view to view_of | done | ccbe842 |
| DX4.10 | Codemod C: node+within to containers | done | 51e19dc |
| DX4.11 | Hand conversion to pressable, one commit per file | done | ad48bd3 |
| DX4.12 | LocalWave review cleanups: id keys, named styles, theme bound once, press state in button contents | done | 0172279 |
| DX4.13 | API-conformance check in make check (lw_ui/tests/api_conformance.rs, shrinking allowlist; empty when DX4 completes) | done | 73512bf |
| DX4.15 | Shadow theme deleted: every look a named const over Rgba::token; colours stay the theme (gap: token with alpha) | done | a3c8ea1 |
| DX4.16 | Stable id keys instead of display text (song columns, settings back/options) | done | ad48bd3 |
| DX5.1 | Pinned glyph entries; label_digits | done | 9260c34 |
| DX5.2 | Debug report for prepared/requested style mismatch | done | f129f4e + pxui ef803a6 |
| DX5.3 | Viewport-distance eviction for list entries | done | de53440 + pxui 5340fe9 |
| DX5.4 | Engine prepare queue; fling exception ceiling | done | b873984 + pxui 62ddbcb |
| DX5.4b | LocalWave adopts the host-owned TextWorker: one worker, SongTextWorker deleted, prewarm policy, fling gated at the pxui ceiling | done | 40ccc78 |
| DX5.4c | A prewarmed natural-width layout serves draws at any width that cannot change it | done | 81f353c + pxui 7f2f738 |
| DX5.4d | LocalWave fling ceiling kept equal to pxui's (parsed test); prewarmed-cells test deterministic | done | 31e87c3 |
| DX5.4e | A layout prepared at width W serves draws between its measured width and W (left-aligned; uncut ellipsis) | done | f27b48b + pxui a3a8924 |
| DX5.4f | New nodes reuse a measured or worker-prepared layout at their measured width (recent-width index; installs recorded) | done | 6559e40 |
| DX5.5 | Reusable layout storage for fresh shapes | done | 71d78a2 + pxui 8bbb922 |
| DX5.6 | Remove remaining caller allocations in DX0.3 workloads | done | 39c4d96 (fling, reverse-scroll, scroll-jump, non-song-lists, search-typing, text-typing, resize, theme-switch gated at 0; first-navigation and first-menu-open stay measured: all their allocations are first-time shaping, glyph rasterising and node creation for never-drawn content) |
| DX-PERF2 | abi_bench: Overlays wrapper saving; stable small-page and start-up estimates (no limit changed) | done | eaa6541 + pxui 3cf7de3 |
| DX-BUG1 | Bugs found by the docs work: Images::insert mapping, a11y_value char boundaries, paint text layout-top placement, hex_exn twin | done | 5edae64 + pxui f51852f |
| OS-B3 | Production libpxui/apps use the system allocator; allocation counting is an explicit opt-in (tests, make performance), checked on both sides of the ABI | done | f2a2e68 + pxui 84752ba |
| OS-B4 | Hosts are multi-window: SDL events routed by window, dialogs return to their opener, AT-SPI serves many surfaces, per-window Wayland clipboard | done | f2a2e68 + pxui 93c7d20 |
| OS-B2b | Text, image and renderer bounded for untrusted callers: text length caps, font caps, layout byte budget, renderer records size-checked, resize capped, per-Ui image table | done | f2a2e68 + pxui 5d11b7e |
| OS-B2a | Every C export validated for untrusted callers (fuzz over the manifest), per-role caps, builders outside a frame refused, wrapper clips display text | done | 6559e40 + pxui c03699f |
| OS-B1 | Core has no process-global state or app policy: per-Ui body registry, per-Ui verify, floating roots are their own layers with roles, Rays kit staged in kit_ext | done | HEAD + pxui b1a4310 |
| OS-LW1 | LocalWave clips user metadata for display once (1024 B) where snapshots are built; stored data unchanged | done | 6559e40 |
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
| DX6.12 | Docs: kit widgets | superseded by U6.3 |  |
| DX6.13 | Docs: options | superseded by U6.3 |  |
| DX6.14 | Docs: inspector | superseded by U6.3 |  |
| DX6.15 | Docs: images and renderer | done | 0d0cd58 + pxui 48be183 |
| DX6.16 | Docs: node.rs | superseded by U6.3 |  |
| DX6.17 | Docs: layout and geometry | done | 0a5c610 + pxui 4c08a69 |
| DX6.18 | Docs: theme | done | c772eb9 + pxui 5b268d7 |
| DX6.19 | Docs: ui, a11y, draw | done | eb23944 + pxui 28893de |
| DX6.20 | Docs: pxui_types root | superseded by U6.3 |  |
| DX6.21 | Docs: ffi.rs (1) | superseded by U6.3 |  |
| DX6.22 | Docs: ffi.rs (2) | superseded by U6.3 |  |
| DX6.23 | Docs: ffi.rs (3) | superseded by U6.3 |  |
| DX6.24 | Docs: ffi.rs (4) | superseded by U6.3 |  |
| DX6.25 | deny(missing_docs); allowlist deleted | superseded by U6.4 |  |
| DX6.26 | pxui-app skill; README and specification fixes | done | 73512bf + pxui a50ab58 |
| DX7.1 | Generated layout manifest; C static asserts | done | 2c4bd61 + pxui 8b4dcae |
| DX7.1a | pxui.h compiles as C11/C++17 and abi-header compiles it | done | 58e0eb8 + pxui 6326d8c |
| DX7.2 | Size-prefix negative table | done | 9770273 + pxui 7a15162 |
| DX7.2a | Read PxuiGestureIn/PxuiSaved after their size check (found by DX7.2) | done | b399fd6 + pxui 61e0cf6 |
| DX7.3 | ABI policy in docs/abi.md | done | 337cd1f + pxui e387123 |
| DX7.4 | Minimum-size rule; abi_built; pxui_supports | done | 63f6734 + pxui 2d2a3db |
| DX7.5 | Frozen identity hash; parent-scoped integer keys | done | ffe72c6 + pxui 6d85f9c |
| DX7.6 | Reserved surface id | done | 53377e7 + pxui e5be793 |
| DX7.7 | Record changes before freeze | superseded by U5.5 |  |
| DX7.8 | Rays vocabulary to extension manifest | done | 7a93fa3 + pxui 3d16c6b |
| DX7.9 | Lint: records built from DEFAULT | superseded by U6.5 |  |
| DX7.10 | Mach-O and ELF versions | superseded by U5.11 |  |
| DX7.11 | Old-app fixture; abi-compat and abi-frozen gates | done | 3a07463 + pxui 6f2df11 |
| DX7.11a | abi-frozen accepts append-only record growth | done | 1d8fd2d + pxui 5330f36 |
| DX7.12 | Freeze 1.N (PERF) | superseded by U6.7 |  |
| U0.1 | Tools home tools/dx; fingerprint frame helper; lane machinery removed | done | 4cc530f |
| U0.2 | Drift meters: surface.py, api_conformance R1/R6/R7/R8 | done | 3310f2b |
| U0.3 | Build-time meter, API ledger, ledger delta | done | 3bd0004, 7073f41; pxui cefefe57, a64d00ca |
| U0.4 | Landed and superseded branches deleted | done | b444b1d |
| U0.5 | Infra traps: tools/t --all, prewarm fingerprint, layout_scaling | done | 8b13a62; pxui eb93fe54 |
| U0.6 | Status and process docs | done | 63972bf; pxui 3db577ab |
| U0.7 | CP0: Tier C baseline, performance and build-time tables | done | 16dc635 (CP0 red: abi_bench both modes, 4 build-CPU budgets) |
| U0.8 | Restore the CP0 build CPU gate | done | LocalWave 9c362bb and pxui 23bb10c4. Final signed run passed all 30 workloads at peak load 2.776. All 28 gated workloads had zero warm Rust allocations. Maximum build CPU p99 was 1.477 ms. The public surface matches. All 115 UI checks passed and their emitted fingerprints matched. Tier A, Tier B, all six CP0 build-time comparisons and online `make check` passed. See `docs/u-plan/u08-investigation.txt`. |
| U1a.1 | Modules (D2) | done | LocalWave f6b3f94 and pxui 1becc803. Callers use root or prelude paths. The namespace ledger and all Tier A checks passed. |
| U1a.2 | Theme (D1) and counter (D3) | done | LocalWave daa47b6 and pxui 62ddd318. Detached theme handles, cached palettes and strict root body-name validation. Gallery checks passed and cached theme swaps had zero warm allocations. |
| U1a.3 | Lifecycle and errors | done | LocalWave 70bef76 and b6a3d2b, pxui 96e7ca2d and 24e54d41. One frame entry preserves event and snapshot input. Layer, focus, accessibility, malformed input and GPU lease regressions passed. The Rust iOS device probe type-checks. |
| U1a.4 | Input (D23) | done | LocalWave f83c3a8, 9173aca and d3c434b, pxui e43ef1e4 and e10307b2. Shortcut regression passed before and after typed queries. ABI 1.75 and the 5,913-item approved surface passed. D9 matched across 9,263 frames. Tier A, Tier B, independent review and 288 online LocalWave checks passed. All six build-time values meet the CP0 allowance. |
| U1b.1 | One result idiom (D4, D10) | done | LocalWave dcca1b6, e2a9942 and ec0e38b, pxui ec95d41d, c46adee7 and e45204f3. PickResponse, SliderResponse, OverlayResponse, SplitResponse and Chosen; id-keyed segments keep identity through reorder. ABI 1.76. |
| U1b.2 | Keys and small returns | done | LocalWave f19b27e, 055307b, 6dcc3e2 and 20463e4, pxui 165fd9d1, c1ad78d0, cbb34aad and 0baa83c6. Key-first lookups; Key::Resolved never rehashed; every key-label export refuses unusable resolved labels (fuzzed). ABI 1.77. |
| U1b.3 | Measurement and units (D20) | done | LocalWave d873f01 and f127faf, pxui 90367067 and d143239c. One in-frame UiText; f32 font, row and padding sizes in existing record padding; inline allocation-free C theme projection; clock and drop overlay placed by baseline. ABI 1.78. |
| U1b.4 | Looks, views, a11y types | done | LocalWave 0da9123, pxui 5444131b. style(States::of), ViewTransform builders, A11yStates and borrowed A11yTree, Paint::tokens. ABI 1.79. |
| U1b.5 | Builders and vocabulary | done | LocalWave e7e9687, pxui 3f1afb57. Const builders, Progress/Spinner/Badge/GridList styles, Node-based scroll_view and text_area. ABI 1.80. |
| U1b.6 | Call-shape lock (D25) | done | LocalWave dada1a1, 1419a82, 13f5428, c2eeb47, 7224eca, 058f687 and 1a999a1, pxui 4ebb0002, 7e3c42e0 and 3805ffcd. Keyless containers plus ui.id, internal positional key namespace, single button/toggle, disabled, chosen_id; engine mirrors deleted; allowlist 439 → 308. ABI 1.81. Surface 6,071 items matches the ledger; fingerprints match U1a; gallery goldens unchanged; Tier B, independent review and 289 online LocalWave checks passed; all six build-time values meet the CP0 allowance. |
| U1c.1 | Generics (Batch K) and the gate | done | LocalWave c33d0af, pxui 81487978. node_mut and trampolines non-generic, canvas_with shell, no inline(always); gate shape rejects generic bodies and inline(always) callees (fixture test); shape allowlist 115 → 113. |
| U1c.2 | Globals (OS-2/4) and verify (D22) | done | LocalWave de687fa and 291d69d, pxui 0df757fb, 8731a0c9 and 81e96ee0. Menu rows capped at 4096 with a refusal test; per-Ui verify_views in debug; wrapper globals documented with bounds and thread rules. |
| U1c.3 | Extension set (D6) | done | LocalWave 821a549 and 6ab2781, pxui dc18bb81 and 9f36189d. 16 extension calls in pxui::ext; inspector toggle and button through their bodies with a custom-body test. |
| U1c.4 | Spec sync | done | LocalWave 6b2b34d, pxui 66c79d42. Ledger and api_spec synced; surface.py --ledger green for every U1 row. |
| U1c.5 | Recover release abi_bench (D26, D27) | done (finished in U2b.10) | LocalWave b982b17, 97bda86 and 687730d, pxui ff71546e through 3d63d11a. Author and every bodies page pass; text inputs answer in one call (ABI 1.82); colour tokens resolved in place; text engine boxed. Kit, Controls, Overlays and Layout stay over in both modes with unchanged limits; per D27 U2b.10 owns them before CP1. Tier B and 289 online LocalWave checks passed; build times within 0.25 s of CP0. |
| U2a.1 | Row keys (dx3.22) | done | LocalWave 8e75a94, pxui c7e6e27e. selection_list and table_view key rows by item id through new row exports. ABI 1.83. |
| U2a.2 | Tabs (dx3.19, G1) | done | LocalWave 16f3df4 and 395e6c2, pxui df47dc48 and 29bfff24. tabs -> PickResponse with focus kept when a choice is refused; the gate's trailing-append growth rule (library-written or size-checked caller records, library code only); the library refuses an app built for a newer minor. ABI 1.84. |
| U2a.3 | Checkbox, disclosure, section index (dx3.20; G5, G9, G10) | done | LocalWave 19eb6d7 and a59e3b2, pxui 87afd1f5 and a6e1b98b. Checkbox<'a> with size, look, passive (READ_ONLY) and disabled; disclosure; section_index -> PickResponse with a section_index body kind. ABI 1.85. |
| U2a.4 | Sheets and picker rows (dx3.21; G6, G12) | done | LocalWave fda5e5f, 1310c3b, bef5c8e and 241ec6e, pxui cde8267a, 404943c2, 39688c00 and e1acbdb9. One validated sheet route with Presentation and DismissOpts; PickRows::push(PickRow) with headings and glyph art; duplicate layout rows fail. ABI 1.86. Each new widget has custom-body, warm-allocation and a11y tests, a gallery entry and a bodies-page override; only the gallery_bodies goldens changed. Tier B, independent review and 289 online LocalWave checks passed; build times within the CP0 allowance. |
| U2b.0 | D28 process change | done | LocalWave 3b7171e, pxui 5c368579. 1.x ABI not frozen; abi-snapshot a change detector; U2c added; abi-frozen/abi-compat leave Tier B until 2.0. |
| U2b.1 | Selection list and table (rest of G4) | done | LocalWave 385078e, pxui 29bd1fcb. App-owned ListState/TableState; selection semantics in the widget; column table and heading. ABI 1.87. |
| U2b.2 | Drop zones (G7) | done | LocalWave 6387233, pxui a17b9e13. Any node takes a drop; reorderable lists report drop_at and draw the insertion line; SelectionList::drop_look. ABI 1.88. |
| U2b.3 | Reorder (G8) | done | LocalWave 40f9dd3, pxui 7819ae5d. A row's Part::HANDLE drags it, passed rows make room along motion.slow; Alt-arrow and Move up/down actions; ListResponse.moved; zero warm allocations dragging. ABI 1.89. |
| U2b.4 | Gap steps raised during U3–U4 (reserved) | reserved | Opened by the GAP protocol in U3–U4. |
| U2b.4.1 | ButtonConfig label styles (gap from U4a.1) | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave e5aef58, pxui 6eadaa1c. ButtonConfig::label, detail_label, accessory_label: a Rust button body reads the caller's styles. |
| U2b.4.2 | Button built_in slot | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 9955721, pxui abc308b5. PxuiButtonConfig.built_in: a registered button body runs the kit's button for configs it does not style. ABI 1.99. |
| U2b.4.3 | SelectionList::bare | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 56c56bb, pxui c5abbaca. A list without its frame look or 1-point inset. ABI 1.100. |
| U2b.4.4 | Search field editor part | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave de4a958, pxui b4c20d7a and ec05e36e. A search field body's Part::EDITOR is the editing node, in its look's font. |
| U2b.4.5 | Node transitions | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 3055d0b, pxui 720226d6 and dbf0d940. A node's own transition runs its state changes under a still theme. |
| U2b.4.6 | Icon button built_in slot | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 5924d13 and 5ebcb1e, pxui 5abfcaa1. PxuiIconButtonConfig.built_in. ABI 1.101. |
| U2b.4.7 | (number not used) | not used | No step was opened under this number. |
| U2b.4.8 | SearchFieldConfig.line | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 39a6774, pxui 417f3127. A search field body sizes the field's line. ABI 1.102. |
| U2b.4.9 | Selected icon button look | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave ec06f5a, pxui 5eb35332 and e1f73dd6. A selected icon button keeps its checked look while hovered or held. |
| U2b.4.10 | section_list row CellState | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 40e13f8, pxui 1878806a. Reorderable section-list rows get their CellState and can be named. |
| U2b.4.11 | ListResponse::focused | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 6f0a967, pxui 6ef06ea5. A list says it holds keyboard focus. ABI 1.104. |
| U2b.4.12 | Ui::reordering | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 7e6374e, pxui cee1afed. An app spends Escape only on a reorder drag. ABI 1.105. |
| U2b.4.13 | Kept list rows | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 265091c, pxui afd2373e, 1c81a433 and 3751a6a8. ListState::revision and LIVE: a list keeps each row's build while its content holds. ABI 1.106. |
| U2b.4.14 | Culled kept rows | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 44c3b45, pxui 6501f528. A kept row past the viewport keeps its build until it scrolls into view. ABI 1.107. |
| U2b.4.15 | Picker editor part, images, headings, scrolling | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 88d13e1, pxui ac4ec089; fixes pxui 71667624 and ac35443f (pointers in LocalWave 1944df5 and 87272df). Picker field Part::EDITOR, Picker::image, PickerConfig.head, scrolling results; fixes: a settling frame keeps its copy/paste request, picker keys read in place, the search keyboard, the caret while focused. ABI 1.110. |
| U2b.4.16 | PickerFieldConfig.list | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 1944df5, pxui 56ca53ea. A picker field body lays out the results region (the line pattern). ABI 1.111. |
| U2b.4.17 | Tree revision: kept shown rows | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave d4e1c96, pxui fd9203b1. `Ui::tree(.., revision: Option<u64>, ..)` as `ListState::revision`, `PxuiTreeViewDesc.has_revision`/`revision`; a warm frame of a tree with a held revision walks no node (gallery Collections 113 to 54 us). Gap found by the U4a abi_bench wall gate. ABI 1.112. |
| U2b.4.18 | text_input_focused is a text editor | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave aa3a485, pxui e4ae5d28 (abi_bench build fix 7ca4a111). A focused widget takes the keys it handles from key_pressed; ListResponse::focused deleted (review finding 1-2). ABI 1.113. |
| U2b.4.19 | PickArt::Draw | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 6ff47eb, pxui e45e96f0. A picker row's app icon by its word, PickerConfig::art() (finding 4). ABI 1.117. |
| U2b.4.20 | One look priority | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 818e926, pxui d93a655b. disabled > checked > pressed > hovered > focused > idle for every kit widget (finding 5). ABI 1.118. |
| U2b.4.21 | Per-row drop acceptance | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 1e4d60f, pxui 0dc3eae4. ListState::NO_DROP; a list's drop look wins over the row body's (finding 7). ABI 1.120. |
| U2b.4.22 | (number not used) | not used | Reserved for finding 8 (table head look); LocalWave d4d5f71 took the look and rule from the app themes' table_head look with no pxui change. |
| U2b.4.23 | Search clear key state | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave dd9569b, pxui a46c66ed. `SearchFieldConfig.clear: State` / `PxuiSearchFieldConfig.clear: PxuiStateDesc` (120 to 128 bytes); the kit owns the clear key's hit flags and state, the body styles it (finding 9b; ruled `Handle`, built as `State` because a kit-declared handle is built before the body). The Find clear key now takes a mouse click. Finding 9a (autofocus as app policy): LocalWave 440a32d. ABI 1.122. |
| U2b.4.24 | Ui::focus_default | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave a16c1fc, pxui bb8ead5f. The default key target while nothing is focused (finding 3). ABI 1.114. Re-review R1 (pxui 61697c35, LocalWave d532195, ABI 1.123): a key the focused widget leaves untaken falls through to it. |
| U2b.4.25 | Escape clears a selection | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave a16c1fc, pxui 468f1be3 and 70b6b326. Escape clears a focused multi-selection; a cancelled carry spends it (finding 3). ABI 1.115. |
| U2b.4.26 | Enter and app-written selection | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave a16c1fc, pxui 83b2761a. Enter activates a selected keyboard row; an app selection moves the keyboard row (finding 3). ABI 1.116. |
| U2b.4.27 | Typed body-config accessors | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave a29429c, pxui 7a8a45d0. PickerConfig::image/head, ButtonConfig::width/height/content_axis, ModalConfig::presentation, PxuiRowConfig::DROP_* (finding 12). Rust only. Re-review R4 (pxui 517d61db, LocalWave f22ec66, ABI 1.123): TableHeadConfig::ascending, SegmentedConfig::selected/stored, ProgressConfig::value, C PXUI_ROW_DROP_*. |
| U2b.4.28 | TreeState | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave a5b1628, pxui c080d3ed. Ui::tree takes one &mut TreeState, as ListState/TableState (finding 13); C record unchanged. Finding 14 (pxui bbbb843d, LocalWave a9b6948): kept tree rows bounds-checked, re-derived on a broken promise. ABI 1.121. Re-review R3 (pxui 735e022a, LocalWave 61fe673): rows checked where read, O(rows in view). |
| U2b.4.29 | PxuiTabConfig.bar | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 3bc7e8e, pxui d89af298. A tabs body may restyle or resize the tab list. ABI 1.124. |
| U2b.4.30 | Body states are the caller's looks | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave add01e2, pxui adebdbd0. A body's `states` are the caller's looks as given (tokens resolved at apply). |
| U2b.4.31 | Tab bar re-tap clicks | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 9b9fb44, pxui 0020222c. A tab bar's response clicks for a press choosing any tab. ABI 1.125. |
| U2b.4.32 | List part variant | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 6416102, pxui 15b405f5. A list part's widgets leave the rows' body variant (bug fix, no ABI change). |
| U2b.4.33 | SelectMode | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 0f9b39b, pxui 5f4a1e46. `SelectionList::mode(SelectMode::{Single, Multi, Toggle, None})` replaces `multi()`. ABI 1.128. |
| U2b.4.34 | Section index value text | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 377e383, pxui 4d8e0ca7. A section index's value reads its current letter. ABI 1.126. |
| U2b.4.35 | ButtonConfig::danger | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 377e383 and 1a4fb56, pxui 3d4cc3cb. `Button::danger` is told to a body. ABI 1.127. |
| U2b.4.36 | TableHeadConfig.built_in | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave edea3ca, pxui f09c9113. A table heading body runs the kit's heading for tables it does not style. ABI 1.129. |
| U2b.4.37 | Table SelectMode, open_on_click, bare | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 7bb0f61, pxui 17068da2. `Table::mode(SelectMode)` replaces `multi()`; `open_on_click` and `bare` as SelectionList's. ABI 1.130. |
| U2b.4.38 | Dialog built_in (the kit's dialog buttons for a registered dialog body) | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave b038594, pxui 07cc8cac. `DialogConfig.built_in`; `sheet_body` hand-built buttons deleted; sheet keys are the button body's SHEET/FORM/CHOICES variants. ABI 1.131 (also the 1.124 caller-looks note and `PXUI_SELECT_*`). |
| U4b review | Independent review rulings 1-8 | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 27b747a, b73ff72, 0272c56, 07d7693, b038594, 40bf0df; pxui 77f9ab8d, 07cc8cac. Tier B green; abi_bench instruction gates pass (Lists wall flap deferred to CP1 f). |
| U2b.5 | Top-anchored overlay | done | LocalWave 7814615, pxui 755ed30d. Presentation::Top (Overlay::at's y, max_h cap); PxuiSheetDesc.top/max_h. ABI 1.90. |
| U2b.6 | Picker fill contract | done | LocalWave ef524a8, pxui 3a2277d1. Fill contract documented; a typed query and a kept worker result allocate nothing per frame (#82). |
| U2b.7 | Value and theme gaps | done | LocalWave 20e14dc, pxui 14d86f0d. Token alpha (D19), ColorToken::ACCENT, SearchField variant/action, Button::icon_size, icons in body configs (D18). ABI 1.91. |
| U2b.8 | Recorded engine bugs | done | LocalWave 50a3040 and ea23b12, pxui 65d304df, 53f92922 and 683ccd27. Gone-slot state, submenu placement, SwiftUI-parity baseline (D29), motion.slow, tooltip text_at; popup layer batched with the base (D26(3): Layout 1 draw call). |
| U2b.9 | Site map | done | LocalWave 2099764. tools/dx/site_map.md with the review fixes. |
| U2b.10 | Gallery, spec, bench | done | LocalWave f740817 through f8287f0 and the U2b status commit, pxui 147a2f63 through aa79017e; review fixes pxui 61ed76b1, LocalWave f00e8d8 and 07acbb5. Release abi_bench passes every page in both modes with unchanged limits (D27 closed); new Lists gallery page (reorder, drop zones, top modal) with only new goldens, its abi_bench mirror and bodies reference. ABI 1.92. |
| U2c.1 | abi_drop.py | done | LocalWave f7d3131. tools/dx/abi_drop.py lists C exports with no caller beyond pxui_abi and tests; --check fails outside its keep list (pulled forward from U5.3). 106 candidates at ABI 1.92. |
| U2c.2 | Widget twins | done | LocalWave 1a9325b, pxui 233baadd. Label/keyed/plain widget exports deleted; checkbox, segmented, tabs, slider, scrub, progress, spinner, badge, toasts, picker, dialog, tree, button, icon button, toggle, key_pressed take the plain name. ABI 1.93. |
| U2c.3 | Container, overlay and image twins | done | LocalWave 70ca98d, pxui a10c8625. panel, popup, modal, anchored, scroll_view, view, split, menu_bar, image take the plain name; col, box, enter/leave, list_measured, tooltip and the engine legacy bodies (tooltip_legacy, anchored_legacy, labelled image, inspector section view) deleted. ABI 1.94. |
| U2c.4 | Text input twins | done | LocalWave a34c700, pxui 52ad8bbd. text/search/line/value field and text area response forms take the plain name; input_box and the label forms deleted; a refused text field leaves its outputs alone. ABI 1.95. |
| U2c.5 | Theme, unit, constructor and setter twins | done | LocalWave 13d95af, pxui 4f23fbe0. ui_theme is the exact snapshot; set_theme (ThemeSource), set_colors, theme_set/load/apply, set_look, font_size/row_height, verify_new_views deleted; checked constructors and setters take the plain name. ABI 1.96. |
| U2c.6 | Text and paint text twins | done | LocalWave 8ced207, pxui 56bd3dde. Resolved-style text twins, integer-size paint text calls, ui_text_width/cap_width/text_top and the C ui_label deleted. 343 to 242 exports; abi_drop.py --check passes. Tier B, make check (289), gcmp (identical to U2b), abi_bench both modes and buildtime passed. ABI 1.97. |
| U2c review | Refusals write no outputs, dead paths, section_index | done | pxui 333bf96f, LocalWave b027a96. 11 exports wrote outputs before a refusal, now only on rc 0 (safety test); dead old-caller paths deleted; ui_section_index_result renamed ui_section_index; docs naming deleted exports fixed. Ui::set_colors deletion deferred: it puts static abi_bench Overlays over 1.011. abi_bench both modes, Tier A, make check passed. ABI 1.98. |
| U3.1 | Looks are consts | done | LocalWave 13fbb81, 2848638, 1d63ef8, 6fe6397. container takes &'static Look; detail::line, compact::line and settings::notice deleted for ui.label with theme Labels; paragraph and the positioned cell labels take a coloured Label (theme::text_look); find washes, clocks, song row looks, scrims and run-time Button bases are consts; canvas colours are Rgba::token (looks.py). Fingerprints identical light and dark except one listed test-fixture colour. |
| U3.2 | Literal codemod | done | LocalWave 5a76301. literals.py: 108 Node/Edges/Look/State/States/Font/Rgba/Event literals in lw_ui become builder chains, 6 by hand; R1 51 to 0. Only test Events with payloads stay literals. |
| U3.3 | Keys | done | LocalWave 24a4a68. Artist headings, folder cells, Find chips and the folder Back key are ids; ArtworkReady::key is artwork_id. R4 has no line. Allowlist 308 to 56; Tier B (make check 289) passed. |
| U4a.1 | sidebar.rs | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave e95dbe0, fd3c065 and 050d029, pxui b94935b2. Sidebar on selection_list + localwave-side-row, badge, progress and ROW buttons. |
| U4a.2 | header.rs, find.rs | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 00d4294, fbeb2e1 and c78b934, pxui a2feca5c, c00f6665 and 29525027. Find chips (checkbox CHIP), tool keys (button TOOL), Find fields on search_field + localwave-search. |
| U4a.3 | albums.rs, artists.rs | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 759c71a, f368ae4 and 95feb63. Album sort on segmented SORT, Shuffle all on button KEY, the artists hover wash on the row's look. |
| U4a.4 | transport.rs, now.rs | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 09ce57b, a383d19, 7c48c86 and 6b2c5e7. Transport mode keys and the favourite on the kit's icon button (selected), the up-next drop on a plain node. |
| U4a.5 | queue_sheet.rs | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave ad33351 and 9d4e93a, pxui a7e1f4be. Queue sheet: bottom modal, Button rows, reorderable section_list; queue targeted run green. |
| U4a.6 | songs/mod.rs, songs/row.rs | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 76e50e5, 4f1ea64, 6a18458, 5e37755, c0ee6c3, 893838a and 83258d1, pxui 96e14180 through c7f981c9. Songs list on selection_list + table_head; the fresh-row gap is CP1 action (e). |
| U4a.7 | settings.rs | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 5f9bb98. Settings links (ROW), chips (segmented CHIPS), rescan (SETTING). |
| U4a.8 | onboarding.rs | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave fc3928e. Onboarding links (LINK), the primary key, the Drive disclosure (localwave-disclosure). |
| U4a.9 | commands.rs, metadata.rs, lists.rs | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave ff8c835, f09b7c4, 87272df and 24d7720. N10 top modal; N9 dialogs; #82 the palette on the picker with localwave-palette bodies (CommandSelect, result_row and the command theme deleted). |
| U4a.10 | byline.rs (D7) | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave 1e8f1ff. byline.rs links stay pressables, allowlisted with the D7 reason. |
| U4a review | Review fix round (findings 1-14) and re-review (R1-R6) | done (landed: LocalWave fac487c, pxui 517d61db) | LocalWave aa3a485 through 13535f2, pxui e4ae5d28 through 517d61db (ABI 1.113-1.123): gap steps U2b.4.18-21 and .23-.28; findings 6, 8, 9a, 10, 11, 14 and the abi_bench cuts (pxui 7f68d240, ee031ac2) as review commits; R1-R6 (pxui 61697c35, 735e022a, 517d61db; LocalWave d532195, 170d633, 61fe673, f22ec66, 460ebc4, 13535f2). Package end: Tier B both repos and gcmp green; abi_bench static passes, dynamic instruction gates pass; dynamic wall flaps (Overlays x1.0647, Controls x1.0749, one run each), edge dynamic margins and buildtime c (+1.17 s, 5.05%) deferred to CP1 action (f) (owner ruling 2026-11-20, no limit changed). Lost until U4b: compact hardware Return/⌘A/Esc. Account: scratch/u4a-final-report.md. |
| U4b.1 | compact.rs | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 5257cbf, 14c7f49, 7a8308f, 838f365. Gear icon_button, expanded player Full modal, mode keys KEY (N1), library modes and tab bar kit tabs (#19, #21, N8). |
| U4b.2 | compact/list.rs | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 40f15c5, d647aba, 9e4f0cb, 5fe6532, 1a4fb56, daaf74d. Word/more/swipe buttons, KEY/MARK/deck, the A-Z rail on section_index, artist link, the phone songs list on selection_list (None/Toggle, #29 reorder), Return/Cmd-A/Esc restored. |
| U4b.3 | detail.rs | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 1986dea. Folder link LINK (ink rule), desktop back KEY. |
| U4b.4 | files.rs | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave d2888bc, 10688ca, 2f324f6, 0d74d56, 0dd4277. files.rs keys/links/meters/filters (#67, #69-#79), source card composite (#71), Add music dialog (#81), desktop folder table on ui.table (#73; review round 7bb0f61: None + open_on_click + bare, opens on press or Return), phone folder list on selection_list (ruling 3, #80; its counts prepared per key, 7bb0f61). |
| U4b.5 | Lock-in: api_conformance allowlist at its final size | done (landed: LocalWave 490e8ec, pxui 07cc8cac) | LocalWave 0be2cb9. Allowlist 10 lines (9 composite + D7 byline); every line must carry a composite/D7/dynamic colour reason. Package end: scratch/u4b-final-report.md. |
| U4b.6 | CP1: Tier C and iOS probes | done | LocalWave 3d0cb03 + pxui 8af33e34. Gate misses are machine speed (A/B vs U0.8); gallery warm allocation fixed; manual compact flows open. |
| U4b.7 | CP1a: abi_bench actions (a), (c), (f), resize (e) | done | LocalWave bcaf517 + pxui 6bc2b6ac (ABI 1.132). abi_bench_start own target; Overlays page one button call site; `Overlay.look` by reference and `PxuiPanel.look` by pointer (static Overlays 1.0100, dynamic Controls 1.0289, Overlays 1.0278; Lists 1.0300 unchanged); frame-entry trim found no repeated work; resize +11-14% build instructions traced to sidebar views rebuilt every frame (evidence in CHECK.md). make performance: zero warm allocations on all 28 gated workloads, build CPU misses as at CP1. |
| U5.1 | Events and surfaces (OS-1, 32–34) | done | LocalWave 1e51903 + pxui 6547fc6f (ABI 1.133). Sized 64-byte `PxuiEvent` with a surface, `pxui_ui_add_surface`/`remove_surface` (one surface per Ui, shortcut), borrowed `FrameIn::events`, `FILE_DROPPED` reserved; C-level rejection tests. Events are a strided span from 1.139 (U5.4). |
| U5.2 | Frames and handles | done | LocalWave f091089 + pxui 9faf7b53 (ABI 1.134). One frame call (`pxui_ui_frame` and `FrameIn::snapshot` deleted), `PxuiBox.ui` Ui serial checked by every box-taking call (`core_box(engine, raw)`); one stale status for every box-taking export (U5.6, 1.144, 48-call test). abi_bench figures: the pxui 98e43f99 commit message. |
| U5.3 | Drop legacy exports | done | pxui 48664a74, 7871b73f, 1.155 text headroom; LocalWave f0424b5, 72fc4e0, d043319: ABI 1.147-1.148, 1.155. Prefix renames (`within`, `part`, `part_box`, `slot`, `scope` -> `pxui_ui_*`, `label_node` -> `label`), `animate_keys`/`shaper_measure`/`ui_text` merged, `pxui_ui_variant` deleted (variant is a record field). Kept as decisions: `hovered`, `*_plain` fast paths, the seven C-only calls (`abi_drop.py`). Review fix pxui 5f091ccb / LocalWave 9b57a34 validates ownership before installing nonzero variants. |
| U5.4 | Sizes (OS-36) | done | pxui c4046ff8, e305273c, 9453102b, 34487458, 29314498, 9ae090ad, 0b296c0f, 9536a197, 98e43f99; LocalWave 7b46f2f, 6dc71a5, a10e33a, 4cb3891, 75a89d7, c9b4fd8, 8bfdfb0, b679fb3, b43abbe: 1.135 old-size reads deleted; 1.136 Touch/Environment/Keyboard/FrameIn; 1.138/1.145 `within` refuses a dead or forged handle (record check, test); 1.139 strided `{ptr,len,stride}` spans for events, a11y nodes/actions and layers, sizes on A11yNode/A11yAction/LayerInfo, stride-too-small tests; 1.140 sizes on LookDesc/StateDesc/CurveDesc/AnchorDesc/TextInputOptions/TextMetrics/Bodies (24 reserved slots); 1.141 `TextSpans` a borrowed slice of up to 256 in one per-Ui arena (ruling 9) with a warm test for fresh rows with spans; the DX7.4 minimum-size rule (`read_desc!` with earlier sizes) is kept and tested (`sized_records::ui_desc`, size 12). Linux-only AT-SPI edits not compiled here. abi_bench: two small misses (CHECK.md; the pxui 98e43f99 commit message). Final static review (pxui 4df547cb) reports the earlier 80-byte whole-record cap failing against the 132-byte ABI. All six widget-body pages pass their runtime instruction and allocation checks. Owner decision on the record-size benchmark is pending. |
| U5.5 | Record changes (DX7.7) | done | pxui 675c7b32 + LocalWave b02ef91 (ABI 1.142): `PxuiThemeSnapshot` holds its parts last with `size`/`part_count`, `PxuiA11yTree` sized (`nodes.stride` is the node stride), `TextSpans` slice (1.141), enum range checks pinned by `enum_values_are_range_checked_on_input` (the audit found every caller-written enum already checked). |
| U5.6 | Encodings (OS-41) | done | pxui e8c6485c, ca1c7c5a + LocalWave de7888e, 372551d (ABI 1.143, 1.144): D19 token alpha final (the 0x4b rule deleted), anchor align 3/4 refused (`legacy_anchor_align` deleted), LIGHT=TOP/INNER=AREA decided and documented, explicit `dark` token (theme files say `tokens.dark = true`; LocalWave dark.theme updated), four documented and tested conventions (SIZE_MAX none, text key 0, SET mask vs 0, repaint_at infinity: `safety::encoding_rules`), one stale-handle status. |
| U5.7 | C vocabulary and menu rows | done | pxui dd8751e9 + LocalWave 60d98cc (ABI 1.149): DX1.12G names and `_Static_assert`s, `PxuiMenuRow`/`PxuiMenuOptions`. Review fix pxui b39573f4 / LocalWave 70c5816 restores the aggregate menu text budget, including shortcuts and submenus. |
| U5.8 | Features | done | pxui 6575d3f3 + LocalWave 526dd66 (ABI 1.150): four ids. |
| U5.9 | OS remainder | done | pxui 1f5dfb4b, e3be1411, 3e23611d + LocalWave bd2c918, 99b9c09, 6c09963 (ABI 1.151-1.153): `pxui_ui_diagnostics` (ext) and `pxui_counter` in the ext set, one create call with fonts and status, layer roles, a11y surfaces, renderer-held Ui, header contracts, OS-6 decision. Review fixes pxui 3a673d0f / LocalWave 16c8930 guard diagnostics; pxui 4b0f1e83 carries accessibility destinations, removes closed surfaces, clears pending requests at teardown, and routes native host actions. Owner approved the unreleased baseline/ledger correction; static, ABI routing and native surface-7 checks pass. Linux ABI (162 tests) and AT-SPI (11 tests) pass, including two-surface action routing and teardown on a private D-Bus server (pxui 71ebaf0d, e575d22f). Queue-bound review fix pxui f0b3aee0 / LocalWave d27b612 refuses whole batches past 4096 pending requests, with frame reset, teardown and zero-warm-allocation regression. Wrapped tooltips (15488a98) and the inspector bar with large hint lists (43b83eaa) allocate nothing once warm in both link modes. Inspector action slices, formatted button and rename keys, clipped chips and action descriptors reuse storage (6fe9dd4b, ea318093). Editor tooltip keys reuse storage and still restart hover on a new token (d0e29b9c). The owner approved a UI-borrowed inspector title and a reusable output buffer for editor descriptions (pxui e6e52052). Focused tests pass in both link modes for accepted renames, hover timing, and warm allocation reuse, including nested descriptions on separate Ui instances. A C test verifies that later header calls preserve earlier title views within the frame. Private scratch placement (pxui d7f770ac) restored the hot field offsets and brought dynamic Controls and Layout back under their instruction limits. |
| U5.10 | Extension set | done | pxui d650edc9 + LocalWave f18d263 (ABI 1.154): ext manifest holds the Rays set and diagnostics; extension body kinds documented. |
| U5.11 | Re-baseline: major 2 | done | pxui 45658786 + LocalWave pointer (ABI 2.0): `libpxui.2`, 2.0 manifests and `fixtures/abi/2.0`, `abi_built` 0 refused, 1.x app refused (test), one history row in docs/abi.md, header cleaned, `PxuiUi.theme` frozen (owner 21 Nov 2026); `abi-frozen` and `abi-compat` green. |
| U5.12 | CP2: Tier C | todo |  |
| U6.1 | Docs gate | todo |  |
| U6.2 | Literal sweep (Batch H) | todo |  |
| U6.3 | Docs (Batch J) | todo |  |
| U6.4 | deny(missing_docs) | todo |  |
| U6.5 | Records lint (DX7.9) | todo |  |
| U6.6 | Ledger check | todo |  |
| U6.7 | Freeze (DX7.12) | todo |  |
| U6.8 | Close-out | todo |  |

## Latest performance

The CP1 (U4b.6) signed counting release run (LocalWave 911d69e, pxui 07cc8cac) on an Apple M1 Mac mini failed the gate: 16 of 28 gated workloads missed build CPU p99 < 1,500 µs, and 9 of those also frame CPU p99 < 2,500 µs. All gated workloads had zero warm Rust allocations (the measured-first-navigation and measured-first-menu-open rows allocate by design, 1,195 and 125). Start load 3.58 (5-minute average), peak under 4. A second run (start load 3.16) missed 15 workloads on build CPU (queue-carry-scroll-cancel and artist-detail-scoped-scroll passed, idle-600-return missed) with the same magnitudes (`scratch/cp1/perf1.log`, `perf2.log`). Work wall p99 stayed under 3,100 µs everywhere. The last U0.8 run (load 2.3–2.8) passed all 30 workloads (build CPU p99 max 1,477 µs); p50 values are now about 20–50% higher on every workload, including idle-like ones, so the machine's current speed (see the 19 Nov U4a.6 note in CHECK.md) is not separated from a code change; no bisect was run.

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
| songs-original | 1068/1467/1478 | 1079/1482/1675/- | 1769/2211/2240 | 1614/2077/2395/2914 | 0 | pass |
| find-filters-title-descending | 1042/1308/1317 | 1072/1295/1378/- | 1664/1954/1958 | 1537/1848/2020/2037 | 0 | pass |
| queue-occurrences | 1309/1730/1735 | 1313/1700/1856/- | 1858/2491/2552 | 1778/2277/2408/7673 | 0 | miss: build CPU |
| queue-carry-scroll-cancel | 1033/1575/1586 | 1046/1413/1568/- | 1636/2373/2424 | 1488/2097/2164/2234 | 0 | miss: build CPU |
| full-queue-playlist-editor | 837/950/953 | 836/965/987/- | 1421/1569/1589 | 1261/1397/1491/1535 | 0 | pass |
| albums-artwork | 1051/1648/1657 | 1064/1370/1664/- | 2056/2721/2808 | 1872/2368/2545/2608 | 0 | miss: build CPU, frame CPU |
| waveform-scrub-and-scroll | 773/1431/1480 | 793/1389/1432/- | 1468/2187/2218 | 1296/1975/2022/2035 | 0 | pass |
| files-indexed-directory | 1392/1773/1820 | 1395/1776/1843/- | 2131/2575/2584 | 1959/2385/2477/2812 | 0 | miss: build CPU, frame CPU |
| external-drop-hover-and-cancel | 536/713/716 | 534/704/724/- | 1109/1380/1390 | 955/1150/1223/1240 | 0 | pass |
| artists-measured-columns | 1139/1448/1460 | 1139/1407/1507/- | 1770/2111/2126 | 1618/1915/2002/2076 | 0 | pass |
| songs-playing-indicator | 1247/1701/1723 | 1248/1574/1719/- | 1888/2298/2322 | 1716/2095/2159/2225 | 0 | miss: build CPU |
| songs-long-scroll | 1444/1834/1847 | 1445/1792/1943/- | 2109/2499/2513 | 1942/2290/2449/2777 | 0 | miss: build CPU |
| album-detail-scoped-scroll | 817/938/965 | 815/940/983/- | 1492/1803/1880 | 1332/1499/1588/1644 | 0 | pass |
| artist-detail-scoped-scroll | 934/1535/1641 | 945/1340/1637/- | 1692/2392/2416 | 1511/2002/2231/2377 | 0 | miss: build CPU |
| list-dialogs-menu-and-name | 834/2098/2100 | 847/1991/2096/- | 1437/2730/2747 | 1291/2484/2598/2640 | 0 | miss: build CPU, frame CPU |
| metadata-editor-typing | 801/2026/2029 | 797/1901/2102/- | 1382/2710/2765 | 1213/2357/2602/2726 | 0 | miss: build CPU, frame CPU |
| song-drag-carry-hover-cancel | 1435/1788/1796 | 1455/1768/1806/- | 2047/2474/2491 | 1927/2267/2353/2414 | 0 | miss: build CPU |
| fling | 852/1339/1361 | 870/1288/1337/- | 1493/2026/2030 | 1319/1753/1835/1838 | 0 | pass |
| reverse-scroll | 1231/1603/1616 | 1262/1605/1640/- | 1910/2313/2314 | 1772/2128/2273/2582 | 0 | miss: build CPU |
| scroll-jump | 686/2314/2393 | 684/2023/2474/- | 1321/2968/2991 | 1139/2496/2929/2939 | 0 | miss: build CPU, frame CPU |
| non-song-lists | 780/2225/2283 | 781/1416/2279/- | 1419/2876/2908 | 1238/1980/2724/2759 | 0 | miss: build CPU, frame CPU |
| search-typing | 979/1178/1197 | 977/1098/1207/- | 1597/1829/1894 | 1422/1581/1673/2094 | 0 | pass |
| measured-first-navigation | 761/5222/5568 | 771/1632/5520/- | 1386/5883/6300 | 1227/2207/6032/6223 | 1195 | measured |
| text-typing | 1782/2035/2044 | 1780/2024/2067/- | 2474/2846/2911 | 2299/2611/2725/2815 | 0 | miss: build CPU, frame CPU |
| measured-first-menu-open | 633/1871/3101 | 638/844/1867/- | 1246/2516/3860 | 1083/1334/2338/3733 | 125 | measured |
| resize | 1764/2543/2554 | 1783/2477/2605/- | 2328/3301/3394 | 2139/3010/3268/3484 | 0 | miss: build CPU, frame CPU |
| theme-switch | 1615/2626/2708 | 1648/2330/2635/- | 2262/3291/3379 | 2139/2840/3141/3299 | 0 | miss: build CPU, frame CPU |
| idle-600-return | 1076/1488/1516 | 1078/1422/1519/- | 1721/2160/2238 | 1547/1948/2022/2023 | 0 | pass |
| clock-120 | 635/768/777 | 633/758/784/- | 1286/1585/1641 | 1097/1330/1367/1394 | 0 | pass |

## Build time

`tools/dx/buildtime.sh --compare 16dc635` reports the minimum of three alternating rounds in seconds. All
six U1b measurements stayed within max(5%, 0.3 s) of CP0, measured on LocalWave 1a999a1 and pxui 0baa83c6
after a stuck fseventsd was restarted. An earlier run under external load (load average 16–29) read clean
pxui_dylib +4.9 s and was discarded. The de53440 values come from the earlier pre-DX comparison.

| Measure | Budget | U1b | CP0 | de53440 |
| --- | --- | --- | --- | --- |
| a clean pxui workspace debug build | 30 s | 26.24 | 26.06 | 39.12 |
| b warm `cargo check -p pxui_core` after a touch | 1 s | 0.28 | 0.28 | 0.33 |
| c clean `cargo build -p pxui_dylib` | – | 23.00 | 22.75 | 33.86 |
| d clean `cargo build -p lw_ui`, dependencies prebuilt | – | 3.07 | 3.04 | 3.36 |
| e warm `cargo check -p lw_ui` after touching shell/sidebar.rs | – | 0.22 | 0.22 | 0.25 |
| f warm `cargo build -p lw_bin` after the same touch | – | 0.98 | 0.98 | 1.29 |

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
