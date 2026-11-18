#!/usr/bin/env python3
"""U2a.4: picker fills push one `PickRow` (code and rustdoc examples, pxui and LocalWave).

    pickrows.py [--dry-run] FILE...

`rows.push(label, detail)` / `out.push(label, detail)` (a receiver named `rows` or `out`, two arguments) ->
`rows.push(P::new(label).detail(detail))`, the `.detail` left out for a `""` detail. P is `crate::PickRow` inside crates/public/pxui/src,
`pxui_types::ui::PickRow` in pxui_types and the engine/abi crates, `pxui::PickRow` elsewhere (and in rustdoc examples). Other `.push(`
calls are not touched; a two-argument push on another receiver is printed as skipped. Prints per-file counts.
"""
import argparse
import re
from pathlib import Path

from rs import args, close, mask

PUSH = re.compile(r"\b(\w+)\.push\(")


def path_for(file, doc):
    p = file.as_posix()
    if doc:
        return "pxui_types::ui::PickRow" if "/pxui_types/" in p else "pxui::PickRow"
    if "crates/public/pxui/src/" in p:
        return "crate::PickRow"
    if "/pxui_types/" in p or "crates/private/" in p:
        return "pxui_types::ui::PickRow"
    return "pxui::PickRow"


def rewrite(text, row, skipped, where):
    m = mask(text)
    edits = []
    for match in PUSH.finditer(m):
        open_ = match.end() - 1
        end = close(m, open_)
        parts = args(m, open_, end)
        if len(parts) != 2:
            continue
        if match[1] not in ("rows", "out"):
            skipped.append(f"{where}: {text[match.start():end + 1]}")
            continue
        label, detail = (text[a:b].strip() for a, b in parts)
        call = f"{row}::new({label})" + ("" if detail == '""' else f".detail({detail})")
        edits.append((open_ + 1, end, call))
    for start, stop, new in reversed(edits):
        text = text[:start] + new + text[stop:]
    return text, len(edits)


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--dry-run", action="store_true")
    cli.add_argument("files", nargs="+", type=Path)
    o = cli.parse_args()
    total, skipped = 0, []
    for file in o.files:
        source = file.read_text()
        out, count = [], 0
        code, n = rewrite(source, path_for(file, False), skipped, file)
        count += n
        for line in code.split("\n"):
            doc = re.match(r"(\s*//[/!] ?)(.*)$", line)
            if doc and ".push(" in doc[2]:
                new, n = rewrite(doc[2], path_for(file, True), skipped, file)
                line, count = doc[1] + new, count + n
            out.append(line)
        if count:
            print(f"{file}: {count}")
            total += count
            if not o.dry_run:
                file.write_text("\n".join(out))
    for site in skipped:
        print("skipped", site)
    print(f"total {total}" + (" (dry run)" if o.dry_run else ""))


if __name__ == "__main__":
    main()
