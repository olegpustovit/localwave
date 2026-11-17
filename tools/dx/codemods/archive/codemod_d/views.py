#!/usr/bin/env python3
"""DX1.12D codemod 3: `ui.view(label, revision, w, h, build)` -> `ui.view_of(label, &revision, &View::DEFAULT[.w..][.h..], build)`.
Usage: views.py <view-path> <file>... ; <view-path> is how the file names the style (`View`, `crate::View`, `pxui::View`).
Skips calls on an engine `Ui` (lines naming `engine::Ui`) and calls whose label is not a string literal (printed for hand edits)."""
import re, sys
from boxes import balanced, split_top

def size(axis, expr):
    e = re.sub(r'\s+', ' ', expr)
    e2 = re.sub(r'^(pxui::ui::|pxui_types::|crate::ui::)', '', e)
    if e2 == 'Size::Grow': return ''
    m = re.fullmatch(r'Size::Px\((.*)\)', e2)
    if m: return f'.{axis}_px({m.group(1)})'
    return f'.{axis}({e})'

def rev(expr):
    e = re.sub(r'\s+', ' ', expr).strip()
    return '&' + e if re.fullmatch(r'[\w.]+', e) else f'&({e})'

path_ = sys.argv[1]
def once(path, s):
    out = ''; i = 0
    for m in re.finditer(r'\bui\s*\.view\(', s):
        if m.start() < i: continue   # nested in a call just rewritten: the next pass takes it
        line = s[s.rfind('\n', 0, m.start()) + 1:s.find('\n', m.start())]
        if 'engine::Ui' in line: continue
        start = m.end() - 1; end = balanced(s, start)
        args = split_top(s[start + 1:end - 1])
        if len(args) != 5: continue
        label, revision, w, h, build = args
        if not re.fullmatch(r'"[^"]*"', label.strip()):
            print(f'{path}:{s.count(chr(10), 0, m.start()) + 1}: non-literal label {label!r}', file=sys.stderr)
        build = re.sub(r'^&mut\s+(?=(move\s+)?\|)', '', build)
        style = f'&{path_}::DEFAULT{size("w", w)}{size("h", h)}'
        out += s[i:m.start()] + f'ui.view_of({label}, {rev(revision)}, {style}, {build})'
        i = end
    return out + s[i:]

for path in sys.argv[2:]:
    s = text = open(path).read()
    while (nxt := once(path, s)) != s: s = nxt
    if s != text: open(path, 'w').write(s); print(path)
