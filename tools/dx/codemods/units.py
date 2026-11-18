#!/usr/bin/env python3
"""Migrate U1b.3 measurement and units (pxui wrapper callers and LocalWave, code and rustdoc examples).

- text service: `.measure/.measure_spans/.width/.quads/.rasterize_rgba(value, &STYLE, W, ..)` -> `STYLE` by value and the max width
  as `Option<f32>` (`0.0` -> `None`, otherwise `Some(W)`); a `quads` origin `(x, y)` -> `Point::new(x, y)`;
- `ui.text_spans/text_line_box(&node, text, &STYLE, spans)` -> `STYLE` by value;
- `paint.text((x, y), &STYLE, w, s)` / `paint.text_chars((x, y), &STYLE, s)` -> `Point::new(x, y)` and `STYLE` by value;
- `ui.menu(key, (x, y), ..)` -> `Point::new(x, y)`;
- `ui.view_size().0/.1` -> `.width/.height`; `let (w, h) = ui.view_size();` -> `let ViewSize { width: w, height: h } = ..`;
- whole-point literals: `Font::BODY.size(13)` / `.font(..size(13))`, `set_font_size(14)`, `PanelOpts .padding(8)` -> `13.0`..;
- `ui.density() as f32` -> `ui.density()`.
No scope analysis: the compiler flags misses. `text_width`/`text_top`/`cap_width`/`Paint::text_metrics`, `paint.text_spans` with a
resolved `TextStyle` and `enable_system_fonts` on a `Ui` are reported for hand edits.
"""
import argparse
from pathlib import Path
import re

from namespace import DOC_CODE
from rs import args, close, line_of, mask

SERVICE = re.compile(r"\.(measure|measure_spans|width|quads|rasterize_rgba)\(")
NODE_TEXT = re.compile(r"\.(text_spans|text_line_box)\(")
PAINT = re.compile(r"\.(text|text_chars)\(")
MENU = re.compile(r"\.menu\(")
VIEW = re.compile(r"\.view_size\(\)\.([01])\b")
VIEW_LET = re.compile(r"let\s+\(\s*(\w+)\s*,\s*(\w+)\s*\)\s*=\s*([\w.]+)\.view_size\(\);")
INT_SIZE = re.compile(r"(Font::BODY\.size|\.set_font_size|PanelOpts::default\(\)\.padding)\((-?\d+)\)")
DENSITY = re.compile(r"\b(?:ui|paint)\.density\(\)\s+as\s+f32")
REPORT = re.compile(r"\.(text_width|text_top|cap_width|text_metrics)\(|\bui\.enable_system_fonts\(")


def tuple_point(source, a, b, point):
    """`Point::new(x, y)` for a `(x, y)` tuple literal argument at source[a:b], else None."""
    text = source[a:b].strip()
    if not (text.startswith("(") and text.endswith(")")):
        return None
    inner = mask(text)
    if close(inner, 0) != len(text) - 1:
        return None
    parts = args(inner, 0, len(text) - 1)
    if len(parts) != 2:
        return None
    x, y = (text[s:e].strip() for s, e in parts)
    return f"{point}::new({x}, {y})"


def arg_edit(source, a, b, new):
    lead = len(source[a:b]) - len(source[a:b].lstrip())
    return (a + lead, a + lead + len(source[a:b].strip()), new)


def by_value(source, a, b):
    text = source[a:b].strip()
    return arg_edit(source, a, b, text[1:].lstrip()) if text.startswith("&") and not text.startswith("&mut") else None


