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
/// Secondary 14-point text: a sidebar "+ label" row, a text link.
pub(super) const LINK_TEXT: Look = text(SECONDARY, 14.0);
/// Secondary 15-point text: a back link.
pub(super) const BACK_TEXT: Look = text(SECONDARY, 15.0);
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
    States::of(rest).hover(lit).pressed(lit)
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
/// A song or artist table row: paper, a 60 percent panel wash while hovered or held.
pub(super) const SONG_ROW_LOOKS: States = lit(PAPER_FILL, PANEL_WASH);
/// Swift's quick hover and release fade (`motion::out(QUICK)`, 140 ms) as a node's own transition.
pub(super) const FADE: pxui::ui::Transition = pxui::ui::Transition::Bezier { duration: crate::motion::QUICK, points: crate::motion::OUT };
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

/// The localwave-* body variants (`Button::variant`, `SelectionList::variant`, `Badge::variant`, `Progress::variant`): one number
/// each, so a body never mistakes another kind's variant for its own.
pub(super) mod variant {
    /// `localwave-button`: a full-width row (the sidebar's Search, "+ label" and shortcuts rows; source rows), laid out from its start.
    pub(in crate::shell) const ROW: u16 = 1;
    /// `localwave-side-row`: a sidebar route or list row.
    pub(in crate::shell) const SIDE_ROW: u16 = 2;
    /// `localwave-badge`: paper 11-point text on ink.
    pub(in crate::shell) const BADGE: u16 = 3;
    /// `localwave-progress`: the 4-point offline meter.
    pub(in crate::shell) const METER: u16 = 4;
    /// `localwave-button`: a 30-point tool key (Find's Play all/Keep offline, a folder's Play/Shuffle/Keep offline): icon and label.
    pub(in crate::shell) const TOOL: u16 = 5;
    /// `localwave-chip`: a Find filter chip.
    pub(in crate::shell) const CHIP: u16 = 6;
    /// `localwave-segmented`: the albums' sort keys, fit to their text, the chosen one framed in ink.
    pub(in crate::shell) const SORT: u16 = 7;
    /// `localwave-button`: a key with a leading icon and its own label colour (no look of its own unless the caller gives one).
    pub(in crate::shell) const KEY: u16 = 8;
    /// `localwave-icon`: a playback mode key, inverse while on, hovered or held too.
    pub(in crate::shell) const MODE: u16 = 9;
}

/// A tool key's label: 13 points in an 18.2-point line, whole-pixel width; the body inks it.
const TOOL_LABEL: Label = TEXT_13;
/// A tool key: paper, the panel fill while hovered or held, the selected fill while checked (the hover wash then stays off); its
/// icon and label secondary, ink while checked.
pub(super) const TOOL_LOOKS: States = lit(PAPER_FILL.text(Rgba::token(SECONDARY)), PANEL_FILL.text(Rgba::token(SECONDARY))).checked(SELECTED_FILL.text(Rgba::token(INK)));
/// A tool key (`localwave-button` / `TOOL`): fit to its icon and label, 30 points high, 10 in at each side, 6 between.
pub(super) const TOOL: pxui::Button = pxui::Button::DEFAULT.variant(variant::TOOL).size(Size::Fit, Size::Px(30.0)).pad(Edges::new(10.0, 0.0, 10.0, 0.0)).gap(6.0).label(TOOL_LABEL).look(&TOOL_LOOKS);
/// A Find filter chip (`localwave-chip` / `CHIP`).
pub(super) const CHIP: pxui::Checkbox = pxui::Checkbox::DEFAULT.variant(variant::CHIP);
/// The albums' sort keys (`localwave-segmented` / `SORT`): 30 points high, named for the strip.
pub(super) const SORT: pxui::Segmented = pxui::Segmented::DEFAULT.h_px(30.0).name("Sort albums").variant(variant::SORT);


