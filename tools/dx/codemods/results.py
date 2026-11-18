#!/usr/bin/env python3
"""Migrate U1b.1 result idioms (pxui and LocalWave, code and rustdoc examples).

- `let (r, v) = ui.radio/dropdown/slider(..);` -> `let r = ui.radio(..); let v = r.chosen;` (`.value` for a slider);
- `ui.radio/dropdown/slider(..).1` -> `.chosen` / `.value`; `.0` -> `.response`;
- `segmented_with` -> `segmented`, `SegmentedResponse` -> `PickResponse`, `PickerOpts` -> `Picker`;
- a popup or modal call read as a bool (after `if`, `!`, `=`, `+=`, `||`, `&&`, `(` or `,`) gains `.open`;
- a closure whose body is a bare panel/popup/modal call (`|ui| ui.panel(..)`) gains a block so it still returns unit.
No scope analysis: the compiler flags misses. Items given as `&[&str]` and a `usize` selection are left for hand edits and
reported, as are tuple results bound in other shapes.
"""
import argparse
from pathlib import Path
import re

from namespace import DOC_CODE
from rs import close, line_of, mask

PICK = re.compile(r"\.(radio|dropdown|slider)\(")
OVERLAY = re.compile(r"\.(popup|modal|panel|anchored)\(")
FIELD = {"radio": "chosen", "dropdown": "chosen", "slider": "value"}
RENAMES = [(re.compile(r"\bsegmented_with\b"), "segmented"), (re.compile(r"\bSegmentedResponse\b"), "PickResponse"),
           (re.compile(r"\bPickerOpts\b"), "Picker")]
BOOL_BEFORE = re.compile(r"(?:\bif|!|[^=!<>]=|\+=|\|\||&&|\(|,)\s*$")


def receiver_start(m, dot):
    """Start of the receiver expression ending at m[dot] (an identifier path, possibly called)."""
    i = dot
    while i > 0 and (m[i - 1].isalnum() or m[i - 1] in "_.:"):
        i -= 1
    return i


def rewrite(source):
    m = mask(source)
    edits, skipped = [], []
    for match in PICK.finditer(m):
        name, open_ = match[1], match.end() - 1
        end = close(m, open_)
        start = receiver_start(m, match.start())
        line = m.rfind("\n", 0, start) + 1
        head = m[line:start]
        tail = m[end + 1:end + 3]
        bind = re.search(r"let\s+(?:mut\s+)?\(\s*(\w+)\s*,\s*(mut\s+)?(\w+)\s*\)\s*=\s*$", head)
        if bind and m[end + 1:].lstrip().startswith(";"):
            semi = m.index(";", end)
            r, value = bind[1], bind[3]
            mut = "mut " if bind[2] else ""
            edits.append((line + bind.start(), start, f"let {r} = "))
            edits.append((semi + 1, semi + 1, f" let {mut}{value} = {r}.{FIELD[name]};"))
        elif tail == ".1" and not m[end + 3:end + 4].isdigit():
            edits.append((end + 1, end + 3, "." + FIELD[name]))
        elif tail == ".0" and not m[end + 3:end + 4].isdigit():
            edits.append((end + 1, end + 3, ".response"))
        else:
            skipped.append((line_of(source, match.start()), source[start:end + 1].splitlines()[0]))
    for match in OVERLAY.finditer(m):
        name, open_ = match[1], match.end() - 1
        end = close(m, open_)
        start = receiver_start(m, match.start())
        before = m[max(0, start - 40):start]
        closure = re.search(r"\|\s*\w*\s*(?::\s*&mut\s+[\w:]+\s*)?\|\s*$", before)
        if closure:
            edits.append((start, start, "{ "))
            edits.append((end + 1, end + 1, "; }"))
        elif name in ("popup", "modal") and BOOL_BEFORE.search(before) and not re.search(r"let\s+_\s*=\s*$", before) and m[end + 1:end + 2] != ".":
            edits.append((end + 1, end + 1, ".open"))
    for pattern, new in RENAMES:
        for match in pattern.finditer(m):
            edits.append((match.start(), match.end(), new))
    return sorted(edits), skipped


def apply(source, edits):
    for start, end, replacement in reversed(edits):
        source = source[:start] + replacement + source[end:]
    return source


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("files", nargs="+", type=Path)
    options = parser.parse_args()
    total = 0
    for path in options.files:
        source = path.read_text()
        edits, skipped = rewrite(source)
        count = len(edits)
        for block in DOC_CODE.finditer(source):
            code = re.sub(r"^[ \t]*//[!/] ?", "", block[2], flags=re.M)
            doc_edits, doc_skips = rewrite(code)
            skipped.extend((line_of(source, block.start(2)) + n - 1, call) for n, call in doc_skips)
            if doc_edits:
                edited = apply(code, doc_edits)
                edits.append((block.start(2), block.end(2), "".join(block[1] + line for line in edited.splitlines(keepends=True))))
                count += len(doc_edits)
        for n, call in skipped:
            print(f"skipped {path}:{n}: {call}")
        if count:
            print(f"{path}: {count}")
            total += count
            if not options.dry_run:
                path.write_text(apply(source, sorted(edits)))
    print(f"total {total} edits")


if __name__ == "__main__":
    src = ("fn f(ui: &mut Ui) { let (r, mut v) = ui.radio(\"k\", &I, s, &R); x = ui.slider(\"s\", \"S\", v, 0.0..=1.0, &S).1;\n"
           "if ui.popup(\"p\", &O, |_| {}) { } let _ = ui.popup(\"q\", &O, |_| {}); ui.modal(\"m\", &O, |_| {}); b(&mut |ui| ui.panel(\"p\", &O, &mut |_| {})); ui.segmented_with(\"s\") }")
    out = apply(src, rewrite(src)[0])
    assert "let r = ui.radio(\"k\", &I, s, &R); let mut v = r.chosen;" in out, out
    assert ").value;" in out and "|_| {}).open {" in out and "ui.modal(\"m\", &O, |_| {});" in out, out
    assert "let _ = ui.popup(\"q\", &O, |_| {});" in out, out
    assert "|ui| { ui.panel(\"p\", &O, &mut |_| {}); }" in out and "ui.segmented(" in out, out
    main()
