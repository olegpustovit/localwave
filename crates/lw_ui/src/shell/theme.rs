//! LocalWave's text styles: Swift's one-line `Text` (lines 1.4 times the size, end ellipsis, whole-pixel fitted width), in the
//! UI face (`TEXT_*`) or Geist Mono (`MONO_*`), and its wrapped running text (`PARAGRAPH_*`). Call sites add the box (`w`, `h`)
//! and the colour token.
use super::*;

/// One line of `points` text in the UI face: lines 1.4 times the size, end ellipsis, a whole-pixel fitted width.
pub(super) const fn line_style(points: f32) -> Label {
    Label::BODY.size(points).line_height(points * 1.4).lines(1).ellipsis().whole_pixels()
}
/// Running text at `points` filling its parent's width: word wrapped, lines 1.4 times the size, read as a paragraph.
const fn paragraph_style(points: f32) -> Label {
    Label::BODY.size(points).line_height(points * 1.4).wrap().w(Size::Grow).role(Role::Paragraph)
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
    /// A sidebar "+ label" row's text: secondary, 14 points.
    pub add_row: Look,
}
/// A button with no look of its own: its contents draw it.
pub(super) const BARE: States = States::all(Look::NONE);
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
    }
}
const LIGHT: Looks = resolve(false);
const DARK: Looks = resolve(true);
/// The looks for `ui`'s appearance (the dark colour table when its theme is dark, as `Ui::color` reads).
pub(super) fn looks(ui: &Ui) -> &'static Looks {
    if pxui::theme::dark(&ui.theme().tokens) { &DARK } else { &LIGHT }
}
