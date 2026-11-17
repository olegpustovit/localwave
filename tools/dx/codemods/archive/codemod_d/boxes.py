#!/usr/bin/env python3
"""DX1.12D codemod 2: `ui.box_(label, BoxOpts::default()...)` -> `ui.node(label.into(), Node::DEFAULT...)`.
Usage: boxes.py <file>... ; only rewrites calls whose options are a plain `BoxOpts::default()` builder chain."""
import re, sys
BUILDER = {'w': 'w', 'h': 'h', 'flags': 'flags', 'padding': 'pad'}   # Node const builders that exist
FIELD = {'at': lambda a: f'at: Some({a}).into()', 'scroll_step': lambda a: f'scroll_step: {a}', 'max_h': lambda a: f'max_h: {a}', 'axis': lambda a: f'axis: {a}'}

def balanced(s, i):
    """s[i] == '(' -> index after the matching ')'"""
    depth = 0
    for j in range(i, len(s)):
        if s[j] == '(': depth += 1
        elif s[j] == ')':
            depth -= 1
            if depth == 0: return j + 1
    raise ValueError

def split_top(s):
    parts, depth, cur = [], 0, ''
    for c in s:
        if c in '([{': depth += 1
        if c in ')]}': depth -= 1
        if c == ',' and depth == 0: parts.append(cur); cur = ''
        else: cur += c
    if cur.strip(): parts.append(cur)
    return [p.strip() for p in parts]

def opts(expr):
    expr = re.sub(r'\s+', ' ', expr).strip()
    if expr in ('Default::default()', 'BoxOpts::default()'): return 'Node::DEFAULT'
    m = re.match(r'BoxOpts::default\(\)', expr)
    if not m: return None
    rest, builders, fields = expr[m.end():], [], []
    while rest:
        m = re.match(r'\s*\.(\w+)\(', rest)
        if not m: return None
        end = balanced(rest, m.end() - 1)
        name, arg = m.group(1), rest[m.end():end - 1].strip()
        if name in BUILDER: builders.append(f'.{BUILDER[name]}({arg})')
        elif name in FIELD: fields.append(FIELD[name](arg))
        else: return None
        rest = rest[end:]
    base = 'Node::DEFAULT' + ''.join(builders)
    return f'Node {{ {", ".join(fields)}, ..{base} }}' if fields else base

def main():
  for path in sys.argv[1:]:
      s = open(path).read(); out = ''; i = 0
      for m in re.finditer(r'\.box_\(', s):
          start = m.end() - 1
          end = balanced(s, start)
          args = split_top(s[start + 1:end - 1])
          if len(args) != 2: continue
          node = opts(args[1])
          if node is None: print(f'{path}: skipped {s[m.start():end][:90]!r}', file=sys.stderr); continue
          out += s[i:m.start()] + f'.node({args[0]}.into(), {node})'
          i = end
      out += s[i:]
      if out != s:
          out = re.sub(r'\bBoxOpts, ', '', out); out = re.sub(r', BoxOpts\b', '', out)
          open(path, 'w').write(out); print(path)

if __name__ == '__main__': main()
