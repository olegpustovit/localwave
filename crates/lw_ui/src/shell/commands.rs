//! Preserve Command-K/slash opening, wrapping selection, selected-row visibility, UTF-8 input,
//! focus restoration and dismissal. Read scalar signals instead of owned key vectors. Raw key
//! queries do not consume events; focus/modal ownership must be arbitrated by the caller.
//!
use pxui::bodies::BodyConfig;
use super::*;
use crate::commands::{Action, Command, Icon, Results};

impl ShellState {
    /// Rows to show: the worker's for this query, or for an empty query the static command list,
    /// so the palette is usable before (and without) a worker round trip.
    pub fn command_items<'a>(&self, catalog: &'a Catalog) -> &'a [crate::commands::Item] {
        match self.command_results(catalog) {
            Some(results) => &results.items,
            None if self.command_query.as_str().trim().is_empty() => crate::commands::static_items(),
            None => &[],
        }
    }

    pub fn command_results<'a>(&self, catalog: &'a Catalog) -> Option<&'a Results> {
        catalog
            .commands
            .as_deref()
            .filter(|results| self.commands_visible && results.query == self.command_query)
    }

    pub fn activate_command(
        &mut self,
        catalog: &Catalog,
        key: u64,
        playing: bool,
        dark: bool,
    ) -> Option<Intent> {
        let action = self.command_items(catalog).iter().find(|item| item.key == key)?.action;
        let query = self.command_query;
        let intent = match action {
            Action::Play(index) => {
                self.command_results(catalog)?.songs.get(index)?;
                return Some(Intent::PlayCommand(key));
            }
            Action::Navigate(route) => Intent::Navigate(route),
            Action::Folder(location) => {
                self.apply(Intent::Navigate(Route::Files));
                Intent::Files(crate::files::Request::Browse(location))
            }
            Action::Find => {
                self.apply(Intent::Navigate(Route::Find));
                Intent::SearchResults(query)
            }
            Action::Command(command) => match command {
                Command::PlayPause => {
                    if playing {
                        Intent::Pause
                    } else {
                        Intent::Play
                    }
                }
                Command::Shuffle => Intent::Shuffle,
                Command::Repeat => Intent::Repeat,
                // shortcut: use Files' existing picker choices until pxui supports mixed file/folder selection.
                Command::AddMusic => Intent::Navigate(Route::Files),
                Command::Rescan(location) => Intent::Files(crate::files::Request::Rescan(location)),
                Command::Network => Intent::Setting(lw_core::preferences::Change::ToggleNoNetwork),
                Command::Theme => {
                    Intent::Setting(lw_core::preferences::Change::Appearance(if dark {
                        lw_core::preferences::Appearance::Light
                    } else {
                        lw_core::preferences::Appearance::Dark
                    }))
                }
                Command::Panel => Intent::TogglePanel,
                Command::Shortcuts => Intent::ShowShortcuts,
                Command::Settings => Intent::ShowSettings,
                Command::Favorite => return None,
            },
        };
        self.apply(Intent::CloseCommands);
        Some(intent)
    }
}

/// The palette's query field (registered as `localwave-palette`, picker_field kind): 59 points high and 18 in at each side, a
/// hairline under it; the query in 20 points on its editor part; the prompt secondary and ellipsized to the card while empty. It
/// lays the results out: at most 420 points (and 176 short of the window), at least 60, 6 in.
pub(crate) fn field_body(ui: &mut Ui, c: &pxui::bodies::PickerFieldConfig) {
    let height = ui.content_rect().h;
    ui.set_node_layout(&c.search, &Node::row().w(Size::Grow).h_px(59.0).padding(Edges::ZERO.l(FIELD_SIDE).r(FIELD_SIDE)).margin(Edges::ZERO.b(1.0)));
    ui.style(&c.search, &pxui::theme::States::of(Look::NONE), State::default());
    ui.part(pxui::Part::EDITOR, Node::column().w(Size::Grow).h_px(59.0).look(PALETTE_QUERY));
    // the hairline spans the field (the card) in its bottom margin
    ui.node("rule", Node::DEFAULT.w(Size::Rel(card_width)).h_px(1.0).at(0.0, 59.0).look(LINE_FILL));
    if c.empty != 0 {
        // the picker's placeholder (its accessible name too), ellipsized to the field
        let placeholder = ui.node("prompt", Node::row().w(Size::Rel(prompt_width)).h_px(59.0).align(Align::Center).at(20.0, 0.0));
        let style = Label::BODY.size(20.0).line_height(28.0).weight(PALETTE_FONT.weight).lines(1).ellipsis();
        ui.within(&placeholder, &mut |ui| {
            let line = ui.node("line", Node::column().w(Size::Grow).h_px(28.0).look(PALETTE_PROMPT));
            ui.text_line_box(&line, c.text_of(&c.placeholder), style, &[]).unwrap();
        });
    }
    let results = 420.0_f32.min((height - 176.0).max(60.0));
    ui.set_node_layout(&c.list, &Node::column().w(Size::Grow).h(Size::Fit).padding(Edges::new(6.0, 6.0, 6.0, 0.0)).min_h(60.0).max_h(results));
    // The bottom inset scrolls with the results and leaves the bottom of the viewport visible.
    ui.within(&c.list, &mut |ui| spacer(ui, "results-bottom", 6.0));
}

