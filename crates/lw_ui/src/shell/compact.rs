//! Select compact composition below 620 points, preserving route, selection, playback and desktop
//! panel preference across resize. Expanded-player state is separate. Apply native safe areas and
//! public keyboard usable bounds to both compositions. Files tools wrap on phones; labels remain
//! worker-prepared. pxui owns touch slop/capture/physics/mouse arbitration, while LocalWave submits
//! typed input and consumes intents. Never add app-owned recognizers to fix routing.
//!
use super::*;
use pxui::GestureOpts;

pub(super) mod list;
pub(super) mod page;

pub(super) const BREAKPOINT: f32 = 620.0;

/// A compact tab bar button: the full width of its slot, 44 points high, its title centred.
const TAB: pxui::Button = pxui::Button::DEFAULT.size(Size::Grow, Size::Px(44.0)).centered();

/// Swift's library mode: label and count, ink with a 2 pt underline when active.
fn mode_tab(ui: &mut Ui, label: &'static str, count: &str, active: bool) -> bool {
    let ink = if active { INK } else { SECONDARY };
    let tab = MODE_TAB.selected(active).name(label);
    ui.button_content(label, &tab, |ui| {
        ui.column("tab", Node::column().w(Size::Fit).h(Size::Grow), |ui| {
            ui.row("content", Node::row().w(Size::Fit).h_px(42.0).gap(6.0).align(Align::Center), |ui| {
                ui.label(label, TEXT_15.h(21.0).color(ink));
                ui.label(count, TEXT_13.h(18.2).color(SECONDARY));
            });
            container(ui, "underline", Axis::Row, Size::Grow, Size::Px(2.0), if active { ui.color(INK) } else { ui.color(PAPER) });
        });
    })
    .clicked
}
/// A library mode tab: as wide as its label and count, 44 points high.
const MODE_TAB: pxui::Button = pxui::Button::DEFAULT.size(Size::Fit, Size::Px(44.0)).centered().look(&BARE);

