//! Compact song lists (Library Songs, album/artist/list pages, Find): Swift's 56 pt rows with
//! worker-prepared "Artist · Album" lines, availability glyph, duration and a "…" menu, UIKit-style
//! two-sided swipe actions (Play next / Add to queue, or Remove inside a list), the A-Z index rail
//! and the artist page's lead-in. Every string is borrowed from the catalog or static.
use super::*;
use crate::browse::Sort;
use crate::catalog::Collection;
use pxui::SwipeActions;

const ROW: f32 = 56.0;
/// Width of the revealed strip on either side; a full swipe of `SWIPE_FULL` commits it.
const SWIPE_ACTION: f32 = 76.0;
const SWIPE_FULL: f32 = 88.0;
const RAIL_STEP: f32 = 17.0;
const RAIL_WIDTH: f32 = 28.0;

struct Cx<'a> {
    catalog: &'a Catalog,
    collection: &'a Collection,
    state: &'a ShellState,
    player: &'a PlayerView<'a>,
    width: f32,
    /// The playlist the rows belong to; its swipe removes instead of queueing.
    list: Option<i64>,
    /// Phone Select mode: checkboxes, tap toggles, no swipe or menu.
    selecting: bool,
    /// Album pages number rows by track instead of showing artwork.
    numbered: bool,
}

pub(super) fn songs(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    intent: &mut Option<Intent>,
) {
    if state.route == Route::Find {
        compact_find(ui, catalog, state, player.command_theme, width, intent);
    }
    // Swift's compact Songs is always title-sorted; the worker projects it.
    if state.route == Route::Songs && state.sort == Sort::Original && intent.is_none() {
        *intent = Some(Intent::Sort(Sort::Title));
    }
    let Some(collection) = state.collection(catalog) else {
        return;
    };
    let artist = match state.route {
        Route::Artist(index) => Some(index),
        _ => None,
    };
    let detail_page = matches!(state.route, Route::Album(_) | Route::Collection(_));
    if collection.rows.is_empty() && artist.is_none() && !detail_page {
        line(ui, "empty", "No songs here yet.", 14.0, ui.color(SECONDARY));
        return;
    }
    let cx = Cx {
        catalog,
        collection,
        state,
        player,
        width,
        list: match &*collection.context {
            lw_core::context::PlaybackContext::Playlist { id, .. } => Some(*id),
            _ => None,
        },
        selecting: state.selecting,
        numbered: matches!(state.route, Route::Album(_)),
    };
    let lead = match state.route {
        Route::Artist(index) => artist_lead_height(catalog, index, width),
        Route::Album(_) => album_lead_height(),
        Route::Collection(_) => collection_lead_height(),
        _ => 0.0,
    };
    // Swift's Find "Play results" / "Keep results offline" keys close the song list (one 56 pt row here).
    let find_keys = state.route == Route::Find;
    ui.stack("songs-stack", Node::stack().w(Size::Grow).h(Size::Grow), |ui| {
        let list = ui.section_list(
            "compact-songs",
            &SectionList {
                count: collection.rows.len() + usize::from(find_keys),
                row_height: ROW,
                leading_height: lead,
                header_height: 0.0,
                trailing_height: if cx.selecting { BAR } else { 0.0 },
                overlay_scrollbar: true,
            },
            // the find keys row after the songs has no song
            &|i| collection.row_keys.get(i).copied().unwrap_or(0),
            &mut |ui, part| match part {
                SectionPart::Leading => {
                    match state.route {
                        Route::Artist(index) => artist_lead(ui, &cx, index, intent),
                        Route::Album(_) => album_lead(ui, &cx, intent),
                        Route::Collection(_) => collection_lead(ui, &cx, intent),
                        _ => {}
                    }
                }
                SectionPart::Header => {}
                SectionPart::Row(index) if index == collection.rows.len() => {
                    find_result_keys(ui, catalog, intent)
                }
                SectionPart::Row(index) => row(ui, &cx, index, intent),
            },
        );
        refresh(ui, &list, state, intent);
        if cx.selecting {
            selection_bar(ui, &cx, intent);
        }
        if state.route == Route::Songs && state.sort == Sort::Title && state.ascending {
            rail(ui, &list, collection);
        }
    });
}

