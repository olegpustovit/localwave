#!/usr/bin/env python3
"""DX1.12C codemod 2: pxui_types::theme free functions over `&Tokens` -> `Tokens` methods/consts.
`theme::NAME(&T)` / `NAME(&T)` / `NAME(T)` -> `T.NAME()` (muted -> ink_2, dark -> is_dark); `theme::default()` -> `Tokens::DEFAULT`.
Usage: theme.py ROOT...   Prints changed files and leftovers (`invalid()`, imports) for hand edits."""
import re, sys, pathlib
NAMES = ['ground', 'ink_2', 'ink_3', 'muted', 'border', 'edge', 'faint_border', 'hover_fill', 'pressed_fill', 'tint', 'dark']
RENAME = {'muted': 'ink_2', 'dark': 'is_dark'}

def arg_end(s, i):
    depth = 0
    while i < len(s):
        c = s[i]
        if c in '([{': depth += 1
        elif c in ')]}':
            if depth == 0: return i
            depth -= 1
        elif c == ',' and depth == 0: return None
        i += 1
    return None

def rewrite(s, imported):
    out, i = [], 0
    pat = re.compile(r'(?<![A-Za-z0-9_.])(?:(?:pxui_types::|pxui::|crate::|pxui_core::)?theme::)?(%s)\(' % '|'.join(NAMES))
    while True:
        m = pat.search(s, i)
        if not m: out.append(s[i:]); break
        qualified = 'theme::' in m.group(0)
        name = m.group(1)
        start_arg = m.end()
        end = arg_end(s, start_arg)
        prev = s[max(0, m.start() - 3):m.start()]
        if end is None or (not qualified and name not in imported) or prev.endswith('fn '):
            out.append(s[i:m.end()]); i = m.end(); continue
        arg = s[start_arg:end].strip()
        if not arg: out.append(s[i:m.end()]); i = m.end(); continue
        if arg.startswith('&'): arg = arg[1:].strip()
        if not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_.\[\]()]*', arg) or arg.endswith('()') and '(' in arg[:-2]: arg = '(%s)' % arg
        out.append(s[i:m.start()]); out.append('%s.%s()' % (arg, RENAME.get(name, name))); i = end + 1
    return ''.join(out)

def imported_names(s):
    names = set()
    for m in re.finditer(r'theme::\{([^}]*)\}', s):
        names |= {x.strip() for x in m.group(1).split(',')}
    for m in re.finditer(r'theme::([a-z_0-9]+);', s): names.add(m.group(1))
    return names & set(NAMES)

def run(p):
    s0 = s = p.read_text()
    if 'pxui_types/src/theme.rs' in str(p): return
    imp = imported_names(s)
    s = re.sub(r'(?<![A-Za-z0-9_])((?:pxui_types::|pxui::|crate::|pxui_core::)?theme::)default\(\)', lambda m: (m.group(1) if m.group(1) != 'theme::' else '') + 'Tokens::DEFAULT', s)
    s = rewrite(s, imp)
    # drop the free-function names from `theme::{..}` imports
    def braces(m):
        items = [x.strip() for x in m.group(1).split(',') if x.strip()]
        items = [x for x in items if x not in NAMES and x != 'self' or x == 'self' and re.search(r'\btheme::', s.replace(m.group(0), ''))]
        return 'theme::{' + ', '.join(items) + '}' if items else None
    def b2(m):
        r = braces(m)
        return r if r else 'theme::{}'
    s = re.sub(r'theme::\{([^}]*)\}', b2, s)
    if s != s0: p.write_text(s); print('changed', p)
    for i, line in enumerate(s.splitlines(), 1):
        if re.search(r'theme::\{\}|\binvalid\(\)|Tokens::DEFAULT', line) and 'Tokens::DEFAULT' not in s0.splitlines()[i-1] if i <= len(s0.splitlines()) else True:
            print('  CHECK %s:%d: %s' % (p, i, line.strip()[:170]))

for root in sys.argv[1:]:
    for p in sorted(pathlib.Path(root).rglob('*.rs')):
        if '/target' in str(p): continue
        run(p)
