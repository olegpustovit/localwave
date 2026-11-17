#!/usr/bin/env python3
"""Codemod A: `Node { ..., ..Node::DEFAULT }` literals -> pxui builder chains.

Usage: codemod.py FILE...   (rewrites in place, prints skipped sites with reasons)

A field becomes a builder call only when that builder sets exactly that field to exactly that value
(see pxui_types/src/node.rs). Every other field stays in the literal over the builder base.
"""
import re
import sys

FLAG_NAMES = {
    "CLICKABLE", "FOCUSABLE", "SCROLL", "CLIP", "BLOCKING", "TAB_ONLY", "TAB_STOP", "KEEP_TAB", "KEEP_FOCUS",
    "OVER_THUMB", "CAPTURE_UNCLAIMED", "FOCUS_MARK", "TAB_STOP_MARKED", "FIT_TEXT", "FIT_TEXT_PIXEL", "SCROLL_X",
    "STICKY", "ROUND_CLIP", "HOVER_FREE", "KEEP_ROOM", "DRAG_FREE",
}
OPEN = "([{"
CLOSE = ")]}"


def skip_literal(s, i):
    """If s[i] starts a string/char/comment, return index after it, else None."""
    c = s[i]
    if s.startswith("//", i):
        j = s.find("\n", i)
        return len(s) if j < 0 else j
    if s.startswith("/*", i):
        return s.index("*/", i) + 2
    m = re.match(r'b?r(#*)"', s[i:])
    if m and (i == 0 or not (s[i - 1].isalnum() or s[i - 1] == "_")):
        end = '"' + m.group(1)
        return s.index(end, i + m.end()) + len(end)
    if c == '"' or (c == "b" and s.startswith('b"', i) and (i == 0 or not s[i - 1].isalnum())):
        j = i + (2 if c == "b" else 1)
        while s[j] != '"':
            j += 2 if s[j] == "\\" else 1
        return j + 1
    if c == "'":
        m = re.match(r"'(\\.[^']*|[^\\'])'", s[i:])
        if m:
            return i + m.end()
    return None


def match_close(s, i):
    """s[i] is an opener; return index of its matching closer."""
    depth = 0
    j = i
    while j < len(s):
        k = skip_literal(s, j)
        if k is not None:
            j = k
            continue
        if s[j] in OPEN:
            depth += 1
        elif s[j] in CLOSE:
            depth -= 1
            if depth == 0:
                return j
        j += 1
    raise ValueError("unbalanced")


def split_top(s, sep=","):
    """Split s at top-level separators (outside brackets/strings/comments)."""
    parts, depth, start, j = [], 0, 0, 0
    while j < len(s):
        k = skip_literal(s, j)
        if k is not None:
            j = k
            continue
        c = s[j]
        if c in OPEN:
            depth += 1
        elif c in CLOSE:
            depth -= 1
        elif depth == 0 and s.startswith(sep, j) and not (sep == "|" and s.startswith("||", j)):
            if sep == "|" and j > 0 and s[j - 1] == "|":
                j += 1
                continue
            parts.append(s[start:j])
            start = j + len(sep)
            j += len(sep)
            continue
        j += 1
    parts.append(s[start:])
    return parts


def has_comment(s):
    j = 0
    while j < len(s):
        if s.startswith("//", j) or s.startswith("/*", j):
            return True
        k = skip_literal(s, j)
        j = k if k is not None else j + 1
    return False


SIMPLE = re.compile(r"^-?(\d+(\.\d+)?(_?f32)?|[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)*)$")
CALL = lambda name: re.compile(r"^(?:[A-Za-z_]\w*::)*" + name + r"\((.*)\)$", re.S)


def call_arg(expr, name):
    """`Name(arg)` -> arg when the parens wrap the whole expression."""
    m = CALL(name).match(expr)
    if not m:
        return None
    open_at = expr.index("(", len(expr) - len(m.group(1)) - 2)
    return m.group(1) if match_close(expr, open_at) == len(expr) - 1 else None


def norm(e):
    return re.sub(r"\s+", " ", e.strip())


def path_is(expr, typ, variant):
    return re.fullmatch(r"(?:[A-Za-z_]\w*::)*" + typ + "::" + variant, expr) is not None