/// The query field's sides: 18 points in from the card's edges.
const FIELD_SIDE: f32 = 18.0;
/// The card's width from the field's inner width (the field spans the card).
extern "C" fn card_width(inner: f32) -> f32 { inner + 2.0 * FIELD_SIDE }
/// The prompt's width: the card less 20 points at each side.
extern "C" fn prompt_width(inner: f32) -> f32 { (inner + 2.0 * FIELD_SIDE - 40.0).max(1.0) }

/// A palette row (registered as `localwave-palette`, picker kind): 44 points, 12 in and 12 apart, the selected fill on the keyboard's
/// row; its artwork (28 points on the panel fill, the image when it has one) or its icon (`PickArt::Draw` of the `Icon`), the label, the
/// hint. A row that starts a group has its heading above it: secondary 12 points, 12 in, its line 10 below the row above and 4 above
/// its row.
pub(crate) fn row_body(ui: &mut Ui, c: &pxui::bodies::PickerConfig) {
    if let Some(head) = c.head() {
        ui.style(&head, &PALETTE_HEADING, State::default());
        // a node's text sits centred in its box, 2 points under `FOOTNOTE`'s line box: the box starts 2 higher (10 above the text's line)
        ui.set_node_layout(&head, &Node::column().w(Size::Grow).h_px(FOOTNOTE_LINE).margin(Edges::new(12.0, 8.0, 12.0, 6.0)));
    }
    ui.set_node_layout(&c.row, &Node::row().w(Size::Grow).h_px(44.0).gap(12.0).pad_x(12.0).align(Align::Center));
    ui.style(&c.row, &PALETTE_ROW, State::default().checked(c.current != 0));
    let art = c.art();
    if let Some(pxui::PickArt::Image(_)) = art {
        let art = container(ui, "artwork", Axis::Column, Size::Px(28.0), Size::Px(28.0), &PANEL_FILL);
        ui.set_role(&art, Role::Image, pxui::A11yStates::NONE);
        if let Some(image) = c.image() {
            // a descriptor that went bad between frames draws nothing (`Paint::image` refuses it)
            ui.canvas(&art, move |paint, rect| {
                let _ = paint.image(rect, &image, &COVER_ART);
            });
        }
    } else if let Some(pxui::PickArt::Draw(word)) = art
        && let Some(&kind) = ICONS.get(word as usize)
    {
        let icon = container(ui, "icon", Axis::Column, Size::Px(16.0), Size::Px(28.0), &Look::NONE);
        ui.canvas(&icon, move |paint, rect| {
            let rect = Rect {
                y: rect.y + (rect.h - 16.0) * 0.5,
                h: 16.0,
                ..rect
            };
            type Line = ((f32, f32), (f32, f32));
            let lines: &[Line] = match kind {
                Icon::Chevron => {
                    crate::icons::chevron(paint, rect, Rgba::token(INK));
                    return;
                }
                Icon::Queue | Icon::Rows => &[((2.0, 4.0), (14.0, 4.0)), ((2.0, 8.0), (14.0, 8.0)), ((2.0, 12.0), (11.0, 12.0))],
                Icon::Folder => &[
                    ((1.0, 4.0), (6.0, 4.0)),
                    ((6.0, 4.0), (8.0, 6.0)),
                    ((8.0, 6.0), (15.0, 6.0)),
                    ((15.0, 6.0), (15.0, 13.0)),
                    ((15.0, 13.0), (1.0, 13.0)),
                    ((1.0, 13.0), (1.0, 4.0)),
                ],
                Icon::Person => &[((3.0, 14.0), (4.0, 10.0)), ((4.0, 10.0), (12.0, 10.0)), ((12.0, 10.0), (13.0, 14.0))],
            };
            for &(a, b) in lines {
                let a = (rect.x + a.0, rect.y + a.1);
                let b = (rect.x + b.0, rect.y + b.1);
                paint.curve(a, ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5), b, Stroke::new(Rgba::token(INK)).width(1.1));
            }
            if kind == Icon::Person {
                paint.circle((rect.x + 8.0, rect.y + 4.0), 3.0, Some(Fill::new(Rgba::token(INK))), None);
            }
        });
    }
    ui.label(c.text_of(&c.label), BODY_INK.grow());
    let hint = c.text_of(&c.detail);
    if !hint.is_empty() {
        ui.label(hint, CAPTION);
    }
}

