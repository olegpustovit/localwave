#!/usr/bin/env python3
"""Migrate U1b.6 call shapes (D25) in pxui and LocalWave, code and rustdoc examples.

- `ui.row/column/stack(key, node, f)` -> `ui.row/column/stack(node, f)` (keyless: the n-th container of its scope). A call under
  `if`/`else`/`match`/`for`/`while`/`loop`, or whose closure animates (`animate`), keeps its identity in `ui.id(key, ..)`:
  as a statement `ui.row(K, N, F);` -> `ui.id(K, |ui| { ui.row(N, F); });`. Such a call in an expression position is printed
  and left alone (it no longer compiles), for a hand edit. Every wrapped site is printed for review.
- `button_with` -> `button`, `toggle_with` -> `toggle`.
- Short forms: `ui.button(t)` -> `ui.button(K, t, &Button::DEFAULT).clicked`, `ui.toggle(t, on)` ->
  `ui.toggle(K, t, on, &Toggle::DEFAULT).changed != on` (the new value), K being the assigned variable's name when the call is
  assigned (`let save = ..`, `on = ..`, `x |= ..`), else "button"/"toggle" (the key the short form used); never the text.
- `.enabled(x)` -> `.disabled(!x)` (a `true`/`false` literal is inverted; a compound `x` is parenthesized).
Only calls on a receiver named `ui` are container/short-form calls; `.enabled(` is rewritten on any receiver. The engine's own
tests (crates/private/pxui_core) get only the `.enabled(` rule: the engine keeps its own button/toggle calls.
"""
import argparse
from pathlib import Path
import re

from namespace import DOC_CODE
from rs import args, close, line_of, mask

CONTAINER = re.compile(r"\bui\.(row|column|stack)\(")
RENAME = re.compile(r"\.(button|toggle)_with\(")
SHORT = re.compile(r"\bui\.(button|toggle)\(")
ENABLED = re.compile(r"\.enabled\(")
BRANCH = re.compile(r"(?:\b(?:if|else|match|for|while|loop)\b[^{};]*|=>\s*)$")


def enclosing_branch(m, at):
    """Whether a block opened by if/else/match/for/while/loop (or a match arm) encloses `at` inside its fn body."""
    depth, i = 0, at - 1
    while i >= 0:
        c = m[i]
        if c in ")]}":
            depth += 1
        elif c in "([{":
            if depth:
                depth -= 1
            elif c == "{":
                head = m[max(0, i - 200):i]
                head = head[head.rfind("\n", 0, len(head) - 1) + 1:] if "\n" in head else head
                if re.search(r"\bfn\b", head):
                    return False
                if BRANCH.search(head.rstrip()) or re.search(r"\b(?:if|else|match|for|while|loop)\b", head):
                    return True
        i -= 1
    return False


def arm_or_branch(m, at):
    # a match arm without braces: `X => ui.row(..)`
    before = m[:at].rstrip()
    return before.endswith("=>") or enclosing_branch(m, at)


def statement_bounds(m, start, end):
    """(is a statement, ends with ';') for the call m[start:end]."""
    before = m[:start].rstrip()
    after = m[end:].lstrip()
    starts = before.endswith((";", "{", "}"))
    return starts and after[:1] in (";", "}"), after[:1] == ";"


def negate(x):
    x = x.strip()
    if x in ("true", "false"):
        return "false" if x == "true" else "true"
    if x.startswith("!") and re.fullmatch(r"![\w.()]+", x):
        return x[1:]
    if re.fullmatch(r"[\w.]+(\([^()]*\))?", x):
        return "!" + x
    return f"!({x})"


def assigned_name(source, at):
    line = source[source.rfind("\n", 0, at) + 1:at]
    m = re.search(r"(?:let\s+(?:mut\s+)?)?([a-z_][a-z0-9_]*)\s*(?:\|=|&=|=)\s*$", line)
    return m[1] if m and m[1] not in ("let", "mut") else None


