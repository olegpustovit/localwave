#!/usr/bin/env python3
"""DX1.12D codemod 4: a 3-argument `view_of(key, &inputs, build)` gains the style `&<path>::DEFAULT` (its old fill-the-parent size).
Usage: view_of3.py <view-path> <file>..."""
import re, sys
from boxes import balanced, split_top
path_ = sys.argv[1]
for path in sys.argv[2:]:
    s = text = open(path).read(); out = ''; i = 0
    for m in re.finditer(r'\.view_of\(', s):
        if m.start() < i: continue
        start = m.end() - 1; end = balanced(s, start)
        inner = s[start + 1:end - 1]; args = split_top(inner)
        if len(args) != 3: continue
        # keep the original text of the closure (it may span lines): split after the second top-level comma
        depth = 0; commas = []
        for k, c in enumerate(inner):
            if c in '([{': depth += 1
            elif c in ')]}': depth -= 1
            elif c == ',' and depth == 0: commas.append(k)
        cut = commas[1] + 1
        out += s[i:start + 1] + inner[:cut] + f' &{path_}::DEFAULT,' + inner[cut:] + ')'
        i = end
    out += s[i:]
    if out != text: open(path, 'w').write(out); print(path)
