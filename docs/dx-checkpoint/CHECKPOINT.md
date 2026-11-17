# CHECKPOINT — pxui API redesign (DX) state, 2026-11-17

This is a hand-off for an agent who will analyse the state and re-plan. The owner stopped new work because the process had become too wasteful. Everything that was in flight was told to finish its current row only. The section "In-flight at wrap-up" records how each lane ended.

## 1. Sources of truth

| What | Where |
|---|---|
| Plan (rows DX0–DX7) | https://artifacts.microapps.space/a/mr55jcx8k90t#plan |
| Status, one row per step | `docs/rust-port.md` (DX rows) |
| Approved API spec, condensed | `docs/dx-checkpoint/api_spec.md`. Covers "How an APP uses pxui" and "Nodes in app code". |
| OS-grade criteria (pxui will be an OS base) | `docs/dx-checkpoint/os_criteria.md` |
| Owner decisions | `AGENTS.md` → Process. Benchmarks and the full suite run only at wave checkpoints. Clean ABI 2.0 at the DX7 freeze. OS-grade review. |
| Audits that generated most of the remaining work | `docs/dx-checkpoint/dx1.12_audit.md`, `os_audit.md`, `widget_reuse_audit.md`, `abi2_cut.md` |
| Follow-ups, bugs, notes | `docs/dx-checkpoint/dx1.12_notes.md` |
| Agent briefs and review checklists | `docs/dx-checkpoint/brief_*.md` |
| Tools (U0.1 moved them; the lane and landing scripts were deleted, landing is the Unification Plan's §7) | `tools/dx/` |

## 2. Where things stand

- **Planned rows:** 112 done, 15 todo in `docs/rust-port.md` before this wrap-up landed.
  - The todo rows are DX1.12, DX6.12–6.16, 6.20–6.25, DX7.7, 7.9, 7.10 and 7.12.
  - Those 15 hide a lot of work. DX1.12 (the public-surface audit) and the widget-reuse audit added about 35 sub-rows that aren't individually listed in the status table.
- **ABI:** pxui `origin/rust-port` was at ABI 1.69 at the start of the wrap-up, with feature ids up to 26.
- **Correctness:** the DX3 regression block was green (full pxui `cargo t`, doctests, gallery, gates, LocalWave `make check`).
- **Performance checkpoints:** none have run since the owner deferred them. They are scheduled after DX3, after DX4 and at the freeze. Release abi_bench and `make performance` are owed.

### Audit batches (DX1.12, 81 findings, batches A–J)

| Batch | Status |
|---|---|
| A, E, F1, F2 | Landed |
| B (paint/image) | In flight at wrap-up |
| C (geometry/theme) | In flight at wrap-up |
| D (authoring layer) | In flight at wrap-up |
| G (body vocabulary) | In flight at wrap-up |
| I (namespace/errors) | In flight at wrap-up |
| H (literal sweep) | Not started |
| J (docs completeness) | Not started |
| K (generics: build speed) | Recorded in notes, not started. Public generics may only be one-line forwarding shells. Known violations: `Paint::selection<F>`, `node_mut<R>`. The `shape` gate exempts the public crates. |

### OS audit (51 findings)

- B1, B2a, B2b, B3 and B4 have landed.
- C1, the ABI 2.0 cut, is not started. Its list is in `abi2_cut.md`: records, Event and FrameIn with surface ids, dropping legacy exports, moving the extension set out, and re-baselining the manifests.

### Widget-reuse audit

There are 85 LocalWave sites: 35 use the correct widget and 50 are drift (controls rebuilt from `node` or `pressable`).

pxui gaps G1–G13 were assigned to these rows:

| Row | Gaps | Covers |
|---|---|---|
| DX3.18 | G2, G3, G11, G13 | Button icon/detail/accessory, IconButton selected, inline link, dialog button detail |
| DX3.19 | G1 | Tabs and segmented: count, no selection, id keys, focus on press |
| DX3.20 | G5, G9, G10 | Passive checkbox, A–Z index, disclosure |
| DX3.21 | G6, G12 | Bottom and full-screen sheet, picker artwork and headings |
| DX3.22 | G4, G7, G8 | List/table id keys and sections, drop, reorder. Only 3.22a was allowed to finish. |

The LocalWave migration of the 50 drift sites onto real widgets with `localwave-*` bodies and theme looks has **not started**. It depends on the gap rows.

### LocalWave rows

| Row | Status |
|---|---|
| DX4.15 | Shadow theme removed: `Looks`/`resolve`/`COLORS` replaced by const token looks. In flight. |
| DX4.16 | Display-text keys replaced by ids. In flight. |
| DX4.11 interim | Remaining hand-built clickables become `pressable`, with `enabled(false)` for inert ones. In flight as the amend of `068ba9d`. |
| DX4.6a | Settings chips on `segmented`. **Blocked** on DX3.19's focus behaviour. WIP branch `dx4.6a-chips-wip`. |

## 3. In-flight at wrap-up

Each lane works in `/Users/snowbear/WORK/GIT/dx-wt/<lane>`, with LocalWave lanes at `lw-<lane>`. Branch name = lane name.

<!-- WRAPUP-RESULTS -->
(updated when lanes report; see bottom)

## 4. Why it got wasteful (input for the re-plan)

1. **Scope grew mid-flight.** Three audits (DX1.12, OS, widget reuse) added about 50 findings after DX3/DX4 were already "done". Several landed rows were then redone: LocalWave sites went first to `pressable`, and now need to become real widgets.
   - Re-plan suggestion: freeze the target API first, then the gap rows, then migrate each LocalWave area once.
2. **Too many parallel lanes on one Mac.** The load average hit 375. Lanes waited on each other's builds and on the shared landing lock.
   - The second machine (gigacomp, WSL) went down because remote build targets filled its Windows host's C: drive. It was fixed: 220 GB freed, vhdx compacted, and the remote area is capped at 120 GB with GC on landing.
   - LocalWave tests need macOS, so they can't be offloaded.
   - Suggestion: at most 4–5 build lanes, with pxui iteration on gigacomp.
3. **Fingerprint-identical requirement on LocalWave migrations.** Every migration must reproduce frame and accessibility fingerprints exactly. Small a11y differences, such as focus-on-press versus focus-on-click and the disabled bits, blocked rows for hours.
   - Suggestion: allow listed, reviewed a11y/behaviour diffs when they are the correct widget behaviour.
4. **Review round-trips.** Agents repeatedly drifted from the spec: shims, extra arguments, text keys, rebuilt widgets. Each drift cost a full rebuild-and-rerun cycle.
   - Strict review checklists now exist (`brief_review.md`, `brief_review_lw.md`). Running them before the first build would be cheaper.
5. **Tooling traps.** Doctests weren't actually running until DX6.0a. Targeted landings didn't compile workspace tests. A shared `git stash` was used across worktrees. The bench lock went stale. Submodule pointers fell out of sync. All are fixed in the scripts, but each one cost hours.
6. **Docs rows (DX6.x) were split too finely** (per file), and they depend on signatures that are still changing. Do them once, after the API is final.

## 5. Remaining work, for re-planning

Roughly in dependency order. Effort is a rough wall-time estimate at the old pace.

1. **Land in-flight work.** See section 3.
2. **pxui gap rows not finished:** see section 3, plus DX3.22b/c (drop, reorder) and DX1.12K (generics). About 3 h.
3. **Audit batches H and J.** About 1.5 h.
4. **LocalWave widget migration** of the 50 drift sites, by area: files, settings, command palette as `picker`, tabs, sort and chips as `segmented`, links, icon keys. Also DX4.6a chips. About 3–4 h.
5. **Docs:** DX6.12–6.25 in one pass, then `deny(missing_docs)`. About 1.5 h.
6. **The 2.0 cut** (OS-C1, `abi2_cut.md`), then DX7.7, 7.9, 7.10 and 7.12 (freeze). About 3 h.
7. **Checkpoints:** full regression, release abi_bench, `make performance`, replace the perf table, measure the popup-layer draw call. About 1–2 h.

Known small follow-ups are in `dx1.12_notes.md`:
- Clock and overlay positioning by baseline.
- `Grid` `'static`.
- The submenu placement bug.
- The flaky `prewarmed_song_cells` fingerprint test.
- `detail::line` flags.

## 6. Non-negotiables (owner)

- **One API:** one high-quality public API as specced. No twins, no compat shims, no second shapes.
- **Customization:** LocalWave customizes pxui widgets through the theme (const token looks) and through registered bodies plus variants. No app-side widget replicas, no shadow theme, keys are ids and never display text.
- **Zero allocations:** zero warm Rust allocations in app and libpxui frames.
- **Build speed:** const `Copy` styles with const fn builders. No generics or `impl Trait` in the public surface beyond one-line key/closure forwarding shells. No proc macros, no new dependencies.
- **OS-grade:** see `os_criteria.md`. ABI 2.0 at the freeze, append-only after it.
- **Testing cadence:** targeted tests per row, with full regression and benchmarks at checkpoints. Never loosen budgets, deadlines or manifests to pass.
- **Process:** one step per commit with its row ID. Push pxui `rust-port` first, then the LocalWave pointer.

## Wrap-up results (lane → outcome)

- **DX4.16 (LocalWave id keys)**
  - Commit `80cd7f2`, branch `dx4.16`. Reviewed by the orchestrator. Landed: see the landing log below.
  - Changes keys for headings and cells, back links and option chips. Accessibility names are unchanged.
- **DX4.11 interim (`pressable` for the last hand-built clickables)**
  - Commit `7dde593`, branch `dx4.11-lw`. **Review FAIL, not landed.**
  - Fingerprints are identical except the accepted disabled bit.
  - The reviewer's 7 mismatches:
    1. `files.rs` buttons, the back link, settings back and the phone Select/Back must be `button_with` / `button_content` with const looks, not `pressable`.
    2. Per-frame look literals must become consts: palette rows via `PLAIN.selected(..).look(&CONST)`, the shuffle/keep fill, the source-card `ui.color()`.
    3. The commit message must name the pxui gap for every site that stays `pressable`:
       - Icon keys: `IconButton` has no `focus_on_click`.
       - Tabs: no count and no `focus_on_click`.
       - Add music: no button detail line.
       - Palette: no picker artwork slot.
  - Re-plan note: these sites belong in the widget migration. The interim `pressable` step is probably not worth landing.
- **DX3.21 (sheets and picker rows)**
  - WIP `b9f0051f`, branch `dx3.21`, about 40% done. Not reviewed. Only `pxui_core` checks.
  - Done:
    - Types: `Presentation` {Centered, Bottom, Full}; `DismissOpts` moved into `pxui_types`; `Overlay.present` and `.dismiss`; `PickArt`; `PickRows` heading/art/group.
    - Engine: placement plus drag-to-dismiss.
    - FFI records `PxuiSheetDesc` and `PxuiPickRow`; feature 27, provisional.
  - Left:
    - Re-exports.
    - The picker engine's headings and artwork.
    - The ABI export `pxui_ui_modal_sheet` plus validation and fuzz.
    - The wrapper.
    - Manifests and header.
    - Tests: custom body, placement, drag, zero allocations.
    - Review, then rebase.
- **DX4.15 (shadow theme deleted)**
  - Landed: LocalWave `a3c8ea1`. Review PASS. Fingerprints are identical in light and dark.
  - New pxui gap **G14**: `Rgba::token` can't carry alpha. The song-row 60% PANEL hover and the `artists.rs:170` hover are still built from `ui.color()` per frame.
  - About 128 `ui.color(X)` arguments to parametric helpers remain, along with the 2 R2 `Look` literals in `files.rs`.
- **DX4.16 (stable id keys)**
  - Landed: LocalWave `ad48bd3`; the status row is fixed in the checkpoint commit.
- **DX1.12G (body vocabulary and sealed body configs)**
  - Landed: pxui `98ca4958`, LocalWave `7c9c670`. Both reviews PASS.
  - Body kinds renamed to match the public calls:

    | Old | New |
    |---|---|
    | `context_menu` | `menu_row` |
    | `list_view` | `selection_list` |
    | `table_view` | `table_row` |
    | `table` | `table_cell` |
    | `tab` | `tabs` |
    | `segment` | `segmented` |
    | `toast` | `toasts` |
    | `tip` | `tooltip` |

  - Body accessors now sit on a sealed per-kind `BodyConfig`, so safe code can't forge a config.
  - The C header names wait for the 2.0 cut; they're listed in `abi2_cut.md`.
  - Untested: the tooltip `text_at` path. `pxui_core::layout_scaling` failed once on the overloaded Mac with a timing assertion; check it at the checkpoint.
- **DX1.12C (value types: one const shape each)**
  - Landed: pxui `8d0e1f41`, LocalWave `bb8df23`. Reviews PASS.
  - Conflicts with G and DX4.15 were resolved by the orchestrator: G's `c.text_of` method, `pxui::ui::key_of`, and the hover wash now uses `ui.color(PANEL).with_alpha(153)`.
- **DX1.12I (namespace and one error policy)**
  - **Not landed.** pxui `3201fa20`, branch `dx1.12i`; LocalWave `49bf3a8`, branch `lw-dx1.12i`. Both are based on pre-G/C pxui and need a rebase.
  - LocalWave review PASS; pxui review **FAIL**, on 3 owner decisions:
    1. One theme setter: `set_theme(&ThemeHandle)` only, carrying density and reduced motion. About 35 call sites, with a golden risk.
    2. Whether `pub mod text` and `pub mod bodies` stay public namespaces.
    3. `library_counter`'s `fn(u32) -> u64` shape (`mem_sys` protocol).
  - Done in the branch:
    - Shim modules deleted; one `Ui::frame(&FrameIn)` plus builders.
    - `Ui::new`/`create`/`Renderer::new` return `Result`.
    - A documented error policy in pxui `AGENTS.md`.
    - `KeyCode` and `Appearance` types.
    - `ThemeHandle` returns `Result`.
    - `a11y_*` calls ignore gone nodes.
  - Not re-verified: LocalWave against the final `3201fa20`; the gallery goldens, since the gallery input moved to `FrameIn`.
  - Follow-ups found:
    - `state`/`set_state` touch a gone node's slot until the slot is reused (engine bug).
    - Leftover String/Option errors: body registration, `color::hex`, `Text::new`, `images::Path::new`, `pxui_host`.
    - Unsound until 2.0: dangling `FrameIn.events` pointers; `PxuiBox` carries no `Ui` identity.
- **DX3.19 (tabs and segmented gaps: G1 plus focus on press)**
  - **Not landed.** pxui `f3d19710`, branch `dx3.19`, provisional ABI 1.70. Not rebased onto G and C.
  - The review found 4 mismatches; 3 are fixed but not re-reviewed.
  - The open one: legacy `pxui_ui_tabs` and `pxui_ui_choice_keyed` kind 1 are kept as "Legacy (cut at 2.0)". Recommendation: accept, consistent with `abi2_cut.md`.
  - Needs a re-review, Mac checks and gates. It unblocks DX4.6a (chips).
- **DX3.20 (passive checkbox, disclosure, A–Z section index: G5, G10, G9)**
  - **Not landed.** pxui `41bd6ea9`, branch `dx3.20`, provisional ABI 1.70 with feature 27 and body kind 40. Collides with DX3.19's 1.70, so renumber at landing. Not rebased onto G and C.
  - New shapes:
    - `Checkbox.size/look/enabled/passive`.
    - `Ui::disclosure(key, label, open, f) -> bool`, built on the `inspector_section` engine.
    - `Ui::section_index(key, letters, &SectionIndex) -> (Response, Option<usize>)` with its own body kind.
  - The review found 4 mismatches; 3 are fixed but not re-reviewed.
  - The open one: `inspector_section` is still public next to `disclosure`. Recommendation: demote it to the gallery/inspector, so there is one public shape.
- **DX3.22a (list and table rows keyed by item id: part of G4)**
  - **Not landed.** pxui `96ceb76f`, branch `dx3.22`, provisional ABI 1.70 with feature 27 `LIST_ROW_KEYS`. Not rebased onto G, C and B.
  - Review: 1 doc-only finding, fixed; not re-reviewed.
  - What it does: `selection_list`/`table_view` take `id: &dyn Fn(usize) -> i64`, and the records grow by `key`. The old record sizes are still read and key by index.
  - Branch `dx3.22-wip` (does not compile) has the start of G7: `drop_target_ref` and `State.drop_hover`.
  - Not started:
    - The rest of G4: sections, header, `secondary`, a per-row handle, sort states, weighted widths, a standalone header.
    - G8: reorder plus a drag handle.
    - The G7 insertion indicator.

### ABI numbering for the re-plan

DX3.19, DX3.20, DX3.22a and DX1.12B each claim provisional 1.70 and feature id 27. Land them in sequence, and each one renumbers its minor, feature id and manifest rows.
- **DX1.12B (paint and images)**
  - Landed: pxui `11a546d1` (ABI 1.70, feature 27 `IMAGE_CHECKED`), LocalWave `02fc990`. Reviews PASS; fingerprints identical.
  - Left over: `Ui::text_width/text_top/cap_width` still take an integer size; `ArtworkReady::key` should be renamed.
- **DX3.18 (Button parts: G2, G3, G13)**
  - Landed: pxui `28fc92de` (ABI 1.71, feature 28 `BUTTON_PARTS`), LocalWave `3d7799f`. Review PASS; gallery goldens pass.
  - Shapes: `Button.icon/detail/accessory/detail_label/accessory_label`, `IconButton.selected`, `DialogButton.detail`.
  - G11, clickable links inside wrapped text, is deferred. It needs a rich-text widget with per-span hit areas, accessibility nodes and responses.
  - Known limit: a registered body can't draw the app's `Icon` by its icon word.
- **DX1.12D (authoring layer)**
  - Landed: pxui `d6b4182b` (ABI 1.72, feature 29 `VIEW_KEYED`), LocalWave `bdf4b81`. Reviews PASS; gallery goldens and `pxui_render` GPU goldens unchanged.
  - Shapes:
    - Containers are `row`/`column`/`stack` or `node` + `within`.
    - One 3-argument `view_of(key, &inputs, f)`; on a `Fit` main axis the engine sizes the view to its content.
    - One state API: `state`/`set_state` plus `text_state`.
    - One `Chosen` result for menus and dialogs.
    - Style values borrow their looks.
    - `Handle` is the one name.
    - `MenuItem::shortcut`, `DialogButton::primary`, `w_px`, `row_key`.
  - Spec text to amend: `list`/`section_list` take `row_key`; `Chosen` replaces `MenuResponse`/`DialogResponse`.
  - Left over: batch H's `Node {..}` literal sweep in examples and tests.

## Final state after wrap-up (2026-11-17)

- **Heads:** pxui `origin/rust-port` = `d6b4182b`, ABI **1.72**, feature ids up to **29**. LocalWave `origin/rust-port` = `bdf4b81` plus this checkpoint commit.
- **Landed in this wrap-up:** DX4.15, DX4.16, DX1.12G, DX1.12C, DX1.12B, DX3.18, DX1.12D.
- **Branches waiting** (each needs a rebase onto `d6b4182b`, an ABI renumber from 1.73 up, and a review to PASS):

  | Row | Branch(es) | Notes |
  |---|---|---|
  | DX1.12I | `dx1.12i`, `lw-dx1.12i` | Has 3 owner decisions |
  | DX3.19 | `dx3.19` | Unblocks the DX4.6a chips |
  | DX3.20 | `dx3.20` | |
  | DX3.21 | `dx3.21` | WIP, about 40% |
  | DX3.22a | `dx3.22`, `dx3.22-wip` | |
  | DX4.6a | `dx4.6a-chips-wip` | |
  | DX4.11 interim | `dx4.11-lw` at `7dde593` | Review FAIL; recommend dropping it in favour of the widget migration |

- **Owed before the freeze:** a regression and performance checkpoint covering DX3 and DX4. Run the full pxui and LocalWave suites, `make check`, release abi_bench, `make performance`, then replace the perf table.
  - Note: the gallery input moved to `FrameIn` in the unlanded DX1.12I.
  - Note: `pxui_core::layout_scaling` and the LocalWave `prewarmed_song_cells` fingerprint are flaky under load.
