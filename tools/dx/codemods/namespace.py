#!/usr/bin/env python3
"""Apply the U1a Rust namespace changes to the supplied source files."""
import argparse
import re
from pathlib import Path

from rs import close, line_of, mask


SHIMS = {
    "ui_core": "ui",
    "ui_text": "ui",
    "ui_widgets": "ui",
}
PRIVATE = {
    "text": {"pragmasevka", "Text", "TextStyle", "UiText", "TextWorker", "PreparedText", "RetiredText", "TextAlign", "Ellipsis", "Wrap"},
    "paint": {"Paint", "Fill", "Stroke"},
    "widgets": {"Scrub"},
    "collections": {"TreeNode"},
    "image": {"ImageStyle"},
    "kit": {"Repeat", "Keyframe", "Keyframes", "Target"},
    "composite": set(),
    "overlays": set(),
}
A11Y = {"A11yAction", "A11yNode", "A11yTree", "Role"}
PATH = re.compile(r"\b(pxui|crate)::(\w+)::(\w+|\{[\w\s,]+\}|\*)")


def modules(source, path):
    in_wrapper = "crates/public/pxui/src/" in path.as_posix()
    # Include explicit paths in rustdoc examples as well as in code.
    visible = list(mask(source))
    for doc in re.finditer(r"^\s*//!.*$|^\s*///.*$", source, re.M):
        visible[doc.start():doc.end()] = doc[0]
    edits, skipped = [], []
    for match in PATH.finditer("".join(visible)):
        crate, module, item = match.groups()
        if crate == "crate" and not in_wrapper:
            continue
        if module == "a11y" and item in A11Y:
            replacement = f"{crate}::{item}"
        elif module == "a11y" and (item == "state" or item.strip("{}").replace(" ", "") == "state,Role"):
            replacement = f"{crate}::ui::{item}"
        elif module in SHIMS and item != "key":
            replacement = f"{crate}::{SHIMS[module]}::{item}"
        elif module in PRIVATE and item in PRIVATE[module]:
            destination = f"{crate}::ui" if item in {"Keyframe", "Target"} else crate
            replacement = f"{destination}::{item}"
        elif module == "a11y" or module in SHIMS or (crate == "pxui" and module in PRIVATE):
            skipped.append(f"{path}:{line_of(source, match.start())}: {match[0]}")
            continue
        else:
            continue
        edits.append((match.start(), match.end(), replacement))
    return edits, skipped


def themes(source, path):
    wrapper = "crate" if "crates/public/pxui/src/" in path.as_posix() else "pxui"
    visible = mask(source)
    edits, skipped = [], []
    # A source record with default flags and a single borrowed string.
    old_source = re.compile(r"\.set_theme(?:_source)?\(\s*&\s*(?:\w+::)*ThemeSource\s*\{")
    for match in old_source.finditer(visible):
        end = close(visible, visible.index("(", match.start()))
        record = visible[match.end():end]
        text = re.search(r"\bptr:\s*([\w:]+)\.as_ptr\(\)\s*,\s*len:\s*\1\.len\(\)", record)
        flags = re.search(r"\bflags:\s*([^,}]+)", record)
        if text and (not flags or flags[1].strip() == "0"):
            edits.append((match.start(), end + 1, f".set_theme(&{wrapper}::ThemeHandle::parse(&{text[1]}).unwrap())"))
        else:
            skipped.append(f"{path}:{line_of(source, match.start())}: theme source or flags")

    # Parse a scoped theme whose only mutation is applying a source.
    old_scope = re.compile(r"\{\s*let mut theme\s*=\s*(?:crate::|pxui::)?Theme::default\(\);\s*theme\.apply\(&ui,\s*")
    for match in old_scope.finditer(visible):
        end = close(visible, match.start())
        apply_end = close(visible, visible.rindex("(", match.start(), match.end()))
        suffix = source[apply_end + 1:end]
        if re.fullmatch(r"\.(?:expect\(.*?\)|unwrap\(\));\s*theme\s*", suffix, re.S):
            text = source[match.end():apply_end]
            edits.append((match.start(), end + 1, f"{wrapper}::ThemeHandle::parse({text}){suffix[:suffix.rindex(';')]}"))
        else:
            skipped.append(f"{path}:{line_of(source, match.start())}: scoped theme")
    return sorted(edits), skipped


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--themes", action="store_true")
    parser.add_argument("files", nargs="+", type=Path)
    options = parser.parse_args()
    total = 0
    for path in options.files:
        source = path.read_text()
        rewrite = themes if options.themes else modules
        edits, skipped = rewrite(source, path)
        for site in skipped:
            print("skipped", site)
        if edits:
            print(f"{path}: {len(edits)}")
            total += len(edits)
            if not options.dry_run:
                for start, end, replacement in reversed(edits):
                    source = source[:start] + replacement + source[end:]
                path.write_text(source)
    print(f"total {total}" + (" (dry run)" if options.dry_run else ""))


if __name__ == "__main__":
    main()
