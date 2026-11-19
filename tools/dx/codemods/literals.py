#!/usr/bin/env python3
"""U3.2 (and U6.2) literal codemod: struct literals of pxui value types become builder chains.

`T { f: v, g, ..BASE }` -> `BASE.f(v).g(g)` for the fields in `SETTERS[T]`, each a plain setter (the builder sets that field
and nothing else, checked against pxui_types), so the result equals the literal for any BASE expression. Specials:
- `Node { at: Some((x, y)).into(), .. }` -> `.at(x, y)`; `Node { wrap: true, .. }` -> `.wrap()`;
- `Rgba { r, g, b, a }` (all four, no base) -> `Rgba::rgba(r, g, b, a)` (`Rgba::rgb` when `a` is 255);
- `Event { kind, code?, x?, y?, ..Default::default() }` -> `Event::new(kind)` / `Event::key(kind, code)` /
  `Event::pointer(kind, code, x, y)` (absent fields are 0).
A literal with a field outside the table, a run-time `wrap`, an `at` of another shape, or no base is skipped and printed.
Patterns (`let Rgba { r, .. } =`, `..}` rests) and type definitions are left alone. Usage: literals.py [--dry-run] FILE...
"""
import argparse
from pathlib import Path
import re

from rs import args, close, line_of, mask

NODE = ["padding", "margin", "look", "axis", "flags", "role", "min_h", "min_w", "max_w", "max_h", "inset", "z", "justify",
        "align", "gap", "row_gap", "scroll_step", "shrink", "state", "w", "h", "grid", "cell", "anchor", "mark", "bar"]
SETTERS = {
    "Node": {f: f for f in NODE},
    "Edges": {f: f for f in ("l", "t", "r", "b")},
    "Look": {f: f for f in ("fill", "text", "font", "opacity")},
    "State": {f: f for f in ("hovered", "pressed", "focused", "disabled", "checked")},
    "States": {f: f for f in ("idle", "hover", "pressed", "focus", "disabled", "checked")},
    "Font": {f: f for f in ("weight", "family", "tracking")},
}
LITERAL = re.compile(r"(?<![\w:])((?:(?:pxui|crate)::(?:\w+::)*)?(Node|Edges|Look|Rgba|State|States|Font|Event)) \{")
DEFAULTS = ("Default::default()", "pxui::Event::default()", "Event::default()")


def fields(source, m, open_, end):
    """[(name, value)] and the base expression (None without `..`), or None for a pattern."""
    out, base = [], None
    if "//" in source[open_:end] or "/*" in source[open_:end]:
        return [("// comment", "")], None  # comments would be lost on one line: skipped, edited by hand
    for a, b in args(m, open_, end):
        text = re.sub(r"\) \.(?=[a-z_])", ").", " ".join(source[a:b].split()))  # a multi-line chain joins on one line
        if text.startswith(".."):
            base = text[2:].strip()
            if not base:
                return None  # a rest pattern
            continue
        name, sep, value = text.partition(":")
        out.append((name.strip(), value.strip() if sep else name.strip()))
    return out, base


def build(kind, path, parsed):
    items, base = parsed
    named = dict(items)
    if kind == "Rgba" and base is None and sorted(named) == ["a", "b", "g", "r"]:
        r, g, b, a = (named[k] for k in "rgba")
        return f"{path}::rgb({r}, {g}, {b})" if a == "255" else f"{path}::rgba({r}, {g}, {b}, {a})"
    if kind == "Event" and base in DEFAULTS and "kind" in named and set(named) <= {"kind", "code", "x", "y"}:
        if "x" in named or "y" in named:
            return f"{path}::pointer({named['kind']}, {named.get('code', '0')}, {named.get('x', '0.0')}, {named.get('y', '0.0')})"
        if "code" in named:
            return f"{path}::key({named['kind']}, {named['code']})"
        return f"{path}::new({named['kind']})"
    table = SETTERS.get(kind)
    if table is None or base is None or base in DEFAULTS:
        return None
    chain = base
    for name, value in items:
        if name == "at" and kind == "Node" and (at := re.fullmatch(r"Some\(\((.+), (.+)\)\)\.into\(\)", value)):
            chain += f".at({at[1]}, {at[2]})"
        elif name == "wrap" and kind == "Node" and value == "true":
            chain += ".wrap()"
        elif name in table:
            chain += f".{table[name]}({value})"
        else:
            return None
    if re.match(r"^[\w:]+$", base) is None and not base.endswith(")"):
        chain = f"({base})" + chain[len(base):]
    return chain


def rewrite(source):
    m = mask(source)
    edits, skipped = [], []
    for match in LITERAL.finditer(m):
        before = m[:match.start()].rstrip()
        if re.search(r"\b(struct|impl|enum|for)$", before) or re.search(r"\blet$", before) or before.endswith(("|", "->")):
            continue
        open_ = match.end() - 1
        end = close(m, open_)
        if any(a < match.start() < b for a, b, _ in edits):
            continue  # nested in a literal already rewritten: the next run takes it
        parsed = fields(source, m, open_, end)
        if parsed is None:
            continue
        new = build(match[2], match[1], parsed)
        if new is None:
            skipped.append((line_of(source, match.start()), " ".join(source[match.start():end + 1].split())[:140]))
            continue
        edits.append((match.start(), end + 1, new))
    return edits, skipped


def apply(source, edits):
    for start, end, replacement in sorted(edits, reverse=True):
        source = source[:start] + replacement + source[end:]
    return source


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("files", nargs="+", type=Path)
    options = parser.parse_args()
    total = 0
    for path in options.files:
        source = original = path.read_text()
        count = 0
        for _ in range(4):  # a literal nested in a rewritten one is taken on the next pass
            edits, skipped = rewrite(source)
            if not edits:
                break
            count += len(edits)
            source = apply(source, edits)
        for n, text in skipped:
            print(f"skipped {path}:{n}: {text}")
        if count:
            print(f"{path}: {count}")
            total += count
            if not options.dry_run and source != original:
                path.write_text(source)
    print(f"total {total} edits")


if __name__ == "__main__":
    src = ('let n = Node { look: X, padding: Edges { l: 4.0, ..Edges::ZERO }, ..Node::column().w(Size::Grow) };\n'
           'let a = Node { at: Some((1.0, 2.0)).into(), wrap: true, ..BASE };\n'
           'let c = Rgba { r: 1, g: 2, b: 3, a: 255 }; let Rgba { r, .. } = c; let s = State { disabled: !on, ..State::default() };\n'
           'let e = pxui::Event { kind: K, x: 1.0, ..Default::default() }; let k = Event { kind: K, code: 3, ..Default::default() };\n'
           'let w = Node { wrap: compact, ..BASE }; struct Node { a: u8 }')
    out = src
    for _ in range(3):
        out = apply(out, rewrite(out)[0])
    assert "let n = Node::column().w(Size::Grow).look(X).padding(Edges::ZERO.l(4.0));" in out, out
    assert "let a = BASE.at(1.0, 2.0).wrap();" in out, out
    assert "Rgba::rgb(1, 2, 3)" in out and "let Rgba { r, .. } = c" in out and "State::default().disabled(!on)" in out, out
    assert "pxui::Event::pointer(K, 0, 1.0, 0.0)" in out and "Event::key(K, 3)" in out, out
    assert "Node { wrap: compact, ..BASE }" in out and "struct Node { a: u8 }" in out, out
    main()
