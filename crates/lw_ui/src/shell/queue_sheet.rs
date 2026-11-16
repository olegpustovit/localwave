use super::*;
use crate::browse::Query;
use crate::catalog::Catalog;
use crate::{Intent, PlayerView};

pub(super) fn queue_action(
    ui: &mut Ui,
    key: &str,
    label: &str,
    name: &str,
    width: f32,
    p: Palette,
) -> bool {
    let button = ui.node(
        key,
        Node {
            flags: CLICKABLE | TAB_STOP | FIT_TEXT,
            role: Role::Button,
            w: Size::Px(width),
            h: Size::Px(28.0),
            look: Look {
                text: p.secondary,
                font: Font {
                    size: 12,
                    ..Font::BODY
                },
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.text(&button, label);
    ui.a11y_name(&button, name);
    ui.signal_state(&button).clicked
}

/// Row metrics: the Now panel's Up next (Swift `desktop`) and the bottom sheet's.
struct Metrics {
    row: f32,
    grip: f32,
    art: f32,
    title: i64,
    title_line: f32,
    artist: i64,
    artist_line: f32,
    /// Item spacing, leading/trailing padding, remove-button width, availability padding.
    gap: f32,
    lead: f32,
    trail: f32,
    remove: f32,
    glyph: f32,
}
const PANEL: Metrics = Metrics {
    row: 44.0,
    grip: 24.0,
    art: 30.0,
    title: 13,
    title_line: 19.0,
    artist: 12,
    artist_line: 17.0,
    gap: 0.0,
    lead: 0.0,
    trail: 0.0,
    remove: 28.0,
    glyph: 4.0,
};
const SHEET: Metrics = Metrics {
    row: 56.0,
    grip: 40.0,
    art: 36.0,
    title: 15,
    title_line: 21.0,
    artist: 13,
    artist_line: 18.2,
    gap: 10.0,
    lead: 4.0,
    trail: 6.0,
    remove: 44.0,
    glyph: 0.0,
};

/// Swift's queue sheet is a full-width bottom sheet on every platform (`LWRootSheetHost`; no
/// macOS variant): handle, header, Now row, Up next rows, Clear up next. Its height is computed
/// from the snapshot so the popup needs no measuring pass; shell() draws the dim behind it.
pub(super) fn queue_sheet(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let content = ui.content_rect();
    let queue = catalog.queue.as_deref();
    let upcoming = queue.map_or(0, |queue| queue.upcoming().len());
    let now = queue
        .and_then(|queue| queue.current.and_then(|index| queue.entries.get(index)))
        .and_then(|entry| catalog.row_index(entry.song_id))
        .map(|index| catalog.row(index));
    let notice = !player.message.is_empty();
    let wanted = 21.0
        + 66.2
        + if state.playlist_editing { 100.0 } else { 0.0 }
        + if notice { 28.0 } else { 0.0 }
        + if now.is_some() { 56.0 } else { 0.0 }
        + 46.2
        + if upcoming > 0 {
            upcoming as f32 * 56.0 + 52.0
        } else {
            96.0
        }
        + 20.0;
    let height = wanted.min((content.h * 0.86 - 33.0).max(0.0));
    let width = content.w;
    let open = ui.popup(
        None,
        Some(height),
        false,
        false,
        (content.x, content.y + content.h - height + state.sheet_dismiss.get()),
        width,
        height,
        "Playback queue",
        &mut |ui| {
            let sheet_node = Node {
                flags: BLOCKING | CLIP | CLICKABLE,
                axis: Axis::Column,
                role: Role::Dialog,
                w: Size::Px(width),
                h: Size::Px(height),
                look: Look {
                    fill: p.paper,
                    ..Look::NONE
                },
                ..Node::DEFAULT
            };
            let sheet = ui.node("queue-sheet", sheet_node);
            let drag = ui.gesture(
                &sheet,
                pxui::GestureOpts {
                    dismiss: Some(Default::default()),
                    ..Default::default()
                },
            );
            // The popup itself follows the finger: moving only the node inside it loses the touch.
            state.sheet_dismiss.set(drag.dismiss);
            if drag.dismiss_committed {
                *intent = Some(Intent::CloseQueue);
            }
            ui.set_role(&sheet, Role::Dialog, pxui::a11y::state::MODAL);
            ui.a11y_name(&sheet, "Playback queue");
            ui.within(&sheet, &mut |ui| {
                let handle = container(
                    ui,
                    "queue-handle",
                    Axis::Stack,
                    Size::Grow,
                    Size::Px(21.0),
                    p.paper,
                );
                ui.draw(&handle, move |paint, rect| {
                    paint.fill(
                        rect.x + (rect.w - 36.0) * 0.5,
                        rect.y + 8.0,
                        36.0,
                        5.0,
                        p.line,
                    );
                });
                let header = ui.node(
                    "full-queue-header",
                    Node {
                        axis: Axis::Row,
                        w: Size::Grow,
                        h: Size::Px(66.2),
                        padding: Edges {
                            l: 16.0,
                            r: 6.0,
                            t: 12.0,
                            b: 8.0,
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&header, &mut |ui| {
                    // Swift reaches Save as list from this header's context menu and an a11y action.
                    let titles = ui.node(
                        "queue-titles",
                        Node {
                            flags: CLICKABLE | TAB_STOP,
                            axis: Axis::Column,
                            role: Role::Button,
                            w: Size::Grow,
                            h: Size::Grow,
                            ..Node::DEFAULT
                        },
                    );
                    ui.a11y_name(&titles, "Save queue as playlist");
                    let signal = ui.signal_state(&titles);
                    if queue.is_some_and(|queue| !queue.entries.is_empty())
                        && (signal.clicked || (signal.pressed && signal.button == Some(1)))
                    {
                        *intent = Some(Intent::NamePlaylist);
                    }
                    ui.within(&titles, &mut |ui| {
                        text(
                            ui,
                            "full-queue-title",
                            "Queue",
                            20,
                            p.ink,
                            (Size::Grow, 28.0),
                            false,
                        );
                        let from = container(
                            ui,
                            "queue-from",
                            Axis::Row,
                            Size::Grow,
                            Size::Px(18.2),
                            p.paper,
                        );
                        ui.within(&from, &mut |ui| {
                            text(
                                ui,
                                "from",
                                "From ",
                                13,
                                p.secondary,
                                (Size::Fit, 18.2),
                                false,
                            );
                            text(
                                ui,
                                "context",
                                player.context,
                                13,
                                p.secondary,
                                (Size::Fit, 18.2),
                                false,
                            );
                            if player.shuffled {
                                text(
                                    ui,
                                    "shuffled",
                                    " · shuffled",
                                    13,
                                    p.secondary,
                                    (Size::Fit, 18.2),
                                    false,
                                );
                            }
                        });
                    });
                    if icon_button(
                        ui,
                        "close-queue",
                        "Close queue",
                        Icon::Clear,
                        (44.0, 44.0),
                        false,
                        p,
                    ) {
                        *intent = Some(Intent::CloseQueue);
                    }
                });
                if state.playlist_editing {
                    text(
                        ui,
                        "playlist-prompt",
                        "Save all tracks in queue order; duplicates are included once.",
                        12,
                        p.secondary,
                        (Size::Grow, 32.0),
                        false,
                    );
                    let row = container(
                        ui,
                        "playlist-editor",
                        Axis::Row,
                        Size::Grow,
                        Size::Px(44.0),
                        p.paper,
                    );
                    ui.within(&row, &mut |ui| {
                        let field = ui.box_(
                            "playlist-name-field",
                            BoxOpts::default().w(Size::Grow).h(Size::Grow),
                        );
                        ui.within(&field, &mut |ui| {
                            if let Some(value) = ui.text_field_with_options(
                                "Playlist name",
                                state.playlist_name.as_str(),
                                pxui::TextInputOptions {
                                    action: pxui::TextInputOptions::DONE,
                                    ..Default::default()
                                },
                            ) {
                                *intent =
                                    Some(Query::new(value).map_or(
                                        Intent::PlaylistNameTooLong,
                                        Intent::EditPlaylistName,
                                    ));
                            }
                        });
                        let valid = !state.playlist_name_too_long
                            && !state.playlist_name.as_str().trim().is_empty()
                            && catalog
                                .queue
                                .as_ref()
                                .is_some_and(|queue| !queue.entries.is_empty());
                        if valid {
                            if queue_action(
                                ui,
                                "create-playlist",
                                "Create",
                                "Create playlist from queue",
                                60.0,
                                p,
                            ) {
                                *intent = Some(Intent::SaveQueue(state.playlist_name));
                            }
                        } else {
                            text(
                                ui,
                                "create-unavailable",
                                "Create",
                                12,
                                p.secondary,
                                (Size::Px(60.0), 28.0),
                                false,
                            );
                        }
                        if queue_action(
                            ui,
                            "cancel-playlist",
                            "Cancel",
                            "Cancel playlist creation",
                            60.0,
                            p,
                        ) {
                            *intent = Some(Intent::CancelPlaylistName);
                        }
                    });
                    text(
                        ui,
                        "playlist-name-error",
                        if state.playlist_name_too_long {
                            "Name exceeds 1,024 UTF-8 bytes. Shorten it to save."
                        } else {
                            ""
                        },
                        12,
                        p.secondary,
                        (Size::Grow, 24.0),
                        false,
                    );
                }
                if notice {
                    text(
                        ui,
                        "queue-notice",
                        player.message,
                        12,
                        p.secondary,
                        (Size::Grow, 28.0),
                        false,
                    );
                }
                if let Some(song) = now {
                    now_row(ui, player, song, width, p);
                }
                let label = ui.node(
                    "up-next-label",
                    Node {
                        axis: Axis::Row,
                        w: Size::Grow,
                        h: Size::Px(46.2),
                        gap: 4.0,
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
                    text(
                        ui,
                        "name",
                        "Up next",
                        13,
                        p.secondary,
                        (Size::Fit, 18.2),
                        false,
                    );
                    text(
                        ui,
                        "count",
                        queue.map_or("0", |queue| queue.count.as_str()),
                        12,
                        p.secondary,
                        (Size::Fit, 18.2),
                        true,
                    );
                });
                if upcoming == 0 {
                    let empty = ui.node(
                        "queue-empty",
                        Node {
                            axis: Axis::Row,
                            align: Align::Center,
                            w: Size::Grow,
                            h: Size::Px(96.0),
                            gap: 8.0,
                            padding: Edges {
                                l: 26.0,
                                ..Edges::ZERO
                            },
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&empty, &mut |ui| {
                        let undo = queue.is_some_and(|queue| queue.can_undo);
                        text(
                            ui,
                            "message",
                            if undo {
                                "Up next cleared."
                            } else {
                                "Nothing queued."
                            },
                            14,
                            p.secondary,
                            (Size::Fit, 20.0),
                            false,
                        );
                        if undo && queue_action(ui, "undo", "Undo", "Undo clear up next", 44.0, p) {
                            *intent = Some(Intent::UndoQueue);
                        }
                    });
                } else {
                    let revision = crate::hash((
                        queue.map(|queue| queue.revision),
                        std::sync::Arc::as_ptr(&catalog.library) as usize,
                        width.to_bits(),
                        player.artwork.map_or(0, crate::artwork::Images::revision),
                    ));
                    let rows = ui.node(
                        "full-queue-body",
                        Node {
                            w: Size::Grow,
                            h: Size::Grow,
                            padding: Edges {
                                l: 10.0,
                                r: 10.0,
                                ..Edges::ZERO
                            },
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&rows, &mut |ui| {
                        ui.view(
                            "full-queue-rows",
                            revision,
                            Size::Grow,
                            Size::Grow,
                            &mut |ui| {
                                queue_rows(
                                    ui,
                                    catalog,
                                    true,
                                    width - 20.0,
                                    player.artwork,
                                    p,
                                    intent,
                                )
                            },
                        );
                    });
                    let clear = ui.node(
                        "clear",
                        Node {
                            flags: CLICKABLE | TAB_STOP,
                            axis: Axis::Row,
                            align: Align::Center,
                            role: Role::Button,
                            w: Size::Grow,
                            h: Size::Px(52.0),
                            padding: Edges {
                                l: 26.0,
                                ..Edges::ZERO
                            },
                            ..Node::DEFAULT
                        },
                    );
                    ui.a11y_name(&clear, "Clear up next");
                    ui.within(&clear, &mut |ui| {
                        text(
                            ui,
                            "label",
                            "Clear up next",
                            16,
                            p.secondary,
                            (Size::Grow, 22.4),
                            false,
                        );
                    });
                    if ui.signal_state(&clear).clicked {
                        *intent = Some(Intent::ClearQueue);
                    }
                }
                spacer(ui, "queue-bottom", 20.0);
            });
        },
    );
    if !open {
        *intent = Some(Intent::CloseQueue);
    }
}

/// Swift's Now row: artwork, title, "Now · artist" and the playing bars.
fn now_row(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    song: crate::catalog::Row<'_>,
    width: f32,
    p: Palette,
) {
    let row = ui.node(
        "queue-now",
        Node {
            axis: Axis::Row,
            align: Align::Center,
            w: Size::Grow,
            h: Size::Px(56.0),
            gap: 10.0,
            padding: Edges {
                l: 24.0,
                r: 16.0,
                ..Edges::ZERO
            },
            role: Role::ListItem,
            ..Node::DEFAULT
        },
    );
    ui.a11y_name(&row, song.title);
    ui.within(&row, &mut |ui| {
        let art = container(
            ui,
            "now-art",
            Axis::Stack,
            Size::Px(36.0),
            Size::Px(36.0),
            p.panel,
        );
        paint_artwork(ui, &art, Some(song), 36.0, player.artwork);
        let titles = ui.node(
            "now-titles",
            Node {
                axis: Axis::Column,
                w: Size::Px((width - 134.0).max(0.0)),
                h: Size::Fit,
                ..Node::DEFAULT
            },
        );
        ui.within(&titles, &mut |ui| {
            text(
                ui,
                "title",
                song.title,
                15,
                p.ink,
                (Size::Grow, 21.0),
                false,
            );
            let line = container(ui, "line", Axis::Row, Size::Grow, Size::Px(18.2), p.paper);
            ui.within(&line, &mut |ui| {
                text(
                    ui,
                    "now",
                    "Now · ",
                    13,
                    p.secondary,
                    (Size::Fit, 18.2),
                    false,
                );
                text(
                    ui,
                    "artist",
                    song.artist,
                    13,
                    p.secondary,
                    (Size::Grow, 18.2),
                    false,
                );
            });
        });
        let bars = ui.node(
            "now-bars",
            Node {
                w: Size::Px(14.0),
                h: Size::Px(11.0),
                ..Node::DEFAULT
            },
        );
        ui.within(&bars, &mut |ui| {
            ui.view(
                "equalizer-view",
                crate::hash((player.playing, song.id)),
                Size::Grow,
                Size::Grow,
                &mut |ui| super::songs::row::equalizer(ui, player.playing, true, song.id, p),
            );
        });
        spacer(ui, "now-trail", 14.0);
    });
}

pub(super) fn queue_rows(
    ui: &mut Ui,
    catalog: &Catalog,
    sheet: bool,
    width: f32,
    images: Option<&crate::artwork::Images>,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let Some(queue) = &catalog.queue else { return };
    let entries = queue.upcoming();
    // Swift's sheet rows (desktop: false) are larger than the panel's.
    let m = if sheet { SHEET } else { PANEL };
    if !sheet {
        let header = container(
            ui,
            "queue-heading",
            Axis::Row,
            Size::Grow,
            Size::Px(34.0),
            p.paper,
        );
        ui.within(&header, &mut |ui| {
            text(
                ui,
                "queue-context",
                &queue.heading,
                13,
                p.secondary,
                (Size::Grow, 28.0),
                false,
            );
            text(
                ui,
                "queue-count",
                &queue.count,
                12,
                p.secondary,
                (Size::Px(56.0), 28.0),
                true,
            );
            if icon_button(
                ui,
                "show-queue",
                "Show full queue",
                Icon::Queue,
                (28.0, 28.0),
                false,
                p,
            ) {
                *intent = Some(Intent::ShowQueue);
            }
            if !queue.upcoming().is_empty()
                && queue_action(ui, "clear", "Clear", "Clear up next", 44.0, p)
            {
                *intent = Some(Intent::ClearQueue);
            }
            if queue.can_undo && queue_action(ui, "undo", "Undo", "Undo clear up next", 44.0, p) {
                *intent = Some(Intent::UndoQueue);
            }
        });
    }
    if entries.is_empty() {
        if sheet {
            return;
        }
        text(
            ui,
            "queue-empty",
            if queue.can_undo {
                "Up next cleared."
            } else {
                "Queue is empty."
            },
            14,
            p.secondary,
            (Size::Grow, 44.0),
            false,
        );
        return;
    }
    let list = ui.grid_list(
        "queue-entries",
        entries.len(),
        width,
        m.row,
        &mut |ui, index| {
            let entry = entries[index];
            let row = catalog
                .row_index(entry.song_id)
                .map(|index| catalog.row(index));
            let title = row.map_or("Unavailable track", |row| {
                if row.title.is_empty() {
                    "Untitled track"
                } else {
                    row.title
                }
            });
            let artist = row.map_or("", |row| row.artist);
            let item = ui.node_keyed(
                int_key(
                    Ui::key_of(
                        0,
                        if sheet {
                            "localwave-full-queue-entry"
                        } else {
                            "localwave-queue-entry"
                        },
                    ),
                    entry.id as i64,
                ),
                Node {
                    w: Size::Grow,
                    h: Size::Px(m.row),
                    axis: Axis::Row,
                    align: Align::Center,
                    gap: m.gap,
                    padding: Edges {
                        l: m.lead,
                        r: m.trail,
                        ..Edges::ZERO
                    },
                    flags: CLICKABLE,
                    role: Role::ListItem,
                    look: Look {
                        fill: p.paper,
                        ..Look::NONE
                    },
                    ..Node::DEFAULT
                },
            );
            ui.a11y_name(&item, title);
            ui.within(&item, &mut |ui| {
                let grip = ui.node(
                    "reorder-queued",
                    Node {
                        flags: CLICKABLE | TAB_STOP,
                        role: Role::Button,
                        w: Size::Px(m.grip),
                        h: Size::Px(m.row),
                        ..Node::DEFAULT
                    },
                );
                ui.a11y_name(&grip, "Reorder queued track");
                ui.draw(&grip, move |paint, rect| {
                    for x in [m.grip / 2.0 - 4.0, m.grip / 2.0] {
                        for y in [m.row / 2.0 - 5.0, m.row / 2.0 - 1.0, m.row / 2.0 + 3.0] {
                            paint.fill(rect.x + x, rect.y + y, 2.0, 2.0, p.secondary);
                        }
                    }
                });
                let signal = ui.signal_state(&grip);
                if signal.held || signal.clicked {
                    let id = ClockLabel::integer(entry.id);
                    const PREFIX: &[u8] = b"localwave.queue-occurrence:";
                    let mut kind = [0u8; 64];
                    kind[..PREFIX.len()].copy_from_slice(PREFIX);
                    let len = PREFIX.len() + id.bytes().len();
                    kind[PREFIX.len()..len].copy_from_slice(id.bytes());
                    ui.carry(
                        if signal.clicked { None } else { Some(&grip) },
                        std::str::from_utf8(&kind[..len]).expect("queue drag kind is ASCII"),
                        title,
                    );
                }
                let art = container(
                    ui,
                    "queue-art",
                    Axis::Stack,
                    Size::Px(m.art),
                    Size::Px(m.art),
                    p.panel,
                );
                paint_artwork(ui, &art, row, m.art, images);
                ui.a11y_name(&art, "Track artwork");
                let play = ui.node(
                    "play-queued",
                    Node {
                        flags: CLICKABLE | TAB_STOP,
                        role: Role::Button,
                        w: Size::Grow,
                        h: Size::Grow,
                        padding: Edges {
                            l: 10.0 - m.gap,
                            t: (m.row - m.title_line - m.artist_line) / 2.0,
                            ..Edges::ZERO
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.a11y_name(&play, title);
                ui.within(&play, &mut |ui| {
                    text(
                        ui,
                        "queue-title",
                        title,
                        m.title,
                        p.ink,
                        (Size::Grow, m.title_line),
                        false,
                    );
                    text(
                        ui,
                        "queue-artist",
                        artist,
                        m.artist,
                        p.secondary,
                        (Size::Grow, m.artist_line),
                        false,
                    );
                });
                if ui.signal_state(&play).clicked {
                    *intent = Some(Intent::PlayQueue(entry.id));
                }
                // Swift marks songs that are not on this device, or kept offline.
                if let Some(row) =
                    row.filter(|row| row.state != lw_core::FileState::Copied || row.pinned)
                {
                    let at = ui.node(
                        "availability-position",
                        Node {
                            w: Size::Px(10.0 + 2.0 * m.glyph),
                            h: Size::Px(10.0),
                            padding: Edges {
                                l: m.glyph,
                                r: m.glyph,
                                ..Edges::ZERO
                            },
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&at, &mut |ui| {
                        super::songs::row::availability(ui, row, true, p)
                    });
                }
                if icon_button(
                    ui,
                    "remove-queued",
                    "Remove from queue",
                    Icon::Clear,
                    (m.remove, m.row),
                    false,
                    p,
                ) {
                    *intent = Some(Intent::RemoveQueue(entry.id));
                }
            });
            let drop = ui.drop_target_ref(&item).and_then(|drop| {
                let (dropped, payload) = match drop {
                    pxui::DropRef::Hover(p) => (false, p),
                    pxui::DropRef::Dropped(p) => (true, p),
                };
                payload
                    .kind
                    .strip_prefix("localwave.queue-occurrence:")?
                    .parse::<u64>()
                    .ok()
                    .map(|id| (dropped, id))
            });
            if let Some((dropped, occurrence)) = drop {
                let after = ui.signal_state(&item).pointer.1 >= ui.rect(&item).y + m.row / 2.0;
                let before = if after {
                    entries.get(index + 1).map(|entry| entry.id)
                } else {
                    Some(entry.id)
                };
                if before != Some(occurrence) {
                    ui.draw(&item, move |paint, rect| {
                        paint.fill(
                            rect.x,
                            if after { rect.y + rect.h - 2.0 } else { rect.y },
                            rect.w,
                            2.0,
                            p.ink,
                        );
                    });
                    if dropped {
                        *intent = Some(Intent::MoveQueue { occurrence, before });
                    }
                }
            }
        },
    );
    ui.a11y_name(
        &list,
        if sheet {
            "Full playback queue"
        } else {
            "Up next queue"
        },
    );
    ui.transfer_carry(&list);
}
