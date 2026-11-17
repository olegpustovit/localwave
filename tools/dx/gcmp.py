#!/usr/bin/env python3
"""Compare two LW_FINGERPRINT dumps (lw_ui::testing::frame): gcmp.py BEFORE AFTER

Prints, per test, frame-count differences and which hashes (count, geom, full, a11y) differ and from which
frame. Exits 1 when any accessibility name or action bit is lost (D9): for every frame both dumps have, each
name in BEFORE must still be present in AFTER with at least the same action bits. Hash differences alone are
reported for review (listed D9 diffs), not failed.
"""
import collections, sys


def load(path):
    frames, nodes = {}, collections.defaultdict(dict)
    for line in open(path, encoding="utf-8", errors="replace"):
        f = line.rstrip("\n").split("\t")
        if f[0] == "F":
            frames[(f[1], int(f[2]))] = tuple(f[3:7])
        elif f[0] == "A":
            names = nodes[(f[1], int(f[2]))]
            name = "\t".join(f[5:])
            names[name] = names.get(name, 0) | int(f[4], 16)
    return frames, nodes


def compare(a, b):
    (fa, na), (fb, nb) = a, b
    lost = []
    for test in sorted({k[0] for k in fa} | {k[0] for k in fb}):
        ka = sorted(n for t, n in fa if t == test)
        kb = sorted(n for t, n in fb if t == test)
        if len(ka) != len(kb):
            print(f"{test}: frames {len(ka)} vs {len(kb)}")
        diffs, first = collections.Counter(), {}
        for n in sorted(set(ka) & set(kb)):
            for i, col in enumerate(("count", "geom", "full", "a11y")):
                if fa[(test, n)][i] != fb[(test, n)][i]:
                    diffs[col] += 1
                    first.setdefault(col, n)
            after = nb.get((test, n), {})
            for name, bits in na.get((test, n), {}).items():
                if name and name not in after:
                    lost.append(f"{test} frame {n}: lost name {name!r}")
                elif bits & ~after.get(name, 0):
                    lost.append(f"{test} frame {n}: {name!r} lost action bits {bits & ~after.get(name, 0):#x}")
        if diffs:
            print(test, dict(diffs), "first", first, "of", len(ka))
    for line in lost:
        print("LOST", line)
    return 1 if lost else 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-check"]:
        base = ({("t", 0): ("1", "a", "b", "c")}, {("t", 0): {"Play": 1, "Next": 1}})
        same = ({("t", 0): ("1", "a", "b", "d")}, {("t", 0): {"Play": 3, "Next": 1}})
        lost = ({("t", 0): ("1", "a", "b", "c")}, {("t", 0): {"Play": 0}})
        assert compare(base, same) == 0 and compare(base, lost) == 1
        print("gcmp self-check ok")
        sys.exit(0)
    sys.exit(compare(load(sys.argv[1]), load(sys.argv[2])))
