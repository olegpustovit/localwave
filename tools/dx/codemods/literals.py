#!/usr/bin/env python3
"""U3.2 (and U6.2) literal codemod: struct literals of pxui value types become builder chains.

`T { f: v, g, ..BASE }` -> `BASE.f(v).g(g)` for the fields in `SETTERS[T]`, each a plain setter (the builder sets that field
and nothing else, checked against pxui_types), so the result equals the literal for any BASE expression. Specials:
- `Node { at: Some((x, y)).into(), .. }` -> `.at(x, y)`; `Node { wrap: true, .. }` -> `.wrap()`;
- `Rgba { r, g, b, a }` (all four, no base) -> `Rgba::rgba(r, g, b, a)` (`Rgba::rgb` when `a` is 255);
- `Event { kind, code?, x?, y?, ..Default::default() }` -> `Event::new(kind)` / `Event::key(kind, code)` /
  `Event::pointer(kind, code, x, y)` (absent fields are 0).
A wire field without a builder can be assigned after construction so malformed fixtures stay malformed. Unsupported literals are printed.
Patterns (`let Rgba { r, .. } =`, `..}` rests) and type definitions are left alone. Usage: literals.py [--dry-run] FILE...
"""
import argparse
from pathlib import Path
import re

from rs import args, close, line_of, mask

NODE = ["padding", "margin", "look", "axis", "flags", "role", "min_h", "min_w", "max_w", "max_h", "inset", "z", "justify",
        "align", "gap", "row_gap", "scroll_step", "shrink", "state", "w", "h", "grid", "cell", "anchor", "mark", "bar", "grow", "aspect", "a11y"]
SETTERS = {
    "Node": {f: f for f in NODE},
    "Edges": {f: f for f in ("l", "t", "r", "b")},
    "Look": {f: f for f in ("fill", "text", "font", "opacity", "radius", "angle", "sides", "radii", "shadows", "ring", "selection", "caret")},
    "State": {f: f for f in ("hovered", "pressed", "focused", "disabled", "checked")},
    "States": {f: f for f in ("idle", "hover", "pressed", "focus", "disabled", "checked")},
    "Font": {f: f for f in ("weight", "family", "tracking")},
}
SETTERS["Look"]["radius"] = "radii"
LITERAL = re.compile(r"(?<![\w:])((?:(?:\w+)::)*(Node|Edges|Look|Rgba|State|States|Font|Event|FrameIn|TextSpan|Image|SectionList|Str))\s*\{")
DEFAULTS = ("Default::default()", "pxui::Event::default()", "Event::default()")