fn row(ui: &mut Ui, cx: &Cx<'_>, index: usize, intent: &mut Option<Intent>) {
    let Cx {
        catalog,
        collection,
        state,
        player,
        width,
        selecting,
        numbered,
        ..
    } = *cx;
    let row = catalog.row(collection.rows[index]);
    let cell = ui.node("swipe-row".into(), Node::stack().w(Size::Grow).h(Size::Grow).flags(Flags::CLIP));
    ui.within(&cell, &mut |ui| {
        let selected = state.selected == Some(index);
        let row_node = Node {
            padding: Edges {
                l: 16.0,
                r: 6.0,
                ..Edges::ZERO
            },
            look: Look {
                fill: if selected { ui.color(SELECTED) } else { ui.color(PAPER) },
                ..Look::NONE
            },
            ..Node::row()
                .w(Size::Grow)
                .h(Size::Grow)
                .gap(12.0)
                .align(Align::Center)
                .role(Role::ListItem)
                .flags(Flags::CLICKABLE | Flags::TAB_STOP)
        };
        let node = ui.node("song".into(), row_node);
        // Touch only: a mouse never starts a gesture, so desktop rows read rest.
        let g = ui.gesture(
            &node,
            if selecting {
                GestureOpts::default()
            } else {
                GestureOpts {
                    long_press: true,
                    swipe_actions: Some(SwipeActions {
                        leading: SWIPE_ACTION,
                        trailing: SWIPE_ACTION,
                        full: SWIPE_FULL,
                    }),
                    ..Default::default()
                }
            },
        );
        let active = g.swiping || g.swipe_open;
        match state.open_swipe.get() {
            Some((open, other)) if active && open != index => {
                ui.close_swipe(&other);
                state.open_swipe.set(Some((index, node)));
            }
            Some((open, _)) if open == index && active => {
                state.open_swipe.set(Some((index, node)));
            }
            Some((open, _)) if open == index && g.swipe == 0.0 => state.open_swipe.set(None),
            None if active => state.open_swipe.set(Some((index, node))),
            _ => {}
        }
        if g.swipe != 0.0 {
            // `swipe` is the row's own x offset; LocalWave never mirrors, so positive reveals the leading side.
            ui.set_node_layout(
                &node,
                &Node {
                    w: Size::Px(width),
                    h: Size::Px(ROW),
                    at: Some((g.swipe, 0.0)).into(),
                    ..row_node
                },
            );
        }
        if g.highlighted {
            ui.set_look(
                &node,
                Look {
                    fill: ui.color(SELECTED),
                    ..row_node.look
                },
                State::default(),
            );
        }
        ui.a11y_name(&node, row.title);
        ui.within(&node, &mut |ui| {
            if selecting {
                checkbox(ui, state.is_marked(index));
            }
            if numbered {
                let mut digits = [0u8; 24];
                let digits = stack_fmt(&mut digits, format_args!("{}", row.track_number.map_or(index as u64 + 1, |n| n.max(0) as u64)));
                ui.row("track", Node { justify: Justify::End, ..Node::row().w_px(22.0).h_px(18.2) }, |ui| {
                    ui.label(digits, MONO_13.h(18.2).color(SECONDARY));
                });
            } else {
                artwork(ui, catalog, player, Some(collection.rows[index]), 40.0);
            }
            let labels = container(ui, "labels", Axis::Column, Size::Grow, Size::Fit, ui.color(PAPER));
            ui.within(&labels, &mut |ui| {
                let ink = if player.current_id == Some(row.id) {
                    ui.tokens().accent
                } else {
                    ui.color(INK)
                };
                line(ui, "title", row.title, 15.0, ink);
                let note = if selecting {
                    row.artist
                } else if numbered {
                    if row.availability == Some(lw_core::FileAvailability::InCloud) { row.cloud_note } else { row.file_note }
                } else {
                    row.subtitle
                };
                line(ui, "artist", note, 13.0, ui.color(SECONDARY));
            });
            if selecting {
                if cx.list.is_some() {
                    reorder_grip(ui, index, row.title);
                }
                return;
            }
            if row.pinned
                || row.availability != Some(lw_core::FileAvailability::OnDevice)
                || matches!(
                    row.state,
                    lw_core::FileState::CopyPending | lw_core::FileState::Failed
                )
            {
                let holder = container(
                    ui,
                    "state",
                    Axis::Row,
                    Size::Px(if row.pinned { 14.0 } else { 10.0 }),
                    Size::Px(10.0),
                    ui.color(PAPER),
                );
                ui.within(&holder, &mut |ui| {
                    super::super::songs::row::glyph(
                        ui,
                        (row.state, row.availability, row.pinned),
                        true,
                    );
                });
            }
            ui.label(row.duration, MONO_12.h(16.8).color(SECONDARY));
            if more_button(ui, row.actions_label) {
                let rect = ui.rect(&node);
                *intent = Some(Intent::SongMenu(
                    index,
                    row.id,
                    collection.key,
                    (rect.x + rect.w - 44.0, rect.y),
                ));
            }
        });
        if selecting && cx.list.is_some() {
            super::super::songs::row::drop_target(ui, &node, index, intent);
        }
        let leading = g.swipe > 0.0;
        let (key, label, trailing) = match cx.list {
            Some(list) => (
                "remove",
                "Remove",
                Intent::RemoveFromList { song: row.id, list },
            ),
            None => ("add-to-queue", "Add to queue", Intent::AppendSong(row.id)),
        };
        let (key, label, action) = if leading {
            ("play-next", "Play next", Intent::PlayNext(row.id))
        } else {
            (key, label, trailing)
        };
        if g.swipe != 0.0 {
            ui.row(
                "swipe-actions",
                Node {
                    justify: if leading { Justify::Start } else { Justify::End },
                    z: -1,
                    ..Node::row().w(Size::Grow).h(Size::Grow)
                },
                |ui| {
                    if swipe_action(ui, key, label, g.swipe.abs()) {
                        *intent = Some(action);
                        ui.close_swipe(&node);
                    }
                },
            );
        }
        // A full swipe lasts one frame; the row settles closed on its own.
        if intent.is_none() && (g.swipe_commit_leading || g.swipe_commit_trailing) {
            *intent = Some(if g.swipe_commit_leading {
                Intent::PlayNext(row.id)
            } else {
                trailing
            });
        }
        if g.long_pressed && intent.is_none() {
            let rect = ui.rect(&node);
            *intent = Some(Intent::SongMenu(
                index,
                row.id,
                collection.key,
                (rect.x + rect.w * 0.5 - 100.0, rect.y + rect.h),
            ));
        }
        if ui.signal(&node).clicked && intent.is_none() {
            if selecting {
                *intent = Some(Intent::Select(Selection::Toggle(index)));
            } else if g.swipe_open {
                ui.close_swipe(&node);
            } else {
                *intent = Some(Intent::PlayRow(index));
            }
        }
    });
}

