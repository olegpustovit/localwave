#!/usr/bin/env python3
"""Rewrite build-path `Node { .. }` and `Edges { .. }` literals as inline builder chains (D25.2; api_conformance R1).

- `Edges { l: a, t: b, ..Edges::ZERO }` -> `Edges::ZERO.l(a).t(b)`; all four sides and no base -> `Edges::new(l, t, r, b)`;
- `Node { padding: p, look: k, ..BASE }` -> `BASE.padding(p).look(k)`: a field set after the base is what the literal's field
  overrides, and every builder used here only sets its field (pxui_types node.rs). `wrap: true` -> `.wrap()`; shorthand `look` ->
  `.look(look)`. A literal with a field that has no plain builder (`at`, `transition`, a runtime `wrap`, ...) or no base is kept.
Literals inside `const`/`static` items and `#[cfg(test)]` items are not R1 violations and are left alone. Inner literals are
rewritten first. Usage: builders.py FILE.. (rewrites in place, prints the count).
"""
import re
import sys

from rs import args, close, mask

NODE_FIELDS = {"w", "h", "grow", "gap", "align", "role", "flags", "justify", "z", "shrink", "padding", "margin", "row_gap", "look",
               "inset", "mark", "state"}
EDGE_SIDES = ("l", "t", "r", "b")
LITERAL = re.compile(r"(?<![\w:])(?:pxui::(?:ui::|geom::)?)?(Node|Edges)\s*\{")
HEAD = re.compile(r"(->|\bimpl|\bfor|\bstruct|\benum|\btrait|\bmod|\btype|\bas|\bdyn)\s*$")


def skipped_spans(m):
    """Spans of const/static items and #[cfg(test)] items (m a mask)."""
    spans = []
    for kw in re.finditer(r"(?<![\w])(const|static) (?!fn )", m):
        eq = re.compile(r"[=;{]").search(m, kw.end())
        if not eq or m[eq.start()] != "=":
            continue
        depth, j = 0, eq.start()
        while j < len(m):
            c = m[j]
            depth += c in "([{"
            depth -= c in ")]}"
            if c == ";" and depth == 0:
                break
            j += 1
        spans.append((kw.start(), j))
    for t in re.finditer(r"#\[cfg\(test\)\]", m):
        brace = m.find("{", t.end())
        semi = m.find(";", t.end())
        if brace >= 0 and (semi < 0 or brace < semi):
            spans.append((t.start(), close(m, brace)))
    return spans


def fields(src, m, open_, end):
    """[(name, value)] of the literal between m[open_] and m[end]; the base as ('..', expr); None when not plain."""
    out = []
    for a, b in args(m, open_, end):
        text, mt = src[a:b].strip(), m[a:b].strip()
        if not text:
            continue
        if text.startswith(".."):
            out.append(("..", text[2:].strip()))
        elif re.fullmatch(r"[a-z_]+", mt):
            out.append((text, text))
        elif mf := re.match(r"([a-z_]+)\s*:", mt):
            out.append((mf.group(1), text[mf.end():].strip()))
        else:
            return None
    return out


def rewrite(kind, items):
    base = [v for k, v in items if k == ".."]
    sets = [(k, v) for k, v in items if k != ".."]
    if kind == "Edges":
        if not base and sorted(k for k, _ in sets) == sorted(EDGE_SIDES):
            side = dict(sets)
            return "Edges::new({})".format(", ".join(side[s] for s in EDGE_SIDES))
        if base != ["Edges::ZERO"] or any(k not in EDGE_SIDES for k, _ in sets):
            return None
        return "Edges::ZERO" + "".join(f".{k}({v})" for k, v in sets)
    if len(base) != 1:
        return None
    chain = base[0]
    for k, v in sets:
        if k == "wrap" and v == "true":
            chain += ".wrap()"
        elif k in NODE_FIELDS:
            chain += f".{k}({v})"
        else:
            return None
    # a base that is not a path or a call chain (`a + b`, a cast) would bind to the first builder
    return chain if re.fullmatch(r"[\w:.()&\[\]\s,\-+*/'\"]*", chain) and not re.search(r"\s[+\-*/]\s", base[0]) else None


def run(src):
    count = 0
    while True:
        m = mask(src)
        spans = skipped_spans(m)
        best = None
        for lit in LITERAL.finditer(m):
            if HEAD.search(m[: lit.start()]) or any(a <= lit.start() < b for a, b in spans):
                continue
            open_ = lit.end() - 1
            end = close(m, open_)
            items = fields(src, m, open_, end)
            new = items and rewrite(lit.group(1), items)
            if new and "\n" not in new and (best is None or lit.start() > best[0]):
                best = (lit.start(), end + 1, new)
        if best is None:
            return src, count
        a, b, new = best
        src, count = src[:a] + new + src[b:], count + 1


def check():
    src = "fn f(ui: &mut Ui) { ui.row(Node { padding: Edges { t: 10.0, l: 6.0, ..Edges::ZERO }, look, ..Node::row().w(Size::Grow) }, |ui| {}); }"
    out, n = run(src)
    assert out == "fn f(ui: &mut Ui) { ui.row(Node::row().w(Size::Grow).padding(Edges::ZERO.t(10.0).l(6.0)).look(look), |ui| {}); }", out
    assert n == 2
    kept = "const N: Node = Node { gap: 1.0, ..Node::DEFAULT };\nfn g() { let n = Node { at: Some((0.0, 0.0)).into(), ..Node::DEFAULT }; }"
    assert run(kept) == (kept, 0)
    assert run("fn h() { e(Edges { l: 1.0, t: 2.0, r: 3.0, b: 4.0 }) }")[0] == "fn h() { e(Edges::new(1.0, 2.0, 3.0, 4.0)) }"


if __name__ == "__main__":
    check()
    total = 0
    for path in sys.argv[1:]:
        src = open(path).read()
        out, n = run(src)
        if n:
            open(path, "w").write(out)
            print(f"{path}: {n}")
        total += n
    print(f"builders: {total} literals rewritten")
