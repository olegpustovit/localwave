#!/usr/bin/env python3
"""DX1.12D LocalWave remainder: add `use pxui::View;` to files that name `View::DEFAULT` without importing it.
Usage: lw_imports.py <file>..."""
import re, sys
for path in sys.argv[1:]:
    s = open(path).read()
    if 'View::DEFAULT' not in s or re.search(r'use pxui::(\{[^}]*\bView\b|View;)', s): continue
    lines = s.split('\n')
    i = next(k for k, l in enumerate(lines) if l.startswith('use '))
    lines.insert(i, 'use pxui::View;')
    open(path, 'w').write('\n'.join(lines)); print(path)
