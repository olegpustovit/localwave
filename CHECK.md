# CHECK — handoff for the pxui Unification Plan

Updated 2026-11-22. The U6 candidate is on rust-port at LocalWave 4564228 and pxui 8f7c5b2b. Its 6,348-item surface matches the ledger, all 1,560 doctests passed, and production source matches CP2. CP2 passed the macOS bundle and idle smoke, the Rust iOS device build, the iOS 18.6 simulator idle smoke and zero warm Rust allocations across all 28 gated workloads. Benchmark follow-ups are in [the deferred list](docs/u-plan/deferred.md). The 2.0 tag and close-out await the records lint scope decision.

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


### U5b independent review and fixes (21 Nov; not landed)

Independent review: `scratch/u5b-review.md`. Original four findings plus native surface propagation and pending-action teardown now pass source re-review. No runtime, broad-validation or Linux claim follows from source review alone.

| Fix | LocalWave | pxui |
|---|---|---|
| U5.7 aggregate menu text/shortcut/submenu budget | 70c5816 | b39573f4 |
| U5.9 diagnostics respects callback/renderer/poison ownership | 16c8930 | 3a673d0f |
| U5.3 refused builders never install descriptor variants | 9b57a34 | 5f091ccb |
| U5.9 surface-aware actions, bridge update/removal, native host routing, pending actions cleared on teardown | 36bfc99 | 4b0f1e83 |
| U5.9 4096 pending accessibility actions across calls, atomic refusal and warm storage reuse | d27b612 | f0b3aee0 |

Owner decision (21 Nov, this Codex session): correct surface routing in the 2.0 API and ledger, then regenerate the unreleased baseline. `PxuiA11yAction.surface` names its nonzero destination; `A11yBridge::update(surface, tree)`/`remove_surface(surface)` and `Accessibility::frame(ui, surface)` preserve it. The layout/snapshot tools and `abi-fixture.sh 0` regenerated the baseline; no gate was weakened. The snapshot gate passes after the baseline commit.

Evidence in `scratch/u5b/review-*.log`: framework Tier A checks and shape/deps/header/snapshot gates; full static API suite (316); focused ABI menu/diagnostics/variant/surface/queue tests; native VoiceOver bridge round trips on non-main surface 7; bridge docs and new injection doctest; frozen/compat gates against the regenerated 2.0 fixture. LocalWave api_conformance, binary build/test compilation, surface and two-way ledger checks pass on the surface correction. First sandboxed full static attempt timed out at the unchanged 60 s run bound; unrestricted runs pass in about 34 s. Full Tier B and benchmarks must be repeated after all runtime fixes.

The nested submenu regressions cover warm allocations and submenu behavior in both link modes. The Linux ABI and private-bus AT-SPI runs exercised surface routing and teardown. The Linux test container is stopped. A desktop AT-SPI registry and libatspi integration was outside those runs.

Remaining before landing: final Tier B, fingerprints, hot-path release abi_bench and the online landing checks. CP2 and U6 follow.


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

## U4b package end done; NEXT: independent U4b review, then landing, then CP1

