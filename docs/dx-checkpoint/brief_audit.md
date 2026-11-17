DX1.12 audit fix batch — common brief
Read first: brief_common.md (FAST MODE + REVIEW rules), brief_abi.md, api_spec.md (incl. "How an APP uses pxui"), and your batch in
dx1.12_audit.md (all in /private/tmp/claude-501/-Users-snowbear-WORK-GIT-localwave/70341840-4a85-459b-b067-76591dd048ed/scratchpad/dx_impl/).
- Fix every finding in your batch the spec-conforming way: one function per job (delete twins; the survivor gets the more capable
  signature; callers updated in the same commit), Copy style/option values with DEFAULT/presets + const fn builders, fields pub(crate)
  where they invite literals, keys first (Key from &'static str or item id), interactive calls return Response/result struct, enums
  instead of magic u32 codes, consistent error policy per the audit. NO compat layer, NO deprecated aliases, NO second shape.
- Legacy C exports stay (append-only ABI) but are marked legacy in include/pxui.h + docs/abi.md; any new/changed C record follows
  brief_abi.md with a provisional minor = 1.6x given in your task (coordinator renumbers).
- Stay inside your batch's file set (see the batch header); if a fix needs another batch's file, make the minimal edit and say so.
- Update every caller in the pxui workspace (gallery, abi_bench, tests, doc examples you touch). LocalWave callers: make the matching
  change in a LocalWave worktree from origin/rust-port as a separate commit (same row id), `cargo t -p lw_ui` + `cargo build -p lw_bin`.
- Default rendering unchanged: run `cargo t -p pxui_gallery` once at the end. Zero warm allocations preserved.
- Iterate on gigacomp (remote.sh). Targeted checks: `cargo t -p` touched crates (pxui in both link modes), `cargo t -p pxui --doc`
  for touched modules, gates shape/abi-header/abi-snapshot/docs, `cargo check -p abi_bench -p pxui_gallery --tests` both modes.
- One pxui commit "[DX1.12<batch>] …" (+ one LocalWave commit if callers changed). Don't push. Report hashes, findings fixed (by
  number), any finding you could not fix and why, and the public items whose shape changed (for the reviewer).
- Also apply os_criteria.md (same dir): global state, multi-surface assumptions, unvalidated C inputs or unbounded growth you see in
  your files are findings — fix them in your batch or report them.
