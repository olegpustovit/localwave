#!/usr/bin/env python3
"""Copy field comments from pxui.h to pxui_types/ffi.rs without replacing Rust docs."""
import argparse
from pathlib import Path
import re

from rs import close, mask


def header_fields(header):
    code = mask(header)
    result = {}
    for record in re.finditer(r"\btypedef\s+struct\s*\{", code):
        end = close(code, record.end() - 1)
        name = re.match(r"\s+(\w+)\s*;", code[end + 1:])
        if not name:
            continue
        start = record.end()
        body = header[start:end]
        for comment in re.finditer(r"/\*\s*(.*?)\s*\*/", body, re.S):
            before = code[start:start + comment.start()].rstrip()
            after = code[start + comment.end():end]
            if before.endswith((";", "{")) or not before:
                # Leading comments are followed by a declaration, not just a field name.
                field = re.match(r"\s*(?:[A-Za-z_]\w*\s+|[*\s])+([A-Za-z_]\w*)\s*(?:[,;\[])", after)
            else:
                field = re.search(r"([A-Za-z_]\w*)(?:\[[^]]*\])?$", before)
            if field:
                text = " ".join(comment[1].split())
                if text:
                    text = text[0].upper() + text[1:]
                    result[(name[1], field[1])] = text if text.endswith('.') else text + '.'
    # The C comment follows the semicolon, but describes the fractional field to its left.
    if text := result.pop(("PxuiFontDesc", "size"), None):
        result[("PxuiFontDesc", "size_points")] = text
    return result


def rewrite(source, comments):
    current = None
    documented = False
    output = []
    for line in source.splitlines(keepends=True):
        if record := re.match(r"^pub struct (\w+).*\{\s*$", line):
            current = record[1]
        elif line.startswith('}'):
            current = None
        if field := re.match(r"^(\s*)pub (\w+):", line):
            if current and not documented:
                comment = comments.get((current, field[2])) or comments.get(('Pxui' + current, field[2]))
                if comment:
                    output.append(f"{field[1]}/// {comment}\n")
        stripped = line.lstrip()
        if not stripped.startswith('#['):
            documented = stripped.startswith('///')
        output.append(line)
    return ''.join(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('header', type=Path)
    parser.add_argument('ffi', type=Path)
    args = parser.parse_args()
    source = args.ffi.read_text()
    after = rewrite(source, header_fields(args.header.read_text()))
    if after != source:
        args.ffi.write_text(after)


if __name__ == '__main__':
    h = 'typedef struct { uint8_t mode /* select mode */, fit; /* float limit in points */ float max_h; } PxuiRows;'
    r = 'pub struct PxuiRows {\n    pub mode: u8,\n    /// Existing.\n    pub max_h: f32,\n}\n'
    fields = header_fields(h)
    assert fields == {('PxuiRows', 'mode'): 'Select mode.', ('PxuiRows', 'max_h'): 'Float limit in points.'}, fields
    after = rewrite(r, fields)
    assert '/// Select mode.' in after and '/// Existing.' in after and '/// Float limit' not in after and rewrite(after, fields) == after
    main()
