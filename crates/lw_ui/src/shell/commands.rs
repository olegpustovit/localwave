//! Preserve Command-K/slash opening, wrapping selection, selected-row visibility, UTF-8 input,
//! focus restoration and dismissal. Read scalar signals instead of owned key vectors. Raw key
//! queries do not consume events; focus/modal ownership must be arbitrated by the caller.
//!
use pxui::bodies::BodyConfig;
use super::*;
use crate::commands::{Action, Command, Results};

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

/// The field body's variant for Swift's phone Find field; the palette's field keeps variant 0.
pub(super) const FIND_FIELD: u16 = 1;

/// The command palette's input: no autocorrect or capitalization, the Search key.
const PALETTE_FIELD: pxui::TextField = pxui::TextField::SEARCH.name("Search and commands");

pub(crate) fn field_body(ui: &mut Ui, c: &pxui::bodies::TextFieldConfig) {
    // One registered body serves the palette (20 pt, 59 pt tall) and Swift's phone Find field (26 pt medium, 56 pt).
    let find = c.variant == FIND_FIELD;
    let (size, height, inset, hint) = if find {
        (26, 56.0, 0.0, "Songs, albums, folders")
    } else {
        (20, 59.0, 18.0, "Search songs, folders, or type a command")
    };
    let font = if find { FIND_FONT } else { PALETTE_FONT };
    let mut layout = ui.node_layout(&c.row);
    layout.h = Size::Px(height);
    layout.padding = Edges::ZERO.l(inset).r(inset);
    ui.set_node_layout(&c.row, &layout);
    ui.style(&c.row, &pxui::theme::States::of(Look::NONE), State::default());
    if ui.state(&c.row, 0) == 0 {
        ui.focus_box(&c.row);
        ui.set_state(&c.row, 1);
    }
    let editor = ui.part(
        pxui::Part::EDITOR,
        Node {
            inset: Edges::ZERO.l(if find { 0.0 } else { 2.0 }).r(if find { 0.0 } else { 2.0 }),
            look: Look::NONE.text(ui.tokens().foreground).font(font),
            ..Node::column().w(Size::Grow).h_px(height)
        },
    );
    let value = c.text_of(&c.value);
    if c.state.focused == 0 {
        ui.text(&editor, value);
    }
    if value.is_empty() {
        // Ellipsize to the field: the palette card is (view - 32) clamped to 600 and the phone Find row is view - 32 - 44.
        let view = ui.content_rect().w;
        let room = if find {
            view - 32.0 - 44.0 + 10.0
        } else {
            (view - 32.0).clamp(0.0, 600.0) - 40.0
        };
        let placeholder = ui.node(
            "prompt",
            Node {
                at: Some((if find { 0.0 } else { 20.0 }, 0.0)).into(),
                ..Node::row().w_px(room.max(1.0)).h_px(height).align(Align::Center)
            },
        );
        // the prompt's tracking in ems (the editor font's -0.52 points at 26)
        let style = Label::BODY
            .size(size as f32)
            .line_height(size as f32 * 1.4)
            .weight(font.weight)
            .tracking(if find { -0.02 } else { 0.0 })
            .lines(1)
            .ellipsis();
        ui.within(&placeholder, &mut |ui| {
            let line = ui.node(
                "line",
                Node {
                    look: if find { FIND_PROMPT } else { PALETTE_PROMPT },
                    ..Node::column().w(Size::Grow).h_px(size as f32 * 1.4)
                },
            );
            ui.text_line_box(&line, hint, style, &[]).unwrap();
        });
    }
}