def fields(source, m, open_, end):
    """[(name, value)] and the base expression (None without `..`), or None for a pattern."""
    out, base = [], None
    if "//" in source[open_:end] or "/*" in source[open_:end]:
        return [("// comment", "")], None  # comments would be lost on one line: skipped, edited by hand
    for a, b in args(m, open_, end):
        text = source[a:b].strip()
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
    if kind == "Edges" and base is None and named.keys() == set("ltrb"):
        return f"{path}::new({', '.join(named[n] for n in 'ltrb')})"
    if kind == "Str" and base is None and named.keys() == {"ptr", "len"}:
        text = re.fullmatch(r'b?"([ -!#-\[\]-~]*)"\.as_ptr\(\)', named["ptr"])
        if text and named["len"] == str(len(text[1])):
            return f'{path}::from("{text[1]}")'
        return assign(f"{path}::default()", items)
    if kind == "States" and base is None and named.keys() == SETTERS["States"].keys():
        return f"{path}::of({named['idle']})" + "".join(f".{name}({v})" for name, v in items if name != "idle")
    if kind == "State" and base is None and named.keys() == SETTERS["State"].keys():
        module = path.removesuffix("State").removesuffix("theme::")
        base = f"{module}Node::DEFAULT.state"
    if kind == "Image" and base is None and named.keys() == {"size", "vk_image_view", "mtl_texture", "width", "height", "format"}:
        if named["size"] == f"core::mem::size_of::<{path}>() as u32" and named["format"] == f"{path}::FORMAT_RGBA8":
            return f"{path}::rgba8({', '.join(named[name] for name in ('width', 'height', 'vk_image_view', 'mtl_texture'))})"
    if kind == "Event" and (base in DEFAULTS or base == f"{path}::default()") and "kind" in named and set(named) <= {"kind", "code", "x", "y"}:
        if "x" in named or "y" in named:
            return f"{path}::pointer({named['kind']}, {named.get('code', '0')}, {named.get('x', '0.0')}, {named.get('y', '0.0')})"
        if "code" in named:
            return f"{path}::key({named['kind']}, {named['code']})"
        return f"{path}::new({named['kind']})"
    vanilla = base is None or base in DEFAULTS or base == f"{path}::default()"
    if kind == "FrameIn" and vanilla and base is not None and set(named) <= {"width", "height", "scale", "time"}:
        chain = f"{path}::new({named.get('width', '0.0')}, {named.get('height', '0.0')})"
        if set(named) <= {"width", "height"}:
            return chain
        # Wire FrameIn has no setters. Use mutations for checked and deliberately malformed test records alike.
        return assign(chain, [(name, v) for name, v in items if name not in {"width", "height"}])
    if kind == "SectionList" and (base is not None or "count" in named):
        chain = base or f"{path}::new({named['count']})"
        setters = {"row_height": "row_h", "leading_height": "lead", "header_height": "header", "trailing_height": "trail", "reorder": "reorder"}
        extras = []
        for name, v in items:
            if name in setters:
                chain += f".{setters[name]}({v})"
            elif name in {"overlay_scrollbar", "measured"} and v in {"true", "false"}:
                if v == "true":
                    chain += f".{name}()"
                elif base is not None:
                    extras.append((name, v))
            elif name == "count" and base is not None:
                extras.append((name, v))
            elif name != "count":
                return None
        return assign(chain, extras)
    if kind == "TextSpan" and vanilla and {"start", "end"} <= named.keys():
        return assign(f"{path}::new({named['start']}, {named['end']})", [(n, v) for n, v in items if n not in {"start", "end"}])
    if kind in {"Event", "FrameIn", "Image", "Str", "TextSpan", "Rgba"} and base is not None:
        # Preserve wire fields for tests where a builder would normalize or reject the input.
        return assign(f"{path}::default()" if vanilla else base, items)
    table = SETTERS.get(kind)
    if table is None or base is None:
        return None
    if base == "Default::default()":
        base = f"{path}::default()"
    chain = base
    unmatched = []
    for name, value in items:
        if chain == base and not (re.fullmatch(r"[\w:.]+", base) or base.endswith(")")):
            chain = f"({base})"
        if name == "at" and kind == "Node" and (at := re.fullmatch(r"Some\(\((.+), (.+)\)\)\.into\(\)", value)):
            chain += f".at({at[1]}, {at[2]})"
        elif name == "at" and kind == "Node" and (at := re.fullmatch(r"Some\((\w+)\)\.into\(\)", value)):
            chain += f".at({at[1]}.0, {at[1]}.1)"
        elif name == "wrap" and kind == "Node" and value == "true":
            chain += ".wrap()"
        elif name in {"grid", "anchor", "bar", "align_self"} and kind == "Node":
            if value.startswith("Some(") and value.endswith(").into()"):
                chain += f".{name}({value[5:-8]})"
            else:
                unmatched.append((name, value))
        elif name == "border" and kind == "Look" and "border_w" in named:
            chain += f".border({value}, {named['border_w']})"
        elif name == "border_w" and kind == "Look" and "border" in named:
            continue
        elif name in table:
            chain += f".{table[name]}({value})"
        else:
            unmatched.append((name, value))
    if unmatched and kind not in {"Node", "Look", "Font"}:
        return None
    return assign(base if chain == f"({base})" else chain, unmatched)


def assign(base, fields):
    if not fields:
        return base
    # Evaluate each fixture expression once before changing the record.
    assignments = " ".join(f"record.{name} = {value};" for name, value in fields)
    return f"{{ let mut record = {base}; {assignments} record }}"


