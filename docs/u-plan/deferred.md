# pxui 2.0 deferred performance work

The 22 Nov 2026 release decision prioritizes a tested, stable API for downstream apps. Timing follow-ups can proceed after the API freeze. Existing limits, workloads, validation and zero-warm-allocation checks still apply. Report a reproducible regression separately from timing variation.

| Follow-up | Release evidence and next check |
| --- | --- |
| Static Overlays instruction margin | U5b 1.01102 against 1.011, below the previously accepted 1.01136–1.01154. Find a reduction in wrapper work and rerun both link modes. |
| Dynamic Lists instruction margin | U5b 1.03079–1.03081 against 1.03, below the previously accepted 1.03088–1.03089. Measure the wrapper work in isolation and verify any reduction with unchanged page inputs. |
| Dynamic Lists wall timing | U5b paired ratio 1.0643–1.0680 against 1.06, with 1.37 µs added. U4b ranged from 1.0242 to 1.0671. Use interleaved baseline/candidate runs on the M1 to distinguish a reproducible regression from variation. |
| Final U5b build-time replay | The last paired CP0 comparison passed all six allowances before the collection-label clipping fix. Repeat `tools/dx/buildtime.sh --compare 16dc635` on the final candidate during a quiet window. |

Both U5b release modes measured zero warm Rust allocations on every page. All six body pages passed after the owner-approved 132-byte structural correction. The original benchmark logs retain the failing page statuses and full measurements in `scratch/u5b/approved-body-layout-abi-{static,dynamic}.log`.