/// Swift's edit-mode drag handle: dragging it reorders the playlist (`MoveListRow` on drop).
fn reorder_grip(ui: &mut Ui, index: usize, title: &str) {
    use super::super::songs::row::LIST_ROW;
    let grip = ui.node("reorder".into(), Node::column().w_px(44.0).h_px(44.0).role(Role::Button).flags(Flags::CLICKABLE | Flags::TAB_STOP));
    ui.a11y_name(&grip, "Reorder track");
    // The finger takes the handle at once as a held press, so the list neither scrolls nor refreshes.
    ui.gesture(&grip, GestureOpts { touch_drag: true, ..Default::default() });
    ui.draw(&grip, move |paint, rect| {
        for y in [-5.0, 0.0, 5.0] {
            paint.fill(rect.x + 13.0, rect.y + rect.h * 0.5 + y - 1.0, 18.0, 2.0, paint.color(SECONDARY));
        }
    });
    let signal = ui.signal(&grip);
    if signal.held || signal.clicked {
        let mut digits = [0u8; 24];
        let id = stack_fmt(&mut digits, format_args!("{}", index)).as_bytes();
        let mut kind = [0u8; 48];
        kind[..LIST_ROW.len()].copy_from_slice(LIST_ROW.as_bytes());
        let len = LIST_ROW.len() + id.len();
        kind[LIST_ROW.len()..len].copy_from_slice(id);
        ui.carry(
            if signal.clicked { None } else { Some(&grip) },
            std::str::from_utf8(&kind[..len]).expect("list drag kind is ASCII"),
            title,
        );
    }
}

fn swipe_action(ui: &mut Ui, key: &'static str, label: &'static str, revealed: f32) -> bool {
    let armed = revealed >= SWIPE_FULL;
    let (fill, ink) = if armed {
        (ui.color(INK), ui.color(PAPER))
    } else {
        (ui.color(PANEL), ui.color(SECONDARY))
    };
    let node = ui.node(
        key.into(),
        Node {
            look: Look {
                fill,
                text: ink,
                ..Look::NONE
            },
            ..Node::column()
                .w_px(revealed.max(SWIPE_ACTION))
                .h(Size::Grow)
                .center()
                .role(Role::Button)
                .flags(Flags::CLICKABLE | Flags::TAB_STOP)
        },
    );
    ui.within(&node, &mut |ui| {
        ui.label(label, TEXT_13.h(18.2).color(if armed { PAPER } else { SECONDARY }));
    });
    ui.a11y_name(&node, label);
    ui.signal(&node).clicked
}

/// Swift's LWMenuButton: a bare "…" in a 44 pt target.
fn more_button(ui: &mut Ui, label: &str) -> bool {
    let node = ui.node(
        "actions".into(),
        Node::column().w_px(44.0).h_px(44.0).role(Role::Button).flags(Flags::CLICKABLE | Flags::TAB_STOP),
    );
    ui.a11y_name(&node, label);
    let color = ui.color(SECONDARY);
    ui.draw(&node, move |paint, rect| {
        kit::paint_icon(
            paint,
            Icon::More,
            rect.x + (rect.w - 20.0) / 2.0,
            rect.y + (rect.h - 20.0) / 2.0,
            color,
        );
    });
    ui.signal(&node).clicked
}

/// `refreshing` holds from release until `end_refresh`, so its first frame starts the load; it can
/// read once more after the end, which `refresh_ending` keeps from starting another.
fn refresh(ui: &mut Ui, list: &Handle, state: &ShellState, intent: &mut Option<Intent>) {
    let pull = ui.gesture(
        list,
        GestureOpts {
            refresh: Some(60.0),
            ..Default::default()
        },
    );
    if !pull.refreshing {
        state.refresh_ending.set(false);
        state.refresh_wait.set(None);
    } else if let Some((epoch, since)) = state.refresh_wait.get() {
        // Ends on the next catalog delivery; the deadline keeps a refresh that changed nothing from holding.
        if epoch != state.refresh_epoch || ui.now() - since > 5.0 {
            ui.end_refresh(list);
            state.refresh_wait.set(None);
            state.refresh_ending.set(true);
        } else {
            ui.repaint_after((5.0 - (ui.now() - since)).max(0.0));
        }
    } else if !state.refresh_ending.get() {
        state
            .refresh_wait
            .set(Some((state.refresh_epoch, ui.now())));
        if intent.is_none() {
            *intent = Some(Intent::RefreshLibrary);
        }
    }
}

