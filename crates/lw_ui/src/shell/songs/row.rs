use super::*;
use crate::catalog::{Collection, Row, SongColumn as Column};

#[derive(Clone, Copy)]
pub(super) struct Viewport {
    pub(super) leading: f32,
    pub(super) heading: f32,
    pub(super) offset: f32,
    pub(super) height: f32,
}

pub(super) fn equalizer(ui: &mut Ui, playing: bool, visible: bool, id: i64, p: Palette) {
    use pxui::kit::{AnimationTarget as Target, Keyframe, Repeat};
    let root = ui.node(
        "equalizer",
        Node {
            w: Size::Px(14.0),
            h: Size::Px(11.0),
            role: Role::Image,
            ..Node::DEFAULT
        },
    );
    ui.a11y_name(&root, if playing { "Playing" } else { "Paused" });
    let song = ui.node(
        "equalizer-song",
        Node {
            w: Size::Px(0.0),
            h: Size::Px(0.0),
            ..Node::DEFAULT
        },
    );
    let active = playing && visible && !ui.tokens().reduced_motion;
    let changed = ui.state(&root, -1) != i64::from(active) || ui.state(&song, i64::MIN) != id;
    ui.set_state(&root, i64::from(active));
    ui.set_state(&song, id);
    ui.within(&root, &mut |ui| {
        for (index, key) in ["bar-0", "bar-1", "bar-2"].into_iter().enumerate() {
            let position = ui.node(
                key,
                Node {
                    w: Size::Px(2.0),
                    h: Size::Px(11.0),
                    at: Some((index as f32 * 4.0, 0.0)).into(),
                    ..Node::DEFAULT
                },
            );
            ui.within(&position, &mut |ui| {
                let bar = ui.view("bar", 0, Size::Grow, Size::Grow, &mut |ui| {
                    let fill = ui.node(
                        "fill",
                        Node {
                            w: Size::Grow,
                            h: Size::Grow,
                            ..Node::DEFAULT
                        },
                    );
                    ui.draw(&fill, move |paint, rect| {
                        paint.fill_fractional(rect.x, rect.y, rect.w, rect.h, p.signal)
                    });
                });
                if changed {
                    let scale = |y| Target::Scale {
                        x: 1.0,
                        y,
                        anchor: (0.5, 1.0),
                    };
                    let curve = Curve::Bezier {
                        duration: 0.45,
                        points: [0.42, 0.0, 0.58, 1.0],
                    };
                    if active {
                        ui.animate_keys_repeat(
                            &bar,
                            &[
                                Keyframe {
                                    time: 0.0,
                                    target: scale(0.4),
                                    curve,
                                },
                                Keyframe {
                                    time: 0.45,
                                    target: scale(1.0),
                                    curve,
                                },
                            ],
                            Repeat::Forever,
                            true,
                            index as f32 * 0.3,
                        );
                    } else {
                        ui.animate_keys(
                            &bar,
                            &[Keyframe {
                                time: 0.0,
                                target: scale(0.4),
                                curve,
                            }],
                        );
                    }
                }
            });
        }
    });
}

