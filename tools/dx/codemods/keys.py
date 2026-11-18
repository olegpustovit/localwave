#!/usr/bin/env python3
"""Migrate known U1a.4 key queries and testing::key calls."""
import argparse
from pathlib import Path
import re

from namespace import DOC_CODE
from rs import args, close, line_of, mask


SPECIAL = """ArrowUp ArrowDown ArrowLeft ArrowRight Space Enter Escape Backspace Tab Shift Ctrl Alt Meta
F1 F2 F3 F4 F5 F6 F7 F8 F9 F10 F11 F12 Home End PageUp PageDown Insert Delete""".split()
MODIFIERS = {"Meta", "Shift", "Alt", "Ctrl"}
CALL = re.compile(r"\.(key_pressed|key_down)\(|\b(?:pxui::)?testing::(key)\(")


def key_code(expr, prefix):
    if char := re.fullmatch(r"('(?:\\(?:u\{[0-9a-fA-F]+\}|.)|[^\\'])')(?:\s*as\s+u32)?", expr):
        return f"{prefix}::KeyCode::Char({char[1]})"
    special = re.fullmatch(r"(?:input::)?Key::(\w+)(?:\.code\(\))?", expr)
    if special and special[1] in SPECIAL:
        return f"{prefix}::KeyCode::{special[1]}"
    wire = re.fullmatch(r"(?:pxui_types::)?KEY_SPECIAL\s*\+\s*(\d+)", expr)
    if wire and int(wire[1]) < len(SPECIAL):
        return f"{prefix}::KeyCode::{SPECIAL[int(wire[1])]}"


def modifiers(expr, prefix):
    if expr == "None":
        return f"{prefix}::Mods::ANY"
    if some := re.fullmatch(r"Some\(\s*(.*?)\s*\)", expr, re.S):
        key = key_code(some[1], prefix)
        if key and (name := key.rsplit("::", 1)[1]) in MODIFIERS:
            return f"{prefix}::Mods::{name.upper()}"


def rewrite(source, prefix):
    visible = mask(source)
    edits, skipped = [], []
    for match in CALL.finditer(visible):
        name = match[1] or match[2]
        start = match.end() - 1
        spans = args(visible, start, close(visible, start))
        at, count = {"key": (1, 3), "key_down": (0, 1), "key_pressed": (0, 2)}[name]
        if len(spans) != count:
            continue
        lo, hi = spans[at]
        key = key_code(source[lo:hi].strip(), prefix)
        if name == "key_pressed":
            mod_lo, mod_hi = spans[1]
            mod = modifiers(source[mod_lo:mod_hi].strip(), prefix)
            if mod is None:
                key = None
            elif key:
                edits.append((mod_lo, mod_hi, " " + mod))
        if key:
            lead = " " if at else ""
            edits.append((lo, hi, lead + key))
        else:
            skipped.append((line_of(source, match.start()), source[match.start():close(visible, start) + 1].splitlines()[0]))
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
        prefix = "crate" if "crates/public/pxui/src/" in path.as_posix() else "pxui"
        edits, skipped = rewrite(source, prefix)
        count = len(edits)
        for block in DOC_CODE.finditer(source):
            code = re.sub(r"^[ \t]*//[!/] ?", "", block[2], flags=re.M)
            doc_edits, doc_skips = rewrite(code, "pxui")
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
    print(f"total {total} argument edits")


if __name__ == "__main__":
    main()
