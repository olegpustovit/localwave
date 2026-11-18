# CHECK — handoff for the pxui Unification Plan

Updated 2026-11-18. Resume at [U1c](#next-u1c).

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

## NEXT: U1c review (implementer stopped at final, BLOCKED item)

U1c is on branch `u1c` in both repos: LocalWave 291d69d, pxui 81e96ee0. U1c.1–U1c.4 are done, plus three fix commits from the
implementer's own Tier B and release runs. Tier B is green (pxui `cargo t --all` 943; offline `make check` 289; gates docs, abi-exports,
abi-frozen, abi-compat). gcmp against U1a shows no D9 diff. Build times are within the CP0 allowance.

**BLOCKED:** the release abi_bench Author page and page limits (the D26 done-when) still fail in both modes. Measured: inlining the key-first bodies is
not the main cause. The fixes so far are static trusted entries for the widgets and an unzeroed text-area buffer. The Author page compares
`id + row` with `node + within`, which is one extra library call per row and about 3.7× the 1% budget. Options and numbers:
`scratch/u1c-final-report.md`. The orchestrator decides between new scope exports and re-pairing the page, then assigns the remaining
page work (text-input response calls, dynamic validation, inspector bodies).

## Found (separate fixes later, not U1b scope)

- `pxui_core/src/ui_core.rs` `tooltip_sheet`: a shown tooltip formats and wraps its text into a `String`/`Vec` every frame.
- `pxui_core/src/ui_text.rs:1733`: the text area's doc tooltip keys its wait with a `format!`ed string every frame.
- `Segment::detail` reaches the engine in both link modes but no body config exposes it.
- `box_ops.rs` `last_press_within`: a touch Began is not matched like a mouse press.
- `lw_apple_sys` `history::tests::snapshots_preserve_other_songs_and_reject_invalid_starts`: its preferences suite is named by pid and thread and persists, so it flaked once in `make check`.

## Commands

- **Tier A (per commit)**
  - pxui: `cargo t -p <crate> [filter]`; `cargo t -p pxui --features static`; `cargo check --workspace --tests`; `cargo check -p abi_bench -p pxui_gallery --tests` (also with `--no-default-features`); `cargo run -p gate -- shape|deps|abi-header|abi-snapshot`.
  - LocalWave: `cargo t -p lw_ui [filter]`; `cargo t -p lw_ui --test api_conformance`; `cargo build -q -p lw_bin`; `cargo check -p lw_bin --tests`; `python3 tools/dx/surface.py --check`; `git diff --check`.
- **Tier B (package end)**
  - pxui: `cargo t --all`; `cargo t -p pxui --features static`; `cargo t -p pxui --doc`; `cargo t -p pxui_gallery`; gates docs, abi-exports, abi-frozen, abi-compat.
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
