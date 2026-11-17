#!/usr/bin/env python3
"""U0.1: route test frame drivers through lw_ui::testing::frame (the LW_FINGERPRINT hook).

Rewrites `RECV.frame(INPUT, &mut BUILD)` (two arguments, the second a `&mut` closure) in test code to
`PATH::frame(UI, INPUT, &mut BUILD)`, where UI is RECV when RECV is bound as `&mut Ui` and `&mut RECV`
otherwise. Test code = files named tests.rs or under a tests/ directory, or text after the file's first
`#[cfg(test)]`. Other `.frame(` calls are printed as skipped with the reason.

Usage: frame_helper.py [--dry-run] FILES...   (PATH is crate::testing inside lw_ui, lw_ui::testing elsewhere)
"""
import os, re, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rs import mask, close, args, line_of

CALL = re.compile(r"([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)\s*\.\s*frame\s*\(")


def by_ref(m, recv, at):
    """True when the nearest in-scope binding of `recv` before `at` is a `&mut` reference.

    Bindings: a typed parameter `recv: &mut ..` (fn or closure; in scope inside the next `{..}` block) and
    `let [mut] recv = ..` (in scope inside its enclosing block). Untyped closure params are not recognised;
    the compiler flags those sites."""
    if "." in recv:
        return False
    best, kind = -1, False
    for b in re.finditer(r"\b%s\s*:\s*&\s*mut\b" % recv, m[:at]):
        o = m.find("{", b.end())
        if b.start() > best and o < at < close(m, o):
            best, kind = b.start(), True
    for b in re.finditer(r"\blet\s+(?:mut\s+)?%s\b" % recv, m[:at]):
        if b.start() > best and at < enclosing_end(m, b.start()):
            best, kind = b.start(), bool(re.match(r"\s*=\s*&\s*mut\b", m[b.end():]))
    return kind


def enclosing_end(m, i):
    """Index of the `}` closing the innermost block open at m[i]."""
    depth = 0
    for j in range(i, -1, -1):
        if m[j] in ")]}":
            depth += 1
        elif m[j] in "([{":
            if depth == 0:
                return close(m, j)
            depth -= 1
    return len(m)


def rewrite(path, dry):
    src = open(path).read()
    m = mask(src)
    norm = path.replace(os.sep, "/")
    test_from = 0 if norm.endswith("/tests.rs") or "/tests/" in norm else (m.find("#[cfg(test)]") if "#[cfg(test)]" in m else len(m))
    target = "crate::testing::frame" if "/lw_ui/src/" in norm else "lw_ui::testing::frame"
    edits, skipped = [], []
    for c in CALL.finditer(m):
        recv, open_ = c.group(1), c.end() - 1
        end = close(m, open_)
        a = args(m, open_, end)
        where = f"{path}:{line_of(src, c.start())}"
        if len(a) != 2 or not m[a[1][0]:a[1][1]].strip().startswith("&mut"):
            skipped.append(f"{where}: {recv}.frame(..) is not (input, &mut build)")
            continue
        if c.start() < test_from:
            skipped.append(f"{where}: {recv}.frame(..) outside test code")
            continue
        ui = recv if by_ref(m, recv, c.start()) else "&mut " + recv
        inner = src[open_ + 1:end]
        edits.append((c.start(), end + 1, f"{target}({ui}, {inner.lstrip()})" if not inner.startswith("\n") else f"{target}(\n{indent(src, a[0][0])}{ui},{inner})"))
    for s, e, text in reversed(edits):
        src = src[:s] + text + src[e:]
    if edits and not dry:
        open(path, "w").write(src)
    return len(edits), skipped


def indent(src, at):
    """Leading whitespace of the first non-blank line at or after `at`."""
    rest = src[at:].lstrip("\n")
    return rest[:len(rest) - len(rest.lstrip(" "))]


if __name__ == "__main__":
    dry = "--dry-run" in sys.argv
    total = 0
    for f in [a for a in sys.argv[1:] if a != "--dry-run"]:
        n, skipped = rewrite(f, dry)
        total += n
        if n:
            print(f"{f}: {n}")
        for s in skipped:
            print("skipped", s)
    print("total", total, "(dry run)" if dry else "")
