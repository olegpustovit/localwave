use super::*;

/// Formats into `buffer` (no heap): a number or clock string for a label, a drag kind or a seek readout.
pub(crate) fn stack_fmt<'a>(buffer: &'a mut [u8; 24], args: std::fmt::Arguments) -> &'a str {
    use std::io::Write;
    let mut rest = &mut buffer[..];
    // 24 bytes hold u64::MAX and h:mm:ss; a longer string truncates at a char boundary of ASCII digits.
    let _ = rest.write_fmt(args);
    let used = 24 - rest.len();
    std::str::from_utf8(&buffer[..used]).unwrap_or("")
}
pub(super) fn small_clock(
    ui: &mut Ui,
    key: &'static str,
    seconds: u64,
    remaining: bool,
    width: f32,
    color: pxui::theme::ColorToken,
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
    ui.set_role(&node, Role::Label, pxui::A11yStates::NONE);
    ui.a11y_number(&node, seconds as f64, 0.0, f64::MAX, 1.0).expect("valid clock value");
    let mut buffer = [0u8; 24];
    let text = stack_fmt(&mut buffer, format_args!("{}{}:{:02}", if remaining { "-" } else { "" }, seconds / 60, seconds % 60));
    let len = text.len();
    const CLOCK: pxui::Label = pxui::Label::BODY.font(1).size(12.0);
    // Swift's clock baseline, 16.06 pt under the node's top (its em box 4 pt down, CoreText's mono ascent 1.005 em)
    const BASELINE: f32 = 16.06;
    ui.canvas(&node, move |paint, rect| {
        let text = std::str::from_utf8(&buffer[..len]).unwrap_or("");
        // `text_chars` places the layout top: the first baseline lies `baseline` under it (the digits share one layout's
        // metrics, so "0" measures it from a cached layout)
        let baseline = paint.text_service().measure("0", CLOCK, None).map_or(BASELINE - 4.0, |m| m.baseline);
        paint.text_chars(pxui::Point::new(rect.x, rect.y + BASELINE - baseline), CLOCK.color(color), text);
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
pub(super) fn matrix_digits(ui: &mut Ui, key: &'static str, digits: &[u8], height: f32, color: Rgba) {
    // Counts only: ASCII digits, at most 12 shown.
    let mut buffer = [b'0'; 12];
    let shown = digits.len().clamp(1, 12);
    let copied = digits.len().min(12);
    buffer[..copied].copy_from_slice(&digits[..copied]);
    let unit = height / 7.0;
    let columns = shown * 6 - 1;
    let node = ui.node(key, Node::column().w_px((columns as f32 - 0.22) * unit).h_px(height));
    ui.canvas(&node, move |paint, rect| {
        for (index, digit) in buffer[..shown].iter().enumerate() {
            let x = rect.x + index as f32 * 6.0 * unit;
            for (row, bits) in DIGITS[(digit - b'0') as usize].iter().enumerate() {
                for column in 0..5 {
                    if bits & (1 << (4 - column)) != 0 {
                        let y = rect.y + row as f32 * unit;
                        paint.fill(Rect::new(x + column as f32 * unit, y, unit * 0.78, unit * 0.78), Fill::new(color).fractional());
                    }
                }
            }
        }
    });
}
pub(super) fn matrix_clock(ui: &mut Ui, seconds: u64, height: f32, color: Rgba) {
    use pxui::{ui::Keyframe, Keyframes};
    const ROW_KEYS: [&str; 7] = [
        "row-0", "row-1", "row-2", "row-3", "row-4", "row-5", "row-6",
    ];
    fn label(seconds: u64) -> ([u8; 24], usize) {
        let mut buffer = [0u8; 24];
        let len = stack_fmt(&mut buffer, format_args!("{:02}:{:02}", seconds / 60, seconds % 60)).len();
        (buffer, len)
    }
    let (shown_buffer, shown_len) = label(seconds);
    let shown = &shown_buffer[..shown_len];
    let unit = height / 7.0;
    let columns: usize = shown
        .iter()
        .map(|b| if *b == b':' { 2 } else { 6 })
        .sum::<usize>()
        - 1;
    let clock = ui.node(
        "matrix-clock",
        Node::column().w_px((columns as f32 - 0.22) * unit).h_px(height).role(Role::Label),
    );
    ui.a11y_name(&clock, "Elapsed time");
    ui.a11y_number(&clock, seconds as f64, 0.0, f64::MAX, 1.0).expect("valid clock value");
    let old = ui.state(&clock, seconds as i64) as u64;
    ui.set_state(&clock, seconds as i64);
    ui.within(&clock, &mut |ui| {
        let previous = ui.node("previous", Node::column().w_px(0.0).h_px(0.0));
        if old != seconds {
            ui.set_state(&previous, old as i64);
        }
        let (before_buffer, before_len) = label(ui.state(&previous, seconds as i64) as u64);
        let before = &before_buffer[..before_len];
        let revision = crate::hash((shown, before));
        // Seven fading rows, shared by every changed digit, bound animation storage on carries.
        for (row, key) in ROW_KEYS.into_iter().enumerate() {
            ui.column(key, Node::column().h_px(unit), |ui| { ui.view_of(key, &revision, |ui| {
                ui.stack("row", Node::stack().w(Size::Grow).h(Size::Grow), |ui| {
                    for changing in [false, true] {
                        let visual = ui.view_of(if changing { "changed" } else { "stable" }, &revision, |ui| {
                                let cells = ui.node("cells", Node::column().w(Size::Grow).h(Size::Grow));
                                ui.canvas(&cells, move |paint, rect| {
                                    let (shown, before) = (&shown_buffer[..shown_len], &before_buffer[..before_len]);
                                    let mut x = rect.x;
                                    for (index, byte) in shown.iter().copied().enumerate() {
                                        let count = if byte == b':' { 1 } else { 5 };
                                        if (before.get(index) != Some(&byte)) == changing {
                                            let bits = if byte == b':' {
                                                [0, 0, 1, 0, 1, 0, 0][row]
                                            } else {
                                                DIGITS[(byte - b'0') as usize][row]
                                            };
                                            for column in 0..count {
                                                if bits & (1 << (count - 1 - column)) != 0 {
                                                    paint.fill(Rect::new(x + column as f32 * unit, rect.y, unit * 0.78, unit * 0.78), Fill::new(color).fractional());
                                                }
                                            }
                                        }
                                        x += (count + 1) as f32 * unit;
                                    }
                                });
                            });
                        if changing && old != seconds && !ui.tokens().reduced_motion {
                            let delay = row as f32 * 0.016;
                            let curve = crate::motion::steps(2, crate::motion::QUICK);
                            ui.animate_keys(
                                &visual,
                                &[
                                    Keyframe::opacity(0.0, 0.0).curve(curve),
                                    Keyframe::opacity(delay, 0.0).curve(curve),
                                    Keyframe::opacity(delay + crate::motion::QUICK, 1.0).curve(curve),
                                ],
                                &Keyframes::DEFAULT,
                            );
                        }
                    }
                });
            }); });
        }
    });
}
