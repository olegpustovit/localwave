use super::*;
use crate::Intent;
use crate::catalog::{Catalog, Route};
use lw_core::context::PlaybackContext;

const PADDING: f32 = 14.0;
const TOP: f32 = 28.0;
const SEARCH_GAP: f32 = 12.0;
const GROUP_HEIGHT: f32 = 2.0 + 14.0 + 16.8 + 6.0;
const OFFLINE_HEIGHT: f32 = 14.0 + 16.8;
const METER_HEIGHT: f32 = 10.0 + 4.0;
const SHORTCUTS_HEIGHT: f32 = 10.0 + 16.8 + 4.0;
/// A Swift group label: 12-point medium secondary text 8 points in, 14 above and 6 below, after the menu's 2-point spacing.
const GROUP: Label = TEXT_12.weight(500).grow().color(SECONDARY);
const GROUP_BLOCK: Node = Node::row()
    .w(Size::Grow)
    .h_px(GROUP_HEIGHT)
    .padding(Edges::ZERO.l(8.0).t(16.0).r(8.0));
/// The Library group's rows: title and route.
const LIBRARY: [(&str, Route); 3] = [
    ("Albums", Route::Albums),
    ("Artists", Route::Artists),
    ("Songs", Route::Songs),
];
/// The Library group: sidebar rows (each carries the menu's 2-point spacing above it), as tall as its three rows.
const SIDE: pxui::SelectionList = pxui::SelectionList::DEFAULT
    .variant(variant::SIDE_ROW)
    .bare()
    .fit();
/// The Lists group: the sidebar rows, scrolling, "+ New list" after the last. A song carried over a playlist row lights it in the
/// selected fill (the other lists are `NO_DROP` rows).
const SIDE_LISTS: pxui::SelectionList = pxui::SelectionList::DEFAULT
    .variant(variant::SIDE_ROW)
    .bare()
    .drop_look(SELECTED_FILL);
/// A sidebar row's title (ink while its row is selected) and its count (the UI face's tabular digits, at the row's end).
const SIDE_TITLE: Label = TEXT_14.grow().color(SECONDARY);
const SIDE_COUNT: Label = TEXT_12.tabular().color(SECONDARY);
/// A source row: a navigation button, never selected.
const SOURCE_ROW: pxui::Button = pxui::Button::DEFAULT
    .variant(variant::ROW)
    .size(Size::Grow, Size::Grow)
    .align(Justify::Start)
    .pad(Edges::new(8.0, 0.0, 8.0, 0.0))
    .look(&SIDE_ROW_LOOKS);
/// A source that needs reconnecting, 6 points after its name.
const RECONNECT: pxui::Badge = pxui::Badge::DEFAULT.variant(variant::BADGE);
/// A "+ label" action row (Swift's Add folder or files, New list): Swift's 16-point plus, 8 points to its label, 8 in.
const ADD_ROW: pxui::Button = pxui::Button::DEFAULT
    .variant(variant::ROW)
    .size(Size::Grow, Size::Grow)
    .align(Justify::Start)
    .pad(Edges::new(8.0, 0.0, 8.0, 0.0))
    .look(&BARE)
    .icon(icon_drawing(Icon::Plus))
    .icon_size(16.0)
    .gap(8.0)
    .label(TEXT_14.color(SECONDARY));
/// The Search row: the panel fill, Swift's 16-point lens 8 points before "Search", "⌘K" at its end.
const SEARCH: pxui::Button = pxui::Button::DEFAULT
    .variant(variant::ROW)
    .size(Size::Grow, Size::Grow)
    .align(Justify::Start)
    .pad(Edges::new(10.0, 0.0, 10.0, 0.0))
    .look(&SEARCH_ROW_LOOKS)
    .icon(icon_drawing(Icon::Search))
    .icon_size(16.0)
    .gap(8.0)
    .label(TEXT_14.grow().color(SECONDARY))
    .accessory("⌘K")
    .accessory_label(MONO_11.color(SECONDARY));