fn overlay(
    ui: &mut Ui,
    inset: f32,
    key: &'static str,
    label: &str,
    dimensions: (f32, Size),
    close: Intent,
    intent: &mut Option<Intent>,
    build: &mut dyn FnMut(&mut Ui, &mut Option<Intent>),
) {
    let (width, height) = dimensions;
    let content = ui.content_rect();
    // Swift's 12% is of the window below the status inset (phone only; desktop windows keep content-rect math).
    let inset = if content.w < super::compact::BREAKPOINT {
        inset
    } else {
        0.0
    };
    let top = inset + ((content.h - inset) * 0.12).min(96.0);
    let width = width.min(content.w);
    let max_height = (content.h - top - 16.0).max(0.0);
    // The popup is just the card: a full-view popup would paint its own sheet fill over the page.
    // Press-outside dismissal is the popup's; shell() draws the dim over the live page.
    let open = ui.popup(key, &Overlay::DEFAULT.at(pxui::Point::new(content.x + (content.w - width) * 0.5, content.y + top)).w_px(width).h_px(60.0).max_h_px(max_height).name(label), |ui| {
            let card = ui.node(
                "card",
                Node {
                    max_h: max_height,
                    look: PAPER_FILL,
                    ..Node::column().w_px(width).h(height).role(Role::Dialog).flags(Flags::BLOCKING | Flags::CLIP)
                },
            );
            ui.set_role(&card, Role::Dialog, pxui::A11yStates::MODAL);
            ui.a11y_name(&card, label);
            ui.within(&card, &mut |ui| build(ui, intent));
        }).open;
    if !open && intent.is_none() {
        *intent = Some(close);
    }
    if intent.is_some_and(|i| i == close || matches!(i, Intent::ActivateCommand(_))) {
        ui.dismiss_popup();
    }
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
    let result_limit = 420.0_f32.min((height - 176.0).max(60.0));
    overlay(
        ui,
        state.top_inset,
        "commands",
        "Search and commands",
        ((width - 32.0).clamp(0.0, 600.0), Size::Fit),
        Intent::CloseCommands,
        intent,
        &mut |ui, intent| {
            let mut selected = state.command_selected.min(items.len().saturating_sub(1));
            if !items.is_empty() {
                if ui.key_pressed(pxui::KeyCode::ArrowDown, pxui::Mods::ANY) {
                    selected = (selected + 1) % items.len();
                }
                if ui.key_pressed(pxui::KeyCode::ArrowUp, pxui::Mods::ANY) {
                    selected = (selected + items.len() - 1) % items.len();
                }
                if selected != state.command_selected {
                    *intent = Some(Intent::CommandSelect(selected));
                }
            }
            let mut field = |ui: &mut Ui| {
                if let Some(value) = ui
                    .text_field("palette-input", state.command_query.as_str(), &PALETTE_FIELD)
                    .changed
                {
                    *intent = Some(
                        Query::new(value).map_or(Intent::CommandTooLong, Intent::CommandQuery),
                    );
                }
            };
            if let Some(theme) = player.command_theme {
                ui.themed(theme, &mut field);
            } else {
                field(ui);
            }
            rule(ui, "input-divider", false);
            if ui.key_pressed(pxui::KeyCode::Enter, pxui::Mods::ANY)
                && (intent.is_none() || matches!(intent, Some(Intent::CommandSelect(_))))
                && let Some(item) = items.get(selected)
            {
                *intent = Some(Intent::ActivateCommand(item.key));
            }
            let rows = SectionList::new(items.len().max(1)).row_h(44.0).measured();
            let list = ui.section_list("results", &rows, &|i| items.get(i).map_or(0, |item| item.key as i64), &mut |ui, part, _| {
                let SectionPart::Row(index) = part else { return };
                let Some(item) = items.get(index) else {
                    ui.label(if state.command_too_long { "Search text is too long. Shorten it." } else if results.is_some() { "No matches." } else { "Searching…" }, TEXT_14.grow().h_px(48.0).color(SECONDARY));
                    return;
                };
                let row = ui.node(
                    Key::Id(item.key as i64),
                    Node::column().w(Size::Grow).h(Size::Fit),
                );
                ui.within(&row, &mut |ui| {
                    if index == 0 || items[index - 1].group != item.group {
                        ui.id("group", |ui| { ui.column(Node::column().w(Size::Grow).h(Size::Fit).padding(Edges::new(12.0, 10.0, 12.0, 4.0)),
                            |ui| {
                                ui.label(item.group, FOOTNOTE.grow());
                            },
                        ); });
                    }
                    result_row(ui, player, catalog, item, index == selected, intent);
                });
            });
            let list = list.handle;
            ui.set_node_layout(
                &list,
                &Node {
                    min_h: 60.0,
                    max_h: result_limit,
                    ..Node::column().w(Size::Grow).h(Size::Fit).pad(6.0)
                },
            );
            let prior_selection = ui.state(&list, -1);
            if !items.is_empty() && prior_selection != selected as i64 {
                ui.reveal(&list, selected);
                ui.set_state(&list, selected as i64);
            } else if items.is_empty() {
                ui.set_state(&list, -1);
            }
        },
    );
}