/// Scroll so `target` starts the viewport (reveal only scrolls the minimum, so aim at the row that
/// ends it when moving down).
fn jump(ui: &mut Ui, list: &Handle, target: usize, count: usize) {
    let visible = ((ui.rect(list).h / ROW) as usize).max(1);
    let top = (ui.scroll_position(list) / ROW) as usize;
    ui.reveal(
        list,
        if target < top {
            target
        } else {
            (target + visible - 1).min(count - 1)
        },
    );
}

/// Swift's trailing index: tap or drag over a letter jumps to its first song.
fn rail(ui: &mut Ui, list: &Handle, collection: &Collection) {
    ui.row(
        "index-holder",
        Node {
            justify: Justify::End,
            ..Node::row().w(Size::Grow).h(Size::Grow).align(Align::Center)
        },
        |ui| {
            let rail = ui.node(
                "index".into(),
                Node::column().w_px(RAIL_WIDTH).h_px(collection.sections.len() as f32 * RAIL_STEP).role(Role::Group),
            );
            ui.a11y_name(&rail, "Section index");
            let top = ui.rect(&rail).y;
            let mut dragging = false;
            ui.within(&rail, &mut |ui| {
                for (letter, first) in collection.sections.iter().copied() {
                    let node = ui.node(
                        letter.into(),
                        Node {
                            justify: Justify::End,
                            padding: Edges {
                                r: 6.0,
                                ..Edges::ZERO
                            },
                            ..Node::row()
                                .w_px(RAIL_WIDTH)
                                .h_px(RAIL_STEP)
                                .align(Align::Center)
                                .role(Role::Button)
                                .flags(Flags::CLICKABLE)
                        },
                    );
                    ui.within(&node, &mut |ui| {
                        ui.label(letter, TEXT_11.h(RAIL_STEP).color(SECONDARY));
                    });
                    ui.a11y_name(&node, letter);
                    // A finger on the rail is a held press-and-drag, not a scroll or a row swipe underneath.
                    ui.gesture(&node, GestureOpts { touch_drag: true, ..Default::default() });
                    let signal = ui.signal(&node);
                    if signal.held {
                        dragging = true;
                        // The press holds while the pointer or finger drags over the index; the letter comes from its y.
                        let under = (((signal.pointer.1 - top) / RAIL_STEP).max(0.0) as usize)
                            .min(collection.sections.len() - 1);
                        if ui.state(&rail, -1) != under as i64 {
                            ui.set_state(&rail, under as i64);
                            jump(ui, list, collection.sections[under].1, collection.rows.len());
                        }
                    } else if signal.clicked {
                        jump(ui, list, first, collection.rows.len());
                    }
                }
            });
            if !dragging && ui.state(&rail, -1) != -1 {
                ui.set_state(&rail, -1);
            }
        },
    );
}

const BACK_ROW: f32 = 54.0;
const TITLE_BLOCK: f32 = 33.6 + 4.0 + 18.2;
const PLAY_BAR: f32 = 52.0;
const SECTION_LABEL: f32 = 20.0 + 18.2 + 8.0;
const CARD_GAP: f32 = 12.0;

fn album_cell(width: f32) -> f32 {
    ((width - 32.0 - CARD_GAP) / 2.0).floor()
}
fn album_card(width: f32) -> f32 {
    album_cell(width) + 8.0 + 17.5 + 18.2
}
fn artist_lead_height(catalog: &Catalog, artist: usize, width: f32) -> f32 {
    let albums = catalog.artist_albums.get(artist).map_or(0, Vec::len);
    let rows = albums.div_ceil(2);
    BACK_ROW
        + TITLE_BLOCK
        + 8.0
        + PLAY_BAR
        + 16.0
        + rows as f32 * album_card(width)
        + rows.saturating_sub(1) as f32 * 20.0
        + 20.0
        + SECTION_LABEL
}

