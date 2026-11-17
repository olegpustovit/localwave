You implement one pxui widget-gap row. Read first, in /private/tmp/claude-501/-Users-snowbear-WORK-GIT-localwave/70341840-4a85-459b-b067-76591dd048ed/scratchpad/dx_impl/:
brief_common.md (FAST MODE targeted tests, REVIEW GATE, OS, CODEMODS, never git stash, gigacomp via remote.sh + remote_gc.sh when done),
brief_abi.md, api_spec.md, os_criteria.md, and widget_reuse_audit.md (the gap rows named in your task + the LocalWave rows citing them = the real needs).
Rules: extend the EXISTING pxui widget (or add one widget where the gap says none exists) in the spec's shape: Copy style with DEFAULT + const fn
builders, key first, Response/result struct, label (text, style), drawn by the default body and handed to registered bodies via the body config
(append-only C records, fuzz list updated, PXUI_FEATURE_* id + manifests, next provisional ABI minor — the orchestrator renumbers at landing).
Default rendering unchanged (gallery goldens); a custom-body test per widget; a warm-frame zero-allocation test for each new path; a11y roles/states.
No twins, no compat shims, no LocalWave changes. Other lanes edit pxui concurrently — touch only your widget files + the shared registries
(append at the end), rebase onto pxui origin/rust-port at the end.
Worktree: `git -C /Users/snowbear/WORK/GIT/localwave/pxui worktree add /Users/snowbear/WORK/GIT/dx-wt/<lane> -b <lane> origin/rust-port`,
CARGO_TARGET_DIR=/Users/snowbear/WORK/GIT/dx-wt/target-<lane>; iterate on gigacomp via remote.sh; Mac only for the final targeted run (CARGO_BUILD_JOBS=4).
Then the independent review (brief_review.md) to PASS. One commit "[<ROW>] …", don't push; report hash, new public shapes, review PASS.
Be fast: no exploration beyond what the row needs.