/// A palette row: the selected fill on the keyboard's row.
const PALETTE_ROW: pxui::theme::States = pxui::theme::States::of(Look::NONE).checked(SELECTED_FILL);
/// A palette group heading: secondary 12 points (`FOOTNOTE`).
const PALETTE_HEADING: pxui::theme::States = pxui::theme::States::of(text_look(FOOTNOTE).font(Font::BODY.size(12.0)));
/// The heading's line: `FOOTNOTE`'s.
const FOOTNOTE_LINE: f32 = 16.8;
/// The palette's query: ink, 20 points.
const PALETTE_QUERY: Look = Look::NONE.text(Rgba::token(INK)).font(PALETTE_FONT);

/// The palette's icons by their word (`PickArt::Draw(icon as u32)`), indexed by discriminant.
const ICONS: [Icon; 5] = [Icon::Chevron, Icon::Queue, Icon::Person, Icon::Folder, Icon::Rows];
// the table is indexed by discriminant: keep it in declaration order
const _: () = {
    let mut i = 0;
    while i < ICONS.len() {
        assert!(ICONS[i] as usize == i);
        i += 1;
    }
};

/// The palette's picker: the results are the field body's to size, no `/` before the query.
const PALETTE: pxui::Picker = pxui::Picker::DEFAULT.slash(false);
/// The palette's status where its results would be (searching, no matches, a query too long): 48 points over the empty results.
const PALETTE_STATUS: Label = TEXT_14.grow().h_px(48.0).color(SECONDARY);

/// The palette and the shortcuts card: a modal centred across, its top edge at Swift's 12% rule (`commands::top`).
const OVERLAY_TOP: Overlay = Overlay::DEFAULT.present(pxui::Presentation::Top);

/// Swift's 12% (capped at 96) is of the area below the status inset on the phone and below the titlebar on the desktop, whose
/// content Swift lays out under it.
fn top(content: Rect, inset: f32) -> f32 {
    inset + ((content.h - inset) * 0.12).min(96.0)
}

fn overlay_border(ui: &mut Ui, card: &Handle) {
    // Swift centers its one-point stroke half a point outside the card.
    ui.canvas_over(card, |paint, rect| {
        paint.stroke(Rect::new(rect.x - 0.5, rect.y - 0.5, rect.w + 1.0, rect.h + 1.0), Stroke::new(Rgba::token(LINE)));
    });
}

