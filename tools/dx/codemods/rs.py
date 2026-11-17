"""Small Rust source helpers for the tools/dx codemods (stdlib only).

mask(src) returns a copy of src of the same length in which comments and the contents of string,
byte-string, raw-string and char literals are replaced by spaces (newlines kept), so a regex or a
paren matcher can run on the mask while edits are applied to src at the same offsets. A quote after
an identifier character or a lifetime (`'a`, `'static`) is left alone.
"""
import re


def mask(src):
    out = list(src)
    i, n = 0, len(src)

    def blank(a, b):
        for k in range(a, b):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
        elif src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif (m := re.compile(r'b?r(#*)"').match(src, i)) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            end = src.find('"' + m.group(1), m.end())
            end = n if end < 0 else end
            blank(m.end(), end)
            i = end + 1 + len(m.group(1))
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            blank(i + 1, j)
            i = j + 1
        elif c == "'":
            m = re.compile(r"'(\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])'").match(src, i)
            if m:
                blank(i + 1, m.end() - 1)
                i = m.end()
            else:
                i += 1  # a lifetime or label
        else:
            i += 1
    return "".join(out)


def close(m, i):
    """Index of the bracket closing the one at m[i] (m is a mask)."""
    depth = 0
    for j in range(i, len(m)):
        if m[j] in "([{":
            depth += 1
        elif m[j] in ")]}":
            depth -= 1
            if depth == 0:
                return j
    raise ValueError("unbalanced bracket at %d" % i)


def args(m, open_, close_):
    """(start, end) offsets of the top-level comma-separated arguments between m[open_] and m[close_]."""
    out, depth, start = [], 0, open_ + 1
    for j in range(open_ + 1, close_):
        if m[j] in "([{":
            depth += 1
        elif m[j] in ")]}":
            depth -= 1
        elif m[j] == "," and depth == 0:
            out.append((start, j))
            start = j + 1
    if m[start:close_].strip():
        out.append((start, close_))
    return out


def line_of(src, i):
    return src.count("\n", 0, i) + 1


if __name__ == "__main__":
    src = 'let a = "x(\\"y"; // ( comment\nlet c = \'(\'; fn f<\'a>(x: &\'a str) { r#"("# }'
    m = mask(src)
    assert len(m) == len(src)
    assert m.count("(") == 1 and m.count(")") == 1, m
    assert "'a" in m and close(m, m.index("{")) == m.rindex("}")
    print("rs.py self-check ok")
