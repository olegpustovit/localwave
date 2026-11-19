#!/usr/bin/env python3
"""Migrate U1b.4 looks, views and accessibility types (pxui wrapper callers and LocalWave, code and rustdoc examples).

- `ui.set_look(&h, LOOK, STATE)` -> `ui.style(&h, &theme::States::of(LOOK), STATE)` (one look for every state);
- `ui.set_role(&h, ROLE, STATES)`: `0` -> `A11yStates::NONE`; `state::X` -> `A11yStates::X`; `a | b` -> `a.with(b)`;
  an `else { 0 }` branch -> `A11yStates::NONE`;
- any other `pxui::ui::state::X` (reading published `A11yNode::states` bits) -> `A11yStates::X.bits()`;
- `ui.view_clip/view_backdrop_blur/view_backdrop_saturation(b, v)` -> `ui.set_view(b, &ViewTransform::DEFAULT.clip/blur/saturation(v))`;
- `ui.a11y_tree().0/.1/.2` -> `.strings/.revision/.first_child`; `paint.theme()` -> `paint.tokens()`.
No scope analysis: the compiler flags misses (state constants imported by name are reported).
"""
import argparse
from pathlib import Path
import re

from namespace import DOC_CODE
from rs import args, close, line_of, mask

SET_LOOK = re.compile(r"\.set_look\(")
SET_ROLE = re.compile(r"\.set_role\(")
STATE = re.compile(r"\b(?:crate::ui::|pxui::ui::|ui::)?state::([A-Z_]+)\b")
VIEW = re.compile(r"\.view_(clip|backdrop_blur|backdrop_saturation)\(")
TREE = re.compile(r"\.a11y_tree\(\)\.([012])\b")
TREE_LET = re.compile(r"let\s+\(\s*(\w+)\s*,\s*(\w+)\s*,\s*(\w+)\s*\)\s*=\s*([\w.]+)\.a11y_tree\(\);")
IMPORTED = re.compile(r"use\s+pxui::ui::state::")


def top_split(m, text, sep="|"):
    """Offsets of top-level single `sep` characters in text (m its mask)."""
    out, depth = [], 0
    for i, c in enumerate(m):
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif c == sep and depth == 0 and m[i - 1:i] != sep and m[i + 1:i + 2] != sep:
            out.append(i)
    return out


def states_arg(text, path):
    """The `A11yStates` form of a 1.x `u32` states expression."""
    m = mask(text)
    if text.strip() == "0":
        return f"{path}::NONE"
    text = STATE.sub(lambda s: f"{path}::{s[1]}", text)
    text = re.sub(r"else\s*\{\s*0\s*\}", f"else {{ {path}::NONE }}", text)
    text = re.sub(r"if\s+(.+?)\s*\{\s*0\s*\}", lambda c: f"if {c[1]} {{ {path}::NONE }}", text)
    m = mask(text)
    cuts = top_split(m, text)
    if not cuts:
        return text.strip()
    parts = [text[a:b].strip() for a, b in zip([0] + [c + 1 for c in cuts], cuts + [len(text)])]
    return parts[0] + "".join(f".with({p})" for p in parts[1:])


def rewrite(source, prefix):
    m = mask(source)
    states, theme, view = f"{prefix}::A11yStates", f"{prefix}::theme::States", f"{prefix}::ViewTransform"
    edits, skipped, taken = [], [], []
    for match in SET_LOOK.finditer(m):
        open_ = match.end() - 1
        parts = args(m, open_, close(m, open_))
        if len(parts) == 3:
            a, b = parts[1]
            look = source[a:b].strip()
            edits.append((match.start() + 1, open_, "style"))
            lead = a + len(source[a:b]) - len(source[a:b].lstrip())
            edits.append((lead, lead + len(look), f"&{theme}::of({look})"))
    for match in SET_ROLE.finditer(m):
        open_ = match.end() - 1
        parts = args(m, open_, close(m, open_))
        if len(parts) == 3:
            a, b = parts[2]
            text = source[a:b]
            if "A11yStates" in text:
                continue
            lead = a + len(text) - len(text.lstrip())
            edits.append((lead, b - (len(text) - len(text.rstrip())), states_arg(text.strip(), states)))
            taken.append((a, b))
    for match in STATE.finditer(m):
        if not match.group(0).startswith(("crate::ui::state", "pxui::ui::state", "ui::state")) or any(a <= match.start() < b for a, b in taken):
            continue
        edits.append((match.start(), match.end(), f"{states}::{match[1]}.bits()"))
    for match in VIEW.finditer(m):
        open_ = match.end() - 1
        parts = args(m, open_, close(m, open_))
        if len(parts) == 2:
            a, b = parts[1]
            field = {"clip": "clip", "backdrop_blur": "blur", "backdrop_saturation": "saturation"}[match[1]]
            edits.append((match.start() + 1, open_, "set_view"))
            lead = a + len(source[a:b]) - len(source[a:b].lstrip())
            edits.append((lead, lead + len(source[a:b].strip()), f"&{view}::DEFAULT.{field}({source[a:b].strip()})"))
    for match in TREE.finditer(m):
        edits.append((match.start(1), match.end(1), ["strings", "revision", "first_child"][int(match[1])]))
    for match in TREE_LET.finditer(m):
        names = [f"{field}: {var}" if var != field else field for field, var in zip(("strings", "revision", "first_child"), match.groups()[:3]) if var != "_"]
        edits.append((match.start(), match.end(), f"let {prefix}::A11yTree {{ {', '.join(names)}{', ..' if len(names) < 3 else ''} }} = {match[4]}.a11y_tree();"))
    for match in re.finditer(r"\bpaint\.theme\(\)", m):
        edits.append((match.start() + len("paint."), match.end() - 2, "tokens"))
    for match in IMPORTED.finditer(m):
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
    src = ('fn f(ui: &mut Ui) { ui.set_look(&b, Look::NONE.fill(x), state); ui.set_role(&a, Role::Group, 0);\n'
           'ui.set_role(&c, Role::Toggle, pxui::ui::state::CHECKABLE | if on { pxui::ui::state::CHECKED } else { 0 });\n'
           'ui.set_role(&d, Role::Button, if enabled { pxui::ui::state::ENABLED } else { 0 }); let v = n.states & pxui::ui::state::VISIBLE != 0;\n'
           'ui.view_clip(&b, Some(r)); ui.view_backdrop_blur(&b, 4.0); let s = ui.a11y_tree().0; let (strings, revision, top) = ui.a11y_tree(); let t = paint.theme().accent; }')
    out = apply(src, rewrite(src, "pxui")[0])
    assert "ui.style(&b, &pxui::theme::States::of(Look::NONE.fill(x)), state)" in out, out
    assert "ui.set_role(&a, Role::Group, pxui::A11yStates::NONE)" in out, out
    assert "pxui::A11yStates::CHECKABLE.with(if on { pxui::A11yStates::CHECKED } else { pxui::A11yStates::NONE })" in out, out
    assert "if enabled { pxui::A11yStates::ENABLED } else { pxui::A11yStates::NONE }" in out, out
    assert "n.states & pxui::A11yStates::VISIBLE.bits() != 0" in out, out
    assert "ui.set_view(&b, &pxui::ViewTransform::DEFAULT.clip(Some(r)))" in out and "DEFAULT.blur(4.0)" in out, out
    assert "a11y_tree().strings" in out and "paint.tokens().accent" in out, out
    assert "let pxui::A11yTree { strings, revision, first_child: top } = ui.a11y_tree();" in out, out
    main()
