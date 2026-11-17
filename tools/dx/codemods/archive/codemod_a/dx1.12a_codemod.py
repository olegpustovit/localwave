#!/usr/bin/env python3
"""DX1.12A mechanical codemod (pxui + LocalWave). Usage: dx1.12a_codemod.py <root> [errors.txt]

Rules (text-level, idempotent):
  1. Label alignment: `.align_center()` / `.align_end()` / `.align_start()` / `.center()` / `.end()` on a Label chain
     -> `.align(<path>::TextAlign::Center|End|Start)` (only on lines that build a Label: `ui.label(`, `Label`, `TEXT_`/`MONO_` consts).
  2. Segmented height: `Segmented::DEFAULT.h(` -> `Segmented::DEFAULT.h_px(`.
  3. Label pixel sizes: on Label lines, `.w(Size::Px(x))` -> `.w_px(x)`, `.w(Size::Grow)` -> `.grow()`.
     (A Node built on the same line keeps `w(Size::Grow)`: Node::grow takes a share; the compiler flags those, fix by hand.)
  4. Compiler-driven (pass the `cargo check --message-format short` output as errors.txt): every
     "no method named `h` found for struct `pxui::Label`" at file:line:col becomes `h_px`, every `end` becomes
     `align(pxui::text::TextAlign::End)`.
TextStyle -> Label/resolve call sites (UI-thread measure, worker styles) were done by hand: they need a Ui in scope and a
decision about where the resolved style lives (TableStyles / BodyStyles in LocalWave).
"""
import collections, glob, re, sys

root = sys.argv[1]
errors = sys.argv[2] if len(sys.argv) > 2 else None
LABEL_LINE = re.compile(r"ui\.label\(|\bLabel\b|\b(TEXT|MONO)_[0-9]+\b")
ALIGN = {"align_center": "Center", "align_end": "End", "align_start": "Start", "center": "Center", "end": "End"}

def text_align_path(path):
    return "crate::text::TextAlign" if "/crates/public/pxui/" in path else "pxui::text::TextAlign"

changed = 0
for path in glob.glob(f"{root}/**/*.rs", recursive=True) + glob.glob(f"{root}/**/*.md", recursive=True):
    if "/target" in path:
        continue
    src = open(path).read()
    lines = src.split("\n")
    for i, line in enumerate(lines):
        new = line.replace("Segmented::DEFAULT.h(", "Segmented::DEFAULT.h_px(")
        if LABEL_LINE.search(new) and "Node" not in new and " fn " not in new:
            for old, variant in ALIGN.items():
                new = re.sub(rf"\.{old}\(\)", f".align({text_align_path(path)}::{variant})", new) if old.startswith("align_") or "Label" in new else new
            new = re.sub(r"\.w\(Size::Grow\)", ".grow()", new)
            new = re.sub(r"\.w\(Size::Px\(([^()]*)\)\)", r".w_px(\1)", new)
        if new != line:
            lines[i] = new
            changed += 1
    out = "\n".join(lines)
    if out != src:
        open(path, "w").write(out)

if errors:
    fix = collections.defaultdict(set)
    for e in open(errors).read().splitlines():
        m = re.match(r"(\S+):(\d+):(\d+): error.*no method named `(h|end)` found for struct `pxui::Label`", e)
        if m:
            fix[m.group(1)].add((int(m.group(2)), int(m.group(3)), m.group(4)))
    for path, locs in fix.items():
        lines = open(path).read().split("\n")
        for ln, col, name in sorted(locs, key=lambda x: (x[0], -x[1])):
            l, i = lines[ln - 1], col - 1
            repl = "h_px" if name == "h" else "align(pxui::text::TextAlign::End)"
            cut = len(name) if name == "h" else len("end()")
            lines[ln - 1] = l[:i] + repl + l[i + cut:]
            changed += 1
        open(path, "w").write("\n".join(lines))
print(f"{changed} edits")
