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
/// Secondary 14-point text: a sidebar "+ label" row, a text link.
pub(super) const LINK_TEXT: Look = text(SECONDARY, 14.0);
/// An unselected settings choice: secondary 13-point text on the panel fill.
const CHOICE: Look = text(SECONDARY, 13.0).fill(Rgba::token(PANEL));
/// The selected settings choice: paper 13-point text on ink.
const CHOICE_ON: Look = text(PAPER, 13.0).fill(Rgba::token(INK));
const CHOICE_LOOKS: States = States::of(CHOICE);
const CHOICE_ON_LOOKS: States = States::of(CHOICE_ON);
/// A Find tool, a card or a row on paper.
pub(super) const PAPER_LOOKS: States = States::of(PAPER_FILL);
/// A key on the panel fill.
pub(super) const PANEL_LOOKS: States = States::of(PANEL_FILL);
/// A key on the ink fill (Play, Reconnect, an active mode key, an armed swipe action).
pub(super) const INK_LOOKS: States = States::of(INK_FILL);
/// A key with no plate in secondary ink (the phone back key).
pub(super) const DIM_LOOKS: States = States::of(SECONDARY_TEXT);
/// A phone play key: ink on the panel fill (a kept Keep offline key looks the same).
pub(super) const PLAY_KEY_LOOKS: States = States::of(PANEL_FILL.text(Rgba::token(INK)));
/// The phone Play key: paper on ink.
pub(super) const PLAY_PRIMARY_LOOKS: States = States::of(INK_FILL.text(Rgba::token(PAPER)));
/// A selection deck key: ink on the key plate, tertiary while disabled.
pub(super) const DECK_LOOKS: States = States::of(KEY_FILL.text(Rgba::token(INK))).disabled(KEY_FILL.text(Rgba::token(TERTIARY)));
/// The deck's Remove key: the danger colour on the key plate, tertiary while disabled.
pub(super) const DECK_REMOVE_LOOKS: States = States::of(KEY_FILL.text(Rgba::token(DANGER))).disabled(KEY_FILL.text(Rgba::token(TERTIARY)));
/// A phone mode key: ink on the key fill, paper on ink while on (checked).
pub(super) const MODE_KEY_LOOKS: States = States::of(KEY_FILL.text(Rgba::token(INK))).checked(INK_FILL.text(Rgba::token(PAPER)));
/// `rest` while idle, `lit` while hovered or held.
const fn lit(rest: Look, lit: Look) -> States {
    States::of(rest).hover(lit).pressed(lit)
}
/// An icon key with no resting plate: the panel fill while hovered or held, ink icon.
pub(super) const ICON_LOOKS: States = lit(INK_TEXT, INK_TEXT.fill(Rgba::token(PANEL)));
/// A secondary icon key with no resting plate (the phone rows' "…"): the panel fill while hovered or held.
pub(super) const ICON_DIM_LOOKS: States = lit(SECONDARY_TEXT, SECONDARY_TEXT.fill(Rgba::token(PANEL)));
/// An icon key on the key plate.
pub(super) const ICON_PLATE_LOOKS: States = lit(INK_TEXT.fill(Rgba::token(KEY)), INK_TEXT.fill(Rgba::token(PANEL)));
/// An icon key in inverse (Play, an active mode): paper icon on ink.
pub(super) const ICON_INVERSE_LOOKS: States = States::of(INK_FILL.text(Rgba::token(PAPER)));
/// A sidebar route row: paper, the panel fill while hovered or held, the selected fill while selected (checked).
pub(super) const SIDE_ROW_LOOKS: States = lit(PAPER_FILL, PANEL_FILL).checked(SELECTED_FILL);
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
/// A song or artist table row: paper, a 60 percent panel wash while hovered or held, the selected fill while selected (checked).
pub(super) const SONG_ROW_LOOKS: States = lit(PAPER_FILL, PANEL_WASH).checked(SELECTED_FILL);
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
    /// `localwave-button`: a full-width row (the sidebar's Search, "+ label" and shortcuts rows; source rows), laid out from its start;
    /// fit and centred, a phone word key (Select, Done) or swipe action.
    pub(in crate::shell) const ROW: u16 = 1;
    /// `localwave-row`: a sidebar route or list row.
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
    /// `localwave-search`: the desktop Find field, 44 points on paper, 15-point ink text, a 36-point clear key on the key plate.
    pub(in crate::shell) const DESKTOP: u16 = 9;
    /// `localwave-search`: the phone Find field, 56 points, 26-point medium text, the 44-point clear glyph, an ink rule under it.
    pub(in crate::shell) const COMPACT: u16 = 10;
    /// `localwave-row`: a desktop song row.
    pub(in crate::shell) const SONG_ROW: u16 = 11;
    /// `localwave-segmented`: the settings chips on the panel fill, the chosen one ink.
    pub(in crate::shell) const CHIPS: u16 = 12;
    /// `localwave-button`: a settings row (title, detail, the dotted chevron and the hairline).
    pub(in crate::shell) const SETTING: u16 = 13;
    /// `localwave-button`: an underlined text link, as wide as its text.
    pub(in crate::shell) const LINK: u16 = 14;
    /// `localwave-button`: the primary key, its label centred (on the caller's ink look).
    pub(in crate::shell) const PRIMARY: u16 = 15;
    /// `localwave-sheet` (`Dialog::variant`): a form (Edit Metadata), its labels and fields 4 apart.
    pub(in crate::shell) const FORM: u16 = 16;
    /// `localwave-tabs`: the phone library modes, each its label and count over a 2-point underline, 20 apart.
    pub(in crate::shell) const MODE: u16 = 17;
    /// `localwave-tabs`: the phone tab bar, its tabs sharing the bar 44 high.
    pub(in crate::shell) const TABBAR: u16 = 18;
    /// `localwave-chip`: the phone selection mark, a 20-point square (ink with a paper check when on, a secondary outline off).
    pub(in crate::shell) const MARK: u16 = 19;
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
    ui.register_picker_field_body("localwave-palette", super::commands::field_body)?;
    ui.register_picker_body("localwave-palette", super::commands::row_body)?;
    ui.register_menu_row_body("localwave-menu", super::kit::menu_row_body)?;
    ui.register_toggle_body("localwave-toggle", super::settings::toggle_body)?;
    ui.register_dialog_body("localwave-sheet", super::files::sheet_body)?;
    ui.register_button_body("localwave-button", button_body)?;
    ui.register_selection_list_body("localwave-row", row_body)?;
    ui.register_table_head_body("localwave-heading", super::songs::heading_body)?;
    ui.register_badge_body("localwave-badge", badge_body)?;
    ui.register_progress_body("localwave-progress", progress_body)?;
    ui.register_checkbox_body("localwave-chip", chip_body)?;
    ui.register_segmented_body("localwave-segmented", segment_body)?;
    ui.register_tabs_body("localwave-tabs", tabs_body)?;
    ui.register_search_field_body("localwave-search", search_body)?;
    ui.register_modal_body("localwave-modal", modal_body)?;
    ui.register_inspector_section_body("localwave-disclosure", super::onboarding::disclosure_body)?;
    Ok(())
}