def flags_expr(e):
    """Translate an i64 flag expression into a Flags expression, or None. Whitespace is preserved."""
    lead = e[: len(e) - len(e.lstrip())]
    trail = e[len(e.rstrip()):]
    e = e.strip()
    if e.startswith("if "):
        brace = None
        j = 3
        while j < len(e):
            k = skip_literal(e, j)
            if k is not None:
                j = k
                continue
            if e[j] in "([":
                j = match_close(e, j) + 1
                continue
            if e[j] == "{":
                brace = j
                break
            j += 1
        if brace is None:
            return None
        end = match_close(e, brace)
        rest = e[end + 1:]
        m = re.match(r"\s*else\s*", rest)
        if not m:
            return None
        tail = rest[m.end():]
        if tail.startswith("if "):
            b = flags_expr(tail)
        elif tail.startswith("{") and match_close(tail, 0) == len(tail) - 1:
            inner = flags_expr(tail[1:-1])
            b = None if inner is None else "{" + inner + "}"
        else:
            return None
        a = flags_expr(e[brace + 1:end])
        if a is None or b is None:
            return None
        return lead + e[: brace + 1] + a + "}" + rest[: m.end()] + b + trail
    terms = [t.strip() for t in split_top(e, "|")]
    if len(terms) == 1 and terms[0] == "0":
        return lead + "Flags::NONE" + trail
    out = []
    for t in terms:
        m = re.fullmatch(r"(?:(?:pxui::)?ui::)?([A-Z_]+)", t)
        if not m or m.group(1) not in FLAG_NAMES:
            return None
        out.append("Flags::" + m.group(1))
    return lead + " | ".join(out) + trail


def padding_calls(expr):
    """Builder calls equal to `padding: expr` over Edges::ZERO, or None."""
    arg = call_arg(expr, "Edges::all")
    if arg is not None:
        return [f".pad({arg.strip()})"]
    arg = call_arg(expr, "Edges::xy")
    if arg is not None:
        xs = split_top(arg)
        if len(xs) == 2:
            return [f".pad_x({xs[0].strip()})", f".pad_y({xs[1].strip()})"]
        return None
    m = re.match(r"^(?:[A-Za-z_]\w*::)*Edges\s*\{", expr)
    if not m or match_close(expr, m.end() - 1) != len(expr) - 1:
        return None
    inner = expr[m.end():-1]
    if has_comment(inner):
        return None
    edges, zero_tail = {}, False
    for part in split_top(inner):
        p = part.strip()
        if not p:
            continue
        if p.startswith(".."):
            if not path_is(p[2:].strip(), "Edges", "ZERO"):
                return None
            zero_tail = True
            continue
        name, _, val = p.partition(":")
        name = name.strip()
        if name not in "ltrb" or len(name) != 1:
            return None
        edges[name] = norm(val) if val else name
    if not zero_tail and len(edges) != 4:
        return None
    zero = lambda v: v is None or re.fullmatch(r"0(\.0*)?(_?f32)?", v) is not None
    l, r, t, b = (edges.get(k) for k in "lrtb")
    calls = []
    if not (zero(l) and zero(r)):
        if l != r or not SIMPLE.match(l):
            return None
        calls.append(f".pad_x({l})")
    if not (zero(t) and zero(b)):
        if t != b or not SIMPLE.match(t):
            return None
        calls.append(f".pad_y({t})")
    if not calls:
        return None  # all zero: same as default; leave verbatim
    if len(calls) == 2 and l == t:
        return [f".pad({l})"]
    return calls


def convert(body):
    """body: text between `Node {` and `}`. Returns (base_chain, kept_segments) or raises Skip."""
    segs = split_top(body)
    tail = segs[-1].strip()
    tm = re.fullmatch(r"\.\.\s*((?:[A-Za-z_]\w*::)*)Node::DEFAULT", tail)
    if not tm:
        raise Skip("tail is not ..Node::DEFAULT")
    prefix = tm.group(1)
    fields = []  # (name, expr, raw segment)
    for seg in segs[:-1]:
        s = re.sub(r"^(\s*//[^\n]*\n)+", "", seg).strip()
        if not s:
            raise Skip("empty field")
        name, colon, val = s.partition(":")
        name = name.strip()
        if not re.fullmatch(r"[a-z_][a-z0-9_]*", name) or (colon and val.startswith(":")):
            raise Skip("unparsed field " + s[:30])
        ind = re.search(r"(?:^|\n)([ ]*)[^\n]*$", seg[: len(seg) - len(seg.lstrip())] + "x").group(1) if "\n" in seg[: len(seg) - len(seg.lstrip())] else ""
        expr = val.strip() if colon else name
        if ind:
            expr = expr.replace("\n" + ind, "\n")  # continuation lines relative to the field
        fields.append((name, expr, seg))
    # a field carrying a comment stays verbatim in the literal
    by = {n: e for n, e, seg in fields if not has_comment(seg)}
    used = set()
    base = prefix + "Node::column()"
    if "axis" in by:
        for v, b in (("Row", "row"), ("Column", "column"), ("Stack", "stack")):
            if path_is(by["axis"], "Axis", v):
                base, _ = f"{prefix}Node::{b}()", used.add("axis")
    calls = []
    for dim in ("w", "h"):
        if dim in by:
            px = call_arg(by[dim], "Size::Px")
            calls.append(f".{dim}_px({px.strip()})" if px is not None else f".{dim}({by[dim]})")
            used.add(dim)
    for f in ("grow", "gap"):
        if f in by:
            calls.append(f".{f}({by[f]})")
            used.add(f)
    if "padding" in by:
        pc = padding_calls(by["padding"])
        if pc:
            calls += pc
            used.add("padding")
    if "justify" in by and "align" in by and path_is(by["justify"], "Justify", "Center") and path_is(by["align"], "Align", "Center"):
        calls.append(".center()")
        used |= {"justify", "align"}
    elif "align" in by:
        calls.append(f".align({by['align']})")
        used.add("align")
    if "role" in by:
        calls.append(f".role({by['role']})")
        used.add("role")
    if "flags" in by:
        fe = flags_expr(by["flags"])
        if fe is not None:
            calls.append(f".flags({fe})")
            used.add("flags")
    kept = [(n, seg) for n, _, seg in fields if n not in used]
    return base, calls, kept


