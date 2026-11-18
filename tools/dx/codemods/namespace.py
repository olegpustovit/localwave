#!/usr/bin/env python3
"""Apply the U1a Rust namespace changes to the supplied source files."""
import argparse
import re
from pathlib import Path

from rs import line_of, mask


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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("files", nargs="+", type=Path)
    options = parser.parse_args()
    total = 0
    for path in options.files:
        source = path.read_text()
        edits, skipped = modules(source, path)
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