/// A row button's leading glyph (the shortcuts row's "?"): Geist Mono 11, secondary, in a 14-point column.
const ROW_GLYPH: Label = MONO_11.w_px(14.0).h_px(16.0).color(SECONDARY);
/// A row button's title without a `Button::label`: secondary 14-point text filling the row.
const ROW_TEXT: Look = LINK_TEXT;
/// LocalWave's button (registered as `localwave-button`). `ROW`: the box its caller sized, its contents in a row centred down it from
/// the start (`Button::pad`, `gap`): the leading glyph, the title (in `Button::label`, else secondary 14 points filling the row), the
/// detail (in `Button::detail_label`, else the caption; `Button::axis(Axis::Column)` stacks it under the title from the top) and the
/// accessory at the end (in `Button::accessory_label`, else Geist Mono 11, secondary); the caller's look per state. `TOOL`: the same
/// row led by the icon (`Button::icon`, `icon_size` square) and the label, both in the look's ink (the kit's state priority picks it:
/// disabled, checked, held, hovered), its fill fading over Swift's quick 140 ms (`FADE`). `KEY`: as `TOOL` without the fade, inked in its look's colour for the state (else its label's), its detail before its label. Any other
/// variant is the kit's own button. `SETTING`: a settings row (`settings::row_body`). `LINK`: the label over a 1-point rule as wide
/// as it. `PRIMARY`: as `ROW`, its contents centred. `Button::centered` centres any of them along the row.
fn button_body(ui: &mut Ui, c: &pxui::bodies::ButtonConfig) {
    use pxui::bodies::BodyConfig;
    if c.variant == variant::SETTING {
        super::settings::row_body(ui, c);
        return;
    }
    if ![variant::ROW, variant::TOOL, variant::KEY, variant::LINK, variant::PRIMARY].contains(&c.variant) {
        ui.slot(c.built_in);
        return;
    }
    let tool = ![variant::ROW, variant::PRIMARY].contains(&c.variant);
    let [l, t, r, b] = c.padding;
    ui.set_node_layout(&c.line, &Node::row().w(c.width().unwrap_or(Size::Fit)).h(c.height().unwrap_or(Size::Fit)));
    // a column (`Button::axis`): a ROW stacks its title over its detail from the top; a centred KEY (the deck) its icon over its label
    let row = if c.content_axis() == Some(Axis::Column) { Node::column().align(if c.centered != 0 { Align::Center } else { Align::Start }) } else { Node::row().align(Align::Center) };
    let row = row.w(if c.width().is_none() { Size::Fit } else { Size::Grow }).h(Size::Grow).padding(Edges::new(l, t, r, b)).gap(c.gap);
    let row = if c.variant == variant::TOOL { row.transition(Some(FADE)) } else { row };
    let row = if c.centered != 0 || c.variant == variant::PRIMARY { row.justify(Justify::Center) } else { row };
    ui.set_node_layout(&c.row, &row);
    if c.variant == variant::LINK {
        // the text over a 1-point rule as wide as it
        ui.column(Node::column().w(Size::Fit).h(Size::Fit), |ui| {
            ui.label(c.text_of(&c.label), c.label().unwrap_or(TEXT_14.color(SECONDARY)));
            rule(ui, "underline", false);
        });
        return;
    }
    let s = &c.state;
    let state = State::default().hovered(s.hovered != 0).pressed(s.pressed != 0).focused(s.focused != 0).disabled(s.disabled != 0).checked(s.checked != 0);
    let states = c.states_of(&c.states);
    if let Some(states) = states {
        ui.style(&c.row, states, state);
    }
    let ink = states.map(|s| s.pick(state).text).or(c.label().and_then(|l| l.fields().color).map(Rgba::token));
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
    let detail = c.text_of(&c.detail);
    // a KEY's detail (the phone queue key's count) leads its label
    let lead_detail = c.variant == variant::KEY && !detail.is_empty();
    if lead_detail {
        ui.label(detail, c.detail_label().unwrap_or(CAPTION));
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
    if !detail.is_empty() && !lead_detail {
        ui.label(detail, c.detail_label().unwrap_or(CAPTION));
    }
    if !accessory.is_empty() {
        ui.label(accessory, c.accessory_label().unwrap_or(MONO_11.color(SECONDARY)));
    }
}

/// LocalWave's modal sheet (registered as `localwave-modal`): a bottom sheet (the queue), a top one (the palette and the shortcuts
/// card) and a full one (the phone's expanded player) are paper edge to edge, without the kit sheet's 1-point edge; a centred sheet
/// is the kit's as it is.
fn modal_body(ui: &mut Ui, c: &pxui::bodies::ModalConfig) {
    if matches!(c.presentation(), pxui::Presentation::Bottom | pxui::Presentation::Top | pxui::Presentation::Full) {
        let layout = ui.node_layout(&c.node).padding(Edges::ZERO);
        ui.set_node_layout(&c.node, &layout);
        ui.style(&c.node, &PAPER_LOOKS, State::default());
    }
    ui.slot(c.content);
}

/// LocalWave's segmented (registered as `localwave-segmented`). `CHIPS`: Swift's settings chips, the strip `padding` in on the panel
/// fill, each chip sharing it (28 high, at least 40 wide, 8 in, 13-point text), the stored choice ink with paper text. Any other
/// variant, the `SORT` look: the
/// strip fit to its keys 8 apart; each key fit to its 13-point text 10 in at each side, secondary on paper, the chosen one ink on the
/// selected fill inside a 1.5-point ink border.
fn segment_body(ui: &mut Ui, c: &pxui::bodies::SegmentedConfig) {
    use pxui::bodies::BodyConfig;
    if c.variant == variant::CHIPS {
        // the strip keeps the width its caller gave (`Segmented::w_px`)
        let w = ui.node_layout(&c.strip).w;
        ui.set_node_layout(&c.line, &Node::row().w(Size::Fit).h(Size::Fit));
        ui.set_node_layout(&c.strip, &Node::row().w(w).h_px(32.0).pad(c.padding));
        ui.style(&c.strip, &PANEL_LOOKS, State::default());
        // the stored choice: the controller applies a press a frame later
        let chosen = c.stored().or(c.selected()) == Some(c.index);
        ui.set_node_layout(&c.node, &Node::column().w(Size::Grow).h_px(28.0).pad_x(8.0).min_w(40.0));
        ui.style(&c.node, if chosen { &CHOICE_ON_LOOKS } else { &CHOICE_LOOKS }, State::default());
        ui.text(&c.node, c.text_of(&c.text));
        return;
    }
    ui.set_node_layout(&c.line, &Node::row().w(Size::Fit).h_px(30.0));
    ui.set_node_layout(&c.strip, &Node::row().w(Size::Fit).h(Size::Grow).gap(8.0));
    ui.set_node_layout(&c.node, &Node::row().w(Size::Fit).h_px(30.0).pad_x(10.0).align(Align::Center));
    let chosen = c.selected() == Some(c.index);
    ui.style(&c.node, if chosen { &SORT_CHOSEN } else { &PAPER_LOOKS }, State::default());
    ui.label(c.text_of(&c.text), TEXT_13.color(if chosen { INK } else { SECONDARY }).role(Role::Unknown));
}
/// A phone tab bar tab: paper, on the selected fill while in use (checked).
const TABBAR_LOOKS: States = States::of(PAPER_FILL).checked(SELECTED_FILL);
/// A library mode's label: secondary, ink while in use.
const MODE_LOOKS: States = States::of(SECONDARY_TEXT).checked(INK_TEXT);
/// A library mode's underline: paper, ink while in use.
const UNDERLINE_LOOKS: States = States::of(PAPER_FILL).checked(INK_FILL);
/// LocalWave's tabs (registered as `localwave-tabs`). `MODE`: Swift's phone library modes: the bar fit to its tabs 20 apart, 44
/// high; each tab its 15-point label and the caption count 6 apart in a 42-point line over a 2-point underline, the label and underline
/// ink on the tab in use. Any other variant, the `TABBAR` look: the bar filling its parent, its tabs sharing it 44 high from its top, each its
/// 14-point ink title centred on paper, the tab in use on the selected fill.
fn tabs_body(ui: &mut Ui, c: &pxui::bodies::TabsConfig) {
    use pxui::bodies::BodyConfig;
    let s = &c.state;
    let state = State::default().hovered(s.hovered != 0).pressed(s.pressed != 0).checked(s.checked != 0);
    // the bar draws nothing of its own (the kit's header look and its rule are off)
    ui.style(&c.bar, &BARE, State::default());
    if c.variant == variant::MODE {
        ui.set_node_layout(&c.bar, &Node::row().w(Size::Fit).h_px(44.0).gap(20.0));
        ui.set_node_layout(&c.node, &Node::column().w(Size::Fit).h(Size::Grow));
        ui.row(Node::row().w(Size::Fit).h_px(42.0).gap(6.0).align(Align::Center), |ui| {
            let ink = MODE_LOOKS.pick(state).text.as_token().unwrap_or(SECONDARY);
            ui.label(c.text_of(&c.text), TEXT_15.color(ink).role(Role::Unknown));
            ui.label(c.text_of(&c.detail), CAPTION);
        });
        let underline = ui.node("underline", Node::row().w(Size::Grow).h_px(2.0));
        ui.style(&underline, &UNDERLINE_LOOKS, state);
        return;
    }
    ui.set_node_layout(&c.bar, &Node::row().w(Size::Grow).h(Size::Grow));
    ui.set_node_layout(&c.node, &Node::row().w(Size::Grow).h_px(44.0).align(Align::Center).justify(Justify::Center));
    ui.style(&c.node, &TABBAR_LOOKS, state);
    ui.label(c.text_of(&c.text), BODY_INK.role(Role::Unknown));
}

/// The chosen sort key: the selected fill inside a 1.5-point ink border.
const SORT_CHOSEN: States = States::of(SELECTED_FILL.border(Rgba::token(INK), 1.5));

/// The desktop Find field's text: 15 points, ink.
const SEARCH_TEXT: Look = INK_TEXT.font(Font::BODY.size(15.0));
/// The phone Find field's text: 26 points medium, ink.
const FIND_TEXT: Look = INK_TEXT.font(FIND_FONT);
/// LocalWave's search field (registered as `localwave-search`). `COMPACT`: Swift's phone Find field: a 56-point line of the 26-point
/// editor (the placeholder tertiary; the app focuses it when Find opens) and the 44-point clear glyph 10 points past the gutter, an ink rule
/// 1.5 points under it. Any other variant (`DESKTOP`): a 44-point paper line of the 15-point editor and a 36-point clear key on the key
/// plate. The clear key shows on an empty query too, as Swift's does.
fn search_body(ui: &mut Ui, c: &pxui::bodies::SearchFieldConfig) {
    let compact = c.variant == variant::COMPACT;
    let (height, side, keys) = if compact { (56.0, 44.0, &ICON_LOOKS) } else { (44.0, 36.0, &ICON_PLATE_LOOKS) };
    ui.set_node_layout(&c.line, &Node::row().w(Size::Grow));
    ui.set_node_layout(&c.well, &Node::column().w(Size::Grow));
    ui.style(&c.well, if compact { &BARE } else { &PAPER_LOOKS }, State::default());
    let line = ui.node("line", Node::row().w(Size::Grow).h_px(height).align(Align::Center));
    ui.within(&line, &mut |ui| {
        let editor = ui.part(pxui::Part::EDITOR, Node::column().w(Size::Grow).h(Size::Grow).inset(Edges::ZERO.l(if compact { 0.0 } else { 2.0 })).look(if compact { FIND_TEXT } else { SEARCH_TEXT }));
        if !ui.slot(c.editor) && compact {
            ui.style(&editor, &States::of(FIND_PROMPT), State::default());
        }
        let clear = ui.part(pxui::Part::CLEAR, Node::column().w_px(side).h_px(side).align(Align::Center).justify(Justify::Center).margin(Edges::ZERO.r(if compact { -10.0 } else { 0.0 })));
        // the kit owns the key's state (U2b.4.23); its look's ink draws the X
        let state = State::default().hovered(c.clear.hovered != 0).pressed(c.clear.pressed != 0);
        ui.style(&clear, keys, state);
        let ink = keys.pick(state).text;
        ui.within(&clear, &mut |ui| {
            let glyph = ui.node("glyph", Node::column().w_px(20.0).h_px(20.0));
            ui.canvas(&glyph, move |paint, rect| paint_icon(paint, Icon::Clear, rect.x, rect.y, ink));
        });
    });
    if compact {
        ui.node("rule", Node::column().w(Size::Grow).h_px(1.5).look(INK_FILL));
    }
}

/// A chip's text: 13 points, secondary, paper while checked.
const CHIP_LABEL: Label = TEXT_13;
/// LocalWave's checkbox (registered as `localwave-chip`). `MARK`: the phone selection mark (`variant::MARK`). Any other variant, the `CHIP` look: a
/// 32-point pill 12 in at each side, fit to its text centred down it, on the panel fill, ink while checked.
fn chip_body(ui: &mut Ui, c: &pxui::bodies::CheckboxConfig) {
    use pxui::bodies::BodyConfig;
    let on = c.on != 0;
    if c.variant == variant::MARK {
        ui.set_node_layout(&c.row, &Node::column().w_px(20.0).h_px(20.0));
        let (ink, outline, paper) = (Rgba::token(INK), Rgba::token(SECONDARY), Rgba::token(PAPER));
        ui.canvas(&c.row, move |paint, rect| {
            if on {
                paint.fill(Rect::new(rect.x, rect.y, 20.0, 20.0), Fill::new(ink));
                crate::icons::offline(paint, pxui::geom::Rect { x: rect.x + 3.0, y: rect.y + 3.0, w: 14.0, h: 14.0 }, true, paper);
            } else {
                paint.stroke(Rect::new(rect.x, rect.y, 20.0, 20.0), Stroke::new(outline).width(1.5));
            }
        });
        return;
    }
    ui.set_node_layout(&c.row, &Node::row().w(Size::Fit).h_px(32.0).pad_x(12.0).align(Align::Center).flags(Flags::CLIP));
    ui.style(&c.row, if on { &INK_LOOKS } else { &PANEL_LOOKS }, State::default());
    ui.label(c.text_of(&c.label), CHIP_LABEL.color(if on { PAPER } else { SECONDARY }).role(Role::Unknown));
}

/// LocalWave's selectable row (registered as `localwave-row`). `SIDE_ROW`: a 32-point sidebar row 8 points in, paper with the
/// panel fill while hovered or held, on the selected fill while selected or under a list's drop look. `SONG_ROW`: a song row as tall as
/// its cells (no inset; they place themselves), paper with the panel wash while hovered or held, on the selected fill while selected
/// (a payload carried over it draws no line: the songs list takes no drops). Any other variant keeps the kit's row. The caller's cells
/// follow.
fn row_body(ui: &mut Ui, c: &pxui::bodies::SelectionListConfig) {
    let s = &c.state;
    // one look table a variant; the kit picks its look for the state (disabled > checked > pressed > hovered, U2b.4.20)
    let state = State::default().hovered(s.hovered != 0).pressed(s.pressed != 0).focused(s.focused != 0).disabled(s.disabled != 0).checked(s.checked != 0);
    if c.variant == variant::SIDE_ROW {
        ui.set_node_layout(&c.row, &Node::row().w(Size::Grow).h_px(32.0).pad_x(8.0).align(Align::Center));
        ui.style(&c.row, &SIDE_ROW_LOOKS, state);
    } else if c.variant == variant::SONG_ROW {
        ui.set_node_layout(&c.row, &Node::column().w(Size::Grow).h(Size::Fit));
        ui.style(&c.row, &SONG_ROW_LOOKS, state);
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
    let fraction = c.value().map_or(0.0, |v| v.clamp(0.0, 1.0));
    ui.within(&track, &mut |ui| { ui.node("fill", Node::column().w(Size::Pct(fraction)).h(Size::Grow).look(INK_FILL)); });
}

#[cfg(test)]
mod body_tests {
    use super::*;
    use pxui::{FrameIn, Platform, ThemeHandle};

    /// The app theme with its U4a body lines (`localwave-button`, `-row`, `-heading`, `-badge`, `-progress`, `-chip`) or without them (the kit's
    /// bodies), the bodies registered either way.
    fn ui(app: bool) -> Ui {
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let source = include_str!("../../themes/app-light.theme");
        let kit: String = source.lines().filter(|l| !["button.body", "selection_list.body", "badge.body", "progress.body", "checkbox.body", "segmented.body", "search_field.body", "modal.body", "table_head.body", "inspector_section.body"].iter().any(|k| l.starts_with(k))).map(|l| format!("{l}\n")).collect();
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

    /// U4a.7/U4a.8 bodies: `CHIPS` chips share their strip, 28 high, the stored choice on ink; a `SETTING` row is at least 60 high; a
    /// `LINK` underlines its text with a 1-point rule as wide as it; `PRIMARY` centres its label on ink; `localwave-disclosure` is a
    /// 19.5-point header, its content under it while open. The kit's bodies draw none of these.
    #[test]
    fn settings_and_onboarding_bodies_draw_their_structures() {
        const ITEMS: [pxui::Segment; 2] = [pxui::Segment::new("4 GB").id(4), pxui::Segment::new("8 GB").id(8)];
        const CHIPS: pxui::Segmented = pxui::Segmented::DEFAULT.h_px(32.0).padding(2.0).variant(variant::CHIPS).w_px(120.0);
        const SETTING: pxui::Button = pxui::Button::DEFAULT.variant(variant::SETTING).detail("Detail");
        const LINK: pxui::Button = pxui::Button::DEFAULT.variant(variant::LINK).size(Size::Fit, Size::Px(44.0)).centered().pad(Edges::ZERO).label(LEAD_INK);
        const PRIMARY: pxui::Button = pxui::Button::DEFAULT.variant(variant::PRIMARY).size(Size::Px(200.0), Size::Px(52.0)).label(TEXT_15.color(PAPER)).look(&INK_LOOKS);
        let mut build = |ui: &mut Ui| { ui.column(Node::column().w(Size::Grow), |ui| {
            ui.segmented("chips", &ITEMS, Some(1), &CHIPS);
            ui.button("rescan", "Rescan", &SETTING);
            ui.button("link", "Link", &LINK);
            ui.button("primary", "Go", &PRIMARY);
            ui.disclosure("more", "More", true, &pxui::Disclosure::DEFAULT, &mut |ui| { ui.label("Inside", TEXT_13); });
        }); };
        let (mut app, mut kit) = (ui(true), ui(false));
        frames(&mut app, &mut build);
        frames(&mut kit, &mut build);
        let ink_chips = |ui: &Ui| colored(ui, INK_RGBA).into_iter().filter(|r| r[3] - r[1] == 28.0).count();
        assert_eq!((ink_chips(&app), ink_chips(&kit)), (1, 0), "the stored chip is a 28-point ink chip");
        assert_eq!((node(&app, "Rescan").h >= 60.0, node(&kit, "Rescan").h >= 60.0), (true, false), "a settings row is at least 60 high");
        let link = node(&app, "Link");
        let rules = |ui: &Ui| ui.layers().flat_map(|l| l.instances.iter()).filter(|i| i.rect[3] - i.rect[1] == 1.0 && i.rect[1] >= link.y && i.rect[1] < link.y + link.h).count();
        assert!(rules(&app) >= 1 && rules(&kit) == 0, "the link is underlined");
        let plates = |ui: &Ui| colored(ui, INK_RGBA).into_iter().filter(|r| r[2] - r[0] == 200.0 && r[3] - r[1] == 52.0).count();
        let go = node(&app, "Go");
        let label = colored(&app, [0xF3, 0xF3, 0xF3, 255]).into_iter().filter(|r| r[1] >= go.y && r[3] <= go.y + go.h).fold((f32::MAX, 0.0f32), |(a, b), r| (a.min(r[0]), b.max(r[2])));
        assert_eq!(plates(&app), 1, "the primary key is an ink plate");
        assert!(((label.0 - go.x) - (go.x + go.w - label.1)).abs() <= 1.0, "the primary key centres its label: {label:?} in {go:?}");
        let (head, inside) = (node(&app, "More"), node(&app, "Inside"));
        assert_eq!(head.h, 19.5, "the disclosure header is 19.5 high");
        assert!(inside.y >= head.y + head.h, "the content is under the header");
        assert_ne!(node(&kit, "More").h, 19.5, "the theme line selects the disclosure body");
    }

    /// `localwave-button` / `ROW` with `Button::axis(Axis::Column)`: the detail sits under the title, both from the box's start.
    #[test]
    fn a_row_column_stacks_its_detail_under_its_title() {
        const TITLES: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Grow, Size::Px(60.0)).align(Justify::Start)
            .axis(Axis::Column).label(TEXT_20.grow()).detail_label(CAPTION.grow().h_px(18.2)).detail("From Library");
        let mut ui = ui(true);
        frames(&mut ui, &mut |ui| { ui.column(Node::column().w(Size::Grow), |ui| { ui.button("t", "Queue", &TITLES); }); });
        // the button is named by its title; the detail starts under the title's 28-point line
        let (button, detail) = (node(&ui, "Queue"), node(&ui, "From Library"));
        assert_eq!((detail.x, detail.y), (button.x, button.y + 28.0), "{button:?} {detail:?}");
    }

    /// `modal.body = localwave-modal`: a bottom, top or full sheet's content starts at the sheet's own edge (no kit edge or inset); a
    /// centred modal is the kit's sheet, its content one point in.
    #[test]
    fn localwave_modal_dresses_bottom_sheets_and_leaves_centred_ones_to_the_kit() {
        for (present, inset) in [(pxui::Presentation::Bottom, 0.0), (pxui::Presentation::Top, 0.0), (pxui::Presentation::Full, 0.0), (pxui::Presentation::Centered, 1.0)] {
            for (app, inset) in [(true, inset), (false, 1.0)] {
                let mut ui = ui(app);
                let sheet = Overlay::DEFAULT.present(present).name("Sheet");
                frames(&mut ui, &mut |ui| {
                    ui.modal("m", &sheet, |ui| {
                        let c = ui.node("c", Node::column().w_px(100.0).h_px(50.0).role(Role::Label));
                        ui.a11y_name(&c, "Content");
                    });
                });
                let (s, c) = (node(&ui, "Sheet"), node(&ui, "Content"));
                assert_eq!((c.x - s.x, c.y - s.y), (inset, inset), "{present:?} app {app}: {s:?} {c:?}");
            }
        }
    }

    /// `dialog.body = localwave-sheet`: `FORM` stacks its content 4 apart (Edit Metadata), any other variant 12.
    #[test]
    fn localwave_sheet_stacks_a_form_four_apart() {
        for (dialog, gap) in [(Dialog::DEFAULT.variant(variant::FORM), 4.0), (Dialog::DEFAULT, 12.0)] {
            let mut ui = ui(true);
            frames(&mut ui, &mut |ui| {
                ui.dialog("d", "Title", &[DialogButton::new("OK")], &dialog, &mut |ui| {
                    ui.label("Above", TEXT_12.grow().h_px(16.0));
                    ui.label("Below", TEXT_12.grow().h_px(16.0));
                });
            });
            let (a, b) = (node(&ui, "Above"), node(&ui, "Below"));
            assert_eq!(b.y - (a.y + a.h), gap, "variant {}: {a:?} {b:?}", dialog.variant);
        }
    }

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

    /// `selection_list.body = localwave-row`: a `SIDE_ROW` row is 32 points on the selected fill when selected (the kit's
    /// list row has its own look).
    #[test]
    fn localwave_side_row_draws_the_sidebar_row() {
        const LIST: pxui::SelectionList = pxui::SelectionList::DEFAULT.row_h(32.0).variant(variant::SIDE_ROW).bare().fit();
        let state = std::cell::RefCell::new(pxui::ListState { rows: vec![pxui::ListState::SELECTED, 0], ..pxui::ListState::new() });
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

    /// `selection_list.body = localwave-row` / `SONG_ROW`: a song row is as tall as its cells (no inset; the list's row height is only
    /// the estimate), on the selected fill when selected. `table_head.body = localwave-heading`: the "#" heading's text starts 32
    /// points into its column and Title's 8 (the kit's heading insets both alike).
    #[test]
    fn localwave_song_row_and_heading_draw_their_structures() {
        const LIST: pxui::SelectionList = pxui::SelectionList::DEFAULT.multi().row_h(38.0).measured().variant(variant::SONG_ROW).bare();
        const COLUMNS: [pxui::Column; 2] = [pxui::Column::new("#").id(0).w_px(59.0).grip(false), pxui::Column::new("Title").id(1).w_px(139.0).grip(false)];
        let state = std::cell::RefCell::new(pxui::ListState { rows: vec![pxui::ListState::SELECTED, 0], ..pxui::ListState::new() });
        let table = std::cell::RefCell::new(pxui::TableState::new());
        let mut build = |ui: &mut Ui| {
            ui.column(Node::column().w(Size::Grow).h_px(40.0), |ui| { ui.table_head("head", &COLUMNS, &mut table.borrow_mut(), &pxui::TableHead::DEFAULT.h_px(40.0)); });
            ui.column(Node::column().w(Size::Grow).h(Size::Grow), |ui| {
                ui.selection_list("rows", 2, &|i| i as i64, &mut state.borrow_mut(), &LIST, &mut |ui, part, _| if let SectionPart::Row(_) = part { ui.node("cells", Node::column().w(Size::Grow).h_px(44.0)); });
            });
        };
        let (mut app, mut kit) = (ui(true), ui(false));
        frames(&mut app, &mut build);
        frames(&mut kit, &mut build);
        assert_eq!(colored(&app, SELECTED_RGBA), vec![[0.0, 40.0, 200.0, 84.0]], "the selected row, as tall as its cells (44), on the selected fill");
        assert!(colored(&kit, SELECTED_RGBA).is_empty(), "the kit's row has its own look: the theme line selects the body");
        // the headings' text: the glyphs in the heading band (instance rects are x0, y0, x1, y1), leftmost in each column
        let first = |ui: &Ui, from: f32, to: f32| ui.layers().flat_map(|l| l.instances.iter()).map(|i| i.rect)
            .filter(|r| r[3] <= 40.0 && r[2] - r[0] < 20.0 && r[0] >= from && r[0] < to).map(|r| r[0]).fold(f32::MAX, f32::min);
        let (number, title) = (first(&app, 0.0, 60.0), first(&app, 60.0, 200.0));
        assert!((32.0..34.0).contains(&number) && (68.0..70.0).contains(&title), "headings 32 and 8 in: # at {number}, Title at {title}");
        let (number, title) = (first(&kit, 0.0, 60.0), first(&kit, 60.0, 200.0));
        assert!((title - 60.0 - number).abs() < 2.0, "the kit insets both alike: # at {number}, Title at {title}");
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

    /// `localwave-button` `KEY` inks its label in its look's colour for the state (U2b.4.30, the caller's table as given): the phone
    /// mode key's label is ink on the key fill, and paper (no ink glyph) on the ink fill while on.
    #[test]
    fn localwave_key_inks_its_label_in_its_looks_colour() {
        const MODE: pxui::Button = pxui::Button::DEFAULT.variant(variant::KEY).size(Size::Fit, Size::Px(44.0)).gap(6.0).label(TEXT_14).look(&MODE_KEY_LOOKS);
        for on in [false, true] {
            let mut ui = ui(true);
            frames(&mut ui, &mut |ui| { ui.button("k", "Shuffle", &MODE.selected(on)); });
            let key = node(&ui, "Shuffle");
            let glyphs = colored(&ui, INK_RGBA).iter().filter(|r| r[0] > key.x && r[2] < key.x + key.w && r[1] >= key.y && r[3] <= key.y + key.h).count();
            assert_eq!(glyphs > 0, !on, "on {on}: {glyphs} ink glyphs in {key:?}");
        }
    }

    /// `segmented.body = localwave-segmented` (`SORT`): keys fit to their text 8 apart, 30 high, the chosen one bordered in a 1.5-point ink
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
        // the border's edges in ink along the chosen key (whole-pixel snapped), over its selected fill
        let edged = |ui: &Ui, key: pxui::Rect| colored(ui, INK_RGBA).iter().filter(|r| r[0] == key.x && r[2] == key.x + key.w && (r[1] == key.y.max(0.0) || r[3] == key.y + key.h)).count();
        assert_eq!(edged(&app, year), 2, "the chosen key's ink border, top and bottom");
        assert!(colored(&app, SELECTED_RGBA).iter().any(|r| r[0] == year.x && r[2] == year.x + year.w), "on the selected fill");
        assert_eq!(edged(&kit, node(&kit, "Year")), 0, "the kit's strip: the theme line selects the body");
        let width = app.text_service().width("Shuffle all", CAPTION, None).unwrap().ceil();
        assert_eq!(node(&app, "Shuffle all").w, 10.0 + 16.0 + 6.0 + width + 10.0, "the key fits its parts");
    }
}
