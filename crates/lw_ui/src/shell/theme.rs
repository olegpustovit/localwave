//! LocalWave's text styles: Swift's one-line `Text` (lines 1.4 times the size, end ellipsis, whole-pixel fitted width), in the
//! UI face (`TEXT_*`) or Geist Mono (`MONO_*`), and its wrapped running text (`PARAGRAPH_*`). Call sites add the box (`w`, `h`)
//! and the colour token.
use super::*;

/// Swift's line height: 1.4 times the size, the one place the factor is written.
const LINE: f64 = 1.4;
/// One line of `points` text in the UI face: a line box 1.4 times the size high, end ellipsis, a whole-pixel fitted width.
pub(super) const fn line_style(points: f32) -> Label {
    Label::BODY.size(points).line_height(points * 1.4).h_px((points as f64 * LINE) as f32).lines(1).ellipsis().whole_pixels()
}
/// Running text at `points` filling its parent's width: word wrapped, lines 1.4 times the size, read as a paragraph.
const fn paragraph_style(points: f32) -> Label {
    Label::BODY.size(points).line_height(points * 1.4).wrap().grow().role(Role::Paragraph)
}
pub(super) const MONO_9: Label = line_style(9.0).font(1);
pub(super) const MONO_11: Label = line_style(11.0).font(1);
pub(super) const MONO_12: Label = line_style(12.0).font(1);
pub(super) const MONO_13: Label = line_style(13.0).font(1);
pub(super) const PARAGRAPH_13: Label = paragraph_style(13.0);
pub(super) const PARAGRAPH_14: Label = paragraph_style(14.0);
pub(super) const PARAGRAPH_15: Label = paragraph_style(15.0);
pub(super) const PARAGRAPH_24: Label = paragraph_style(24.0);
pub(super) const TEXT_11: Label = line_style(11.0);
pub(super) const TEXT_12: Label = line_style(12.0);
pub(super) const TEXT_13: Label = line_style(13.0);
pub(super) const TEXT_14: Label = line_style(14.0);
pub(super) const TEXT_15: Label = line_style(15.0);
pub(super) const TEXT_16: Label = line_style(16.0);
pub(super) const TEXT_17: Label = line_style(17.0);
pub(super) const TEXT_18: Label = line_style(18.0);
pub(super) const TEXT_20: Label = line_style(20.0);
pub(super) const TEXT_24: Label = line_style(24.0);
pub(super) const TEXT_28: Label = line_style(28.0);
pub(super) const TEXT_32: Label = line_style(32.0);
pub(super) const TEXT_34: Label = line_style(34.0);
/// Secondary 13-point text: counts, captions, helper copy.
pub(super) const CAPTION: Label = TEXT_13.color(SECONDARY);
/// Secondary 12-point text.
pub(super) const FOOTNOTE: Label = TEXT_12.color(SECONDARY);
/// Secondary 14-point text.
pub(super) const BODY_DIM: Label = TEXT_14.color(SECONDARY);
/// Ink 14-point text.
pub(super) const BODY_INK: Label = TEXT_14.color(INK);
/// Ink 15-point text.
pub(super) const LEAD_INK: Label = TEXT_15.color(INK);
/// Secondary 15-point text.
pub(super) const LEAD_DIM: Label = TEXT_15.color(SECONDARY);
/// Secondary 12-point mono text.
pub(super) const MONO_DIM: Label = MONO_12.color(SECONDARY);

