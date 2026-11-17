#!/usr/bin/env python3
"""DX1.12G codemod: body-kind renames. usage: rename.py <root>... ; rewrites .rs/.theme/.h/.md/.txt/.tsv under roots."""
import re, sys, os
# order matters: table_view before table
MAP = [("context_menu","menu_row"),("list_view","selection_list"),("table_view","table_row"),("table","table_cell"),
       ("tab","tabs"),("segment","segmented"),("toast","toasts"),("tip","tooltip")]
EXT = {".rs",".theme",".md",".h",".txt",".tsv"}
SKIP = {"target",".git"}
def conv(text, path):
    for old, new in MAP:
        text = re.sub(rf"\bregister_{old}_body\b", f"register_{new}_body", text)
        text = re.sub(rf"\bBodyKind::{old}\b", f"BodyKind::{new}", text)
        text = re.sub(rf"\b(body|bodies)\.{old}\b(?!\w)", rf"\1.{new}", text)
        # theme-file / theme-text keys: "<kind>.body"
        text = re.sub(rf"(?:(?<=\\n)|(?<![\w.])){old}\.body\b", f"{new}.body", text)
    return text
for root in sys.argv[1:]:
    for d, ds, fs in os.walk(root):
        ds[:] = [x for x in ds if x not in SKIP]
        for f in fs:
            if os.path.splitext(f)[1] not in EXT: continue
            p = os.path.join(d, f)
            if "fixtures/abi" in p or "/abi/" in p: continue
            try: s = open(p).read()
            except Exception: continue
            n = conv(s, p)
            if n != s: open(p, "w").write(n); print(p)
