#!/usr/bin/env python3
"""List C exports (abi/pxui.v1.txt + abi/pxui.ext.txt) and who calls them.

An export whose only references (outside pxui_abi, which defines them and
may call one export from another) are its own definition, the fuzz list, tests,
the header, the manifests and docs serves no current caller: a drop candidate.
Run from the LocalWave root: python3 tools/dx/abi_drop.py [--all | --check]
--check fails while a candidate is not in KEEP (C-only calls with no successor).
"""
import os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
PX = os.path.join(ROOT, "pxui")
SKIP_DIRS = {"target", ".git", "abi", "docs", "fixtures", "specification", "scratch", "include", "__pycache__"}
EXTS = (".rs", ".c", ".m", ".mm", ".h", ".sh", ".py", ".swift")
# Files that only define, list or document exports; never callers.
# C-only calls with no successor export: no pre-change twin, kept until U5.3 decides.
KEEP = {
    "pxui_paint_caret_x", "pxui_paint_selection", "pxui_paint_ratio", "pxui_paint_input_region",  # canvas text helpers
    "pxui_ui_input",  # the routed input record for an app's own content
    "pxui_theme_last_error",  # a theme object's error detail (theme_parse, theme_set_color)
    "pxui_ui_node_keyed",  # a node under an integer key (the C form of a resolved key); pxui_part builds through it
}
NOT_CALLERS = ("pxui_abi/src/fuzz.rs", "tools/abi-old-app.c", "tools/abi-fixture.sh", "tools/dx/abi_drop.py")


def exports():
    names = []
    for f in ("pxui.v1.txt", "pxui.ext.txt"):
        for line in open(os.path.join(PX, "abi", f)):
            if line.startswith("function "):
                names.append(line.split()[1])
    return names


def files():
    for base in (PX, os.path.join(ROOT, "crates"), os.path.join(ROOT, "tools")):
        for d, subs, fs in os.walk(base):
            subs[:] = [s for s in subs if s not in SKIP_DIRS and not (d == ROOT and s == "pxui")]
            for f in fs:
                if f.endswith(EXTS):
                    yield os.path.join(d, f)


def test_start(lines):
    """Index of the first line of a trailing `#[cfg(test)]` module, else len."""
    for i, l in enumerate(lines[:-1]):
        nxt = lines[i + 1].strip()
        if l.startswith("#[cfg(test)]") and nxt.startswith("mod ") and nxt.endswith("{"):
            return i
    return len(lines)


def main():
    names = exports()
    word = re.compile(r"\b(pxui_[a-z0-9_]+)\b")
    refs = {n: {"caller": set(), "internal": set(), "test": set()} for n in names}
    for path in files():
        rel = os.path.relpath(path, ROOT)
        if rel.endswith(NOT_CALLERS):
            continue
        lines = open(path, errors="replace").read().split("\n")
        t0 = test_start(lines) if path.endswith(".rs") else len(lines)
        is_test_file = "/tests/" in rel or rel.endswith(("_test.rs", "tests.rs"))
        for i, l in enumerate(lines):
            for n in word.findall(l):
                if n not in refs:
                    continue
                s = l.strip()
                # Definitions and wrapper declarations are not calls.
                if re.search(r"\bfn %s\s*\(" % n, l) or s.startswith(("//", "/*", "* ", "#define", "#include")):
                    continue
                # the static wrapper's trusted entry of the same name is a Rust function, not a call of the export
                if re.search(r"trusted::(\{[^}]*)?\b%s\b" % n, l) and "pxui_abi/src/" not in rel:
                    continue
                kind = "test" if is_test_file or i >= t0 else "internal" if "pxui_abi/src/" in rel else "caller"
                refs[n][kind].add(rel)
    if "--check" in sys.argv:
        bad = [n for n in names if not refs[n]["caller"] and n not in KEEP]
        for n in bad:
            print("abi_drop: %s has no caller outside pxui_abi and tests" % n)
        sys.exit(1 if bad else 0)
    show_all = "--all" in sys.argv
    for n in names:
        if refs[n]["caller"] and not show_all:
            continue
        print(("DROP? " if not refs[n]["caller"] else "keep  ") + n)
        for kind in ("caller", "internal", "test"):
            for r in sorted(refs[n][kind]):
                print("    %-8s %s" % (kind, r))


if __name__ == "__main__":
    main()
