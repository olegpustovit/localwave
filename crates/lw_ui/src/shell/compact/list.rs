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
        header_view(ui, catalog, state, width, p, intent);
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
    if collection.rows.is_empty() && artist.is_none() {
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
        p,
    };
    let lead = artist.map_or(0.0, |index| artist_lead_height(catalog, index, width));
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
                count: collection.rows.len(),
                row_height: ROW,
                leading_height: lead,
                header_height: 0.0,
                trailing_height: 0.0,
                overlay_scrollbar: true,
            },
            &mut |ui, part| match part {
                SectionPart::Leading => {
                    if let Some(index) = artist {
                        artist_lead(ui, &cx, index, intent);
                    }
                }
                SectionPart::Header => {}
                SectionPart::Row(index) => row(ui, &cx, index, intent),
            },
        );
        refresh(ui, &list, state, intent);
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
            GestureOpts {
                long_press: true,
                swipe_actions: Some(SwipeActions {
                    leading: SWIPE_ACTION,
                    trailing: SWIPE_ACTION,
                    full: SWIPE_FULL,
                }),
                ..Default::default()
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
            artwork(ui, catalog, player, Some(collection.rows[index]), 40.0, p);
            let labels = container(ui, "labels", Axis::Column, Size::Grow, Size::Fit, p.paper);
            ui.within(&labels, &mut |ui| {
                let ink = if player.current_id == Some(row.id) {
                    p.signal
                } else {
                    p.ink
                };
                line(ui, "title", row.title, 15.0, ink);
                line(ui, "artist", row.subtitle, 13.0, p.secondary);
            });
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
            if g.swipe_open {
                ui.close_swipe(&node);
            } else {
                *intent = Some(Intent::PlayRow(index));
            }
        }
    });
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
    let revision = pxui::hash!((
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        width.to_bits(),
        player.artwork.map_or(0, crate::artwork::Images::revision),
    ));
    ui.view(
        "artist-lead",
        revision,
        Size::Grow,
        Size::Px(artist_lead_height(catalog, artist, width)),
        &mut |ui| {
            let column = container(ui, "lead", Axis::Column, Size::Grow, Size::Grow, p.paper);
            ui.within(&column, &mut |ui| {
                let back = ui.node(
                    "back-row",
                    Node {
                        axis: Axis::Row,
                        align: Align::Center,
                        w: Size::Grow,
                        h: Size::Px(BACK_ROW),
                        padding: Edges {
                            t: 10.0,
                            l: 6.0,
                            ..Edges::ZERO
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&back, &mut |ui| {
                    let button = ui.node(
                        "back",
                        Node {
                            flags: CLICKABLE | TAB_STOP,
                            role: Role::Button,
                            axis: Axis::Row,
                            align: Align::Center,
                            w: Size::Px(96.0),
                            h: Size::Px(44.0),
                            ..Node::DEFAULT
                        },
                    );
                    ui.a11y_name(&button, "Back");
                    ui.within(&button, &mut |ui| {
                        let glyph = ui.node(
                            "icon",
                            Node {
                                w: Size::Px(20.0),
                                h: Size::Px(20.0),
                                ..Node::DEFAULT
                            },
                        );
                        let color = p.secondary;
                        ui.draw(&glyph, move |paint, rect| {
                            kit::paint_icon(paint, Icon::Back, rect.x, rect.y, color)
                        });
                        text(ui, "label", "Library", 15, p.secondary, (Size::Fit, 21.0), false);
                    });
                    if ui.signal_state(&button).clicked {
                        *intent = Some(Intent::Navigate(Route::Artists));
                    }
                });
                let titles = ui.node(
                    "titles",
                    Node {
                        w: Size::Grow,
                        h: Size::Px(TITLE_BLOCK),
                        padding: Edges {
                            l: 16.0,
                            r: 16.0,
                            ..Edges::ZERO
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&titles, &mut |ui| {
                    let title = if collection.title.is_empty() {
                        "Unknown artist"
                    } else {
                        &collection.title
                    };
                    detail::line(ui, "name", title, (32.0, 33.6, 500, -1.12), p.ink, 0);
                    spacer(ui, "gap", 4.0);
                    detail::line(
                        ui,
                        "summary",
                        &collection.summary,
                        (13.0, 18.2, 400, 0.0),
                        p.secondary,
                        0,
                    );
                });
                spacer(ui, "keys-gap", 8.0);
                let keys = ui.node(
                    "keys",
                    Node {
                        axis: Axis::Row,
                        gap: 8.0,
                        w: Size::Grow,
                        h: Size::Px(PLAY_BAR),
                        padding: Edges {
                            l: 16.0,
                            r: 16.0,
                            ..Edges::ZERO
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&keys, &mut |ui| {
                    if key(ui, "play", "Play", Icon::Play, true, p) {
                        *intent = Some(Intent::PlayRow(0));
                    }
                    if key(ui, "shuffle", "Shuffle", Icon::Shuffle, false, p) {
                        *intent = Some(Intent::ShuffleCollection);
                    }
                });
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

/// Swift's playKeyButtons: a 52 pt key, ink for Play.
fn key(ui: &mut Ui, id: &str, label: &'static str, icon: Icon, primary: bool, p: Palette) -> bool {
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
                fill: if primary { p.ink } else { p.key },
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