def rewrite(source, prefix):
    m = mask(source)
    point, view = f"{prefix}::Point", f"{prefix}::ViewSize"
    edits, skipped = [], []
    for match in SERVICE.finditer(m):
        name, open_ = match[1], match.end() - 1
        # only the in-frame service: a worker `Text`, shaper or `TextWorker` keeps its resolved `&TextStyle` and `f32` width
        if not re.search(r"(?:text_service\(\)|\bservice)\s*$", m[:match.start()]):
            continue
        parts = args(m, open_, close(m, open_))
        least = {"measure": 3, "width": 3, "measure_spans": 4, "quads": 6, "rasterize_rgba": 6}[name]
        if len(parts) != least or not source[parts[1][0]:parts[1][1]].strip().startswith("&"):
            continue
        edits.append(by_value(source, *parts[1]))
        w = source[parts[2][0]:parts[2][1]].strip()
        if not w.startswith(("None", "Some(")):
            edits.append(arg_edit(source, *parts[2], "None" if w in ("0.0", "0.") else f"Some({w})"))
        if name == "quads" and (origin := tuple_point(source, *parts[3], point)):
            edits.append(arg_edit(source, *parts[3], origin))
    for match in NODE_TEXT.finditer(m):
        open_ = match.end() - 1
        parts = args(m, open_, close(m, open_))
        if len(parts) == 4 and source[parts[0][0]:parts[0][1]].strip().startswith("&") and (e := by_value(source, *parts[2])):
            edits.append(e)
    for match in PAINT.finditer(m):
        name, open_ = match[1], match.end() - 1
        parts = args(m, open_, close(m, open_))
        if len(parts) != (4 if name == "text" else 3) or not (origin := tuple_point(source, *parts[0], point)):
            continue
        edits.append(arg_edit(source, *parts[0], origin))
        if e := by_value(source, *parts[1]):
            edits.append(e)
    for match in MENU.finditer(m):
        open_ = match.end() - 1
        parts = args(m, open_, close(m, open_))
        if len(parts) == 4 and (at := tuple_point(source, *parts[1], point)):
            edits.append(arg_edit(source, *parts[1], at))
    for match in VIEW.finditer(m):
        edits.append((match.end(1) - 1, match.end(1), "width" if match[1] == "0" else "height"))
    for match in VIEW_LET.finditer(m):
        field = lambda name, var: "" if var == "_" else name if var == name else f"{name}: {var}"
        w, h = field("width", match[1]), field("height", match[2])
        fields = ", ".join(f for f in (w, h) if f) + (", .." if "" in (w, h) else "")
        edits.append((match.start(), match.end(), f"let {view} {{ {fields} }} = {match[3]}.view_size();"))
    for match in INT_SIZE.finditer(m):
        edits.append((match.start(2), match.end(2), match[2] + ".0"))
    for match in DENSITY.finditer(m):
        edits.append((match.start() + match[0].index(")") + 1, match.end(), ""))
    for match in REPORT.finditer(m):
        skipped.append((line_of(source, match.start()), source[match.start():source.find("\n", match.start())].strip()))
    for match in re.finditer(r"\.text_spans\(", m):
        parts = args(m, match.end() - 1, close(m, match.end() - 1))
        if len(parts) == 5:
            skipped.append((line_of(source, match.start()), "paint.text_spans with a resolved TextStyle"))
    return sorted(e for e in edits if e), skipped


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
        prefix = "crate" if "crates/public/pxui/src" in str(path) else "pxui"
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
    print(f"total {total} edits")


if __name__ == "__main__":
    src = ('fn f(ui: &mut Ui) { let m = ui.text_service().measure("a", &L, 0.0); ui.text_service().width(v, &crate::Label::BODY, 40.0);\n'
           'service.quads(v, &L, 0.0, (1.0, 2.0), 1.0, &mut g); text.measure(v, &R, 0.0); ui.text_line_box(&n, "x", &S, &[]); paint.text((r.x, r.y + 2.0), &L, 0.0, s);\n'
           'paint.text_chars((1.0, 2.0), &C.color(c), s); ui.menu("m", (x, at.1), &R, &M); let w = ui.view_size().0;\n'
           'let (width, _) = ui.view_size(); Font::BODY.size(13); ui.set_font_size(14); let d = ui.density() as f32; node.width(4.0); }')
    out = apply(src, rewrite(src, "pxui")[0])
    assert 'measure("a", L, None)' in out and "ui.text_service().width(v, crate::Label::BODY, Some(40.0))" in out, out
    assert "service.quads(v, L, None, pxui::Point::new(1.0, 2.0), 1.0, &mut g)" in out and "text.measure(v, &R, 0.0)" in out and 'text_line_box(&n, "x", S, &[])' in out, out
    assert "paint.text(pxui::Point::new(r.x, r.y + 2.0), L, 0.0, s)" in out and "text_chars(pxui::Point::new(1.0, 2.0), C.color(c), s)" in out, out
    assert 'ui.menu("m", pxui::Point::new(x, at.1), &R, &M)' in out and "view_size().width" in out, out
    assert "let pxui::ViewSize { width, .. } = ui.view_size();" in out, out
    assert "size(13.0)" in out and "set_font_size(14.0)" in out and "let d = ui.density();" in out and "node.width(4.0)" in out, out
    main()
