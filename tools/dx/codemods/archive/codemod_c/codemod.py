#!/usr/bin/env python3
"""Codemod C: `let b = ui.node(KEY, NODE); ui.within(&b, &mut |ui| BODY);` -> `ui.row|column|stack(KEY, NODE, |ui| BODY);`

Usage: codemod.py [--dry] FILE...   Prints every site with CONVERT or SKIP(reason)."""
import re, sys, collections
sys.path.insert(0, __file__.rsplit("/", 2)[0] + "/codemod_a")
from codemod import skip_literal, match_close, split_top, has_comment

AXIS = {"row": "row", "column": "column", "stack": "stack"}
LET = re.compile(r"let (mut )?([a-z_][a-z0-9_]*) = ([a-z_][a-z0-9_]*)\.(node|node_keyed)\(")
LIMIT = 120


def axis_of(expr):
    e = expr.strip()
    m = re.match(r"Node::(row|column|stack)\(\)", e)
    if m:
        rest = e[m.end():]
        if re.search(r"\.\s*axis\b|\b(row|column|stack)\(\)", rest):
            return None, "axis changed in chain"
        return m.group(1), None
    if re.match(r"Node::DEFAULT\b", e):
        rest = e[len("Node::DEFAULT"):]
        if re.search(r"\baxis\b|\b(row|column|stack)\(\)", rest):
            return None, "axis changed in chain"
        return "column", None
    if e.startswith("Node {") or e.startswith("Node{"):
        o = e.index("{")
        c = match_close(e, o)
        if e[c + 1:].strip():
            return None, "literal followed by chain"
        fields = [f.strip() for f in split_top(e[o + 1:c]) if f.strip()]
        ax = None
        for f in fields:
            if f.startswith("axis:"):
                v = f[5:].strip()
                mm = re.fullmatch(r"Axis::(Row|Column|Stack)", v)
                if not mm:
                    return None, "axis field not constant"
                ax = mm.group(1).lower()
        base = [f for f in fields if f.startswith("..")]
        if ax:
            return ax, None
        if not base:
            return None, "literal without axis/base"
        return axis_of(base[0][2:])
    return None, "node expression axis unknown"


def word_in(s, w):
    j = 0
    while j < len(s):
        k = skip_literal(s, j)
        if k is not None:
            j = k
            continue
        if s.startswith(w, j) and (j == 0 or not (s[j - 1].isalnum() or s[j - 1] == "_")) and \
                (j + len(w) == len(s) or not (s[j + len(w)].isalnum() or s[j + len(w)] == "_")):
            return True
        j += 1
    return False


def block_end(s, i):
    depth = 0
    j = i
    while j < len(s):
        k = skip_literal(s, j)
        if k is not None:
            j = k
            continue
        if s[j] in "([{":
            depth += 1
        elif s[j] in ")]}":
            depth -= 1
            if depth < 0:
                return j
        j += 1
    return len(s)


def collapse(args):
    lines = [l.strip() for l in args.strip().split("\n")]
    out = ""
    for l in lines:
        if not l:
            continue
        if not out or l.startswith(".") or l.startswith(")") or l.startswith("?") or out.endswith("("):
            out += l
        else:
            out += " " + l
    return out.rstrip(",").strip()


def multiline_literal(s):
    j = 0
    while j < len(s):
        k = skip_literal(s, j)
        if k is not None:
            if s[j] != "/" and "\n" in s[j:k]:
                return True
            j = k
            continue
        j += 1
    return False


def reindent(text, extra):
    return "\n".join((extra + l if l.strip() else l) for l in text.split("\n"))


def process(path, dry, report):
    s = open(path).read()
    ns = transform(s, path, 0, report)
    if not dry and ns != s:
        open(path, "w").write(ns)


def transform(s, path, base, report):
    out, pos = [], 0
    for m in LET.finditer(s):
        if m.start() < pos:
            continue
        line_start = s.rfind("\n", 0, m.start()) + 1
        indent = s[line_start:m.start()]
        if indent.strip():
            continue
        lineno = base + s.count("\n", 0, m.start()) + 1
        site = f"{path}:{lineno}"
        name, recv, fn = m.group(2), m.group(3), m.group(4)
        po = m.end() - 1
        pc = match_close(s, po)
        if s[pc + 1] != ";":
            continue
        after = pc + 2
        wm = re.compile(r"\s*" + re.escape(recv) + r"\.within\(&" + name + r", ").match(s, after)
        if not wm:
            # within on this node elsewhere (statements between) or never
            if re.search(r"\.within\(&" + name + r",", s[after:block_end(s, after)]):
                report["skip: statements between node and within"].append(site)
            continue
        if fn == "node_keyed":
            report["skip: node_keyed (Key::Id hashes a label, node_keyed keys raw: identity differs)"].append(site); continue
        if m.group(1):
            report["skip: let mut"].append(site); continue
        args = s[po + 1:pc]
        parts = [p for p in split_top(args)]
        if parts and not parts[-1].strip():
            parts = parts[:-1]
        if len(parts) != 2:
            report["skip: unexpected args"].append(site); continue
        key, node = parts[0].strip(), parts[1].strip()
        if re.fullmatch(r'"[^"\\]*"', key):
            kind = "literal"
        elif re.fullmatch(r"[a-z_][a-z0-9_]*", key):
            kind = "ident"
        else:
            report["skip: key not a literal (runtime/formatted label)"].append(site); continue
        ax, why = axis_of(node)
        if not ax:
            report[f"skip: {why}"].append(site); continue
        wo = s.index("(", wm.end() - len(name) - 4)
        wc = match_close(s, wo)
        clos = s[wm.end():wc]
        if not clos.startswith("&mut |"):
            report["skip: within body not an inline closure"].append(site); continue
        clos = clos[len("&mut "):].rstrip()
        if clos.endswith(","):
            clos = clos[:-1]
        semi = s[wc + 1] == ";"
        end = wc + 2 if semi else wc + 1
        if word_in(clos, name) or word_in(s[end:block_end(s, end)], name):
            report["skip: node handle used after within"].append(site); continue
        clos = transform(clos, path, base + s.count("\n", 0, wm.end()), report)
        one = collapse(args) if "\n" in args else args.strip().rstrip(",")
        multi = "\n" in args
        if multi and (has_comment(args) or "{" in args):
            one = None
        head = f"{recv}.{ax}({one}, " if one is not None else None
        first_line = clos.split("\n")[0]
        if head is not None and len(indent) + len(head) + len(first_line) <= LIMIT or (head is not None and not multi):
            new = f"{head}{clos});"
        else:
            # rustfmt style: arguments one per line, closure as the last argument, body indented one level more
            arg_lines = args.rstrip()
            if not arg_lines.endswith(","):
                arg_lines += ","
            if not multi:
                inner = indent + "    "
                arg_lines = "\n" + inner + key + ",\n" + inner + node + ","
            body = reindent(clos, "    ")
            new = f"{recv}.{ax}({arg_lines}\n{indent}    {body.lstrip()},\n{indent});"
            if multiline_literal(clos):
                report["skip: multi-line string in body"].append(site); continue
        report[f"convert ({kind} key)"].append(site)
        out.append(s[pos:m.start()])
        out.append(new)
        pos = end
    out.append(s[pos:])
    return "".join(out)


if __name__ == "__main__":
    dry = "--dry" in sys.argv
    report = collections.defaultdict(list)
    for p in [a for a in sys.argv[1:] if a != "--dry"]:
        process(p, dry, report)
    for k in sorted(report):
        print(f"{len(report[k]):4d} {k}")
        for site in report[k]:
            print("       ", site)
