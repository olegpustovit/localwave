use super::*;
use crate::browse::Query;

/// The playlist name input of the queue sheet: Done ends editing.
const PLAYLIST_NAME: pxui::TextField = pxui::TextField::DEFAULT
    .action(pxui::ReturnKey::Done)
    .name("Playlist name");
use crate::catalog::Catalog;
use crate::{Intent, PlayerView};
/// A queue sheet text action (`localwave-button` / `ROW`): secondary 12-point text from its start, its line 2 points below the
/// key's centre (where Swift's text sits); its width is set where it is built.
const QUEUE: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).align(Justify::Start).pad(Edges::new(0.0, 4.0, 0.0, 0.0)).label(TEXT_12.color(SECONDARY));
/// A queue text action `w` points wide, 28 high, named `name`.
const fn queue_action(w: f32, name: &'static str) -> pxui::Button<'static> {
    QUEUE.size(Size::Px(w), Size::Px(28.0)).name(name)
}
/// The sheet's titles (`localwave-button` / `ROW`, a column): "Queue" in 20-point ink over the worker's "From …" line.
const TITLES: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Grow, Size::Grow).align(Justify::Start).axis(Axis::Column)
    .label(TEXT_20.grow().color(INK)).detail_label(CAPTION.grow().h_px(18.2)).name("Save queue as playlist");
/// Clear up next (`localwave-button` / `ROW`): a 52-point row, its 16-point secondary text 26 in.
const CLEAR_UP_NEXT: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Grow, Size::Px(52.0)).align(Justify::Start)
    .pad(Edges::new(26.0, 0.0, 0.0, 0.0)).label(TEXT_16.grow().color(SECONDARY));
/// The queue sheet: a bottom modal of the content area's width, dismissed by a downward drag with UIKit's thresholds.
const QUEUE_SHEET: Overlay = Overlay::DEFAULT.present(pxui::Presentation::Bottom).dismiss(pxui::DismissOpts::DEFAULT).name("Playback queue");

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
/// from the snapshot so the sheet needs no measuring pass; shell() draws the dim behind it.
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
    let cap = (content.h * 0.86 - 33.0).max(0.0);
    let height = wanted.min(cap);
    let width = content.w;
    let open = ui.modal("queue-sheet", &QUEUE_SHEET.max_h_px(cap), |ui| {
        ui.column(Node::column().w(Size::Grow).h_px(height), |ui| {
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
                        let empty = queue.is_none_or(|queue| queue.entries.is_empty());
                        let titles = ui.button("queue-titles", "Queue", &TITLES.detail(player.queue_from).disabled(empty));
                        if titles.clicked || titles.secondary {
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
                        if ui.button("create-playlist", "Create", &queue_action(60.0, "Create playlist from queue").disabled(!valid)).clicked {
                            *intent = Some(Intent::SaveQueue(state.playlist_name));
                        }
                        if ui.button("cancel-playlist", "Cancel", &queue_action(60.0, "Cancel playlist creation")).clicked {
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
                            if undo && ui.button("undo", "Undo", &queue_action(44.0, "Undo clear up next")).clicked {
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
                    if ui.button("clear", "Clear up next", &CLEAR_UP_NEXT).clicked {
                        *intent = Some(Intent::ClearQueue);
                    }
                }
                spacer(ui, "queue-bottom", 20.0);
        });
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
            if !queue.upcoming().is_empty() && ui.button("clear", "Clear", &queue_action(44.0, "Clear up next")).clicked {
                *intent = Some(Intent::ClearQueue);
            }
            if queue.can_undo && ui.button("undo", "Undo", &queue_action(44.0, "Undo clear up next")).clicked {
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
    // a row is keyed by its occurrence, so it keeps its identity when the queue reorders
    let rows = SectionList::new(entries.len()).row_h(m.row).reorder(true);
    let list = ui.section_list("queue-entries", &rows, &|index| entries[index].id as i64, &mut |ui, part, cell| {
        let SectionPart::Row(index) = part else { return };
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
        ui.a11y_name(&cell.handle, title);
        ui.row(Node::row().w(Size::Grow).h_px(m.row).gap(m.gap).align(Align::Center).padding(Edges::ZERO.l(m.lead).r(m.trail)).look(PAPER_FILL), |ui| {
            // the list's handle: a press lifts the row at once (touch too) and the list reports the move
            let grip = ui.part(pxui::Part::HANDLE, Node::column().w_px(m.grip).h_px(m.row));
            ui.a11y_name(&grip, "Reorder queued track");
            ui.canvas(&grip, move |paint, rect| {
                for x in [m.grip / 2.0 - 4.0, m.grip / 2.0] {
                    for y in [m.row / 2.0 - 5.0, m.row / 2.0 - 1.0, m.row / 2.0 + 3.0] {
                        paint.fill(Rect::new(rect.x + x, rect.y + y, 2.0, 2.0), Fill::new(Rgba::token(SECONDARY)));
                    }
                }
            });
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
    });
    // `to` is the moved row's final index in `entries`' pre-move order: the occurrence after it is old `to + 1` moving down, `to` moving up
    if let Some((from, to)) = list.moved {
        *intent = Some(Intent::MoveQueue { occurrence: entries[from].id, before: entries.get(to + usize::from(to > from)).map(|entry| entry.id) });
    }
    ui.a11y_name(
        &list.handle,
        if sheet {
            "Full playback queue"
        } else {
            "Up next queue"
        },
    );
}
