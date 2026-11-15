use super::*;
use crate::catalog::{Collection, Row, SongColumn as Column};

pub(super) fn body_style(column: Column, current: bool) -> pxui::TextStyle {
    let number = column == Column::Number;
    let mono = matches!(column, Column::Format | Column::Time);
    let wrap = number || mono;
    pxui::TextStyle {
        font: u16::from(mono),
        size_points: if number {
            13.0
        } else if mono {
            12.0
        } else {
            14.0
        },
        line_height: if number {
            18.2
        } else if mono {
            16.8
        } else {
            19.6
        },
        weight: if current && !number && !mono {
            500
        } else {
            400
        },
        features: if number {
            pxui::TextStyle::FEATURE_TABULAR_NUMBERS
        } else {
            0
        },
        wrap: if wrap {
            pxui::TextStyle::WRAP_WORD_SPACES
        } else {
            pxui::TextStyle::WRAP_NONE
        },
        max_lines: if wrap { 0 } else { 1 },
        ellipsis: if wrap {
            pxui::TextStyle::ELLIPSIS_NONE
        } else {
            pxui::TextStyle::ELLIPSIS_END
        },
        ..Default::default()
    }
}
pub(super) fn body_value<'a>(
    row: &Row<'a>,
    column: Column,
    numbered: bool,
    number: &'a str,
) -> &'a str {
    match column {
        Column::Number if numbered && row.track_number.is_some() => row.track_label,
        Column::Number => number,
        Column::Title => row.title,
        Column::Artist => row.artist,
        Column::Album => row.album,
        Column::Format => {
            if row.has_format {
                row.format
            } else {
                "—"
            }
        }
        Column::Time => row.duration,
        _ => "",
    }
}

#[derive(Clone, Copy)]
pub(super) struct Viewport {
    pub(super) leading: f32,
    pub(super) heading: f32,
    pub(super) offset: f32,
    pub(super) height: f32,
}

pub(super) fn equalizer(ui: &mut Ui, playing: bool, visible: bool, id: i64, p: Palette) {
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
    let clock = ui.node(
        "equalizer-clock",
        Node {
            w: Size::Px(0.0),
            h: Size::Px(0.0),
            ..Node::DEFAULT
        },
    );
    let song = ui.node(
        "equalizer-song",
        Node {
            w: Size::Px(0.0),
            h: Size::Px(0.0),
            ..Node::DEFAULT
        },
    );
    let now = ui.now();
    if ui.state(&root, -1) != i64::from(playing) || ui.state(&song, i64::MIN) != id {
        ui.set_state(&root, i64::from(playing));
        ui.set_state(&song, id);
        ui.set_state(&clock, now.to_bits() as i64);
    }
    let elapsed = (now - f64::from_bits(ui.state(&clock, now.to_bits() as i64) as u64)).max(0.0);
    let animate = playing && !ui.tokens().reduced_motion;
    let heights = std::array::from_fn::<_, 3, _>(|index| {
        let phase = (elapsed / 0.9 + index as f64 / 3.0).fract() as f32;
        let target = if phase < 0.5 {
            phase * 2.0
        } else {
            (1.0 - phase) * 2.0
        };
        11.0 * if animate {
            0.4 + 0.6
                * Curve::Bezier {
                    duration: 1.0,
                    points: [0.42, 0.0, 0.58, 1.0],
                }
                .at(target)
        } else {
            0.4
        }
    });
    ui.draw(&root, move |paint, rect| {
        for (index, height) in heights.into_iter().enumerate() {
            paint.fill_fractional(
                rect.x + index as f32 * 4.0,
                rect.y + 11.0 - height,
                2.0,
                height,
                p.signal,
            );
        }
    });
    if animate && visible {
        ui.repaint_after(1.0 / 60.0);
    }
}

pub(super) fn availability(ui: &mut Ui, row: Row<'_>, visible: bool, p: Palette) {
    use lw_core::{FileAvailability, FileState};
    let root = ui.node(
        "availability",
        Node {
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
    let alternate = row.state == FileState::CopyPending && !ui.tokens().reduced_motion;
    let left = !alternate || (ui.now() * 2.0).floor() as i64 % 2 == 0;
    if alternate && visible {
        ui.repaint_after(0.5 - (ui.now() % 0.5));
    }
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
            if state == FileState::CopyPending {
                paint.fill_fractional(x + if left { 0.0 } else { 5.0 }, y, 5.0, 10.0, p.signal);
            }
            if state == FileState::Failed {
                paint.line((x + 1.0, y + 9.0), (x + 9.0, y + 1.0), p.signal);
            }
        }
    });
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
impl Rows<'_> {
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
        let current = player.current_id == Some(row.id);
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
        let visual_revision =
            if player.current_id == Some(row.id) || row.state == lw_core::FileState::CopyPending {
                let mut hash = FastHasher::default();
                (revision, visible).hash(&mut hash);
                hash.finish()
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
                        fill: if state.selected == Some(index) {
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
                    *intent = Some(Intent::SelectRow(index));
                }
                if signal.double_clicked {
                    *intent = Some(Intent::PlayRow(index));
                }
            },
        );
        entry.row(ui, &visual, index);
    }
}