U4b (compact, files, detail) is done on branch `u4b` in both repos, nothing pushed: LocalWave rust-port..0be2cb9 (+ this status
commit) and pxui rust-port..17068da2 (ABI 1.124-1.130; gap steps U2b.4.29-U2b.4.37; next gap number U2b.4.38). Review round
(orchestrator rulings): U2b.4.37 `Table::mode(SelectMode)` replaces `multi()`, with `open_on_click` and `bare` (pxui 17068da2,
LocalWave 7bb0f61): the folder table selects nothing, opens on a press or Return, never on an arrow, and has no frame;
`compact_folder`'s per-frame counting passes are folded into its per-key flag pass (a warm frame walks no row; test counter). Every compact, files
and detail site is on its widget per the site map and the rulings (#73 desktop folder table on `ui.table` + `table_head` with
U2b.4.36 `TableHeadConfig.built_in`; ruling 3 phone folder list on `selection_list` Toggle/None, #80 composites deleted). The
api_conformance allowlist is at its final size, 10 lines (9 composite pressables + the D7 byline), and U4b.5 fails any line without
a composite/D7/dynamic-colour reason. Account (commits, signatures beside the ledger, D9, drift list, perf, Tier B, questions):
`scratch/u4b-final-report.md`, local only.

1. NEXT: independent U4b review, then landing (pxui `rust-port` first,
   `tools/check-pxui-remote.sh`, LocalWave online `make check`), then CP1 (U4b.6: Tier C, iOS build and probes, make performance).
2. Package end (20 Nov): Tier B both repos green (LocalWave `LOCALWAVE_OFFLINE=1 make check` 307 tests; pxui all, static, doc,
   gallery, gates); gcmp light/dark vs package-start bases rc 1 with the same D9 classes (focus/scroll folded into lists and tabs,
   the rail letters, mode-key and player names) plus frame shifts; warm-allocation tests green; release abi_bench instruction gates
   pass in both modes; dynamic Lists wall flap and buildtime figures under CP1 action (f).
3. Orchestrator rulings carried by U4a (19 Nov): GAP U2b.4.16 shape A (`PickerFieldConfig.list`); abi_bench option (a) (regrouping
   the C node/look/shadow/font records for fewer checks, about 190 instructions a Lists frame) left undone and recorded below for
   the 2.0 record review; the static Overlays margin of about 16 instructions is accepted as code placement and must not drop further
   (20 Nov: it went over at ee031ac2, 1.0110-1.0112, and is back under at 517d61db, 21-28 under; carried by CP1 action (f)).
4. U5.3's drop list shrank. What is left: `pxui_scope`, `pxui_ui_animate_keys_repeat`, `pxui_ui_hovered`,
   `pxui_ui_variant`, `pxui_ui_text`, the `pxui_frame`/`pxui_ui_frame` pair (U5.2), the prefix renames of `pxui_panel`, `pxui_within`,
   `pxui_part` and `pxui_slot`, the keep list in `tools/dx/abi_drop.py` and the record cleanup below.

### CP1 actions (from the U2b review)

- (a) DONE (CP1a, pxui [CP1.a]): the Overlays page's four buttons are one `button::<&str>` call site in pages.rs (shared by both sides), so the inliner's choice for a further call no longer moves the count; static Overlays 1.01103-1.01121 -> 1.00997-1.01002 (with (f)). Make the abi_bench pages' call shapes deterministic (static Overlays sits about 27 instructions under 1.011, and whether the
  gallery crate inlines a sixth `button::<&str>` call swings it by about 180). No limit change.
