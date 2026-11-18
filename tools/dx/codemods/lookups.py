#!/usr/bin/env python3
"""Migrate U1b.2 keys and small returns (pxui and LocalWave, code and rustdoc examples).

- `ui.node(x.into(), n)` -> `ui.node(x, n)` (node is key-first generic);
- `.gesture(&h, OPTS)` -> `.gesture(&h, &OPTS)`;
- `.hovered_exactly(b)` -> `.hovered(b, false)`, `.hovered_within(b)` -> `.hovered(b, true)`;
- `.text_input_options()` -> `.text_traits()`;
- `.box_of_key(e)` / `.load_text_edit(e, ..)` with an i64 expression -> `Key::Resolved(e)` (a string literal or a `Key::`
  expression is left alone): an i64 would otherwise convert to `Key::Id` and name another node.
Run it on the wrapper's callers only: the engine (crates/private, and engine calls in abi_equal/abi_bench) keeps `node(Key, ..)` and
its i64 lookups. Calls that need a handle or a key the old call did
not have (`remember`, `part_box`/`part_rect` by owner key, `root_at`/`last_press_within` candidates, a keyless `image`,
`Paint::image` fit/tint, `ime()` tuples) are reported for hand edits.
"""
import argparse
from pathlib import Path
import re

from namespace import DOC_CODE
from rs import args, close, line_of, mask

NODE = re.compile(r"\.node\(")
GESTURE = re.compile(r"\.gesture\(")
HOVER = re.compile(r"\.hovered_(exactly|within)\(")
RESOLVE = re.compile(r"\.(box_of_key|load_text_edit)\(")
REPORT = re.compile(r"\.(remember|part_box|part_rect|root_at|last_press_within)\(|\.ime\(\)\s*\.map|\bpaint\.image\(")


def rewrite(source, key_path):
    m = mask(source)
    edits, skipped = [], []
    for match in NODE.finditer(m):
        open_ = match.end() - 1
        parts = args(m, open_, close(m, open_))
        if parts:
            a, b = parts[0]
            first = m[a:b].rstrip()
            if first.endswith(".into()") and first.strip() != ".into()":
                end = a + len(first)
                lead = a + len(m[a:b]) - len(m[a:b].lstrip())
                inner = end - len(".into()")
                if m[lead] == "(" and close(m, lead) == inner - 1:
                    # `(expr).into()` -> `expr`
                    edits.append((lead, lead + 1, ""))
                    edits.append((inner - 1, end, ""))
                else:
                    edits.append((inner, end, ""))
    for match in GESTURE.finditer(m):
        open_ = match.end() - 1
        parts = args(m, open_, close(m, open_))
        if len(parts) == 2:
            a, b = parts[1]
            lead = len(m[a:b]) - len(m[a:b].lstrip())
            if not m[a + lead:b].startswith("&"):
                edits.append((a + lead, a + lead, "&"))
    for match in HOVER.finditer(m):
        open_ = match.end() - 1
        end = close(m, open_)
        edits.append((match.start() + 1, open_, "hovered"))
        edits.append((end, end, ", " + ("false" if match[1] == "exactly" else "true")))
    for match in re.finditer(r"\.text_input_options\(\)", m):
        edits.append((match.start() + 1, match.start() + 1 + len("text_input_options"), "text_traits"))
    for match in RESOLVE.finditer(m):
        open_ = match.end() - 1
        parts = args(m, open_, close(m, open_))
        if not parts:
            continue
        a, b = parts[0]
        expr = source[a:b].strip()
        if expr.startswith('"') or re.match(r"(?:(?:crate|pxui|pxui_types::ui)::)?Key::", expr):
            continue
        lead = len(source[a:b]) - len(source[a:b].lstrip())
        edits.append((a + lead, a + lead + len(expr), f"{key_path}::Resolved({expr})"))
    for match in REPORT.finditer(m):
        skipped.append((line_of(source, match.start()), source[match.start():source.find("\n", match.start())].strip()))
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
        inside = "crates/public/pxui/src" in str(path)
        edits, skipped = rewrite(source, "crate::Key" if inside else "pxui::Key")
        count = len(edits)
        for block in DOC_CODE.finditer(source):
            code = re.sub(r"^[ \t]*//[!/] ?", "", block[2], flags=re.M)
            doc_edits, doc_skips = rewrite(code, "pxui::Key")
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
    src = ('fn f(ui: &mut Ui) { let a = ui.node("a".into(), N); ui.node((i as i64).into(), N); ui.node(id.into(), N); ui.node(k, N); ui.gesture(&h, OPTS); ui.gesture(&h, &O);\n'
           'ui.hovered_exactly(&b); ui.hovered_within(&b); ui.text_input_options(); ui.box_of_key(key_of(s, "x")); ui.box_of_key("x");\n'
           'ui.load_text_edit(b.box_key, "t"); ui.box_of_key(pxui::Key::Resolved(k)); }')
    out = apply(src, rewrite(src, "pxui::Key")[0])
    assert 'ui.node("a", N); ui.node(i as i64, N); ui.node(id, N); ui.node(k, N);' in out, out
    assert "ui.gesture(&h, &OPTS); ui.gesture(&h, &O);" in out, out
    assert "ui.hovered(&b, false); ui.hovered(&b, true); ui.text_traits();" in out, out
    assert 'ui.box_of_key(pxui::Key::Resolved(key_of(s, "x"))); ui.box_of_key("x");' in out, out
    assert 'ui.load_text_edit(pxui::Key::Resolved(b.box_key), "t"); ui.box_of_key(pxui::Key::Resolved(k));' in out, out
    main()