/// Registers LocalWave's bodies once, outside frames and before its themes are parsed; the app themes' `KIND.body` lines select
/// them and a style's `variant` picks the structure inside one.
pub(crate) fn register_bodies(ui: &mut Ui) -> Result<(), pxui::Error> {
    ui.register_text_field_body("localwave-command", super::commands::field_body)?;
    ui.register_menu_row_body("localwave-menu", super::kit::menu_row_body)?;
    ui.register_toggle_body("localwave-toggle", super::settings::toggle_body)?;
    ui.register_dialog_body("localwave-sheet", super::files::sheet_body)?;
    ui.register_button_body("localwave-button", button_body)?;
    ui.register_selection_list_body("localwave-side-row", side_row_body)?;
    ui.register_badge_body("localwave-badge", badge_body)?;
    ui.register_progress_body("localwave-progress", progress_body)?;
    ui.register_checkbox_body("localwave-chip", chip_body)?;
    ui.register_segmented_body("localwave-segmented", segment_body)?;
    ui.register_icon_button_body("localwave-icon", icon_body)?;
    Ok(())
}

/// A row button's leading glyph (the shortcuts row's "?"): Geist Mono 11, secondary, in a 14-point column.
const ROW_GLYPH: Label = MONO_11.w_px(14.0).h_px(16.0).color(SECONDARY);
/// A row button's title without a `Button::label`: secondary 14-point text filling the row.
const ROW_TEXT: Look = LINK_TEXT;
/// LocalWave's button (registered as `localwave-button`). `ROW`: the box its caller sized, its contents in a row centred down it from
/// the start (`Button::pad`, `gap`): the leading glyph, the title (in `Button::label`, else secondary 14 points filling the row) and the
/// accessory at the end (in `Button::accessory_label`, else Geist Mono 11, secondary); the caller's look per state. `TOOL`: the same
/// row led by the icon (`Button::icon`, `icon_size` square) and the label, both in the look's ink; checked keeps its look while
/// hovered or disabled, ink while held, its fill fading over Swift's quick 140 ms (`FADE`). `KEY`: as `TOOL`, inked in the label's colour without a look. Any other
/// variant is the kit's own button.
fn button_body(ui: &mut Ui, c: &pxui::bodies::ButtonConfig) {
    use pxui::bodies::BodyConfig;
    if ![variant::ROW, variant::TOOL, variant::KEY].contains(&c.variant) {
        ui.slot(c.built_in);
        return;
    }
    let tool = c.variant != variant::ROW;
    let size = |kind: u8, points: f32| match kind { 1 => Size::Px(points), 2 => Size::Grow, _ => Size::Fit };
    let [l, t, r, b] = c.padding;
    ui.set_node_layout(&c.line, &Node::row().w(size(c.w_kind, c.w)).h(size(c.h_kind, c.h)));
    let row = Node::row().w(if c.w_kind == 0 { Size::Fit } else { Size::Grow }).h(Size::Grow).padding(Edges::new(l, t, r, b)).gap(c.gap).align(Align::Center);
    let row = if c.variant == variant::TOOL { row.transition(Some(FADE)) } else { row };
    ui.set_node_layout(&c.row, &row);
    let s = &c.state;
    let checked = tool && s.checked != 0;
    let state = State::default().hovered(s.hovered != 0 && !checked).pressed(s.pressed != 0).focused(s.focused != 0).disabled(s.disabled != 0 && !checked).checked(s.checked != 0);
    let states = c.states_of(&c.states);
    if let Some(states) = states {
        ui.style(&c.row, states, state);
    }
    // a tool's ink: the checked look's while checked, held or not
    let ink = states.map(|s| if checked { s.checked.text } else { s.pick(state).text }).or(c.label().and_then(|l| l.fields().color).map(Rgba::token));
    let mut _icon = None;
    if tool && let (Some(pxui::Icon::Draw(draw, word)), Some(ink)) = (c.icon(), ink) {
        let side = if c.icon_size > 0.0 { c.icon_size } else { 20.0 };
        let icon = ui.node("icon", Node::column().w_px(side).h_px(side));
        ui.canvas(&icon, move |paint, rect| draw(paint, rect, ink, word));
        _icon = Some(icon);
    }
    let (glyph, text, accessory) = (c.text_of(&c.lead), c.text_of(&c.label), c.text_of(&c.accessory));
    if !glyph.is_empty() && !tool {
        ui.label(glyph, ROW_GLYPH);
    }
    match (text.is_empty(), c.label()) {
        (true, _) => {}
        // the tool's label is the button's name: no second accessible node
        (false, Some(style)) if tool => {
            let _copy = ui.label(text, style.color(ink.and_then(Rgba::as_token).unwrap_or(SECONDARY)).role(Role::Unknown));
            #[cfg(test)]
            if let Some(icon) = _icon { super::find::tests::tool_built(text, icon, _copy) }
        }
        (false, Some(style)) => { ui.label(text, style); }
        (false, None) => {
            let title = ui.node("title", Node::column().w(Size::Grow).h(Size::Grow).look(ROW_TEXT));
            ui.text(&title, text);
        }
    }
    if !accessory.is_empty() {
        ui.label(accessory, c.accessory_label().unwrap_or(MONO_11.color(SECONDARY)));
    }
}

