#!/usr/bin/env python3
"""DX1.12C codemod 3: free-function geometry twins -> methods. `contains(R, P)` -> `R.contains(P)`, `intersect(A, B)` -> `A.intersect(B)`
(only in files given), `Ui::key_of(` / `pxui::Ui::key_of(` -> `pxui::ui::key_of(`.  Usage: geom_twins.py FILE..."""
import re, sys
def args(s, i):
    depth, parts, start = 0, [], i
    while i < len(s):
        c = s[i]
        if c in '([{': depth += 1
        elif c in ')]}':
            if depth == 0: parts.append(s[start:i]); return parts, i
            depth -= 1
        elif c == ',' and depth == 0: parts.append(s[start:i]); start = i + 1
        i += 1
    return None, None
def methods(s):
    out, i = [], 0
    pat = re.compile(r'(?<![A-Za-z0-9_.:])(contains|intersect)\(')
    while True:
        m = pat.search(s, i)
        if not m: out.append(s[i:]); return ''.join(out)
        a, end = args(s, m.end())
        if not a or len(a) != 2 or s[max(0, m.start() - 3):m.start()] == 'fn ':
            out.append(s[i:m.end()]); i = m.end(); continue
        recv = a[0].strip()
        if not re.fullmatch(r'[A-Za-z_*][A-Za-z0-9_.]*(\([^()]*\))?', recv) or recv.startswith('*'): recv = '(%s)' % recv
        out.append(s[i:m.start()] + '%s.%s(%s)' % (recv, m.group(1), a[1].strip())); i = end + 1
for f in sys.argv[1:]:
    s0 = open(f).read(); s = s0
    if '/lw-' not in f and 'pxui_types/src/ui.rs' not in f: s = methods(s)
    s = re.sub(r'(?<![A-Za-z0-9_:])(?:pxui::)?Ui::key_of\(', 'pxui::ui::key_of(', s)
    if s != s0: open(f, 'w').write(s); print('changed', f)
