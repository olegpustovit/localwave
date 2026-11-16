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
    p: Palette,
}

pub(super) fn songs(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    if state.route == Route::Find {
        compact_find(ui, catalog, state, player.command_theme, width, p, intent);
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
        line(ui, "empty", "No songs here yet.", 14.0, p.secondary);
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
        p,
    };
    let lead = match state.route {
        Route::Artist(index) => artist_lead_height(catalog, index, width),
        Route::Album(_) => album_lead_height(),
        Route::Collection(_) => collection_lead_height(),
        _ => 0.0,
    };
    // Swift's Find "Play results" / "Keep results offline" keys close the song list (one 56 pt row here).
    let find_keys = state.route == Route::Find;
    let stack = ui.node(
        "songs-stack",
        Node {
            axis: Axis::Stack,
            w: Size::Grow,
            h: Size::Grow,
            ..Node::DEFAULT
        },
    );
    ui.within(&stack, &mut |ui| {
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
                    find_result_keys(ui, catalog, p, intent)
                }
                SectionPart::Row(index) => row(ui, &cx, index, intent),
            },
        );
        refresh(ui, &list, state, intent);
        if cx.selecting {
            selection_bar(ui, &cx, intent);
        }
        if state.route == Route::Songs && state.sort == Sort::Title && state.ascending {
            rail(ui, &list, collection, p);
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
        p,
        ..
    } = *cx;
    let row = catalog.row(collection.rows[index]);
    let cell = ui.node(
        "swipe-row",
        Node {
            axis: Axis::Stack,
            w: Size::Grow,
            h: Size::Grow,
            flags: CLIP,
            ..Node::DEFAULT
        },
    );
    ui.within(&cell, &mut |ui| {
        let selected = state.selected == Some(index);
        let row_node = Node {
            axis: Axis::Row,
            align: Align::Center,
            gap: 12.0,
            role: Role::ListItem,
            flags: CLICKABLE | TAB_STOP,
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges {
                l: 16.0,
                r: 6.0,
                ..Edges::ZERO
            },
            look: Look {
                fill: if selected { p.selected } else { p.paper },
                ..Look::NONE
            },
            ..Node::DEFAULT
        };
        let node = ui.node("song", row_node);
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
                    fill: p.selected,
                    ..row_node.look
                },
                State::default(),
            );
        }
        ui.a11y_name(&node, row.title);
        ui.within(&node, &mut |ui| {
            if selecting {
                checkbox(ui, state.is_marked(index), p);
            }
            if numbered {
                let digits = ClockLabel::integer(row.track_number.map_or(index as u64 + 1, |n| n.max(0) as u64));
                let cell = ui.node("track", Node { axis: Axis::Row, justify: Justify::End, w: Size::Px(22.0), h: Size::Px(18.2), ..Node::DEFAULT });
                ui.within(&cell, &mut |ui| {
                    text(ui, "number", std::str::from_utf8(digits.bytes()).unwrap_or(""), 13, p.secondary, (Size::Fit, 18.2), true);
                });
            } else {
                artwork(ui, catalog, player, Some(collection.rows[index]), 40.0, p);
            }
            let labels = container(ui, "labels", Axis::Column, Size::Grow, Size::Fit, p.paper);
            ui.within(&labels, &mut |ui| {
                let ink = if player.current_id == Some(row.id) {
                    p.signal
                } else {
                    p.ink
                };
                line(ui, "title", row.title, 15.0, ink);
                let note = if selecting {
                    row.artist
                } else if numbered {
                    if row.availability == Some(lw_core::FileAvailability::InCloud) { row.cloud_note } else { row.file_note }
                } else {
                    row.subtitle
                };
                line(ui, "artist", note, 13.0, p.secondary);
            });
            if selecting {
                if cx.list.is_some() {
                    reorder_grip(ui, index, row.title, p);
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
                    p.paper,
                );
                ui.within(&holder, &mut |ui| {
                    super::super::songs::row::glyph(
                        ui,
                        (row.state, row.availability, row.pinned),
                        true,
                        p,
                    );
                });
            }
            text(ui, "duration", row.duration, 12, p.secondary, (Size::Fit, 16.8), true);
            if more_button(ui, row.actions_label, p) {
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
            super::super::songs::row::drop_target(ui, p, &node, index, intent);
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
            let strip = ui.node(
                "swipe-actions",
                Node {
                    axis: Axis::Row,
                    justify: if leading { Justify::Start } else { Justify::End },
                    z: -1,
                    w: Size::Grow,
                    h: Size::Grow,
                    ..Node::DEFAULT
                },
            );
            ui.within(&strip, &mut |ui| {
                if swipe_action(ui, key, label, g.swipe.abs(), p) {
                    *intent = Some(action);
                    ui.close_swipe(&node);
                }
            });
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
        if ui.signal_state(&node).clicked && intent.is_none() {
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
fn reorder_grip(ui: &mut Ui, index: usize, title: &str, p: Palette) {
    use super::super::songs::row::LIST_ROW;
    let grip = ui.node("reorder", Node { flags: CLICKABLE | TAB_STOP, role: Role::Button, w: Size::Px(44.0), h: Size::Px(44.0), ..Node::DEFAULT });
    ui.a11y_name(&grip, "Reorder track");
    // The finger takes the handle at once as a held press, so the list neither scrolls nor refreshes.
    ui.gesture(&grip, GestureOpts { touch_drag: true, ..Default::default() });
    ui.draw(&grip, move |paint, rect| {
        for y in [-5.0, 0.0, 5.0] {
            paint.fill(rect.x + 13.0, rect.y + rect.h * 0.5 + y - 1.0, 18.0, 2.0, p.secondary);
        }
    });
    let signal = ui.signal_state(&grip);
    if signal.held || signal.clicked {
        let id = ClockLabel::integer(index as u64);
        let mut kind = [0u8; 48];
        kind[..LIST_ROW.len()].copy_from_slice(LIST_ROW.as_bytes());
        let len = LIST_ROW.len() + id.bytes().len();
        kind[LIST_ROW.len()..len].copy_from_slice(id.bytes());
        ui.carry(
            if signal.clicked { None } else { Some(&grip) },
            std::str::from_utf8(&kind[..len]).expect("list drag kind is ASCII"),
            title,
        );
    }
}

fn swipe_action(ui: &mut Ui, key: &str, label: &'static str, revealed: f32, p: Palette) -> bool {
    let armed = revealed >= SWIPE_FULL;
    let (fill, ink) = if armed {
        (p.ink, p.paper)
    } else {
        (p.panel, p.secondary)
    };
    let node = ui.node(
        key,
        Node {
            flags: CLICKABLE | TAB_STOP,
            role: Role::Button,
            w: Size::Px(revealed.max(SWIPE_ACTION)),
            h: Size::Grow,
            align: Align::Center,
            justify: Justify::Center,
            look: Look {
                fill,
                text: ink,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&node, &mut |ui| {
        text(ui, "label", label, 13, ink, (Size::Fit, 18.2), false);
    });
    ui.a11y_name(&node, label);
    ui.signal_state(&node).clicked
}

/// Swift's LWMenuButton: a bare "…" in a 44 pt target.
fn more_button(ui: &mut Ui, label: &str, p: Palette) -> bool {
    let node = ui.node(
        "actions",
        Node {
            flags: CLICKABLE | TAB_STOP,
            role: Role::Button,
            w: Size::Px(44.0),
            h: Size::Px(44.0),
            ..Node::DEFAULT
        },
    );
    ui.a11y_name(&node, label);
    let color = p.secondary;
    ui.draw(&node, move |paint, rect| {
        kit::paint_icon(
            paint,
            Icon::More,
            rect.x + (rect.w - 20.0) / 2.0,
            rect.y + (rect.h - 20.0) / 2.0,
            color,
        );
    });
    ui.signal_state(&node).clicked
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
fn rail(ui: &mut Ui, list: &Handle, collection: &Collection, p: Palette) {
    let holder = ui.node(
        "index-holder",
        Node {
            axis: Axis::Row,
            justify: Justify::End,
            align: Align::Center,
            w: Size::Grow,
            h: Size::Grow,
            ..Node::DEFAULT
        },
    );
    ui.within(&holder, &mut |ui| {
        let rail = ui.node(
            "index",
            Node {
                axis: Axis::Column,
                w: Size::Px(RAIL_WIDTH),
                h: Size::Px(collection.sections.len() as f32 * RAIL_STEP),
                role: Role::Group,
                ..Node::DEFAULT
            },
        );
        ui.a11y_name(&rail, "Section index");
        let top = ui.rect(&rail).y;
        let mut dragging = false;
        ui.within(&rail, &mut |ui| {
            for (letter, first) in collection.sections.iter().copied() {
                let node = ui.node(
                    letter,
                    Node {
                        flags: CLICKABLE,
                        role: Role::Button,
                        axis: Axis::Row,
                        justify: Justify::End,
                        align: Align::Center,
                        w: Size::Px(RAIL_WIDTH),
                        h: Size::Px(RAIL_STEP),
                        padding: Edges {
                            r: 6.0,
                            ..Edges::ZERO
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&node, &mut |ui| {
                    text(ui, "letter", letter, 11, p.secondary, (Size::Fit, RAIL_STEP), false);
                });
                ui.a11y_name(&node, letter);
                // A finger on the rail is a held press-and-drag, not a scroll or a row swipe underneath.
                ui.gesture(&node, GestureOpts { touch_drag: true, ..Default::default() });
                let signal = ui.signal_state(&node);
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
    });
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
        p,
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
            let column = container(ui, "lead", Axis::Column, Size::Grow, Size::Grow, p.paper);
            ui.within(&column, &mut |ui| {
                nav_row(ui, cx, "Library", Route::Artists, intent);
                let name = if collection.title.is_empty() { "Unknown artist" } else { &collection.title };
                titles(ui, name, &collection.summary, p);
                spacer(ui, "keys-gap", 8.0);
                play_keys(ui, p, None, intent);
                spacer(ui, "albums-gap", 16.0);
                let cell = album_cell(width);
                for (line, pair) in albums.chunks(2).enumerate() {
                    if line > 0 {
                        spacer(ui, "album-row-gap", 20.0);
                    }
                    let cards = ui.node_keyed(
                        int_key(column.box_key, line as i64),
                        Node {
                            axis: Axis::Row,
                            gap: CARD_GAP,
                            w: Size::Grow,
                            h: Size::Px(album_card(width)),
                            padding: Edges {
                                l: 16.0,
                                r: 16.0,
                                ..Edges::ZERO
                            },
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&cards, &mut |ui| {
                        for index in pair.iter().copied() {
                            let group = &catalog.albums[index];
                            let card = ui.node_keyed(
                                int_key(cards.box_key, index as i64),
                                Node {
                                    flags: CLICKABLE | TAB_STOP,
                                    role: Role::Button,
                                    w: Size::Px(cell),
                                    h: Size::Grow,
                                    ..Node::DEFAULT
                                },
                            );
                            ui.a11y_name(&card, &group.title);
                            ui.within(&card, &mut |ui| {
                                let art = container(
                                    ui,
                                    "artwork",
                                    Axis::Stack,
                                    Size::Px(cell),
                                    Size::Px(cell),
                                    p.panel,
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
                                    p.ink,
                                    0,
                                );
                                let year = group.rows.first().and_then(|row| catalog.row(*row).release_year);
                                let year = ClockLabel::integer(year.unwrap_or(0).max(0) as u64);
                                detail::line(
                                    ui,
                                    "year",
                                    if year.bytes() == b"0" {
                                        ""
                                    } else {
                                        std::str::from_utf8(year.bytes()).unwrap_or("")
                                    },
                                    (13.0, 18.2, 400, 0.0),
                                    p.secondary,
                                    0,
                                );
                            });
                            if ui.signal_state(&card).clicked {
                                *intent = Some(Intent::Navigate(Route::Album(index)));
                            }
                        }
                    });
                }
                spacer(ui, "songs-gap", 20.0);
                let label = ui.node(
                    "songs-label",
                    Node {
                        w: Size::Grow,
                        h: Size::Px(SECTION_LABEL),
                        padding: Edges {
                            l: 16.0,
                            t: 20.0,
                            b: 8.0,
                            ..Edges::ZERO
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&label, &mut |ui| {
                    detail::line(ui, "text", "Songs", (13.0, 18.2, 500, 0.0), p.secondary, 0);
                });
            });
        },
    );
}

/// Swift's detail back row: chevron and the parent's name, 44 pt tall.
fn back_row(
    ui: &mut Ui,
    label: &'static str,
    parent: Route,
    p: Palette,
    intent: &mut Option<Intent>,
    extras: &mut dyn FnMut(&mut Ui, &mut Option<Intent>),
) {
    let back = ui.node(
        "back-row",
        Node { axis: Axis::Row, align: Align::Center, w: Size::Grow, h: Size::Px(BACK_ROW), padding: Edges { t: 10.0, l: 6.0, ..Edges::ZERO }, ..Node::DEFAULT },
    );
    ui.within(&back, &mut |ui| {
        let button = ui.node(
            "back",
            Node { flags: CLICKABLE | TAB_STOP, role: Role::Button, axis: Axis::Row, align: Align::Center, w: Size::Px(96.0), h: Size::Px(44.0), ..Node::DEFAULT },
        );
        ui.a11y_name(&button, "Back");
        ui.within(&button, &mut |ui| {
            let glyph = ui.node("icon", Node { w: Size::Px(20.0), h: Size::Px(20.0), ..Node::DEFAULT });
            let color = p.secondary;
            ui.draw(&glyph, move |paint, rect| kit::paint_icon(paint, Icon::Back, rect.x, rect.y, color));
            text(ui, "label", label, 15, p.secondary, (Size::Fit, 21.0), false);
        });
        if ui.signal_state(&button).clicked {
            *intent = Some(Intent::Navigate(parent));
        }
        extras(ui, intent);
    });
}

/// Name (32 pt) over a 13 pt summary.
fn titles(ui: &mut Ui, title: &str, summary: &str, p: Palette) {
    let titles = ui.node("titles", Node { w: Size::Grow, h: Size::Px(TITLE_BLOCK), padding: Edges { l: 16.0, r: 16.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&titles, &mut |ui| {
        detail::line(ui, "name", title, (32.0, 33.6, 500, -1.12), p.ink, 0);
        spacer(ui, "gap", 4.0);
        detail::line(ui, "summary", summary, (13.0, 18.2, 400, 0.0), p.secondary, 0);
    });
}

fn find_result_keys(ui: &mut Ui, catalog: &Catalog, p: Palette, intent: &mut Option<Intent>) {
    let kept = catalog.view.as_ref().and_then(|view| view.find.as_ref()).is_some_and(|find| find.pinned);
    let keys = ui.node("find-keys", Node { axis: Axis::Row, gap: 8.0, w: Size::Grow, h: Size::Px(ROW), padding: Edges { l: 16.0, r: 16.0, t: 2.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&keys, &mut |ui| {
        if key(ui, "play-results", "Play results", Icon::Play, false, p) {
            *intent = Some(Intent::PlayRow(0));
        }
        let (label, icon) = if kept { ("Kept offline", Icon::Check) } else { ("Keep results offline", Icon::Down) };
        if key(ui, "keep-results-offline", label, icon, false, p) {
            *intent = Some(Intent::TogglePins);
        }
    });
}

/// Play and Shuffle keys.
fn play_keys(ui: &mut Ui, p: Palette, offline: Option<bool>, intent: &mut Option<Intent>) {
    let keys = ui.node("keys", Node { axis: Axis::Row, gap: 8.0, w: Size::Grow, h: Size::Px(PLAY_BAR), padding: Edges { l: 16.0, r: 16.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&keys, &mut |ui| {
        if key(ui, "play", "Play", Icon::Play, true, p) {
            *intent = Some(Intent::PlayRow(0));
        }
        if key(ui, "shuffle", "Shuffle", Icon::Shuffle, false, p) {
            *intent = Some(Intent::ShuffleCollection);
        }
        if let Some(kept) = offline
            && offline_key(ui, kept, p)
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
pub(in crate::shell) fn offline_key(ui: &mut Ui, kept: bool, p: Palette) -> bool {
    let label = if kept { "Kept offline" } else { "Keep offline" };
    let node = ui.node(
        "keep-offline",
        Node {
            axis: Axis::Row,
            flags: CLICKABLE | TAB_STOP,
            role: Role::Button,
            align: Align::Center,
            justify: Justify::Center,
            gap: 8.0,
            w: Size::Grow,
            h: Size::Px(PLAY_BAR),
            look: Look { fill: p.panel, ..Look::NONE },
            ..Node::DEFAULT
        },
    );
    ui.within(&node, &mut |ui| {
        let glyph = ui.node("icon", Node { w: Size::Px(20.0), h: Size::Px(20.0), ..Node::DEFAULT });
        let color = p.ink;
        ui.draw(&glyph, move |paint, rect| {
            crate::icons::offline(paint, pxui::geom::Rect { x: rect.x, y: rect.y, w: 20.0, h: 20.0 }, kept, color)
        });
        text(ui, "label", label, 15, p.ink, (Size::Fit, 21.0), false);
    });
    ui.a11y_name(&node, label);
    ui.signal_state(&node).clicked
}

/// Select-mode part of a lead's cache key: the marks only matter while selecting.
fn select_revision(state: &ShellState) -> (bool, u64) {
    (state.selecting, if state.selecting { state.selection_revision } else { 0 })
}

const BAR: f32 = 64.0;

/// Back row, or Swift's selection header (Select all, "N selected", Done) in Select mode.
/// Albums and lists add the page "…" menu; artists only Select.
fn nav_row(ui: &mut Ui, cx: &Cx<'_>, label: &'static str, parent: Route, intent: &mut Option<Intent>) {
    let Cx { collection, state, p, .. } = *cx;
    let playlist = cx.list.is_some();
    if !state.selecting {
        back_row(ui, label, parent, p, intent, &mut |ui, intent| {
            container(ui, "nav-space", Axis::Row, Size::Grow, Size::Px(1.0), p.paper);
            // Playlists edit (select and remove); the others select.
            if !(playlist && collection.rows.is_empty())
                && word(ui, "select", if playlist { "Edit" } else { "Select" }, false, p)
            {
                *intent = Some(Intent::SelectMode(true));
            }
            if !matches!(state.route, Route::Artist(_))
                && more_button(ui, if playlist { "List actions" } else { "Album actions" }, p)
            {
                *intent = Some(Intent::PageMenu((cx.width - 8.0, BACK_ROW)));
            }
        });
        return;
    }
    selection_header(ui, p, state.marked.len(), collection.rows.len(), collection.rows.len(), intent);
}

/// Swift's WaveMobileSelectionHeader: Select all, "N selected", Done. `selectable` rows count toward "all";
/// "Select all" marks `rows` positions.
pub(in crate::shell) fn selection_header(ui: &mut Ui, p: Palette, selected: usize, selectable: usize, rows: usize, intent: &mut Option<Intent>) {
    let row = ui.node("selection-header", Node { axis: Axis::Row, align: Align::Center, w: Size::Grow, h: Size::Px(BACK_ROW), padding: Edges { t: 10.0, l: 12.0, r: 12.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&row, &mut |ui| {
        let all = selected > 0 && selected == selectable;
        if word(ui, "select-all", if all { "Deselect all" } else { "Select all" }, false, p) {
            *intent = Some(Intent::Select(if all { Selection::Clear } else { Selection::All(rows) }));
        }
        container(ui, "left-space", Axis::Row, Size::Grow, Size::Px(1.0), p.paper);
        let mut buffer = [0u8; 40];
        text(ui, "count", selected_label(&mut buffer, selected), 15, p.ink, (Size::Fit, 21.0), false);
        container(ui, "right-space", Axis::Row, Size::Grow, Size::Px(1.0), p.paper);
        if word(ui, "done", "Done", true, p) {
            *intent = Some(Intent::SelectMode(false));
        }
    });
}

/// "N selected" without a heap allocation.
fn selected_label(buffer: &mut [u8; 40], count: usize) -> &str {
    let digits = ClockLabel::integer(count as u64);
    let (head, tail) = (digits.bytes(), b" selected");
    buffer[..head.len()].copy_from_slice(head);
    buffer[head.len()..head.len() + tail.len()].copy_from_slice(tail);
    std::str::from_utf8(&buffer[..head.len() + tail.len()]).unwrap_or("")
}

/// A 44 pt text button (Swift's Select / Done at 16 pt).
pub(in crate::shell) fn word(ui: &mut Ui, key: &'static str, label: &'static str, _medium: bool, p: Palette) -> bool {
    let node = ui.node(key, Node { flags: CLICKABLE | TAB_STOP, role: Role::Button, axis: Axis::Row, align: Align::Center, w: Size::Fit, h: Size::Px(44.0), padding: Edges { l: 4.0, r: 4.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&node, &mut |ui| {
        text(ui, "label", label, 16, p.ink, (Size::Fit, 22.4), false);
    });
    ui.a11y_name(&node, label);
    ui.signal_state(&node).clicked
}

/// Swift's 20 pt selection square: ink with a check when marked, an outline otherwise.
pub(in crate::shell) fn checkbox(ui: &mut Ui, marked: bool, p: Palette) {
    let node = ui.node("checkbox", Node { w: Size::Px(20.0), h: Size::Px(20.0), ..Node::DEFAULT });
    let (ink, outline, paper) = (p.ink, p.secondary, p.paper);
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
pub(in crate::shell) fn deck(ui: &mut Ui, p: Palette, enabled: bool, remove: bool) -> Option<(&'static str, pxui::geom::Rect)> {
    let mut pressed = None;
    let holder = ui.node("selection-holder", Node { axis: Axis::Column, justify: Justify::End, w: Size::Grow, h: Size::Grow, ..Node::DEFAULT });
    ui.within(&holder, &mut |ui| {
        let deck = ui.node("selection-bar", Node { axis: Axis::Row, gap: 2.0, w: Size::Grow, h: Size::Px(BAR), padding: Edges { l: 2.0, r: 2.0, t: 2.0, ..Edges::ZERO }, look: Look { fill: p.panel, ..Look::NONE }, ..Node::DEFAULT });
        ui.set_role(&deck, Role::Group, 0);
        ui.a11y_name(&deck, "Selection actions");
        ui.within(&deck, &mut |ui| {
            for (key, label, glyph) in [
                ("play-next", "Play next", Icon::Next),
                ("add-to-list", "Add to list", Icon::Clear),
                ("offline", "Offline", Icon::Down),
                (if remove { "remove" } else { "queue" }, if remove { "Remove" } else { "Queue" }, if remove { Icon::Clear } else { Icon::Queue }),
            ] {
                let ink = if !enabled { p.tertiary } else if key == "remove" { p.danger } else { p.ink };
                let node = ui.node(
                    key,
                    Node {
                        flags: if enabled { CLICKABLE | TAB_STOP } else { 0 },
                        role: Role::Button,
                        state: State { disabled: !enabled, ..State::default() },
                        axis: Axis::Column,
                        align: Align::Center,
                        justify: Justify::Center,
                        gap: 4.0,
                        w: Size::Grow,
                        h: Size::Grow,
                        look: Look { fill: p.key, ..Look::NONE },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&node, &mut |ui| {
                    let icon = ui.node("icon", Node { w: Size::Px(20.0), h: Size::Px(20.0), ..Node::DEFAULT });
                    ui.draw(&icon, move |paint, rect| match key {
                        "add-to-list" => {
                            paint.fill(rect.x + 3.0, rect.y + 9.25, 14.0, 1.5, ink);
                            paint.fill(rect.x + 9.25, rect.y + 3.0, 1.5, 14.0, ink);
                        }
                        "offline" => crate::icons::offline(paint, pxui::geom::Rect { x: rect.x, y: rect.y, w: 20.0, h: 20.0 }, false, ink),
                        _ => kit::paint_icon(paint, glyph, rect.x, rect.y, ink),
                    });
                    text(ui, "label", label, 12, ink, (Size::Fit, 16.8), false);
                });
                ui.a11y_name(&node, label);
                if enabled && ui.signal_state(&node).clicked {
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
    if let Some((key, rect)) = deck(ui, cx.p, first.is_some(), remove.is_some()) {
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
    let Cx { catalog, collection, state, player, width, p, .. } = *cx;
    let revision = crate::hash((
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        width.to_bits(),
        player.artwork.map_or(0, crate::artwork::Images::revision),
        select_revision(state),
    ));
    ui.view("album-lead", revision, Size::Grow, Size::Px(album_lead_height()), &mut |ui| {
        let column = container(ui, "lead", Axis::Column, Size::Grow, Size::Grow, p.paper);
        ui.within(&column, &mut |ui| {
            nav_row(ui, cx, "Library", Route::Albums, intent);
            let top = ui.node("album-top", Node { axis: Axis::Row, gap: 16.0, w: Size::Grow, h: Size::Px(ALBUM_ART), padding: Edges { l: 16.0, r: 16.0, ..Edges::ZERO }, ..Node::DEFAULT });
            ui.within(&top, &mut |ui| {
                let art = container(ui, "artwork", Axis::Stack, Size::Px(ALBUM_ART), Size::Px(ALBUM_ART), p.panel);
                paint_artwork(ui, &art, collection.artwork_row.map(|row| catalog.row(row)), ALBUM_ART, player.artwork);
                ui.a11y_name(&art, "Album artwork");
                let meta = ui.node("meta", Node { w: Size::Grow, h: Size::Grow, justify: Justify::End, gap: 6.0, ..Node::DEFAULT });
                ui.within(&meta, &mut |ui| {
                    let text_width = (width - 32.0 - ALBUM_ART - 16.0).max(1.0);
                    super::super::settings::paragraph(ui, "title", &collection.title, text_width, 24.0, p.ink);
                    if let lw_core::context::PlaybackContext::Album { artist, .. } = collection.context.as_ref()
                        && !artist.is_empty()
                    {
                        let link = detail::line(ui, "artist", artist, (17.0, 23.8, 400, 0.0), p.secondary, CLICKABLE | TAB_STOP);
                        if ui.signal_state(&link).clicked
                            && let Some(index) = catalog.library.artists.iter().position(|a| a.title == *artist)
                        {
                            *intent = Some(Intent::Navigate(Route::Artist(index)));
                        }
                    }
                    text(ui, "summary", &collection.summary, 13, p.secondary, (Size::Grow, 18.2), true);
                });
            });
            spacer(ui, "keys-gap", 16.0);
            play_keys(ui, p, offline_state(cx), intent);
        });
    });
}

/// Compact list page lead-in (playlists, Favorites and the other automatic lists).
fn collection_lead_height() -> f32 {
    BACK_ROW + TITLE_BLOCK + 8.0 + PLAY_BAR + 16.0
}
fn collection_lead(ui: &mut Ui, cx: &Cx<'_>, intent: &mut Option<Intent>) {
    let Cx { catalog, collection, state, p, .. } = *cx;
    let revision = crate::hash((
        state.revision,
        collection.key,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        select_revision(state),
    ));
    ui.view("collection-lead", revision, Size::Grow, Size::Px(collection_lead_height()), &mut |ui| {
        let column = container(ui, "lead", Axis::Column, Size::Grow, Size::Grow, p.paper);
        ui.within(&column, &mut |ui| {
            nav_row(ui, cx, "Lists", Route::Lists, intent);
            titles(ui, &collection.title, &collection.summary, p);
            spacer(ui, "keys-gap", 8.0);
            play_keys(ui, p, offline_state(cx), intent);
            spacer(ui, "list-gap", 16.0);
        });
    });
}

/// Swift's playKeyButtons: a 52 pt key, ink for Play.
pub(in crate::shell) fn key(ui: &mut Ui, id: &str, label: &'static str, icon: Icon, primary: bool, p: Palette) -> bool {
    let node = ui.node(
        id,
        Node {
            axis: Axis::Row,
            flags: CLICKABLE | TAB_STOP,
            role: Role::Button,
            align: Align::Center,
            justify: Justify::Center,
            gap: 8.0,
            w: Size::Grow,
            h: Size::Px(PLAY_BAR),
            look: Look {
                fill: if primary { p.ink } else { p.panel },
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    let color = if primary { p.paper } else { p.ink };
    ui.within(&node, &mut |ui| {
        let glyph = ui.node(
            "icon",
            Node {
                w: Size::Px(20.0),
                h: Size::Px(20.0),
                ..Node::DEFAULT
            },
        );
        ui.draw(&glyph, move |paint, rect| {
            kit::paint_icon(paint, icon, rect.x, rect.y, color)
        });
        text(ui, "label", label, 15, color, (Size::Fit, 21.0), false);
    });
    ui.a11y_name(&node, label);
    ui.signal_state(&node).clicked
}