/// LocalWave's segmented (registered as `localwave-segmented`), the `SORT` look for any variant (LocalWave has no other strip): the
/// strip fit to its keys 8 apart; each key fit to its 13-point text 10 in at each side, secondary on paper, the chosen one ink on the
/// selected fill inside a 1.5-point ink frame.
fn segment_body(ui: &mut Ui, c: &pxui::bodies::SegmentedConfig) {
    use pxui::bodies::BodyConfig;
    ui.set_node_layout(&c.line, &Node::row().w(Size::Fit).h_px(30.0));
    ui.set_node_layout(&c.strip, &Node::row().w(Size::Fit).h(Size::Grow).gap(8.0));
    ui.set_node_layout(&c.node, &Node::row().w(Size::Fit).h_px(30.0).pad_x(10.0).align(Align::Center));
    let chosen = c.has_selected != 0 && c.selected == c.index;
    ui.style(&c.node, if chosen { &SELECTED_LOOKS } else { &PAPER_LOOKS }, State::default());
    ui.label(c.text_of(&c.text), TEXT_13.color(if chosen { INK } else { SECONDARY }).role(Role::Unknown));
    if chosen {
        ui.canvas_over(&c.node, move |paint, rect| {
            paint.fill(Rect::new(rect.x, rect.y, rect.w, 1.5), Fill::new(Rgba::token(INK)).fractional());
            paint.fill(Rect::new(rect.x, rect.y + rect.h - 1.5, rect.w, 1.5), Fill::new(Rgba::token(INK)).fractional());
            paint.fill(Rect::new(rect.x, rect.y + 1.5, 1.5, rect.h - 3.0), Fill::new(Rgba::token(INK)).fractional());
            paint.fill(Rect::new(rect.x + rect.w - 1.5, rect.y + 1.5, 1.5, rect.h - 3.0), Fill::new(Rgba::token(INK)).fractional());
        });
    }
}