- (f) Final U5b release measurements (22 Nov, pxui d4ba6ebc): Static Overlays instruction ratio 1.01102 and dynamic Lists 1.03079–1.03081, both below the accepted U5b figures. Lists paired wall ratio 1.0643–1.0680 with 1.37 µs added, in line with the recurring U4b wall fluctuation and prior U5b 1.0671–1.0698 (1.33–1.42 µs). All other page gates pass, with zero warm allocations throughout. See [the deferred list](docs/u-plan/deferred.md).
- (f) U5b minor misses (21 Nov, limits unchanged, deferred here): static Overlays 1.01145-1.01154 (limit 1.011; U5a 1.0111-1.0112) and dynamic Lists 1.03089 (limit 1.03; U5a 1.0308); dynamic Layout 1.04459 and Controls 1.02948 are under their limits (about +250 and +210 instructions over U5a). Logs scratch/u5b/abi-*.log. Any real growth beyond these figures blocks the landing.
- (f) DONE in part (CP1a, pxui 6bc2b6ac, ABI 1.132): option (b) `Overlay.look: Option<&Look>` and `PxuiPanel.look` by pointer (record 200 -> 56 bytes): dynamic panel 357 -> 249 instructions (probe, wrapper minus direct), release abi_bench instruction ratios, 3 runs each (`scratch/cp1a/abi-{static,dyn}-*.log`), before -> after: static Overlays 1.01103-1.01121 -> 1.00997-1.01002 (limit 1.011), Controls 1.00878 -> 1.00823, Kit 1.01204 -> 1.01190, others unchanged; dynamic Controls 1.02949 -> 1.02886-1.02887, Overlays 1.02892 -> 1.02767-1.02785, Kit 1.02275 -> 1.02251, Lists 1.02995 -> 1.02998-1.02999 (8 under 1.03; it has no panel, so (b) does not help it), Layout 1.04289, Collections 1.01710. Direct counts also moved by placement (Overlays +58, Controls +132), so ratios move more than the work. NOT DONE: the trimmed frame entry. lldb instruction trace of one warm frame (`frame_body`, static and dynamic): 275 instructions before the engine's `frame` (checks of the descriptor 45, environment/keyboard byte compares 23, the events clear with drop glue 28, decode 47, fold 19, system preferences 12, run_frame setup 30, prologue/epilogue 21) and 70 after (finish_frame, FrameOut, restore); every one is a validation or bookkeeping step, none is repeated work, so no cut was made; remaining dynamic Lists margin is 8 instructions and needs either the `node_ok` regroup (2.0 record review) or a ruling. Earlier text: Wrapper headroom (one dedicated CP1 step; limits unchanged). Owner ruling 2026-11-20: a minor benchmark miss (noise level,
  no growth in work) is deferred here with its figures; only big offenders block. No-drift, zero warm allocations and real
  performance stay hard rules. Parts: option (b) `Overlay.look` by reference (`Option<&Look>`) and the panel record's look by
  pointer (about 25 dynamic instructions a panel); a trimmed frame entry (about 370 instructions a frame of trust-boundary and
  bookkeeping work, see the 17th/18th handoff notes); action (a) above, deterministic bench call shapes. Deferred misses (U4a
  package end, LocalWave 13535f2 / pxui 517d61db; logs `scratch/u4a-rr-abi-*.log`, `scratch/u4a-pe-buildtime.log`):
  - Dynamic abi_bench wall flaps (gate 6% or the per-node gate): run 1 Overlays x1.0647 (20.45 us/1000 nodes, start load
    2.47/3.82), run 2 Controls x1.0749 (18.08 us/1000 nodes, start 2.27/2.74), run 3 passes (Controls 1.0354, Overlays 1.0499).
    Instruction counts unchanged; at U2b.4.17 the pages read Controls 1.054-1.056, Overlays 1.045-1.047. Not bisected.
  - Edge dynamic instruction margins against 1.03: Controls 1.02952-1.02953 (140-142 instructions under), Overlays
    1.02886-1.02902 (65-76 under), Lists 1.02992-1.02994 (18-24 under). Static Overlays 1.01058-1.01068 against 1.011 (21-28 under).
  - U4b U2b.4.30 (accepted minor, orchestrator 20 Nov): dynamic abi_bench Controls direct instructions 294664 -> 294906 (+242,
    +0.08%; one token check of a built-in button's text colour, the icon button's three key colours), wrapper ratio 1.02951 ->
    1.02948; Kit -1597, Lists -430, others flat; no allocations (base d89af298, after adebdbd0;
    `scratch/u4b-430-abi-dyn-{before2,after2}.log`, both rc 101 on the known wall flaps).
  - buildtime `--compare 16dc635` c (clean pxui_dylib): 24.32 s vs 23.15 s, +1.17 s against a 1.16 s allowance (5.05%); another
    agent's av-drift and a video encoder ran during it (load 2.2 at start, 10.0 at end). Every other measure passes.
  - U4b package end (LocalWave 0be2cb9 / pxui f09c9113; logs `scratch/u4b-final-abi-{static,dynamic,dynamic2}.log`,
    `scratch/u4b-final-buildtime{,2}.log`): dynamic abi_bench wall flap Lists x1.0671 / x1.0646 (15.5 / 15.0 us per 1000 nodes) in
    the quiet run (x1.0242 in the earlier one); every instruction gate passes in both modes; dynamic Lists margin 10-16 instructions
    under 1.03 (U4a 18-24), static Overlays 27-33 under 1.011. buildtime: a run under outside load read a +1.83 s / c +1.31 s
    (over); the quiet rerun passes every measure (a +1.09 s, c +0.37 s). No limit changed.
  CP1 post-fix run (pxui 8af33e34): static Overlays 1.01103-1.01121 vs 1.011 (+8 to +14 instructions; gallery-only change).
  U5a (orchestrator ruling 20 Nov, owner rule: minor): after U5.2's Ui-serial handle check (mandated validation, ~4 wrapper instructions a box call; duplicate checks removed in pxui 9453102b, ABI 1.137) dynamic Lists reads 1.03032 vs 1.03 (~100 over) and static Overlays 1.01105 vs 1.011 (3 over). Rejected: moving the check into the engine (+~1600 a frame on both sides; moves the ratio, not the work). Open: a node-head presence byte for the Opt tags, or the 2.0 record review.
  U5a end (orchestrator ruling 21 Nov, owner rule: minor; mandated trust-boundary validation): `within` restored a record check (index < frame node count, key equal -> PXUI_ERR_STALE) after 1.137's serial-only form let a dead same-Ui handle index out of range; at pxui 98e43f99 (ABI 1.145) dynamic Lists 1.0307 vs 1.03 and static Overlays 1.0116 vs 1.011; no-check build: Overlays 762, Lists 9239 wrapper-minus-direct vs 764-775 / 9415 with it. Every other page under its limit.
