#!/usr/bin/env python3
"""DX1.12I codemod: Ui::create -> Result, set_theme_source -> set_theme, Theme handle -> ThemeHandle,
key_pressed/key_down -> KeyCode arguments. Usage: codemod_i.py <keycode path prefix: crate|pxui> <files...>"""
import re, sys

SPECIAL = {"ArrowUp": "ARROW_UP", "ArrowDown": "ARROW_DOWN", "ArrowLeft": "ARROW_LEFT", "ArrowRight": "ARROW_RIGHT", "Space": "SPACE",
           "Enter": "ENTER", "Escape": "ESCAPE", "Backspace": "BACKSPACE", "Tab": "TAB", "Shift": "SHIFT", "Ctrl": "CTRL", "Alt": "ALT",
           "Meta": "META", "Home": "HOME", "End": "END", "PageUp": "PAGE_UP", "PageDown": "PAGE_DOWN", "Insert": "INSERT", "Delete": "DELETE"}
SPECIAL.update({f"F{i}": f"F{i}" for i in range(1, 13)})


def close_paren(s, i):
    """Index of the ')' matching the '(' at s[i]."""
    depth = 0
    j = i
    while j < len(s):
        c = s[j]
        if c == "'" and j + 2 < len(s) and (s[j + 2] == "'" or (s[j + 1] == "\\" and s[j + 3] == "'")):
            j += 3 if s[j + 2] == "'" else 4
            continue
        if c == '"':
            j += 1
            while s[j] != '"':
                j += 2 if s[j] == "\\" else 1
        elif c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
            if depth == 0:
                return j
        j += 1
    raise ValueError("unbalanced")


def split_args(a):
    out, depth, cur, j = [], 0, "", 0
    while j < len(a):
        c = a[j]
        if c == "'" and j + 2 < len(a) and a[j + 2] == "'":
            cur += a[j:j + 3]; j += 3; continue
        if c in "([{": depth += 1
        if c in ")]}": depth -= 1
        if c == "," and depth == 0:
            out.append(cur); cur = ""
        else:
            cur += c
        j += 1
    out.append(cur)
    return out


def key(expr, kc):
    e = expr.strip()
    if e == "None":
        return e
    m = re.fullmatch(r"Some\((.*)\)", e, re.S)
    if m:
        return f"Some({key(m.group(1), kc)})"
    m = re.fullmatch(r"('(?:\\.|.)')(?:\s*as\s+u32)?", e)
    if m:
        return f"{kc}::char({m.group(1)})"
    m = re.fullmatch(r"(?:input::)?Key::(\w+)(?:\.code\(\))?", e)
    if m and m.group(1) in SPECIAL:
        return f"{kc}::{SPECIAL[m.group(1)]}"
    if e.startswith(kc) or e.startswith("KeyCode"):
        return e
    return f"{kc}({e})"


def keys(s, kc):
    for name in ("key_pressed", "key_down"):
        pos = 0
        pat = re.compile(r"\." + name + r"\(")
        while True:
            m = pat.search(s, pos)
            if not m:
                break
            o = m.end() - 1
            c = close_paren(s, o)
            args = split_args(s[o + 1:c])
            new = ", ".join(key(a, kc) for a in args)
            s = s[:o + 1] + new + s[c:]
            pos = o + 1 + len(new)
    return s


def create(s):
    pos = 0
    pat = re.compile(r"\bUi::create\(")
    while True:
        m = pat.search(s, pos)
        if not m:
            break
        o = m.end() - 1
        c = close_paren(s, o)
        args = s[o + 1:c]
        before = s[max(0, m.start() - 9):m.start()]
        if "Text::new" in args or "engine::" in before or "///" in s[s.rfind("\n", 0, m.start()):m.start()] or s[c + 1:c + 2] == ".":
            pos = c
            continue
        s = s[:c + 1] + '.expect("pxui Ui")' + s[c + 1:]
        pos = c
    return s


def main():
    kc = {"crate": "crate::KeyCode", "pxui": "pxui::KeyCode"}[sys.argv[1]]
    for path in sys.argv[2:]:
        s = open(path).read()
        t = create(s)
        t = t.replace("set_theme_source(", "set_theme(")
        t = re.sub(r"(?<![:\w])(crate::|pxui::)?Theme::(default|from_tokens)\(", lambda m: (m.group(1) or "") + "ThemeHandle::" + m.group(2) + "(", t)
        t = keys(t, kc)
        if t != s:
            open(path, "w").write(t)
            print("changed", path)


main()
