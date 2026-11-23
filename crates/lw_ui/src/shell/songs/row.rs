use super::*;
use crate::catalog::{Collection, Row, SongColumn as Column, SongTable};

#[derive(Clone, Copy)]
pub(super) struct Viewport {
    pub(super) leading: f32,
    pub(super) heading: f32,
    pub(super) offset: f32,
    pub(super) height: f32,
}

/// The equalizer bars' loop: forward and back, each bar started a phase later.
const EQUALIZER_LOOP: pxui::Keyframes = pxui::Keyframes::DEFAULT
    .repeat(pxui::Repeat::Forever)
    .autoreverse()
    .rate(crate::motion::PLAYBACK_RATE);
/// The playing glyph's blink loop, each half started a phase later.
const BLINK_LOOP: pxui::Keyframes = pxui::Keyframes::DEFAULT.repeat(pxui::Repeat::Forever);

pub(in crate::shell) fn equalizer(ui: &mut Ui, playing: bool, visible: bool, id: i64) {
    use pxui::{Keyframes, ui::Keyframe};
    let root = ui.node(
        "equalizer",
        Node::column().w_px(14.0).h_px(11.0).role(Role::Image),
    );
    ui.a11y_name(&root, if playing { "Playing" } else { "Paused" });
    let song = ui.node("equalizer-song", Node::column().w_px(0.0).h_px(0.0));
    let active = playing && visible && !ui.tokens().reduced_motion;
    let changed = ui.state(&root, -1) != i64::from(active) || ui.state(&song, i64::MIN) != id;
    ui.set_state(&root, i64::from(active));
    ui.set_state(&song, id);
    ui.within(&root, &mut |ui| {
        for (index, key) in ["bar-0", "bar-1", "bar-2"].into_iter().enumerate() {
            // keyed nodes, not `ui.id` scopes: each bar's position is looked up by its key
            let position = ui.node(
                key,
                Node::column()
                    .w_px(2.0)
                    .h_px(11.0)
                    .at(index as f32 * 4.0, 0.0),
            );
            ui.within(&position, &mut |ui| {
                let bar = ui.view_of("bar", &0, |ui| {
                    let fill = ui.node("fill", Node::column().w(Size::Grow).h(Size::Grow));
                    let accent = Rgba::token(ColorToken::ACCENT);
                    ui.canvas(&fill, move |paint, rect| {
                        paint.fill(rect, Fill::new(accent).fractional())
                    });
                });
                if changed {
                    let curve = Curve::Bezier {
                        duration: 0.45,
                        points: [0.42, 0.0, 0.58, 1.0],
                    };
                    if active {
                        ui.animate_keys(
                            &bar,
                            &[
                                Keyframe::scale(0.0, 1.0, 0.4).anchor(0.5, 1.0).curve(curve),
                                Keyframe::scale(0.45, 1.0, 1.0)
                                    .anchor(0.5, 1.0)
                                    .curve(curve),
                            ],
                            &EQUALIZER_LOOP.offset(index as f32 * 0.3),
                        );
                    } else {
                        ui.animate_keys(
                            &bar,
                            &[Keyframe::scale(0.0, 1.0, 0.4).anchor(0.5, 1.0).curve(curve)],
                            &Keyframes::DEFAULT,
                        );
                    }
                }
            });
        }
    });
}

pub(in crate::shell) fn availability(ui: &mut Ui, row: Row<'_>, visible: bool) {
    if row.blocked {
        // Swift's blocked glyph: a tertiary 1.5-point outline with a 2-point bar along its foot
        let root = ui.node(
            "availability",
            Node::stack().w_px(10.0).h_px(10.0).role(Role::Image),
        );
        ui.a11y_name(&root, "Source needs reconnecting");
        ui.canvas(&root, |paint, rect| {
            let ink = Fill::new(Rgba::token(TERTIARY)).fractional();
            let (x, y) = (rect.x, rect.y);
            paint.fill(Rect::new(x, y, 10.0, 1.5), ink);
            paint.fill(Rect::new(x, y + 1.5, 1.5, 7.0), ink);
            paint.fill(Rect::new(x + 8.5, y + 1.5, 1.5, 7.0), ink);
            paint.fill(Rect::new(x, y + 8.0, 10.0, 2.0), ink);
        });
        return ui.set_state(&root, -1);
    }
    glyph(ui, (row.state, row.availability, row.pinned), visible);
}