/// LocalWave's icon button (registered as `localwave-icon`). `MODE` while on (`IconButton::selected`) and hovered or held: the
/// caller's size on the inverse plate (paper icon on ink), the app's icon in the 20-point square at its centre. Anything else is the
/// kit's icon button (`built_in`).
fn icon_body(ui: &mut Ui, c: &pxui::bodies::IconButtonConfig) {
    let s = &c.state;
    // the kit draws an on key right until it is hovered or held (its look picks those states over checked)
    if c.variant != variant::MODE || s.checked == 0 || s.hovered == 0 && s.pressed == 0 {
        ui.slot(c.built_in);
        return;
    }
    // shortcut: an on key has no 0.96 press dip (the kit's body picks the hover and held looks over checked); upgrade when it can keep checked
    let side = |kind: u8, points: f32| Size::Px(if kind == 1 { points } else { 20.0 });
    ui.set_node_layout(&c.node, &Node::column().w(side(c.w_kind, c.w)).h(side(c.h_kind, c.h)).align(Align::Center).justify(Justify::Center));
    ui.style(&c.node, &ICON_INVERSE_LOOKS, State::default().hovered(s.hovered != 0).pressed(s.pressed != 0).checked(true));
    if let Some(pxui::Icon::Draw(draw, word)) = c.icon() {
        let ink = ICON_INVERSE_LOOKS.idle.text;
        let glyph = ui.node("glyph", Node::column().w_px(20.0).h_px(20.0));
        ui.canvas(&glyph, move |paint, rect| draw(paint, rect, ink, word));
    }
}

/// A chip's text: 13 points, secondary, paper while checked.
const CHIP_LABEL: Label = TEXT_13;
/// LocalWave's checkbox (registered as `localwave-chip`), the `CHIP` look for any variant (LocalWave has no other checkbox): a
/// 32-point pill 12 in at each side, fit to its text centred down it, on the panel fill, ink while checked.
fn chip_body(ui: &mut Ui, c: &pxui::bodies::CheckboxConfig) {
    use pxui::bodies::BodyConfig;
    let on = c.on != 0;
    ui.set_node_layout(&c.row, &Node::row().w(Size::Fit).h_px(32.0).pad_x(12.0).align(Align::Center).flags(Flags::CLIP));
    ui.style(&c.row, if on { &INK_LOOKS } else { &PANEL_LOOKS }, State::default());
    ui.label(c.text_of(&c.label), CHIP_LABEL.color(if on { PAPER } else { SECONDARY }).role(Role::Unknown));
}

/// LocalWave's selectable row (registered as `localwave-side-row`). `SIDE_ROW`: a 32-point sidebar row 8 points in, paper with the
/// panel fill while hovered or held, on the selected fill while selected or under a list's drop look; any other variant keeps the
/// kit's row. The caller's cells follow.
fn side_row_body(ui: &mut Ui, c: &pxui::bodies::SelectionListConfig) {
    if c.variant == variant::SIDE_ROW {
        ui.set_node_layout(&c.row, &Node::row().w(Size::Grow).h_px(32.0).pad_x(8.0).align(Align::Center));
        let s = &c.state;
        let state = State::default().hovered(s.hovered != 0).pressed(s.pressed != 0).focused(s.focused != 0).disabled(s.disabled != 0);
        ui.style(&c.row, if s.checked != 0 || c.drop == 3 { &SELECTED_LOOKS } else { &SIDE_ROW_LOOKS }, state);
    }
    ui.slot(c.content);
}

/// A badge's text: paper 11 points in a 16-point line.
const BADGE_TEXT: Label = TEXT_11.h_px(16.0).color(PAPER);
/// A badge's plate: paper 11-point text on ink.
const BADGE_PLATE: States = States::of(text(PAPER, 11.0).fill(Rgba::token(INK)));
/// LocalWave's badge (registered as `localwave-badge`): its text in paper 11 points on an ink plate, the plate the text's own box.
fn badge_body(ui: &mut Ui, c: &pxui::bodies::BadgeConfig) {
    use pxui::bodies::BodyConfig;
    ui.set_node_layout(&c.node, &Node::row());
    let plate = ui.label(c.text_of(&c.text), BADGE_TEXT);
    ui.style(&plate, &BADGE_PLATE, State::default());
}