/// The shortcuts key: "?" in Geist Mono 11 on the panel fill in a hairline box, 5 points before "shortcuts".
const SHORTCUT_KEY: Label = MONO_11.color(SECONDARY);
const SHORTCUT_PLATE: Look = PANEL_FILL.border(Rgba::token(LINE), 1.0);
/// The offline space meter.
const METER: pxui::Progress = pxui::Progress::DEFAULT.variant(variant::METER);

/// Swift's sidebar fit for `height` points and `rows` rows: rows and Search shrink together (to 62 percent at most) so every
/// group fits above the meter; 32-point rows and a 34-point Search at full size.
pub(super) fn row_fit(height: f32, rows: usize) -> f32 {
    let fixed = 28.0 + 28.0 + 3.0 * 36.8 + 73.6 + 12.0 + (5 + rows) as f32 * 2.0;
    ((height - fixed) / (rows as f32 * 32.0 + 34.0)).clamp(0.62, 1.0)
}

/// One sidebar row slot `h` points high under the menu's 2-point spacing.
fn slot(ui: &mut Ui, key: &'static str, h: f32, build: &mut dyn FnMut(&mut Ui)) {
    ui.id(key, |ui| {
        ui.column(
            Node::column()
                .w(Size::Grow)
                .h_px(h + 2.0)
                .padding(Edges::ZERO.t(2.0)),
            |ui| build(ui),
        );
    });
}

/// The sidebar's rows: Library's three, the sources and Add, the lists and New list (Swift's `rows`).
pub(super) fn row_count(catalog: &Catalog) -> usize {
    3 + catalog.source_rows.len() + 1 + catalog.collections.len() + 1
}

