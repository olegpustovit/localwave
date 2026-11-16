use super::*;

#[derive(Clone, Copy)]
pub(super) struct ClockLabel {
    bytes: [u8; 24],
    start: usize,
}
impl ClockLabel {
    pub(super) fn integer(mut number: u64) -> Self {
        let mut out = Self {
            bytes: [0; 24],
            start: 24,
        };
        loop {
            out.start -= 1;
            out.bytes[out.start] = b'0' + (number % 10) as u8;
            number /= 10;
            if number == 0 {
                return out;
            }
        }
    }

    fn new(seconds: u64, remaining: bool) -> Self {
        let mut out = Self {
            bytes: [0; 24],
            start: 21,
        };
        out.bytes[21] = b':';
        out.bytes[22] = b'0' + (seconds % 60 / 10) as u8;
        out.bytes[23] = b'0' + (seconds % 10) as u8;
        let mut minutes = seconds / 60;
        loop {
            out.start -= 1;
            out.bytes[out.start] = b'0' + (minutes % 10) as u8;
            minutes /= 10;
            if minutes == 0 {
                break;
            }
        }
        if remaining {
            out.start -= 1;
            out.bytes[out.start] = b'-';
        }
        out
    }
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes[self.start..]
    }
}
pub(super) fn small_clock(
    ui: &mut Ui,
    key: &str,
    seconds: u64,
    remaining: bool,
    width: f32,
    color: Rgba,
) -> Handle {
    let node = container(
        ui,
        key,
        Axis::Row,
        Size::Px(width),
        Size::Px(20.0),
        Rgba::default(),
    );
    ui.a11y_name(
        &node,
        if remaining {
            "Remaining time"
        } else {
            "Elapsed time"
        },
    );
    ui.set_role(&node, Role::Label, 0);
    ui.a11y_number(&node, seconds as f64, 0.0, f64::MAX, 1.0);
    let label = ClockLabel::new(seconds, remaining);
    ui.draw(&node, move |paint, rect| {
        let mut x = rect.x;
        for byte in label.bytes() {
            x += paint.icon((x, rect.y + 4.0), 1, 12.0, *byte as char, color);
        }
    });
    node
}
const DIGITS: [[u8; 7]; 10] = [
    [14, 17, 17, 17, 17, 17, 14],
    [4, 12, 4, 4, 4, 4, 14],
    [14, 17, 1, 2, 4, 8, 31],
    [31, 2, 4, 2, 1, 17, 14],
    [2, 6, 10, 18, 31, 2, 2],
    [31, 16, 30, 1, 1, 17, 14],
    [6, 8, 16, 30, 17, 17, 14],
    [31, 1, 2, 4, 8, 8, 8],
    [14, 17, 17, 14, 17, 17, 14],
    [14, 17, 17, 15, 1, 2, 12],
];
/// Static matrix numerals (Swift's `MatrixText` without the change animation), `height` tall.
pub(super) fn matrix_digits(ui: &mut Ui, key: &str, digits: &[u8], height: f32, color: Rgba) {
    // Counts only: ASCII digits, at most 12 shown.
    let mut buffer = [b'0'; 12];
    let shown = digits.len().clamp(1, 12);
    let copied = digits.len().min(12);
    buffer[..copied].copy_from_slice(&digits[..copied]);
    let unit = height / 7.0;
    let columns = shown * 6 - 1;
    let node = ui.node(
        key,
        Node {
            w: Size::Px((columns as f32 - 0.22) * unit),
            h: Size::Px(height),
            ..Node::DEFAULT
        },
    );
    ui.draw(&node, move |paint, rect| {
        for (index, digit) in buffer[..shown].iter().enumerate() {
            let x = rect.x + index as f32 * 6.0 * unit;
            for (row, bits) in DIGITS[(digit - b'0') as usize].iter().enumerate() {
                for column in 0..5 {
                    if bits & (1 << (4 - column)) != 0 {
                        let y = rect.y + row as f32 * unit;
                        paint.fill_fractional(x + column as f32 * unit, y, unit * 0.78, unit * 0.78, color);
                    }
                }
            }
        }
    });
}
pub(super) fn matrix_clock(ui: &mut Ui, seconds: u64, height: f32, color: Rgba) {
    use pxui::kit::{AnimationTarget, Keyframe};
    const ROW_KEYS: [&str; 7] = [
        "row-0", "row-1", "row-2", "row-3", "row-4", "row-5", "row-6",
    ];
    fn label(seconds: u64) -> ClockLabel {
        let mut label = ClockLabel::new(seconds, false);
        if label.bytes().len() < 5 {
            label.start -= 1;
            label.bytes[label.start] = b'0';
        }
        label
    }
    let shown = label(seconds);
    let unit = height / 7.0;
    let columns: usize = shown
        .bytes()
        .iter()
        .map(|b| if *b == b':' { 2 } else { 6 })
        .sum::<usize>()
        - 1;
    let clock = ui.node(
        "matrix-clock",
        Node {
            w: Size::Px((columns as f32 - 0.22) * unit),
            h: Size::Px(height),
            role: Role::Label,
            ..Node::DEFAULT
        },
    );
    ui.a11y_name(&clock, "Elapsed time");
    ui.a11y_number(&clock, seconds as f64, 0.0, f64::MAX, 1.0);
    let old = ui.state(&clock, seconds as i64) as u64;
    ui.set_state(&clock, seconds as i64);
    ui.within(&clock, &mut |ui| {
        let previous = ui.node(
            "previous",
            Node {
                w: Size::Px(0.0),
                h: Size::Px(0.0),
                ..Node::DEFAULT
            },
        );
        if old != seconds {
            ui.set_state(&previous, old as i64);
        }
        let before = label(ui.state(&previous, seconds as i64) as u64);
        let revision = pxui::hash!((shown.bytes(), before.bytes()));
        // Seven fading rows, shared by every changed digit, bound animation storage on carries.
        for (row, key) in ROW_KEYS.into_iter().enumerate() {
            ui.view(key, revision, Size::Grow, Size::Px(unit), &mut |ui| {
                let stack = ui.node(
                    "row",
                    Node {
                        axis: Axis::Stack,
                        w: Size::Grow,
                        h: Size::Grow,
                        ..Node::DEFAULT
                    },
                );
                ui.within(&stack, &mut |ui| {
                    for changing in [false, true] {
                        let visual = ui.view(
                            if changing { "changed" } else { "stable" },
                            revision,
                            Size::Grow,
                            Size::Grow,
                            &mut |ui| {
                                let cells = ui.node(
                                    "cells",
                                    Node {
                                        w: Size::Grow,
                                        h: Size::Grow,
                                        ..Node::DEFAULT
                                    },
                                );
                                ui.draw(&cells, move |paint, rect| {
                                    let mut x = rect.x;
                                    for (index, byte) in shown.bytes().iter().copied().enumerate() {
                                        let count = if byte == b':' { 1 } else { 5 };
                                        if (before.bytes().get(index) != Some(&byte)) == changing {
                                            let bits = if byte == b':' {
                                                [0, 0, 1, 0, 1, 0, 0][row]
                                            } else {
                                                DIGITS[(byte - b'0') as usize][row]
                                            };
                                            for column in 0..count {
                                                if bits & (1 << (count - 1 - column)) != 0 {
                                                    paint.fill_fractional(
                                                        x + column as f32 * unit,
                                                        rect.y,
                                                        unit * 0.78,
                                                        unit * 0.78,
                                                        color,
                                                    );
                                                }
                                            }
                                        }
                                        x += (count + 1) as f32 * unit;
                                    }
                                });
                            },
                        );
                        if changing && old != seconds && !ui.tokens().reduced_motion {
                            let delay = row as f32 * 0.016;
                            let curve = crate::motion::steps(2, crate::motion::QUICK);
                            ui.animate_keys(
                                &visual,
                                &[
                                    Keyframe {
                                        time: 0.0,
                                        target: AnimationTarget::Opacity(0.0),
                                        curve,
                                    },
                                    Keyframe {
                                        time: delay,
                                        target: AnimationTarget::Opacity(0.0),
                                        curve,
                                    },
                                    Keyframe {
                                        time: delay + crate::motion::QUICK,
                                        target: AnimationTarget::Opacity(1.0),
                                        curve,
                                    },
                                ],
                            );
                        }
                    }
                });
            });
        }
    });
}
