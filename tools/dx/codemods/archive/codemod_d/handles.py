#!/usr/bin/env python3
"""DX1.12D codemod 5: the wrapper names its node handle `Handle` (pxui::Handle) only; `PxuiBox` stays the C record's name.
Usage: handles.py <file>... (files of crates/public/pxui/src). C declarations (`fn pxui_...`) and `use` lines keep `PxuiBox`;
every other `PxuiBox` becomes `Handle`; a file left with `PxuiBox` only in its `use` lines drops it there; `use crate::Handle;`
is added after the first `use` line when the file names `Handle` but does not import it (lib.rs defines it)."""
import re, sys
WORD = re.compile(r'\bPxuiBox\b')
for path in sys.argv[1:]:
    lines = open(path).read().split('\n'); text = '\n'.join(lines)
    out = [l if ('fn pxui_' in l or l.lstrip().startswith(('use ', 'pub use ')) or 'pub type Handle' in l) else WORD.sub('Handle', l) for l in lines]
    body = [l for l in out if not l.lstrip().startswith(('use ', 'pub use '))]
    if not any(WORD.search(l) for l in body):
        fixed = []
        for l in out:
            if l.lstrip().startswith(('use ', 'pub use ')) and WORD.search(l) and not l.lstrip().startswith('pub use'):
                l = re.sub(r'\bPxuiBox, |, PxuiBox\b', '', l)
                l = re.sub(r'\{(\w+)\}', r'\1', l) if l.count('{') == 1 else l
                if re.search(r'::PxuiBox;$', l.strip()): continue
            fixed.append(l)
        out = fixed
    new = '\n'.join(out).replace('pxui_types::Handle', 'crate::Handle')
    out = new.split('\n')
    names = any(re.search(r'\bHandle\b', l) for l in body)
    imports = re.search(r'use crate::(\{[^}]*\bHandle\b[^}]*\}|Handle);', new) or path.endswith('/lib.rs')
    if names and not imports:
        i = next(k for k, l in enumerate(out) if l.startswith('use '))
        out.insert(i, 'use crate::Handle;'); new = '\n'.join(out)
    if new != text: open(path, 'w').write(new); print(path)
