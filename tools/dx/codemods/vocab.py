#!/usr/bin/env python3
"""Migrate U1b.5 builders and vocabulary (pxui and LocalWave, code and rustdoc examples).

- In an `Overlay::DEFAULT` builder chain: `.w(x)` -> `.w_px(x)`, `.h(x)` -> `.h_px(x)`, `.max_h(x)` -> `.max_h_px(x)` and
  `.at(x, y)` -> `.at(Point::new(x, y))` (D21);
- `ui.progress(k, t, v)` / `ui.spinner(k, n)` / `ui.badge(k, n, t)` gain `&Progress::DEFAULT` / `&Spinner::DEFAULT` /
  `&Badge::DEFAULT`;
- `ui.grid_list(k, n, w, h, build)` -> `ui.grid_list(k, n, &GridList::DEFAULT.w_px(w).h_px(h), build)`.
Paths: `crate::` inside the pxui crate, `pxui_types::geom::Point` in the engine's tests and the gallery, `pxui::` elsewhere.
Run the widget rules on the wrapper's callers only (`--overlay-only` for the engine's tests: they call the engine's own
progress/spinner/badge/grid_list). `scroll_view(k, w, h, f)` and `text_area(k, v, rect, ..)` sites are reported for hand edits.
"""
import argparse
from pathlib import Path
import re

from namespace import DOC_CODE
from rs import args, close, line_of, mask

CHAIN = re.compile(r"\bOverlay::DEFAULT\b")
WIDGET = re.compile(r"\.(progress|spinner|badge|grid_list)\(")
REPORT = re.compile(r"\.(scroll_view|text_area)\(")
ARITY = {"progress": 3, "spinner": 2, "badge": 3, "grid_list": 5}
STYLE = {"progress": "Progress", "spinner": "Spinner", "badge": "Badge"}


def rewrite(source, prefix, point, overlay_only):
    m = mask(source)
    edits, skipped = [], []
    for match in CHAIN.finditer(m):
        i = match.end()
        while True:
            call = re.compile(r"\s*\.\s*(\w+)\s*\(").match(m, i)
            if not call:
                break
            open_ = call.end() - 1
            end = close(m, open_)
            name = call[1]
            if name in ("w", "h", "max_h"):
                edits.append((call.start(1), call.end(1), name + "_px"))
            elif name == "at":
                parts = args(m, open_, end)
                if len(parts) == 2:
                    x, y = (source[a:b].strip() for a, b in parts)
                    edits.append((open_ + 1, end, f"{point}::new({x}, {y})"))
            i = end + 1
    if not overlay_only:
        for match in WIDGET.finditer(m):
            name, open_ = match[1], match.end() - 1
            end = close(m, open_)
            parts = args(m, open_, end)
            # a build closure's `|ui, i|` splits into more parts: grid_list needs at least its five, its third not a style yet
            grid_ok = name == "grid_list" and len(parts) >= 5 and not source[parts[2][0]:parts[2][1]].strip().startswith("&")
            if not grid_ok and (name == "grid_list" or len(parts) != ARITY[name]):
                continue
            if name == "grid_list":
                (wa, wb), (ha, hb) = parts[2], parts[3]
                w, h = source[wa:wb].strip(), source[ha:hb].strip()
                lead = wa + len(source[wa:wb]) - len(source[wa:wb].lstrip())
                edits.append((lead, hb, f"&{prefix}::GridList::DEFAULT.w_px({w}).h_px({h})"))
            else:
                last = parts[-1][1]
                edits.append((last, last, f", &{prefix}::{STYLE[name]}::DEFAULT"))
    for match in REPORT.finditer(m):
        skipped.append((line_of(source, match.start()), source[match.start():source.find("\n", match.start())].strip()[:100]))
    return sorted(edits), skipped


def apply(source, edits):
    for start, end, replacement in reversed(edits):
        source = source[:start] + replacement + source[end:]
    return source


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--overlay-only", action="store_true")
    parser.add_argument("files", nargs="+", type=Path)
    options = parser.parse_args()
    total = 0
    for path in options.files:
        source = path.read_text()
        inside = "crates/public/pxui/src" in str(path)
        engine_side = "crates/private" in str(path) or "pxui_gallery" in str(path) or "abi_bench" in str(path)
        prefix = "crate" if inside else "pxui"
        point = "crate::Point" if inside else "pxui_types::geom::Point" if engine_side else "pxui::Point"
        edits, skipped = rewrite(source, prefix, point, options.overlay_only)
        count = len(edits)
        for block in DOC_CODE.finditer(source):
            code = re.sub(r"^[ \t]*//[!/] ?", "", block[2], flags=re.M)
            doc_point = "pxui_types::geom::Point" if "crates/public/pxui_types" in str(path) else "pxui::Point"
            doc_edits, doc_skips = rewrite(code, "pxui", doc_point, options.overlay_only)
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
    src = ('fn f(ui: &mut Ui) { ui.popup("p", &Overlay::DEFAULT.at(x, y + 1.0).w(80.0).h(h).max_h(m).name("n"), |_| {});\n'
           'let n = Node::DEFAULT.w(Size::Grow); ui.progress("e", "E", Some(0.5)); ui.spinner("s", "S"); ui.badge("b", "B", "3");\n'
           'ui.grid_list("g", 10, width - 2.0, 32.0, &mut |ui, i| {}); }')
    out = apply(src, rewrite(src, "pxui", "pxui::Point", False)[0])
    assert '&Overlay::DEFAULT.at(pxui::Point::new(x, y + 1.0)).w_px(80.0).h_px(h).max_h_px(m).name("n")' in out, out
    assert "Node::DEFAULT.w(Size::Grow)" in out, out
    assert 'progress("e", "E", Some(0.5), &pxui::Progress::DEFAULT)' in out and 'spinner("s", "S", &pxui::Spinner::DEFAULT)' in out, out
    assert 'badge("b", "B", "3", &pxui::Badge::DEFAULT)' in out, out
    assert 'grid_list("g", 10, &pxui::GridList::DEFAULT.w_px(width - 2.0).h_px(32.0), &mut' in out, out
    main()
