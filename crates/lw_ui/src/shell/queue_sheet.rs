use super::*;
use crate::browse::Query;
use crate::catalog::Catalog;
use crate::{Intent, PlayerView};
use std::hash::{DefaultHasher, Hash, Hasher};

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

pub(super) fn queue_sheet(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let content = ui.content_rect();
    let (window_width, window_height) = (content.w, content.h);
    let width = (window_width - 32.0).clamp(240.0, 680.0);
    let height = (window_height - 64.0).clamp(160.0, 620.0);
    let open = ui.modal(Some(width), "Playback queue", &mut |ui| {
        let mut revision = DefaultHasher::new();
        (
            state.revision,
            player.artwork.map_or(0, crate::artwork::Images::revision),
            catalog.queue.as_ref().map(|queue| queue.revision),
            std::sync::Arc::as_ptr(&catalog.library) as usize,
            player.message.as_ptr() as usize,
            player.message.len(),
            width.to_bits(),
            height.to_bits(),
        )
            .hash(&mut revision);
        ui.view(
            "queue-sheet-content",
            revision.finish(),
            Size::Grow,
            Size::Px(height),
            &mut |ui| {
                let body = ui.node(
                    "queue-sheet-body",
                    Node {
                        w: Size::Grow,
                        h: Size::Px(height),
                        padding: Edges::all(20.0),
                        look: Look {
                            fill: p.paper,
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&body, &mut |ui| {
                    let header = container(
                        ui,
                        "full-queue-header",
                        Axis::Row,
                        Size::Grow,
                        Size::Px(40.0),
                        p.paper,
                    );
                    ui.within(&header, &mut |ui| {
                        text(
                            ui,
                            "full-queue-title",
                            "Queue",
                            24,
                            p.ink,
                            (Size::Grow, 36.0),
                            false,
                        );
                        if catalog
                            .queue
                            .as_ref()
                            .is_some_and(|queue| !queue.entries.is_empty())
                            && queue_action(
                                ui,
                                "save-playlist",
                                "Save as playlist",
                                "Save queue as playlist",
                                128.0,
                                p,
                            )
                        {
                            *intent = Some(Intent::NamePlaylist);
                        }
                        if icon_button(
                            ui,
                            "close-queue",
                            "Close queue",
                            Icon::Clear,
                            (28.0, 28.0),
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
                                    *intent = Some(Query::new(value).map_or(
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
                    text(
                        ui,
                        "queue-notice",
                        player.message,
                        12,
                        p.secondary,
                        (Size::Grow, 28.0),
                        false,
                    );
                    rule(ui, "full-queue-divider", false, p);
                    let mut revision = DefaultHasher::new();
                    (
                        catalog.queue.as_ref().map(|queue| queue.revision),
                        std::sync::Arc::as_ptr(&catalog.library) as usize,
                        width.to_bits(),
                        player.artwork.map_or(0, crate::artwork::Images::revision),
                    )
                        .hash(&mut revision);
                    ui.view(
                        "full-queue-rows",
                        revision.finish(),
                        Size::Grow,
                        Size::Grow,
                        &mut |ui| {
                            queue_rows(ui, catalog, true, width - 42.0, player.artwork, p, intent)
                        },
                    );
                });
            },
        );
    });
    if !open {
        *intent = Some(Intent::CloseQueue);
    }
}

pub(super) fn queue_rows(
    ui: &mut Ui,
    catalog: &Catalog,
    full: bool,
    width: f32,
    images: Option<&crate::artwork::Images>,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let Some(queue) = &catalog.queue else { return };
    let entries = if full {
        &queue.entries[..]
    } else {
        queue.upcoming()
    };
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
            if full {
                queue.context.title()
            } else {
                &queue.heading
            },
            13,
            p.secondary,
            (Size::Grow, 28.0),
            false,
        );
        text(
            ui,
            "queue-count",
            if full { &queue.total } else { &queue.count },
            12,
            p.secondary,
            (Size::Px(56.0), 28.0),
            true,
        );
        if !full
            && icon_button(
                ui,
                "show-queue",
                "Show full queue",
                Icon::Queue,
                (28.0, 28.0),
                false,
                p,
            )
        {
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
    if entries.is_empty() {
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
        44.0,
        &mut |ui, index| {
            let entry = entries[index];
            let current = queue
                .current
                .and_then(|index| queue.entries.get(index))
                .map(|entry| entry.id)
                == Some(entry.id);
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
                        if full {
                            "localwave-full-queue-entry"
                        } else {
                            "localwave-queue-entry"
                        },
                    ),
                    entry.id as i64,
                ),
                Node {
                    w: Size::Grow,
                    h: Size::Px(44.0),
                    axis: Axis::Row,
                    flags: CLICKABLE,
                    role: Role::ListItem,
                    look: Look {
                        fill: if current { p.selected } else { p.paper },
                        ..Look::NONE
                    },
                    ..Node::DEFAULT
                },
            );
            ui.a11y_name(&item, title);
            if current {
                ui.set_role(&item, Role::ListItem, pxui::a11y::state::SELECTED);
            }
            ui.within(&item, &mut |ui| {
                let grip = ui.node(
                    "reorder-queued",
                    Node {
                        flags: CLICKABLE | TAB_STOP,
                        role: Role::Button,
                        w: Size::Px(24.0),
                        h: Size::Px(44.0),
                        ..Node::DEFAULT
                    },
                );
                ui.a11y_name(&grip, "Reorder queued track");
                ui.draw(&grip, move |paint, rect| {
                    for x in [8.0, 12.0] {
                        for y in [17.0, 21.0, 25.0] {
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
                    Size::Px(30.0),
                    Size::Px(30.0),
                    p.panel,
                );
                paint_artwork(ui, &art, row, 30.0, images);
                ui.a11y_name(
                    &art,
                    if current {
                        "Currently playing"
                    } else {
                        "Track artwork"
                    },
                );
                if current {
                    ui.draw(&art, move |paint, rect| {
                        for (x, height) in [(9.0, 8.0), (14.0, 14.0), (19.0, 10.0)] {
                            paint.fill(
                                rect.x + x,
                                rect.y + (30.0 - height) / 2.0,
                                2.0,
                                height,
                                p.ink,
                            );
                        }
                    });
                }
                let play = ui.node(
                    "play-queued",
                    Node {
                        flags: CLICKABLE | TAB_STOP,
                        role: Role::Button,
                        w: Size::Grow,
                        h: Size::Grow,
                        padding: Edges {
                            l: 10.0,
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
                        13,
                        p.ink,
                        (Size::Grow, 19.0),
                        false,
                    );
                    text(
                        ui,
                        "queue-artist",
                        artist,
                        12,
                        p.secondary,
                        (Size::Grow, 17.0),
                        false,
                    );
                });
                if ui.signal_state(&play).clicked {
                    *intent = Some(Intent::PlayQueue(entry.id));
                }
                if !current
                    && icon_button(
                        ui,
                        "remove-queued",
                        "Remove from queue",
                        Icon::Clear,
                        (28.0, 44.0),
                        false,
                        p,
                    )
                {
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
                let after = ui.signal_state(&item).pointer.1 >= ui.rect(&item).y + 22.0;
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
                        *intent = Some(if full {
                            Intent::MoveFullQueue { occurrence, before }
                        } else {
                            Intent::MoveQueue { occurrence, before }
                        });
                    }
                }
            }
        },
    );
    ui.a11y_name(
        &list,
        if full {
            "Full playback queue"
        } else {
            "Up next queue"
        },
    );
    ui.transfer_carry(&list);
}