def rewrite(source, prefix, enabled_only=False):
    m = mask(source)
    edits, report = [], []
    for match in ([] if enabled_only else CONTAINER.finditer(m)):
        open_ = match.end() - 1
        end = close(m, open_)
        parts = args(m, open_, end)
        if len(parts) < 3:
            continue
        (ka, kb), (na, _) = parts[0], parts[1]
        key = source[ka:kb].strip()
        body = m[parts[2][0]:end]
        wrap = arm_or_branch(m, match.start()) or "animate" in body
        line = line_of(source, match.start())
        # the node argument's first character: everything before it (`key, `) goes
        node_start = na + len(source[na:parts[1][1]]) - len(source[na:parts[1][1]].lstrip())
        if not wrap:
            edits.append((open_ + 1, node_start, ""))
            continue
        statement, _ = statement_bounds(m, match.start(), end + 1)
        if not statement:
            report.append((line, f"conditional/animated {match[1]} in an expression: hand edit (key {key})"))
            continue
        # two edits around the call (never one over it), so edits inside its arguments still apply
        edits.append((match.start(), node_start, f"ui.id({key}, |ui| {{ {source[match.start():open_ + 1]}"))
        edits.append((end + 1, end + 1, "; })"))
        report.append((line, f"wrapped {match[1]} in ui.id({key}, ..)"))
    for match in ([] if enabled_only else RENAME.finditer(m)):
        edits.append((match.start() + 1, match.end() - 1, match[1]))
    for match in ([] if enabled_only else SHORT.finditer(m)):
        name, open_ = match[1], match.end() - 1
        end = close(m, open_)
        parts = args(m, open_, end)
        if (name, len(parts)) not in (("button", 1), ("toggle", 2)):
            continue
        key = assigned_name(source, match.start()) or name
        text = source[parts[0][0]:parts[0][1]].strip()
        if name == "button":
            new = f'ui.button("{key}", {text}, &{prefix}::Button::DEFAULT).clicked'
        else:
            on = source[parts[1][0]:parts[1][1]].strip()
            new = f'ui.toggle("{key}", {text}, {on}, &{prefix}::Toggle::DEFAULT).changed != {on}'
        edits.append((match.start(), end + 1, new))
    for match in ENABLED.finditer(m):
        open_ = match.end() - 1
        end = close(m, open_)
        x = source[open_ + 1:end]
        edits.append((match.start() + 1, end + 1, f"disabled({negate(x)})"))
    return sorted(edits), report


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
        engine = "crates/private/pxui_core" in str(path)
        edits, report = rewrite(source, prefix, engine)
        count = len(edits)
        for block in DOC_CODE.finditer(source):
            code = re.sub(r"^[ \t]*//[!/] ?", "", block[2], flags=re.M)
            doc_edits, doc_report = rewrite(code, "pxui", engine)
            report.extend((line_of(source, block.start(2)) + n - 1, what) for n, what in doc_report)
            if doc_edits:
                edited = apply(code, doc_edits)
                edits.append((block.start(2), block.end(2), "".join(block[1] + line for line in edited.splitlines(keepends=True))))
                count += len(doc_edits)
        for n, what in sorted(report):
            print(f"{path}:{n}: {what}")
        if count:
            print(f"{path}: {count}")
            total += count
            if not options.dry_run:
                path.write_text(apply(source, sorted(edits)))
    print(f"total {total} edits")


if __name__ == "__main__":
    src = ('fn f(ui: &mut Ui) {\n    ui.row("bar", N, |ui| {});\n    if x { ui.column(id, Node::DEFAULT, |ui| { ui.label("a", L); }); }\n'
           '    for i in 0..3 { let h = ui.row(i, N, |_| {}); }\n    ui.stack("s", N, |ui| { ui.animate(&h, &A); });\n'
           '    let save = ui.button("Save"); on = ui.toggle("Night", on); ui.button_with("k", "t", &B);\n'
           '    let b = B.enabled(false).x(); let c = C.enabled(ok); let d = D.enabled(a && b); item.enabled(item.enabled);\n}\n')
    out, rep = rewrite(src, "pxui")
    out = apply(src, out)
    assert 'ui.row(N, |ui| {});' in out, out
    assert 'if x { ui.id(id, |ui| { ui.column(Node::DEFAULT, |ui| { ui.label("a", L); }); }); }' in out, out
    assert 'let h = ui.row(i, N, |_| {});' in out and any("expression" in r for _, r in rep), out
    assert 'ui.id("s", |ui| { ui.stack(N, |ui| { ui.animate(&h, &A); }); });' in out, out
    nest = 'fn g(ui: &mut Ui) {\n    if a { ui.row("o", N, |ui| { ui.row("i", M, |_| {}); }); }\n}'
    assert apply(nest, rewrite(nest, "pxui")[0]) == 'fn g(ui: &mut Ui) {\n    if a { ui.id("o", |ui| { ui.row(N, |ui| { ui.id("i", |ui| { ui.row(M, |_| {}); }); }); }); }\n}', apply(nest, rewrite(nest, "pxui")[0])
    assert 'let save = ui.button("save", "Save", &pxui::Button::DEFAULT).clicked;' in out, out
    assert 'on = ui.toggle("on", "Night", on, &pxui::Toggle::DEFAULT).changed != on;' in out and 'ui.button("k", "t", &B)' in out, out
    assert 'B.disabled(true).x()' in out and 'C.disabled(!ok)' in out and 'D.disabled(!(a && b))' in out, out
    assert 'item.disabled(!item.enabled)' in out, out
    main()