/// Swift's compact artistDetail: back, name and summary, Play/Shuffle keys, album cards, "Songs".
fn artist_lead(ui: &mut Ui, cx: &Cx<'_>, artist: usize, intent: &mut Option<Intent>) {
    let Cx {
        catalog,
        collection,
        state,
        player,
        width,
        ..
    } = *cx;
    let albums = catalog
        .artist_albums
        .get(artist)
        .map_or(&[][..], Vec::as_slice);
    let revision = crate::hash((
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        width.to_bits(),
        player.artwork.map_or(0, crate::artwork::Images::revision),
        select_revision(state),
    ));
    ui.view(
        "artist-lead",
        revision,
        Size::Grow,
        Size::Px(artist_lead_height(catalog, artist, width)),
        &mut |ui| {
            let column = container(ui, "lead", Axis::Column, Size::Grow, Size::Grow, ui.color(PAPER));
            ui.within(&column, &mut |ui| {
                nav_row(ui, cx, "Library", Route::Artists, intent);
                let name = if collection.title.is_empty() { "Unknown artist" } else { &collection.title };
                titles(ui, name, &collection.summary);
                spacer(ui, "keys-gap", 8.0);
                play_keys(ui, None, intent);
                spacer(ui, "albums-gap", 16.0);
                let cell = album_cell(width);
                for (line, pair) in albums.chunks(2).enumerate() {
                    if line > 0 {
                        spacer(ui, "album-row-gap", 20.0);
                    }
                    let cards = ui.node(
                        Key::Id(line as i64),
                        Node::row().w(Size::Grow).h_px(album_card(width)).gap(CARD_GAP).pad_x(16.0),
                    );
                    ui.within(&cards, &mut |ui| {
                        for index in pair.iter().copied() {
                            let group = &catalog.albums[index];
                            let card = ui.node(
                                Key::Id(index as i64),
                                Node::column()
                                    .w_px(cell)
                                    .h(Size::Grow)
                                    .role(Role::Button)
                                    .flags(Flags::CLICKABLE | Flags::TAB_STOP),
                            );
                            ui.a11y_name(&card, &group.title);
                            ui.within(&card, &mut |ui| {
                                let art = container(
                                    ui,
                                    "artwork",
                                    Axis::Stack,
                                    Size::Px(cell),
                                    Size::Px(cell),
                                    ui.color(PANEL),
                                );
                                paint_artwork(
                                    ui,
                                    &art,
                                    group.artwork_row.map(|row| catalog.row(row)),
                                    cell,
                                    player.artwork,
                                );
                                spacer(ui, "art-gap", 8.0);
                                detail::line(
                                    ui,
                                    "name",
                                    &group.title,
                                    (14.0, 17.5, 500, 0.0),
                                    ui.color(INK),
                                    Flags::NONE,
                                );
                                let year = group.rows.first().and_then(|row| catalog.row(*row).release_year);
                                let mut digits = [0u8; 24];
                                let year = stack_fmt(&mut digits, format_args!("{}", year.unwrap_or(0).max(0)));
                                detail::line(
                                    ui,
                                    "year",
                                    if year == "0" { "" } else { year },
                                    (13.0, 18.2, 400, 0.0),
                                    ui.color(SECONDARY),
                                    Flags::NONE,
                                );
                            });
                            if ui.signal(&card).clicked {
                                *intent = Some(Intent::Navigate(Route::Album(index)));
                            }
                        }
                    });
                }
                spacer(ui, "songs-gap", 20.0);
                ui.column(
                    "songs-label",
                    Node {
                        padding: Edges {
                            l: 16.0,
                            t: 20.0,
                            b: 8.0,
                            ..Edges::ZERO
                        },
                        ..Node::column().w(Size::Grow).h_px(SECTION_LABEL)
                    },
                    |ui| {
                        detail::line(ui, "text", "Songs", (13.0, 18.2, 500, 0.0), ui.color(SECONDARY), Flags::NONE);
                    },
                );
            });
        },
    );
}

/// Swift's detail back row: chevron and the parent's name, 44 pt tall.
fn back_row(
    ui: &mut Ui,
    label: &'static str,
    parent: Route,
    intent: &mut Option<Intent>,
    extras: &mut dyn FnMut(&mut Ui, &mut Option<Intent>),
) {
    ui.row(
        "back-row",
        Node { padding: Edges { t: 10.0, l: 6.0, ..Edges::ZERO }, ..Node::row().w(Size::Grow).h_px(BACK_ROW).align(Align::Center) },
        |ui| {
            let button = ui.node(
                "back".into(),
                Node::row().w_px(96.0).h_px(44.0).align(Align::Center).role(Role::Button).flags(Flags::CLICKABLE | Flags::TAB_STOP),
            );
            ui.a11y_name(&button, "Back");
            ui.within(&button, &mut |ui| {
                let glyph = ui.node("icon".into(), Node::column().w_px(20.0).h_px(20.0));
                let color = ui.color(SECONDARY);
                ui.draw(&glyph, move |paint, rect| kit::paint_icon(paint, Icon::Back, rect.x, rect.y, color));
                ui.label(label, TEXT_15.h(21.0).color(SECONDARY));
            });
            if ui.signal(&button).clicked {
                *intent = Some(Intent::Navigate(parent));
            }
            extras(ui, intent);
        },
    );
}

/// Name (32 pt) over a 13 pt summary.
fn titles(ui: &mut Ui, title: &str, summary: &str) {
    ui.column("titles", Node::column().w(Size::Grow).h_px(TITLE_BLOCK).pad_x(16.0), |ui| {
        detail::line(ui, "name", title, (32.0, 33.6, 500, -0.035), ui.color(INK), Flags::NONE);
        spacer(ui, "gap", 4.0);
        detail::line(ui, "summary", summary, (13.0, 18.2, 400, 0.0), ui.color(SECONDARY), Flags::NONE);
    });
}

fn find_result_keys(ui: &mut Ui, catalog: &Catalog, intent: &mut Option<Intent>) {
    let kept = catalog.view.as_ref().and_then(|view| view.find.as_ref()).is_some_and(|find| find.pinned);
    ui.row("find-keys", Node { padding: Edges { l: 16.0, r: 16.0, t: 2.0, ..Edges::ZERO }, ..Node::row().w(Size::Grow).h_px(ROW).gap(8.0) }, |ui| {
        if key(ui, "play-results", "Play results", Icon::Play, false) {
            *intent = Some(Intent::PlayRow(0));
        }
        let (label, icon) = if kept { ("Kept offline", Icon::Check) } else { ("Keep results offline", Icon::Down) };
        if key(ui, "keep-results-offline", label, icon, false) {
            *intent = Some(Intent::TogglePins);
        }
    });
}

