#!/usr/bin/env python3
"""DX1.12B codemod: Paint geometry/style calls -> Rect + Fill/Stroke values; draw/draw_over -> canvas/canvas_over;
text_styled -> text; image/image_fit -> image(rect, img, fit, tint). Usage: paint_codemod.py [--dry] FILE..."""
import re, sys

RECEIVERS = r'(?:paint|p|\$p|painter|pa)'
def split_args(s, i):
    """s[i] is just after '('; returns (args, end) where s[end] == ')'."""
    depth, start, args, j = 0, i, [], i
    while j < len(s):
        c = s[j]
        if c in '([{': depth += 1
        elif c in ')]}':
            if depth == 0:
                args.append(s[start:j]); return args, j
            depth -= 1
        elif c == ',' and depth == 0:
            args.append(s[start:j]); start = j + 1
        elif c == '"':
            j += 1
            while s[j] != '"':
                if s[j] == '\\': j += 1
                j += 1
        elif c == "'" and j + 2 < len(s) and (s[j+2] == "'" or s[j+1] == '\\'):
            j = s.index("'", j + 2)
        j += 1
    raise ValueError('unbalanced')

def clean(a): return ' '.join(a.split())
def rect(x, y, w, h):
    x, y, w, h = map(clean, (x, y, w, h))
    m = re.fullmatch(r'(.+)\.x', x)
    if m and y == m.group(1)+'.y' and w == m.group(1)+'.w' and h == m.group(1)+'.h':
        return m.group(1)
    return f'Rect::new({x}, {y}, {w}, {h})'
def stroke(color, width=None):
    color = clean(color)
    return f'Stroke::new({color})' + (f'.width({clean(width)})' if width is not None else '')

RULES = {
    'fill':            (5, lambda a: ('fill', [rect(*a[:4]), f'Fill::new({clean(a[4])})'])),
    'fill_fractional': (5, lambda a: ('fill', [rect(*a[:4]), f'Fill::new({clean(a[4])}).fractional()'])),
    'stroke':          (5, lambda a: ('stroke', [rect(*a[:4]), stroke(a[4])])),
    'stroke_w':        (6, lambda a: ('stroke', [rect(*a[:4]), stroke(a[5], a[4])])),
    'frame':           (5, lambda a: ('frame', [rect(*a[:4]), stroke(a[4])])),
    'frame_w':         (6, lambda a: ('frame', [rect(*a[:4]), stroke(a[5], a[4])])),
    'line':            (3, lambda a: ('line', [clean(a[0]), clean(a[1]), stroke(a[2])])),
    'line_w':          (4, lambda a: ('line', [clean(a[0]), clean(a[1]), stroke(a[3], a[2])])),
    'dashed':          (3, lambda a: ('dashed', [clean(a[0]), clean(a[1]), stroke(a[2])])),
    'dashed_w':        (4, lambda a: ('dashed', [clean(a[0]), clean(a[1]), stroke(a[3], a[2])])),
    'dashed_rect':     (5, lambda a: ('dashed_rect', [rect(*a[:4]), clean(a[4])])),
    'brackets':        (8, lambda a: ('brackets', [rect(*a[:4]), clean(a[4]), clean(a[5]), stroke(a[7], a[6])])),
    'grid':            (8, lambda a: ('grid', [rect(*a[:4])] + [clean(x) for x in a[4:]])),
    'image_fit':       (7, lambda a: ('image', [rect(*a[:4])] + [clean(x) for x in a[4:]])),
    'image':           (5, lambda a: ('image', [rect(*a[:4]), clean(a[4]), 'Fit::Fill', 'Rgba::rgb(255, 255, 255)'])),
    'text_styled':     (4, lambda a: ('text', [clean(x) for x in a])),
}

def transform(src):
    out, i, changed = [], 0, 0
    pat = re.compile(r'\b' + RECEIVERS + r'\.(' + '|'.join(sorted(RULES, key=len, reverse=True)) + r')\(')
    pat = re.compile(r'(?<![\w.])(' + RECEIVERS + r')\.(' + '|'.join(sorted(RULES, key=len, reverse=True)) + r')\(')
    while True:
        m = pat.search(src, i)
        if not m: out.append(src[i:]); break
        name = m.group(2)
        try: args, end = split_args(src, m.end())
        except Exception: out.append(src[i:m.end()]); i = m.end(); continue
        args = [a for a in args if a.strip()] if args and not args[-1].strip() else args
        n, f = RULES[name]
        if len(args) != n:
            out.append(src[i:m.end()]); i = m.end(); continue
        new_name, new_args = f(args)
        # recurse into args (nested paint calls are rare; keep simple)
        out.append(src[i:m.start()] + f'{m.group(1)}.{new_name}(' + ', '.join(new_args) + ')')
        i = end + 1; changed += 1
    src = ''.join(out)
    # canvas twins (a Ui receiver taking a box reference first)
    src, n2 = re.subn(r'\.draw_over\(', '.canvas_over(', src)
    src, n3 = re.subn(r'\b(ui|u)\.draw\((?=&)', r'\1.canvas(', src)
    return src, changed + n2 + n3

if __name__ == '__main__':
    dry = '--dry' in sys.argv
    for path in [a for a in sys.argv[1:] if a != '--dry']:
        s = open(path).read()
        t, n = transform(s)
        if n:
            print(f'{path}: {n}')
            if not dry: open(path, 'w').write(t)
