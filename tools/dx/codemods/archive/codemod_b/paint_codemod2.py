#!/usr/bin/env python3
"""DX1.12B review round: the remaining Paint shapes take Fill/Stroke values.
triangle(a,b,c,color)->Fill; curve(f,c,t,w,color)/cubic(pts,w,color)->Stroke(.width); circle(at,r,Option<Color>,Option<Color>) ->
Option<Fill>/Option<Stroke>; grid(rect,o,s,d,color)->Fill; chevron(at,dir,color)->Stroke; dashed_rect(rect,color)->Stroke.
Usage: paint_codemod2.py FILE..."""
import re, sys
sys.path.insert(0, __import__('os').path.dirname(__file__))
from paint_codemod import split_args, clean, RECEIVERS

def opt(a, ty):
    a = clean(a)
    if a == 'None': return a
    m = re.fullmatch(r'Some\((.*)\)', a)
    return f'Some({ty}::new({m.group(1)}))' if m else f'{a}.map({ty}::new)'
def stroke(color, width=None):
    return f'Stroke::new({clean(color)})' + (f'.width({clean(width)})' if width is not None else '')
RULES = {
    'triangle': (4, lambda a: [clean(x) for x in a[:3]] + [f'Fill::new({clean(a[3])})']),
    'curve':    (5, lambda a: [clean(x) for x in a[:3]] + [stroke(a[4], a[3])]),
    'cubic':    (3, lambda a: [clean(a[0]), stroke(a[2], a[1])]),
    'circle':   (4, lambda a: [clean(a[0]), clean(a[1]), opt(a[2], 'Fill'), opt(a[3], 'Stroke')]),
    'grid':     (5, lambda a: [clean(x) for x in a[:4]] + [f'Fill::new({clean(a[4])})']),
    'chevron':  (3, lambda a: [clean(a[0]), clean(a[1]), stroke(a[2])]),
    'dashed_rect': (2, lambda a: [clean(a[0]), stroke(a[1])]),
}
def transform(src):
    pat = re.compile(r'(?<![\w.])(' + RECEIVERS + r')\.(' + '|'.join(RULES) + r')\(')
    out, i, n = [], 0, 0
    while True:
        m = pat.search(src, i)
        if not m: out.append(src[i:]); break
        args, end = split_args(src, m.end())
        if args and not args[-1].strip(): args = args[:-1]
        k, f = RULES[m.group(2)]
        if len(args) != k or (m.group(2) in ('triangle', 'grid') and clean(args[-1]).startswith('Fill::')) or (m.group(2) == 'circle' and 'Fill::' in ''.join(args)):
            out.append(src[i:m.end()]); i = m.end(); continue
        out.append(src[i:m.start()] + f'{m.group(1)}.{m.group(2)}(' + ', '.join(f(args)) + ')'); i = end + 1; n += 1
    return ''.join(out), n
for path in sys.argv[1:]:
    s = open(path).read(); t, n = transform(s)
    if n: open(path, 'w').write(t); print(path, n)
