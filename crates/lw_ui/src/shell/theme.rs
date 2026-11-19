//! LocalWave's text styles: Swift's one-line `Text` (lines 1.4 times the size, end ellipsis, whole-pixel fitted width), in the
//! UI face (`TEXT_*`) or Geist Mono (`MONO_*`), and its wrapped running text (`PARAGRAPH_*`). Call sites add the box (`w`, `h`)
//! and the colour token. Its looks are named consts over colour tokens (`Rgba::token`), which pxui resolves from the theme's
//! colour table where a look is applied.
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
/// One line in the UI face filling its row: a `line_height`-point line box, end ellipsis (Swift's fixed-frame title `Text`).
const fn line_box(points: f32, line_height: f32, weight: u16, tracking: f32) -> Label {
    Label::BODY.size(points).line_height(line_height).weight(weight).tracking(tracking).lines(1).ellipsis().grow().h_px(line_height)
}
/// A page title (Library, Settings, a list or folder name): 32 points, ink.
pub(super) const PAGE_TITLE: Label = line_box(32.0, 33.6, 500, -0.035).color(INK);
/// An album page title: 40 points, ink.
pub(super) const ALBUM_TITLE: Label = line_box(40.0, 40.8, 500, TIGHT_3).color(INK);
/// An artist page title: 48 points, ink.
pub(super) const ARTIST_TITLE: Label = line_box(48.0, 48.0, 500, -0.025).color(INK);
/// A sheet title (Add music): 20 points, ink.
pub(super) const SHEET_TITLE: Label = line_box(20.0, 28.0, 500, -0.01).color(INK);
/// A sheet option title: 17 points, ink.
pub(super) const OPTION_TITLE: Label = line_box(17.0, 23.8, 500, 0.0).color(INK);
/// An album card title: 14 points, ink.
pub(super) const CARD_TITLE: Label = line_box(14.0, 17.5, 500, 0.0).color(INK);
/// A detail summary, kind or year: 13 points, secondary.
pub(super) const SUMMARY: Label = line_box(13.0, 18.2, 400, 0.0).color(SECONDARY);
/// An artist page summary: 14 points, secondary.
pub(super) const ARTIST_SUMMARY: Label = line_box(14.0, 19.6, 400, 0.0).color(SECONDARY);
/// A section caption (Songs, the song count): 13 points medium, secondary.
pub(super) const SECTION_CAPTION: Label = line_box(13.0, 18.2, 500, 0.0).color(SECONDARY);
/// One phone line at `points` filling its row: lines 1.4 times the size, end ellipsis, the theme's weight.
const fn phone_line(points: f32) -> Label {
    Label::BODY.size(points).line_height(points * 1.4).lines(1).ellipsis().grow().h_px(points * 1.4)
}
/// A phone page title: 28 points, ink.
pub(super) const PHONE_TITLE: Label = phone_line(28.0).color(INK);
/// A phone album or artist row title: 16 points, ink.
pub(super) const GROUP_TITLE: Label = phone_line(16.0).color(INK);
/// A phone song row title: 15 points, ink.
pub(super) const ROW_TITLE: Label = phone_line(15.0).color(INK);
/// The playing phone song row's title: 15 points, the accent.
pub(super) const ROW_TITLE_NOW: Label = phone_line(15.0).color(ColorToken::ACCENT);
/// The mini player's title: 14 points, ink.
pub(super) const MINI_TITLE: Label = phone_line(14.0).color(INK);
/// An empty phone page's note: 14 points, secondary.
pub(super) const EMPTY_NOTE: Label = phone_line(14.0).color(SECONDARY);
/// A phone song row's artist or file note: 13 points, secondary.
pub(super) const ROW_NOTE: Label = phone_line(13.0).color(SECONDARY);
/// A small phone note (the mini player's artist, a row count, the player notice): 12 points, secondary.
pub(super) const SMALL_NOTE: Label = phone_line(12.0).color(SECONDARY);
/// A dialog's subject (the song, list or folder it acts on): 15-point running text, ink.
pub(super) const DIALOG_NAME: Label = PARAGRAPH_15.color(INK);
/// A dialog's consequences: 14-point running text, secondary.
pub(super) const DIALOG_NOTE: Label = PARAGRAPH_14.color(SECONDARY);
/// An Add music option's detail: 13-point running text, secondary.
pub(super) const OPTION_DETAIL: Label = PARAGRAPH_13.color(SECONDARY);