def rewrite(source, tests_only=False):
    m = mask(source)
    aliases = {alias: kind for kind, alias in re.findall(r"\b(\w+)\s+as\s+([A-Z]\w*)\b", m)
               if LITERAL.fullmatch(kind + " {")}
    pattern = LITERAL if not aliases else re.compile(
        LITERAL.pattern.replace("|SectionList|Str)", "|SectionList|Str|" + "|".join(sorted(aliases)) + ")"))
    test_ranges = []
    if tests_only:
        attribute_tail = r"\s*(?:#\[[^\n]*\]\s*)*"
        test_module = r"#\[cfg\((?:all\(\s*)?test[^\]\n]*\]" + attribute_tail + r"(?:pub\s+)?mod\s+\w+\s*\{"
        test_fn = r"#\[test\]" + attribute_tail + r"(?:pub\s+)?fn\s+\w+[^{};]*\{"
        test_ranges = [(test.end(), close(m, test.end() - 1)) for test in re.finditer(f"{test_module}|{test_fn}", m)]
    edits, skipped = [], []
    for match in pattern.finditer(m):
        if tests_only and not any(start <= match.start() < end for start, end in test_ranges):
            continue
        before = m[:match.start()].rstrip()
        if re.search(r"\b(struct|impl|enum|for)$", before) or re.search(r"\blet$", before) or before.endswith("->"):
            continue
        open_ = match.end() - 1
        end = close(m, open_)
        if m[end + 1:].lstrip().startswith(("=", "|")):
            continue  # destructuring or a match arm
        if any(a < match.start() < b for a, b, _ in edits):
            continue  # nested in a literal already rewritten: the next run takes it
        parsed = fields(source, m, open_, end)
        if parsed is None:
            continue
        new = build(aliases.get(match[2], match[2]), match[1], parsed)
        if new is None:
            skipped.append((line_of(source, match.start()), " ".join(source[match.start():end + 1].split())[:140]))
            continue
        edits.append((match.start(), end + 1, new))
    return edits, skipped


def apply(source, edits):
    for start, end, replacement in sorted(edits, reverse=True):
        source = source[:start] + replacement + source[end:]
    return source


def apply_qualification(source, path, test_files=()):
    """Touch only fenced rustdoc examples, test modules and standalone qualification sources."""
    docs = re.compile(r"(?m)^(?P<prefix>[ \t]*//[/!]) ?```(?:rust)?[ \t]*\n(?P<body>(?:(?P=prefix).*\n)*?)(?P=prefix) ?```[ \t]*$")
    edits, skipped = [], []
    for block in docs.finditer(source):
        prefix = block["prefix"]
        lines = block["body"].splitlines()
        snippet = "\n".join(line[len(prefix):].removeprefix(" ") for line in lines)
        changes, gaps = rewrite(snippet)
        skipped.extend((line_of(source, block.start("body")) + line - 1, text) for line, text in gaps)
        if changes:
            replacement = "".join(prefix + (" " + line if line else "") + "\n" for line in apply(snippet, changes).splitlines())
            edits.append((block.start("body"), block.end("body"), replacement))
    standalone = path in test_files or "tests" in path.parts or path.stem.startswith("test_") or path.stem.endswith("_tests") or any(part in {"pxui_gallery", "abi_bench", "abi_equal", "ios_spike"} for part in path.parts)
    changes, gaps = rewrite(source, tests_only=not standalone)
    return edits + changes, skipped + gaps


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--qualification", action="store_true", help="Limit source edits to examples, tests and gallery or benchmark tools")
    parser.add_argument("files", nargs="+", type=Path)
    options = parser.parse_args()
    test_files = set()
    if options.qualification:
        external = re.compile(r"#\[cfg\((?:all\(\s*)?test[^\]\n]*\]\s*(?:#\[[^\n]*\]\s*)*mod\s+(\w+)\s*;")
        for path in options.files:
            for module in external.findall(mask(path.read_text())):
                test_files.add(path.with_name(module + ".rs"))
                test_files.add(path.with_name(module) / "mod.rs")
    total = 0
    for path in options.files:
        source = original = path.read_text()
        count = 0
        for _ in range(4):  # a literal nested in a rewritten one is taken on the next pass
            edits, skipped = apply_qualification(source, path, test_files) if options.qualification else rewrite(source)
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
    assert "record.wrap = compact;" in out and "struct Node { a: u8 }" in out, out
    snippet = '/// ```\n/// let edge = Edges { l: 1.0, t: 2.0, r: 3.0, b: 4.0 };\n/// ```\nlet other = Node { wrap: true, ..BASE };'
    swept = apply(snippet, apply_qualification(snippet, Path('lib.rs'))[0])
    assert '/// let edge = Edges::new(1.0, 2.0, 3.0, 4.0);' in swept and 'Node { wrap: true, ..BASE }' in swept, swept
    main()
