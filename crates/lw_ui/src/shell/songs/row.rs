use super::*;
use crate::catalog::{Collection, Row, SongColumn as Column};

#[derive(Clone, Copy)]
pub(super) struct Viewport {
    pub(super) leading: f32,
    pub(super) heading: f32,
    pub(super) offset: f32,
    pub(super) height: f32,
}

/// The equalizer bars' loop: forward and back, each bar started a phase later.
const EQUALIZER_LOOP: pxui::Keyframes = pxui::Keyframes::DEFAULT.repeat(pxui::Repeat::Forever).autoreverse();
/// The playing glyph's blink loop, each half started a phase later.
const BLINK_LOOP: pxui::Keyframes = pxui::Keyframes::DEFAULT.repeat(pxui::Repeat::Forever);

pub(in crate::shell) fn equalizer(ui: &mut Ui, playing: bool, visible: bool, id: i64) {
    use pxui::{ui::Keyframe, Keyframes};
    let root = ui.node("equalizer".into(), Node::column().w_px(14.0).h_px(11.0).role(Role::Image));
    ui.a11y_name(&root, if playing { "Playing" } else { "Paused" });
    let song = ui.node("equalizer-song".into(), Node::column().w_px(0.0).h_px(0.0));
    let active = playing && visible && !ui.tokens().reduced_motion;
    let changed = ui.state(&root, -1) != i64::from(active) || ui.state(&song, i64::MIN) != id;
    ui.set_state(&root, i64::from(active));
    ui.set_state(&song, id);
    ui.within(&root, &mut |ui| {
        for (index, key) in ["bar-0", "bar-1", "bar-2"].into_iter().enumerate() {
            ui.column(
                key,
                Node {
                    at: Some((index as f32 * 4.0, 0.0)).into(),
                    ..Node::column().w_px(2.0).h_px(11.0)
                },
                |ui| {
                    let bar = ui.view("bar", 0, Size::Grow, Size::Grow, &mut |ui| {
                        let fill = ui.node("fill".into(), Node::column().w(Size::Grow).h(Size::Grow));
                        let accent = ui.tokens().accent;
                        ui.draw(&fill, move |paint, rect| {
                            paint.fill_fractional(rect.x, rect.y, rect.w, rect.h, accent)
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
                                    Keyframe::scale(0.45, 1.0, 1.0).anchor(0.5, 1.0).curve(curve),
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
                },
            );
        }
    });
}

pub(in crate::shell) fn availability(ui: &mut Ui, row: Row<'_>, visible: bool) {
    glyph(ui, (row.state, row.availability, row.pinned), visible);
}

/// Swift's LWStateGlyph from its three inputs; Files rows share it.
pub(in crate::shell) fn glyph(
    ui: &mut Ui,
    (state, available, pinned): (lw_core::FileState, Option<lw_core::FileAvailability>, bool),
    visible: bool,
) {
    use lw_core::{FileAvailability, FileState};
    let root = ui.node("availability".into(), Node::stack().w_px(10.0).h_px(10.0).role(Role::Image));
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
    let accent = ui.tokens().accent;
    ui.draw(&root, move |paint, rect| {
        let (x, y) = (rect.x, rect.y);
        if available == Some(FileAvailability::OnDevice)
            && !matches!(state, FileState::CopyPending | FileState::Failed)
        {
            if pinned {
                paint.fill_fractional(x - 3.5, y - 3.5, 17.0, 17.0, paint.color(INK));
                paint.fill_fractional(x - 2.0, y - 2.0, 14.0, 14.0, paint.color(PAPER));
            }
            paint.fill_fractional(x, y, 10.0, 10.0, paint.color(INK));
        } else {
            let unknown = available.is_none_or(|state| state == FileAvailability::Unknown)
                && !matches!(state, FileState::CopyPending | FileState::Failed);
            let ink = if matches!(state, FileState::CopyPending | FileState::Failed) {
                accent
            } else {
                paint.color(SECONDARY)
            };
            if unknown {
                for start in [0.75, 4.75, 8.75] {
                    let length = 2.0f32.min(9.25 - start);
                    paint.fill_fractional(x + start, y + 0.25, length, 1.0, paint.color(TERTIARY));
                    paint.fill_fractional(x + start, y + 8.75, length, 1.0, paint.color(TERTIARY));
                    paint.fill_fractional(x + 0.25, y + start, 1.0, length, paint.color(TERTIARY));
                    paint.fill_fractional(x + 8.75, y + start, 1.0, length, paint.color(TERTIARY));
                }
            } else {
                paint.fill_fractional(x, y, 10.0, 1.5, ink);
                paint.fill_fractional(x, y + 8.5, 10.0, 1.5, ink);
                paint.fill_fractional(x, y + 1.5, 1.5, 7.0, ink);
                paint.fill_fractional(x + 8.5, y + 1.5, 1.5, 7.0, ink);
            }
            if state == FileState::Failed {
                paint.line((x + 1.0, y + 9.0), (x + 9.0, y + 1.0), accent);
            }
        }
    });
    if state == FileState::CopyPending {
        use pxui::{ui::Keyframe, Keyframes};
        let active = visible && !ui.tokens().reduced_motion;
        let changed = ui.state(&root, -1) != i64::from(active);
        ui.set_state(&root, i64::from(active));
        ui.within(&root, &mut |ui| {
            for (index, key) in ["left", "right"].into_iter().enumerate() {
                let half = ui.view(key, 0, Size::Grow, Size::Grow, &mut |ui| {
                    let fill = ui.node("fill".into(), Node::column().w(Size::Grow).h(Size::Grow));
                    let accent = ui.tokens().accent;
                    ui.draw(&fill, move |paint, rect| {
                        paint.fill_fractional(
                            rect.x + index as f32 * 5.0,
                            rect.y,
                            5.0,
                            10.0,
                            accent,
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
                            &[Keyframe::opacity(0.0, if index == 0 { 1.0 } else { 0.0 }).curve(curve)],
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
    pub(super) state: &'a ShellState,
    pub(super) player: &'a PlayerView<'a>,
    pub(super) widths: [f32; 8],
    pub(super) revision: u64,
    pub(super) numbered: bool,
    pub(super) entry: &'a crate::motion::Page,
    pub(super) viewport: Viewport,
}
pub(in crate::shell) const LIST_ROW: &str = "localwave.list-row:";
/// Carry kind of a row drag; the target resolves the selection from the song id.
pub(crate) const SONG_DRAG: &str = "localwave.song:";

pub(in crate::shell) fn drop_target(ui: &mut Ui, visual: &Handle, index: usize, intent: &mut Option<Intent>) {
    let Some(drop) = ui.drop_target(visual).and_then(|drop| {
        let (dropped, payload) = match drop {
            pxui::DropRef::Hover(p) => (false, p),
            pxui::DropRef::Dropped(p) => (true, p),
        };
        payload
            .kind
            .strip_prefix(LIST_ROW)?
            .parse::<usize>()
            .ok()
            .map(|from| (dropped, from))
    }) else {
        return;
    };
    let (dropped, from) = drop;
    let after =
        ui.signal(visual).pointer.1 >= ui.rect(visual).y + ui.rect(visual).h * 0.5;
    let before = index + usize::from(after);
    // Final position once the dragged row leaves its slot.
    let to = if from < before { before - 1 } else { before };
    if to == from {
        return;
    }
    ui.draw_over(visual, move |paint, rect| {
        paint.fill(
            rect.x,
            if after { rect.y + rect.h - 2.0 } else { rect.y },
            rect.w,
            2.0,
            paint.color(INK),
        );
    });
    if dropped {
        *intent = Some(Intent::MoveListRow { from, to });
    }
}

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
    fn drop_target(&self, ui: &mut Ui, visual: &Handle, index: usize, intent: &mut Option<Intent>) {
        drop_target(ui, visual, index, intent);
    }
    pub(super) fn row(
        &self,
        ui: &mut Ui,
        index: usize,
        content_y: f32,
        intent: &mut Option<Intent>,
    ) {
        let Self {
            catalog,
            collection,
            state,
            player,
            widths,
            revision,
            numbered,
            entry,
            viewport,
        } = *self;
        let row = catalog.row(collection.rows[index]);
        let reorder = self.reorderable() && widths[Column::Number as usize] >= 32.0;
        let current = player.current_id == Some(row.id);
        let marked = state.is_marked(index);
        let revision = crate::hash((revision, current, marked));
        let layout = row_layout(
            ui,
            catalog,
            collection.rows[index],
            index + 1,
            numbered,
            widths,
            current,
        );
        let y = content_y - viewport.offset + layout.y(11.0);
        let visible = y + 11.0 > (viewport.leading - viewport.offset).max(0.0) + viewport.heading
            && y < viewport.height;
        let visual_revision = if current || row.state == lw_core::FileState::CopyPending {
            crate::hash((revision, current && player.playing, visible))
        } else {
            revision
        };
        let visual = ui.view(
            "song-visual",
            visual_revision,
            Size::Grow,
            Size::Fit,
            &mut |ui| {
                let row = catalog.row(collection.rows[index]);
                let song_row = song_row_looks(ui);
                let look = PLAIN.look(if marked { &SELECTED_LOOKS } else { &song_row });
                let node = Node::column().w(Size::Grow).h_px(layout.height).role(Role::ListItem);
                let current = player.current_id == Some(row.id);
                let root = ui.pressable("song", node, &look, |ui, _| {
                    ui.view("song-copy", revision, Size::Grow, Size::Grow, &mut |ui| {
                        let mut x = 0.0;
                        for column in Column::ALL {
                            let w = widths[column as usize];
                            if w == 0.0 {
                                continue;
                            }
                            if !(column == Column::Number && current)
                                && !matches!(column, Column::State | Column::Actions)
                            {
                                let mut digits = [0u8; 24];
                                let value = body_value(
                                    &row,
                                    column,
                                    numbered,
                                    stack_fmt(&mut digits, format_args!("{}", index + 1)),
                                );
                                let (fit, h) = layout.sizes[column as usize];
                                let y = layout.y(baseline(column));
                                let left = if column == Column::Number { 32.0 } else { 8.0 };
                                label(
                                    ui,
                                    column.title(),
                                    value,
                                    Rect {
                                        x: x + if column == Column::Time {
                                            w - 8.0 - fit
                                        } else {
                                            left
                                        },
                                        y,
                                        w: fit,
                                        h,
                                    },
                                    body_label(column, current),
                                    if column == Column::Title {
                                        ui.color(INK)
                                    } else {
                                        ui.color(SECONDARY)
                                    },
                                    Role::Label,
                                );
                            }
                            x += w;
                        }
                    });
                    if current {
                        ui.column(
                            "equalizer-position",
                            Node {
                                at: Some((32.0, layout.y(11.0))).into(),
                                ..Node::column().w_px(14.0).h_px(11.0)
                            },
                            |ui| {
                                ui.view(
                                    "equalizer-view",
                                    visual_revision,
                                    Size::Grow,
                                    Size::Grow,
                                    &mut |ui| equalizer(ui, player.playing, visible, row.id),
                                );
                            },
                        );
                    }
                    if widths[Column::State as usize] > 0.0 {
                        let x = widths[..Column::State as usize].iter().sum::<f32>() + 8.0;
                        ui.column(
                            "availability-position",
                            Node {
                                at: Some((x, layout.y(10.0))).into(),
                                ..Node::column().w_px(10.0).h_px(10.0)
                            },
                            |ui| {
                                let visual = ui.view(
                                    "availability-view",
                                    visual_revision,
                                    Size::Grow,
                                    Size::Grow,
                                    &mut |ui| availability(ui, row, visible),
                                );
                                let previous = ui.state(&visual, -1);
                                let next = row.state as i64;
                                if previous != next {
                                    ui.set_state(&visual, next);
                                    if previous >= 0 && !ui.tokens().reduced_motion {
                                        ui.animate_keys(
                                            &visual,
                                            &[
                                                Keyframe::scale(0.0, 0.3, 0.3).curve(crate::motion::spring(crate::motion::BASE)),
                                                Keyframe::scale(crate::motion::BASE, 1.0, 1.0).curve(crate::motion::spring(crate::motion::BASE)),
                                            ],
                                            &Keyframes::DEFAULT,
                                        );
                                    }
                                }
                            },
                        );
                    }
                    let x = widths[..7].iter().sum::<f32>();
                    let node = Node {
                        at: Some((x, (layout.height - 32.0) * 0.5)).into(),
                        ..Node::column().w_px(32.0).h_px(32.0).role(Role::Button)
                    };
                    let more = ui.pressable("actions", node, &PLAIN, |_, _| {});
                    ui.a11y_name(&more.handle, row.actions_label);
                    ui.draw(&more.handle, move |paint, rect| {
                        for x in [5.5, 12.0, 18.5] {
                            paint.circle(
                                (rect.x + 8.0 + x * (2.0 / 3.0), rect.y + 16.0),
                                1.4 * (2.0 / 3.0),
                                Some(paint.color(SECONDARY)),
                                None,
                            );
                        }
                    });
                    if more.clicked {
                        let at = ui.rect(&more.handle);
                        *intent = Some(Intent::SongMenu(
                            index,
                            row.id,
                            collection.key,
                            (at.x, at.y + at.h),
                        ));
                    }
                    if reorder {
                        let node = Node {
                            at: Some((2.0, 0.0)).into(),
                            ..Node::column().w_px(24.0).h_px(layout.height).role(Role::Button)
                        };
                        let grip = ui.pressable("reorder-listed", node, &PLAIN, |_, _| {});
                        ui.a11y_name(&grip.handle, "Reorder track");
                        ui.draw(&grip.handle, move |paint, rect| {
                            for x in [8.0, 12.0] {
                                for y in [-4.0, 0.0, 4.0] {
                                    paint.fill(
                                        rect.x + x,
                                        rect.y + rect.h * 0.5 + y,
                                        2.0,
                                        2.0,
                                        paint.color(SECONDARY),
                                    );
                                }
                            }
                        });
                        if grip.held || grip.clicked {
                            let mut digits = [0u8; 24];
                            let id = stack_fmt(&mut digits, format_args!("{}", index)).as_bytes();
                            let mut kind = [0u8; 48];
                            kind[..LIST_ROW.len()].copy_from_slice(LIST_ROW.as_bytes());
                            let len = LIST_ROW.len() + id.len();
                            kind[LIST_ROW.len()..len].copy_from_slice(id);
                            ui.carry(
                                if grip.clicked { None } else { Some(&grip.handle) },
                                std::str::from_utf8(&kind[..len]).expect("list drag kind is ASCII"),
                                row.title,
                            );
                        }
                    }
                });
                ui.a11y_name(&root.handle, row.title);
                let signal = ui.signal(&root.handle);
                if root.held {
                    let mut digits = [0u8; 24];
                    let id = stack_fmt(&mut digits, format_args!("{}", row.id)).as_bytes();
                    let mut kind = [0u8; 48];
                    kind[..SONG_DRAG.len()].copy_from_slice(SONG_DRAG.as_bytes());
                    let len = SONG_DRAG.len() + id.len();
                    kind[SONG_DRAG.len()..len].copy_from_slice(id);
                    ui.carry(
                        Some(&root.handle),
                        std::str::from_utf8(&kind[..len]).expect("song drag kind is ASCII"),
                        row.title,
                    );
                }
                let last = index + 1 == collection.rows.len();
                ui.draw_over(&root.handle, move |paint, rect| {
                    paint.fill_fractional(rect.x, rect.y.floor(), rect.w, 1.0, paint.color(LINE));
                    if last {
                        paint.fill_fractional(
                            rect.x,
                            (rect.y + rect.h).floor(),
                            rect.w,
                            1.0,
                            paint.color(LINE),
                        );
                    }
                });
                if signal.pressed && signal.button == Some(1) {
                    *intent = Some(Intent::SongMenu(
                        index,
                        row.id,
                        collection.key,
                        signal.pointer,
                    ));
                }
                if intent.is_none() && signal.clicked {
                    let (meta, shift) = (input::Key::Meta.code(), input::Key::Shift.code());
                    *intent = Some(if ui.key_down(meta) {
                        Intent::Select(crate::Selection::Toggle(index))
                    } else if ui.key_down(shift) {
                        Intent::Select(crate::Selection::Range(index))
                    } else {
                        Intent::SelectRow(index)
                    });
                }
                if signal.double_clicked {
                    *intent = Some(Intent::PlayRow(index));
                }
                if reorder {
                    self.drop_target(ui, &root.handle, index, intent);
                }
            },
        );
        entry.row(ui, &visual, index);
    }
}
