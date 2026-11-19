use super::*;
use crate::browse::Query;

/// The playlist name input of the queue sheet: Done ends editing.
const PLAYLIST_NAME: pxui::TextField = pxui::TextField::DEFAULT
    .action(pxui::ReturnKey::Done)
    .name("Playlist name");
use crate::catalog::Catalog;
use crate::{Intent, PlayerView};
/// A queue sheet text action with no look of its own; its width is set where it is built.
const QUEUE_ACTION: pxui::Button = pxui::Button::DEFAULT.centered().look(&BARE);

pub(super) fn queue_action(
    ui: &mut Ui,
    key: &'static str,
    label: &str,
    name: &'static str,
    width: f32,
) -> bool {
    // The text starts at the left edge as Swift's does; `button_with` centres it, so it sits on a node of its own.
    let action = QUEUE_ACTION.size(Size::Px(width), Size::Px(28.0)).name(name);
    ui.button_content(key, &action, |ui, _| {
        let copy = ui.node("label", Node::column().w(Size::Grow).h(Size::Grow).look(QUEUE_ACTION_TEXT));
        ui.text(&copy, label);
    })
    .clicked
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
/// Drag down to dismiss with UIKit's sheet thresholds.
const DISMISS: pxui::GestureOpts = pxui::GestureOpts::DEFAULT.dismiss(pxui::DismissOpts::DEFAULT);

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
    let open = ui.popup("queue-sheet", &Overlay::DEFAULT.at(pxui::Point::new(content.x, content.y + content.h - height + state.sheet_dismiss.get())).w_px(width).h_px(height).max_h_px(height).name("Playback queue"), |ui| {
            let sheet_node = Node::column().w_px(width).h_px(height).role(Role::Dialog).flags(Flags::CLIP).look(PAPER_FILL);
            // The sheet takes the pointer for its dismiss gesture but is no tab stop.
            let sheet = ui.pressable("queue-sheet", sheet_node, &PLAIN.tab_stop(false), |ui, _| {
                let handle = container(
                    ui,
                    "queue-handle",
                    Axis::Stack,
                    Size::Grow,
                    Size::Px(21.0),
                    &PAPER_FILL,
                );
                ui.canvas(&handle, move |paint, rect| {
                    paint.fill(Rect::new(rect.x + (rect.w - 36.0) * 0.5, rect.y + 8.0, 36.0, 5.0), Fill::new(Rgba::token(LINE)));
                });
                ui.row(Node::row().w(Size::Grow).h_px(66.2).padding(Edges::new(16.0, 12.0, 6.0, 8.0)),
                    |ui| {
                        // Swift reaches Save as list from this header's context menu and an a11y action.
                        let node = Node::column().w(Size::Grow).h(Size::Grow).role(Role::Button);
                        let titles = ui.pressable("queue-titles", node, &PLAIN, |ui, _| {
                            ui.label("Queue", TEXT_20.grow().color(INK));
                            let from = container(
                                ui,
                                "queue-from",
                                Axis::Row,
                                Size::Grow,
                                Size::Px(18.2),
                                &PAPER_FILL,
                            );
                            ui.within(&from, &mut |ui| {
                                ui.label("From ", CAPTION);
                                ui.label(player.context, CAPTION);
                                if player.shuffled {
                                    ui.label(" · shuffled", CAPTION);
                                }
                            });
                        });
                        ui.a11y_name(&titles.handle, "Save queue as playlist");
                        if queue.is_some_and(|queue| !queue.entries.is_empty()) && (titles.clicked || titles.secondary) {
                            *intent = Some(Intent::NamePlaylist);
                        }
                        if ui.icon_button("close-queue", icon_drawing(Icon::Clear), "Close queue", &ICON_44.look(&ICON_PLATE_LOOKS)).clicked {
                            *intent = Some(Intent::CloseQueue);
                        }
                    },
                );
                if state.playlist_editing {
                    ui.label("Save all tracks in queue order; duplicates are included once.", TEXT_12.grow().h_px(32.0).color(SECONDARY));
                    let row = container(
                        ui,
                        "playlist-editor",
                        Axis::Row,
                        Size::Grow,
                        Size::Px(44.0),
                        &PAPER_FILL,
                    );
                    ui.within(&row, &mut |ui| {
                        let field = ui.node("playlist-name-field", Node::DEFAULT.w(Size::Grow).h(Size::Grow));
                        ui.within(&field, &mut |ui| {
                            if let Some(value) = ui
                                .text_field("playlist-name", state.playlist_name.as_str(), &PLAYLIST_NAME)
                                .changed
                            {
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
                            ) {
                                *intent = Some(Intent::SaveQueue(state.playlist_name));
                            }
                        } else {
                            ui.label("Create", TEXT_12.w_px(60.0).h_px(28.0).color(SECONDARY));
                        }
                        if queue_action(
                            ui,
                            "cancel-playlist",
                            "Cancel",
                            "Cancel playlist creation",
                            60.0,
                        ) {
                            *intent = Some(Intent::CancelPlaylistName);
                        }
                    });
                    ui.label(if state.playlist_name_too_long { "Name exceeds 1,024 UTF-8 bytes. Shorten it to save." } else { "" }, TEXT_12.grow().h_px(24.0).color(SECONDARY));
                }
                if notice {
                    ui.label(player.message, TEXT_12.grow().h_px(28.0).color(SECONDARY));
                }
                if let Some(song) = now {
                    now_row(ui, player, song, width);
                }
                ui.row(Node::row().w(Size::Grow).h_px(46.2).gap(4.0).padding(Edges::ZERO.l(16.0).t(20.0).b(8.0)),
                    |ui| {
                        ui.label("Up next", CAPTION);
                        ui.label(queue.map_or("0", |queue| queue.count.as_str()), MONO_12.h_px(18.2).color(SECONDARY));
                    },
                );
                if upcoming == 0 {
                    ui.id("queue-empty", |ui| { ui.row(Node::row().w(Size::Grow).h_px(96.0).gap(8.0).align(Align::Center).padding(Edges::ZERO.l(26.0)),
                        |ui| {
                            let undo = queue.is_some_and(|queue| queue.can_undo);
                            ui.label(if undo { "Up next cleared." } else { "Nothing queued." }, TEXT_14.h_px(20.0).color(SECONDARY));
                            if undo && queue_action(ui, "undo", "Undo", "Undo clear up next", 44.0) {
                                *intent = Some(Intent::UndoQueue);
                            }
                        },
                    ); });
                } else {
                    ui.id("full-queue-body", |ui| { ui.column(Node::column().w(Size::Grow).h(Size::Grow).pad_x(10.0), |ui| {
                        ui.view_of(
                            "full-queue-rows",
                            &(
                                queue.map(|queue| queue.revision),
                                std::sync::Arc::as_ptr(&catalog.library) as usize,
                                width.to_bits(),
                                player.artwork.map_or(0, crate::artwork::Images::revision),
                            ),
                            |ui| {
                                queue_rows(
                                    ui,
                                    catalog,
                                    true,
                                    player.artwork,
                                    intent,
                                )
                            },
                        );
                    }); });
                    let node = Node::row().w(Size::Grow).h_px(52.0).align(Align::Center).role(Role::Button).padding(Edges::ZERO.l(26.0));
                    let clear = ui.pressable("clear", node, &PLAIN, |ui, _| {
                        ui.label("Clear up next", TEXT_16.grow().color(SECONDARY));
                    });
                    ui.a11y_name(&clear.handle, "Clear up next");
                    if clear.clicked {
                        *intent = Some(Intent::ClearQueue);
                    }
                }
                spacer(ui, "queue-bottom", 20.0);
            });
            let drag = ui.gesture(&sheet.handle, &DISMISS);
            // The popup itself follows the finger: moving only the node inside it loses the touch.
            state.sheet_dismiss.set(drag.dismiss);
            if drag.dismiss_committed {
                *intent = Some(Intent::CloseQueue);
            }
            ui.set_role(&sheet.handle, Role::Dialog, pxui::A11yStates::MODAL);
            ui.a11y_name(&sheet.handle, "Playback queue");
        }).open;
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
) {
    let row = ui.node(
        "queue-now",
        Node::row().w(Size::Grow).h_px(56.0).gap(10.0).align(Align::Center).role(Role::ListItem).padding(Edges::ZERO.l(24.0).r(16.0)),
    );
    ui.a11y_name(&row, song.title);
    ui.within(&row, &mut |ui| {
        let art = container(
            ui,
            "now-art",
            Axis::Stack,
            Size::Px(36.0),
            Size::Px(36.0),
            &PANEL_FILL,
        );
        paint_artwork(ui, &art, Some(song), 36.0, player.artwork);
        ui.column(Node::column().w_px((width - 134.0).max(0.0)).h(Size::Fit), |ui| {
            ui.label(song.title, LEAD_INK.grow());
            let line = container(ui, "line", Axis::Row, Size::Grow, Size::Px(18.2), &PAPER_FILL);
            ui.within(&line, &mut |ui| {
                ui.label("Now · ", CAPTION);
                ui.label(song.artist, CAPTION.grow());
            });
        });
        ui.column(Node::column().w_px(14.0).h_px(11.0), |ui| {
            ui.view_of(
                "equalizer-view",
                &(player.playing, song.id),
                |ui| super::songs::row::equalizer(ui, player.playing, true, song.id),
            );
        });
        spacer(ui, "now-trail", 14.0);
    });
}

pub(super) fn queue_rows(
    ui: &mut Ui,
    catalog: &Catalog,
    sheet: bool,
    images: Option<&crate::artwork::Images>,
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
            &PAPER_FILL,
        );
        ui.within(&header, &mut |ui| {
            ui.label(&queue.heading, TEXT_13.grow().h_px(28.0).color(SECONDARY));
            ui.label(&queue.count, MONO_12.w_px(56.0).h_px(28.0).color(SECONDARY));
            if ui.icon_button("show-queue", icon_drawing(Icon::Queue), "Show full queue", &ICON_28.look(&ICON_PLATE_LOOKS)).clicked {
                *intent = Some(Intent::ShowQueue);
            }
            if !queue.upcoming().is_empty()
                && queue_action(ui, "clear", "Clear", "Clear up next", 44.0)
            {
                *intent = Some(Intent::ClearQueue);
            }
            if queue.can_undo && queue_action(ui, "undo", "Undo", "Undo clear up next", 44.0) {
                *intent = Some(Intent::UndoQueue);
            }
        });
    }
    if entries.is_empty() {
        if sheet {
            return;
        }
        ui.label(if queue.can_undo { "Up next cleared." } else { "Queue is empty." }, TEXT_14.grow().h_px(44.0).color(SECONDARY));
        return;
    }
    let list = ui.list(
        "queue-entries",
        entries.len(),
        m.row,
        // a row is keyed by its occurrence, so it keeps its identity when the queue reorders
        &|index| entries[index].id as i64,
        |ui, index| {
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
            let item_node = Node::row().w(Size::Grow).h_px(m.row).gap(m.gap).align(Align::Center).role(Role::ListItem).padding(Edges::ZERO.l(m.lead).r(m.trail)).look(PAPER_FILL);
            // The row takes the pointer for drops and drags but is no tab stop; its own controls are.
            let item = ui.pressable(Key::Id(entry.id as i64), item_node, &PLAIN.tab_stop(false), |ui, _| {
                let grip = ui.pressable("reorder-queued", Node::column().w_px(m.grip).h_px(m.row).role(Role::Button), &PLAIN, |_, _| {});
                ui.a11y_name(&grip.handle, "Reorder queued track");
                ui.canvas(&grip.handle, move |paint, rect| {
                    for x in [m.grip / 2.0 - 4.0, m.grip / 2.0] {
                        for y in [m.row / 2.0 - 5.0, m.row / 2.0 - 1.0, m.row / 2.0 + 3.0] {
                            paint.fill(Rect::new(rect.x + x, rect.y + y, 2.0, 2.0), Fill::new(Rgba::token(SECONDARY)));
                        }
                    }
                });
                if grip.held || grip.clicked {
                    let mut digits = [0u8; 24];
                    let id = stack_fmt(&mut digits, format_args!("{}", entry.id)).as_bytes();
                    const PREFIX: &[u8] = b"localwave.queue-occurrence:";
                    let mut kind = [0u8; 64];
                    kind[..PREFIX.len()].copy_from_slice(PREFIX);
                    let len = PREFIX.len() + id.len();
                    kind[PREFIX.len()..len].copy_from_slice(id);
                    ui.carry(
                        if grip.clicked { None } else { Some(&grip.handle) },
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
                    &PANEL_FILL,
                );
                paint_artwork(ui, &art, row, m.art, images);
                ui.a11y_name(&art, "Track artwork");
                let node = Node::column().w(Size::Grow).h(Size::Grow).role(Role::Button).padding(Edges::ZERO.l(10.0 - m.gap).t((m.row - m.title_line - m.artist_line) / 2.0));
                let play = ui.pressable("play-queued", node, &PLAIN, |ui, _| {
                    ui.label(title, line_style(m.title as f32).grow().h_px(m.title_line).color(INK));
                    ui.label(artist, line_style(m.artist as f32).grow().h_px(m.artist_line).color(SECONDARY));
                });
                ui.a11y_name(&play.handle, title);
                if play.clicked {
                    *intent = Some(Intent::PlayQueue(entry.id));
                }
                // Swift marks songs that are not on this device, or kept offline.
                if let Some(row) =
                    row.filter(|row| row.state != lw_core::FileState::Copied || row.pinned)
                {
                    ui.column(Node::column().w_px(10.0 + 2.0 * m.glyph).h_px(10.0).padding(Edges::ZERO.l(m.glyph).r(m.glyph)),
                        |ui| {
                            super::songs::row::availability(ui, row, true)
                        },
                    );
                }
                if ui.icon_button(
                    "remove-queued",
                    icon_drawing(Icon::Clear),
                    "Remove from queue",
                    &icon_key(m.remove, m.row).look(&ICON_PLATE_LOOKS),
                ).clicked {
                    *intent = Some(Intent::RemoveQueue(entry.id));
                }
            });
            ui.a11y_name(&item.handle, title);
            let item = item.handle;
            let drop = ui.drop_target(&item).and_then(|drop| {
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
                let after = ui.signal(&item).pointer.1 >= ui.rect(&item).y + m.row / 2.0;
                let before = if after {
                    entries.get(index + 1).map(|entry| entry.id)
                } else {
                    Some(entry.id)
                };
                if before != Some(occurrence) {
                    ui.canvas(&item, move |paint, rect| {
                        paint.fill(Rect::new(rect.x, if after { rect.y + rect.h - 2.0 } else { rect.y }, rect.w, 2.0), Fill::new(Rgba::token(INK)));
                    });
                    if dropped {
                        *intent = Some(Intent::MoveQueue { occurrence, before });
                    }
                }
            }
        },
    );
    let list = list.handle;
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
