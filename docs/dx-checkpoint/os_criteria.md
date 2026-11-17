OS-GRADE CRITERIA (owner, 2026-11-17: "that os thing is super important") — pxui will be the base of an OS. Every pxui change is
judged against these as well as api_spec.md; a violation is a blocker unless the owner accepts it explicitly.
1. Multi-surface: nothing assumes one window/surface per process. State lives per Ui/surface, input events carry or are routed by
   surface, a11y trees and focus are per surface. No hidden "the" window, screen size or focus singleton.
2. No process-global mutable state in the library or wrapper (no global tables, registries, caches or counters shared across Ui
   instances/threads) unless it is immutable, or explicitly per-process by design and documented with its bounds and thread-safety.
3. Untrusted callers: a compositor hosts many apps. Every C entry point validates every input (sizes, pointers, enums, UTF-8,
   ranges, counts) and returns an error — never UB, never a panic/abort across the ABI, never an unbounded allocation from caller
   input. Rust wrapper panics only for programmer errors that are documented.
4. Bounded resources: every cache/pool/queue has a documented cap; nothing grows with uptime (no "entries are never removed").
   Work per frame is bounded.
5. Long-lived ABI (2.0 at the freeze): size-prefixed records; no ambiguous in-band encodings without a documented, tested,
   final rule; features discoverable (`pxui_supports`), not guessed from versions; no twins/legacy carried into 2.0.
6. Policy-free core: nothing LocalWave- or app-specific in pxui (names, defaults, behaviours); apps customise through theme,
   bodies, variants and style values.
7. Threading and lifetime: which thread owns what is explicit; callbacks are call-scoped borrows; nothing keeps app pointers
   past the call unless documented.
8. Performance and build speed stay first-class: zero warm allocations, non-generic public API (shape gate), no heavy const-eval.
