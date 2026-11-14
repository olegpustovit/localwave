use super::*;

pub(super) const BREAKPOINT: f32 = 620.0;

fn button(ui: &mut Ui, key: &str, label: &str, selected: bool, p: Palette) -> bool {
    let node = ui.node(
        key,
        Node {
            flags: CLICKABLE | TAB_STOP,
            role: Role::Button,
            w: Size::Grow,
            h: Size::Px(44.0),
            align: Align::Center,
            justify: Justify::Center,
            look: Look {
                fill: if selected { p.selected } else { p.paper },
                text: p.ink,
                font: Font {
                    size: 14,
                    ..Font::BODY
                },
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&node, &mut |ui| {
        text(ui, "label", label, 14, p.ink, (Size::Fit, 19.6), false);
    });
    ui.a11y_name(&node, label);
    ui.set_role(
        &node,
        Role::Button,
        if selected {
            pxui::a11y::state::SELECTED
        } else {
            0
        },
    );
    ui.signal_state(&node).clicked
}

fn line(ui: &mut Ui, key: &str, value: &str, size: f32, color: Rgba) {
    let node = ui.node(
        key,
        Node {
            role: Role::Label,
            w: Size::Grow,
            h: Size::Px(size * 1.4),
            look: Look {
                text: color,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.text_line_box(
        &node,
        value,
        &pxui::TextStyle {
            size_points: size,
            line_height: size * 1.4,
            max_lines: 1,
            ellipsis: pxui::TextStyle::ELLIPSIS_END,
            ..Default::default()
        },
        &[],
    )
    .unwrap();
}

fn artwork(
    ui: &mut Ui,
    catalog: &Catalog,
    player: &PlayerView<'_>,
    row: Option<usize>,
    size: f32,
    p: Palette,
) {
    let art = container(
        ui,
        "artwork",
        Axis::Stack,
        Size::Px(size),
        Size::Px(size),
        p.panel,
    );
    paint_artwork(
        ui,
        &art,
        row.map(|index| catalog.row(index)),
        size,
        player.artwork,
    );
}

pub(super) fn view(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) -> Handle {
    let (width, height) = ui.view_size();
    let [top, right, bottom, left] = ui.environment().safe_area;
    let width = (width - left - right).max(1.0);
    let root = ui.node(
        "compact-shell",
        Node {
            role: Role::Window,
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges {
                t: top.max(state.top_inset),
                r: right,
                b: bottom,
                l: left,
            },
            look: Look {
                fill: p.paper,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.a11y_name(&root, "LocalWave");
    ui.within(&root, &mut |ui| {
        if state.player_expanded {
            expanded(
                ui,
                player,
                catalog,
                (width, height - top.max(state.top_inset) - bottom),
                state.entry_revision,
                p,
                intent,
            );
            return;
        }
        if !state.settings_visible && state.route != Route::Files && state.route != Route::Find {
            let head = ui.node(
                "compact-heading",
                Node {
                    axis: Axis::Row,
                    align: Align::Center,
                    w: Size::Grow,
                    h: Size::Px(64.0),
                    padding: Edges {
                        l: 16.0,
                        r: 16.0,
                        ..Edges::ZERO
                    },
                    ..Node::DEFAULT
                },
            );
            ui.within(&head, &mut |ui| {
                let parent = match state.route {
                    Route::Album(_) => Some(Route::Albums),
                    Route::Artist(_) => Some(Route::Artists),
                    Route::Collection(_) => Some(Route::Lists),
                    _ => None,
                };
                if let Some(parent) = parent {
                    let back = ui.node(
                        "back-holder",
                        Node {
                            w: Size::Px(64.0),
                            h: Size::Px(44.0),
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&back, &mut |ui| {
                        if button(ui, "back", "Back", false, p) {
                            *intent = Some(Intent::Navigate(parent));
                        }
                    });
                }
                let title = match state.route {
                    Route::Lists => "Lists",
                    Route::Album(_) | Route::Artist(_) | Route::Collection(_) => state
                        .collection(catalog)
                        .map_or("Library", |group| group.title.as_str()),
                    _ => "Library",
                };
                line(ui, "heading", title, 28.0, p.ink);
                let settings = ui.node(
                    "settings-holder",
                    Node {
                        w: Size::Px(72.0),
                        h: Size::Px(44.0),
                        ..Node::DEFAULT
                    },
                );
                ui.within(&settings, &mut |ui| {
                    if button(ui, "settings", "Settings", false, p) {
                        *intent = Some(Intent::ShowSettings);
                    }
                });
            });
            if matches!(state.route, Route::Albums | Route::Artists | Route::Songs) {
                let hero = ui.node(
                    "library-summary",
                    Node {
                        w: Size::Grow,
                        h: Size::Px(72.0),
                        padding: Edges {
                            l: 16.0,
                            r: 16.0,
                            ..Edges::ZERO
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&hero, &mut |ui| {
                    let count = container(
                        ui,
                        "song-count",
                        Axis::Row,
                        Size::Grow,
                        Size::Px(42.0),
                        p.paper,
                    );
                    ui.within(&count, &mut |ui| {
                        text(
                            ui,
                            "number",
                            &catalog.song_count,
                            30,
                            p.ink,
                            (Size::Fit, 42.0),
                            false,
                        );
                        text(
                            ui,
                            "unit",
                            " songs",
                            15,
                            p.secondary,
                            (Size::Fit, 42.0),
                            false,
                        );
                    });
                    line(
                        ui,
                        "library-size",
                        &catalog.library_subtitle,
                        12.0,
                        p.secondary,
                    );
                });
                let tabs = container(
                    ui,
                    "library-tabs",
                    Axis::Row,
                    Size::Grow,
                    Size::Px(44.0),
                    p.paper,
                );
                ui.set_role(&tabs, Role::Group, 0);
                ui.a11y_name(&tabs, "Library view");
                ui.within(&tabs, &mut |ui| {
                    for (label, route) in [
                        ("Albums", Route::Albums),
                        ("Artists", Route::Artists),
                        ("Songs", Route::Songs),
                    ] {
                        if button(ui, label, label, state.route == route, p) {
                            *intent = Some(Intent::Navigate(route));
                        }
                    }
                });
                rule(ui, "tabs-divider", false, p);
            }
        }
        let body = container(
            ui,
            "compact-content",
            Axis::Column,
            Size::Grow,
            Size::Grow,
            p.paper,
        );
        let mut route_key = DefaultHasher::new();
        (
            state.route,
            state.settings_visible,
            if state.route == Route::Find {
                state.query_revision
            } else {
                0
            },
        )
            .hash(&mut route_key);
        ui.within(&body, &mut |ui| {
            let page = ui.node_keyed(
                int_key(body.box_key, route_key.finish() as i64),
                Node {
                    w: Size::Grow,
                    h: Size::Grow,
                    ..Node::DEFAULT
                },
            );
            ui.within(&page, &mut |ui| {
                if state.settings_visible {
                    settings::view(ui, player, state, width, p, intent);
                } else {
                    match state.route {
                        Route::Files => files::view(ui, player, width, p, intent),
                        Route::Albums => {
                            albums::gallery(ui, catalog, state, player, width, p, intent)
                        }
                        Route::Artists => groups(ui, catalog, player, true, width, p, intent),
                        Route::Lists => collections(ui, catalog, player, width, p, intent),
                        _ => songs(ui, catalog, state, player, width, p, intent),
                    }
                }
            });
        });
        crate::motion::enter(
            ui,
            &body,
            state.entry_revision,
            (24.0, 0.0),
            crate::motion::BASE,
            0.0,
        );
        rule(ui, "mini-divider", false, p);
        mini(ui, player, catalog, p, intent);
        rule(ui, "nav-divider", false, p);
        let nav = container(
            ui,
            "compact-navigation",
            Axis::Row,
            Size::Grow,
            Size::Px(54.0),
            p.paper,
        );
        ui.set_role(&nav, Role::Group, 0);
        ui.a11y_name(&nav, "Main navigation");
        ui.within(&nav, &mut |ui| {
            let selected = match state.route {
                Route::Files => 1,
                Route::Find => 2,
                Route::Lists | Route::Collection(_) => 3,
                _ => 0,
            };
            for (index, (title, route)) in [
                ("Library", Route::Albums),
                ("Files", Route::Files),
                ("Find", Route::Find),
                ("Lists", Route::Lists),
            ]
            .into_iter()
            .enumerate()
            {
                if button(
                    ui,
                    title,
                    title,
                    !state.settings_visible && selected == index,
                    p,
                ) {
                    *intent = Some(Intent::Navigate(route));
                }
            }
        });
    });
    root
}

fn mini(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let root = ui.node(
        "mini-player",
        Node {
            axis: Axis::Row,
            align: Align::Center,
            w: Size::Grow,
            h: Size::Px(64.0),
            padding: Edges::all(8.0),
            gap: 8.0,
            look: Look {
                fill: p.panel,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let open = ui.node(
            "open-player",
            Node {
                axis: Axis::Row,
                align: Align::Center,
                gap: 10.0,
                flags: CLICKABLE | TAB_STOP,
                role: Role::Button,
                w: Size::Grow,
                h: Size::Px(48.0),
                ..Node::DEFAULT
            },
        );
        ui.a11y_name(&open, "Open Now Playing");
        ui.within(&open, &mut |ui| {
            artwork(
                ui,
                catalog,
                player,
                player.current_id.and_then(|id| catalog.row_index(id)),
                44.0,
                p,
            );
            let labels = container(ui, "labels", Axis::Column, Size::Grow, Size::Fit, p.panel);
            ui.within(&labels, &mut |ui| {
                line(
                    ui,
                    "title",
                    if player.title.is_empty() {
                        "Nothing playing"
                    } else {
                        player.title
                    },
                    14.0,
                    p.ink,
                );
                line(ui, "artist", player.artist, 12.0, p.secondary);
            });
        });
        if ui.signal_state(&open).clicked {
            *intent = Some(Intent::TogglePlayer);
        }
        play(ui, player, 44.0, p, intent);
        if icon_button(ui, "next", "Next", Icon::Next, (44.0, 44.0), false, p) {
            *intent = Some(Intent::Next);
        }
    });
}

fn play(ui: &mut Ui, player: &PlayerView<'_>, size: f32, p: Palette, intent: &mut Option<Intent>) {
    if icon_button(
        ui,
        "play",
        if player.playing { "Pause" } else { "Play" },
        if player.playing {
            Icon::Pause
        } else {
            Icon::Play
        },
        (size, size),
        true,
        p,
    ) {
        *intent = Some(if player.playing {
            Intent::Pause
        } else {
            Intent::Play
        });
    }
}

fn songs(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    if state.route == Route::Find {
        header_view(ui, catalog, state, width, p, intent);
    }
    let Some(collection) = state.collection(catalog) else {
        return;
    };
    if collection.rows.is_empty() {
        line(ui, "empty", "No songs here yet.", 14.0, p.secondary);
        return;
    }
    ui.grid_list(
        "compact-songs",
        collection.rows.len(),
        width,
        64.0,
        &mut |ui, index| {
            let row = catalog.row(collection.rows[index]);
            let node = ui.node(
                "song",
                Node {
                    axis: Axis::Row,
                    align: Align::Center,
                    gap: 12.0,
                    role: Role::ListItem,
                    flags: CLICKABLE | TAB_STOP,
                    w: Size::Grow,
                    h: Size::Grow,
                    padding: Edges {
                        l: 16.0,
                        r: 4.0,
                        ..Edges::ZERO
                    },
                    look: Look {
                        fill: if state.selected == Some(index) {
                            p.selected
                        } else {
                            p.paper
                        },
                        ..Look::NONE
                    },
                    ..Node::DEFAULT
                },
            );
            ui.a11y_name(&node, row.title);
            ui.within(&node, &mut |ui| {
                artwork(ui, catalog, player, Some(collection.rows[index]), 44.0, p);
                let labels = container(ui, "labels", Axis::Column, Size::Grow, Size::Fit, p.paper);
                ui.within(&labels, &mut |ui| {
                    line(
                        ui,
                        "title",
                        row.title,
                        15.0,
                        if player.current_id == Some(row.id) {
                            p.signal
                        } else {
                            p.ink
                        },
                    );
                    line(ui, "artist", row.artist, 12.0, p.secondary);
                });
                text(
                    ui,
                    "duration",
                    row.duration,
                    12,
                    p.secondary,
                    (Size::Px(40.0), 18.0),
                    true,
                );
                if icon_button(
                    ui,
                    "actions",
                    row.actions_label,
                    Icon::Queue,
                    (44.0, 44.0),
                    false,
                    p,
                ) {
                    let rect = ui.rect(&node);
                    *intent = Some(Intent::SongMenu(
                        index,
                        row.id,
                        collection.key,
                        (rect.x + rect.w - 44.0, rect.y),
                    ));
                }
            });
            if ui.signal_state(&node).clicked && intent.is_none() {
                *intent = Some(Intent::PlayRow(index));
            }
        },
    );
}

pub(super) fn collections(
    ui: &mut Ui,
    catalog: &Catalog,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    groups(ui, catalog, player, false, width, p, intent);
}

fn groups(
    ui: &mut Ui,
    catalog: &Catalog,
    player: &PlayerView<'_>,
    artists: bool,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let groups = if artists {
        &catalog.artists
    } else {
        &catalog.collections
    };
    if groups.is_empty() {
        line(ui, "empty-groups", "Nothing here yet.", 14.0, p.secondary);
        return;
    }
    ui.grid_list(
        "compact-groups",
        groups.len(),
        width,
        68.0,
        &mut |ui, index| {
            let group = &groups[index];
            let node = ui.node(
                "group",
                Node {
                    role: Role::Button,
                    flags: CLICKABLE | TAB_STOP,
                    axis: Axis::Row,
                    align: Align::Center,
                    gap: 12.0,
                    w: Size::Grow,
                    h: Size::Grow,
                    padding: Edges {
                        l: 16.0,
                        r: 16.0,
                        ..Edges::ZERO
                    },
                    ..Node::DEFAULT
                },
            );
            ui.a11y_name(&node, &group.title);
            ui.within(&node, &mut |ui| {
                artwork(ui, catalog, player, group.artwork_row, 44.0, p);
                let labels = container(ui, "labels", Axis::Column, Size::Grow, Size::Fit, p.paper);
                ui.within(&labels, &mut |ui| {
                    line(ui, "title", &group.title, 16.0, p.ink);
                    line(ui, "count", &group.count, 12.0, p.secondary);
                });
            });
            if ui.signal_state(&node).clicked {
                *intent = Some(Intent::Navigate(if artists {
                    Route::Artist(index)
                } else {
                    Route::Collection(index)
                }));
            }
        },
    );
}

fn expanded(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    size: (f32, f32),
    generation: u64,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let root = ui.node(
        "expanded-player",
        Node {
            flags: SCROLL | CLIP,
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges {
                l: 24.0,
                r: 24.0,
                ..Edges::ZERO
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let head = container(
            ui,
            "now-heading",
            Axis::Row,
            Size::Grow,
            Size::Px(52.0),
            p.paper,
        );
        ui.within(&head, &mut |ui| {
            line(ui, "heading", "Now playing", 16.0, p.secondary);
            if icon_button(
                ui,
                "close-player",
                "Close Now Playing",
                Icon::Clear,
                (44.0, 44.0),
                false,
                p,
            ) {
                *intent = Some(Intent::TogglePlayer);
            }
        });
        let art_size = (size.0 - 48.0).min((size.1 * 0.40).max(120.0)).max(1.0);
        artwork(
            ui,
            catalog,
            player,
            player.current_id.and_then(|id| catalog.row_index(id)),
            art_size,
            p,
        );
        spacer(ui, "art-gap", 18.0);
        line(
            ui,
            "title",
            if player.title.is_empty() {
                "Nothing playing"
            } else {
                player.title
            },
            24.0,
            p.ink,
        );
        byline::draw(ui, player, catalog, (size.0 - 48.0).max(1.0), p, intent);
        spacer(ui, "clock-gap", 18.0);
        let clock = container(ui, "clock", Axis::Row, Size::Grow, Size::Px(43.0), p.paper);
        ui.within(&clock, &mut |ui| {
            matrix_clock(ui, player.position.max(0.0) as u64, p.ink);
            ui.box_("flex", BoxOpts::default().w(Size::Grow));
            small_clock(
                ui,
                "remaining",
                (player.duration - player.position).max(0.0) as u64,
                true,
                50.0,
                p.secondary,
            );
        });
        seek(ui, player, p, intent);
        let keys = ui.node(
            "keys",
            Node {
                axis: Axis::Row,
                align: Align::Center,
                justify: Justify::Center,
                w: Size::Grow,
                h: Size::Px(72.0),
                gap: 16.0,
                ..Node::DEFAULT
            },
        );
        ui.within(&keys, &mut |ui| {
            if icon_button(
                ui,
                "previous",
                "Previous",
                Icon::Previous,
                (56.0, 56.0),
                false,
                p,
            ) {
                *intent = Some(Intent::Previous);
            }
            play(ui, player, 64.0, p, intent);
            if icon_button(ui, "next", "Next", Icon::Next, (56.0, 56.0), false, p) {
                *intent = Some(Intent::Next);
            }
        });
        let actions = ui.node(
            "actions",
            Node {
                axis: Axis::Row,
                justify: Justify::SpaceBetween,
                w: Size::Grow,
                h: Size::Px(48.0),
                ..Node::DEFAULT
            },
        );
        ui.within(&actions, &mut |ui| {
            if icon_button(
                ui,
                "shuffle",
                if player.shuffled {
                    "Shuffle on"
                } else {
                    "Shuffle off"
                },
                Icon::Shuffle,
                (44.0, 44.0),
                player.shuffled,
                p,
            ) {
                *intent = Some(Intent::Shuffle);
            }
            if icon_button(
                ui,
                "repeat",
                match player.repeat {
                    lw_core::queue::RepeatMode::None => "Repeat off",
                    lw_core::queue::RepeatMode::All => "Repeat all",
                    lw_core::queue::RepeatMode::One => "Repeat one",
                },
                Icon::Repeat,
                (44.0, 44.0),
                player.repeat != lw_core::queue::RepeatMode::None,
                p,
            ) {
                *intent = Some(Intent::Repeat);
            }
            if icon_button(ui, "stop", "Stop", Icon::Stop, (44.0, 44.0), false, p) {
                *intent = Some(Intent::Stop);
            }
            if icon_button(
                ui,
                "queue",
                "Show full queue",
                Icon::Queue,
                (44.0, 44.0),
                false,
                p,
            ) {
                *intent = Some(Intent::ShowQueue);
            }
        });
        volume(ui, player, Size::Grow, p, intent);
        line(ui, "context", player.context, 12.0, p.secondary);
        line(ui, "notice", player.message, 12.0, p.secondary);
    });
    crate::motion::enter(ui, &root, generation, (0.0, 24.0), crate::motion::BASE, 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::{FrameIn, Platform};

    fn catalog(count: usize) -> Catalog {
        crate::tests::prepared(
            (0..count)
                .map(|id| lw_core::Song {
                    id: Some(id as i64),
                    title: format!("Track {id:05}"),
                    artist: "Example artist".into(),
                    album: format!("Album {:02}", id % 20),
                    duration: Some(240.0),
                    ..Default::default()
                })
                .collect(),
        )
    }
    fn frame(
        ui: &mut Ui,
        catalog: &Catalog,
        state: &ShellState,
        player: &PlayerView<'_>,
        width: f32,
        time: f64,
    ) -> Option<Intent> {
        let mut intent = None;
        ui.frame(
            &FrameIn {
                width,
                height: 844.0,
                time,
                environment: pxui::Environment {
                    safe_area: [59.0, 0.0, 34.0, 0.0],
                    ..Default::default()
                },
                ..Default::default()
            },
            &mut |ui| shell(ui, player, catalog, state, &mut intent),
        )
        .unwrap();
        intent
    }
    fn click(ui: &mut Ui, name: &str) {
        let node = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == name)
            .unwrap_or_else(|| panic!("missing button {name}"));
        ui.inject_a11y(&[pxui::A11yAction {
            id: node.id,
            action_code: 1,
            ..Default::default()
        }]);
    }

    #[test]
    fn compact_navigation_player_and_resize_preserve_domain_state() {
        let catalog = catalog(100);
        let mut ui = Ui::create(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Album(0)));
        state.apply(Intent::SelectRow(2));
        let player = PlayerView {
            current_id: Some(0),
            title: "Track 00000",
            artist: "Example artist",
            playing: true,
            initialized: true,
            duration: 240.0,
            ..Default::default()
        };
        for (width, time) in [(1440.0, 1.0), (390.0, 2.0), (390.0, 3.0)] {
            assert_eq!(frame(&mut ui, &catalog, &state, &player, width, time), None);
        }
        let nav = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Main navigation")
            .unwrap();
        assert!(
            (nav.rect.y + nav.rect.h - 810.0).abs() < 0.01,
            "safe-area nav: {:?}",
            nav.rect
        );
        let open = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Open Now Playing")
            .unwrap();
        assert!(open.rect.x >= 0.0 && open.rect.x + open.rect.w <= 390.0 && open.rect.h >= 44.0);
        assert_eq!(state.route, Route::Album(0));
        assert_eq!(state.selected, Some(2));
        click(&mut ui, "Open Now Playing");
        assert_eq!(
            frame(&mut ui, &catalog, &state, &player, 390.0, 4.0),
            Some(Intent::TogglePlayer)
        );
        state.apply(Intent::TogglePlayer);
        frame(&mut ui, &catalog, &state, &player, 390.0, 5.0);
        frame(&mut ui, &catalog, &state, &player, 390.0, 6.0);
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Playback position")
        );
        click(&mut ui, "Pause");
        assert_eq!(
            frame(&mut ui, &catalog, &state, &player, 390.0, 7.0),
            Some(Intent::Pause)
        );
        frame(&mut ui, &catalog, &state, &player, 1440.0, 8.0);
        frame(&mut ui, &catalog, &state, &player, 390.0, 9.0);
        assert!(state.player_expanded && state.panel_visible);
        assert_eq!(state.route, Route::Album(0));
        assert_eq!(state.selected, Some(2));
        click(&mut ui, "Close Now Playing");
        assert_eq!(
            frame(&mut ui, &catalog, &state, &player, 390.0, 10.0),
            Some(Intent::TogglePlayer)
        );
        state.apply(Intent::TogglePlayer);
        frame(&mut ui, &catalog, &state, &player, 390.0, 11.0);
        click(&mut ui, "Lists");
        assert_eq!(
            frame(&mut ui, &catalog, &state, &player, 390.0, 12.0),
            Some(Intent::Navigate(Route::Lists))
        );
        state.apply(Intent::Navigate(Route::Lists));
        frame(&mut ui, &catalog, &state, &player, 390.0, 13.0);
        frame(&mut ui, &catalog, &state, &player, 390.0, 14.0);
        click(&mut ui, "Favorites");
        assert_eq!(
            frame(&mut ui, &catalog, &state, &player, 390.0, 15.0),
            Some(Intent::Navigate(Route::Collection(0)))
        );
    }

    #[test]
    fn compact_ten_thousand_songs_virtualize_and_resize_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog(10_000);
        let mut ui = Ui::create(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let state = ShellState::default();
        let player = PlayerView {
            title: "Track 00000",
            initialized: true,
            duration: 240.0,
            ..Default::default()
        };
        for tick in 0..182 {
            let widths = [
                390.0, 390.0, 620.0, 620.0, 844.0, 844.0, 1100.0, 1100.0, 1440.0, 1440.0,
            ];
            let width = widths[tick % widths.len()];
            mem_sys::local_reset();
            assert_eq!(
                frame(&mut ui, &catalog, &state, &player, width, tick as f64),
                None
            );
            if tick >= 82 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "resize frame {tick} allocated"
                );
            }
            if width >= BREAKPOINT {
                let panel = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| {
                        node.is_role(Role::Button) && ui.a11y_name_of(node) == "Toggle player panel"
                    })
                    .unwrap();
                assert!(
                    panel.rect.x + panel.rect.w <= width,
                    "transport clips at {width}: {:?}",
                    panel.rect
                );
            }
            if width == 390.0 {
                let rows = ui
                    .a11y_nodes()
                    .iter()
                    .filter(|node| {
                        node.is_role(Role::ListItem) && ui.a11y_name_of(node).starts_with("Track ")
                    })
                    .count();
                assert!((1..20).contains(&rows), "unbounded phone rows: {rows}");
            }
        }
        let mut saw_later_row = false;
        for tick in 0..182 {
            let events = [
                pxui::Event {
                    kind: input::wire::POINTER_MOVED,
                    x: 200.0,
                    y: 400.0,
                    ..Default::default()
                },
                pxui::Event {
                    kind: input::wire::SCROLLED,
                    y: if tick % 40 < 20 { -1.0 } else { 1.0 },
                    ..Default::default()
                },
            ];
            mem_sys::local_reset();
            ui.frame(
                &FrameIn {
                    width: 390.0,
                    height: 844.0,
                    time: 1000.0 + tick as f64 / 60.0,
                    events: events.as_ptr(),
                    event_count: events.len(),
                    ..Default::default()
                },
                &mut |ui| shell(ui, &player, &catalog, &state, &mut None),
            )
            .unwrap();
            saw_later_row |= ui
                .a11y_nodes()
                .iter()
                .any(|node| node.is_role(Role::ListItem) && ui.a11y_name_of(node) == "Track 00020");
            if tick >= 82 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "phone scroll frame {tick} allocated"
                );
            }
        }
        assert!(saw_later_row, "phone list must scroll to later songs");
        for tick in 0..182 {
            let time = 2000.0 + tick as f64 / 60.0;
            let point = if tick % 5 == 1 || tick % 5 == 2 {
                if tick % 10 < 5 { 320.0 } else { 480.0 }
            } else {
                400.0
            };
            let touch = pxui::Touch {
                device: u64::MAX,
                finger: 7,
                x: 200.0,
                y: point,
                time,
                pressure: 1.0,
                phase: match tick % 5 {
                    0 | 3 => 0,
                    1 => 1,
                    2 => 2,
                    _ => 3,
                },
            };
            let event = pxui::Event {
                kind: pxui::Event::TOUCH,
                text: pxui::Str {
                    ptr: (&touch as *const pxui::Touch).cast(),
                    len: core::mem::size_of::<pxui::Touch>(),
                },
                ..Default::default()
            };
            let mut intent = None;
            mem_sys::local_reset();
            ui.frame(
                &FrameIn {
                    width: 390.0,
                    height: 844.0,
                    time,
                    events: &event,
                    event_count: 1,
                    ..Default::default()
                },
                &mut |ui| shell(ui, &player, &catalog, &state, &mut intent),
            )
            .unwrap();
            assert_eq!(intent, None, "finger scroll/catch/cancel activated a song");
            if tick >= 82 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "phone finger frame {tick} allocated"
                );
            }
        }
    }
    #[test]
    fn compact_pages_fit_phone_controls_and_keep_warm_frames_allocation_free() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog(200);
        let mut ui = Ui::create(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        let files = crate::files::Snapshot {
            account: Some(1),
            directory: Some(std::sync::Arc::new(crate::files::Directory {
                location: crate::files::Location {
                    user: 1,
                    source: 2,
                    parent: None,
                },
                source_root: 3,
                up: None,
                title: "Music".into(),
                path: "/Music".into(),
                detail: "200 songs".into(),
                entries: Vec::new(),
            })),
            ..Default::default()
        };
        let mut player = PlayerView {
            files: Some(&files),
            current_id: Some(0),
            title: "Track 00000",
            artist: "Example artist",
            album: "Album 00",
            initialized: true,
            duration: 240.0,
            ..Default::default()
        };
        for (page, route) in [
            Route::Albums,
            Route::Artists,
            Route::Lists,
            Route::Files,
            Route::Songs,
        ]
        .into_iter()
        .enumerate()
        {
            state.apply(Intent::Navigate(route));
            if route == Route::Songs {
                state.apply(Intent::TogglePlayer);
            }
            for tick in 0..182 {
                let width = if tick % 4 < 2 { 390.0 } else { 320.0 };
                player.position = tick as f64;
                player.revision = tick as u64;
                mem_sys::local_reset();
                let intent = frame(
                    &mut ui,
                    &catalog,
                    &state,
                    &player,
                    width,
                    (page * 1000 + tick) as f64,
                );
                if tick >= 82 {
                    assert_eq!(
                        mem_sys::local_alloc_count(),
                        0,
                        "{route:?} frame {tick} allocated"
                    );
                }
                assert_eq!(intent, None);
                if route == Route::Files {
                    for label in [
                        "Add folder",
                        "Add audio files",
                        "Back",
                        "Rescan source",
                        "Import folder",
                        "Folder options",
                    ] {
                        let node = ui
                            .a11y_nodes()
                            .iter()
                            .find(|node| {
                                node.is_role(Role::Button) && ui.a11y_name_of(node) == label
                            })
                            .unwrap();
                        assert!(
                            node.rect.x >= 0.0
                                && node.rect.x + node.rect.w <= width
                                && node.rect.h >= 44.0,
                            "{width} {label}: {:?}",
                            node.rect
                        );
                    }
                }
            }
        }
        click(&mut ui, "Show full queue");
        assert_eq!(
            frame(&mut ui, &catalog, &state, &player, 390.0, 6000.0),
            Some(Intent::ShowQueue)
        );
    }
}