pub(super) fn palette_view(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    state: &ShellState,
    intent: &mut Option<Intent>,
) {
    let content = ui.content_rect();
    let (width, height) = (content.w, content.h);
    let results = state.command_results(catalog);
    let items = state.command_items(catalog);
    let top = top(content, state.top_inset);
    let palette = OVERLAY_TOP.at(pxui::Point::new(0.0, top)).w_px((width - 32.0).clamp(0.0, 600.0).min(width)).max_h_px((height - top - 16.0).max(0.0));
    let cover = |index: usize| items.get(index)?.artwork.filter(|row| *row < catalog.len()).map(|row| catalog.row(row));
    let image = |index: usize| {
        let row = cover(index)?;
        player.artwork?.get(row.cover_art_id, row.cover_art_path?, 28.0, None)
    };
    let modal = ui.modal("commands", &palette.name("Search and commands"), |ui| {
        let r = ui.picker("palette", "Search songs, folders, or type a command", state.command_query.as_str(), &PALETTE.image(&image), &mut |_, rows| {
            for (index, item) in items.iter().enumerate() {
                let heading = if index == 0 || items[index - 1].group != item.group { item.group } else { "" };
                let art = cover(index).map_or(PickArt::Draw(item.icon as u32), |row| PickArt::Image(row.cover_art_id));
                rows.push(pxui::PickRow::new(&item.label).detail(&item.hint).heading(heading).art(art));
            }
        });
        // a changed query hides the rows it was typed over: their pick never activates
        if r.query != state.command_query.as_str() {
            *intent = Some(Query::new(r.query).map_or(Intent::CommandTooLong, Intent::CommandQuery));
        } else if let Pick::Pick(index) = r.pick
            && let Some(item) = usize::try_from(index).ok().and_then(|index| items.get(index))
        {
            *intent = Some(Intent::ActivateCommand(item.key));
        } else if r.pick == Pick::Cancel {
            *intent = Some(Intent::CloseCommands);
        }
        if items.is_empty() {
            // over the empty results (60 high under the 59-point field and its hairline), where the first row would be
            let status = ui.node("status", Node::column().w(Size::Grow).h_px(48.0).at(0.0, 66.0).pad_x(6.0));
            ui.within(&status, &mut |ui| {
                ui.label(if state.command_too_long { "Search text is too long. Shorten it." } else if results.is_some() { "No matches." } else { "Searching…" }, PALETTE_STATUS);
            });
        }
    });
    if modal.open {
        const LOOK: pxui::theme::States = pxui::theme::States::of(
            PAPER_FILL.shadow(pxui::theme::Shadow::NONE.color(Rgba::BLACK.alpha(102)).offset(0.0, 30.0).blur(40.0).spread(-20.0)),
        );
        ui.style(&modal.handle, &LOOK, State::default());
        overlay_border(ui, &modal.handle);
    } else if intent.is_none() {
        *intent = Some(Intent::CloseCommands);
    }
}

