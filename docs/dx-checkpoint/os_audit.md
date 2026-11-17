# pxui OS-grade audit (origin/rust-port 6f2df11, read-only)

Scope: crates/public (pxui, pxui_types), crates/private (pxui_core, pxui_abi, pxui_text, pxui_render, pxui_atspi, pxui_dylib),
crates/host/pxui_host, the hosts/leaf crates pxui owns (input, mem_sys, pxui_sdl, sdl3/sdl3_sys, pxui_wayland), include/pxui.h,
abi/*.txt, docs/abi.md. Criteria = os_criteria.md 1–8. Paths are relative to the pxui repo root. Severity: **blocker** (for 2.0),
**should fix**, **note**. Test-only thread_locals (pxui/src/bodies.rs:100, author/*.rs, pxui_abi/src/bodies.rs:314+, settled.rs,
surface.rs, test_layout_rows.rs) were checked and are `#[cfg(test)]`: not findings.

## Findings

### Multi-surface (criterion 1)

1. `crates/public/pxui_types/src/lib.rs:448-449`, `docs/abi.md:112`, `crates/private/pxui_abi/src/ui.rs:371-406` — C1, C5 — **blocker**.
   `FrameIn.surface` is reserved and ignored; `Event` carries no surface ("events belong to the frame's"). One `Ui` = one implicit
   surface; nothing names or checks it. Fix: in 2.0 a `Ui` is created for (or registers) surface ids; `FrameIn.surface` and every
   event carry the id; the library refuses an unknown surface with `PXUI_ERR_ARGUMENT`; focus, a11y tree, IME and keyboard geometry
   are keyed per surface (they are per-`Ui` today, which is correct for one-surface-per-Ui; say so in the header).
2. `crates/open/pxui_sdl/src/lib.rs:266-296`, `crates/open/input/src/lib.rs:74` — C1 — **blocker**. `Window::pump` drains the
   process-global SDL event queue and attributes every event (including `Closed`, `Resized`, keys) to this window; `input::Event`
   has no window id (`sdl3_sys/src/lib.rs:363,592` already knows ids). Two windows split each other's events. Fix: one process pump
   (`pxui_sdl::app`) that routes by SDL window id into per-`Window` queues; events carry the surface id (finding 1).
3. `crates/private/pxui_atspi/src/bridge.rs:97-160`, `crates/private/pxui_abi/src/a11y.rs:23-83` — C1 — should fix. The AT-SPI bridge
   registers one application per bus connection serving one tree; a multi-window app must open N bridges that appear as N apps.
   Fix: one bridge per process (app root) with `add_surface`/`update(surface, tree)`/`remove_surface`, window objects as the root's
   children; `pxui_a11y_update` takes the surface id. Also `update_ffi` allocates a fresh `A11yTree` + `HashSet` per changed revision
   (`bridge.rs:168-173, 791-813`): reuse buffers (C8).
4. `crates/open/pxui_wayland/src/lib.rs:78-90` — C1 — should fix. Clipboard free functions use "the newest open window" and a
   process-local string when none is open. Fix: clipboard per window/seat (the focused surface's), passed explicitly.
5. `crates/private/pxui_core/src/ui_core.rs:6458-6460`, `ui_widgets.rs:1008` — C1 — should fix. Menus, popups, tooltips, dialogs and
   toasts are layers of the one surface, placed and clipped in its window points; a host cannot put a popup in its own surface (an
   OS menu must overflow its window). Fix: tag `Output` layers with a role (popup/tooltip/modal) and anchor so a host may present them
   in child surfaces; input for them arrives with that surface id.
6. `crates/host/pxui_host/src/graphics.rs:25-35` — C1, C8 — note. `Graphics::open` creates a device and queue per window; a
   multi-window app should share one device across surfaces.

### Process-global state (criterion 2)

7. `crates/open/mem_sys/src/lib.rs:47` (+ `mem_sys` in pxui_core/pxui_abi/pxui_text `Cargo.toml`) — C2, C8 — **blocker**. The library
   links a `#[global_allocator]` (counting, atomics on every allocation, `REPORTS: Mutex<Vec<String>>` without a cap at `:85,133-138`)
   into libpxui and into every statically linked app: a UI library dictates the process allocator. Fix: the counting allocator only
   behind a test/bench feature (the gates' binaries install it); the library exposes its own counters through a hook, never a
   `global_allocator`.
8. `crates/private/pxui_core/src/ui_core.rs:622, 1379, 2021`; `crates/private/pxui_abi/src/ui_state.rs:144` — C2 — **blocker**.
   `VERIFY_NEW_VIEWS` is a process-global flag that a frozen C export (`pxui_verify_new_views`) flips for every later `Ui`. Fix: drop
   the export; harnesses call `pxui_ui_verify_views(ui, 1)` per `Ui`.
9. `crates/private/pxui_core/src/bodies.rs:491-510`, `include/pxui.h:543` — C2, C4 — should fix. Body names are a process-global
   `Mutex<Vec<HashMap<String, BodyId>>>`, never released, 65,535 per kind per process, names up to 256 MiB. It is documented, but in a
   compositor hosting many apps it is a shared namespace that is exhausted by uptime. Fix: per-`Ui` name→id table; a `Theme` stores
   names and resolves them per `Ui` when applied; cap a name at 64 bytes.
10. `crates/private/pxui_abi/src/state.rs:10-14`, `ui.rs:27-32` — C2, C5, C7 — should fix. `CALLS` is process-global with a
    non-atomic load+store "for the one thread the contract allows" (a process-wide one-thread assumption). `pxui_counter(which)`
    mixes the call count with `mem_sys` allocator counters, and `which = COUNT_LOCAL_RESET` resets state through a getter. Fix:
    per-`Ui` counter (or `fetch_add`); move `pxui_counter` to a diagnostics extension; a reset is its own call.
11. `crates/private/pxui_abi/src/image.rs:22-51` — C2, C4, C8 — should fix. Native-image identity interning (`HANDLES`) is a
    thread_local shared by every `Ui`/renderer on the thread: one surface's churn evicts another's identities. Eviction scans all
    entries (`:38`, `min_by_key`), while `docs/abi.md:156` says "O(1) LRU". Fix: per-renderer table with an O(1) LRU (index-linked
    list), or correct the doc.
12. `crates/public/pxui/src/lib.rs:323-329` — C2, C4 — should fix. `ORPHANS`: a `Ui` dropped before its renderer waits in a
    thread_local and leaks forever if the renderer is leaked or dropped on another thread. Fix: the library refcounts (a renderer
    holds the `Ui`; `pxui_ui_destroy` with live renderers defers destruction to the last renderer), and the wrapper list goes.
13. `crates/public/pxui/src/paint.rs:316-323`, `overlays.rs:76,82`, `composite.rs:11-13` — C2, C4 — note. Closure pool (64 per
    layout, never freed), picker/menu column scratch (grows to peak) and deferred-panic slot are per-thread and shared by all `Ui`s on
    the thread. Bounded by code/peak and commented; document as per-thread by design in one place.
14. `crates/private/pxui_core/src/ui_kit.rs:653-666` — C2 — note. `control_hit` keeps one `Rc` closure per shape function per thread
    for the thread's life. Fix: make hit shapes plain `fn` pointers (no `Rc`), removing the cache.
15. `crates/private/pxui_abi/src/bodies.rs:51`, `crates/private/pxui_core/src/ui_core.rs:1229` — C2 — note. Process-global
    monotonic id counters (slot serials, `Ui` serials). Benign (u64, no wrap); document as per-process by design.
16. `crates/private/pxui_abi/src/renderer.rs:83-88`, `crates/host/pxui_host/src/graphics.rs:26` — C2 — note.
    `ogpu_core::impl_::select` is a process-global backend choice, first selection wins. Document it as per-process by design.

### Untrusted callers (criterion 3)

17. `crates/private/pxui_abi/src/trusted.rs:39-45, 88-100, 656-660` → `crates/private/pxui_core/src/ui_core.rs:3165-3168`;
    `crates/private/pxui_abi/src/safety.rs:17-34` — C3 — **blocker**. Builder exports called outside a frame (`pxui_ui_node`,
    `_node_keyed`, `pxui_button`, `pxui_toggle`, `grid_list`, …) reach `require_building` and panic. The macro catches the panic,
    poisons the `Ui`, and the next `pxui_frame` returns `PXUI_ERR_PANIC` and skips that frame. The test codifies this as intended. A
    misuse becomes a denial of service. Fix: `enter!` for builder entries checks `in_frame` and returns `PXUI_ERR_STATE`; a gate
    calls every export outside and inside a frame with zero/garbage arguments and asserts no `PXUI_ERR_PANIC`; `catch_unwind` stays
    as a backstop for library bugs only.
18. `crates/private/pxui_abi/src/ui.rs:240-265` (`translate_event`, `push_events`) vs `kit.rs:169` — C3 — **blocker**. In the frame
    path, event x/y/value are not checked finite (POINTER_*, SCROLLED, PINCHED, TRACKPAD, TEXT_EDITING start/length via `f64 as i64`,
    RESIZED), although `pxui_ui_wants_frame` checks them. Event text uses `text()` (no `MAX_TEXT_BYTES`) and every text event is
    copied (`to_owned`), so up to 4096 events × unbounded bytes are allocated per frame from caller input. Fix: one per-kind
    validator shared by both paths; finite coordinates; a per-frame total text budget (`MAX_TOTAL_TEXT_BYTES`).
19. `crates/private/pxui_abi/src/renderer.rs:74-77, 116` — C3, C5 — **blocker**. `pxui_renderer_create` reads `*target` and `&*gpu`
    whole before checking their `size`, over-reading a shorter caller record. This breaks the growth rule for any later record
    version. Fix: `read_desc!` both before use.
20. `crates/private/pxui_abi/src/text.rs:261-264, 283-287`, `crates/private/pxui_text/src/lib.rs:876-887`, `ui.rs:49-58` — C3, C4 —
    **blocker**. `pxui_text_add_font`/`pxui_shaper_add_font` have no size or count cap, accept empty or unparseable bytes (they still
    push a name), copy the bytes, and fonts are never removed, so the font system grows with every call. `ui_create_with_fonts` allows
    128 × 16 MiB = 2 GiB copied. Fix: per-font cap (32 MiB), at most 64 fonts per text service (`Style.font` is u16), refuse
    unparseable fonts with an error, and a total-bytes budget.
21. `crates/private/pxui_abi/src/ui.rs:372-374, 436-451` — C3 — should fix. `FrameIn.width/height` are only checked finite (negative
    or 1e30 pass, then `as i64`). `Input.logical_size` is not checked at all. Held key codes become `Key::Unknown(code)` unvalidated.
    Fix: 0 ≤ size ≤ a documented maximum (e.g. 1<<16 points); refuse unknown key codes as the event path does.
22. `crates/private/pxui_abi/src/renderer.rs:203-208`, `crates/private/pxui_render/src/lib.rs:490-497` — C3, C4 — should fix.
    `pxui_renderer_resize` takes any u32 extent and allocates two blur targets of half that size. Fix: cap at the device's maximum
    texture dimension and refuse larger extents.
23. `crates/private/pxui_abi/src/text.rs:13-21` — C3 — should fix. The text-service exports (`text_width*`, `shaper_*`, `text_quads`,
    `text_rasterize_rgba`) use `bytes`/`utf8` without `MAX_TEXT_BYTES`, while UI exports use `checked`. Fix: one checker everywhere.
24. `crates/private/pxui_abi/src/state.rs:17` — C3, C4 — should fix. One 256 MiB cap for every string: keys, body names, a11y
    names, theme keys and labels may each copy 256 MiB per call. Fix: per-role caps (identifiers/keys/names ≤ 256 B, labels ≤ 64 KiB,
    editor text a documented larger cap).
25. `crates/private/pxui_abi/src/macros.rs:46-51` — C3 — should fix. A panic in a `bare` export (shaper, theme, a11y bridge, prepared
    text) is swallowed with no poison, so the object may be left half-mutated. Fix: a poison flag per bare object, as `Shared` has.
26. Core `assert!`/`panic!` invariants reachable from C (`crates/private/pxui_core/src/ui_core.rs:3198, 3383, 3873, 3898, 6678-6680,
    6938, 6977, 7039, 7087, 7156`) — C3 — should fix. Spot checks show the ABI pre-validates them (`trusted.rs:602, 658`,
    `box_ops.rs:356`, `node.rs:191`). The only proof is the hand-picked `safety.rs`. Fix: the export-fuzz gate of finding 17 covers
    every export.
27. `crates/private/pxui_abi/src/state.rs:111-118, 191-206`, `ui.rs:78-88` — C3 — note. Handles are raw pointers, so a foreign or
    dangling pointer is UB (`pxui_ui_destroy` reads `owner` through it). This is inherent to an in-process C ABI. Decide explicitly for
    2.0: either keep pointers (document) or use generation-checked handles validated against a per-process table.
28. `crates/private/pxui_abi/src/theme.rs:89-96` — C3, C6 — should fix. `pxui_theme_load` does filesystem I/O by caller path inside
    the library, with an unbounded `read_to_string`. A sandboxed compositor cannot allow this. Fix (2.0 cut): drop it; the app reads
    the file and passes the bytes to `pxui_ui_theme_apply` (size-capped).

### Bounded resources (criterion 4)

29. `crates/private/pxui_abi/src/state.rs:123-137` (`lend`/`give`); callers `text_inputs.rs:217-223` (`pxui_ui_load_text_edit`),
    `box_ops.rs:331, 370, 405`, `overlays.rs:198`; cleared only at frame start `ui.rs:337-338` — C4 — should fix. Calls outside a
    frame push a new owned copy per call until the next frame: unbounded between frames. Fix: one return slot per export (overwritten
    by its next call, documented as "valid until the next call of this function or the next frame"), or a cap returning
    `PXUI_ERR_STATE`.
30. `crates/private/pxui_abi/src/node.rs:151, 226-233` — C4, C8 — should fix. Each grid node is deduplicated by a linear scan over all
    stored track lists, and every frame runs `retain` + `box_of_key` over all of them: O(grids²) per frame. Fix: hash the track list,
    and retire tracks via the engine's gone-slot list.
31. `crates/private/pxui_text/src/lib.rs:275-283` (`MAX_LAYOUTS` 4096, `KEEP_FRAMES` 600), `specification/pxui.md:712-714` — C4 —
    should fix. The layout cache is capped by count, not bytes: 4096 entries × up to 256 MiB text each. Fix: a byte budget on retained
    text (or a per-entry cap, with larger texts laid out uncached).

### Long-lived ABI (criterion 5)

32. `crates/public/pxui_types/src/lib.rs:348-384`, `include/pxui.h:24-27`, `crates/private/pxui_abi/src/ui.rs:240-245`,
    `crates/open/input/src/wire.rs:134-178` — C5 — **blocker**. `Event` is an unsized 48-byte array element with in-band payloads:
    TOUCH is a `PxuiTouch` smuggled through `text` with `len == sizeof(PxuiTouch)`; FILES_DROPPED/FILE_DIALOG are NUL-joined paths
    with `code` 0/1 meaning Ok/Err-text; TEXT_EDITING start/length ride in f64 `x`/`y`; FILE_DIALOG's id is an f64 `value` (loses
    ids > 2^53); DIALOG_OPENED's id is a u32 `code`; RESIZED/PIXEL_SIZE_CHANGED are f64→i64. Fix: the 2.0 Event (see the cut list).
33. `crates/public/pxui_types/src/lib.rs:368, 383`; `wire.rs:148-156` — C5 — **blocker**. Twin event kinds: FILE_DROPPED (11, one
    path) vs FILES_DROPPED (26, set). Drop 11 in 2.0; renumber nothing, keep the value reserved.
34. `crates/private/pxui_abi/src/ui.rs:411-475`, `include/pxui.h:407-410` — C5, C3 — **blocker**. Two frame entry points: `pxui_frame`
    (`FrameIn`, five accepted sizes) and `pxui_ui_frame` (`PxuiInput`, a full host-snapshot record that resets environment and
    keyboard to defaults on every call, `ui.rs:461-462`). Fix: one frame call (cut list); `pxui_ui_input` returns the routed frame in
    the same record.
35. No `pxui_supports` anywhere (`docs/abi.md:16` "the app checks the minor"; `crates/public/pxui/src/lib.rs:383`) — C5 —
    **blocker**. Fix: `uint8_t pxui_supports(uint32_t feature)` with an append-only `PxuiFeature` enum (one value per optional
    capability: touch, gestures, keyboard geometry, blur, bodies, surfaces, a11y bridge …); the wrapper gates on features, not minors.
36. Records without `size` that must be able to grow (`include/pxui.h`, from a header scan): `PxuiEvent`, `PxuiTouch`, `PxuiA11yNode`,
    `PxuiA11yAction`, `PxuiA11yTree`, `PxuiTextInputOptions`, `PxuiAnchorDesc`, `PxuiLookDesc`/`PxuiLook`, `PxuiTokens`/
    `PxuiThemeSnapshot`/`PxuiStates`, `PxuiBodies` (grows with every body kind, `:119`), `PxuiCurveDesc`, `PxuiTextMetrics`,
    `PxuiLayerInfo`, `PxuiStateDesc`, `PxuiEnvironment`/`PxuiKeyboard` (embedded) — C5 — **blocker**. Fix: a `size` prefix on every
    growable record. Arrays cross as `{ptr, len, stride}` spans, so elements can grow too. Pure values (Rect, Point, Color, Str, Box,
    Range) stay unsized and are declared final.
37. Twins/legacy in the frozen manifest (`abi/pxui.v1.txt`; `docs/abi.md:97-112` mark several legacy) — C5 — **blocker**. Full list
    in "2.0 cut items".
38. `abi/pxui.ext.txt`, `docs/abi.md:32-37` — C5, C6 — should fix. The Rays extension set (inspector_*, picker, line/value field)
    ships in libpxui and `include/pxui.h` with "no stability guarantee": an OS ABI should not ship unfrozen calls in its frozen
    library. Fix: move them to a separate library/crate (`pxui_rays`) built on the public ABI, or drop them from 2.0.
39. Diagnostics in the frozen manifest: `pxui_counter`, `pxui_verify_new_views`, `pxui_ui_verify_views`, `pxui_ui_verifying`,
    `pxui_ui_set_timing`, `pxui_ui_phase_times`, `pxui_ui_phase_cpu_times`, `pxui_ui_text_layout_count`, `pxui_ui_rebuilt_named` —
    C5 — should fix. Fix: one `pxui_ui_diagnostics(ui, PxuiDiagnostics *out)` sized record behind `pxui_supports`; test-only switches
    move to a diagnostics extension.
40. Error model fragmented: `pxui_ui_create*` return null with no reason (`ui.rs:42-58`); shaper/a11y/prepared calls have no error
    text; value exports use 0, `UINT32_MAX`, `SIZE_MAX` or zeroed results as refusals (`macros.rs:36-39`, `include/pxui.h:349-351`);
    `pxui_paint_*` and `pxui_ui_text_width` return void/float with no error channel — C5 — should fix. Fix (2.0): every fallible
    call returns `int32_t` status with out-params, every object has `last_error`, and create functions take an out-pointer plus a
    status.
41. In-band sentinels and mixed default rules — C5 — should fix: `SIZE_MAX` means "none" for selection/activated
    (`collections.rs:60, 84, 110`); text key 0 means "derive from content" (`text.rs:293`; the key is a pure hint, as `pxui_text` compares
    the text); node keys 0/1 are reserved; a 0 field means "the theme's" in most descs while labels use `PXUI_LABEL_SET_*` bits;
    a `ColorToken` past the table reads transparent (`theme.rs:44`); dark mode is inferred from the panel colour channel sum < 384
    (`docs/abi.md:96`); `repaint_at = INFINITY`. Fix: one documented, tested rule per kind: `has_*`/SET bitmasks for optionals, an
    explicit `is_dark` in the theme, an error for an out-of-table token, and the text key parameter removed in 2.0.
42. `docs/abi.md:107` (`pxui_ui_variant`) — C5 — should fix. A widget's variant is a hidden modal side channel: set before the call,
    cleared after, kept 0 inside bodies. Fix (2.0): `variant` is a field of each widget desc; drop `pxui_ui_variant`.

### Policy-free core (criterion 6)

43. `crates/private/pxui_core/src/theme.rs:4, 55, 213`; `ui_kit.rs:2, 418-419, 678-915`; `ui_text.rs:764-783`; `include/pxui.h:119,
    543` (`PxuiBodies`, `PxuiBodyKind`: INSPECTOR_SECTION=17, INSPECTOR_ROW=18, picker, line/value field) — C6 — **blocker** (the
    frozen `PxuiBodies` record and body-kind enum carry the Rays app's vocabulary). Rays' inspector sheet (label column 0.3 of the
    panel less 16, pin dot, 24-point rows) is core widgetry, and `Theme::default()` is "the Rays look". Fix: move the inspector,
    picker and value/line fields with their body kinds to the Rays extension (finding 38); the 2.0 body-kind enum lists only generic
    widgets; rename the default theme for what it is (the PXUI kit) with no app name.
44. `crates/open/sdl3_sys/src/lib.rs:363, 592, 864` (`rays_sdl3_*` shim symbols) — C6 — note. Rename to `pxui_sdl3_*`.
45. `docs/abi.md:109-111` ("What LocalWave's text and button helpers need", "LocalWave's settings bodies", "LocalWave's buttons"),
    `crates/public/pxui/src/author/toggle.rs:384, 461`, song/album doc examples (`box_ops.rs:504-541`, `text.rs:131-424`) — C6 —
    note. The behaviour is generic; reword the rationale and examples without the app (LocalWave stays the reference app in the
    gates, not in library text).
46. `crates/private/pxui_abi/src/ui.rs:46`, `text.rs:280`, `crates/private/pxui_text/src/lib.rs:294` — C6 — note. Every `Ui` loads
    the embedded Pragmasevka face, and the default theme's font is a monospace face. That is a product choice. Fix (2.0): one create
    call that takes the font list (empty: the platform UI font where available); `pxui_default_font` stays as an opt-in asset.

### Threading and lifetime (criterion 7)

47. `include/pxui.h:647-648` (only the shaper's threading is stated), `AGENTS.md:105` ("Calls stay on one thread"), `state.rs:9` — C7,
    C1 — should fix. The header never says which thread owns a `PxuiUi`, a renderer or a paint handle, and the repo-wide rule is
    one thread per process. Multi-surface on per-surface threads needs per-object affinity. Fix: header contract "a `Ui`, its
    renderers and its paint handles belong to the thread that created the `Ui`; different `Ui`s may live on different threads;
    shapers/prepared text are `Send`". Add a cheap creating-thread check that returns `PXUI_ERR_STATE`, and make the remaining
    globals thread-safe (findings 10, 11).
48. `include/pxui.h:290, 310`; `crates/private/pxui_abi/src/trusted.rs:379` — C7 — should fix. `PxuiIconDraw` + `draw_user` run at
    paint time, after the widget call returned, with no release callback and no stated lifetime (unlike `pxui_ui_draw` and its
    `release`). Fix: document "called during this frame's prepare/paint only; `draw_user` must stay valid until the next
    `pxui_frame` returns", or add a release callback as `pxui_ui_draw` has.
49. `crates/private/pxui_abi/src/kit_ext.rs:127-135`, `crates/private/pxui_core/src/ui_widgets.rs:1317-1319`,
    `crates/private/pxui_abi/src/state.rs:62-87, 248-254` — C7 — note. Caller memory is laundered to `'static` (press curves,
    `text()` → `&'static str`, the `menu`/`hints`/`strs`/`menus` pools). The docs say "borrowed for the call", and the pools are
    emptied after the call, but the types do not enforce it. Fix: real lifetimes, or raw spans converted at use, plus a test that
    nothing survives the call.
50. `include/pxui.h:63, 79` (`PxuiSizeDesc.relative`, `PxuiSize.u.relative`) — C7 — note. A bare function pointer is stored in
    retained nodes and called in later layouts. Document that the code must stay loaded while any node holds it (plugins).

### Performance (criterion 8)

51. `crates/private/pxui_abi/src/kit.rs:166` — C8 — should fix. `pxui_ui_wants_frame` allocates a `Vec` per call (an idle host calls
    it every wake). Fix: reuse a `Shared` buffer, as `push_events` does.

## Counts

| Criterion | Findings (primary) | Blocker | Should fix | Note |
| --- | --- | --- | --- | --- |
| 1 Multi-surface | 1, 2, 3, 4, 5, 6 | 2 | 3 | 1 |
| 2 Global state | 7–16 | 2 | 4 | 4 |
| 3 Untrusted callers | 17–28 | 4 | 7 | 1 |
| 4 Bounded resources | 29, 30, 31 | 0 | 3 | 0 |
| 5 Long-lived ABI | 32–42 | 6 | 5 | 0 |
| 6 Policy-free | 43–46 | 1 | 0 | 3 |
| 7 Threading/lifetime | 47–50 | 0 | 2 | 2 |
| 8 Performance | 51 | 0 | 1 | 0 |
| **Total** | **51** | **15** | **25** | **11** |

Secondary tags (counted once above under their primary): C1 on 47; C2 on 47; C3 on 19, 34; C4 on 9, 11, 12, 13, 20, 22, 24;
C5 on 1, 10, 19, 38; C6 on 28, 38; C7 on 10; C8 on 3, 6, 7, 11, 30.

## Fix batches (disjoint file sets)

B1–B5 can run in parallel; C1 runs after them because it owns every ABI-surface file. Within C1 the sub-steps are sequential.

- **B1 core globals and policy** — `crates/private/pxui_core/src/{bodies.rs, ui_kit.rs, ui_core.rs, theme.rs, ui_text.rs, ui_widgets.rs}`.
  Findings 9 (per-Ui names; the header text follows in C1), 8 (core flag removed; the export is dropped in C1), 14, 15 (doc),
  43 (core side: inspector/picker/value field behind a module the Rays crate takes; default theme naming), 5 (layer roles in
  `Output`; the C record changes in C1), 26 (no change in core; the gate is in B2a).
- **B2a ABI state and validation** — `crates/private/pxui_abi/src/{state.rs, macros.rs, trusted.rs, safety.rs, text_inputs.rs,
  box_ops.rs, node.rs, kit.rs}`. Findings 17, 24, 25, 26 (export-fuzz test in `safety.rs`), 29, 30, 51, 10 (`CALLS` per Ui/atomic).
- **B2b text, image, renderer** — `crates/private/pxui_abi/src/{text.rs, renderer.rs, image.rs}`, `crates/private/pxui_render/src/lib.rs`,
  `crates/private/pxui_text/src/lib.rs`. Findings 19, 20, 22, 23, 11, 31.
- **B3 allocator** — `crates/open/mem_sys/src/lib.rs`, `crates/private/{pxui_core,pxui_abi,pxui_text}/Cargo.toml`,
  `crates/public/pxui/Cargo.toml`, `crates/public/pxui/src/composite.rs`. Finding 7 (and the 13 doc note for `composite.rs`).
- **B4 hosts** — `crates/open/pxui_sdl/src/*`, `crates/open/sdl3/src/lib.rs`, `crates/open/sdl3_sys/src/lib.rs` (+ its C shim),
  `crates/open/pxui_wayland/src/lib.rs`, `crates/private/pxui_atspi/src/{bridge.rs, map.rs}`, `crates/host/pxui_host/src/*`.
  Findings 2 (routing inside pxui_sdl by window id; the `input::Event` field lands in C1), 3, 4, 6, 44, 16 (host doc).
- **C1 the 2.0 cut** — owns `include/pxui.h`, `abi/*.txt`, `docs/abi.md`, `docs/abi-baseline.md`, `fixtures/abi/`, `tools/gate/`,
  `tools/abi-old-app.c`, `tools/abi-fixture.sh`, `crates/public/pxui_types/src/*`, `crates/open/input/src/{lib.rs, wire.rs}`,
  `crates/private/pxui_abi/src/{lib.rs, ui.rs, ui_state.rs, kit_ext.rs, widgets.rs, collections.rs, overlays.rs, composite.rs,
  paint.rs, theme.rs, style.rs, bodies.rs, a11y.rs, interaction.rs, a11y_state.rs, text_width.rs}`, and every `crates/public/pxui/src`
  file except `composite.rs` (B3). Findings 1, 12, 13, 18, 21, 27 (decision), 28, 32–42, 45, 46, 47, 48, 49, 50, plus
  the C/header halves of 2, 3, 5, 8, 9, 43. Sub-steps:
  C1.1 records (size prefixes, strided spans, `pxui_supports`, error model);
  C1.2 Event/FrameIn redesign with surface ids;
  C1.3 export drops and renames;
  C1.4 extension and diagnostics moved out;
  C1.5 manifest re-baseline;
  C1.6 LocalWave migration (separate LocalWave commit).

## 2.0 cut items

### Exports to drop (survivor in brackets; the survivor takes the plain name)

- Frame and input: `pxui_ui_frame` + the `PxuiInput` input role [`pxui_frame` with the 2.0 FrameIn]. `pxui_ui_input` returns the
  routed 2.0 frame record.
- Create: `pxui_ui_create` [`pxui_ui_create_with_fonts`, renamed `pxui_ui_create`, with a status out-param].
- Labels: `pxui_label`, `pxui_ui_label` [`pxui_ui_label_node` → `pxui_ui_label`].
- Buttons: `pxui_button`, `pxui_ui_button_styled`, `pxui_ui_button_with`, `pxui_ui_button_plain`, `pxui_ui_button_response`
  [`pxui_ui_button_content` → `pxui_ui_button`, with an optional `PxuiSignal *` out].
- Icon buttons: `pxui_ui_icon_button` (deprecated 1.46), `pxui_ui_icon_button_plain`, `pxui_ui_icon_button_with`
  [`pxui_ui_icon_button_response` → `pxui_ui_icon_button`, with an optional signal out].
- Toggle: `pxui_toggle`, `pxui_ui_toggle_plain` [`pxui_ui_toggle_response` → `pxui_ui_toggle`].
- Segmented: `pxui_ui_segmented`, `pxui_ui_segmented_plain` [`pxui_ui_segmented_response` → `pxui_ui_segmented`].
- Dialog: `pxui_ui_dialog` [`pxui_ui_dialog_response` → `pxui_ui_dialog`].
- Collections: `pxui_ui_list_view`, `pxui_ui_tree`, `pxui_ui_table_view` (text-callback forms) [the `_with` forms + `pxui_ui_item_text`];
  `pxui_ui_table` (pre-table_view table) [`pxui_ui_table_view`].
- Text field: `pxui_text_field` [`pxui_text_field_with_options` → `pxui_ui_text_field`].
- Ui text: `pxui_ui_text`, `pxui_ui_text_spans` [`pxui_ui_text_line_box` → `pxui_ui_text` with spans and line box optional];
  `pxui_ui_text_top` (a paint-geometry helper) [`pxui_ui_resolve_label` + metrics].
- Measurement: `pxui_text_width`, `pxui_ui_text_width`, `pxui_ui_cap_width` [`pxui_text_width_spans` → `pxui_text_measure`];
  `pxui_shaper_width` [`pxui_shaper_width_spans` → `pxui_shaper_measure`]; the `key` parameter goes from both.
- Painter text: `pxui_paint_text`, `pxui_paint_text_color`, `pxui_paint_text_size`, `pxui_paint_text_with`, `pxui_paint_text_styled`
  [`pxui_paint_label` (with a `digits` flag, absorbing `pxui_paint_label_chars`) and `pxui_paint_text_spans`].
- Keys: `pxui_ui_key_pressed` [`pxui_ui_key_pressed_with`]; `pxui_ui_hovered` vs `pxui_ui_hovered_within/exactly` (keep one with a
  flag).
- Theme: `pxui_theme_load` (file I/O), `pxui_ui_set_theme` (text + flags) [`pxui_theme_create` + `pxui_ui_theme_apply` + `pxui_ui_use_theme`];
  the density/reduced-motion flags become theme keys.
- Variant: `pxui_ui_variant` [a `variant` field in every widget desc].
- Process/diagnostics: `pxui_verify_new_views`, `pxui_counter`, `pxui_ui_phase_times`, `pxui_ui_phase_cpu_times`, `pxui_ui_set_timing`,
  `pxui_ui_verify_views`, `pxui_ui_verifying`, `pxui_ui_text_layout_count`, `pxui_ui_rebuilt_named` [`pxui_ui_diagnostics` sized
  record behind `pxui_supports`, or a diagnostics extension].
- Prefix consistency (rename, same shape): `pxui_col`, `pxui_panel`, `pxui_scope`, `pxui_within`, `pxui_part`, `pxui_part_box`,
  `pxui_slot` → `pxui_ui_*`.
- Extension set (`abi/pxui.ext.txt`, 27 calls: inspector_*, picker, line_field, value_field, …) leaves libpxui 2.0 for a Rays library.
- Add: `pxui_supports`, `pxui_ui_diagnostics`, the surface registration calls of finding 1.

### Records whose compat-size reads go (2.0 reads only the full 2.0 size; `read_desc!`'s `$old` arm is deleted, `macros.rs:81-92`)

- `FrameIn` 48 / 56 / 128 / 152 → one size (`ui.rs:376-384`, `read_frame` `ui.rs:411-421`; the layout rows `FrameIn`, `FrameInV1_8`,
  `FrameInV1_24`, `FrameInV1_26`, `FrameInV1_57` collapse to one).
- `PxuiButtonDesc` 96/104 (`kit_ext.rs:66`), `PxuiIconButtonDesc` 96 (`kit_ext.rs:114`), `PxuiToggleDesc` 88 (`kit_ext.rs:167`),
  `PxuiSegmentedDesc` 64 (`kit_ext.rs:202`).
- `PxuiMenuExtra` 80/104 (`overlays.rs:67, 90-115`, `MENU_EXTRA_V1`/`MENU_EXTRA_V149`).
- `PxuiSectionListDesc`/`PxuiMeasuredSectionDesc` 64 (no `key`) (`kit.rs:24-47`, `read_section_desc`).
- `PxuiNode` pre-1.39 sizes 488/600 (`docs/abi.md:92`; `node.rs`).
- Body configs: "a body reads `variant` only when `size` covers it" (`docs/abi.md:107`) and every `Pxui*Config` grown-at-end tail
  (Label 56→144, Button →232, IconButton →136, Toggle →192, Segment →112, Dialog →88, MenuRow →112): 2.0 passes the full record;
  bodies may assume it.
- `TextStyle` "legacy measure/quads/raster/canvas" prefix reads (`text.rs:1-6`).

### Event redesign (size prefix + surface id)

```c
typedef struct {
    uint32_t size, kind;            /* size = sizeof(PxuiEvent2); kinds append-only, 11 (FILE_DROPPED) reserved */
    uint64_t surface;               /* the surface it is for; must be a surface of this Ui */
    double   time;                  /* host monotonic seconds */
    uint32_t modifiers, reserved;
    union {
        struct { double x, y; uint32_t button, clicks; } pointer;      /* finite, logical points */
        struct { uint32_t code, repeat; } key;
        struct { PxuiStr text; int64_t start, length; } text;          /* TEXT_INPUT, PASTE, TEXT_EDITING (byte offsets, checked) */
        struct { double dx, dy; uint32_t phase, reserved; } scroll;    /* SCROLLED, TRACKPAD_SCROLLED, PINCHED (dx = scale) */
        PxuiTouch touch;
        struct { double x, y; PxuiStrSpan paths; PxuiStr error; uint8_t failed, reserved[7]; } files; /* FILES_DROPPED, drag moved/ended */
        struct { uint64_t id; PxuiStrSpan paths; PxuiStr error; uint8_t failed, reserved[7]; } dialog; /* FILE_DIALOG, DIALOG_OPENED */
        struct { uint32_t width, height; float scale, reserved; } resize; /* RESIZED, PIXEL_SIZE_CHANGED */
        uint8_t pad[64];
    } u;
} PxuiEvent2;
typedef struct { const PxuiEvent2 *ptr; size_t len, stride; } PxuiEvent2Span;  /* stride >= the 2.0 size; elements can grow */
```

- `FrameIn` 2.0: `{size, surface, PxuiEvent2Span events, width, height, scale, time, appearance, reduced_motion, environment (sized),
  keyboard (sized)}`. Width/height/scale finite and in range; the surface is known. Per-surface environment/keyboard.
- Per-kind validation table in one function used by `pxui_frame`, `pxui_ui_wants_frame` and the wrapper. Finite coordinates,
  UTF-8 and byte budget per frame, path count and total bytes, known codes. One table-driven test with a valid case and an
  invalid case per field.
- Window-level kinds (RESIZED, CLOSED, FOCUS_*, VISIBILITY, ENVIRONMENT, PIXEL_SIZE) apply to `surface`.
- `input::Event` gains the surface id (C1). `pxui_sdl` routes by window id (B4).

### Manifest re-baseline steps

1. `ABI_VERSION` = `2 << 16 | 0`; the wrapper refuses major ≠ 2 (`crates/public/pxui/src/lib.rs:383`); update `abi_version` docs.
2. Freeze `abi/pxui.v1.txt` and `abi/layout.v1.txt` as history (no edits). Generate `abi/pxui.v2.txt` and `abi/layout.v2.txt` from code
   (`cargo run -p gate -- abi-snapshot --write` taught v2). `minor 0`, and only the survivors and new calls.
3. Ext set: delete `abi/pxui.ext.txt` from the frozen library (or move it with the Rays library); `abi-snapshot` stops special-casing it.
4. `include/pxui.h`: drop the dropped declarations and the "ABI 1.x" history comments. Add static size/offset asserts for every
   sized record and `PxuiEvent2`. State the thread/lifetime contract (finding 47) and the error model (finding 40).
5. `docs/abi.md`: restart the history table at 2.0 with one row "2.0 — clean cut" that lists drops and renames. Keep the 1.x
   table under "History (1.x, frozen)". Update the Policy paragraphs: features via `pxui_supports`; no compat-size reads inside a
   major.
6. Old-app gates: `fixtures/abi/1.*` cannot run against 2.0. `abi-compat` and `abi-frozen` either retire the 1.x fixtures or pin
   them to a 1.x library build. Then `tools/abi-fixture.sh 0` records `fixtures/abi/2.0/` (header, manifest, C app) on macOS and
   Linux.
7. Re-run the `abi-header`, `abi-snapshot`, `abi-compat`, `abi-frozen` and shape gates. Run `abi_bench` (static and dynamic, including
   the bodies page), the gallery goldens, and the warm-allocation suites in both link modes.
8. LocalWave: one migration commit on `rust-port` (`lw_ui`/`lw_bin` callers of renamed/dropped calls). Then `cargo t -p lw_ui`,
   `cargo build -p lw_bin` and `make check`.
