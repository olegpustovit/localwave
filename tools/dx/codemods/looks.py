#!/usr/bin/env python3
"""U3.1 looks are consts (LocalWave): run-time colours and per-call text styles become theme consts.

- `container(ui, key, axis, w, h, FILL)`: the `Rgba` fill becomes a `&'static Look` from `FILLS`
  (`ui.color(PAPER)` -> `&PAPER_FILL`); an `if c { A } else { B }` fill maps both arms.
- `detail::line(ui, key, value, (size, line_height, weight, tracking), ui.color(T), Flags::NONE)` -> `ui.label(value, CONST)`
  with CONST from `LINES` (a keyless label in a `theme.rs` line-box style).
- compact's `line(ui, key, value, size, ui.color(T))` (compact.rs, compact/list.rs) -> `ui.label(value, CONST)`, CONST from
  `COMPACT_LINES`.
Fills, styles or call shapes missing from the tables are printed as skipped and left for hand edits; no scope analysis, the
compiler flags misses. Usage: looks.py [--dry-run] FILE...
"""
import argparse
from pathlib import Path
import re

from rs import args, close, line_of, mask

FILLS = {
    "ui.color(PAPER)": "&PAPER_FILL",
    "Rgba::token(PAPER)": "&PAPER_FILL",
    "ui.color(INK)": "&INK_FILL",
    "ui.color(PANEL)": "&PANEL_FILL",
    "ui.color(kit::PANEL)": "&PANEL_FILL",
    "ui.color(LINE)": "&LINE_FILL",
    "Rgba::default()": "&Look::NONE",
    "Rgba { a: 153, ..ui.color(PANEL) }": "&PANEL_WASH",
}
LINES = {
    ("(32.0, 33.6, 500, -0.035)", "ui.color(INK)"): "PAGE_TITLE",
    ("(40.0, 40.8, 500, TIGHT_3)", "ui.color(INK)"): "ALBUM_TITLE",
    ("(48.0, 48.0, 500, -0.025)", "ui.color(INK)"): "ARTIST_TITLE",
    ("(20.0, 28.0, 500, -0.01)", "ui.color(INK)"): "SHEET_TITLE",
    ("(17.0, 23.8, 500, 0.0)", "ui.color(INK)"): "OPTION_TITLE",
    ("(14.0, 17.5, 500, 0.0)", "ui.color(INK)"): "CARD_TITLE",
    ("(13.0, 18.2, 400, 0.0)", "ui.color(SECONDARY)"): "SUMMARY",
    ("(14.0, 19.6, 400, 0.0)", "ui.color(SECONDARY)"): "ARTIST_SUMMARY",
    ("(13.0, 18.2, 500, 0.0)", "ui.color(SECONDARY)"): "SECTION_CAPTION",
}
COMPACT_LINES = {
    ("28.0", "ui.color(INK)"): "PHONE_TITLE",
    ("16.0", "ui.color(INK)"): "GROUP_TITLE",
    ("15.0", "ui.color(INK)"): "ROW_TITLE",
    ("14.0", "ui.color(INK)"): "MINI_TITLE",
    ("14.0", "ui.color(SECONDARY)"): "EMPTY_NOTE",
    ("13.0", "ui.color(SECONDARY)"): "ROW_NOTE",
    ("12.0", "ui.color(SECONDARY)"): "SMALL_NOTE",
}
CONTAINER = re.compile(r"(?<![\w.])container\(")
# `line(` is detail's own helper only inside detail.rs (compact.rs has a five-argument `line`)
DETAIL_LINE = re.compile(r"(?<![\w.])(?:super::)?detail::line\(")
LOCAL_LINE = re.compile(r"(?<![\w.:])line\(")
IF_ELSE = re.compile(r"^if (.+?) \{ (.+?) \} else \{ (.+?) \}$", re.S)


def norm(text):
    return " ".join(text.split())


def fill(text):
    text = norm(text)
    if text in FILLS:
        return FILLS[text]
    m = IF_ELSE.match(text)
    if m and m[2] in FILLS and m[3] in FILLS:
        return f"if {m[1]} {{ {FILLS[m[2]]} }} else {{ {FILLS[m[3]]} }}"
    return None