/// Swift's LWStateGlyph from its three inputs; Files rows share it.
pub(in crate::shell) fn glyph(
    ui: &mut Ui,
    (state, available, pinned): (lw_core::FileState, Option<lw_core::FileAvailability>, bool),
    visible: bool,
) {
    use lw_core::{FileAvailability, FileState};
    let root = ui.node(
        "availability",
        Node::stack().w_px(10.0).h_px(10.0).role(Role::Image),
    );
    let label = match state {
        FileState::CopyPending => "Preparing offline copy",
        FileState::Failed => "File unavailable — retry",
        _ => match available {
            Some(FileAvailability::OnDevice) if pinned => "Kept offline",
            Some(FileAvailability::OnDevice) => "On this device",
            Some(FileAvailability::InCloud) => "In cloud — downloads when played",
            Some(FileAvailability::Unknown) => "Availability unknown",
            None => "Checking availability",
        },
    };
    ui.a11y_name(&root, label);
    let accent = Rgba::token(ColorToken::ACCENT);
    ui.canvas(&root, move |paint, rect| {
        let (x, y) = (rect.x, rect.y);
        if available == Some(FileAvailability::OnDevice)
            && !matches!(state, FileState::CopyPending | FileState::Failed)
        {
            if pinned {
                paint.fill(
                    Rect::new(x - 3.5, y - 3.5, 17.0, 17.0),
                    Fill::new(Rgba::token(INK)).fractional(),
                );
                paint.fill(
                    Rect::new(x - 2.0, y - 2.0, 14.0, 14.0),
                    Fill::new(Rgba::token(PAPER)).fractional(),
                );
            }
            paint.fill(
                Rect::new(x, y, 10.0, 10.0),
                Fill::new(Rgba::token(INK)).fractional(),
            );
        } else {
            let unknown = available.is_none_or(|state| state == FileAvailability::Unknown)
                && !matches!(state, FileState::CopyPending | FileState::Failed);
            let ink = if matches!(state, FileState::CopyPending | FileState::Failed) {
                accent
            } else {
                Rgba::token(SECONDARY)
            };
            if unknown {
                for start in [0.75, 4.75, 8.75] {
                    let length = 2.0f32.min(9.25 - start);
                    paint.fill(
                        Rect::new(x + start, y + 0.25, length, 1.0),
                        Fill::new(Rgba::token(TERTIARY)).fractional(),
                    );
                    paint.fill(
                        Rect::new(x + start, y + 8.75, length, 1.0),
                        Fill::new(Rgba::token(TERTIARY)).fractional(),
                    );
                    paint.fill(
                        Rect::new(x + 0.25, y + start, 1.0, length),
                        Fill::new(Rgba::token(TERTIARY)).fractional(),
                    );
                    paint.fill(
                        Rect::new(x + 8.75, y + start, 1.0, length),
                        Fill::new(Rgba::token(TERTIARY)).fractional(),
                    );
                }
            } else {
                paint.fill(Rect::new(x, y, 10.0, 1.5), Fill::new(ink).fractional());
                paint.fill(
                    Rect::new(x, y + 8.5, 10.0, 1.5),
                    Fill::new(ink).fractional(),
                );
                paint.fill(Rect::new(x, y + 1.5, 1.5, 7.0), Fill::new(ink).fractional());
                paint.fill(
                    Rect::new(x + 8.5, y + 1.5, 1.5, 7.0),
                    Fill::new(ink).fractional(),
                );
            }
            if state == FileState::Failed {
                paint.line((x + 1.0, y + 9.0), (x + 9.0, y + 1.0), Stroke::new(accent));
            }
        }
    });
    if state == FileState::CopyPending {
        use pxui::{Keyframes, ui::Keyframe};
        let active = visible && !ui.tokens().reduced_motion;
        let changed = ui.state(&root, -1) != i64::from(active);
        ui.set_state(&root, i64::from(active));
        ui.within(&root, &mut |ui| {
            for (index, key) in ["left", "right"].into_iter().enumerate() {
                let half = ui.view_of(key, &0, |ui| {
                    let fill = ui.node("fill", Node::column().w(Size::Grow).h(Size::Grow));
                    let accent = Rgba::token(ColorToken::ACCENT);
                    ui.canvas(&fill, move |paint, rect| {
                        paint.fill(
                            Rect::new(rect.x + index as f32 * 5.0, rect.y, 5.0, 10.0),
                            Fill::new(accent).fractional(),
                        )
                    });
                });
                if changed {
                    let curve = crate::motion::steps(1, 0.5);
                    if active {
                        ui.animate_keys(
                            &half,
                            &[
                                Keyframe::opacity(0.0, 1.0).curve(curve),
                                Keyframe::opacity(0.5, 0.0).curve(curve),
                                Keyframe::opacity(1.0, 1.0).curve(curve),
                            ],
                            &BLINK_LOOP.offset(index as f32 * 0.5),
                        );
                    } else {
                        ui.animate_keys(
                            &half,
                            &[Keyframe::opacity(0.0, if index == 0 { 1.0 } else { 0.0 })
                                .curve(curve)],
                            &Keyframes::DEFAULT,
                        );
                    }
                }
            }
        });
    } else {
        ui.set_state(&root, -1);
    }
}