/// A button with no look of its own: its contents draw it.
pub(super) const BARE: States = States::of(Look::NONE);
/// A pressable that keeps its node's own look (the node draws itself, or its contents do).
pub(super) const PLAIN: pxui::Pressable = pxui::Pressable::DEFAULT;

/// The text look of `style`'s colour token, for text a node lays with `text_line_box` (which colours it from the node's look,
/// not the style); no look for a style without a colour.
pub(super) const fn text_look(style: Label) -> Look {
    match style.fields().color {
        Some(token) => Look::NONE.text(Rgba::token(token)),
        None => Look::NONE,
    }
}
/// The paper fill.
pub(super) const PAPER_FILL: Look = Look::NONE.fill(Rgba::token(PAPER));
/// The panel fill.
pub(super) const PANEL_FILL: Look = Look::NONE.fill(Rgba::token(PANEL));
/// The selected fill.
pub(super) const SELECTED_FILL: Look = Look::NONE.fill(Rgba::token(SELECTED));
/// The ink fill.
pub(super) const INK_FILL: Look = Look::NONE.fill(Rgba::token(INK));
/// The key fill.
pub(super) const KEY_FILL: Look = Look::NONE.fill(Rgba::token(KEY));
/// The hairline fill.
pub(super) const LINE_FILL: Look = Look::NONE.fill(Rgba::token(super::kit::LINE));
/// A 60 percent panel wash (an artist row while hovered or held).
pub(super) const PANEL_WASH: Look = Look::NONE.fill(Rgba::token(PANEL).alpha(153));
/// Ink text.
pub(super) const INK_TEXT: Look = Look::NONE.text(Rgba::token(INK));
/// Secondary text.
pub(super) const SECONDARY_TEXT: Look = Look::NONE.text(Rgba::token(SECONDARY));
/// An ink border 1.5 points wide (an error row).
pub(super) const INK_OUTLINE: Look = Look::NONE.border(Rgba::token(INK), 1.5);
/// A source folder card: paper with a hairline border.
pub(super) const FOLDER_CARD: Look = PAPER_FILL.border(Rgba::token(super::kit::LINE), 1.0);
/// A source folder card with an error: paper with an ink border.
pub(super) const FOLDER_CARD_ERROR: Look = PAPER_FILL.border(Rgba::token(INK), 1.0);
/// `color` text at `size` points.
const fn text(color: ColorToken, size: f32) -> Look {
    Look::NONE.text(Rgba::token(color)).font(Font::BODY.size(size))
}
/// Secondary 12-point text: a queue sheet text action.
pub(super) const QUEUE_ACTION_TEXT: Look = text(SECONDARY, 12.0);
/// Secondary 13-point text: an unselected chip.
pub(super) const CHIP_TEXT: Look = text(SECONDARY, 13.0);
/// Paper 13-point text: a selected chip.
pub(super) const CHIP_TEXT_ON: Look = text(PAPER, 13.0);
/// Secondary 14-point text: a sidebar "+ label" row, a text link.
pub(super) const LINK_TEXT: Look = text(SECONDARY, 14.0);
/// Secondary 15-point text: a back link.
pub(super) const BACK_TEXT: Look = text(SECONDARY, 15.0);
/// A sidebar count badge: paper 11-point text on ink.
pub(super) const BADGE: Look = text(PAPER, 11.0).fill(Rgba::token(INK));
/// An unselected settings choice: secondary 13-point text on the panel fill.
pub(super) const CHOICE: Look = text(SECONDARY, 13.0).fill(Rgba::token(PANEL));
/// The selected settings choice: paper 13-point text on ink.
pub(super) const CHOICE_ON: Look = text(PAPER, 13.0).fill(Rgba::token(INK));
/// A compact tab bar button: ink 14-point text on paper.
pub(super) const TAB_LOOKS: States = States::of(text(INK, 14.0).fill(Rgba::token(PAPER)));
/// The current compact tab: on the selected fill.
pub(super) const TAB_ON_LOOKS: States = States::of(text(INK, 14.0).fill(Rgba::token(SELECTED)));
/// A Find tool, a card or a row on paper.
pub(super) const PAPER_LOOKS: States = States::of(PAPER_FILL);
/// The current Find tool or route: on the selected fill.
pub(super) const SELECTED_LOOKS: States = States::of(SELECTED_FILL);
/// A key on the panel fill.
pub(super) const PANEL_LOOKS: States = States::of(PANEL_FILL);
/// A key on the ink fill (Play, Reconnect, an active mode key, an armed swipe action).
pub(super) const INK_LOOKS: States = States::of(INK_FILL);
/// A mode key on the key fill.
pub(super) const KEY_LOOKS: States = States::of(KEY_FILL);
/// `rest` while idle, `lit` while hovered or held.
const fn lit(rest: Look, lit: Look) -> States {
    States { hover: lit, pressed: lit, ..States::of(rest) }
}
/// An icon key with no resting plate: the panel fill while hovered or held, ink icon.
pub(super) const ICON_LOOKS: States = lit(INK_TEXT, INK_TEXT.fill(Rgba::token(PANEL)));
/// An icon key on the key plate.
pub(super) const ICON_PLATE_LOOKS: States = lit(INK_TEXT.fill(Rgba::token(KEY)), INK_TEXT.fill(Rgba::token(PANEL)));
/// An icon key in inverse (Play, an active mode): paper icon on ink.
pub(super) const ICON_INVERSE_LOOKS: States = States::of(INK_FILL.text(Rgba::token(PAPER)));
/// A sidebar route row: paper, the panel fill while hovered or held.
pub(super) const SIDE_ROW_LOOKS: States = lit(PAPER_FILL, PANEL_FILL);
/// The sidebar search row: the panel fill, the selected fill while hovered or held.
pub(super) const SEARCH_ROW_LOOKS: States = lit(PANEL_FILL, SELECTED_FILL);
/// The phone song menu's backdrop: near-black at a quarter alpha, the same in both appearances.
pub(super) const SCRIM: Look = Look::NONE.fill(SCRIM_INK.alpha(64));
/// Swift's scrim colour, the same in both appearances; overlays dim with it at a run-time alpha.
pub(super) const SCRIM_INK: Rgba = Rgba::rgb(10, 10, 10);
/// The waveform's bars: ink at 42 percent (Swift's opacity 0.42 of an opaque ink).
pub(super) const BAR_INK: Rgba = Rgba::token(INK).alpha(107);
/// The phone Find field's text: 26 points medium, tracking -0.52 (the value the field has always had).
pub(super) const FIND_FONT: Font = Font::BODY.size(26.0).weight(500).tracking(-0.52);
/// The command palette field's text: 20 points.
pub(super) const PALETTE_FONT: Font = Font::BODY.size(20.0);
/// The phone Find field's empty prompt: tertiary.
pub(super) const FIND_PROMPT: Look = Look::NONE.text(Rgba::token(TERTIARY)).font(FIND_FONT);
/// The command palette's empty prompt: secondary.
pub(super) const PALETTE_PROMPT: Look = Look::NONE.text(Rgba::token(SECONDARY)).font(PALETTE_FONT);
/// A song table row: paper, a 60 percent panel wash while hovered or held.
pub(super) const SONG_ROW_LOOKS: States = lit(PAPER_FILL, PANEL_WASH);
/// LocalWave's icon key `w` by `h` points: it dips to 0.96 while held, springing in at the instant pace and back at the base pace.
pub(super) const fn icon_key(w: f32, h: f32) -> pxui::IconButton<'static> {
    pxui::IconButton::sized(w, h).press_scale(0.96).press_curves(&PRESS)
}
/// The icon key sizes LocalWave repeats: 28, 36 and 44 points square, and 44 by 40.
pub(super) const ICON_28: pxui::IconButton = icon_key(28.0, 28.0);
pub(super) const ICON_36: pxui::IconButton = icon_key(36.0, 36.0);
pub(super) const ICON_44: pxui::IconButton = icon_key(44.0, 44.0);
pub(super) const ICON_44X40: pxui::IconButton = icon_key(44.0, 40.0);
const PRESS: [pxui::theme::Curve; 2] = [crate::motion::spring(crate::motion::INSTANT), crate::motion::spring(crate::motion::BASE)];
