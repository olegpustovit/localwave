#!/usr/bin/env python3
"""DX1.12D codemod 6 (review fix): `view_of(key, &inputs, &View::DEFAULT[sizes], f)` -> the spec's `view_of(key, &inputs, f)`.
A view fills its parent, so a sized one gets an enclosing node of that size (a column; `View`'s default size was Grow x Grow):
  statement `ui.view_of(..);`        -> `ui.column(K, NODE, |ui| { ui.view_of(K, I, F); });`
  `let X = ui.view_of(..);`          -> `let X_box = ui.node(K.into(), NODE); let mut X = X_box; ui.within(&X_box, &mut |ui| X = ui.view_of(K, I, F));`
  any other position                 -> the same as a block expression.
`.flat()` is dropped (the wrapper no longer offers it). Usage: views2.py <file>..."""
import re, sys
from boxes import balanced, split_top

def node_of(chain):
    w, h, rest = 'Size::Grow', 'Size::Grow', chain
    while rest:
        m = re.match(r'\.(\w+)\(', rest)
        end = balanced(rest, m.end() - 1); arg = rest[m.end():end - 1]
        name = m.group(1)
        if name == 'w_px': w = f'Size::Px({arg})'
        elif name == 'h_px': h = f'Size::Px({arg})'
        elif name == 'w': w = arg
        elif name == 'h': h = arg
        elif name != 'flat': raise ValueError(name)
        rest = rest[end:]
    w = re.sub(r'^(crate::ui::|pxui::ui::)', '', w); h = re.sub(r'^(crate::ui::|pxui::ui::)', '', h)
    b = 'Node::column()'
    b += f'.w_px({w[9:-1]})' if w.startswith('Size::Px(') else ('' if w == 'Size::Grow' else f'.w({w})')
    b += f'.h_px({h[9:-1]})' if h.startswith('Size::Px(') else ('' if h == 'Size::Fit' else f'.h({h})')
    return b

def once(s):
    out = ''; i = 0
    for m in re.finditer(r'\bui\s*\.view_of\(', s):
        if m.start() < i: continue
        start = m.end() - 1; end = balanced(s, start)
        inner = s[start + 1:end - 1].rstrip()
        inner = inner[:-1] if inner.endswith(',') else inner
        depth = 0; commas = []
        for k, c in enumerate(inner):
            if c in '([{': depth += 1
            elif c in ')]}': depth -= 1
            elif c == ',' and depth == 0: commas.append(k)
        if len(commas) != 3: continue
        style = inner[commas[1] + 1:commas[2]].strip()
        sm = re.fullmatch(r'&(?:[\w:]*::)?View::DEFAULT(.*)', style, re.S)
        if not sm: continue
        key, inputs, build = inner[:commas[0]].strip(), inner[commas[0] + 1:commas[1]].strip(), inner[commas[2] + 1:].lstrip()
        call = f'ui.view_of({key}, {inputs}, {build})'
        chain = sm.group(1).strip()
        if not chain:
            out += s[i:m.start()] + call; i = end; continue
        node = node_of(chain)
        line_start = s.rfind('\n', 0, m.start()) + 1
        before = s[line_start:m.start()]
        after = s[end:end + 1]
        lm = re.fullmatch(r'(\s*)let (\w+) = ', before)
        if lm and after == ';':
            ind, x = lm.group(1), lm.group(2)
            out += s[i:line_start] + f'{ind}let {x}_box = ui.node({key}.into(), {node});\n{ind}let mut {x} = {x}_box;\n{ind}ui.within(&{x}_box, &mut |ui| {x} = {call})'
        elif before.strip() == '' and after == ';':
            out += s[i:m.start()] + f'ui.column({key}, {node}, |ui| {{ {call}; }})'
        else:
            out += s[i:m.start()] + f'{{ let enclosing = ui.node({key}.into(), {node}); let mut view = enclosing; ui.within(&enclosing, &mut |ui| view = {call}); view }}'
        i = end
    return out + s[i:]

for path in sys.argv[1:]:
    s = text = open(path).read()
    while (nxt := once(s)) != s: s = nxt
    if s != text: open(path, 'w').write(s); print(path)
