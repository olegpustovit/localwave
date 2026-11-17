#!/usr/bin/env python3
"""DX1.12C: generates the missing const builders (Node, Look, Font, Ring, Shadow, States) into pxui_types."""
import sys
root = sys.argv[1]
def b(ty, name, params, body, doc, use, const, check):
    sig = ', '.join(params)
    return f'''    /// {doc}
    ///
    /// Allocates: never.
    /// Panics: never.
    ///
    /// ```
    /// use {use};
    /// const V: {ty} = {const};
    /// assert!({check});
    /// ```
    #[inline]
    pub const fn {name}(mut self{', ' + sig if sig else ''}) -> {ty} {{
        {body}
        self
    }}
'''
N = 'pxui_types::{layout::{Align, Anchor, Cell, Grid, Justify, Placement, Track}, geom::{Edges, Rect}, node::{Bar, Mark, Dir}, a11y::state, Look, Node}'
node = [
 ('axis', ['axis: Axis'], 'self.axis = axis;', 'Sets how children flow: `Axis::Row`, `Column` or `Stack` (`Node::row()` and its siblings start from one).', 'Node::DEFAULT.axis(pxui_types::Axis::Row)', 'matches!(V.axis, pxui_types::Axis::Row)'),
 ('justify', ['justify: Justify'], 'self.justify = justify;', 'Sets where children sit along the main axis (CSS `justify-content`).', 'Node::row().justify(Justify::SpaceBetween)', 'V.justify == Justify::SpaceBetween'),
 ('wrap', [], 'self.wrap = true;', 'Breaks the flow into lines when the children overflow the main axis (`row_gap` between lines).', 'Node::row().wrap()', 'V.wrap'),
 ('z', ['order: i16'], 'self.z = order;', 'Sets the order among a stack\'s children: higher draws over lower and takes the pointer first.', 'Node::DEFAULT.z(2)', 'V.z == 2'),
 ('min_w', ['points: f32'], 'self.min_w = points;', 'Sets the smallest width in points (`min_*` wins over `max_*`).', 'Node::DEFAULT.min_w(40.0)', 'V.min_w == 40.0'),
 ('min_h', ['points: f32'], 'self.min_h = points;', 'Sets the smallest height in points (`min_*` wins over `max_*`).', 'Node::DEFAULT.min_h(20.0)', 'V.min_h == 20.0'),
 ('max_w', ['points: f32'], 'self.max_w = points;', 'Sets the largest width in points (`f32::INFINITY`: no limit).', 'Node::DEFAULT.max_w(300.0)', 'V.max_w == 300.0'),
 ('max_h', ['points: f32'], 'self.max_h = points;', 'Sets the largest height in points (`f32::INFINITY`: no limit).', 'Node::DEFAULT.max_h(200.0)', 'V.max_h == 200.0'),
 ('shrink', ['factor: f32'], 'self.shrink = factor;', 'Sets how much room this node gives up, times its size, when its line overflows (0: none).', 'Node::DEFAULT.shrink(1.0)', 'V.shrink == 1.0'),
 ('aspect', ['ratio: f32'], 'self.aspect = ratio;', 'Sets the width-to-height ratio a `Fit` side is derived from (0: none).', 'Node::DEFAULT.aspect(1.5)', 'V.aspect == 1.5'),
 ('padding', ['edges: Edges'], 'self.padding = edges;', 'Sets the inset of the children on each side in points (`pad`, `pad_x`, `pad_y` set it evenly).', 'Node::DEFAULT.padding(Edges::xy(4.0, 2.0))', 'V.padding == Edges::xy(4.0, 2.0)'),
 ('margin', ['edges: Edges'], 'self.margin = edges;', 'Sets the room kept around this node on each side in points (`l` is the start edge).', 'Node::DEFAULT.margin(Edges::all(8.0))', 'V.margin == Edges::all(8.0)'),
 ('row_gap', ['points: f32'], 'self.row_gap = points;', 'Sets the points between wrapped lines and between grid rows (`gap` is between children and grid columns).', 'Node::row().wrap().row_gap(6.0)', 'V.row_gap == 6.0'),
 ('grid', ['grid: Grid'], 'self.grid = OptGrid::Some(grid);', 'Lays the children out on `grid`\'s tracks instead of the axis.', 'Node::DEFAULT.grid(Grid::new(&[Track::Fr(1.0), Track::Fr(1.0)], &[]))', 'V.grid.get().is_some_and(|g| g.cols().len() == 2)'),
 ('cell', ['cell: Cell'], 'self.cell = cell;', 'Sets this node\'s place in its parent\'s grid (`Cell::DEFAULT`: the next free cell).', 'Node::DEFAULT.cell(Cell::at(1, 0).span(2, 1))', 'V.cell.col_span == 2'),
 ('align_self', ['align: Align'], 'self.align_self = OptAlign::Some(align);', 'Overrides the parent\'s `align` for this node alone.', 'Node::DEFAULT.align_self(Align::End)', 'V.align_self.get() == Some(Align::End)'),
 ('anchor', ['anchor: Anchor'], 'self.anchor = OptAnchor::Some(anchor);', 'Places this node by a window rectangle (a popup) instead of its parent\'s flow (`place_anchored` is the rule).', 'Node::DEFAULT.anchor(Anchor { rect: Rect::new(0.0, 0.0, 10.0, 10.0), placement: Placement::BELOW })', 'V.anchor.is_some()'),
 ('a11y', ['states: u32'], 'self.a11y = states;', 'Sets the accessibility states the node\'s widget declares (`a11y::state` bits; focus, press, enabled and visibility come from the engine).', 'Node::DEFAULT.a11y(state::CHECKABLE | state::CHECKED)', 'V.a11y == state::CHECKABLE | state::CHECKED'),
 ('at', ['x: f32', 'y: f32'], 'self.at = OptPoint::Some(x, y);', 'Places this node outside the flow, `x`, `y` points from its parent\'s top-left.', 'Node::DEFAULT.at(4.0, 8.0)', 'V.at.get() == Some((4.0, 8.0))'),
 ('scroll_step', ['points: f32'], 'self.scroll_step = points;', 'Sets the points one wheel line or arrow key scrolls a scrolling node.', 'Node::DEFAULT.scroll_step(48.0)', 'V.scroll_step == 48.0'),
 ('look', ['look: Look'], 'self.look = look;', 'Sets how the node is painted (`set_look` replaces it by state while the app builds).', 'Node::DEFAULT.look(Look::NONE.radius(4.0))', 'V.look.radius == [4.0; 4]'),
 ('inset', ['edges: Edges'], 'self.inset = edges;', 'Sets where the node\'s text sits inside it, in points from each side.', 'Node::DEFAULT.inset(Edges::xy(6.0, 0.0))', 'V.inset.l == 6.0'),
 ('mark', ['mark: Mark'], 'self.mark = mark;', 'Sets the mark drawn over the node\'s text (a chevron or a line).', 'Node::DEFAULT.mark(Mark::Chevron(Dir::Down))', 'V.mark == Mark::Chevron(Dir::Down)'),
 ('bar', ['bar: Bar'], 'self.bar = OptBar::Some(bar);', 'Gives a scrolling node a thumb painted as `bar`.', 'Node::DEFAULT.bar(Bar { color: pxui_types::geom::Rgba::BLACK, radius: 2.0, width: 4.0, gap: 2.0, margin: 2.0, whole: false })', 'V.bar.is_some()'),
]
L = 'pxui_types::{geom::Rgba, theme::{Ring, Shadow}, Look}'
look = [
 ('gradient', ['to: Rgba'], 'self.gradient = crate::node::OptColor::Some(to);', 'Fills with a linear gradient from `fill` to `to` along `angle`.', 'Look::NONE.fill(Rgba::BLACK).gradient(Rgba::WHITE)', 'V.gradient.get() == Some(Rgba::WHITE)'),
 ('angle', ['degrees: f32'], 'self.angle = degrees;', 'Sets the gradient\'s direction in degrees, CSS style (180: top to bottom).', 'Look::NONE.angle(90.0)', 'V.angle == 90.0'),
 ('sides', ['sides: u8'], 'self.sides = sides;', 'Limits the border to `sides` (`draw::flags::rect::TOP | ...`; 0: all four).', 'Look::NONE.border(Rgba::BLACK, 1.0).sides(pxui_types::draw::flags::rect::BOTTOM)', 'V.sides == pxui_types::draw::flags::rect::BOTTOM'),
 ('radii', ['points: [f32; 4]'], 'self.radius = points;', 'Sets each corner\'s radius in points: top-left, top-right, bottom-right, bottom-left (`radius` sets all four).', 'Look::NONE.radii([8.0, 8.0, 0.0, 0.0])', 'V.radius == [8.0, 8.0, 0.0, 0.0]'),
 ('shadow', ['shadow: Shadow'], 'self.shadows[0] = shadow;', 'Sets the first box shadow (`shadows` sets both).', 'Look::NONE.shadow(Shadow::NONE.color(Rgba::BLACK).blur(6.0))', 'V.shadows[0].blur == 6.0'),
 ('shadows', ['shadows: [Shadow; 2]'], 'self.shadows = shadows;', 'Sets both box shadows, drawn first to last.', 'Look::NONE.shadows([Shadow::NONE.blur(2.0), Shadow::NONE.inner()])', 'V.shadows[1].inner'),
 ('opacity', ['opacity: f32'], 'self.opacity = opacity;', 'Sets the opacity the node and its children are drawn at, 0.0 to 1.0.', 'Look::NONE.opacity(0.5)', 'V.opacity == 0.5'),
 ('ring', ['ring: Ring'], 'self.ring = ring;', 'Sets the outline drawn outside the border (a focus ring).', 'Look::NONE.ring(Ring::NONE.color(Rgba::BLACK).width(2.0))', 'V.ring.width == 2.0'),
 ('selection', ['color: Rgba'], 'self.selection = color;', 'Sets the fill behind selected text.', 'Look::NONE.selection(Rgba::rgba(0, 0, 255, 60))', 'V.selection.a == 60'),
 ('caret', ['color: Rgba'], 'self.caret = color;', 'Sets the text caret\'s colour.', 'Look::NONE.caret(Rgba::BLACK)', 'V.caret == Rgba::BLACK'),
 ('dashed', [], 'self.dashed = true;', 'Draws the border dashed.', 'Look::NONE.border(Rgba::BLACK, 1.0).dashed()', 'V.dashed'),
]
F = 'pxui_types::Font'
font = [
 ('family', ['family: u16'], 'self.family = family;', 'Sets the `pxui_text` font index.', 'Font::BODY.family(1)', 'V.family == 1'),
 ('tracking', ['ems: f32'], 'self.tracking = ems;', 'Sets the extra spacing after every glyph, in ems.', 'Font::BODY.tracking(0.04)', 'V.tracking == 0.04'),
 ('caps', [], 'self.caps = true;', 'Upper-cases the text.', 'Font::BODY.caps()', 'V.caps'),
]
R = 'pxui_types::{geom::Rgba, Ring}'
ring = [
 ('color', ['color: Rgba'], 'self.color = color;', 'Sets the ring\'s colour (transparent: no ring).', 'Ring::NONE.color(Rgba::BLACK)', 'V.color == Rgba::BLACK'),
 ('width', ['points: f32'], 'self.width = points;', 'Sets the ring\'s width in points.', 'Ring::NONE.width(2.0)', 'V.width == 2.0'),
 ('sides', ['sides: u8'], 'self.sides = sides;', 'Limits the ring to `sides` (`draw::flags::rect::TOP | ...`; 0: all four).', 'Ring::NONE.sides(pxui_types::draw::flags::rect::LEFT)', 'V.sides == pxui_types::draw::flags::rect::LEFT'),
]
S = 'pxui_types::{geom::Rgba, Shadow}'
shadow = [
 ('color', ['color: Rgba'], 'self.color = color;', 'Sets the shadow\'s colour (transparent: no shadow).', 'Shadow::NONE.color(Rgba::BLACK)', 'V.color == Rgba::BLACK'),
 ('offset', ['x: f32', 'y: f32'], 'self.offset = [x, y];', 'Sets the shadow\'s offset in points, right and down.', 'Shadow::NONE.offset(0.0, 2.0)', 'V.offset == [0.0, 2.0]'),
 ('blur', ['points: f32'], 'self.blur = points;', 'Sets the blur radius in points.', 'Shadow::NONE.blur(8.0)', 'V.blur == 8.0'),
 ('spread', ['points: f32'], 'self.spread = points;', 'Sets how many points the shadow grows (negative: shrinks) before blurring.', 'Shadow::NONE.spread(-1.0)', 'V.spread == -1.0'),
 ('inner', [], 'self.inner = true;', 'Draws the shadow inside the box (CSS `inset`).', 'Shadow::NONE.inner()', 'V.inner'),
]
ST = 'pxui_types::{geom::Rgba, Look, States}'
states = [(n, ['look: Look'], f'self.{n} = look;', d, f'States::all(Look::NONE).{n}(Look::NONE.fill(Rgba::BLACK))', f'V.{n}.fill == Rgba::BLACK') for n, d in [
 ('idle', 'Sets the look with no other state.'), ('hover', 'Sets the look under the pointer.'), ('pressed', 'Sets the look while pressed.'),
 ('focus', 'Sets the look while focused.'), ('disabled', 'Sets the look while disabled (it wins over every other state).'), ('checked', 'Sets the look while checked (on, selected).')]]

def gen(ty, use, specs):
    return ''.join(b(ty, n, p, body, doc, use, c, chk) for n, p, body, doc, c, chk in specs)

def insert_after(path, marker, code):
    s = open(path).read()
    assert s.count(marker) == 1, (path, marker)
    s = s.replace(marker, marker + code)
    open(path, 'w').write(s)

import os
nodef = os.path.join(root, 'node.rs'); themef = os.path.join(root, 'theme.rs')
insert_after(nodef, '''    pub const fn flags(mut self, flags: Flags) -> Node {
        self.flags = flags;
        self
    }
''', gen('Node', N, node))
insert_after(themef, '''    pub const fn font(mut self, font: Font) -> Look {
        self.font = font;
        self
    }
''', gen('Look', L, look))
insert_after(themef, '''    pub const fn weight(mut self, weight: u16) -> Font {
        self.weight = weight;
        self
    }
''', gen('Font', F, font))
insert_after(themef, '''        sides: 0,
    };
''', '\n' + gen('Ring', R, ring))
insert_after(themef, '''        spread: 0.0,
        inner: false,
    };
''', '\n' + gen('Shadow', S, shadow))
insert_after(themef, '''            checked: look,
        }
    }
''', gen('States', ST, states))