/// LocalWave's progress (registered as `localwave-progress`), the `METER` look: a 4-point line track 10 points under the row
/// above, 8 in from each side, filled in ink to the value (no label: the caller lays its own).
fn progress_body(ui: &mut Ui, c: &pxui::bodies::ProgressConfig) {
    ui.set_node_layout(&c.row, &Node::row().w(Size::Grow).h_px(4.0).margin(Edges::ZERO.t(10.0).l(8.0).r(8.0)));
    let track = ui.part(pxui::Part::TRACK, Node::row().w(Size::Grow).h(Size::Grow).look(LINE_FILL));
    let fraction = if c.has_value != 0 { c.value.clamp(0.0, 1.0) } else { 0.0 };
    ui.within(&track, &mut |ui| { ui.node("fill", Node::column().w(Size::Pct(fraction)).h(Size::Grow).look(INK_FILL)); });
}

#[cfg(test)]
mod body_tests {
    use super::*;
    use pxui::{FrameIn, Platform, ThemeHandle};

    /// The app theme with its U4a body lines (`localwave-button`, `-side-row`, `-badge`, `-progress`, `-chip`) or without them (the kit's
    /// bodies), the bodies registered either way.
    fn ui(app: bool) -> Ui {
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let source = include_str!("../../themes/app-light.theme");
        let kit: String = source.lines().filter(|l| !["button.body", "selection_list.body", "badge.body", "progress.body", "checkbox.body", "segmented.body", "icon_button.body"].iter().any(|k| l.starts_with(k))).map(|l| format!("{l}\n")).collect();
        let mut theme = ThemeHandle::parse(if app { source } else { &kit }).unwrap();
        crate::apply_palette(&mut theme, false);
        ui.set_theme(&theme).unwrap();
        ui
    }
    fn frames(ui: &mut Ui, build: &mut dyn FnMut(&mut Ui)) {
        for _ in 0..2 { crate::testing::frame(ui, &FrameIn::new(200.0, 300.0), build).unwrap(); }
    }
    fn node(ui: &Ui, name: &str) -> pxui::Rect { ui.a11y_nodes().iter().find(|n| ui.a11y_name_of(n) == name).unwrap_or_else(|| panic!("no {name}")).rect }
    fn colored(ui: &Ui, color: [u8; 4]) -> Vec<[f32; 4]> { ui.layers().flat_map(|l| l.instances.iter()).filter(|i| i.color == color).map(|i| i.rect).collect() }
    const INK_RGBA: [u8; 4] = [0x11, 0x11, 0x11, 255];
    const SELECTED_RGBA: [u8; 4] = [0xD3, 0xD3, 0xD3, 255];