pub(super) fn availability(ui: &mut Ui, row: Row<'_>, visible: bool, p: Palette) {
    use lw_core::{FileAvailability, FileState};
    let root = ui.node(
        "availability",
        Node {
            axis: Axis::Stack,
            w: Size::Px(10.0),
            h: Size::Px(10.0),
            role: Role::Image,
            ..Node::DEFAULT
        },
    );
    let label = match row.state {
        FileState::CopyPending => "Preparing offline copy",
        FileState::Failed => "File unavailable — retry",
        _ => match row.availability {
            Some(FileAvailability::OnDevice) if row.pinned => "Kept offline",
            Some(FileAvailability::OnDevice) => "On this device",
            Some(FileAvailability::InCloud) => "In cloud — downloads when played",
            Some(FileAvailability::Unknown) => "Availability unknown",
            None => "Checking availability",
        },
    };
    ui.a11y_name(&root, label);
    let (state, available, pinned) = (row.state, row.availability, row.pinned);
    ui.draw(&root, move |paint, rect| {
        let (x, y) = (rect.x, rect.y);
        if available == Some(FileAvailability::OnDevice)
            && !matches!(state, FileState::CopyPending | FileState::Failed)
        {
            if pinned {
                paint.fill_fractional(x - 3.5, y - 3.5, 17.0, 17.0, p.ink);
                paint.fill_fractional(x - 2.0, y - 2.0, 14.0, 14.0, p.paper);
            }
            paint.fill_fractional(x, y, 10.0, 10.0, p.ink);
        } else {
            let unknown = available.is_none_or(|state| state == FileAvailability::Unknown)
                && !matches!(state, FileState::CopyPending | FileState::Failed);
            let ink = if matches!(state, FileState::CopyPending | FileState::Failed) {
                p.signal
            } else {
                p.secondary
            };
            if unknown {
                for start in [0.75, 4.75, 8.75] {
                    let length = 2.0f32.min(9.25 - start);
                    paint.fill_fractional(x + start, y + 0.25, length, 1.0, p.tertiary);
                    paint.fill_fractional(x + start, y + 8.75, length, 1.0, p.tertiary);
                    paint.fill_fractional(x + 0.25, y + start, 1.0, length, p.tertiary);
                    paint.fill_fractional(x + 8.75, y + start, 1.0, length, p.tertiary);
                }
            } else {
                paint.fill_fractional(x, y, 10.0, 1.5, ink);
                paint.fill_fractional(x, y + 8.5, 10.0, 1.5, ink);
                paint.fill_fractional(x, y + 1.5, 1.5, 7.0, ink);
                paint.fill_fractional(x + 8.5, y + 1.5, 1.5, 7.0, ink);
            }
            if state == FileState::Failed {
                paint.line((x + 1.0, y + 9.0), (x + 9.0, y + 1.0), p.signal);
            }
        }
    });
    if row.state == FileState::CopyPending {
        use pxui::kit::{AnimationTarget as Target, Keyframe, Repeat};
        let active = visible && !ui.tokens().reduced_motion;
        let changed = ui.state(&root, -1) != i64::from(active);
        ui.set_state(&root, i64::from(active));
        ui.within(&root, &mut |ui| {
            for (index, key) in ["left", "right"].into_iter().enumerate() {
                let half = ui.view(key, 0, Size::Grow, Size::Grow, &mut |ui| {
                    let fill = ui.node(
                        "fill",
                        Node {
                            w: Size::Grow,
                            h: Size::Grow,
                            ..Node::DEFAULT
                        },
                    );
                    ui.draw(&fill, move |paint, rect| {
                        paint.fill_fractional(
                            rect.x + index as f32 * 5.0,
                            rect.y,
                            5.0,
                            10.0,
                            p.signal,
                        )
                    });
                });
                if changed {
                    let curve = crate::motion::steps(1, 0.5);
                    if active {
                        ui.animate_keys_repeat(
                            &half,
                            &[
                                Keyframe {
                                    time: 0.0,
                                    target: Target::Opacity(1.0),
                                    curve,
                                },
                                Keyframe {
                                    time: 0.5,
                                    target: Target::Opacity(0.0),
                                    curve,
                                },
                                Keyframe {
                                    time: 1.0,
                                    target: Target::Opacity(1.0),
                                    curve,
                                },
                            ],
                            Repeat::Forever,
                            false,
                            index as f32 * 0.5,
                        );
                    } else {
                        ui.animate_keys(
                            &half,
                            &[Keyframe {
                                time: 0.0,
                                target: Target::Opacity(if index == 0 { 1.0 } else { 0.0 }),
                                curve,
                            }],
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
    pub(super) p: Palette,
    pub(super) entry: &'a crate::motion::Page,
    pub(super) viewport: Viewport,
}
pub(super) const LIST_ROW: &str = "localwave.list-row:";

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
        let p = self.p;
        let Some(drop) = ui.drop_target_ref(visual).and_then(|drop| {
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
            ui.signal_state(visual).pointer.1 >= ui.rect(visual).y + ui.rect(visual).h * 0.5;
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
                p.ink,
            );
        });
        if dropped {
            *intent = Some(Intent::MoveListRow { from, to });
        }
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
            p,
            entry,
            viewport,
        } = *self;
        let row = catalog.row(collection.rows[index]);
        let reorder = self.reorderable() && widths[Column::Number as usize] >= 32.0;
        let current = player.current_id == Some(row.id);
        let marked = state.is_marked(index);
        let revision = pxui::hash!((revision, current, marked));
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
            pxui::hash!((revision, current && player.playing, visible))
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
                let root = ui.node(
                    "song",
                    Node {
                        flags: CLICKABLE | TAB_STOP,
                        role: Role::ListItem,
                        w: Size::Grow,
                        h: Size::Px(layout.height),
                        ..Node::DEFAULT
                    },
                );
                ui.a11y_name(&root, row.title);
                let signal = ui.signal_state(&root);
                ui.set_look(
                    &root,
                    Look {
                        fill: if marked {
                            p.selected
                        } else if signal.hovered {
                            Rgba { a: 153, ..p.panel }
                        } else {
                            p.paper
                        },
                        ..Look::NONE
                    },
                    State::default(),
                );
                let current = player.current_id == Some(row.id);
                ui.within(&root, &mut |ui| {
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
                                let number = ClockLabel::integer((index + 1) as u64);
                                let value = body_value(
                                    &row,
                                    column,
                                    numbered,
                                    std::str::from_utf8(number.bytes()).unwrap(),
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
                                    body_style(column, current),
                                    if column == Column::Title {
                                        p.ink
                                    } else {
                                        p.secondary
                                    },
                                    Role::Label,
                                );
                            }
                            x += w;
                        }
                    });
                    if current {
                        let at = ui.node(
                            "equalizer-position",
                            Node {
                                w: Size::Px(14.0),
                                h: Size::Px(11.0),
                                at: Some((32.0, layout.y(11.0))).into(),
                                ..Node::DEFAULT
                            },
                        );
                        ui.within(&at, &mut |ui| {
                            ui.view(
                                "equalizer-view",
                                visual_revision,
                                Size::Grow,
                                Size::Grow,
                                &mut |ui| equalizer(ui, player.playing, visible, row.id, p),
                            );
                        });
                    }
                    if widths[Column::State as usize] > 0.0 {
                        let x = widths[..Column::State as usize].iter().sum::<f32>() + 8.0;
                        let at = ui.node(
                            "availability-position",
                            Node {
                                w: Size::Px(10.0),
                                h: Size::Px(10.0),
                                at: Some((x, layout.y(10.0))).into(),
                                ..Node::DEFAULT
                            },
                        );
                        ui.within(&at, &mut |ui| {
                            let visual = ui.view(
                                "availability-view",
                                visual_revision,
                                Size::Grow,
                                Size::Grow,
                                &mut |ui| availability(ui, row, visible, p),
                            );
                            let previous = ui.state(&visual, -1);
                            let next = row.state as i64;
                            if previous != next {
                                ui.set_state(&visual, next);
                                if previous >= 0 && !ui.tokens().reduced_motion {
                                    ui.animate_keys(
                                        &visual,
                                        &[
                                            pxui::kit::Keyframe {
                                                time: 0.0,
                                                target: pxui::kit::AnimationTarget::Scale {
                                                    x: 0.3,
                                                    y: 0.3,
                                                    anchor: (0.5, 0.5),
                                                },
                                                curve: crate::motion::spring(crate::motion::BASE),
                                            },
                                            pxui::kit::Keyframe {
                                                time: crate::motion::BASE,
                                                target: pxui::kit::AnimationTarget::Scale {
                                                    x: 1.0,
                                                    y: 1.0,
                                                    anchor: (0.5, 0.5),
                                                },
                                                curve: crate::motion::spring(crate::motion::BASE),
                                            },
                                        ],
                                    );
                                }
                            }
                        });
                    }
                    let x = widths[..7].iter().sum::<f32>();
                    let more = ui.node(
                        "actions",
                        Node {
                            flags: CLICKABLE | TAB_STOP,
                            role: Role::Button,
                            w: Size::Px(32.0),
                            h: Size::Px(32.0),
                            at: Some((x, (layout.height - 32.0) * 0.5)).into(),
                            ..Node::DEFAULT
                        },
                    );
                    ui.a11y_name(&more, row.actions_label);
                    ui.draw(&more, move |paint, rect| {
                        for x in [5.5, 12.0, 18.5] {
                            paint.circle(
                                (rect.x + 8.0 + x * (2.0 / 3.0), rect.y + 16.0),
                                1.4 * (2.0 / 3.0),
                                Some(p.secondary),
                                None,
                            );
                        }
                    });
                    if ui.signal_state(&more).clicked {
                        let at = ui.rect(&more);
                        *intent = Some(Intent::SongMenu(
                            index,
                            row.id,
                            collection.key,
                            (at.x, at.y + at.h),
                        ));
                    }
                    if reorder {
                        let grip = ui.node(
                            "reorder-listed",
                            Node {
                                flags: CLICKABLE | TAB_STOP,
                                role: Role::Button,
                                w: Size::Px(24.0),
                                h: Size::Px(layout.height),
                                at: Some((2.0, 0.0)).into(),
                                ..Node::DEFAULT
                            },
                        );
                        ui.a11y_name(&grip, "Reorder track");
                        ui.draw(&grip, move |paint, rect| {
                            for x in [8.0, 12.0] {
                                for y in [-4.0, 0.0, 4.0] {
                                    paint.fill(
                                        rect.x + x,
                                        rect.y + rect.h * 0.5 + y,
                                        2.0,
                                        2.0,
                                        p.secondary,
                                    );
                                }
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
                                row.title,
                            );
                        }
                    }
                });
                let last = index + 1 == collection.rows.len();
                ui.draw_over(&root, move |paint, rect| {
                    paint.fill_fractional(rect.x, rect.y.floor(), rect.w, 1.0, p.line);
                    if last {
                        paint.fill_fractional(
                            rect.x,
                            (rect.y + rect.h).floor(),
                            rect.w,
                            1.0,
                            p.line,
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
                    self.drop_target(ui, &root, index, intent);
                }
            },
        );
        entry.row(ui, &visual, index);
    }
}
