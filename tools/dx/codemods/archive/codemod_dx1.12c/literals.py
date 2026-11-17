#!/usr/bin/env python3
"""DX1.12C codemod 4: struct literals of Node / Look / Font / Rgba -> const builders / Rgba::rgb(a).
`Node { w: Size::Px(10.0), look: L, ..Node::DEFAULT }` -> `Node::DEFAULT.w_px(10.0).look(L)`; innermost literals first.
A literal with a field that has no builder (state, transition, a lone border_w ...) is left as written and reported.
Usage: literals.py FILE..."""
import re, sys

TYPES = ('Node', 'Look', 'Font', 'Rgba')
START = re.compile(r'(?<![A-Za-z0-9_])((?:[a-z_]+::)*)(Node|Look|Font|Rgba)\s*\{')
SIMPLE = {
    'Node': {'flags', 'role', 'padding', 'margin', 'gap', 'grow', 'max_w', 'min_w', 'min_h', 'max_h', 'inset', 'look', 'axis', 'align',
             'justify', 'row_gap', 'cell', 'z', 'shrink', 'aspect', 'scroll_step', 'a11y', 'mark'},
    'Look': {'fill', 'text', 'font', 'opacity', 'ring', 'selection', 'caret', 'angle', 'sides', 'shadows'},
    'Font': {'size', 'weight', 'family', 'tracking'},
}

def split_top(s):
    parts, depth, start = [], 0, 0
    for i, c in enumerate(s):
        if c in '([{': depth += 1
        elif c in ')]}': depth -= 1
        elif c == ',' and depth == 0: parts.append(s[start:i]); start = i + 1
    parts.append(s[start:])
    return [p.strip() for p in parts if p.strip()]

def some_into(v):
    m = re.fullmatch(r'Some\((.*)\)\.into\(\)', v, re.S)
    return m.group(1) if m else None

def convert(prefix, ty, body):
    parts = split_top(body)
    base, fields = None, []
    for p in parts:
        if p.startswith('..'): base = p[2:].strip()
        else:
            if ':' in p and not p.startswith('::'):
                k, v = p.split(':', 1)
                if '::' in k + ':' + v[:1]: k, v = p.split(':', 1)
                fields.append((k.strip(), v.strip()))
            elif re.fullmatch(r'[a-z_][a-z0-9_]*', p): fields.append((p, p))
            else: return None, 'odd field %r' % p
    if ty == 'Rgba':
        ch = dict(fields)
        if any(k not in 'rgba' for k in ch) or (len(ch) < 4 and base != 'Default::default()'): return None, 'rgba fields'
        r, g, b, a = (ch.get(k, '0') for k in 'rgba')
        return (prefix + ('Rgba::rgb(%s, %s, %s)' % (r, g, b) if a == '255' else 'Rgba::rgba(%s, %s, %s, %s)' % (r, g, b, a))), None
    if base is None or base == 'Default::default()': return None, 'no base'
    recv = base if re.fullmatch(r'[A-Za-z_][A-Za-z0-9_:]*(\([^()]*\))?(\.[a-z_]+)*', base) else '(%s)' % base
    calls, d = [], dict(fields)
    if ty == 'Look' and ('border' in d) != ('border_w' in d): return None, 'border without border_w'
    for k, v in fields:
        if ty == 'Node' and k in ('w', 'h'):
            m = re.fullmatch(r'(?:[a-z_]+::)*Size::Px\((.*)\)', v, re.S)
            calls.append('%s_px(%s)' % (k, m.group(1)) if m else '%s(%s)' % (k, v))
        elif k in SIMPLE.get(ty, ()): calls.append('%s(%s)' % (k, v))
        elif ty == 'Node' and k == 'at':
            m = re.fullmatch(r'Some\(\((.*)\)\)\.into\(\)', v, re.S)
            if not m: return None, 'at %r' % v
            calls.append('at(%s)' % m.group(1))
        elif ty == 'Node' and k in ('grid', 'anchor', 'align_self', 'bar'):
            x = some_into(v)
            if x is None: return None, '%s %r' % (k, v)
            calls.append('%s(%s)' % (k, x))
        elif (ty, k) in (('Node', 'wrap'), ('Look', 'dashed'), ('Font', 'caps')):
            if v != 'true': return None, '%s %r' % (k, v)
            calls.append('%s()' % k)
        elif ty == 'Look' and k == 'radius':
            m = re.fullmatch(r'\[(.*);\s*4\]', v, re.S)
            calls.append('radius(%s)' % m.group(1) if m else 'radii(%s)' % v)
        elif ty == 'Look' and k == 'border': calls.append('border(%s, %s)' % (v, d['border_w']))
        elif ty == 'Look' and k == 'border_w': pass
        elif ty == 'Look' and k == 'gradient':
            x = some_into(v)
            if x is None: return None, 'gradient %r' % v
            calls.append('gradient(%s)' % x)
        else: return None, 'no builder for %s.%s' % (ty, k)
    return recv + ''.join('.' + c for c in calls), None

def run(path):
    s = open(path).read(); skipped = {}
    while True:
        changed = False
        for m in list(START.finditer(s))[::-1]:
            open_at = m.end() - 1
            depth, i = 0, open_at
            while i < len(s):
                if s[i] == '{': depth += 1
                elif s[i] == '}':
                    depth -= 1
                    if depth == 0: break
                i += 1
            body = s[open_at + 1:i]
            if '{' in body: continue
            before = s[max(0, m.start() - 12):m.start()]
            if re.search(r'(struct|impl|enum|let|fn|for)\s+$', before) or re.search(r'\bmatch\b[^{]*$', s[max(0, m.start() - 60):m.start()]): continue
            out, why = convert(m.group(1), m.group(2), body)
            if out is None:
                skipped[m.start()] = why; continue
            s = s[:m.start()] + out + s[i + 1:]; changed = True
            break
        if not changed: break
    open(path, 'w').write(s)
    for at, why in sorted(skipped.items()):
        line = s[:at].count('\n') + 1
        print('  SKIP %s:%d (%s)' % (path, line, why))

for f in sys.argv[1:]: run(f)