    /// `button.body = localwave-button`: a `ROW` button is exactly its size (the kit's line adds a margin round its box), its accessory
    /// a label at its end; a variant-0 button draws exactly what the kit's body draws (`built_in`).
    #[test]
    fn localwave_button_lays_out_rows_and_leaves_variant_zero_to_the_kit() {
        const ROW: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Grow, Size::Px(34.0)).align(Justify::Start).pad(Edges::new(10.0, 0.0, 10.0, 0.0))
            .label(TEXT_14.grow().h_px(20.0).color(SECONDARY)).accessory("⌘K");
        let mut rows = |ui: &mut Ui| { ui.column(Node::column().w(Size::Grow), |ui| { ui.button("a", "Alpha", &ROW); ui.button("b", "Beta", &ROW); }); };
        let (mut app, mut kit) = (ui(true), ui(false));
        frames(&mut app, &mut rows);
        frames(&mut kit, &mut rows);
        let (a, b) = (node(&app, "Alpha"), node(&app, "Beta"));
        assert_eq!((a.h, b.y - a.y), (34.0, 34.0), "the ROW box is its size, stacked without a margin");
        assert_ne!(node(&kit, "Beta").y - node(&kit, "Alpha").y, 34.0, "the kit's line adds a margin: the theme line selects the body");
        let accessory = app.a11y_nodes().iter().filter(|n| app.a11y_name_of(n) == "⌘K").map(|n| n.rect).next().unwrap();
        assert!((accessory.x + accessory.w - (a.x + a.w - 10.0)).abs() < 1.0, "the accessory ends 10 in: {accessory:?} in {a:?}");
        let mut plain = |ui: &mut Ui| { ui.button_content("k", &pxui::Button::sized(60.0, 24.0).name("Key").look(&PANEL_LOOKS), |ui, _| { ui.label("Key", TEXT_12); }); };
        let (mut app, mut kit) = (ui(true), ui(false));
        frames(&mut app, &mut plain);
        frames(&mut kit, &mut plain);
        let draw = |ui: &Ui| ui.layers().flat_map(|l| l.instances.iter()).map(|i| (i.rect, i.color)).collect::<Vec<_>>();
        assert_eq!(draw(&app), draw(&kit), "variant 0 is the kit's button");
    }

    /// `selection_list.body = localwave-side-row`: a `SIDE_ROW` row is 32 points on the selected fill when selected (the kit's
    /// list row has its own look).
    #[test]
    fn localwave_side_row_draws_the_sidebar_row() {
        const LIST: pxui::SelectionList = pxui::SelectionList::DEFAULT.row_h(32.0).variant(variant::SIDE_ROW).bare().fit();
        let state = std::cell::RefCell::new(pxui::ListState { rows: vec![pxui::ListState::SELECTED, 0], anchor: None });
        let mut build = |ui: &mut Ui| {
            ui.selection_list("rows", 2, &|i| i as i64, &mut state.borrow_mut(), &LIST, &mut |ui, part, _| if let SectionPart::Row(i) = part { ui.label(["One", "Two"][i], SIDE_TEXT); });
        };
        const SIDE_TEXT: Label = TEXT_14.grow().h_px(20.0).color(SECONDARY);
        let (mut app, mut kit) = (ui(true), ui(false));
        frames(&mut app, &mut build);
        frames(&mut kit, &mut build);
        assert_eq!(colored(&app, SELECTED_RGBA), vec![[0.0, 0.0, 200.0, 32.0]], "the selected row on the selected fill");
        assert!(colored(&kit, SELECTED_RGBA).is_empty(), "the kit's row has its own look: the theme line selects the body");
    }

    /// `badge.body = localwave-badge` and `progress.body = localwave-progress`: the badge is paper text on an ink plate 16 points high;
    /// the meter a 4-point track 10 under its row's top and 8 in, inked to the value.
    #[test]
    fn localwave_badge_and_meter_draw_their_structures() {
        const BADGE_STYLE: pxui::Badge = pxui::Badge::DEFAULT.variant(variant::BADGE);
        const METER_STYLE: pxui::Progress = pxui::Progress::DEFAULT.variant(variant::METER);
        let mut build = |ui: &mut Ui| {
            ui.row(Node::row(), |ui| { ui.badge("badge", "Reconnect", "Reconnect", &BADGE_STYLE); });
            ui.column(Node::column().w_px(200.0), |ui| { ui.progress("meter", "Offline space", Some(0.25), &METER_STYLE); });
        };
        let (mut app, mut kit) = (ui(true), ui(false));
        frames(&mut app, &mut build);
        frames(&mut kit, &mut build);
        let plates = colored(&app, INK_RGBA);
        assert!(plates.iter().any(|r| r[1] == 0.0 && r[3] - r[1] == 16.0), "an ink plate 16 high: {plates:?}");
        let track = node(&app, "Offline space");
        assert_eq!((track.x, track.w, track.h), (8.0, 184.0, 4.0), "the track 8 in from each side, 4 high");
        assert!(plates.iter().any(|r| r[0] == 8.0 && r[2] == 8.0 + 46.0 && r[1] == track.y), "the fill inked to a quarter: {plates:?}");
        let kit_track = node(&kit, "Offline space");
        assert_ne!((kit_track.x, kit_track.w), (8.0, 184.0), "the kit's progress: the theme line selects the body");
    }

    /// `localwave-button` `TOOL`: the key fits its 10-point sides, icon, 6-point gap and whole-pixel label, 30 high, the selected fill
    /// while checked; `checkbox.body = localwave-chip`: a 32-point pill on ink while on.
    #[test]
    fn localwave_tool_and_chip_draw_their_structures() {
        let mut build = |ui: &mut Ui| {
            ui.row(Node::row(), |ui| {
                ui.button("play", "Play all", &TOOL.icon(super::super::kit::icon_drawing(super::super::kit::Icon::Play)).icon_size(16.0).selected(true));
                ui.checkbox("flac", "flac", true, &CHIP);
            });
        };
        let (mut app, mut kit) = (ui(true), ui(false));
        frames(&mut app, &mut build);
        frames(&mut kit, &mut build);
        let width = app.text_service().width("Play all", TOOL_LABEL, None).unwrap().ceil();
        let tool = node(&app, "Play all");
        assert_eq!((tool.w, tool.h), (10.0 + 16.0 + 6.0 + width + 10.0, 30.0), "the tool fits its parts");
        assert!(colored(&app, SELECTED_RGBA).contains(&[tool.x, tool.y, tool.x + tool.w, tool.y + tool.h]), "checked: the selected fill");
        assert_ne!(node(&kit, "Play all").w, tool.w, "the kit's button lays out its own way: the theme line selects the body");
        let chip = node(&app, "flac");
        assert_eq!(chip.h, 32.0);
        assert!(colored(&app, INK_RGBA).contains(&[chip.x, chip.y, chip.x + chip.w, chip.y + chip.h]), "on: the ink plate");
        assert!(!colored(&kit, INK_RGBA).iter().any(|r| r[3] - r[1] == 32.0), "the kit's checkbox: the theme line selects the body");
    }

    /// `segmented.body = localwave-segmented` (`SORT`): keys fit to their text 8 apart, 30 high, the chosen one framed in a 1.5-point ink
    /// line; `localwave-button` `KEY` fits its icon and label like `TOOL`, with no look of its own.
    #[test]
    fn localwave_sort_and_key_draw_their_structures() {
        const ITEMS: [pxui::Segment; 2] = [pxui::Segment::new("Artist"), pxui::Segment::new("Year")];
        const KEY: pxui::Button = pxui::Button::DEFAULT.variant(variant::KEY).size(Size::Fit, Size::Px(30.0)).pad(Edges::new(10.0, 0.0, 10.0, 0.0)).gap(6.0)
            .icon(super::super::kit::icon_drawing(super::super::kit::Icon::Shuffle)).icon_size(16.0).label(CAPTION);
        let mut build = |ui: &mut Ui| {
            ui.segmented("sort", &ITEMS, Some(1), &SORT);
            ui.button("shuffle", "Shuffle all", &KEY);
        };
        let (mut app, mut kit) = (ui(true), ui(false));
        frames(&mut app, &mut build);
        frames(&mut kit, &mut build);
        let (artist, year) = (node(&app, "Artist"), node(&app, "Year"));
        assert_eq!((year.x - (artist.x + artist.w), artist.h), (8.0, 30.0), "keys 8 apart, 30 high");
        assert!(colored(&app, INK_RGBA).iter().any(|r| r[0] == year.x && r[3] - r[1] == 1.5), "the chosen key's ink frame");
        assert!(!colored(&kit, INK_RGBA).iter().any(|r| r[3] - r[1] == 1.5), "the kit's strip: the theme line selects the body");
        let width = app.text_service().width("Shuffle all", CAPTION, None).unwrap().ceil();
        assert_eq!(node(&app, "Shuffle all").w, 10.0 + 16.0 + 6.0 + width + 10.0, "the key fits its parts");
    }
}