/// Play and Shuffle keys.
fn play_keys(ui: &mut Ui, offline: Option<bool>, intent: &mut Option<Intent>) {
    ui.row("keys", Node::row().w(Size::Grow).h_px(PLAY_BAR).gap(8.0).pad_x(16.0), |ui| {
        if key(ui, "play", "Play", Icon::Play, true) {
            *intent = Some(Intent::PlayRow(0));
        }
        if key(ui, "shuffle", "Shuffle", Icon::Shuffle, false) {
            *intent = Some(Intent::ShuffleCollection);
        }
        if let Some(kept) = offline
            && offline_key(ui, kept)
        {
            *intent = Some(Intent::Page(crate::PageAction::KeepOffline));
        }
    });
}

/// Swift's third play key on albums only (its list page has Play and Shuffle).
fn offline_state(cx: &Cx<'_>) -> Option<bool> {
    matches!(&*cx.collection.context, lw_core::context::PlaybackContext::Album { .. })
        .then(|| !cx.collection.rows.is_empty() && cx.collection.rows.iter().all(|&row| cx.catalog.row(row).pinned))
}
pub(in crate::shell) fn offline_key(ui: &mut Ui, kept: bool) -> bool {
    let label = if kept { "Kept offline" } else { "Keep offline" };
    let node = ui.node(
        "keep-offline".into(),
        Node {
            look: Look { fill: ui.color(PANEL), ..Look::NONE },
            ..Node::row()
                .w(Size::Grow)
                .h_px(PLAY_BAR)
                .gap(8.0)
                .center()
                .role(Role::Button)
                .flags(Flags::CLICKABLE | Flags::TAB_STOP)
        },
    );
    ui.within(&node, &mut |ui| {
        let glyph = ui.node("icon".into(), Node::column().w_px(20.0).h_px(20.0));
        let color = ui.color(INK);
        ui.draw(&glyph, move |paint, rect| {
            crate::icons::offline(paint, pxui::geom::Rect { x: rect.x, y: rect.y, w: 20.0, h: 20.0 }, kept, color)
        });
        ui.label(label, TEXT_15.h(21.0).color(INK));
    });
    ui.a11y_name(&node, label);
    ui.signal(&node).clicked
}

/// Select-mode part of a lead's cache key: the marks only matter while selecting.
fn select_revision(state: &ShellState) -> (bool, u64) {
    (state.selecting, if state.selecting { state.selection_revision } else { 0 })
}

const BAR: f32 = 64.0;

/// Back row, or Swift's selection header (Select all, "N selected", Done) in Select mode.
/// Albums and lists add the page "…" menu; artists only Select.
fn nav_row(ui: &mut Ui, cx: &Cx<'_>, label: &'static str, parent: Route, intent: &mut Option<Intent>) {
    let Cx { collection, state, .. } = *cx;
    let playlist = cx.list.is_some();
    if !state.selecting {
        back_row(ui, label, parent, intent, &mut |ui, intent| {
            container(ui, "nav-space", Axis::Row, Size::Grow, Size::Px(1.0), ui.color(PAPER));
            // Playlists edit (select and remove); the others select.
            if !(playlist && collection.rows.is_empty())
                && word(ui, "select", if playlist { "Edit" } else { "Select" }, false)
            {
                *intent = Some(Intent::SelectMode(true));
            }
            if !matches!(state.route, Route::Artist(_))
                && more_button(ui, if playlist { "List actions" } else { "Album actions" })
            {
                *intent = Some(Intent::PageMenu((cx.width - 8.0, BACK_ROW)));
            }
        });
        return;
    }
    selection_header(ui, state.marked.len(), collection.rows.len(), collection.rows.len(), intent);
}

/// Swift's WaveMobileSelectionHeader: Select all, "N selected", Done. `selectable` rows count toward "all";
/// "Select all" marks `rows` positions.
pub(in crate::shell) fn selection_header(ui: &mut Ui, selected: usize, selectable: usize, rows: usize, intent: &mut Option<Intent>) {
    ui.row("selection-header", Node { padding: Edges { t: 10.0, l: 12.0, r: 12.0, ..Edges::ZERO }, ..Node::row().w(Size::Grow).h_px(BACK_ROW).align(Align::Center) }, |ui| {
        let all = selected > 0 && selected == selectable;
        if word(ui, "select-all", if all { "Deselect all" } else { "Select all" }, false) {
            *intent = Some(Intent::Select(if all { Selection::Clear } else { Selection::All(rows) }));
        }
        container(ui, "left-space", Axis::Row, Size::Grow, Size::Px(1.0), ui.color(PAPER));
        let mut buffer = [0u8; 40];
        ui.label(selected_label(&mut buffer, selected), TEXT_15.h(21.0).color(INK));
        container(ui, "right-space", Axis::Row, Size::Grow, Size::Px(1.0), ui.color(PAPER));
        if word(ui, "done", "Done", true) {
            *intent = Some(Intent::SelectMode(false));
        }
    });
}

