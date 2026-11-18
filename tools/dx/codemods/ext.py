#!/usr/bin/env python3
"""U1c.3: method calls of the extension set become pxui::ext free functions.

    ext.py [--dry-run] FILE...

`recv.inspector_row(a, b)` -> `pxui::ext::inspector_row(recv, a, b)` (`crate::ext::` inside crates/public/pxui), for the names of
the extension set. The receiver is the identifier (or `self.field` path) right before the dot; any other receiver is printed as
skipped. Prints per-file counts.
"""
import re, sys
NAMES = ["inspector_row", "inspector_section", "inspector_toggle", "inspector_button", "inspector_readout", "inspector_message",
         "inspector_bar", "inspector_body", "inspector_header", "inspector_width", "inspector_label_x", "footer", "line_field",
         "value_field", "grip", "field_drag"]
CALL = re.compile(r"(?<![\w:])((?:&mut |&)?[A-Za-z_][\w]*(?:\.[A-Za-z_]\w*)*)\.(" + "|".join(NAMES) + r")\(")
dry = "--dry-run" in sys.argv
for path in [a for a in sys.argv[1:] if a != "--dry-run"]:
    source = open(path).read()
    prefix = "crate::ext::" if "/crates/public/pxui/src/" in path or path.startswith("crates/public/pxui/src/") else "pxui::ext::"
    count = 0
    def swap(m):
        global count
        recv, name = m.group(1), m.group(2)
        count += 1
        return f"{prefix}{name}({recv}, "
    out = CALL.sub(swap, source)
    # a call with no other argument: `f(recv, )` -> `f(recv)`
    out = re.sub(r"(::ext::\w+\([^(),]+), \)", r"\1)", out)
    for line_no, line in enumerate(source.split("\n"), 1):
        for name in NAMES:
            if re.search(r"\)\." + name + r"\(", line): print(f"skipped {path}:{line_no}: {line.strip()[:100]}")
    print(f"{path}: {count}")
    if not dry and out != source: open(path, "w").write(out)