/// LocalWave's resolved button looks for one appearance: `looks(ui)` picks the light or dark set, so a theme change selects the
/// other set and nothing is resolved per frame.
pub(super) struct Looks {
    /// A compact tab bar button: ink 14-point text on paper.
    pub tab: States,
    /// The current compact tab: on the selected fill.
    pub tab_on: States,
    /// A Find tool on paper.
    pub paper: States,
    /// The current Find tool: on the selected fill.
    pub selected: States,
    /// A key on the panel fill.
    pub panel: States,
    /// A key on the ink fill (Play, Reconnect, an active mode key, an armed swipe action).
    pub ink: States,
    /// A mode key on the key fill.
    pub key: States,
    /// A queue sheet text action's text: secondary, 12 points.
    pub queue_action: Look,
    /// An icon key with no resting plate: the panel fill while hovered or held, ink icon.
    pub icon: States,
    /// An icon key on the key plate.
    pub icon_plate: States,
    /// An icon key in inverse (Play, an active mode): paper icon on ink.
    pub icon_inverse: States,
    /// A sidebar "+ label" row's text: secondary, 14 points.
    pub add_row: Look,
}
/// A button with no look of its own: its contents draw it.
pub(super) const BARE: States = States::all(Look::NONE);
/// A pressable that keeps its node's own look (the node draws itself, or its contents do).
pub(super) const PLAIN: pxui::Pressable = pxui::Pressable::DEFAULT;
const fn token(token: ColorToken, dark: bool) -> Rgba {
    let pair = COLORS[token.0 as usize];
    if dark { pair.1 } else { pair.0 }
}
const fn fill(color: ColorToken, dark: bool) -> States {
    States::all(Look { fill: token(color, dark), ..Look::NONE })
}
const fn text(color: ColorToken, size: i64, dark: bool) -> Look {
    Look { text: token(color, dark), font: Font { size, ..Font::BODY }, ..Look::NONE }
}
/// An ink icon key resting on `rest`, on the panel fill while hovered or held.
const fn icon(rest: Rgba, dark: bool) -> States {
    let idle = Look { fill: rest, text: token(INK, dark), ..Look::NONE };
    let lit = Look { fill: token(PANEL, dark), ..idle };
    States { hover: lit, pressed: lit, ..States::all(idle) }
}
const fn resolve(dark: bool) -> Looks {
    let tab = text(INK, 14, dark);
    Looks {
        tab: States::all(Look { fill: token(PAPER, dark), ..tab }),
        tab_on: States::all(Look { fill: token(SELECTED, dark), ..tab }),
        paper: fill(PAPER, dark),
        selected: fill(SELECTED, dark),
        panel: fill(PANEL, dark),
        ink: fill(INK, dark),
        key: fill(KEY, dark),
        queue_action: text(SECONDARY, 12, dark),
        add_row: text(SECONDARY, 14, dark),
        icon: icon(Look::NONE.fill, dark),
        icon_plate: icon(token(KEY, dark), dark),
        icon_inverse: States::all(Look { fill: token(INK, dark), text: token(PAPER, dark), ..Look::NONE }),
    }
}
const LIGHT: Looks = resolve(false);
const DARK: Looks = resolve(true);
/// The looks for `ui`'s appearance (the dark colour table when its theme is dark, as `Ui::color` reads).
pub(super) fn looks(ui: &Ui) -> &'static Looks {
    if pxui::theme::dark(&ui.theme().tokens) { &DARK } else { &LIGHT }
}
/// LocalWave's icon key `w` by `h` points: it dips to 0.96 while held, springing in at the instant pace and back at the base pace.
pub(super) const fn icon_key(w: f32, h: f32) -> pxui::IconButton {
    pxui::IconButton::sized(w, h).press_scale(0.96).press_curves(&PRESS)
}
/// The icon key sizes LocalWave repeats: 28, 36 and 44 points square, and 44 by 40.
pub(super) const ICON_28: pxui::IconButton = icon_key(28.0, 28.0);
pub(super) const ICON_36: pxui::IconButton = icon_key(36.0, 36.0);
pub(super) const ICON_44: pxui::IconButton = icon_key(44.0, 44.0);
pub(super) const ICON_44X40: pxui::IconButton = icon_key(44.0, 40.0);
const PRESS: [pxui::theme::Curve; 2] = [crate::motion::spring(crate::motion::INSTANT), crate::motion::spring(crate::motion::BASE)];