/// "N selected" without a heap allocation.
fn selected_label(buffer: &mut [u8; 40], count: usize) -> &str {
    use std::io::Write;
    let mut rest = &mut buffer[..];
    let _ = write!(rest, "{count} selected");
    let used = 40 - rest.len();
    std::str::from_utf8(&buffer[..used]).unwrap_or("")
}

/// A 44 pt text button (Swift's Select / Done at 16 pt).
pub(in crate::shell) fn word(ui: &mut Ui, key: &'static str, label: &'static str, _medium: bool) -> bool {
    let node = ui.node(key.into(), Node::row().w(Size::Fit).h_px(44.0).pad_x(4.0).align(Align::Center).role(Role::Button).flags(Flags::CLICKABLE | Flags::TAB_STOP));
    ui.within(&node, &mut |ui| {
        ui.label(label, TEXT_16.h(22.4).color(INK));
    });
    ui.a11y_name(&node, label);
    ui.signal(&node).clicked
}

/// Swift's 20 pt selection square: ink with a check when marked, an outline otherwise.
pub(in crate::shell) fn checkbox(ui: &mut Ui, marked: bool) {
    let node = ui.node("checkbox".into(), Node::column().w_px(20.0).h_px(20.0));
    let (ink, outline, paper) = (ui.color(INK), ui.color(SECONDARY), ui.color(PAPER));
    ui.draw(&node, move |paint, rect| {
        if marked {
            paint.fill(rect.x, rect.y, 20.0, 20.0, ink);
            crate::icons::offline(paint, pxui::geom::Rect { x: rect.x + 3.0, y: rect.y + 3.0, w: 14.0, h: 14.0 }, true, paper);
        } else {
            paint.stroke_w(rect.x, rect.y, 20.0, 20.0, 1.5, outline);
        }
    });
}

/// Swift's selection deck: Play next, Add to list, Offline, then Queue (Remove inside a list). Returns the pressed
/// key ("play-next", "add-to-list", "offline", "queue" or "remove") and the key's rectangle.
pub(in crate::shell) fn deck(ui: &mut Ui, enabled: bool, remove: bool) -> Option<(&'static str, pxui::geom::Rect)> {
    let mut pressed = None;
    ui.column("selection-holder", Node { justify: Justify::End, ..Node::column().w(Size::Grow).h(Size::Grow) }, |ui| {
        let deck = ui.node("selection-bar".into(), Node { padding: Edges { l: 2.0, r: 2.0, t: 2.0, ..Edges::ZERO }, look: Look { fill: ui.color(PANEL), ..Look::NONE }, ..Node::row().w(Size::Grow).h_px(BAR).gap(2.0) });
        ui.set_role(&deck, Role::Group, 0);
        ui.a11y_name(&deck, "Selection actions");
        ui.within(&deck, &mut |ui| {
            for (key, label, glyph) in [
                ("play-next", "Play next", Icon::Next),
                ("add-to-list", "Add to list", Icon::Clear),
                ("offline", "Offline", Icon::Down),
                (if remove { "remove" } else { "queue" }, if remove { "Remove" } else { "Queue" }, if remove { Icon::Clear } else { Icon::Queue }),
            ] {
                let tone = if !enabled { TERTIARY } else if key == "remove" { DANGER } else { INK };
                let ink = ui.color(tone);
                let node = ui.node(
                    key.into(),
                    Node {
                        state: State { disabled: !enabled, ..State::default() },
                        look: Look { fill: ui.color(KEY), ..Look::NONE },
                        ..Node::column()
                            .w(Size::Grow)
                            .h(Size::Grow)
                            .gap(4.0)
                            .center()
                            .role(Role::Button)
                            .flags(if enabled { Flags::CLICKABLE | Flags::TAB_STOP } else { Flags::NONE })
                    },
                );
                ui.within(&node, &mut |ui| {
                    let icon = ui.node("icon".into(), Node::column().w_px(20.0).h_px(20.0));
                    ui.draw(&icon, move |paint, rect| match key {
                        "add-to-list" => {
                            paint.fill(rect.x + 3.0, rect.y + 9.25, 14.0, 1.5, ink);
                            paint.fill(rect.x + 9.25, rect.y + 3.0, 1.5, 14.0, ink);
                        }
                        "offline" => crate::icons::offline(paint, pxui::geom::Rect { x: rect.x, y: rect.y, w: 20.0, h: 20.0 }, false, ink),
                        _ => kit::paint_icon(paint, glyph, rect.x, rect.y, ink),
                    });
                    ui.label(label, TEXT_12.h(16.8).color(tone));
                });
                ui.a11y_name(&node, label);
                if enabled && ui.signal(&node).clicked {
                    pressed = Some((key, ui.rect(&node)));
                }
            }
        });
    });
    pressed
}