- (b) Measure the per-row `drop_target`/drop-look cost in the U4a.5 queue workload; the gallery Lists page does not exercise it.
  Done at U4a.6 (release COUNTING bundle, 76e50e5 / pxui 24f2965f, load 3.3-3.9): full-queue-playlist-editor build CPU
  p50/p99 812/916 µs with reorder on vs 846/997 with it off (a temporary patch); the per-row reorder cost is below run-to-run
  noise (under ~2 µs a row, ~15 rows in view). Accepted by the orchestrator.
- (c) DONE (CP1a, pxui fb89883e): `abi_bench_start` (29 s dynamic, 14 s static) is its own test target `abi_bench_start`; dynamic abi_bench now runs the pages and the rest in about 3 s and start alone in 29 s (a contention retry repeats only start). Split the dynamic abi_bench target (start/Author vs the pages): the run takes about 59 s of the runner's 60 s cap. Never raise the cap.
- (d) RESOLVED (owner, 2026-11-21): `PxuiUi.theme` (the handle's public first field) is frozen ABI as one read-only field at offset 0; its lifetime is documented in include/pxui.h and docs/abi.md (U5.11).
- (e) DONE as evidence (CP1a, see RESULT below; no code change): U4a.6 fresh-row gap (owner, 19 Nov: accepted for now; the CP1 performance step owns it, together with option (i), the
  per-row list revision in Found). Two cuts landed: the list row box holds the row view (pxui 0c74f978) and `pxui_ui_style` takes
  one look (pxui f76bdac3, ABI 1.108). Thread instructions per fresh song row in the list's build loop (scroll jump, lite probe):
  050d029 29.5k; 44c3b45 35.8k; after the cuts 33.1-33.5k (+3.6-4.0k). Of that: the row node 1.3k, `row_in` (signal, selection,
  style, role) 0.8k, the registered row body round trip (now ~1.5k with one look), loop glue 0.3k, app content +0.6k (the
  actions `icon_button`). Timed round, 3 rounds at load 2.2-3.1 (`scratch/u4a6/timed-cut2.log`, base LW-base-i vs LW-cut2-i,
  mean of 100 frames): scroll jump instructions mean 1.21M -> 1.16M (-5%), p99 2.95-3.00M -> 3.07-3.13M (+4%), build CPU p99
  2135-2245 -> 2331-2398 us (still over base's spread); list-dialogs mean 1.81-1.83M -> 1.80-1.82M (-0.3%), build CPU p99
  2152-2159 -> 2049-2139 us; every other song workload is lower in mean instructions (fling -17%, long-scroll -9%,
  reverse-scroll -9%, song-drag -14%, indicators -13%) and build CPU p99. Zero warm allocations everywhere.
  RESULT (CP1a, U0.8 4675f7d / pxui 23bb10c4 built in a scratch worktree, removed; counting release bundles A/B; temporary patches that made the engine's cpu clock read retired instructions and counted view builds by label, all reverted, nothing committed): resize is real work, not noise: the build phase retires 1107-1128k (U0.8) vs 1245-1268k (now) instructions a frame (+11-14%); intrinsic +10%, arrange +5%, paint +2.5%. Where: the Library/sidebar view is rebuilt every frame now (300 builds in 300 frames against 200 at U0.8, 227k -> 186k instructions each): `end_frame` marks its owner view dirty when a build read a box size that layout then changed (`ExpectedSize`, ui_core.rs `expect`), and the window's height changes every resize frame, so a sidebar holding `selection_list`s that read their viewport (U4a/U4b; plain buttons at U0.8) rebuilds next frame each time (about +34k a frame). The song rows are NOT the cause: a fresh row is 58k instructions now (row 58k incl. song-visual 30k) against 73k (song-copy 21k + song-visual 52k) at U0.8, the songs page 849k against 964k, transport 217k against 241k, the now panel 1.00M against 1.08M. About +100k a frame stays in shell glue outside any retained view (shell body 800k against 660k per build while its views are cheaper); not bisected further. Legitimate cut, not made (engine change, a U5-scale item): give a list's viewport read the grid's exemption (`ExpectedSize.grid`: no rebuild while the new room still holds every row) so a short sidebar list stops dirtying its view on a resize; and bisect the shell glue. Timed (load 5-7): p99 build CPU resize 2636 us now (CP1 2525/2538, U0.8 2261/2263), zero warm allocations. Earlier: resize build CPU p99 ~11% above U0.8 in an interleaved A/B (2525/2538 vs 2261/2263 us, cp1 3d0cb03); profile it here.
  RESULT (CP1b, CP1.e1, retired instructions per frame from the thread counter, temporary uncommitted clock patch; scratch worktrees removed): resize build phase 1114k (5f6c3a7) -> 1017k (U0.8 1027k); whole build p50 2853k -> 2612k (U0.8 2680k); intrinsic, arrange and paint phases also at or below U0.8. Fix: fixed `section_list`s (`selection_list`) record their viewport with the grid's exemption (`GridFit`, one column), so a resize no longer rebuilds the sidebar view while every row still fits; test `layout_section_list_keeps_fitting_rows_on_resize` proves one build while rows fit and a rebuild when they stop fitting. The 'shell glue +100k' was not a separate cost: a scan of the nine package-end commits shows U1b.4 (looks codemod: `set_look` became `style(&States::of(look))`, which validated and encoded six looks per call, +55k in the build phase) fixed at U4a.6 (one look crosses the ABI), and the remainder is the sidebar rebuild above (+97k in the build phase). Other workloads (songs, fling, scroll-jump, non-song-lists, theme-switch, clock-120, search-typing, text-typing, reverse-scroll) build instructions identical to before the change within 1%.

## Found (separate fixes later)


- U4a review (ruling, accepted for now): the compact (phone) layout lost hardware ⌘A and Esc when finding 3 deleted the app's
  selection copy, and hardware Return when R2 deleted the shell's Enter path (170d633); U4b must restore all three when the
  compact list moves onto `selection_list` (`find_done_dismisses_input_without_playing_the_selected_song` expects None at 320/390).
  Resolved in U4b.2/6 (LocalWave daaf74d on U2b.4.33): Return plays the keyboard's row at every width; Command-A and Escape mark and
  clear in Select mode only (D9: iOS has no select-all outside edit mode).
- U4b (orchestrator): pxui has no iOS accessibility bridge. VoiceOver on iPhone does not get pxui's accessibility values (for
  example the section index's letter, U2b.4.34); the macOS and AT-SPI bridges carry them. For CP1 and the plan's iOS items.
- U4b review 2 (pxui, found 20 Nov): `Ui::focus_default(&list.handle)` does nothing for a `SelectionList::fit()` list: its
  `ListResult::handle` is the fitted rows column, which is not in the hit tree, so the key press finds no focusable ancestor
  (a scrolling list's handle is its scroll box and works). No LocalWave list both fits and is a default key target; fix with a
  test when one is (ui_lists.rs `selection_rows` `out.handle`).
- U4a package end: under the scratch dark test_ui patch (gcmp dark dumps only), test failures vary from run to run; one run showed a
  warm-allocation assertion at 3 against 0 under load. Not seen in the shipping (light) suite or `make check`. Investigate at CP1:
  warm allocations are a hard rule.
- U4a package end: the dynamic abi_bench pages Controls, Overlays and Lists sit within 18-142 instructions of their 1.03 limit (and
  static Overlays 21-28 under 1.011); any wrapper cost added before CP1 (f) will flip them. Figures under CP1 action (f).
- U4a review: abi_bench dynamic Lists fails its instruction gate at 83b2761a (U2b.4.18-26; 314723 / 305543 = 1.03005 > 1.03,
  measured with the U2b.4.18 abi_bench build fix) and at c3f43871 (finding 6: 1.03103). A legitimate cut of about 450 wrapper
  instructions a Lists frame is owed before the package end. Resolved: pxui 7f68d240 and ee031ac2; dynamic Lists reads
  1.02992-1.02994 at the package end, carried to CP1 action (f).
- U4a.5: in `pxui_core`, a view building itself clears any rebuild mark made during that build (`r.dirty = r.escapes`); only the
  list's late move (`build_again` + escape) is fixed. Other `mark_view` calls made during a view's own build have the same shape.
- U4a.5: lw_bin `production_controller_restores…` flakes under parallel load (green alone).
- U4a.6: on this machine today (19 Nov, load 3-4) the HEAD songs workloads already exceed the 1.5 ms build CPU p99 gate
  (back to back at 7e6374e: long-scroll 1545-1941, fling 1398-1574, scroll-jump 2060, reverse-scroll 1726 µs; U0.8 measured
  1253/978/1312/1043). The gate is defined on the M1 reference machine and is re-checked at CP1; U4a only must not regress.
- U4a.5 ruling check: pxui cancels a reorder drag on Escape (`a_drag_in_a_kept_view_survives_scrolling_and_cancels`), but a plain held
  press is not cancelled by Escape (`pxui_core/src/ui_core.rs:3081`: only PointerCancelled and FocusLost end the capture), so while
  any press is held LocalWave's Escape (spent on `ui.captured()`, shell.rs) does nothing and the press still clicks on release.
  Ruling (U4a.6): pxui keeps macOS button semantics for a plain press; LocalWave narrows its rule to "Escape is spent only while a
  reorder drag is active". The ledger has no per-frame "a list row is lifted" query (a grip's `Signal::held` is read only when its
  row builds, and the Up Next rows build inside retained views), so this is GAP U2b.4.12 (`Ui::reordering() -> bool`, as
  `Ui::captured`); until it lands shell.rs keeps `ui.captured()`.
- U4a.6: `Ui::text_input_focused()` is true for any keyboard focus (`pxui_core/src/ui_core.rs:3172`, `focus != 0`), not only a text
  editor; with no `ListResponse::focused` an app cannot tell its focused list from typing (the U4a.6 GAP, closed by U2b.4.11).
- U4a.6 (U2b.4.13): a list's rows past its viewport (overscan, culled) rebuild every frame even when kept: a culled box's views lose
  their run (`pxui_core` `defer_culled` / `!shown` -> `valid = false`), so the next build rebuilds them; HEAD's song rows did the same.
  Keeping a culled view's build until it shows would save about 8 row builds a warm scroll frame. Kept rows of a reorderable list
  rebuild every frame (the row's `Part::HANDLE` registers each frame in `rows_move_row`).
- U4a.6 fresh-row cost (ruling: option (ii), keep `song-visual`, no per-row revision API now). Option (i) is a candidate for the
  2.0 API review: a per-row content revision in the list (`row_revision: Option<&dyn Fn(usize) -> u64>` beside `row_key`, or per-row
  revisions in `ListState`) mixed into each kept row's key, so the app could drop `player.current_id` from the list revision and drop
  its inner `song-visual` view (`scratch/u4a6/row-merge-wip.diff` shows what that needs). Attribution (release counting bundle, scroll
  jump, thread instructions per fresh row in the list's build loop, 260 fresh rows over 100 frames, `scratch/u4a6/probe/`): 050d029
  29.5k, 44c3b45 35.8k (+6.3k). The app's row content is 30.0k (+0.6k: the actions `icon_button` replaces the pressable and
  `song-copy`); the list's row machinery is the rest: row view (its box node, record, checks) 1.6k, the row node 1.3k, `row_in`
  (signal, selection, style, role) 0.8k, the registered body round trip (trampoline, `row_body`, `set_node_layout`, `style`,
  `slot`) 2.2k, loop glue 0.3k. Jump frames also lay out more: arrange +25-34k, intrinsic +9-13k, finish +5k a frame. Two engine
  cuts suggested by fine probes (a decoded six-look memo for widget calls, resolving only the picked icon-button look) measured a
  ~180-instruction gain a fresh row and +6-11k build instructions a frame on every workload, so they were dropped.
- U4a.6: a view with its own table entry stores its instances in view-local coordinates, so the fingerprint's geom/full hashes
  change for content moved into one (the song heading, kept rows) without a pixel change; gcmp cannot tell the two apart.
- U4a.6: the name `Ui::text_input_focused` means "any keyboard focus", which is misleading (a focused list or button counts); its
  semantics stay (pxui_core tests rely on control focus taking keys from the host). For the 2.0 API review (U5/U6).
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

- 2.0 record review (orchestrator ruling, 19 Nov; abi_bench option (a)): `node_ok` checks a C node record's flag and tag bytes
  scattered across the record (about 810 instructions a dynamic Lists frame); regrouping them into about seven blocks would save
  about 190, but needs the Look, Shadow, Font and Node records reordered. Left for the 2.0 record review (U5).
- U2b.4.15 fix: a frame that settles builds again through `begin_frame`, which also resets `out.background`; a theme change whose
  frame settles loses the window-ground update for that frame (`pxui_core/src/ui_core.rs` `frame`). Copy and paste requests are now
  kept across the settle; the background is not.
- U4a.9: the picker's first frame, and any frame whose results change height, settles once (its measured list was guessed or laid
  out at another height): one more build of the whole frame. LocalWave's palette list did the same before (it was a measured
  section list); a kit picker did not.
- U4a.9: a keyboard reveal made during a build (the picker's wrap from the last row to the first) scrolls at layout, so the revealed
  row is built the frame after; the frame shows the old window once.
- U4a.9: a node's own text is placed centred in its box and ignores the node's padding; `set_node_layout` takes no text inset, so a
  body lays out a behaviour-made text node (`PickerConfig.head`) with margins. API review item for U6 (a body cannot use the text's
  line box or inset).

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

### Owner-approved U5.4 structural gate correction (2026-11-22)

The owner explicitly selected “Keep the 132-byte layout; approve correcting the size gate”. The body benchmark now pins the complete 132-byte 2.0 record, including its size prefix and 64 two-byte slots. The earlier 80-byte whole-record assertion could not accommodate even the 40 named fields plus the required prefix (84 bytes). Runtime instruction, wall-time and allocation limits are unchanged. No API, body id range, reserved slot or baseline layout was removed. pxui d4ba6ebc applies the correction. Tier A dependency and diff checks pass; release abi_bench_bodies passes all six pages with draw parity, unchanged runtime instruction limits and zero warm allocations. Full release qualification is running in both link modes; logs: scratch/u5b/approved-body-layout-abi-{static,dynamic}.log.
