#!/usr/bin/env python3
"""DX1.12D codemod 1: style-value vocabulary. Usage: styles.py <root>... (rewrites *.rs in place, skips target/)."""
import re, sys, pathlib
CHAIN = r'(?:\.\w+\((?:[^()]|\([^()]*\))*\))*'
RULES = [
    (re.compile(r'(DialogButton::new\((?:[^()]|\([^()]*\))*\)' + CHAIN + r')\.default\(\)'), r'\1.primary()'),
    (re.compile(r'(MenuItem::new\((?:[^()]|\([^()]*\))*\)' + CHAIN + r')\.key\('), r'\1.shortcut('),
    (re.compile(r'(MenuItem::new\((?:[^()]|\([^()]*\))*\)' + CHAIN + r')\.selected\(\)'), r'\1.selected(true)'),
    (re.compile(r'(Menu::DEFAULT' + CHAIN + r')\.width\('), r'\1.w_px('),
    (re.compile(r'(Dialog::DEFAULT' + CHAIN + r')\.width\('), r'\1.w_px('),
    (re.compile(r'(Segmented::DEFAULT' + CHAIN + r')\.size\('), r'\1.w_px('),
    (re.compile(r'\b(?:MenuResponse|DialogResponse)\b(?! = pxui_types::feature)'), 'Chosen'),
]
for root in sys.argv[1:]:
    for path in pathlib.Path(root).rglob('*.rs'):
        if 'target' in path.parts: continue
        text = path.read_text(); new = text
        for pattern, repl in RULES:
            while True:
                nxt = pattern.sub(repl, new)
                if nxt == new: break
                new = nxt
        if new != text: path.write_text(new); print(path)