fn selection_bar(ui: &mut Ui, cx: &Cx<'_>, intent: &mut Option<Intent>) {
    let Cx { collection, state, .. } = *cx;
    let first = state.marked.first().and_then(|&index| collection.ids.get(index).map(|&id| (index, id)));
    let remove = cx.list;
    if let Some((key, rect)) = deck(ui, first.is_some(), remove.is_some()) {
        let (index, id) = first.unwrap_or_default();
        *intent = Some(match key {
            "play-next" => Intent::PlayNext(id),
            "add-to-list" => Intent::SelectionLists(index, id, collection.key, (rect.x, rect.y - 36.0 * 8.0)),
            "offline" => Intent::KeepOffline(id),
            "remove" => Intent::RemoveFromList { song: id, list: remove.unwrap_or_default() },
            _ => Intent::AppendSong(id),
        });
    }
}

/// Compact album page lead-in: back, 150 pt artwork beside title, artist and summary, Play/Shuffle.
const ALBUM_ART: f32 = 150.0;
fn album_lead_height() -> f32 {
    BACK_ROW + ALBUM_ART + 16.0 + PLAY_BAR + 16.0
}
fn album_lead(ui: &mut Ui, cx: &Cx<'_>, intent: &mut Option<Intent>) {
    let Cx { catalog, collection, state, player, width, .. } = *cx;
    let revision = crate::hash((
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        width.to_bits(),
        player.artwork.map_or(0, crate::artwork::Images::revision),
        select_revision(state),
    ));
    ui.view("album-lead", revision, Size::Grow, Size::Px(album_lead_height()), &mut |ui| {
        let column = container(ui, "lead", Axis::Column, Size::Grow, Size::Grow, ui.color(PAPER));
        ui.within(&column, &mut |ui| {
            nav_row(ui, cx, "Library", Route::Albums, intent);
            ui.row("album-top", Node::row().w(Size::Grow).h_px(ALBUM_ART).gap(16.0).pad_x(16.0), |ui| {
                let art = container(ui, "artwork", Axis::Stack, Size::Px(ALBUM_ART), Size::Px(ALBUM_ART), ui.color(PANEL));
                paint_artwork(ui, &art, collection.artwork_row.map(|row| catalog.row(row)), ALBUM_ART, player.artwork);
                ui.a11y_name(&art, "Album artwork");
                ui.column("meta", Node { justify: Justify::End, ..Node::column().w(Size::Grow).h(Size::Grow).gap(6.0) }, |ui| {
                    ui.label(&collection.title, PARAGRAPH_24.color(INK));
                    if let lw_core::context::PlaybackContext::Album { artist, .. } = collection.context.as_ref()
                        && !artist.is_empty()
                    {
                        let link = detail::line(ui, "artist", artist, (17.0, 23.8, 400, 0.0), ui.color(SECONDARY), Flags::CLICKABLE | Flags::TAB_STOP);
                        if ui.signal(&link).clicked
                            && let Some(index) = catalog.library.artists.iter().position(|a| a.title == *artist)
                        {
                            *intent = Some(Intent::Navigate(Route::Artist(index)));
                        }
                    }
                    ui.label(&collection.summary, MONO_13.w(Size::Grow).h(18.2).color(SECONDARY));
                });
            });
            spacer(ui, "keys-gap", 16.0);
            play_keys(ui, offline_state(cx), intent);
        });
    });
}

/// Compact list page lead-in (playlists, Favorites and the other automatic lists).
fn collection_lead_height() -> f32 {
    BACK_ROW + TITLE_BLOCK + 8.0 + PLAY_BAR + 16.0
}
fn collection_lead(ui: &mut Ui, cx: &Cx<'_>, intent: &mut Option<Intent>) {
    let Cx { catalog, collection, state, .. } = *cx;
    let revision = crate::hash((
        state.revision,
        collection.key,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        select_revision(state),
    ));
    ui.view("collection-lead", revision, Size::Grow, Size::Px(collection_lead_height()), &mut |ui| {
        let column = container(ui, "lead", Axis::Column, Size::Grow, Size::Grow, ui.color(PAPER));
        ui.within(&column, &mut |ui| {
            nav_row(ui, cx, "Lists", Route::Lists, intent);
            titles(ui, &collection.title, &collection.summary);
            spacer(ui, "keys-gap", 8.0);
            play_keys(ui, offline_state(cx), intent);
            spacer(ui, "list-gap", 16.0);
        });
    });
}

/// Swift's playKeyButtons: a 52 pt key, ink for Play.
pub(in crate::shell) fn key(ui: &mut Ui, id: &'static str, label: &'static str, icon: Icon, primary: bool) -> bool {
    let node = ui.node(
        id.into(),
        Node {
            look: Look {
                fill: if primary { ui.color(INK) } else { ui.color(PANEL) },
                ..Look::NONE
            },
            ..Node::row()
                .w(Size::Grow)
                .h_px(PLAY_BAR)
                .gap(8.0)
                .center()
                .role(Role::Button)
                .flags(Flags::CLICKABLE | Flags::TAB_STOP)
        },
    );
    let tone = if primary { PAPER } else { INK };
    let color = ui.color(tone);
    ui.within(&node, &mut |ui| {
        let glyph = ui.node("icon".into(), Node::column().w_px(20.0).h_px(20.0));
        ui.draw(&glyph, move |paint, rect| {
            kit::paint_icon(paint, icon, rect.x, rect.y, color)
        });
        ui.label(label, TEXT_15.h(21.0).color(tone));
    });
    ui.a11y_name(&node, label);
    ui.signal(&node).clicked
}