def rewrite(source, detail, compact=False):
    m = mask(source)
    edits, skipped = [], []

    def skip(at, why):
        skipped.append((line_of(source, at), why, norm(source[at:source.find("\n", at)])))

    for match in CONTAINER.finditer(m):
        if m[max(0, match.start() - 3):match.start()] == "fn ":
            continue
        open_ = match.end() - 1
        parts = args(m, open_, close(m, open_))
        if len(parts) != 6:
            skip(match.start(), "container arguments")
            continue
        a, b = parts[5]
        if norm(source[a:b]).startswith("&") or re.match(r"if .+ \{ &", norm(source[a:b])):
            continue  # already a look
        new = fill(source[a:b])
        if new is None:
            skip(match.start(), "fill")
            continue
        lead = a + len(source[a:b]) - len(source[a:b].lstrip())
        edits.append((lead, b - (len(source[a:b]) - len(source[a:b].rstrip())), new))
    patterns = [DETAIL_LINE] + ([LOCAL_LINE] if detail else [])
    for pattern in patterns:
        for match in pattern.finditer(m):
            if m[max(0, match.start() - 3):match.start()] == "fn ":
                continue
            open_ = match.end() - 1
            end = close(m, open_)
            parts = args(m, open_, end)
            if len(parts) != 6 or norm(source[slice(*parts[5])]) != "Flags::NONE" or norm(source[slice(*parts[0])]) != "ui":
                skip(match.start(), "line call shape")
                continue
            style = LINES.get((norm(source[slice(*parts[3])]), norm(source[slice(*parts[4])])))
            if style is None:
                skip(match.start(), "line style")
                continue
            edits.append((match.start(), end + 1, f"ui.label({norm(source[slice(*parts[2])])}, {style})"))
    if compact:
        for match in LOCAL_LINE.finditer(m):
            if m[max(0, match.start() - 3):match.start()] == "fn ":
                continue
            open_ = match.end() - 1
            end = close(m, open_)
            parts = args(m, open_, end)
            if len(parts) != 5 or norm(source[slice(*parts[0])]) != "ui":
                skip(match.start(), "compact line call shape")
                continue
            style = COMPACT_LINES.get((norm(source[slice(*parts[3])]), norm(source[slice(*parts[4])])))
            if style is None:
                skip(match.start(), "compact line style")
                continue
            edits.append((match.start(), end + 1, f"ui.label({norm(source[slice(*parts[2])])}, {style})"))
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
        compact = path.name == "compact.rs" or (path.name == "list.rs" and path.parent.name == "compact")
        edits, skipped = rewrite(source, path.name == "detail.rs", compact)
        for n, why, call in skipped:
            print(f"skipped {path}:{n} ({why}): {call}")
        if edits:
            print(f"{path}: {len(edits)}")
            total += len(edits)
            if not options.dry_run:
                path.write_text(apply(source, edits))
    print(f"total {total} edits")


if __name__ == "__main__":
    src = ('fn f(ui: &mut Ui) { let a = container(ui, "a", Axis::Row, Size::Grow, Size::Px(1.0), ui.color(PAPER));\n'
           'container(ui, "u", Axis::Row, Size::Grow, Size::Px(2.0), if active { ui.color(INK) } else { ui.color(PAPER) });\n'
           'container(ui, "x", Axis::Row, Size::Grow, Size::Px(2.0), ui.color(ACCENT));\n'
           'detail::line(ui, "name", &g.title, (14.0, 17.5, 500, 0.0), ui.color(INK), Flags::NONE);\n'
           'line(ui, "kind", if album { "Album" } else { "Artist" }, (13.0, 18.2, 400, 0.0), ui.color(SECONDARY), Flags::NONE); }\n'
           'pub(super) fn container(ui: &mut Ui, key: &str) {}')
    edits, skipped = rewrite(src, True)
    out = apply(src, edits)
    assert 'Size::Px(1.0), &PAPER_FILL);' in out and "if active { &INK_FILL } else { &PAPER_FILL });" in out, out
    assert "ui.label(&g.title, CARD_TITLE);" in out, out
    assert 'ui.label(if album { "Album" } else { "Artist" }, SUMMARY);' in out, out
    assert [why for _, why, _ in skipped] == ["fill"] and "ui.color(ACCENT)" in out, skipped
    src = 'fn g(ui: &mut Ui) { line(ui, "count", &group.count, 12.0, ui.color(SECONDARY)); line(ui, "title", t, 15.0, ink); }'
    edits, skipped = rewrite(src, False, True)
    out = apply(src, edits)
    assert "ui.label(&group.count, SMALL_NOTE);" in out and [why for _, why, _ in skipped] == ["compact line style"], (out, skipped)
    main()
