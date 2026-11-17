# CHECK — handoff for the pxui Unification Plan

Read this first. It says where the work stands and what to do next. Updated 2026-11-17.

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

### U0: landed (rust-port, pushed)

| Step | LocalWave | pxui |
|---|---|---|
| U0.1 tools/dx, `lw_ui::testing::frame` fingerprint helper (`LW_FINGERPRINT=<path>`) | 4cc530f | |
| U0.2 `surface.py`, api_conformance R1/R6/R7/R8 (allowlist blessed at 440 lines, shrink-only) | 3310f2b | |
| U0.3 `buildtime.sh`, ledger `docs/api.md`, `tools/dx/ledger_delta.txt` (1349 lines, 0 untagged) | 3bd0004, 7073f41 | cefefe57, a64d00ca |
| U0.4 branch cleanup | b444b1d | |
| U0.5 `tools/t --all`; prewarm test made deterministic | 8b13a62 | eb93fe54 |
| U0.6 status/process docs | 63972bf | 3db577ab |
| U0.7 CP0 recorded (**red**) | 16dc635 | |
| Status commit | 8ac43fd | |

### CP0 results and decision D26

The plan's §4 table already records D26.

1. **abi_bench.** Release abi_bench instruction ratios fail in both modes (Kit/Controls/Overlays static; Kit/Controls/Overlays/Layout dynamic). The failure is deterministic. It comes from pxui cf953b35 (DX1.12E) and b6958651 (DX1.12F): out-of-line key-first shims. **This is folded into U1c.1**, whose done-when is now: release abi_bench passes all pages in both modes.
2. **Build CPU p99 > 1.5 ms** on resize, theme-switch, non-song-lists, reverse-scroll and scroll-jump. Warm allocations stay zero everywhere. **This is U0.8, which is in progress (below).**
3. **Popup layer** draws 2 calls on the Layout page (baseline 1). This goes to U2b.8 and must be fixed before U2b.10.
4. **`Ui::id() -> usize`** becomes `instance_id()` in U1b.6, so `ui.id(key, f)` names one thing. It is already in the ledger.

## NEXT: finish U0.8 (build-CPU gate), then start U1a

### U0.8 state

The previous agent was stopped mid-work.

- **pxui:** the checkout is on local branch `u0.8` (= rust-port a64d00ca), with **uncommitted, unverified WIP** in `crates/private/pxui_core/src/ui_core.rs`. That WIP adds a memoised `curve_at` so the tweens one change starts share one curve bisection, and stops copying the whole tween per frame in `step_tweens`. The same diff is saved as `docs/u-plan/u08-curve-memo-wip.diff`. Keep it only if it measurably helps and the tests pass; otherwise run `git -C pxui checkout -- . && git -C pxui checkout rust-port`.
- **LocalWave:** on `rust-port`, with no U0.8 commits yet.

#### Paired runs

These are in `docs/u-plan/u08-paired-runs.txt`. Values are build CPU in µs, as p50/p99, at load < 4. "head" is HEAD fb16eaa-era code; "base" is de53440, before the DX API work.

| workload | head p50/p99 | base p50/p99 |
|---|---|---|
| resize | ~1505 / ~1770 | ~1317 / ~1464 |
| theme-switch | ~1508 / ~2050 | ~1364 / ~1775 |
| non-song-lists | ~660 / ~1815 | ~513 / ~1494 |
| reverse-scroll | ~1028 / ~1276 | ~911 / ~1092 |
| scroll-jump | ~530 / ~1608 | ~463 / ~1562 |

- HEAD is **12–15% slower** at p50: a real regression.
- **theme-switch (and scroll-jump) already exceeded 1.5 ms p99 at de53440**, so fixing the regression alone may not turn the gate green.
- The bisect over LocalWave commits de53440..HEAD was not finished. `run.sh` and `cmp.sh` in `docs/u-plan/` do alternating paired runs of saved bundles, waiting for load < 4.
  - Bundles built so far are in `/private/tmp/claude-501/-Users-snowbear-WORK-GIT-localwave/439fa08c-eb2b-4fe2-904d-20cdf233f9c0/scratchpad/u08/` (`head.app`, `base.app`, `b_<commit>.app`). That is a temp dir, so rebuild them if it's gone.
  - The `sample` profiles there (`head_resize.sample`, `base_resize.sample`) show where resize spends time.

#### Do

1. Bisect de53440..HEAD on resize/theme-switch p50.
   - Build each candidate with `make bundle PROFILE=release COUNTING=1` in a temp worktree (`git worktree add`, `git -c protocol.file.allow=always submodule update --init pxui`, symlink `target/sdl`), copy the `.app` aside, then remove the worktree.
2. Name the commit(s) and the hot path. Fix the root cause internally, with **no public pxui signature changes** (U1 owns signatures). If the only fix needs one, stop and add it to the ledger as a U1 step.
3. Get all gated workloads to build CPU p99 < 1.5 ms and frame CPU p99 < 2.5 ms with zero warm allocations.
   - Never loosen budgets, extend warm-up, drop input or change workloads.
   - If theme-switch can't get under budget without a signature change, record the numbers and the cause in `docs/rust-port.md` and ask the owner.
4. Commit on `u0.8` in both repos with subjects `[U0.8] …`. Include before/after numbers and load averages.
5. Run Tier B, then a full `make performance` (it must exit 0), then land per the lifecycle above.

#### Then U1a

Read plan §10 "U1a" (namespace, lifecycle, errors, input; it lands dx1.12i). Brief from §12. It has a shape stop.

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
- Don't run builds while measuring. Use one implementer at a time, in the main checkouts, with no stash and no extra worktrees except temporary bisect/buildtime ones.

## Never

- Push from a `u<N>` branch before review.
- Amend.
- `rustfmt` pxui or pxui_types.
- Hand-edit ABI manifests (use `gate -- abi-snapshot --write`).
- Add dependencies, twins, shims, aliases, compat layers, or public API outside the ledger.
- Weaken a test, deadline, budget or limit.
