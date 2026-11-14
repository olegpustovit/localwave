use super::*;
use crate::commands::{Action, Command, Results};

impl ShellState {
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
        let results = self.command_results(catalog)?;
        let action = results.items.iter().find(|item| item.key == key)?.action;
        let query = results.query;
        let intent = match action {
            Action::Play(index) => {
                results.songs.get(index)?;
                return Some(Intent::PlayCommand(key));
            }
            Action::Navigate(route) => Intent::Navigate(route),
            Action::Folder(location) => {
                self.apply(Intent::Navigate(Route::Files));
                Intent::Files(crate::files::Request::Browse(location))
            }
            Action::Find => {
                self.apply(Intent::Navigate(Route::Find));
                Intent::Search(query)
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
            },
        };
        self.apply(Intent::CloseCommands);
        Some(intent)
    }
}

pub(crate) fn field_body(ui: &mut Ui, c: &pxui::bodies::PxuiTextFieldConfig) {
    let mut layout = ui.node_layout(&c.row);
    layout.h = Size::Px(59.0);
    layout.padding = Edges {
        l: 18.0,
        r: 18.0,
        ..Edges::ZERO
    };
    ui.set_node_layout(&c.row, &layout);
    ui.set_look(&c.row, Look::NONE, State::default());
    if ui.state(&c.row, 0) == 0 {
        ui.focus_box(&c.row);
        ui.set_state(&c.row, 1);
    }
    let editor = ui.part(
        pxui::Part::EDITOR,
        Node {
            w: Size::Grow,
            h: Size::Px(59.0),
            inset: Edges {
                l: 2.0,
                r: 2.0,
                ..Edges::ZERO
            },
            look: Look {
                text: ui.tokens().foreground,
                font: Font {
                    size: 20,
                    ..Font::BODY
                },
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    let value = pxui::bodies::text_of(&c.value);
    if c.state.focused == 0 {
        ui.text(&editor, value);
    }
    if value.is_empty() {
        let placeholder = ui.node(
            "prompt",
            Node {
                at: Some((20.0, 0.0)).into(),
                w: Size::Grow,
                h: Size::Px(59.0),
                look: Look {
                    text: palette(ui).secondary,
                    font: Font {
                        size: 20,
                        ..Font::BODY
                    },
                    ..Look::NONE
                },
                ..Node::DEFAULT
            },
        );
        ui.text(&placeholder, "Search songs, folders, or type a command");
    }
}

fn row_top(items: &[crate::commands::Item], index: usize) -> f32 {
    let headers = items[..=index]
        .iter()
        .enumerate()
        .filter(|(i, item)| *i == 0 || items[*i - 1].group != item.group)
        .count();
    6.0 + index as f32 * 44.0 + headers as f32 * 30.8
}

fn overlay(
    ui: &mut Ui,
    label: &str,
    dimensions: (f32, f32),
    p: Palette,
    close: Intent,
    intent: &mut Option<Intent>,
    build: &mut dyn FnMut(&mut Ui, &mut Option<Intent>),
) {
    let (width, height) = dimensions;
    let (vw, vh) = ui.view_size();
    let content = ui.content_rect();
    let top = (content.h * 0.12).min(96.0);
    let width = width.min(content.w);
    let height = height.min((content.h - top - 16.0).max(0.0));
    let open = ui.popup(
        None,
        None,
        false,
        false,
        (0.0, 0.0),
        vw,
        vh,
        label,
        &mut |ui| {
            let extent = container(
                ui,
                "extent",
                Axis::Stack,
                Size::Px(vw),
                Size::Px(vh),
                Rgba::default(),
            );
            ui.within(&extent, &mut |ui| {
                let scrim = ui.node(
                    "scrim",
                    Node {
                        flags: CLICKABLE,
                        w: Size::Grow,
                        h: Size::Grow,
                        look: Look {
                            fill: Rgba {
                                r: 10,
                                g: 10,
                                b: 10,
                                a: 71,
                            },
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
                if ui.signal_state(&scrim).clicked {
                    *intent = Some(close);
                }
                let card = ui.node(
                    "card",
                    Node {
                        flags: BLOCKING | CLIP,
                        axis: Axis::Column,
                        role: Role::Dialog,
                        at: Some((content.x + (content.w - width) * 0.5, content.y + top)).into(),
                        w: Size::Px(width),
                        h: Size::Px(height),
                        look: Look {
                            fill: p.paper,
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.set_role(&card, Role::Dialog, pxui::a11y::state::MODAL);
                ui.a11y_name(&card, label);
                ui.within(&card, &mut |ui| build(ui, intent));
            });
        },
    );
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
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let content = ui.content_rect();
    let (width, height) = (content.w, content.h);
    let results = state.command_results(catalog);
    let items = results.map_or(&[][..], |results| results.items.as_slice());
    let groups = items
        .iter()
        .enumerate()
        .filter(|(i, item)| !items[..*i].iter().any(|old| old.group == item.group))
        .count();
    let result_height = (items.len() as f32 * 44.0 + groups as f32 * 30.8 + 12.0)
        .clamp(60.0, 420.0)
        .min((height - 176.0).max(60.0));
    overlay(
        ui,
        "Search and commands",
        ((width - 32.0).clamp(0.0, 600.0), 60.0 + result_height),
        p,
        Intent::CloseCommands,
        intent,
        &mut |ui, intent| {
            let mut selected = state.command_selected.min(items.len().saturating_sub(1));
            if !items.is_empty() {
                if ui.key_pressed(input::Key::ArrowDown) {
                    selected = (selected + 1) % items.len();
                }
                if ui.key_pressed(input::Key::ArrowUp) {
                    selected = (selected + items.len() - 1) % items.len();
                }
                if selected != state.command_selected {
                    *intent = Some(Intent::CommandSelect(selected));
                }
            }
            let mut field = |ui: &mut Ui| {
                if let Some(value) =
                    ui.text_field("Search and commands", state.command_query.as_str())
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
            rule(ui, "input-divider", false, p);
            if ui.key_pressed(input::Key::Enter)
                && (intent.is_none() || matches!(intent, Some(Intent::CommandSelect(_))))
                && let Some(item) = items.get(selected)
            {
                *intent = Some(Intent::ActivateCommand(item.key));
            }
            let list = ui.node(
                "results",
                Node {
                    flags: SCROLL | CLIP,
                    axis: Axis::Column,
                    w: Size::Grow,
                    h: Size::Px(result_height),
                    scroll_step: 44.0,
                    ..Node::DEFAULT
                },
            );
            let prior_selection = ui.state(&list, -1);
            if !items.is_empty() && prior_selection != selected as i64 {
                let top = row_top(items, selected);
                let offset = ui.scroll_offset(&list);
                ui.set_scroll_offset(
                    &list,
                    if top < offset {
                        top
                    } else {
                        offset.max(top + 44.0 - result_height)
                    },
                );
                ui.set_state(&list, selected as i64);
            }
            ui.within(&list, &mut |ui| {
                let content = ui.node(
                    "content",
                    Node {
                        axis: Axis::Column,
                        w: Size::Grow,
                        h: Size::Fit,
                        padding: Edges::all(6.0),
                        ..Node::DEFAULT
                    },
                );
                ui.within(&content, &mut |ui| {
                    for (index, item) in items.iter().enumerate() {
                        let row = ui.node_keyed(
                            int_key(content.box_key, item.key as i64),
                            Node {
                                axis: Axis::Column,
                                w: Size::Grow,
                                h: Size::Fit,
                                ..Node::DEFAULT
                            },
                        );
                        ui.within(&row, &mut |ui| {
                            if index == 0 || items[index - 1].group != item.group {
                                let header = ui.node(
                                    "group",
                                    Node {
                                        axis: Axis::Column,
                                        w: Size::Grow,
                                        h: Size::Px(30.8),
                                        padding: Edges {
                                            l: 12.0,
                                            r: 12.0,
                                            t: 10.0,
                                            b: 4.0,
                                        },
                                        ..Node::DEFAULT
                                    },
                                );
                                ui.within(&header, &mut |ui| {
                                    text(
                                        ui,
                                        "name",
                                        item.group,
                                        12,
                                        p.secondary,
                                        (Size::Grow, 16.8),
                                        false,
                                    );
                                });
                            }
                            result_row(ui, player, catalog, item, index == selected, p, intent);
                        });
                    }
                    if items.is_empty() {
                        text(
                            ui,
                            "empty",
                            if state.command_too_long {
                                "Search text is too long. Shorten it."
                            } else if results.is_some() {
                                "No matches."
                            } else {
                                "Searching…"
                            },
                            14,
                            p.secondary,
                            (Size::Grow, 48.0),
                            false,
                        );
                    }
                });
            });
        },
    );
}

fn result_row(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    item: &crate::commands::Item,
    selected: bool,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let row = ui.node(
        "item",
        Node {
            flags: CLICKABLE,
            axis: Axis::Row,
            w: Size::Grow,
            h: Size::Px(44.0),
            role: Role::Button,
            padding: Edges {
                l: 12.0,
                r: 12.0,
                ..Edges::ZERO
            },
            gap: 12.0,
            align: Align::Center,
            look: Look {
                fill: if selected {
                    p.selected
                } else {
                    Rgba::default()
                },
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.set_role(
        &row,
        Role::Button,
        pxui::a11y::state::ENABLED
            | if selected {
                pxui::a11y::state::SELECTED
            } else {
                0
            },
    );
    ui.a11y_name(&row, &item.label);
    if ui.signal_state(&row).clicked
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
                p.panel,
            );
            paint_artwork(ui, &art, Some(catalog.row(index)), 28.0, player.artwork);
        } else {
            let icon = container(
                ui,
                "icon",
                Axis::Column,
                Size::Px(16.0),
                Size::Px(16.0),
                Rgba::default(),
            );
            let kind = item.icon;
            ui.draw(&icon, move |paint, rect| {
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
                    paint.curve(a, ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5), b, 1.1, p.ink);
                }
                if kind == crate::commands::Icon::Person {
                    paint.circle((rect.x + 8.0, rect.y + 4.0), 3.0, Some(p.ink), None);
                }
            });
        }
        text(
            ui,
            "label",
            &item.label,
            14,
            p.ink,
            (Size::Grow, 19.6),
            false,
        );
        if !item.hint.is_empty() {
            text(
                ui,
                "hint",
                &item.hint,
                13,
                p.secondary,
                (Size::Fit, 18.2),
                false,
            );
        }
    });
}

pub(super) fn shortcuts(ui: &mut Ui, p: Palette, intent: &mut Option<Intent>) {
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
    let (width, height) = ui.view_size();
    let body_height: f32 = 44.0 + 25.2 + 12.0 + 12.0 * 19.6 + 11.0 * 8.0;
    overlay(
        ui,
        "Keyboard shortcuts",
        (
            (width - 32.0).clamp(0.0, 520.0),
            body_height.min((height - (height * 0.12).min(96.0) - 16.0).max(60.0)),
        ),
        p,
        Intent::CloseShortcuts,
        intent,
        &mut |ui, _| {
            ui.scroll_view("shortcuts", Size::Grow, Size::Grow, &mut |ui| {
                let body = ui.node(
                    "body",
                    Node {
                        axis: Axis::Column,
                        w: Size::Grow,
                        h: Size::Fit,
                        gap: 8.0,
                        padding: Edges {
                            l: 24.0,
                            r: 24.0,
                            t: 22.0,
                            b: 22.0,
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&body, &mut |ui| {
                    text(
                        ui,
                        "heading",
                        "Keyboard",
                        18,
                        p.ink,
                        (Size::Grow, 25.2),
                        false,
                    );
                    spacer(ui, "title-gap", 4.0);
                    for (index, (label, key)) in ROWS.iter().enumerate() {
                        let row = ui.node_keyed(
                            int_key(body.box_key, index as i64),
                            Node {
                                axis: Axis::Row,
                                w: Size::Grow,
                                h: Size::Px(19.6),
                                gap: 16.0,
                                ..Node::DEFAULT
                            },
                        );
                        ui.within(&row, &mut |ui| {
                            text(ui, "label", label, 14, p.ink, (Size::Grow, 19.6), false);
                            text(ui, "key", key, 12, p.secondary, (Size::Fit, 16.8), true);
                        });
                    }
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
        theme: &pxui::Theme,
        catalog: &Catalog,
        state: &mut ShellState,
        time: &mut f64,
        events: &[Event],
    ) -> Option<Intent> {
        *time += 1.0 / 60.0;
        let mut intent = None;
        ui.frame(
            &FrameIn {
                width: 1000.0,
                height: 700.0,
                time: *time,
                reduced_motion: 1,
                events: events.as_ptr(),
                event_count: events.len(),
                ..Default::default()
            },
            &mut |ui| {
                shell(
                    ui,
                    &PlayerView {
                        command_theme: Some(theme),
                        ..Default::default()
                    },
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
            (Action::Find, Intent::Search(query)),
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
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        crate::commands::register(&mut ui).unwrap();
        let mut theme = pxui::Theme::default();
        theme.apply(&ui, crate::LIGHT).unwrap();
        ui.use_theme(&theme);
        theme
            .apply(&ui, "text_field.body = localwave-command")
            .unwrap();
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
                n.states & pxui::a11y::state::SELECTED != 0 && ui.a11y_name_of(n) == "Result 23"
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
