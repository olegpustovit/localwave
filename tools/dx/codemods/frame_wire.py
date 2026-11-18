#!/usr/bin/env python3
"""Move diagnosed FrameIn field accesses to wire. Run from the cargo check workspace."""
import argparse
from collections import defaultdict
import json
from pathlib import Path
import re


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("diagnostics", type=Path, help="cargo check --tests --message-format=json output")
    cli.add_argument("--dry-run", action="store_true")
    options = cli.parse_args()
    edits = defaultdict(set)
    pattern = re.compile(r"(?:no field|attempted to take value of method) `(\w+)` on type `(?:&mut |&)?(?:pxui::)?FrameIn<'_>`")
    for line in options.diagnostics.read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line).get("message", {})
        match = pattern.search(message.get("message", ""))
        if match:
            for span in message["spans"]:
                if span["is_primary"]:
                    edits[Path(span["file_name"])].add((span["byte_start"], match[1].encode()))
    for path, fields in sorted(edits.items()):
        source = path.read_bytes()
        for at, field in sorted(fields, reverse=True):
            if source[at:at + len(field)] != field or source[at - 1:at] != b".":
                raise ValueError(f"stale diagnostic for {path} at byte {at}")
            source = source[:at] + b"wire." + source[at:]
        print(f"{path}: {len(fields)}")
        if not options.dry_run:
            path.write_bytes(source)
    print(f"total {sum(map(len, edits.values()))}")


if __name__ == "__main__":
    main()