/// The sidebar at Swift's fit `factor` (`row_fit`) of its 32-point rows and 34-point Search.
pub(super) fn sidebar(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    height: f32,
    factor: f32,
    intent: &mut Option<Intent>,
) {
    let row = 32.0 * factor;
    let fixed = 2.0 * PADDING
        + TOP
        + 34.0 * factor
        + 2.0
        + SEARCH_GAP
        + 3.0 * GROUP_HEIGHT
        + (LIBRARY.len() + catalog.source_rows.len() + 1) as f32 * (row + 2.0)
        + OFFLINE_HEIGHT
        + METER_HEIGHT
        + SHORTCUTS_HEIGHT;
    let route = if state.settings_visible {
        None
    } else {
        Some(state.route)
    };
    // a new route (or list count) selects its row in either group and none in the other; otherwise the rows hold, so every
    // build of the frame a click selects a row builds the same rows until its route follows
    let shown = Some((route, catalog.collections.len()));
    if state.side_route.replace(shown) != shown {
        let mut library = state.side_library.borrow_mut();
        library.rows.clear();
        library.rows.extend(LIBRARY.iter().map(|&(_, r)| {
            if route == Some(r) {
                pxui::ListState::SELECTED
            } else {
                0
            }
        }));
        let mut lists = state.side_lists.borrow_mut();
        lists.rows.clear();
        lists.rows.resize(catalog.collections.len(), 0);
        if let Some(Route::Collection(i)) = route
            && let Some(flags) = lists.rows.get_mut(i)
        {
            *flags = pxui::ListState::SELECTED;
        }
    }
    ui.column(
        Node::column()
            .w(Size::Grow)
            .h(Size::Grow)
            .padding(Edges::ZERO.l(10.0).t(PADDING).r(10.0).b(PADDING)),
        |ui| {
            let leading = ui.node(
                "leading",
                Node::column().w(Size::Grow).h_px(
                    TOP + 34.0 * factor
                        + 2.0
                        + SEARCH_GAP
                        + 3.0 * GROUP_HEIGHT
                        + (LIBRARY.len() + catalog.source_rows.len() + 1) as f32 * (row + 2.0),
                ),
            );
            ui.within(&leading, &mut |ui| {
                ui.view_of(
                    "sidebar-groups",
                    &(
                        std::sync::Arc::as_ptr(&catalog.library) as usize,
                        route,
                        factor.to_bits(),
                    ),
                    |ui| {
                        ui.retain_view_on_resize();
                        spacer(ui, "sidebar-top", TOP);
                        slot(ui, "search-slot", 34.0 * factor, &mut |ui| {
                            if ui.button("search", "Search", &SEARCH).clicked {
                                *intent = Some(Intent::ShowCommands);
                            }
                        });
                        spacer(ui, "search-gap", SEARCH_GAP);
                        ui.row(GROUP_BLOCK, |ui| {
                            ui.label("Library", GROUP);
                        });
                        let counts = [
                            &catalog.album_count,
                            &catalog.artist_count,
                            &catalog.song_count,
                        ];
                        let library = ui.selection_list(
                            "library",
                            LIBRARY.len(),
                            &|i| i as i64,
                            &mut state.side_library.borrow_mut(),
                            &SIDE.row_h(row + 2.0),
                            &mut |ui, part, cell| {
                                if let SectionPart::Row(i) = part {
                                    ui.a11y_name(&cell.handle, LIBRARY[i].0);
                                    ui.row(
                                        Node::row().w(Size::Grow).h_px(row).align(Align::Center),
                                        |ui| {
                                            ui.label(
                                                LIBRARY[i].0,
                                                SIDE_TITLE.color(if cell.selected {
                                                    INK
                                                } else {
                                                    SECONDARY
                                                }),
                                            );
                                            ui.label(counts[i], SIDE_COUNT);
                                        },
                                    );
                                }
                            },
                        );
                        ui.a11y_name(&library.handle, "Library");
                        if let Some(i) = library.selected.or(library.activated) {
                            *intent = Some(Intent::Navigate(LIBRARY[i].1));
                        }
                        ui.row(GROUP_BLOCK, |ui| {
                            ui.label("Sources", GROUP);
                        });
                        for source in &catalog.source_rows {
                            ui.id(Key::Id(source.location.source), |ui| {
                                slot(ui, "source", row, &mut |ui| {
                                    let button = ui.button_content(
                                        "row",
                                        &SOURCE_ROW.name(&source.name),
                                        |ui, _| {
                                            ui.label(
                                                &source.name,
                                                SIDE_TITLE.w(if source.reconnect {
                                                    Size::Fit
                                                } else {
                                                    Size::Grow
                                                }),
                                            );
                                            if source.reconnect {
                                                ui.node("badge-gap", Node::DEFAULT.w_px(6.0));
                                                ui.badge(
                                                    "reconnect",
                                                    "Reconnect",
                                                    "Reconnect",
                                                    &RECONNECT,
                                                );
                                                ui.node("badge-end", Node::DEFAULT.w(Size::Grow));
                                            }
                                            ui.label(&source.count, SIDE_COUNT);
                                        },
                                    );
                                    if button.clicked {
                                        *intent = Some(Intent::Files(
                                            crate::files::Request::Browse(source.location),
                                        ));
                                    }
                                })
                            });
                        }
                        slot(ui, "add-slot", row, &mut |ui| {
                            if ui
                                .button("add-music", "Add folder or files", &ADD_ROW)
                                .clicked
                            {
                                *intent = Some(Intent::ChooseAudio);
                            }
                        });
                        ui.row(GROUP_BLOCK, |ui| {
                            ui.label("Lists", GROUP);
                        });
                    },
                );
            });
            let playlist = |i: usize| match &*catalog.collections[i].context {
                PlaybackContext::Playlist { id, .. } => Some(*id),
                _ => None,
            };
            let collections = &catalog.collections;
            let mut side_lists = state.side_lists.borrow_mut();
            // only playlists take a song
            for (i, flags) in side_lists.rows.iter_mut().enumerate() {
                *flags = if playlist(i).is_some() {
                    *flags & !pxui::ListState::NO_DROP
                } else {
                    *flags | pxui::ListState::NO_DROP
                };
            }
            let viewport = ui.node(
                "list-viewport",
                Node::column().w(Size::Grow).h_px((height - fixed).max(0.0)),
            );
            ui.within(&viewport, &mut |ui| {
                let lists = ui.selection_list(
                    "lists",
                    collections.len(),
                    &|i| collections[i].key as i64,
                    &mut side_lists,
                    &SIDE_LISTS.row_h(row + 2.0).trail(row + 2.0),
                    &mut |ui, part, cell| match part {
                        SectionPart::Row(i) => {
                            let collection = &collections[i];
                            ui.a11y_name(&cell.handle, &collection.title);
                            // The virtual row's fixed slot includes the menu's leading gap.
                            ui.row(
                                Node::row()
                                    .w(Size::Grow)
                                    .h_px(row + 2.0)
                                    .padding(Edges::ZERO.t(2.0))
                                    .align(Align::Center),
                                |ui| {
                                    ui.label(
                                        &collection.title,
                                        SIDE_TITLE.color(if cell.selected {
                                            INK
                                        } else {
                                            SECONDARY
                                        }),
                                    );
                                    ui.label(&collection.count, SIDE_COUNT);
                                },
                            );
                            if let Some(list) = playlist(i)
                                && let Some((true, song)) =
                                    super::songs::song_drop(ui, &cell.handle)
                            {
                                *intent = Some(Intent::AddToList { song, list });
                            }
                        }
                        SectionPart::Trailing => slot(ui, "new-slot", row, &mut |ui| {
                            if ui.button("new-list", "New list", &ADD_ROW).clicked {
                                *intent = Some(Intent::NewList(None));
                            }
                        }),
                        _ => {}
                    },
                );
                ui.a11y_name(&lists.handle, "Lists");
                if let Some(i) = lists.selected.or(lists.activated) {
                    *intent = Some(Intent::Navigate(Route::Collection(i)));
                }
                if let (Some(i), Some(row)) = (lists.secondary, lists.row)
                    && let Some(list) = playlist(i)
                {
                    *intent = Some(Intent::ListMenu(list, ui.signal(&row).pointer));
                }
            });
            let footer = ui.node(
                "footer",
                Node::column()
                    .w(Size::Grow)
                    .h_px(OFFLINE_HEIGHT + METER_HEIGHT + SHORTCUTS_HEIGHT),
            );
            ui.within(&footer, &mut |ui| {
                ui.view_of(
                    "sidebar-footer",
                    &(
                        std::sync::Arc::as_ptr(&catalog.library) as usize,
                        catalog.offline_fraction.to_bits(),
                    ),
                    |ui| {
                        ui.retain_view_on_resize();
                        ui.row(
                            Node::row()
                                .w(Size::Grow)
                                .h_px(OFFLINE_HEIGHT)
                                .padding(Edges::ZERO.t(14.0).l(8.0).r(8.0)),
                            |ui| {
                                ui.label("Offline", TEXT_12.grow().color(SECONDARY));
                                ui.label(&catalog.offline_usage, MONO_12.color(SECONDARY));
                            },
                        );
                        ui.progress(
                            "offline",
                            "Offline space",
                            Some(catalog.offline_fraction),
                            &METER,
                        );
                        ui.row(
                            Node::row()
                                .w(Size::Grow)
                                .h_px(SHORTCUTS_HEIGHT)
                                .padding(Edges::ZERO.t(10.0).l(8.0).b(4.0)),
                            |ui| {
                                let key = ui.button_content("shortcuts", &SHORTCUTS, |ui, _| {
                                    ui.row(
                                        Node::row()
                                            .w(Size::Fit)
                                            .h_px(17.4)
                                            .padding(Edges::new(6.0, 1.0, 6.0, 1.0))
                                            .look(SHORTCUT_PLATE),
                                        |ui| {
                                            ui.label("?", SHORTCUT_KEY);
                                        },
                                    );
                                    ui.label("shortcuts", TEXT_12.color(SECONDARY));
                                });
                                if key.clicked {
                                    *intent = Some(Intent::ShowShortcuts);
                                }
                            },
                        );
                    },
                );
            });
        },
    );
}

/// The shortcuts row's key and label, 5 points apart, as tall as the label's line.
const SHORTCUTS: pxui::Button = pxui::Button::DEFAULT
    .variant(variant::ROW)
    .size(Size::Fit, Size::Px(16.8))
    .align(Justify::Start)
    .gap(5.0)
    .look(&BARE)
    .name("Keyboard shortcuts");