fn line(ui: &mut Ui, key: &'static str, value: &str, size: f32, color: Rgba) {
    let node = ui.node(
        key.into(),
        Node {
            look: Look {
                text: color,
                ..Look::NONE
            },
            ..Node::column().w(Size::Grow).h_px(size * 1.4).role(Role::Label)
        },
    );
    ui.text_line_box(
        &node,
        value,
        &Label::BODY.size(size).line_height(size * 1.4).lines(1).ellipsis(),
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
) {
    let art = container(
        ui,
        "artwork",
        Axis::Stack,
        Size::Px(size),
        Size::Px(size),
        ui.color(PANEL),
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
    position: f64,
    catalog: &Catalog,
    state: &ShellState,
    intent: &mut Option<Intent>,
) -> Handle {
    let (width, height) = ui.view_size();
    let content = ui.content_rect();
    let (top, right, bottom, left) = (
        content.y,
        width - content.x - content.w,
        height - content.y - content.h,
        content.x,
    );
    let width = (width - left - right).max(1.0);
    let root = ui.node(
        "compact-shell".into(),
        Node {
            padding: Edges {
                t: top.max(state.top_inset),
                r: right,
                b: bottom,
                l: left,
            },
            look: Look {
                fill: ui.color(PAPER),
                ..Look::NONE
            },
            ..Node::column().w(Size::Grow).h(Size::Grow).role(Role::Window)
        },
    );
    ui.a11y_name(&root, "LocalWave");
    ui.within(&root, &mut |ui| {
        if state.player_expanded {
            expanded(
                ui,
                player,
                position,
                catalog,
                (width, height - top.max(state.top_inset) - bottom),
                state.entry_revision,
                intent,
            );
            return;
        }
        let revision = library_revision(player, catalog, state);
        ui.view(
            "compact-library",
            revision,
            Size::Grow,
            Size::Grow,
            &mut |ui| {
                if !state.settings_visible
                    && state.route != Route::Files
                    && state.route != Route::Find
                    && !matches!(state.route, Route::Artist(_) | Route::Album(_) | Route::Collection(_))
                {
                    let library = matches!(state.route, Route::Albums | Route::Artists | Route::Songs);
                    // Select mode swaps the Songs title for Swift's selection header.
                    let picking = state.selecting && state.route == Route::Songs;
                    ui.row(
                        "compact-heading",
                        Node {
                            padding: if picking { Edges::ZERO } else { Edges {
                                l: 16.0,
                                r: if library { 6.0 } else { 16.0 },
                                // Swift's compact header: 10 pt pad with the glyph cap 4 pt higher than a 33.6 box; Lists centres in 54.
                                t: if library { 6.0 } else { 0.0 },
                                b: if library { 0.0 } else { 10.0 },
                                ..Edges::ZERO
                            } },
                            ..Node::row()
                                .w(Size::Grow)
                                .h_px(if picking { 54.0 } else if library { 50.0 } else { 64.0 })
                                .align(if library { Align::Start } else { Align::Center })
                        },
                        |ui| {
                            if picking {
                                let rows = state.collection(catalog).map_or(0, |c| c.rows.len());
                                list::selection_header(ui, state.marked.len(), rows, rows, intent);
                            } else if library {
                                detail::line(ui, "heading", "Library", (32.0, 33.6, 500, -0.035), ui.color(INK), Flags::NONE);
                                // Swift's header gear; Settings stays one tap away.
                                let settings = ui.node(
                                    "settings".into(),
                                    Node::column()
                                        .w_px(44.0)
                                        .h_px(44.0)
                                        .role(Role::Button)
                                        .flags(Flags::CLICKABLE | Flags::TAB_STOP),
                                );
                                ui.a11y_name(&settings, "Settings");
                                let color = ui.color(INK);
                                ui.draw(&settings, move |paint, rect| {
                                    kit::paint_icon(
                                        paint,
                                        Icon::Settings,
                                        rect.x + (rect.w - 20.0) / 2.0,
                                        rect.y + (rect.h - 20.0) / 2.0,
                                        color,
                                    )
                                });
                                if ui.signal(&settings).clicked {
                                    *intent = Some(Intent::ShowSettings);
                                }
                            } else {
                                let title = match state.route {
                                    Route::Lists => "Lists",
                                    _ => "Library",
                                };
                                line(ui, "heading", title, 28.0, ui.color(INK));
                            }
                        },
                    );
                    if library {
                        ui.row(
                            "library-summary",
                            Node {
                                padding: Edges {
                                    l: 16.0,
                                    r: 16.0,
                                    b: 14.0,
                                    ..Edges::ZERO
                                },
                                ..Node::row().w(Size::Grow).h_px(60.0).gap(10.0).align(Align::End)
                            },
                            |ui| {
                                // Swift's MatrixText counter at 46 pt, then "songs" and the totals on the right.
                                clock::matrix_digits(ui, "number", catalog.song_count.as_bytes(), 46.0, ui.color(INK));
                                ui.label("songs", TEXT_15.h(21.0).color(INK));
                                container(ui, "fill", Axis::Row, Size::Grow, Size::Px(1.0), ui.color(PAPER));
                                ui.column("library-size", Node::column().w(Size::Fit).h(Size::Fit).align(Align::End), |ui| {
                                    ui.label(&catalog.totals_label, TEXT_13.h(19.5).color(SECONDARY));
                                    ui.label(&catalog.sources_label, TEXT_13.h(19.5).color(SECONDARY));
                                });
                            },
                        );
                        let tabs = ui.node(
                            "library-tabs".into(),
                            Node::row().w(Size::Grow).h_px(44.0).gap(20.0).pad_x(16.0).align(Align::Center),
                        );
                        ui.set_role(&tabs, Role::Group, 0);
                        ui.a11y_name(&tabs, "Library view");
                        ui.within(&tabs, &mut |ui| {
                            for (label, count, route) in [
                                ("Albums", &catalog.album_count, Route::Albums),
                                ("Artists", &catalog.artist_count, Route::Artists),
                                ("Songs", &catalog.song_count, Route::Songs),
                            ] {
                                if mode_tab(ui, label, count, state.route == route) {
                                    *intent = Some(Intent::Navigate(route));
                                }
                            }
                            if state.route == Route::Songs {
                                container(ui, "sort-fill", Axis::Row, Size::Grow, Size::Px(1.0), ui.color(PAPER));
                                ui.label("Title \u{2193}", TEXT_14.h(19.6).color(INK));
                                if !state.selecting
                                    && state.collection(catalog).is_some_and(|c| !c.rows.is_empty())
                                    && list::word(ui, "select", "Select", false)
                                {
                                    *intent = Some(Intent::SelectMode(true));
                                }
                            }
                        });
                        rule(ui, "tabs-divider", false);
                    }
                }
                let content = ui.view(
                    "compact-content",
                    revision,
                    Size::Grow,
                    Size::Grow,
                    &mut |ui| {
                        let body = container(
                            ui,
                            "compact-content",
                            Axis::Column,
                            Size::Grow,
                            Size::Grow,
                            ui.color(PAPER),
                        );
                        let route_key = crate::hash((
                            state.route,
                            state.settings_visible,
                            if state.route == Route::Find {
                                state.query_revision
                            } else {
                                0
                            },
                        ));
                        ui.within(&body, &mut |ui| {
                            let page = ui.node(
                                Key::Id(route_key as i64),
                                Node::column().w(Size::Grow).h(Size::Grow),
                            );
                            ui.within(&page, &mut |ui| {
                                if state.settings_visible {
                                    settings::view(ui, player, state, width, intent);
                                } else {
                                    match state.route {
                                        Route::Files => files::view(ui, player, state, width, intent),
                                        Route::Albums => {
                                            albums::gallery(ui, catalog, state, player, width, intent)
                                        }
                                        Route::Artists => {
                                            groups(ui, catalog, player, true, width, intent)
                                        }
                                        Route::Lists => collections(ui, catalog, player, width, intent),
                                        _ => list::songs(ui, catalog, state, player, width, intent),
                                    }
                                }
                            });
                        });
                    },
                );
                crate::motion::enter(
                    ui,
                    &content,
                    state.entry_revision,
                    (24.0, 0.0),
                    crate::motion::BASE,
                    0.0,
                );
            },
        );
        rule(ui, "mini-divider", false);
        mini(ui, player, catalog, intent);
        rule(ui, "nav-divider", false);
        let nav = container(
            ui,
            "compact-navigation",
            Axis::Row,
            Size::Grow,
            Size::Px(54.0),
            ui.color(PAPER),
        );
        ui.set_role(&nav, Role::Group, 0);
        ui.a11y_name(&nav, "Main navigation");
        ui.within(&nav, &mut |ui| {
            let selected = match state.route {
                Route::Files => 1,
                Route::Lists | Route::Collection(_) => 2,
                Route::Find => 3,
                _ => 0,
            };
            for (index, (title, route)) in [
                ("Library", Route::Albums),
                ("Folders", Route::Files),
                ("Lists", Route::Lists),
                ("Find", Route::Find),
            ]
            .into_iter()
            .enumerate()
            {
                let current = !state.settings_visible && selected == index;
                let theme = looks(ui);
                let tab = TAB.look(if current { &theme.tab_on } else { &theme.tab }).selected(current).name(title);
                if ui.button_content(title, &tab, |ui| { ui.label(title, TEXT_14.h(19.6).color(INK)); }).clicked {
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
    intent: &mut Option<Intent>,
) {
    ui.row(
        "mini-player",
        Node {
            look: Look {
                fill: ui.color(PANEL),
                ..Look::NONE
            },
            ..Node::row().w(Size::Grow).h_px(64.0).gap(8.0).pad(8.0).align(Align::Center)
        },
        |ui| {
            let open = ui.node(
                "open-player".into(),
                Node::row()
                    .w(Size::Grow)
                    .h_px(48.0)
                    .gap(10.0)
                    .align(Align::Center)
                    .role(Role::Button)
                    .flags(Flags::CLICKABLE | Flags::TAB_STOP),
            );
            ui.a11y_name(&open, "Open Now Playing");
            ui.within(&open, &mut |ui| {
                artwork(
                    ui,
                    catalog,
                    player,
                    player.current_id.and_then(|id| catalog.row_index(id)),
                    44.0,
                );
                let labels = container(ui, "labels", Axis::Column, Size::Grow, Size::Fit, ui.color(PANEL));
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
                        ui.color(INK),
                    );
                    line(ui, "artist", player.artist, 12.0, ui.color(SECONDARY));
                });
            });
            if ui.signal(&open).clicked {
                *intent = Some(Intent::TogglePlayer);
            }
            play(ui, player, (44.0, 44.0), intent);
            if icon_button(ui, "next", "Next", Icon::Next, (44.0, 44.0), false, true) {
                *intent = Some(Intent::Next);
            }
        },
    );
}

fn play(ui: &mut Ui, player: &PlayerView<'_>, size: (f32, f32), intent: &mut Option<Intent>) {
    if icon_button(
        ui,
        "play",
        if player.playing { "Pause" } else { "Play" },
        if player.playing {
            Icon::Pause
        } else {
            Icon::Play
        },
        size,
        true,
        true,
    ) {
        *intent = Some(if player.playing {
            Intent::Pause
        } else {
            Intent::Play
        });
    }
}

pub(super) fn collections(
    ui: &mut Ui,
    catalog: &Catalog,
    player: &PlayerView<'_>,
    width: f32,
    intent: &mut Option<Intent>,
) {
    groups(ui, catalog, player, false, width, intent);
}

fn groups(
    ui: &mut Ui,
    catalog: &Catalog,
    player: &PlayerView<'_>,
    artists: bool,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let groups = if artists {
        &catalog.artists
    } else {
        &catalog.collections
    };
    if groups.is_empty() {
        line(ui, "empty-groups", "Nothing here yet.", 14.0, ui.color(SECONDARY));
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
                "group".into(),
                Node::row()
                    .w(Size::Grow)
                    .h(Size::Grow)
                    .gap(12.0)
                    .pad_x(16.0)
                    .align(Align::Center)
                    .role(Role::Button)
                    .flags(Flags::CLICKABLE | Flags::TAB_STOP),
            );
            ui.a11y_name(&node, &group.title);
            ui.within(&node, &mut |ui| {
                artwork(ui, catalog, player, group.artwork_row, 44.0);
                let labels = container(ui, "labels", Axis::Column, Size::Grow, Size::Fit, ui.color(PAPER));
                ui.within(&labels, &mut |ui| {
                    line(ui, "title", &group.title, 16.0, ui.color(INK));
                    line(ui, "count", &group.count, 12.0, ui.color(SECONDARY));
                });
            });
            if ui.signal(&node).clicked {
                *intent = Some(Intent::Navigate(if artists {
                    Route::Artist(index)
                } else {
                    Route::Collection(index)
                }));
            }
        },
    );
}

/// Swift's `WaveModeKeys` (compact): icon + label key on a light fill, ink when active.
fn mode_key(
    ui: &mut Ui,
    key: &'static str,
    label: &str,
    icon: Icon,
    active: bool,
    mut inner: impl FnMut(&mut Ui),
) -> bool {
    let theme = looks(ui);
    let mode = pxui::Button::DEFAULT.size(Size::Grow, Size::Px(44.0)).centered().look(if active { &theme.ink } else { &theme.key });
    let color = if active { ui.color(PAPER) } else { ui.color(INK) };
    let button = ui.button_content(key, &mode, |ui| {
        ui.row("content", Node::row().w(Size::Fit).h(Size::Fit).gap(6.0).align(Align::Center), |ui| {
            let glyph = ui.node("icon".into(), Node::column().w_px(20.0).h_px(20.0));
            ui.draw(&glyph, move |paint, rect| {
                kit::paint_icon(paint, icon, rect.x, rect.y, color)
            });
            inner(ui);
        });
    });
    ui.a11y_name(&button.handle, label);
    button.clicked
}

#[allow(clippy::too_many_arguments)] // Keep ticking position separate from stable player data.
fn expanded(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    catalog: &Catalog,
    size: (f32, f32),
    generation: u64,
    intent: &mut Option<Intent>,
) {
    use lw_core::queue::RepeatMode;
    let root_node = Node {
        look: Look {
            fill: ui.color(PAPER),
            ..Look::NONE
        },
        ..Node::column().w(Size::Grow).h(Size::Grow).flags(Flags::CLICKABLE)
    };
    let root = ui.node("expanded-player".into(), root_node);
    // Drag down to dismiss (UIKit sheet thresholds); the button below stays the alternative.
    let drag = ui.gesture(
        &root,
        GestureOpts {
            dismiss: Some(Default::default()),
            ..Default::default()
        },
    );
    if drag.dismiss != 0.0 {
        ui.set_node_layout(
            &root,
            &Node {
                w: Size::Px(size.0),
                h: Size::Px(size.1),
                at: Some((0.0, drag.dismiss)).into(),
                ..root_node
            },
        );
    }
    if drag.dismiss_committed && intent.is_none() {
        *intent = Some(Intent::TogglePlayer);
    }
    let favorite = player
        .current_id
        .map(|id| (id, catalog.row_index(id).is_some_and(|i| catalog.row(i).favorite)));
    // Swift `artworkRegionHeight`: what the fixed rows leave, never below 124.
    let region = (size.1 - (48.0 + 86.0 + 10.0 + 16.0 + 50.6 + 14.0 + 72.0 + 14.0 + 77.4 + 14.0 + 48.0)).max(124.0);
    let art_size = (size.0 - 32.0).min(region - 4.0).clamp(1.0, 358.0);
    ui.within(&root, &mut |ui| {
        ui.row("now-heading", Node::row().w(Size::Grow).h_px(48.0).pad_x(6.0).align(Align::Center), |ui| {
            let left = container(ui, "close-holder", Axis::Row, Size::Px(88.0), Size::Px(44.0), ui.color(PAPER));
            ui.within(&left, &mut |ui| {
                if icon_button(ui, "close-player", "Close now playing", Icon::Down, (44.0, 44.0), false, true) {
                    *intent = Some(Intent::TogglePlayer);
                }
            });
            ui.row("playing-from", Node::row().w(Size::Grow).h_px(44.0).gap(4.0).center().role(Role::Label), |ui| {
                ui.label("Playing from", TEXT_13.h(18.2).color(SECONDARY));
                ui.label(player.context, TEXT_13.h(18.2).color(INK));
            });
            container(ui, "balance", Axis::Row, Size::Px(88.0), Size::Px(1.0), ui.color(PAPER));
        });
        ui.row(
            "artwork-region",
            Node {
                justify: Justify::Center,
                padding: Edges { t: 4.0, ..Edges::ZERO },
                ..Node::row().w(Size::Grow).h_px(region)
            },
            |ui| {
                artwork(
                    ui,
                    catalog,
                    player,
                    player.current_id.and_then(|id| catalog.row_index(id)),
                    art_size,
                );
            },
        );
        ui.column(
            "now-body",
            Node {
                padding: Edges { l: 16.0, r: 16.0, t: 16.0, b: 10.0 },
                ..Node::column().w(Size::Grow).h(Size::Grow)
            },
            |ui| {
                let info = ui.node("title-row".into(), Node::row().w(Size::Grow).h_px(50.6).gap(12.0).align(Align::Start));
                ui.within(&info, &mut |ui| {
                    let names = container(ui, "names", Axis::Column, Size::Grow, Size::Fit, ui.color(PAPER));
                    ui.within(&names, &mut |ui| {
                        let title = ui.node(
                            "title".into(),
                            Node {
                                look: Look { text: ui.color(INK), ..Look::NONE },
                                ..Node::column().w(Size::Grow).h_px(27.6).role(Role::Label)
                            },
                        );
                        ui.text_line_box(
                            &title,
                            if player.title.is_empty() { "Nothing playing" } else { player.title },
                            &Label::BODY.size(24.0).weight(500).tracking(TIGHT_3).line_height(27.6).lines(1).ellipsis(),
                            &[],
                        )
                        .unwrap();
                        if player.current_id.is_some() {
                            byline::draw(ui, player, catalog, (size.0 - 32.0 - 48.0).max(1.0), intent);
                        }
                    });
                    if now::favorite_button(ui, favorite)
                        && let Some((song, on)) = favorite
                    {
                        *intent = Some(Intent::SetFavorite { song, on: !on });
                    }
                });
                spacer(ui, "clock-gap", 14.0);
                ui.row("clock", Node::row().w(Size::Grow).h_px(72.0).align(Align::End), |ui| {
                    matrix_clock(ui, position.max(0.0) as u64, 72.0, ui.color(INK));
                    ui.box_("flex", BoxOpts::default().w(Size::Grow));
                    ui.column(
                        "clock-side",
                        Node {
                            padding: Edges { b: 2.0, ..Edges::ZERO },
                            ..Node::column().w_px(80.0).h(Size::Fit).gap(2.0).align(Align::End)
                        },
                        |ui| {
                            if let Some(info) = player.info {
                                ui.label(&info.summary, MONO_12.h(16.8).color(SECONDARY));
                            }
                            small_clock(
                                ui,
                                "remaining",
                                (player.duration - position).max(0.0) as u64,
                                true,
                                50.0,
                                SECONDARY,
                            );
                        },
                    );
                });
                spacer(ui, "seek-gap", 14.0);
                seek(ui, player, position, intent);
                let times = container(ui, "times", Axis::Row, Size::Grow, Size::Px(15.4), ui.color(PAPER));
                ui.within(&times, &mut |ui| {
                    let start = small_clock(ui, "start", 0, false, 40.0, SECONDARY);
                    ui.a11y_name(&start, "Start time");
                    ui.box_("flex", BoxOpts::default().w(Size::Grow));
                    let total = small_clock(ui, "duration", player.duration.max(0.0) as u64, false, 40.0, SECONDARY);
                    ui.a11y_name(&total, "Duration");
                });
                spacer(ui, "modes-gap", 14.0);
                ui.row(
                    "modes",
                    Node {
                        look: Look { fill: ui.color(PANEL), ..Look::NONE },
                        ..Node::row().w(Size::Grow).h_px(48.0).gap(2.0).pad(2.0)
                    },
                    |ui| {
                        if mode_key(
                            ui,
                            "shuffle",
                            if player.shuffled { "Shuffle on" } else { "Shuffle off" },
                            Icon::Shuffle,
                            player.shuffled,
                            |ui| {
                                let c = if player.shuffled { PAPER } else { INK };
                                ui.label("Shuffle", TEXT_14.h(19.6).color(c));
                            },
                        ) {
                            *intent = Some(Intent::Shuffle);
                        }
                        let again = player.repeat != RepeatMode::None;
                        if mode_key(
                            ui,
                            "repeat",
                            match player.repeat {
                                RepeatMode::None => "Repeat off",
                                RepeatMode::All => "Repeat all",
                                RepeatMode::One => "Repeat one",
                            },
                            Icon::Repeat,
                            again,
                            |ui| {
                                let c = if again { PAPER } else { INK };
                                let label = if player.repeat == RepeatMode::One { "One" } else { "Repeat" };
                                ui.label(label, TEXT_14.h(19.6).color(c));
                            },
                        ) {
                            *intent = Some(Intent::Repeat);
                        }
                        if mode_key(ui, "queue", "Show full queue", Icon::Queue, false, |ui| {
                            let count = catalog.queue.as_ref().map_or("0", |queue| queue.count.as_str());
                            ui.label(count, MONO_11.h(15.4).color(INK));
                            ui.label("up next", TEXT_14.h(19.6).color(INK));
                        }) {
                            *intent = Some(Intent::ShowQueue);
                        }
                    },
                );
                if !player.message.is_empty() {
                    line(ui, "notice", player.message, 12.0, ui.color(SECONDARY));
                }
            },
        );
        // Swift `WavePlaybackKeys`: prev / play / next fill an 86 pt bar.
        ui.row(
            "keys",
            Node {
                padding: Edges { l: 2.0, r: 2.0, t: 2.0, b: 0.0 },
                look: Look { fill: ui.color(SELECTED), ..Look::NONE },
                ..Node::row().w(Size::Grow).h_px(86.0).gap(2.0)
            },
            |ui| {
                let key = ((size.0 - 8.0) / 3.0).max(1.0);
                if icon_button(ui, "previous", "Previous", Icon::Previous, (key, 84.0), false, true) {
                    *intent = Some(Intent::Previous);
                }
                play(ui, player, (key, 84.0), intent);
                if icon_button(ui, "next", "Next", Icon::Next, (key, 84.0), false, true) {
                    *intent = Some(Intent::Next);
                }
            },
        );
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
        position: f64,
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
            &mut |ui| shell(ui, player, position, catalog, state, &mut intent),
        )
        .unwrap();
        intent
    }
    /// Compact Songs is title-sorted by an intent; the tests start from its result.
    fn songs_state() -> ShellState {
        ShellState {
            sort: crate::browse::Sort::Title,
            ..ShellState::default()
        }
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
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = songs_state();
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
            assert_eq!(
                frame(&mut ui, &catalog, &state, &player, 0.0, width, time),
                None
            );
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
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 4.0),
            Some(Intent::TogglePlayer)
        );
        state.apply(Intent::TogglePlayer);
        frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 5.0);
        frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 6.0);
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Playback position")
        );
        click(&mut ui, "Pause");
        assert_eq!(
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 7.0),
            Some(Intent::Pause)
        );
        frame(&mut ui, &catalog, &state, &player, 0.0, 1440.0, 8.0);
        frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 9.0);
        assert!(state.player_expanded && state.panel_visible);
        assert_eq!(state.route, Route::Album(0));
        assert_eq!(state.selected, Some(2));
        click(&mut ui, "Close now playing");
        assert_eq!(
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 10.0),
            Some(Intent::TogglePlayer)
        );
        state.apply(Intent::TogglePlayer);
        frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 11.0);
        click(&mut ui, "Lists");
        assert_eq!(
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 12.0),
            Some(Intent::Navigate(Route::Lists))
        );
        state.apply(Intent::Navigate(Route::Lists));
        frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 13.0);
        frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 14.0);
        click(&mut ui, "Favorites");
        assert_eq!(
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 15.0),
            Some(Intent::Navigate(Route::Collection(0)))
        );
    }

    fn names(ui: &Ui) -> Vec<String> {
        ui.a11y_nodes().iter().map(|node| ui.a11y_name_of(node).to_string()).collect()
    }
    /// Activates `name` and returns the intent of the next frame, applying it like the controller.
    fn press(ui: &mut Ui, catalog: &Catalog, state: &mut ShellState, player: &PlayerView<'_>, time: &mut f64, name: &str) -> Option<Intent> {
        click(ui, name);
        *time += 1.0;
        let intent = frame(ui, catalog, state, player, 0.0, 390.0, *time);
        if let Some(intent) = intent {
            state.apply(intent);
        }
        for _ in 0..2 {
            *time += 1.0;
            frame(ui, catalog, state, player, 0.0, 390.0, *time);
        }
        intent
    }

    #[test]
    fn phone_select_mode_marks_rows_and_the_bar_acts_on_them_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog(40);
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = songs_state();
        state.apply(Intent::Navigate(Route::Album(0)));
        let player = PlayerView::default();
        let mut time = 0.0;
        for _ in 0..3 {
            time += 1.0;
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
        }
        let seen = names(&ui);
        for want in ["Back", "Select", "Album actions", "Play", "Shuffle", "Keep offline"] {
            assert!(seen.iter().any(|name| name == want), "missing {want}: {seen:?}");
        }
        assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Select"), Some(Intent::SelectMode(true)));
        assert!(state.selecting());
        let seen = names(&ui);
        for want in ["Select all", "Done", "Play next", "Add to list", "Offline", "Queue"] {
            assert!(seen.iter().any(|name| name == want), "missing {want}: {seen:?}");
        }
        assert!(!seen.iter().any(|name| name == "Back"), "selection header replaces the back row");
        // Row taps toggle marks instead of playing.
        let mut titles: Vec<String> = names(&ui).into_iter().filter(|name| name.starts_with("Track ")).collect();
        titles.dedup();
        assert!(titles.len() >= 2, "{titles:?}");
        for (title, count) in [(&titles[0], 1), (&titles[1], 2), (&titles[0], 1)] {
            let node = ui.a11y_nodes().iter().find(|node| ui.a11y_name_of(node) == title.as_str()).unwrap();
            ui.inject_a11y(&[pxui::A11yAction { id: node.id, action_code: 1, ..Default::default() }]);
            time += 1.0;
            let intent = frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
            assert!(matches!(intent, Some(Intent::Select(Selection::Toggle(_)))), "{intent:?}");
            state.apply(intent.unwrap());
            assert_eq!(state.marked.len(), count);
            for _ in 0..2 {
                time += 1.0;
                frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
            }
        }
        assert!(names(&ui).iter().any(|name| name == "1 selected"), "{:?}", names(&ui));
        let first = state.selected_song_ids(&catalog).next().unwrap();
        assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Play next"), Some(Intent::PlayNext(first)));
        assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Queue"), Some(Intent::AppendSong(first)));
        assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Offline"), Some(Intent::KeepOffline(first)));
        let lists = press(&mut ui, &catalog, &mut state, &player, &mut time, "Add to list");
        assert!(matches!(lists, Some(Intent::SelectionLists(_, id, _, _)) if id == first), "{lists:?}");
        assert!(state.song_menu.is_some() && state.song_lists);
        state.apply(Intent::CloseSongMenu);
        assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Select all"), Some(Intent::Select(Selection::All(state.collection(&catalog).unwrap().rows.len()))));
        assert!(names(&ui).iter().any(|name| name == "Deselect all"));
        // The controller ends the mode after a bar action.
        state.finish_selecting();
        assert!(!state.selecting() && state.marked.is_empty());
        state.apply(Intent::SelectMode(true));
        state.apply(Intent::Select(Selection::Toggle(0)));
        for tick in 0..120 {
            mem_sys::local_reset();
            time += 1.0;
            assert_eq!(frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time), None);
            if tick >= 60 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "select-mode frame {tick} allocated");
            }
        }
        assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Done"), Some(Intent::SelectMode(false)));
        assert!(!state.selecting());
    }

    #[test]
    fn phone_lists_edit_and_remove_and_have_no_offline_key() {
        let catalog = {
            let mut catalog = catalog(10);
            catalog.add_collection(
                "Mix",
                vec![0, 1, 2],
                lw_core::context::PlaybackContext::Playlist { id: 7, name: "Mix".into() },
            );
            catalog
        };
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let player = PlayerView::default();
        let mut time = 0.0;
        let mut state = songs_state();
        state.apply(Intent::Navigate(Route::Collection(0)));
        for _ in 0..3 {
            time += 1.0;
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
        }
        let seen = names(&ui);
        assert!(seen.iter().any(|name| name == "Select") && !seen.iter().any(|name| name == "Keep offline"), "{seen:?}");
        let index = catalog.collections.iter().position(|c| c.title == "Mix").unwrap();
        state.apply(Intent::Navigate(Route::Collection(index)));
        for _ in 0..3 {
            time += 1.0;
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
        }
        assert!(!names(&ui).iter().any(|name| name == "Keep offline"), "Swift's list page has no offline key");
        assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Edit"), Some(Intent::SelectMode(true)));
        state.apply(Intent::Select(Selection::Toggle(1)));
        time += 1.0;
        frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
        let song = state.selected_song_ids(&catalog).next().unwrap();
        assert_eq!(
            press(&mut ui, &catalog, &mut state, &player, &mut time, "Remove"),
            Some(Intent::RemoveFromList { song, list: 7 })
        );
    }

    #[test]
    fn phone_page_menu_offers_swift_page_actions() {
        let catalog = {
            let mut catalog = catalog(10);
            catalog.add_collection("Mix", vec![0, 1], lw_core::context::PlaybackContext::Playlist { id: 7, name: "Mix".into() });
            catalog.add_collection("Other", vec![2], lw_core::context::PlaybackContext::Playlist { id: 8, name: "Other".into() });
            catalog
        };
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let player = PlayerView::default();
        let (mut time, mut state) = (0.0, songs_state());
        state.apply(Intent::Navigate(Route::Album(0)));
        for _ in 0..3 {
            time += 1.0;
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
        }
        let opened = press(&mut ui, &catalog, &mut state, &player, &mut time, "Album actions");
        assert!(matches!(opened, Some(Intent::PageMenu(_))), "{opened:?}");
        let seen = names(&ui);
        for want in ["Play", "Shuffle", "Play next", "Add to queue", "Add to list", "Keep offline"] {
            assert!(seen.iter().any(|name| name == want), "missing {want}: {seen:?}");
        }
        for (label, action) in [
            ("Shuffle", crate::PageAction::Shuffle),
            ("Play next", crate::PageAction::PlayNext),
            ("Add to queue", crate::PageAction::AddToQueue),
        ] {
            state.apply(Intent::PageMenu((380.0, 54.0)));
            for _ in 0..3 {
                time += 1.0;
                frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
            }
            click_menu(&mut ui, label);
            time += 1.0;
            let intent = frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
            assert_eq!(intent, Some(Intent::Page(action)), "{label}");
            assert!(!state.apply(intent.unwrap()), "page actions go to the controller");
            assert!(state.page_menu.is_none(), "{label} closes the menu");
        }
        state.apply(Intent::PageMenu((380.0, 54.0)));
        state.apply(Intent::PageLists);
        for _ in 0..3 {
            time += 1.0;
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
        }
        click_menu(&mut ui, "Other");
        time += 1.0;
        assert_eq!(frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time), Some(Intent::Page(crate::PageAction::AddToList(8))));
        // A playlist page also renames and deletes.
        let index = catalog.collections.iter().position(|c| c.title == "Mix").unwrap();
        state.apply(Intent::Navigate(Route::Collection(index)));
        state.apply(Intent::PageMenu((380.0, 54.0)));
        for _ in 0..3 {
            time += 1.0;
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
        }
        click_menu(&mut ui, "Delete list");
        time += 1.0;
        assert_eq!(frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time), Some(Intent::ConfirmDeleteList(7)));
    }
    #[test]
    fn phone_select_mode_reaches_songs_artist_and_find_pages_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut catalog = catalog(40);
        let player = PlayerView::default();
        let find = |catalog: &mut Catalog| {
            let mut state = songs_state();
            state.apply(Intent::Navigate(Route::Find));
            state.apply(Intent::Search(Query::new("Track").unwrap()));
            catalog.project(state.request(catalog).unwrap(), (0..40).collect());
            state
        };
        let find_state = find(&mut catalog);
        for (page, state) in [("songs", songs_state()), ("artist", {
            let mut state = songs_state();
            state.apply(Intent::Navigate(Route::Artist(0)));
            state
        }), ("find", find_state)] {
            let mut state = state;
            let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
            ui.set_a11y(true);
            let mut time = 0.0;
            for _ in 0..3 {
                time += 1.0;
                frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
            }
            let seen = names(&ui);
            assert!(seen.iter().any(|name| name == "Select"), "{page} lacks Select: {seen:?}");
            assert!(!seen.iter().any(|name| name == "Album actions" || name == "List actions"), "{page}");
            assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Select"), Some(Intent::SelectMode(true)), "{page}");
            let seen = names(&ui);
            for want in ["Select all", "Done", "Play next", "Add to list", "Offline", "Queue"] {
                assert!(seen.iter().any(|name| name == want), "{page} missing {want}: {seen:?}");
            }
            let rows = state.collection(&catalog).unwrap().rows.len();
            assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Select all"), Some(Intent::Select(Selection::All(rows))), "{page}");
            assert!(names(&ui).iter().any(|name| *name == format!("{rows} selected")), "{page}: {:?}", names(&ui));
            let first = state.selected_song_ids(&catalog).next().unwrap();
            assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Queue"), Some(Intent::AppendSong(first)), "{page}");
            for tick in 0..90 {
                mem_sys::local_reset();
                time += 1.0;
                assert_eq!(frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time), None);
                if tick >= 60 {
                    assert_eq!(mem_sys::local_alloc_count(), 0, "{page} select-mode frame {tick} allocated");
                }
            }
            assert_eq!(press(&mut ui, &catalog, &mut state, &player, &mut time, "Done"), Some(Intent::SelectMode(false)), "{page}");
            assert!(!state.selecting() && state.marked.is_empty(), "{page}");
        }
    }

    #[test]
    fn phone_page_menu_duplicates_lists_offers_new_list_and_shows_albums_in_their_folder() {
        let catalog = {
            let mut catalog = catalog(10);
            catalog.add_collection("Mix", vec![0, 1], lw_core::context::PlaybackContext::Playlist { id: 7, name: "Mix".into() });
            catalog
        };
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let player = PlayerView::default();
        let (mut time, mut state) = (0.0, songs_state());
        let menu = |ui: &mut Ui, state: &mut ShellState, time: &mut f64, label: &str| {
            state.apply(Intent::PageMenu((380.0, 54.0)));
            for _ in 0..3 {
                *time += 1.0;
                frame(ui, &catalog, state, &player, 0.0, 390.0, *time);
            }
            click_menu(ui, label);
            *time += 1.0;
            let intent = frame(ui, &catalog, state, &player, 0.0, 390.0, *time);
            if let Some(intent) = intent {
                state.apply(intent);
            }
            intent
        };
        state.apply(Intent::Navigate(Route::Album(0)));
        assert_eq!(menu(&mut ui, &mut state, &mut time, "Show in folder"), Some(Intent::Page(crate::PageAction::ShowInFolder)));
        let index = catalog.collections.iter().position(|c| c.title == "Mix").unwrap();
        state.apply(Intent::Navigate(Route::Collection(index)));
        assert_eq!(menu(&mut ui, &mut state, &mut time, "Duplicate"), Some(Intent::Page(crate::PageAction::Duplicate)));
        // The list choices lead with "New list…", which opens the name dialog for the page's songs.
        state.apply(Intent::Navigate(Route::Album(0)));
        state.apply(Intent::PageMenu((380.0, 54.0)));
        state.apply(Intent::PageLists);
        for _ in 0..3 {
            time += 1.0;
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
        }
        click_menu(&mut ui, "New list…");
        time += 1.0;
        let intent = frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, time);
        assert_eq!(intent, Some(Intent::NewPageList));
        state.apply(intent.unwrap());
        assert_eq!(state.list_dialog, Some(crate::ListDialog::NewPage));
        assert!(state.page_menu.is_none());
    }

    #[test]
    fn touch_dragging_a_playlist_handle_in_edit_mode_reorders_the_list() {
        mem_sys::set_peer(pxui::library_counter);
        let mut c = catalog(30);
        c.add_collection("Mix", vec![0, 1, 2], lw_core::context::PlaybackContext::Playlist { id: 7, name: "Mix".into() });
        let mut state = songs_state();
        state.apply(Intent::Navigate(Route::Collection(c.collections.len() - 1)));
        state.apply(Intent::SelectMode(true));
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut t = 1.0;
        for _ in 0..3 {
            t += 0.5;
            idle(&mut ui, &c, &state, t);
        }
        let grips: Vec<_> = ui.a11y_nodes().iter().filter(|node| ui.a11y_name_of(node) == "Reorder track").map(|node| node.rect).collect();
        assert_eq!(grips.len(), 3, "one handle per row");
        let from = centre(grips[0]);
        let to = (from.0, centre(grips[2]).1 + 20.0);
        assert_eq!(finger(&mut ui, &c, &state, 0, from, t), None);
        // The handle holds the row before it drags (touch hold), so the list never scrolls or refreshes.
        for _ in 0..3 {
            t += 0.2;
            assert_eq!(finger(&mut ui, &c, &state, 1, from, t), None);
        }
        let mut intent = None;
        for (step, fraction) in [0.2, 0.5, 0.9, 1.0].into_iter().enumerate() {
            t += 0.05;
            let at = (from.0, from.1 + (to.1 - from.1) * fraction);
            intent = finger(&mut ui, &c, &state, 1, at, t);
            assert_eq!(intent, None, "step {step}");
        }
        t += 0.05;
        intent = finger(&mut ui, &c, &state, 2, to, t).or(intent);
        assert_eq!(intent, Some(Intent::MoveListRow { from: 0, to: 2 }));
    }

    fn click_menu(ui: &mut Ui, name: &str) {
        let node = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::MenuItem) && ui.a11y_name_of(node) == name)
            .unwrap_or_else(|| panic!("missing menu item {name}"));
        ui.inject_a11y(&[pxui::A11yAction { id: node.id, action_code: 1, ..Default::default() }]);
    }

    #[test]
    fn expanded_player_has_swift_structure_and_its_controls_emit_intents() {
        let catalog = catalog(10);
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = songs_state();
        let info = crate::now_info::NowInfo { summary: "flac 96/24".into(), ..Default::default() };
        let player = PlayerView {
            current_id: Some(0),
            title: "Track 00000",
            artist: "Example artist",
            context: "Weather Systems",
            initialized: true,
            duration: 240.0,
            info: Some(&info),
            ..Default::default()
        };
        frame(&mut ui, &catalog, &state, &player, 61.0, 390.0, 1.0);
        click(&mut ui, "Open Now Playing");
        assert_eq!(frame(&mut ui, &catalog, &state, &player, 61.0, 390.0, 2.0), Some(Intent::TogglePlayer));
        state.apply(Intent::TogglePlayer);
        for time in [3.0, 4.0, 5.0] {
            frame(&mut ui, &catalog, &state, &player, 61.0, 390.0, time);
        }
        let names: Vec<String> = ui.a11y_nodes().iter().map(|node| ui.a11y_name_of(node).to_string()).collect();
        for want in [
            "Close now playing", "Weather Systems", "Favorite current song", "Elapsed time", "Remaining time",
            "Playback position", "Start time", "Duration", "Shuffle off", "Repeat off", "Show full queue",
            "Previous", "Play", "Next",
        ] {
            assert!(names.iter().any(|name| name == want), "missing {want}: {names:?}");
        }
        for (label, want, time) in [
            ("Shuffle off", Intent::Shuffle, 6.0),
            ("Repeat off", Intent::Repeat, 7.0),
            ("Next", Intent::Next, 8.0),
            ("Previous", Intent::Previous, 9.0),
            ("Play", Intent::Play, 10.0),
            ("Favorite current song", Intent::SetFavorite { song: 0, on: true }, 11.0),
        ] {
            click(&mut ui, label);
            assert_eq!(frame(&mut ui, &catalog, &state, &player, 61.0, 390.0, time), Some(want), "{label}");
        }
    }

    #[test]
    fn compact_ten_thousand_songs_virtualize_and_resize_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog(10_000);
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let state = songs_state();
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
                frame(&mut ui, &catalog, &state, &player, 0.0, width, tick as f64),
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
                        node.is_role(Role::Button) && ui.a11y_name_of(node) == "Toggle now playing panel"
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
                &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut None),
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
                &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut intent),
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
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = songs_state();
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
                detail: "200 songs".into(),
                ..Default::default()
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
                let position = tick as f64;
                player.revision = tick as u64;
                mem_sys::local_reset();
                let intent = frame(
                    &mut ui,
                    &catalog,
                    &state,
                    &player,
                    position,
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
                // The first frame of a page is mid-entrance, offset by the slide-in.
                if route == Route::Files && tick > 0 {
                    for label in ["Back", "Play folder", "Folder options"] {
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
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, 6000.0),
            Some(Intent::ShowQueue)
        );
    }

    #[test]
    fn phone_text_fields_follow_keyboard_bounds_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog(200);
        let player = PlayerView::default();
        for mode in 0..3 {
            let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
            ui.set_a11y(true);
            let mut state = songs_state();
            if mode == 0 {
                state.apply(Intent::Navigate(Route::Find));
            }
            if mode == 1 {
                state.apply(Intent::ShowCommands);
            }
            if mode == 2 {
                state.apply(Intent::ShowQueue);
                state.apply(Intent::NamePlaylist);
            }
            for tick in 0..182 {
                let width = if tick % 4 < 2 { 390.0 } else { 320.0 };
                let bottom = [810.0, 700.0, 600.0, 508.0, 600.0, 700.0][tick % 6];
                mem_sys::local_reset();
                let mut intent = None;
                ui.frame(
                    &FrameIn {
                        width,
                        height: 844.0,
                        time: tick as f64,
                        environment: pxui::Environment {
                            safe_area: [59.0, 0.0, 34.0, 0.0],
                            ..Default::default()
                        },
                        keyboard: pxui::Keyboard {
                            frame: [0.0, bottom, width, 844.0 - bottom],
                            flags: pxui::Keyboard::VALID | pxui::Keyboard::ANIMATING,
                        },
                        ..Default::default()
                    },
                    &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut intent),
                )
                .unwrap();
                assert_eq!(intent, None);
                let field = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| node.is_role(Role::TextField))
                    .expect("phone input");
                let rect = field.rect;
                assert!(
                    rect.x >= 0.0
                        && rect.x + rect.w <= width
                        && rect.y >= 59.0
                        && rect.y + rect.h <= bottom,
                    "mode {mode}, width {width}, keyboard {bottom}: {rect:?}"
                );
                if tick >= 82 {
                    assert_eq!(
                        mem_sys::local_alloc_count(),
                        0,
                        "phone keyboard mode {mode}, frame {tick}"
                    );
                }
            }
        }
    }

    #[test]
    fn phone_find_leads_with_swift_caption_field_chips_and_hint_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog(20);
        let mut state = songs_state();
        state.apply(Intent::Navigate(Route::Find));
        let player = PlayerView::default();
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        for tick in 0..6 {
            mem_sys::local_reset();
            frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, f64::from(tick));
            if tick >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "frame {tick}");
            }
        }
        let rect_of = |role: Role, name: &str| {
            ui.a11y_nodes()
                .iter()
                .find(|node| node.is_role(role) && ui.a11y_name_of(node) == name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .rect
        };
        let caption = rect_of(Role::Label, "Find");
        let chip = rect_of(Role::Toggle, "flac");
        let clear = rect_of(Role::Button, "Clear search");
        assert_eq!((chip.h, clear.w, clear.h), (32.0, 44.0, 44.0));
        // 20 pt top pad, 21 pt caption, 56 pt field, 1.5 pt rule, 12 pt gap.
        assert_eq!(chip.y - caption.y, 21.0 + 56.0 + 1.5 + 12.0);
        assert!(chip.x == 16.0 && clear.x + clear.w > 390.0 - 16.0);
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| node.is_role(Role::Paragraph)),
            "usage hint"
        );
    }

    #[test]
    fn phone_find_results_close_with_play_and_keep_offline_keys_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        for (kept, label, expected) in [
            (false, "Keep results offline", Intent::TogglePins),
            (true, "Kept offline", Intent::TogglePins),
        ] {
            let mut catalog = catalog(3);
            if kept {
                catalog.prepare_pins(&mut |_| true);
                catalog.prepare_tables().unwrap();
            }
            let mut state = songs_state();
            state.apply(Intent::Navigate(Route::Find));
            state.apply(Intent::Search(Query::new("Track").unwrap()));
            catalog.project(state.request(&catalog).unwrap(), (0..3).collect());
            let player = PlayerView::default();
            let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
            ui.set_a11y(true);
            for tick in 0..6 {
                mem_sys::local_reset();
                frame(&mut ui, &catalog, &state, &player, 0.0, 390.0, f64::from(tick));
                if tick >= 3 {
                    assert_eq!(mem_sys::local_alloc_count(), 0, "frame {tick}");
                }
            }
            for (name, want) in [("Play results", Intent::PlayRow(0)), (label, expected)] {
                let rect = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == name)
                    .unwrap_or_else(|| panic!("missing {name}"))
                    .rect;
                let press = pxui::Event {
                    kind: pxui::Event::POINTER_PRESSED,
                    x: f64::from(rect.x + rect.w * 0.5),
                    y: f64::from(rect.y + rect.h * 0.5),
                    ..Default::default()
                };
                let events = [press, pxui::Event { kind: pxui::Event::POINTER_RELEASED, ..press }];
                let mut intent = None;
                ui.frame(
                    &FrameIn {
                        width: 390.0,
                        height: 844.0,
                        time: 10.0,
                        events: events.as_ptr(),
                        event_count: 2,
                        environment: pxui::Environment { safe_area: [59.0, 0.0, 34.0, 0.0], ..Default::default() },
                        ..Default::default()
                    },
                    &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut intent),
                )
                .unwrap();
                assert_eq!(intent, Some(want), "{name}");
            }
        }
    }

    #[test]
    fn find_done_dismisses_input_without_playing_the_selected_song() {
        mem_sys::set_peer(pxui::library_counter);
        let mut catalog = catalog(20);
        let mut state = songs_state();
        state.apply(Intent::Navigate(Route::Find));
        state.apply(Intent::Search(Query::new("Track").unwrap()));
        state.apply(Intent::SelectRow(0));
        catalog.project(state.request(&catalog).unwrap(), (0..20).collect());
        let player = PlayerView::default();
        for width in [320.0, 390.0, 1000.0] {
            let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
            ui.set_a11y(true);
            frame(&mut ui, &catalog, &state, &player, 0.0, width, 0.0);
            frame(&mut ui, &catalog, &state, &player, 0.0, width, 1.0);
            let rect = ui
                .a11y_nodes()
                .iter()
                .find(|node| node.is_role(Role::TextField))
                .unwrap()
                .rect;
            for tick in 0..182 {
                let step = tick % 3;
                let mut events = [pxui::Event::default(); 2];
                if step == 0 {
                    events[0] = pxui::Event {
                        kind: pxui::Event::POINTER_PRESSED,
                        x: f64::from(rect.x + rect.w * 0.75),
                        y: f64::from(rect.y + rect.h * 0.5),
                        ..Default::default()
                    };
                    events[1] = pxui::Event {
                        kind: pxui::Event::POINTER_RELEASED,
                        ..events[0]
                    };
                } else {
                    events[0] = pxui::Event {
                        kind: pxui::Event::KEY_PRESSED,
                        code: input::Key::Enter.code(),
                        ..Default::default()
                    };
                    events[1] = pxui::Event {
                        kind: pxui::Event::KEY_RELEASED,
                        ..events[0]
                    };
                }
                mem_sys::local_reset();
                let mut intent = None;
                ui.frame(
                    &FrameIn {
                        width,
                        height: 844.0,
                        time: 2.0 + tick as f64,
                        events: events.as_ptr(),
                        event_count: 2,
                        environment: pxui::Environment {
                            safe_area: [59.0, 0.0, 34.0, 0.0],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut intent),
                )
                .unwrap();
                assert_eq!(
                    intent,
                    if step == 2 {
                        Some(Intent::PlayRow(0))
                    } else {
                        None
                    },
                    "width {width}, frame {tick}"
                );
                assert_eq!(
                    ui.ime().is_some(),
                    step == 0,
                    "width {width}, frame {tick}, field={rect:?}, now={:?}",
                    ui.a11y_nodes()
                        .iter()
                        .find(|node| node.is_role(Role::TextField))
                        .unwrap()
                        .rect
                );
                if step == 0 {
                    assert_eq!(
                        ui.text_input_options(),
                        pxui::TextInputOptions::SEARCH_FIELD.action(pxui::TextInputOptions::DONE)
                    );
                }
                if tick >= 82 {
                    assert_eq!(
                        mem_sys::local_alloc_count(),
                        0,
                        "Find focus/Done/shortcut frame {tick}"
                    );
                }
            }
        }
    }

    /// One finger event (`phase` 0 down, 1 move, 2 up) then a frame at `time`.
    fn finger(
        ui: &mut Ui,
        c: &Catalog,
        state: &ShellState,
        phase: u32,
        at: (f32, f32),
        time: f64,
    ) -> Option<Intent> {
        let touch = pxui::Touch {
            device: u64::MAX,
            finger: 3,
            x: f64::from(at.0),
            y: f64::from(at.1),
            time,
            pressure: 1.0,
            phase,
        };
        let event = pxui::Event {
            kind: pxui::Event::TOUCH,
            text: pxui::Str {
                ptr: (&touch as *const pxui::Touch).cast(),
                len: core::mem::size_of::<pxui::Touch>(),
            },
            ..Default::default()
        };
        let player = PlayerView {
            initialized: true,
            ..Default::default()
        };
        let mut intent = None;
        ui.frame(
            &FrameIn {
                width: 390.0,
                height: 844.0,
                time,
                events: &event,
                event_count: 1,
                environment: pxui::Environment {
                    safe_area: [59.0, 0.0, 34.0, 0.0],
                    ..Default::default()
                },
                ..Default::default()
            },
            &mut |ui| shell(ui, &player, 0.0, c, state, &mut intent),
        )
        .unwrap();
        intent
    }
    fn idle(ui: &mut Ui, c: &Catalog, state: &ShellState, time: f64) -> Option<Intent> {
        let player = PlayerView {
            initialized: true,
            ..Default::default()
        };
        frame(ui, c, state, &player, 0.0, 390.0, time)
    }
    fn rect_of(ui: &Ui, role: Role, name: &str) -> pxui::geom::Rect {
        ui.a11y_nodes()
            .iter()
            .find(|node| node.is_role(role) && ui.a11y_name_of(node) == name)
            .unwrap_or_else(|| panic!("missing {name}"))
            .rect
    }
    fn centre(r: pxui::geom::Rect) -> (f32, f32) {
        (r.x + r.w / 2.0, r.y + r.h / 2.0)
    }
    /// Drag a row horizontally by `dx` (negative reveals the trailing side) and release; returns
    /// the clock after the spring settles and the intent the release produced.
    fn swipe_row(
        ui: &mut Ui,
        c: &Catalog,
        state: &ShellState,
        name: &str,
        dx: f32,
        mut t: f64,
    ) -> (f64, Option<Intent>) {
        let (x, y) = {
            let r = rect_of(ui, Role::ListItem, name);
            (if dx < 0.0 { r.x + 250.0 } else { r.x + 60.0 }, r.y + r.h / 2.0)
        };
        assert_eq!(finger(ui, c, state, 0, (x, y), t), None);
        for (step, fraction) in [0.15, 0.5, 1.0].into_iter().enumerate() {
            t += 0.016;
            assert_eq!(finger(ui, c, state, 1, (x + dx * fraction, y), t), None, "step {step}");
        }
        t += 0.4;
        let intent = finger(ui, c, state, 2, (x + dx, y), t);
        for _ in 0..4 {
            t += 0.5;
            idle(ui, c, state, t);
        }
        (t, intent)
    }
    /// Tap a revealed action button; returns the clock and the intent.
    fn tap_action(ui: &mut Ui, c: &Catalog, state: &ShellState, name: &str, mut t: f64) -> (f64, Option<Intent>) {
        let (x, y) = centre(rect_of(ui, Role::Button, name));
        t += 0.1;
        finger(ui, c, state, 0, (x, y), t);
        t += 0.05;
        let intent = finger(ui, c, state, 2, (x, y), t);
        for _ in 0..4 {
            t += 0.5;
            idle(ui, c, state, t);
        }
        (t, intent)
    }

    #[test]
    fn touch_swipe_actions_emit_intents_close_and_never_tap_through() {
        mem_sys::set_peer(pxui::library_counter);
        let c = catalog(30);
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let state = songs_state();
        let mut t = 1.0;
        idle(&mut ui, &c, &state, t);
        // The page's slide-in finishes before rows are measured.
        t += 1.0;
        idle(&mut ui, &c, &state, t);
        for round in 0..4 {
            mem_sys::local_reset();
            let resting = rect_of(&ui, Role::ListItem, "Track 00000").x;
            // A short drag left opens the trailing strip; its button queues the song and closes.
            let (next, intent) = swipe_row(&mut ui, &c, &state, "Track 00000", -60.0, t + 1.0);
            assert_eq!(intent, None, "a swipe is not a tap");
            let shifted = rect_of(&ui, Role::ListItem, "Track 00000").x;
            assert!(shifted <= resting - 60.0, "row shows its action: {resting} -> {shifted}");
            let (next, intent) = tap_action(&mut ui, &c, &state, "Add to queue", next);
            assert_eq!(intent, Some(Intent::AppendSong(0)));
            assert_eq!(rect_of(&ui, Role::ListItem, "Track 00000").x, resting, "closed after the action");
            // A short drag right opens the leading strip: Play next.
            let (next, intent) = swipe_row(&mut ui, &c, &state, "Track 00000", 60.0, next + 0.5);
            assert_eq!(intent, None);
            assert!(rect_of(&ui, Role::ListItem, "Track 00000").x >= resting + 60.0);
            let (next, intent) = tap_action(&mut ui, &c, &state, "Play next", next);
            assert_eq!(intent, Some(Intent::PlayNext(0)));
            // A full swipe commits its side on release without opening, and never plays the row.
            let (next, intent) = swipe_row(&mut ui, &c, &state, "Track 00001", -140.0, next + 0.5);
            assert_eq!(intent, Some(Intent::AppendSong(1)));
            assert_eq!(rect_of(&ui, Role::ListItem, "Track 00001").x, resting);
            let (next, intent) = swipe_row(&mut ui, &c, &state, "Track 00001", 140.0, next + 0.5);
            assert_eq!(intent, Some(Intent::PlayNext(1)));
            assert_eq!(rect_of(&ui, Role::ListItem, "Track 00001").x, resting);
            // Swiping another row closes the open one.
            let (next, _) = swipe_row(&mut ui, &c, &state, "Track 00000", -60.0, next + 0.5);
            let (mut t2, _) = swipe_row(&mut ui, &c, &state, "Track 00001", -60.0, next + 0.5);
            assert_eq!(rect_of(&ui, Role::ListItem, "Track 00000").x, resting, "first row closed");
            assert!(rect_of(&ui, Role::ListItem, "Track 00001").x < resting - 60.0);
            // Tapping an open row closes it instead of playing.
            let r = rect_of(&ui, Role::ListItem, "Track 00001");
            finger(&mut ui, &c, &state, 0, (100.0, r.y + 30.0), t2 + 0.1);
            assert_eq!(finger(&mut ui, &c, &state, 2, (100.0, r.y + 30.0), t2 + 0.15), None);
            t2 += 0.15;
            for _ in 0..4 {
                t2 += 0.5;
                idle(&mut ui, &c, &state, t2);
            }
            assert_eq!(rect_of(&ui, Role::ListItem, "Track 00001").x, resting);
            t = t2;
            if round >= 2 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm swipe round {round}");
            }
        }
    }

    #[test]
    fn touch_swipe_trailing_removes_inside_a_playlist() {
        mem_sys::set_peer(pxui::library_counter);
        let mut c = catalog(30);
        c.add_collection(
            "Mix",
            vec![0, 1, 2],
            lw_core::context::PlaybackContext::Playlist {
                id: 7,
                name: "Mix".into(),
            },
        );
        let mut state = songs_state();
        state.apply(Intent::Navigate(Route::Collection(c.collections.len() - 1)));
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut t = 1.0;
        idle(&mut ui, &c, &state, t);
        idle(&mut ui, &c, &state, t + 0.5);
        t += 1.0;
        let (next, intent) = swipe_row(&mut ui, &c, &state, "Track 00001", -60.0, t);
        assert_eq!(intent, None);
        let (next, intent) = tap_action(&mut ui, &c, &state, "Remove", next);
        assert_eq!(intent, Some(Intent::RemoveFromList { song: 1, list: 7 }));
        let (_, intent) = swipe_row(&mut ui, &c, &state, "Track 00002", -140.0, next + 0.5);
        assert_eq!(intent, Some(Intent::RemoveFromList { song: 2, list: 7 }));
    }

    fn lettered(count: usize) -> Catalog {
        crate::tests::prepared(
            (0..count)
                .map(|id| lw_core::Song {
                    id: Some(id as i64),
                    title: format!("{}{id:02}", (b'A' + (id / 2 % 26) as u8) as char),
                    artist: "Example artist".into(),
                    album: format!("Album {:02}", id % 4),
                    duration: Some(240.0),
                    ..Default::default()
                })
                .collect(),
        )
    }
    fn has(ui: &Ui, role: Role, name: &str) -> bool {
        ui.a11y_nodes()
            .iter()
            .any(|node| node.is_role(role) && ui.a11y_name_of(node) == name)
    }

    #[test]
    fn songs_page_has_swift_header_tab_order_and_a_working_alphabet_index() {
        mem_sys::set_peer(pxui::library_counter);
        let c = lettered(52);
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let state = songs_state();
        let mut t = 1.0;
        idle(&mut ui, &c, &state, t);
        t += 1.0;
        idle(&mut ui, &c, &state, t);
        // Header: title, counts on the tabs, Settings by icon, Swift's tab order.
        assert!(has(&ui, Role::Button, "Settings"));
        assert!(!has(&ui, Role::Button, "Files"));
        for tab in ["Albums", "Artists", "Songs"] {
            assert!(has(&ui, Role::Button, tab), "{tab}");
        }
        let mut nav: Vec<_> = ["Library", "Folders", "Lists", "Find"]
            .into_iter()
            .map(|name| (rect_of(&ui, Role::Button, name).x, name))
            .collect();
        let order: Vec<_> = nav.iter().map(|entry| entry.1).collect();
        nav.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(nav.iter().map(|entry| entry.1).collect::<Vec<_>>(), order, "tab bar order");
        assert!(has(&ui, Role::Label, "52") || has(&ui, Role::Label, "songs"));
        // Rows carry the prepared "Artist · Album" line and a "…" menu.
        assert!(has(&ui, Role::Label, "Example artist · Album 00"));
        assert!(has(&ui, Role::Button, "Actions for A00"));
        assert!(has(&ui, Role::Group, "Section index"));
        // Tapping a letter jumps to its first song; dragging over the index follows the finger.
        assert!(!has(&ui, Role::ListItem, "Z50"));
        click(&mut ui, "Z");
        for step in 1..4 {
            idle(&mut ui, &c, &state, t + f64::from(step));
        }
        t += 4.0;
        assert!(has(&ui, Role::ListItem, "Z50"), "jumped to Z");
        let top = rect_of(&ui, Role::ListItem, "Z50").y;
        assert!(top < 700.0, "Z is in view: {top}");
        let (x, y) = centre(rect_of(&ui, Role::Button, "A"));
        let (_, m) = centre(rect_of(&ui, Role::Button, "M"));
        let pointer = |ui: &mut Ui, kind: u32, y: f32, time: f64| {
            let event = pxui::Event {
                kind,
                x: f64::from(x),
                y: f64::from(y),
                ..Default::default()
            };
            let player = PlayerView {
                initialized: true,
                ..Default::default()
            };
            ui.frame(
                &FrameIn {
                    width: 390.0,
                    height: 844.0,
                    time,
                    events: &event,
                    event_count: 1,
                    environment: pxui::Environment {
                        safe_area: [59.0, 0.0, 34.0, 0.0],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                &mut |ui| shell(ui, &player, 0.0, &c, &state, &mut None),
            )
            .unwrap();
        };
        pointer(&mut ui, pxui::Event::POINTER_PRESSED, y, t);
        pointer(&mut ui, pxui::Event::POINTER_MOVED, (y + m) / 2.0, t + 0.1);
        pointer(&mut ui, pxui::Event::POINTER_MOVED, m, t + 0.2);
        pointer(&mut ui, pxui::Event::POINTER_RELEASED, m, t + 0.3);
        for step in 1..4 {
            idle(&mut ui, &c, &state, t + 1.0 + f64::from(step));
        }
        assert!(has(&ui, Role::ListItem, "M24"), "dragged to M");
        // A finger drag over the rail follows the finger too (pxui touch_drag), not the rows' swipe or scroll.
        t += 5.0;
        let (fx, fy) = centre(rect_of(&ui, Role::Button, "A"));
        let (_, ty) = centre(rect_of(&ui, Role::Button, "T"));
        finger(&mut ui, &c, &state, 0, (fx, fy), t);
        finger(&mut ui, &c, &state, 1, (fx, (fy + ty) / 2.0), t + 0.05);
        finger(&mut ui, &c, &state, 1, (fx, ty), t + 0.1);
        finger(&mut ui, &c, &state, 2, (fx, ty), t + 0.15);
        for step in 1..4 {
            idle(&mut ui, &c, &state, t + 1.0 + f64::from(step));
        }
        assert!(has(&ui, Role::ListItem, "T38"), "finger dragged to T");
        mem_sys::local_reset();
        for step in 0..6 {
            idle(&mut ui, &c, &state, t + 10.0 + f64::from(step));
        }
        assert_eq!(mem_sys::local_alloc_count(), 0, "warm Songs page");
    }

    #[test]
    fn artist_page_has_back_summary_play_shuffle_album_cards_and_songs() {
        mem_sys::set_peer(pxui::library_counter);
        let c = catalog(30);
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = songs_state();
        state.apply(Intent::Navigate(Route::Artist(0)));
        let mut t = 1.0;
        for _ in 0..3 {
            idle(&mut ui, &c, &state, t);
            t += 1.0;
        }
        assert!(has(&ui, Role::Label, "20 albums · 30 songs"));
        assert!(has(&ui, Role::Label, "Songs"));
        assert!(has(&ui, Role::Label, "Example artist"));
        assert!(!has(&ui, Role::Button, "Settings"), "the artist page replaces the Library header");
        for (name, want) in [
            ("Back", Intent::Navigate(Route::Artists)),
            ("Play", Intent::PlayRow(0)),
            ("Shuffle", Intent::ShuffleCollection),
            ("Album 00", Intent::Navigate(Route::Album(0))),
        ] {
            click(&mut ui, name);
            assert_eq!(idle(&mut ui, &c, &state, t), Some(want), "{name}");
            t += 1.0;
            idle(&mut ui, &c, &state, t);
            t += 1.0;
        }
        mem_sys::local_reset();
        for step in 0..6 {
            idle(&mut ui, &c, &state, t + f64::from(step));
        }
        assert_eq!(mem_sys::local_alloc_count(), 0, "warm artist page");
    }

    #[test]
    fn album_and_list_pages_have_back_title_summary_and_play_shuffle_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut c = catalog(30);
        c.add_collection("Mix", vec![0, 1, 2], lw_core::context::PlaybackContext::Playlist { id: 7, name: "Mix".into() });
        c.add_collection("Empty", vec![], lw_core::context::PlaybackContext::Playlist { id: 8, name: "Empty".into() });
        let lists = c.collections.len();
        let routes = [
            (Route::Album(0), Route::Albums, "Example artist"),
            (Route::Collection(lists - 2), Route::Lists, ""),
            (Route::Collection(lists - 1), Route::Lists, ""),
        ];
        let mut t = 1.0;
        for (route, parent, link) in routes {
            // A fresh Ui per page: a new state restarts the revision the retained library view keys on.
            let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
            ui.set_a11y(true);
            let mut state = songs_state();
            state.apply(Intent::Navigate(route));
            for _ in 0..3 {
                idle(&mut ui, &c, &state, t);
                t += 1.0;
            }
            assert!(!has(&ui, Role::Button, "Settings"), "the detail lead replaces the Library header");
            if !link.is_empty() {
                assert!(has(&ui, Role::Button, link), "album artist link");
            }
            for (name, want) in [("Back", Intent::Navigate(parent)), ("Play", Intent::PlayRow(0)), ("Shuffle", Intent::ShuffleCollection)] {
                click(&mut ui, name);
                assert_eq!(idle(&mut ui, &c, &state, t), Some(want), "{name} on {route:?}");
                t += 1.0;
                idle(&mut ui, &c, &state, t);
                t += 1.0;
            }
            mem_sys::local_reset();
            for step in 0..6 {
                idle(&mut ui, &c, &state, t + f64::from(step));
            }
            assert_eq!(mem_sys::local_alloc_count(), 0, "warm detail page");
        }
    }

    /// Drag the finger straight down from `from` by `dy` in three moves and release.
    fn drag_down(ui: &mut Ui, c: &Catalog, state: &ShellState, from: (f32, f32), dy: f32, mut t: f64) -> (f64, Option<Intent>) {
        assert_eq!(finger(ui, c, state, 0, from, t), None);
        // Slow enough that only the distance, never the release velocity, can commit.
        for fraction in [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0] {
            t += 0.1;
            assert_eq!(finger(ui, c, state, 1, (from.0, from.1 + dy * fraction), t), None);
        }
        t += 0.5;
        let intent = finger(ui, c, state, 2, (from.0, from.1 + dy), t);
        for _ in 0..4 {
            t += 0.5;
            idle(ui, c, state, t);
        }
        (t, intent)
    }

    #[test]
    fn expanded_player_dismisses_by_dragging_down_and_springs_back_when_short() {
        mem_sys::set_peer(pxui::library_counter);
        let c = catalog(10);
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = songs_state();
        state.apply(Intent::TogglePlayer);
        let mut t = 1.0;
        for _ in 0..3 {
            idle(&mut ui, &c, &state, t);
            t += 1.0;
        }
        let rest = rect_of(&ui, Role::Button, "Close now playing").y;
        for round in 0..4 {
            mem_sys::local_reset();
            let (next, intent) = drag_down(&mut ui, &c, &state, (195.0, 250.0), 60.0, t + 1.0);
            assert_eq!(intent, None, "short drag, round {round}");
            assert_eq!(rect_of(&ui, Role::Button, "Close now playing").y, rest, "springs back");
            let (next, intent) = drag_down(&mut ui, &c, &state, (195.0, 250.0), 400.0, next + 1.0);
            assert_eq!(intent, Some(Intent::TogglePlayer), "long drag, round {round}");
            // The shell closes the player; opening it again starts from rest.
            state.apply(Intent::TogglePlayer);
            idle(&mut ui, &c, &state, next + 1.0);
            state.apply(Intent::TogglePlayer);
            t = next + 2.0;
            for _ in 0..2 {
                idle(&mut ui, &c, &state, t);
                t += 1.0;
            }
            if round >= 2 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm dismiss round {round}");
            }
        }
    }

    #[test]
    fn queue_sheet_dismisses_by_dragging_down_and_springs_back_when_short() {
        mem_sys::set_peer(pxui::library_counter);
        let c = catalog(10);
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = songs_state();
        state.apply(Intent::ShowQueue);
        let mut t = 1.0;
        for _ in 0..3 {
            idle(&mut ui, &c, &state, t);
            t += 1.0;
        }
        let sheet = rect_of(&ui, Role::Dialog, "Playback queue");
        let from = (195.0, sheet.y + 10.0);
        for round in 0..4 {
            mem_sys::local_reset();
            let (next, intent) = drag_down(&mut ui, &c, &state, from, 30.0, t + 1.0);
            assert_eq!(intent, None, "short drag, round {round}");
            assert_eq!(rect_of(&ui, Role::Dialog, "Playback queue").y, sheet.y, "springs back");
            let (next, intent) = drag_down(&mut ui, &c, &state, from, 200.0, next + 1.0);
            assert_eq!(intent, Some(Intent::CloseQueue), "long drag, round {round}");
            state.apply(Intent::CloseQueue);
            idle(&mut ui, &c, &state, next + 1.0);
            state.apply(Intent::ShowQueue);
            t = next + 2.0;
            for _ in 0..2 {
                idle(&mut ui, &c, &state, t);
                t += 1.0;
            }
            if round >= 2 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm dismiss round {round}");
            }
        }
    }

    #[test]
    fn touch_long_press_opens_the_track_menu_without_a_tap() {
        mem_sys::set_peer(pxui::library_counter);
        let c = catalog(30);
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = songs_state();
        let mut t = 1.0;
        idle(&mut ui, &c, &state, t);
        // The page's slide-in finishes before rows are measured.
        t += 1.0;
        idle(&mut ui, &c, &state, t);
        for round in 0..4 {
            mem_sys::local_reset();
            let r = rect_of(&ui, Role::ListItem, "Track 00002");
            let at = (r.x + 120.0, r.y + r.h / 2.0);
            t += 1.0;
            assert_eq!(finger(&mut ui, &c, &state, 0, at, t), None);
            assert_eq!(idle(&mut ui, &c, &state, t + 0.2), None);
            let opened = idle(&mut ui, &c, &state, t + 0.6);
            assert!(
                matches!(opened, Some(Intent::SongMenu(2, 2, _, _))),
                "{opened:?}"
            );
            state.apply(opened.unwrap());
            // Lifting the finger after the long press must not play the row.
            assert_eq!(finger(&mut ui, &c, &state, 2, at, t + 0.7), None);
            idle(&mut ui, &c, &state, t + 0.8);
            rect_of(&ui, Role::MenuItem, "Play next");
            let behind = rect_of(&ui, Role::ListItem, "Track 00002");
            assert_eq!(
                (behind.x, behind.y),
                (r.x, r.y),
                "the blurred backdrop leaves the page in place"
            );
            state.apply(Intent::CloseSongMenu);
            idle(&mut ui, &c, &state, t + 1.0);
            t += 1.0;
            if round >= 2 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm long-press round {round}"
                );
            }
        }
    }

    #[test]
    fn touch_pull_to_refresh_asks_once_and_ends_with_the_next_catalog() {
        mem_sys::set_peer(pxui::library_counter);
        let c = catalog(30);
        let mut ui = crate::test_ui(Platform::Apple, Some(18), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = songs_state();
        let mut t = 1.0;
        idle(&mut ui, &c, &state, t);
        let mut asked = 0;
        for round in 0..4 {
            mem_sys::local_reset();
            t += 1.0;
            let y = rect_of(&ui, Role::ListItem, "Track 00000").y + 20.0;
            finger(&mut ui, &c, &state, 0, (200.0, y), t);
            for (i, dy) in [10.0, 60.0, 140.0, 200.0].into_iter().enumerate() {
                asked += usize::from(
                    finger(
                        &mut ui,
                        &c,
                        &state,
                        1,
                        (200.0, y + dy),
                        t + 0.016 * (i + 1) as f64,
                    ) == Some(Intent::RefreshLibrary),
                );
            }
            t += 0.5;
            asked += usize::from(
                finger(&mut ui, &c, &state, 2, (200.0, y + 200.0), t)
                    == Some(Intent::RefreshLibrary),
            );
            for _ in 0..3 {
                t += 0.3;
                asked += usize::from(idle(&mut ui, &c, &state, t) == Some(Intent::RefreshLibrary));
            }
            assert_eq!(asked, round + 1, "one refresh per pull");
            assert!(state.refresh_wait.get().is_some(), "held while loading");
            // A delivered catalog ends the hold.
            state.refresh_epoch += 1;
            for _ in 0..4 {
                t += 0.5;
                idle(&mut ui, &c, &state, t);
            }
            assert!(state.refresh_wait.get().is_none(), "ended with the catalog");
            if round >= 2 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm refresh round {round}"
                );
            }
        }
    }
}