def fmt(base, calls, indent, width):
    """One line when it fits and no argument spans lines; else one call per line at `indent`."""
    one = base + "".join(calls)
    if width + len(one) <= 120 and "\n" not in one:
        return one
    return base + "".join("\n" + indent + c.replace("\n", "\n" + indent) for c in calls)


SPLIT_CALLS = r"(?=\.(?:w|h|w_px|h_px|grow|gap|pad|pad_x|pad_y|center|align|role|flags)\()"


class Skip(Exception):
    pass


def find_literals(src):
    """Yield (start, brace, end) of every `Node {` ... `}` whose body has no inner `Node {`."""
    out = []
    for m in re.finditer(r"(?<![\w:])(?:[A-Za-z_]\w*::)*Node\s*\{", src):
        # ignore matches inside comments/strings: check by scanning from line start is costly; rely on context
        brace = m.end() - 1
        end = match_close(src, brace)
        out.append((m.start(), brace, end))
    return out


def in_code(src, pos):
    j = 0
    while j < pos:
        k = skip_literal(src, j)
        if k is not None:
            if k > pos:
                return False
            j = k
        else:
            j += 1
    return True


def rewrite(src, path, report):
    lits = [l for l in find_literals(src) if in_code(src, l[0])]
    # innermost first: drop those containing another
    lits = [l for l in lits if not any(o is not l and l[0] < o[0] and o[2] < l[2] for o in lits)]
    for start, brace, end in sorted(lits, reverse=True):
        body = src[brace + 1:end]
        line = src.count("\n", 0, start) + 1
        if "Node::DEFAULT" not in body:
            continue
        try:
            base, calls, kept = convert(body)
        except Skip as e:
            report.append(f"{path}:{line}: skipped: {e}")
            continue
        if kept:
            segs = split_top(body)
            tail_seg = segs[-1]
            tail_indent = re.search(r"\n([ ]*)\.\.", tail_seg)
            if tail_indent:
                chain = fmt(base, calls, tail_indent.group(1) + "    ", len(tail_indent.group(1)) + 2)
            else:
                chain = base + "".join(calls)
            new_body = "".join(seg + "," for _, seg in kept) + re.sub(r"(?:[A-Za-z_]\w*::)*Node::DEFAULT", lambda _: chain, tail_seg)
            new = src[start:brace] + "{" + new_body + "}"
        else:
            line_start = src.rfind("\n", 0, start) + 1
            indent = re.match(r"[ ]*", src[line_start:]).group(0)
            # a one-line literal stays one line; a multi-line one wraps past 120 columns
            width = start - line_start + 2 if "\n" in body else 0
            new = fmt(base, calls, indent + "    ", width) if width else base + "".join(calls)
        src = src[:start] + ("\x00" if not kept else "") + new + src[end + 1:]
    return collapse(src)


NODE_CALL = re.compile(
    r"(?P<lead>[^\n]*?)(?P<call>\b(?:ui\.node|ui\.node_keyed)\()\n(?P<i1>[ ]+)(?P<key>[^\n]+),\n[ ]+\x00(?P<chain>(?:pxui::ui::)?Node::(?:row|column|stack)\(\)[^\n]*?),?\n(?P<i0>[ ]*)\)"
)


def collapse(src):
    """`ui.node(\n key,\n Node::row()...,\n)` -> one line when it fits in 120 columns."""
    def one(m):
        line = m.group("lead") + m.group("call") + m.group("key").strip() + ", " + m.group("chain") + ")"
        if len(line) > 120 or m.group("key").strip().endswith(("(", "{")):
            return m.group(0)
        return line
    return NODE_CALL.sub(one, src).replace("\x00", "")


def main():
    report = []
    for path in sys.argv[1:]:
        src = open(path).read()
        out = rewrite(src, path, report)
        if out != src:
            open(path, "w").write(out)
    print("\n".join(report))


if __name__ == "__main__":
    main()