pub(super) fn shortcuts(ui: &mut Ui, inset: f32, intent: &mut Option<Intent>) {
    const ROWS: [(&str, &str); 12] = [
        ("Play / pause", "Space"),
        ("Seek 5 s · 30 s", "← →  ·  ⌥← ⌥→"),
        ("Previous / next song", "⇧← ⇧→"),
        ("Search and commands", "⌘K or /"),
        ("Play selected song", "Return"),
        ("Move selection", "↑ ↓"),
        ("Shuffle · repeat", "S · R"),
        ("Small player / full panel", "I"),
        ("Favorite current song", "L"),
        ("Volume", "⌘↑ ⌘↓"),
        ("Back", "⌘["),
        ("Close panels", "Esc"),
    ];
    let pxui::ViewSize { width, height } = ui.view_size();
    let card = (width - 32.0).clamp(0.0, 520.0);
    // Swift's Grid gives label and key columns equal halves; labels wrap inside theirs (phone).
    let half = ((card - 48.0 - 16.0) * 0.5).max(1.0);
    const KEY_LABEL: Label = Label::BODY.line_height(19.6).wrap_spaces();
    let mut heights = [19.6_f32; ROWS.len()];
    let mut labels = ROWS.map(|(label, _)| label);
    for (index, (height, (label, _))) in heights.iter_mut().zip(ROWS).enumerate() {
        *height = ui
            .text_service()
            .measure(label, KEY_LABEL, Some(half))
            .map_or(19.6, |m| m.height.max(19.6));
        if label == "Small player / full panel" && *height > 19.6 {
            labels[index] = "Small player /\nfull panel";
            *height = ui.text_service().measure(labels[index], KEY_LABEL, Some(half))
                .map_or(*height, |m| m.height.max(19.6));
        }
    }
    let body_height: f32 = 44.0 + 25.2 + 12.0 + heights.iter().sum::<f32>() + 11.0 * 8.0;
    let content = ui.content_rect();
    let top = top(content, inset);
    let shortcuts = OVERLAY_TOP.at(pxui::Point::new(0.0, top)).w_px(card.min(content.w)).max_h_px((content.h - top - 16.0).max(0.0));
    let modal = ui.modal("shortcuts", &shortcuts.name("Keyboard shortcuts"), |ui| {
        ui.column(Node::column().w(Size::Grow).h_px(body_height.min((height - (height * 0.12).min(96.0) - 16.0).max(60.0))), |ui| {
            let scroll = ui.scroll_view("shortcuts", Node::DEFAULT.h(Size::Grow), |ui| {
                let body = ui.node("body", Node::column().w(Size::Grow).h(Size::Fit).gap(12.0).pad_x(24.0).pad_y(22.0));
                ui.within(&body, &mut |ui| {
                    ui.label("Keyboard", TEXT_18.grow().weight(500).color(INK));
                    ui.column(Node::column().w(Size::Grow).h(Size::Fit).gap(8.0), |ui| {
                        for (index, (label, key)) in ROWS.iter().enumerate() {
                            let row = ui.node(
                                Key::Id(index as i64),
                                Node::row().w(Size::Grow).h_px(heights[index]).gap(16.0)
                                    .align(Align::Center),
                            );
                            ui.within(&row, &mut |ui| {
                                let node = ui.node(
                                    "label",
                                    Node::column().w_px(half).h_px(heights[index]).role(Role::Label).look(INK_TEXT),
                                );
                                ui.text_line_box(&node, labels[index], KEY_LABEL, &[]).unwrap();
                                if labels[index] != *label { ui.a11y_name(&node, label); }
                                ui.node("fill", Node::DEFAULT.w(Size::Grow));
                                ui.label(key, MONO_DIM);
                            });
                        }
                    });
                });
            });
            // The inner scroller has no frame in Swift. Its default one-point inset shifts every line down.
            let layout = ui.node_layout(&scroll).padding(Edges::ZERO);
            ui.set_node_layout(&scroll, &layout);
            ui.style(&scroll, &BARE, State::default());
        });
    });
    if modal.open {
        overlay_border(ui, &modal.handle);
    } else if intent.is_none() {
        *intent = Some(Intent::CloseShortcuts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Icon, Item};
    use lw_core::context::PlaybackContext;
    use pxui::{Event, FrameIn, Platform, Str};
    use std::sync::Arc;

    fn results(query: Query, actions: &[Action]) -> Arc<Results> {
        Arc::new(Results {
            query,
            items: actions
                .iter()
                .enumerate()
                .map(|(i, action)| {
                    Item::new(
                        if i < 12 { "Commands" } else { "Lists" },
                        &format!("Result {i:02}"),
                        "",
                        Icon::Chevron,
                        None,
                        *action,
                    )
                })
                .collect(),
            songs: Arc::from([901, 702, 503, 304, 105, 106, 107, 108]),
            context: Arc::new(PlaybackContext::Selection(query.as_str().trim().into())),
        })
    }

    fn frame(
        ui: &mut Ui,
        catalog: &Catalog,
        state: &mut ShellState,
        time: &mut f64,
        events: &[Event],
    ) -> Option<Intent> {
        *time += 1.0 / 60.0;
        let mut intent = None;
        crate::testing::frame(
            ui,
            &{
                let mut frame = FrameIn::new(1000.0, 700.0).time(*time).reduced_motion(true);
                frame = frame.events(&events[..]);
                frame
            },
            &mut |ui| {
                shell(
                    ui,
                    &PlayerView::default(),
                    0.0,
                    catalog,
                    state,
                    &mut intent,
                )
            },
        )
        .unwrap();
        if let Some(intent) = intent {
            state.apply(intent);
        }
        intent
    }

    /// `localwave-palette`: the field 59 high with the results 1 below it, padded 6 and as tall as their rows; a group heading 12 in,
    /// its text line 10 under the padding; 44-point rows that are the list's items; the status over the empty results.
    #[test]
    fn localwave_palette_lays_out_its_field_results_headings_and_status() {
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let catalog = crate::tests::prepared(Vec::new());
        let mut state = ShellState::default();
        state.apply(Intent::ShowCommands);
        let mut time = 0.0;
        for _ in 0..4 {
            frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        }
        let at = |ui: &Ui, role: Role, name: &str| {
            ui.a11y_nodes().iter().find(|n| n.role == role as u32 && ui.a11y_name_of(n) == name).map(|n| n.rect).unwrap_or_else(|| panic!("{name}"))
        };
        let field = at(&ui, Role::TextField, "Search songs, folders, or type a command");
        let list = ui.a11y_nodes().iter().find(|n| n.role == Role::List as u32 && n.rect.y > field.y).map(|n| n.rect).unwrap();
        let (heading, first, last) = (at(&ui, Role::Heading, "Commands"), at(&ui, Role::ListItem, "Play / pause"), at(&ui, Role::ListItem, "Show keyboard shortcuts"));
        assert_eq!((field.h, list.y), (59.0, field.y + 60.0), "the field and its hairline");
        assert_eq!((heading.x, heading.y + 2.0), (list.x + 18.0, list.y + 6.0 + 10.0), "the heading's line, 12 in past the padding");
        assert_eq!((first.y, first.h), (list.y + 6.0 + 30.8, 44.0), "rows under their heading");
        assert!((list.y + list.h - (last.y + last.h) - 6.0).abs() < 0.01, "the results fit their rows, 6 in: {list:?} {last:?}");
        state.apply(Intent::CommandQuery(Query::new("zz").unwrap()));
        for _ in 0..3 {
            frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        }
        let list = ui.a11y_nodes().iter().find(|n| n.role == Role::List as u32 && n.rect.y > field.y).map(|n| n.rect).unwrap();
        let status = at(&ui, Role::Label, "Searching…");
        assert_eq!(list.h, 60.0, "the empty results keep their 60 points");
        assert!(status.y >= list.y + 6.0 && status.y + status.h <= list.y + list.h, "the status sits over the empty results: {status:?} {list:?}");
    }

    #[test]
    fn phone_shortcuts_wrap_labels_in_the_left_half() {
        let mut ui = crate::test_ui(
            pxui::Platform::Apple,
            Some(18),
            &[crate::FONT, crate::MONO],
        );
        ui.set_a11y(true);
        let catalog = Catalog::new(Vec::new());
        let state = ShellState {
            shortcuts_visible: true,
            ..ShellState::default()
        };
        for tick in 0..4 {
            let mut intent = None;
            crate::testing::frame(
                &mut ui,
                &FrameIn::new(390.0, 778.0).time(f64::from(tick)).reduced_motion(true),
                &mut |ui| shell(ui, &PlayerView::default(), 0.0, &catalog, &state, &mut intent),
            )
            .unwrap();
        }
        let bounds = |name: &str| {
            ui.a11y_nodes()
                .iter()
                .find(|node| ui.a11y_name_of(node) == name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .rect
        };
        assert_eq!(bounds("Play / pause").h, 19.6);
        assert_eq!(bounds("Search and commands").h, 39.2);
        assert_eq!(bounds("Keyboard").y - bounds("Keyboard shortcuts").y, 22.0);
    }

    #[test]
    fn command_palette_requests_preserve_find_and_validate_actions_against_current_results() {
        let mut catalog = crate::tests::prepared(Vec::new());
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Find));
        let background = Query::new("live flac").unwrap();
        state.apply(Intent::Search(background));
        state.apply(Intent::SortBy(Some((Sort::Title, true))));
        state.apply(Intent::ShowCommands);
        let query = Query::new(" café ").unwrap();
        state.apply(Intent::CommandQuery(query));
        let request = state.request(&catalog).unwrap();
        assert_eq!(request.commands, Some(query));
        assert_eq!(request.query, background);
        assert_eq!(request.scope, Scope::Find);
        assert_eq!(request.sort, Sort::Title);
        catalog.commands = Some(results(Query::default(), &[Action::Play(0)]));
        let key = catalog.commands.as_ref().unwrap().items[0].key;
        assert!(
            state
                .activate_command(&catalog, key, false, false)
                .is_none(),
            "old query must never activate"
        );
        assert!(state.commands_visible);
        catalog.commands = Some(results(query, &[Action::Play(3), Action::Play(100)]));
        let data = catalog.commands.as_ref().unwrap();
        assert!(
            state
                .activate_command(&catalog, data.items[1].key, false, false)
                .is_none()
        );
        assert_eq!(
            state.activate_command(&catalog, data.items[0].key, false, false),
            Some(Intent::PlayCommand(data.items[0].key))
        );
        assert_eq!(
            &*data.songs,
            &[901, 702, 503, 304, 105, 106, 107, 108],
            "play keeps the full ranked queue"
        );
        let location = crate::files::Location {
            user: 1,
            source: 2,
            parent: Some(3),
        };
        for (action, expected) in [
            (
                Action::Navigate(Route::Artists),
                Intent::Navigate(Route::Artists),
            ),
            (
                Action::Folder(location),
                Intent::Files(crate::files::Request::Browse(location)),
            ),
            (Action::Find, Intent::SearchResults(query)),
            (Action::Command(Command::PlayPause), Intent::Pause),
            (Action::Command(Command::Shuffle), Intent::Shuffle),
            (Action::Command(Command::Repeat), Intent::Repeat),
            (
                Action::Command(Command::AddMusic),
                Intent::Navigate(Route::Files),
            ),
            (
                Action::Command(Command::Rescan(location)),
                Intent::Files(crate::files::Request::Rescan(location)),
            ),
            (
                Action::Command(Command::Network),
                Intent::Setting(lw_core::preferences::Change::ToggleNoNetwork),
            ),
            (
                Action::Command(Command::Theme),
                Intent::Setting(lw_core::preferences::Change::Appearance(
                    lw_core::preferences::Appearance::Light,
                )),
            ),
            (Action::Command(Command::Panel), Intent::TogglePanel),
            (Action::Command(Command::Shortcuts), Intent::ShowShortcuts),
            (Action::Command(Command::Settings), Intent::ShowSettings),
        ] {
            state.apply(Intent::ShowCommands);
            state.apply(Intent::CommandQuery(query));
            catalog.commands = Some(results(query, &[action]));
            let key = catalog.commands.as_ref().unwrap().items[0].key;
            assert_eq!(
                state.activate_command(&catalog, key, true, true),
                Some(expected)
            );
            assert!(!state.commands_visible);
            assert!(
                state.activate_command(&catalog, key, true, true).is_none(),
                "dismissed results cannot reactivate"
            );
        }
        assert_eq!(
            state.query, background,
            "palette requests leave the page query alone"
        );
        state.apply(Intent::Navigate(Route::Albums));
        state.apply(Intent::ShowCommands);
        assert_eq!(state.request(&catalog).unwrap().scope, Scope::Overview);
        let previous = catalog.clone();
        catalog.commands = Some(results(Query::default(), &[Action::Find]));
        let revision = state.revision;
        state.refresh(&previous, &catalog);
        assert!(
            state.revision > revision,
            "command-only worker updates invalidate the shell even when its page Arc is unchanged"
        );
    }

    /// The palette row the keyboard is on (pxui marks it selected).
    fn cursor_row(ui: &Ui) -> String {
        let node = ui.a11y_nodes().iter().find(|n| {
            n.role == Role::ListItem as u32 && n.states & pxui::A11yStates::SELECTED.bits() != 0 && ui.a11y_name_of(n).starts_with("Result ")
        });
        node.map_or_else(String::new, |n| ui.a11y_name_of(n).to_owned())
    }

    #[test]
    fn command_palette_keyboard_selection_focus_utf8_dismissal_and_warm_frames() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        crate::commands::register(&mut ui).unwrap();
        let mut root = pxui::ThemeHandle::parse(crate::LIGHT).unwrap();
        crate::apply_palette(&mut root, false);
        ui.set_theme(&root).unwrap();
        let mut catalog = crate::tests::prepared(
            (0..10_000)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Track {id:05}"),
                    artist: "Example artist".into(),
                    album: "Example album".into(),
                    duration: Some(240.0),
                    ..Default::default()
                })
                .collect(),
        );
        let actions = [Action::Command(Command::Panel); 24];
        catalog.commands = Some(results(Query::default(), &actions));
        let mut state = ShellState::default();
        let mut time = 0.0;
        let key = |kind, k: input::Key| Event {
            kind,
            code: k.code(),
            ..Default::default()
        };
        let open = [
            key(Event::KEY_PRESSED, input::Key::Meta),
            Event::key(Event::KEY_PRESSED, 'k' as u32),
            key(Event::KEY_RELEASED, input::Key::Meta),
        ];
        frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        assert_eq!(
            frame(&mut ui, &catalog, &mut state, &mut time, &open),
            Some(Intent::ShowCommands)
        );
        for _ in 0..4 {
            assert_eq!(
                frame(&mut ui, &catalog, &mut state, &mut time, &[]),
                None
            );
        }
        assert!(ui.text_input_focused());
        assert_eq!(
            ui.text_traits(),
            pxui::TextTraits::SEARCH
        );
        let field = ui
            .a11y_nodes()
            .iter()
            .find(|n| {
                n.role == Role::TextField as u32 && ui.a11y_name_of(n) == "Search songs, folders, or type a command"
            })
            .unwrap();
        assert_eq!(field.rect.h, 59.0);
                let dialog = ui
            .a11y_nodes()
            .iter()
            .find(|n| n.role == Role::Dialog as u32 && ui.a11y_name_of(n) == "Search and commands")
            .unwrap();
        assert_eq!(dialog.rect.w, 600.0);
        assert!((dialog.rect.y - 84.0).abs() < 0.01);
        for _ in 0..3 {
            for _ in 0..24 {
                frame(
                    &mut ui,
                    &catalog,
                    &mut state,
                    &mut time,
                    &[key(Event::KEY_PRESSED, input::Key::ArrowDown)],
                );
            }
        }
        // the wrap's reveal scrolls the list back to the top: the first row is built the frame after
        frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        assert_eq!(cursor_row(&ui), "Result 00", "down wraps");
        frame(
            &mut ui,
            &catalog,
            &mut state,
            &mut time,
            &[key(Event::KEY_PRESSED, input::Key::ArrowUp)],
        );
        frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        assert_eq!(cursor_row(&ui), "Result 23", "up wraps");
        let selected = ui
            .a11y_nodes()
            .iter()
            .find(|n| {
                n.states & pxui::A11yStates::SELECTED.bits() != 0 && ui.a11y_name_of(n) == "Result 23"
            })
            .unwrap();
        assert!(
            selected.rect.y >= 144.0 && selected.rect.y + selected.rect.h <= 565.0,
            "keyboard selection must scroll into view: {:?}",
            selected.rect
        );
        for _ in 0..48 {
            mem_sys::local_reset();
            frame(
                &mut ui,
                &catalog,
                &mut state,
                &mut time,
                &[key(Event::KEY_PRESSED, input::Key::ArrowDown)],
            );
            assert_eq!(
                mem_sys::local_alloc_count(),
                0,
                "warm palette navigation includes the app and dylib"
            );
        }
        let unicode = "café";
        assert_eq!(
            frame(
                &mut ui,
                &catalog,
                &mut state,
                &mut time,
                &[Event {
                    kind: Event::TEXT_INPUT,
                    text: Str {
                        ptr: unicode.as_ptr(),
                        len: unicode.len()
                    },
                    ..Default::default()
                }]
            ),
            Some(Intent::CommandQuery(Query::new(unicode).unwrap()))
        );
        let suffix = " live";
        let edit_and_submit = [
            Event {
                kind: Event::TEXT_INPUT,
                text: Str {
                    ptr: suffix.as_ptr(),
                    len: suffix.len(),
                },
                ..Default::default()
            },
            key(Event::KEY_PRESSED, input::Key::Enter),
        ];
        assert_eq!(
            frame(
                &mut ui,
                &catalog,
                &mut state,
                &mut time,
                &edit_and_submit
            ),
            Some(Intent::CommandQuery(Query::new("café live").unwrap())),
            "editing and submitting in one batch cannot activate the previous query"
        );
        assert_eq!(
            frame(
                &mut ui,
                &catalog,
                &mut state,
                &mut time,
                &[key(Event::KEY_PRESSED, input::Key::Enter)]
            ),
            None,
            "query changes must hide stale actions"
        );
        catalog.commands = Some(results(state.command_query, &actions));
        frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        assert_eq!(cursor_row(&ui), "Result 00", "a new query's results start at the first row");
        let expected = catalog.commands.as_ref().unwrap().items[0].key;
        assert_eq!(
            frame(
                &mut ui,
                &catalog,
                &mut state,
                &mut time,
                &[key(Event::KEY_PRESSED, input::Key::Enter)]
            ),
            Some(Intent::ActivateCommand(expected))
        );
        state.apply(Intent::CloseCommands);
        frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        state.apply(Intent::ShowCommands);
        for _ in 0..3 {
            frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        }
        assert!(ui.text_input_focused(), "reopening restores input focus");
        assert_eq!(
            frame(
                &mut ui,
                &catalog,
                &mut state,
                &mut time,
                &[key(Event::KEY_PRESSED, input::Key::Escape)]
            ),
            Some(Intent::CloseCommands)
        );
        frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        state.apply(Intent::ShowCommands);
        frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        for kind in [Event::POINTER_PRESSED, Event::POINTER_RELEASED] {
            frame(
                &mut ui,
                &catalog,
                &mut state,
                &mut time,
                &[Event::pointer(kind, 0, 10.0, 200.0)],
            );
        }
        assert!(
            !state.commands_visible,
            "outside click dismisses without activating the sidebar"
        );
        assert_eq!(state.route, Route::Songs);
        state.apply(Intent::ShowCommands);
        frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        assert_eq!(
            frame(
                &mut ui,
                &catalog,
                &mut state,
                &mut time,
                &[Event::new(Event::FOCUS_LOST)]
            ),
            Some(Intent::CloseCommands)
        );
        frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        state.apply(Intent::ShowShortcuts);
        for _ in 0..6 {
            frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        }
        mem_sys::local_reset();
        frame(&mut ui, &catalog, &mut state, &mut time, &[]);
        assert_eq!(mem_sys::local_alloc_count(), 0);
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|n| ui.a11y_name_of(n) == "Keyboard shortcuts")
        );
        assert_eq!(
            frame(
                &mut ui,
                &catalog,
                &mut state,
                &mut time,
                &[key(Event::KEY_PRESSED, input::Key::Escape)]
            ),
            Some(Intent::CloseShortcuts)
        );
    }
}