pub(super) struct Rows<'a> {
    pub(super) catalog: &'a Catalog,
    pub(super) collection: &'a Collection,
    pub(super) table: &'a SongTable,
    pub(super) state: &'a ShellState,
    pub(super) player: &'a PlayerView<'a>,
    pub(super) widths: [f32; 8],
    pub(super) density: f32,
    pub(super) revision: u64,
    pub(super) numbered: bool,
    pub(super) entry: &'a crate::motion::Page,
    pub(super) viewport: Viewport,
}
/// A song row's actions key: 32 points, the secondary ink, no wash.
const ROW_MORE: pxui::IconButton = pxui::IconButton::sized(32.0, 32.0);
/// Carry kind of a row drag; the target resolves the selection from the song id.
pub(crate) const SONG_DRAG: &str = "localwave.song:";

impl Rows<'_> {
    /// Rows of a list in saved order can be dragged to reorder (not while sorted).
    pub(super) fn reorderable(&self) -> bool {
        matches!(self.state.route, Route::Collection(_))
            && matches!(
                &*self.collection.context,
                lw_core::context::PlaybackContext::Playlist { .. }
            )
            && self.state.sort == Sort::Original
            && self.state.ascending
    }
    /// Row `index`'s cells inside the row the list's `SONG_ROW` body shapes (its look follows `cell`'s state): the columns' text, the
    /// playing and file-state marks, the actions key and, in a reorderable list, the grip (the list's handle). The row is named by its
    /// title; a held row carries its song.
    pub(super) fn row(
        &self,
        ui: &mut Ui,
        index: usize,
        cell: pxui::CellState,
        intent: &mut Option<Intent>,
    ) {
        let Self {
            catalog,
            collection,
            table,
            state,
            player,
            widths,
            density,
            revision,
            numbered,
            entry,
            viewport,
        } = *self;
        let row = catalog.row(collection.rows[index]);
        let reorder = self.reorderable() && widths[Column::Number as usize] >= 32.0;
        let current = player.current_id == Some(row.id);
        let revision = crate::hash((revision, current));
        let number = table.number_label(index);
        let measure = |ui: &mut Ui| {
            row_layout(
                ui,
                catalog,
                (collection.rows[index], &row),
                (index + 1, number),
                numbered,
                (widths, density),
                current,
            )
        };
        let active_mark = current || row.state == lw_core::FileState::CopyPending;
        let laid_out = (active_mark || reorder).then(|| measure(ui));
        let visible = active_mark && {
            let layout = laid_out.unwrap();
            let y = cell.y - viewport.offset + layout.y(11.0);
            y + 11.0 > (viewport.leading - viewport.offset).max(0.0) + viewport.heading
                && y < viewport.height
        };
        let visual_revision = if active_mark {
            // its marks pause out of view: the list rebuilds the row every frame (`ListState::LIVE`, set from this next frame)
            state.song_live.borrow_mut().1.push(index);
            crate::hash((revision, current && player.playing, visible))
        } else {
            revision
        };
        ui.a11y_name(&cell.handle, row.title);
        if cell.held {
            let mut digits = [0u8; 24];
            let id = stack_fmt(&mut digits, format_args!("{}", row.id)).as_bytes();
            let mut kind = [0u8; 48];
            kind[..SONG_DRAG.len()].copy_from_slice(SONG_DRAG.as_bytes());
            let len = SONG_DRAG.len() + id.len();
            kind[SONG_DRAG.len()..len].copy_from_slice(id);
            ui.carry(
                Some(&cell.handle),
                std::str::from_utf8(&kind[..len]).expect("song drag kind is ASCII"),
                row.title,
            );
        }
        let visual = ui.view_of("song-visual", &visual_revision, |ui| {
            if visual_revision == revision {
                ui.retain_palette_paint();
            }
            let layout = laid_out.unwrap_or_else(|| measure(ui));
            let copy = |ui: &mut Ui| {
                let mut x = 0.0;
                for column in Column::ALL {
                    let w = widths[column as usize];
                    if w == 0.0 {
                        continue;
                    }
                    if !(column == Column::Number && current)
                        && !matches!(column, Column::State | Column::Actions)
                    {
                        let value = body_value(&row, column, numbered, number);
                        let (fit, h) = layout.sizes[column as usize];
                        let y = layout.y(baseline(column));
                        let left = if column == Column::Number { 32.0 } else { 8.0 };
                        ui.label_at(
                            [
                                "#", "title", "artist", "album", "format", "state", "time",
                                "actions",
                            ][column as usize]
                                .into(),
                            Rect {
                                x: x + if column == Column::Time {
                                    w - 8.0 - fit
                                } else {
                                    left
                                },
                                y,
                                w: fit.max(0.0),
                                h,
                            },
                            value,
                            body_label(column, current).color(if column == Column::Title {
                                INK
                            } else {
                                SECONDARY
                            }),
                        )
                        .unwrap();
                    }
                    x += w;
                }
            };
            if visual_revision != revision {
                ui.view_of("song-copy", &revision, |ui| {
                    copy(ui);
                    ui.retain_palette_labels();
                });
            } else {
                copy(ui);
            }
            if current {
                let bars = ui.view_of("equalizer-view", &visual_revision, |ui| {
                    equalizer(ui, player.playing, visible, row.id);
                });
                ui.set_node_layout(
                    &bars,
                    &Node::column()
                        .w_px(14.0)
                        .h_px(11.0)
                        .at(32.0, layout.y(11.0)),
                );
            }
            if widths[Column::State as usize] > 0.0 {
                let x = widths[..Column::State as usize].iter().sum::<f32>() + 8.0;
                let pending = row.state == lw_core::FileState::CopyPending;
                let glyph = (
                    row.blocked,
                    row.state as u8,
                    row.availability.map(|value| value as u8),
                    row.pinned,
                    pending && visible,
                );
                let mark = ui.view_of("availability-view", &glyph, |ui| {
                    // The glyph is always 10 points. The column layout below positions the retained mark.
                    ui.retain_view_on_resize();
                    if !pending {
                        ui.retain_palette_paint();
                    }
                    availability(ui, row, visible);
                });
                ui.set_node_layout(
                    &mark,
                    &Node::column().w_px(10.0).h_px(10.0).at(x, layout.y(10.0)),
                );
                let previous = ui.state(&mark, -1);
                let next = row.state as i64;
                if previous != next {
                    ui.set_state(&mark, next);
                    if previous >= 0 && !ui.tokens().reduced_motion {
                        ui.animate_keys(
                            &mark,
                            &[
                                Keyframe::scale(0.0, 0.3, 0.3)
                                    .curve(crate::motion::spring(crate::motion::BASE)),
                                Keyframe::scale(crate::motion::BASE, 1.0, 1.0)
                                    .curve(crate::motion::spring(crate::motion::BASE)),
                            ],
                            &Keyframes::DEFAULT,
                        );
                    }
                }
            }
            let x = widths[..7].iter().sum::<f32>();
            {
                // placed itself: a positioning column per row is a node per fresh row
                let more = ui.icon_button(
                    "actions",
                    icon_drawing(Icon::RowMore),
                    row.actions_label,
                    &ROW_MORE,
                );
                ui.set_at(&more.handle, (x, (layout.height - 32.0) * 0.5));
                if more.clicked {
                    let at = ui.rect(&more.handle);
                    *intent = Some(Intent::SongMenu(
                        index,
                        row.id,
                        collection.key,
                        (at.x, at.y + at.h),
                    ));
                }
            }
            // The placed cells stay out of flow. The separator also gives the measured row its height.
            let separator = ui.node(
                "separator",
                Node::column().w(Size::Grow).h_px(layout.height),
            );
            let last = index + 1 == collection.rows.len();
            ui.canvas(&separator, move |paint, rect| {
                paint.fill(
                    Rect::new(rect.x, rect.y.floor(), rect.w, 1.0),
                    Fill::new(Rgba::token(LINE)).fractional(),
                );
                if last {
                    paint.fill(
                        Rect::new(rect.x, (rect.y + rect.h).floor(), rect.w, 1.0),
                        Fill::new(Rgba::token(LINE)).fractional(),
                    );
                }
            });
        });
        if reorder {
            // the list's handle over the number column: a press lifts the row at once (touch too) and the list reports the move;
            // Swift's desktop rows draw no grip
            ui.id("grip", |ui| {
                let grip = ui.part(
                    pxui::Part::HANDLE,
                    Node::column()
                        .w_px(24.0)
                        .h_px(laid_out.unwrap().height)
                        .at(2.0, 0.0),
                );
                ui.a11y_name(&grip, "Reorder track");
            });
        }
        entry.row(ui, &visual, index);
    }
}
