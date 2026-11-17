#!/usr/bin/env python3
"""DX1.12B codemod for pxui::images callers: Key -> Want (accessors), Request::keys -> wants, Request::insert(id, path, px) -> insert(Want),
Images::get_at/visible_requests/requests -> get(.., Some(node))/requests(Some(ui))/requests(None). Usage: images_codemod.py FILE..."""
import re, sys
for path in sys.argv[1:]:
    s = open(path).read(); o = s
    # field reads on a want held in `key`/`other`/`ready.key`/`image.key` -> accessors
    s = re.sub(r'\b((?:ready|image|wanted|self)\.key|key|other)\.(id|max_pixels)\b(?!\s*[:(])', r'\1.\2()', s)
    s = re.sub(r'\b((?:ready|image|wanted|self)\.key|key|other)\.path\.as_str\(\)', r'\1.path()', s)
    s = re.sub(r'\b((?:ready|image|wanted|self)\.key|key|other)\.path\b(?!\s*[:(])', r'\1.path()', s)
    # Request::insert(want.id(), want.path(), want.max_pixels()) -> insert(want)
    s = re.sub(r'\.insert\((\w+)\.id\(\), \1\.path\(\), \1\.max_pixels\(\)\)', r'.insert(*\1)', s)
    s = re.sub(r'\.keys\(\)', '.wants()', s) if re.search(r'artwork|images::|Request', s) else s
    s = s.replace('.visible_requests(', '.requests(Some(').replace('requests()', 'requests(None)')
    s = re.sub(r'\.requests\(Some\(([^()]*)\)', r'.requests(Some(\1))', s)
    s = re.sub(r'\bKey\b(?=[,}>)\s])', 'Want', s) if 'artwork::{' in s or 'images::{' in s else s
    if s != o:
        open(path, 'w').write(s); print(path)