fn result_row(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    item: &crate::commands::Item,
    selected: bool,
    intent: &mut Option<Intent>,
) {
    let row = ui.node(
        "item",
        Node {
            look: if selected { SELECTED_FILL } else { Look::NONE },
            ..Node::row()
                .w(Size::Grow)
                .h_px(44.0)
                .gap(12.0)
                .pad_x(12.0)
                .align(Align::Center)
                .role(Role::Button)
                .flags(Flags::CLICKABLE)
        },
    );
    ui.set_role(
        &row,
        Role::Button,
        pxui::A11yStates::ENABLED.with(if selected {
                pxui::A11yStates::SELECTED
            } else { pxui::A11yStates::NONE }),
    );
    ui.a11y_name(&row, &item.label);
    if ui.signal(&row).clicked
        && (intent.is_none() || matches!(intent, Some(Intent::CommandSelect(_))))
    {
        *intent = Some(Intent::ActivateCommand(item.key));
    }
    ui.within(&row, &mut |ui| {
        if let Some(index) = item.artwork.filter(|i| *i < catalog.len()) {
            let art = container(
                ui,
                "artwork",
                Axis::Column,
                Size::Px(28.0),
                Size::Px(28.0),
                &PANEL_FILL,
            );
            paint_artwork(ui, &art, Some(catalog.row(index)), 28.0, player.artwork);
        } else {
            let icon = container(
                ui,
                "icon",
                Axis::Column,
                Size::Px(16.0),
                Size::Px(28.0),
                &Look::NONE,
            );
            let kind = item.icon;
            ui.canvas(&icon, move |paint, rect| {
                let rect = Rect {
                    y: rect.y + (rect.h - 16.0) * 0.5,
                    h: 16.0,
                    ..rect
                };
                type Line = ((f32, f32), (f32, f32));
                let lines: &[Line] = match kind {
                    crate::commands::Icon::Chevron => {
                        &[((5.0, 3.0), (10.0, 8.0)), ((10.0, 8.0), (5.0, 13.0))]
                    }
                    crate::commands::Icon::Queue | crate::commands::Icon::Rows => &[
                        ((2.0, 4.0), (14.0, 4.0)),
                        ((2.0, 8.0), (14.0, 8.0)),
                        ((2.0, 12.0), (11.0, 12.0)),
                    ],
                    crate::commands::Icon::Folder => &[
                        ((1.0, 4.0), (6.0, 4.0)),
                        ((6.0, 4.0), (8.0, 6.0)),
                        ((8.0, 6.0), (15.0, 6.0)),
                        ((15.0, 6.0), (15.0, 13.0)),
                        ((15.0, 13.0), (1.0, 13.0)),
                        ((1.0, 13.0), (1.0, 4.0)),
                    ],
                    crate::commands::Icon::Person => &[
                        ((3.0, 14.0), (4.0, 10.0)),
                        ((4.0, 10.0), (12.0, 10.0)),
                        ((12.0, 10.0), (13.0, 14.0)),
                    ],
                };
                for &(a, b) in lines {
                    let a = (rect.x + a.0, rect.y + a.1);
                    let b = (rect.x + b.0, rect.y + b.1);
                    paint.curve(a, ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5), b, Stroke::new(Rgba::token(INK)).width(1.1));
                }
                if kind == crate::commands::Icon::Person {
                    paint.circle((rect.x + 8.0, rect.y + 4.0), 3.0, Some(Fill::new(Rgba::token(INK))), None);
                }
            });
        }
        ui.label(&item.label, BODY_INK.grow());
        if !item.hint.is_empty() {
            ui.label(&item.hint, CAPTION);
        }
    });
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
    for (height, (label, _)) in heights.iter_mut().zip(ROWS) {
        *height = ui
            .text_service()
            .measure(label, KEY_LABEL, Some(half))
            .map_or(19.6, |m| m.height.max(19.6));
    }
    let body_height: f32 = 44.0 + 25.2 + 12.0 + heights.iter().sum::<f32>() + 11.0 * 8.0;
    overlay(
        ui,
        inset,
        "shortcuts",
        "Keyboard shortcuts",
        (
            card,
            Size::Px(body_height.min((height - (height * 0.12).min(96.0) - 16.0).max(60.0))),
        ),
        Intent::CloseShortcuts,
        intent,
        &mut |ui, _| {
            ui.scroll_view("shortcuts", Node::DEFAULT.h(Size::Grow), |ui| {
                let body = ui.node("body", Node::column().w(Size::Grow).h(Size::Fit).gap(12.0).pad_x(24.0).pad_y(22.0));
                ui.within(&body, &mut |ui| {
                    ui.label("Keyboard", TEXT_18.grow().color(INK));
                    ui.column(Node::column().w(Size::Grow).h(Size::Fit).gap(8.0), |ui| {
                        for (index, (label, key)) in ROWS.iter().enumerate() {
                            let row = ui.node(
                                Key::Id(index as i64),
                                Node::row().w(Size::Grow).h_px(heights[index]).gap(16.0),
                            );
                            ui.within(&row, &mut |ui| {
                                let node = ui.node(
                                    "label",
                                    Node::column().w_px(half).h_px(heights[index]).role(Role::Label).look(INK_TEXT),
                                );
                                ui.text_line_box(&node, label, KEY_LABEL, &[]).unwrap();
                                ui.node("fill", Node::DEFAULT.w(Size::Grow));
                                ui.label(key, MONO_DIM);
                            });
                        }
                    });
                });
            });
        },
    );
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
        theme: &pxui::ThemeHandle,
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
                frame.wire.events = events.as_ptr();
                frame.wire.event_count = events.len();
                frame
            },
            &mut |ui| {
                shell(
                    ui,
                    &PlayerView {
                        command_theme: Some(theme),
                        ..Default::default()
                    },
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
        let height = |name: &str| {
            ui.a11y_nodes()
                .iter()
                .find(|node| ui.a11y_name_of(node) == name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .rect
                .h
        };
        assert_eq!(height("Play / pause"), 19.6);
        assert_eq!(height("Search and commands"), 39.2);
    }

    #[test]
    fn command_palette_requests_preserve_find_and_validate_actions_against_current_results() {
        let mut catalog = crate::tests::prepared(Vec::new());
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Find));
        let background = Query::new("live flac").unwrap();
        state.apply(Intent::Search(background));
        state.apply(Intent::Sort(Sort::Title));
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

    #[test]
    fn command_palette_keyboard_selection_focus_utf8_dismissal_and_warm_frames() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        crate::commands::register(&mut ui).unwrap();
        let mut root = pxui::ThemeHandle::parse(crate::LIGHT).unwrap();
        crate::apply_palette(&mut root, false);
        ui.set_theme(&root).unwrap();
        let source = format!("{}\ntext_field.body = localwave-command", crate::LIGHT);
        let mut theme = pxui::ThemeHandle::parse(&source).unwrap();
        crate::apply_palette(&mut theme, false);
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
            Event {
                kind: Event::KEY_PRESSED,
                code: 'k' as u32,
                ..Default::default()
            },
            key(Event::KEY_RELEASED, input::Key::Meta),
        ];
        frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]);
        assert_eq!(
            frame(&mut ui, &theme, &catalog, &mut state, &mut time, &open),
            Some(Intent::ShowCommands)
        );
        for _ in 0..4 {
            assert_eq!(
                frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]),
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
                n.role == Role::TextField as u32 && ui.a11y_name_of(n) == "Search and commands"
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
                    &theme,
                    &catalog,
                    &mut state,
                    &mut time,
                    &[key(Event::KEY_PRESSED, input::Key::ArrowDown)],
                );
            }
        }
        assert_eq!(state.command_selected, 0, "down wraps");
        frame(
            &mut ui,
            &theme,
            &catalog,
            &mut state,
            &mut time,
            &[key(Event::KEY_PRESSED, input::Key::ArrowUp)],
        );
        assert_eq!(state.command_selected, 23, "up wraps");
        frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]);
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
                &theme,
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
                &theme,
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
        assert_eq!(state.command_selected, 0);
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
                &theme,
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
                &theme,
                &catalog,
                &mut state,
                &mut time,
                &[key(Event::KEY_PRESSED, input::Key::Enter)]
            ),
            None,
            "query changes must hide stale actions"
        );
        catalog.commands = Some(results(state.command_query, &actions));
        let expected = catalog.commands.as_ref().unwrap().items[0].key;
        assert_eq!(
            frame(
                &mut ui,
                &theme,
                &catalog,
                &mut state,
                &mut time,
                &[key(Event::KEY_PRESSED, input::Key::Enter)]
            ),
            Some(Intent::ActivateCommand(expected))
        );
        state.apply(Intent::CloseCommands);
        frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]);
        state.apply(Intent::ShowCommands);
        for _ in 0..3 {
            frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]);
        }
        assert!(ui.text_input_focused(), "reopening restores input focus");
        assert_eq!(
            frame(
                &mut ui,
                &theme,
                &catalog,
                &mut state,
                &mut time,
                &[key(Event::KEY_PRESSED, input::Key::Escape)]
            ),
            Some(Intent::CloseCommands)
        );
        frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]);
        state.apply(Intent::ShowCommands);
        frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]);
        for kind in [Event::POINTER_PRESSED, Event::POINTER_RELEASED] {
            frame(
                &mut ui,
                &theme,
                &catalog,
                &mut state,
                &mut time,
                &[Event {
                    kind,
                    x: 10.0,
                    y: 200.0,
                    ..Default::default()
                }],
            );
        }
        assert!(
            !state.commands_visible,
            "outside click dismisses without activating the sidebar"
        );
        assert_eq!(state.route, Route::Songs);
        state.apply(Intent::ShowCommands);
        frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]);
        assert_eq!(
            frame(
                &mut ui,
                &theme,
                &catalog,
                &mut state,
                &mut time,
                &[Event {
                    kind: Event::FOCUS_LOST,
                    ..Default::default()
                }]
            ),
            Some(Intent::CloseCommands)
        );
        frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]);
        state.apply(Intent::ShowShortcuts);
        for _ in 0..6 {
            frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]);
        }
        mem_sys::local_reset();
        frame(&mut ui, &theme, &catalog, &mut state, &mut time, &[]);
        assert_eq!(mem_sys::local_alloc_count(), 0);
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|n| ui.a11y_name_of(n) == "Keyboard shortcuts")
        );
        assert_eq!(
            frame(
                &mut ui,
                &theme,
                &catalog,
                &mut state,
                &mut time,
                &[key(Event::KEY_PRESSED, input::Key::Escape)]
            ),
            Some(Intent::CloseShortcuts)
        );
    }
}
