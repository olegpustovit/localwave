#!/usr/bin/env python3
"""DX1.12C codemod 1: pxui_types::geom::color free functions and the `Color`/`color::T` aliases -> `Rgba` consts/methods.
Usage: color.py ROOT...  (rewrites *.rs in place; prints files changed and leftover `color::` sites for hand edits)."""
import re, sys, pathlib

SIMPLE = [
    (r'\bcolor::white\(\)', 'Rgba::WHITE'), (r'\bcolor::black\(\)', 'Rgba::BLACK'), (r'\bcolor::transparent\(\)', 'Rgba::CLEAR'),
    (r'\bcolor::T\b', 'Rgba'), (r'\bcolor::hex\(', 'Rgba::parse('),
]
LIT = r'\s*(?:0x[0-9a-fA-F_]+|[0-9_]+)\s*'
def lit_calls(s):
    # rgb/rgba/gray with integer literal args only: same values, now u8
    for name, n in (('rgb', 3), ('rgba', 4), ('gray', 1)):
        s = re.sub(r'\bcolor::%s\((%s)\)' % (name, ','.join([LIT] * n)), lambda m, name=name: 'Rgba::%s(%s)' % (name, m.group(1)), s)
    return s

def fix_use(s):
    # `geom::{color, X}` -> `geom::{Rgba, X}`; `geom::color` -> `geom::Rgba`; `Rgba as Color` -> `Rgba`
    s = re.sub(r'\bRgba as Color\b', 'Rgba', s)
    def braces(m):
        items = [i.strip() for i in m.group(2).split(',') if i.strip()]
        items = ['Rgba' if i in ('color', 'Color') else i for i in items]
        out = []
        for i in items:
            if i not in out: out.append(i)
        return m.group(1) + '{' + ', '.join(out) + '}'
    s = re.sub(r'(geom::)\{([^{}]*)\}', braces, s)
    s = re.sub(r'(use [a-z_:]*geom::)(color|Color);', r'\1Rgba;', s)
    return s

def run(path):
    s0 = s = path.read_text()
    if 'color::' not in s and not re.search(r'\bColor\b', s): return
    had_alias = bool(re.search(r'Rgba as Color|geom::\{[^}]*\bColor\b|geom::Color\b', s))
    for a, b in SIMPLE: s = re.sub(a, b, s)
    s = lit_calls(s)
    s = fix_use(s)
    if had_alias: s = re.sub(r'(?<![A-Za-z_"])Color\b(?!")', 'Rgba', s)
    s = re.sub(r'geom::\{Rgba, Rgba\}', 'geom::Rgba', s)
    if s != s0:
        path.write_text(s); print('changed', path)
    for i, line in enumerate(s.splitlines(), 1):
        if re.search(r'\bcolor::', line): print('  LEFT %s:%d: %s' % (path, i, line.strip()[:160]))

for root in sys.argv[1:]:
    for p in sorted(pathlib.Path(root).rglob('*.rs')):
        if '/target' in str(p): continue
        run(p)
