# CHECK — handoff for the pxui Unification Plan

Updated 2026-11-19. Resume at [U4a](#next-u4a).

## What we're doing

LocalWave and pxui are being moved onto **one uniform public pxui UI API**: SwiftUI/React-grade ergonomics, with zero warm allocations, fast frames and fast builds. LocalWave uses pxui widgets everywhere, customised structurally (registered bodies) and visually (theme looks/tokens). Breaking changes are welcome; no compat layers or shims.

- **Plan (source of truth):** https://artifacts.microapps.space/a/nnhryen1cd09 (version 4). A local text copy is at `docs/u-plan/plan.txt`.
  - §3 non-negotiables, §4 decisions D1–D26, §5 API ledger, §7 process, §8 tests/checkpoints, §9 codemods, §10 packages U0–U6, §12 implementer brief, §13 review checklist.
- **API reference spec:** https://artifacts.microapps.space/a/mr55jcx8k90t. The plan's D25 deliberately departs from it on `_with`/short forms.
- **Machine-readable ledger:** `pxui/docs/api.md` part 2. `python3 tools/dx/surface.py --ledger [--all]` diffs the real surface against it.
- **Status:** `docs/rust-port.md`, one row per U step.
- **Process rules:** `AGENTS.md` ("Process") and `pxui/AGENTS.md` record owner sign-offs 1–5.

### Owner decisions (2026-11-17)

1. No Swift work and no Swift/Xcode checks.
2. Per-commit checks are targeted (Tier A); the full suite runs at each package end (Tier B).
3. Unpublished pxui pointers are OK on `u<N>` branches.
4. Build-time script exceptions are OK.
5. Breaking changes are welcome, with no compat layers. The only constraints are performance, build speed and zero warm allocations.

### Package order

Strictly one at a time:

U0 → U1a → U1b → U1c → U2a → U2b → U3 → U4a → U4b (CP1) → U5a → U5b (CP2) → U6.

Each package has the following lifecycle:

1. Work on branch `u<N>` in both repos, from `rust-port`.
2. Each step is one commit per repo, with a subject of the form `[U<N>.<k>] …`. LocalWave commits carry the pxui pointer.
3. Run Tier A per commit and Tier B at the end.
4. Self-review against §13.
5. To land:
   1. Fast-forward pxui `rust-port`, push, then run `tools/check-pxui-remote.sh`.
   2. Fast-forward LocalWave `rust-port`, run `make check` online, then push.
   3. Make one `[U<N>] Status` commit and delete the `u<N>` branches.
6. Fixes after review are new `[U<N>.<k> fix]` commits, never amends.

**Stops:**
- Shape stop (uncommitted signature diff): U1a, U1b, U2a, U2b, U5a.
- Pattern stop (first area committed): U3, U4a, U4b.
- **GAP protocol (U3–U4):** a LocalWave site the ledger can't express gets a gap step `U2b.4.<n>`, added to the ledger and built on the current `u<N>` branch.

## Where it stands

### U0.1–U0.8 landed on `rust-port`

| Step | LocalWave | pxui |
|---|---|---|
| U0.1 tools/dx, `lw_ui::testing::frame` fingerprint helper (`LW_FINGERPRINT=<path>`) | 4cc530f | |
| U0.2 `surface.py`, api_conformance R1/R6/R7/R8 (allowlist blessed at 440 lines, shrink-only) | 3310f2b | |
| U0.3 `buildtime.sh`, ledger `docs/api.md`, `tools/dx/ledger_delta.txt` (1349 lines, 0 untagged) | 3bd0004, 7073f41 | cefefe57, a64d00ca |
| U0.4 branch cleanup | b444b1d | |
| U0.5 `tools/t --all`; prewarm test made deterministic | 8b13a62 | eb93fe54 |
| U0.6 status/process docs | 63972bf | 3db577ab |
| U0.7 CP0 recorded (**red**) | 16dc635 | |
| U0.1–U0.7 status | 8ac43fd | |
| U0.8 restored the build CPU gate | 9c362bb | 23bb10c4 |

### CP0 results and decision D26

The plan's §4 table already records D26.

1. **abi_bench.** Release abi_bench instruction ratios fail in both modes (Kit/Controls/Overlays static; Kit/Controls/Overlays/Layout dynamic). The failure is deterministic. It comes from pxui cf953b35 (DX1.12E) and b6958651 (DX1.12F): out-of-line key-first shims. **This is folded into U1c.1**, whose done-when is now: release abi_bench passes all pages in both modes.
2. **Build CPU p99 > 1.5 ms** on resize, theme-switch, non-song-lists, reverse-scroll and scroll-jump. Warm allocations stay zero everywhere. **U0.8 restores the build CPU gate.**
3. **Popup layer** draws 2 calls on the Layout page (baseline 1). This goes to U2b.8 and must be fixed before U2b.10.
4. **`Ui::id() -> usize`** becomes `instance_id()` in U1b.6, so `ui.id(key, f)` names one thing. It is already in the ledger.

### U0.8 results

U0.8 restores the performance budgets through internal pxui changes to text,
animation, layout and retained views. LocalWave removes redundant row
containers and prepares packed song numbers on the worker. The public Rust
surface and ABI match the baseline. All 115 LocalWave UI checks passed and
their emitted fingerprints match.

The final signed run passed all 30 workloads at peak load 2.776. The highest
gated p99 values were 1.477 ms build CPU, 2.000 ms frame CPU and 3.535 ms work
wall. All 28 gated workloads had zero warm Rust allocations. The previous
run of identical machine code missed the build budget on resize (1.546 ms)
and theme-switch (1.517 ms). Tier A, Tier B and online `make check` passed.
All six build-time measurements met the CP0 allowance, including a 0.98 s
warm app relink. The clean pxui build took 30.42 s against the 30 s target.

See [performance and build times](docs/rust-port.md#latest-performance) and
the [investigation](docs/u-plan/u08-investigation.txt) for benchmark and
package evidence.

### U1a landed on `rust-port`

| Step | LocalWave | pxui |
|---|---|---|
| U1a.1 Namespace | f6b3f94 | 1becc803 |
| U1a.2 Themes and counter | daa47b6 | 62ddd318 |
| U1a.3 Frame, lifecycle and errors | 70bef76, b6a3d2b | 96e7ca2d, 24e54d41 |
| U1a.4 Typed input and docs | f83c3a8, 9173aca, d3c434b | e43ef1e4, e10307b2 |

The 5,913-item Rust snapshot matches the approved U1a ledger. The C ABI is
1.75 and the old 1.57 consumer passes. The shortcut regression passed against
both query APIs. Draw and accessibility fingerprints match U0.8 across 9,263
frames. Warm allocation checks, gallery goldens, Tier A, Tier B and all 288
online LocalWave checks passed. The focused Rust iOS device probe compiles.
Simulator and release qualification follow the named checkpoints.

All six build measurements meet the CP0 allowance. Warm app relink is 0.98 s
against CP0's 1.02 s. Clean framework build is 30.36 s against the nominal
30 s target. See [paired build times](docs/rust-port.md#build-time).

### U1b landed on `rust-port`

| Step | LocalWave | pxui |
|---|---|---|
| U1b.1 Results | dcca1b6, e2a9942, ec0e38b | ec95d41d, c46adee7, e45204f3 |
| U1b.2 Keys | f19b27e, 055307b, 6dcc3e2, 20463e4 | 165fd9d1, c1ad78d0, cbb34aad, 0baa83c6 |
| U1b.3 Measurement and units | d873f01, f127faf | 90367067, d143239c |
| U1b.4 Looks, views, a11y | 0da9123 | 5444131b |
| U1b.5 Builders | e7e9687 | 3f1afb57 |
| U1b.6 Call-shape lock | dada1a1, 1419a82, 13f5428, c2eeb47, 7224eca, 058f687, 1a999a1 | 4ebb0002, 7e3c42e0, 3805ffcd |

The 6,071-item surface matches the ledger. ABI 1.81; the 1.57 consumer, frozen,
export and docs gates pass. Fingerprints match U1a (no D9). Gallery goldens
are unchanged. Every key-label C export refuses unusable resolved labels
(fuzzed); identity-only C labels are capped at 256 bytes. The allowlist shrank
439 → 308. Tier B, two independent review rounds and 289 online LocalWave checks
passed. Build times are within the CP0 allowance; clean pxui is 26.24 s.
Orchestrator rulings: fractional sizes ride in existing record padding (valid
only within 0.5 of the integer field); keyed `node` + `within` stays where tests
query children; `tooltip_legacy`/`anchored_legacy` serve only 1.x C exports;
the six-look `style` cost is measured at CP1.

### U1c landed on `rust-port`

LocalWave c33d0af..687730d and pxui 81487978..3d63d11a (see `docs/rust-port.md` rows U1c.1–U1c.5).
Generics and the shape gate, bounded globals, the `pxui::ext` extension set and the ledger sync are done.
Tier B, 289 online LocalWave checks, fingerprints (no D9) and build times (within 0.25 s of CP0) passed.

**D27 (owner, 18 Nov): abi_bench carried to CP1.** Author and every bodies page pass. Kit, Controls,
Overlays and Layout stay over in both modes with unchanged limits:

| Mode | Kit | Controls | Overlays | Layout |
|---|---|---|---|---|
| static | 1.01631 (1.013) | 1.01422 (1.01) | 1.02103 (1.011) | 1.01626 (1.01) |
| dynamic | 1.03028 (1.03) | 1.04084 (1.03) | 1.03692 (1.03) | 1.04965 (1.045) |

The dynamic Kit small-page wall gate is also over (1.54 µs against 1.5). U2b.10 owns all of it and must pass it before CP1, against the
final U2 panel, pick-item and field shapes. Cost breakdown and the dropped typed-key experiment are in the U1c report
(`scratch/u1c-final-report.md`, local only): per-item radio/dropdown validation, panels ~190/420, nodes ~48/100, and the static
value_field/text_area path (reference-counted field callbacks).

### U2a landed on `rust-port`

LocalWave 8e75a94..241ec6e and pxui c7e6e27e..e1acbdb9 (ABI 1.83–1.86; see `docs/rust-port.md` U2a rows). The four gap
branches are landed in ledger shape and deleted; `dx3.22-wip` remains for U2b.2. Orchestrator rulings: ABI records grow only
by trailing append under a minor bump, for library-written or size-checked caller records (gate-enforced, library code only);
the library refuses an app built for a newer minor (minimum-library rule). abi_bench is unchanged in kind (D27, U2b.10).

### U2b landed on `rust-port`

LocalWave 3b7171e..07acbb5 and pxui 5c368579..61ed76b1 (ABI 1.87–1.92; see `docs/rust-port.md` U2b rows and
`scratch/u2b-final-report.md`, local only). D27 is closed: release abi_bench passes every page in both modes with unchanged
limits, plus a new Lists gallery page (reorder, drop zones, top-anchored modal) with only new goldens, its abi_bench mirror and
bodies reference. Thin margins: static Overlays (about 27 instructions under 1.011), dynamic Controls and Overlays. Tier B,
`LOCALWAVE_OFFLINE=1 make check`, gcmp (no D9), surface/ledger (0 delta) and buildtime against CP0 (clean pxui 27.35 s vs 26.29 s, within allowance) passed.

Independent review: land after fixes; the renderer fence now also clears `building` (a C split callback could build during
draw), static `drop_target` refuses a stale box, header/hash/bench docs, D30 (abi_bench closure, harness changes accepted).

### U2c landed on `rust-port`

LocalWave f7d3131..8ced207 and pxui 233baadd..56bd3dde (ABI 1.93–1.97; see `docs/rust-port.md` U2c rows and
`scratch/u2c-final-report.md`, local only). 101 superseded exports deleted (343 to 242), each successor under the plain name, the engine
legacy bodies gone; `tools/dx/abi_drop.py --check` passes (keep list: C-only calls with no successor). Tier B, `LOCALWAVE_OFFLINE=1 make
check` (289), gcmp (identical to U2b), surface/ledger, release abi_bench both modes (every page within its unchanged limit, no ratio worse
than U2b) and buildtime against CP0 (clean pxui 27.17 s vs 26.02 s, within allowance) passed. Found by a ported test: the text field wrote its outputs before refusing a call outside a frame (fixed in U2c.4).
Review fixes (pxui 333bf96f, ABI 1.98): eleven more exports wrote outputs before a refusal, now only on success; dead old-caller
paths deleted; `ui_section_index_result` renamed `ui_section_index`; docs naming deleted exports fixed.

## NEXT: U4a

0. U4a per the plan (desktop shell; pattern stop), branch `u4a`. Orchestrator rulings on U3: the command field's kit-theme
   foreground stays the one allowlisted R6 dynamic case until U4 replaces the field; settings::paragraph needs no codemod table.
1. Landed U3 (`u3`, LocalWave only, pxui unchanged at 333bf96f): commits 13fbb81, 2848638, 1d63ef8, 6fe6397 (U3.1),
   5a76301 (U3.2), 24a4a68 (U3.3) and the status commit; report `scratch/u3-final-report.md` (local only). Allowlist 308 to 56
   (R1 0, R6 1 with its reason, R4 none); `LOCALWAVE_OFFLINE=1 make check` (289) passed; fingerprints identical light and dark
   except the listed test-fixture colour.
2. Then U4a per the plan (desktop shell; pattern stop), branch `u4a`.
3. U5.3's drop list shrank. What is left: `pxui_scope`, `pxui_ui_animate_keys_repeat`, `pxui_ui_hovered`,
   `pxui_ui_variant`, `pxui_ui_text`, the `pxui_frame`/`pxui_ui_frame` pair (U5.2), the prefix renames of `pxui_panel`, `pxui_within`,
   `pxui_part` and `pxui_slot`, the keep list in `tools/dx/abi_drop.py` and the record cleanup below.

### CP1 actions (from the U2b review)

- (a) Make the abi_bench pages' call shapes deterministic (static Overlays sits about 27 instructions under 1.011, and whether the
  gallery crate inlines a sixth `button::<&str>` call swings it by about 180). No limit change.
- (b) Measure the per-row `drop_target`/drop-look cost in the U4a.5 queue workload; the gallery Lists page does not exercise it.
- (c) Split the dynamic abi_bench target (start/Author vs the pages): the run takes about 59 s of the runner's 60 s cap. Never raise the cap.
- (d) 2.0 cut: the owner decides whether `PxuiUi.theme` (the handle's public first field) is frozen ABI.

## Found (separate fixes later)

- U4a.5: in `pxui_core`, a view building itself clears any rebuild mark made during that build (`r.dirty = r.escapes`); only the
  list's late move (`build_again` + escape) is fixed. Other `mark_view` calls made during a view's own build have the same shape.
- U4a.5: lw_bin `production_controller_restores…` flakes under parallel load (green alone).
- U4a.5 ruling check: pxui cancels a reorder drag on Escape (`a_drag_in_a_kept_view_survives_scrolling_and_cancels`), but a plain held
  press is not cancelled by Escape (`pxui_core/src/ui_core.rs:3081`: only PointerCancelled and FocusLost end the capture), so while
  any press is held LocalWave's Escape (spent on `ui.captured()`, shell.rs) does nothing and the press still clicks on release.
- U4a.6: `Ui::text_input_focused()` is true for any keyboard focus (`pxui_core/src/ui_core.rs:3172`, `focus != 0`), not only a text
  editor; with no `ListResponse::focused` an app cannot tell its focused list from typing (the U4a.6 GAP).
- U4a.1: `selection_list` names its List node from the key text, so an id key gives a meaningless lowercase name ("library");
  LocalWave names its groups with `ui.a11y_name(&response.handle, ..)`. U6 decides whether the default name should be empty.
- U4a.1: `ValueField::bare(on: bool)` takes a bool where `SelectionList::bare()` (U2b.4.3) and the other style flags are no-arg
  builders; API-consistency item for U6.2.
- U3: `Ui::text_line_box` colours node text from the node's look and ignores the `Label`'s colour token; its docs do not say so
  (LocalWave's positioned labels set `theme::text_look(style)`).
- U3: the `Rgba::token` doc says a `Paint` call given a raw token paints nothing, but every Rust `Paint` shape call resolves it
  (`Paint::ink`); stale doc.
- U3: the phone Find field's `Font` has tracking -0.52, but `Font::tracking` is in ems and the code comment says points (-0.02 em
  at 26 pt); kept as is (no visible change in U3), likely a bug.
- U3: `Flags` has no const union, so a `const Node` cannot combine flags (`Flags::SCROLL | Flags::CLIP` stays inline).
- `pxui_core/src/kit_ext.rs:565` `inspector_header_body` keys its name field with `format!` every build (and :613, :877 `inspector_bar`, :1004 `head_button`): warm inspector headers allocate.
- lw_bin `files_worker_adds_and_imports_recursive_folders_browses_rescans_and_rejects_stale_accounts` fails under heavy load (`directory.is_none()` after a stale Browse); green alone.
- The dynamic release abi_bench run takes about 59–60.4 s against the runner's 60 s cap (`abi_bench_start` 29 s; one retry seen). CP1 action (c).
- A drag that pushes a sheet fully below the window cancels instead of dismissing.
- Picker and menu wrappers keep scratch buffers in thread-locals (global state).
- A C app refused at UI creation only gets null; no error channel exists before a Ui.

- `pxui_core/src/ui_core.rs` `tooltip_sheet`: a shown tooltip formats and wraps its text into a `String`/`Vec` every frame.
- `pxui_core/src/ui_text.rs:1733`: the text area's doc tooltip keys its wait with a `format!`ed string every frame.
- `Segment::detail` reaches the engine in both link modes but no body config exposes it.
- `box_ops.rs` `last_press_within`: a touch Began is not matched like a mouse press.
- U2c left for U5 (2.0 re-baseline): dead record fields and old recorded sizes kept (`PxuiSegmentedDesc.names` must be empty; the
  64/96/104-byte segmented, 96/104/192-byte button and other pre-change record sizes are still read), feature ids naming deleted calls
  (`PXUI_FEATURE_TEXT_FIELD_KEYED`, `_SEGMENTED_RESULT`, `_WIDGETS_KEYED`, ids are never reused), unused header records `PxuiF32MutSpan`
  and `PxuiTouch` (before U2c), and `PxuiThemeSnapshot`'s `int64_t` token slots that carry a `float`.
- Engine API that only core tests still call (no export): `Ui::set_colors` (the light/dark token table), `Ui::button(text)`,
  `Ui::toggle(text, on)`, the integer-size `Paint::text_*`/`cap` calls. Deleting `set_colors` (review item, test ported to the theme
  palette, goldens unchanged) makes the direct engine's palette fallback about 60 instructions cheaper and the static wrapper about
  25, so static abi_bench Overlays reads 1.01107-1.01125 against its 1.011 limit; deferred to CP1 action (a). Patch:
  `scratch/u2c-review-set-colors.diff` (local only).
- `pxui_ui_set_role` returns 0 for a stale box, while `pxui_ui_a11y_name` and `pxui_ui_a11y_number` return `PXUI_ERR_STALE`.
- The dynamic release abi_bench run took 60.42 s again (rc 0; CP1 action (c)).

## Commands

- **Tier A (per commit)**
  - pxui: `cargo t -p <crate> [filter]`; `cargo t -p pxui --features static`; `cargo check --workspace --tests`; `cargo check -p abi_bench -p pxui_gallery --tests` (also with `--no-default-features`); `cargo run -p gate -- shape|deps|abi-header|abi-snapshot`.
  - LocalWave: `cargo t -p lw_ui [filter]`; `cargo t -p lw_ui --test api_conformance`; `cargo build -q -p lw_bin`; `cargo check -p lw_bin --tests`; `python3 tools/dx/surface.py --check`; `git diff --check`.
- **Tier B (package end)**
  - pxui: `cargo t --all`; `cargo t -p pxui --features static`; `cargo t -p pxui --doc`; `cargo t -p pxui_gallery`; gates docs, abi-exports. (abi-frozen/abi-compat are retired for 1.x by D28; they return at 2.0 against re-baselined manifests.)
  - LocalWave: `LOCALWAVE_OFFLINE=1 make check`; `tools/dx/buildtime.sh --compare <CP0 commit 16dc635>`.
- **Checkpoints (CP0/CP1/CP2):** Tier B, plus `make performance`, release abi_bench in both modes (`RAYS_SLOW=1 cargo t --release -p pxui_gallery [--no-default-features] abi_bench`), and `make ios-build`.
- **The test runner** (`cargo t`) enforces 5 s per case and 60 s per run. Never raise these deadlines.

## Machine hygiene (Mac mini, the M1 reference machine)

- Performance numbers are only valid at **load average < 4**. Check with `sysctl -n vm.loadavg`, and wait with an until-loop.
- Earlier runs were polluted by load averages of 30–130: DaisyDisk, Notion Calendar, the Simulator, Spotlight and fseventsd indexing `target/`. `target/` and `pxui/target/` now carry `.metadata_never_index`.
- fseventsd can sit at 100% CPU for weeks; restarting it (sudo killall fseventsd) brought the clean pxui build from ~34 s to 26 s. Don't restart it during a build: an in-flight incremental move failed once.
- Don't run builds while measuring. Use one implementer at a time, in the main checkouts, with no stash and no extra worktrees except temporary bisect/buildtime ones.

## Never

- Push from a `u<N>` branch before review.
- Amend.
- `rustfmt` pxui or pxui_types.
- Hand-edit ABI manifests (use `gate -- abi-snapshot --write`).
- Add dependencies, twins, shims, aliases, compat layers, or public API outside the ledger.
- Weaken a test, deadline, budget or limit.
