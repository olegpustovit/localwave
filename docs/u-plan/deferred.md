# pxui 2.0 deferred performance work

The 22 Nov 2026 release decision prioritizes a tested, stable API for downstream apps. Timing follow-ups can proceed after the API freeze. Existing limits, workloads, validation and zero-warm-allocation checks still apply. Report a reproducible regression separately from timing variation.

| Follow-up | Release evidence and next check |
| --- | --- |
| Static Overlays instruction margin | U5b 1.01102 against 1.011, below the previously accepted 1.01136–1.01154. Find a reduction in wrapper work and rerun both link modes. |
| Dynamic Lists instruction margin | U5b 1.03079–1.03081 against 1.03, below the previously accepted 1.03088–1.03089. Measure the wrapper work in isolation and verify any reduction with unchanged page inputs. |
| Dynamic Lists wall timing | U5b paired ratio 1.0643–1.0680 against 1.06, with 1.37 µs added. U4b ranged from 1.0242 to 1.0671. Use interleaved baseline/candidate runs on the M1 to distinguish a reproducible regression from variation. |
| Final U5b build-time replay | The last paired CP0 comparison passed all six allowances before the collection-label clipping fix. Repeat `tools/dx/buildtime.sh --compare 16dc635` on the final candidate during a quiet window. |
| CP2 signed frame timing replay | The complete signed run exercised all 30 workloads and measured zero warm allocations on the 28 gated workloads. Six timing misses at start/end load 32.39/6.06 are diagnostic. Rerun `make performance` below load 4 and compare any reproducible miss with the last qualified baseline. |
| CP2 and U6 build-time comparisons | Run `tools/dx/buildtime.sh --compare de53440` for CP2 and the U6 comparison against CP0. Preserve the existing allowances and compare on the M1 during a quiet window. |
| U6 gallery Lists instruction replay | After the builder sweep, release runs measured 1.01013–1.01019 static against 1.01 and 1.03110–1.03117 dynamic against 1.03. The earlier U5b runs measured 1.00963–1.00964 static and 1.03079–1.03081 dynamic. Direct and wrapper now both retire about 1,500 more instructions per frame in this gallery workload. Compare the original and builder-based gallery authoring code against the same CP2 engine build. Warm allocations stayed zero and the page draw and instance checks passed. |
| U6 release timing replay | Static Overlays crossed its instruction limit by at most 0.00007. The dynamic startup case exceeded 2.5 ms on two unqualified runs, reported as 3.61 ms at load 23.5 and 2.50 ms rounded at load 25.1. Pair static and dynamic replays on the M1 with the same workloads and limits. The Controls default-body instruction check passed after the gallery reused its row style. |

Both U5b release modes measured zero warm Rust allocations on every page. All six body pages passed after the owner-approved 132-byte structural correction. The original benchmark logs retain the failing page statuses and full measurements in `scratch/u5b/approved-body-layout-abi-{static,dynamic}.log`.
