use pxui::bodies::BodyConfig;
use super::*;

// LocalWave's colour tokens (`Ui::set_colors`), read with `ui.color` and `paint.color`; the accent is the theme's own.
pub(super) const PAPER: ColorToken = ColorToken(0);
pub(super) const PANEL: ColorToken = ColorToken(1);
pub(super) const SELECTED: ColorToken = ColorToken(2);
pub(super) const INK: ColorToken = ColorToken(3);
pub(super) const SECONDARY: ColorToken = ColorToken(4);
pub(super) const TERTIARY: ColorToken = ColorToken(5);
pub(super) const LINE: ColorToken = ColorToken(6);
pub(super) const KEY: ColorToken = ColorToken(7);
pub(super) const DANGER: ColorToken = ColorToken(8);
pub(super) const fn rgb(hex: u32) -> Rgba {
    Rgba {
        r: (hex >> 16) as u8,
        g: (hex >> 8) as u8,
        b: hex as u8,
        a: 255,
    }
}
/// The (light, dark) pair of each token, in token order.
pub(super) const COLORS: [(Rgba, Rgba); 9] = [
    (rgb(0xF3F3F3), rgb(0x101010)),
    (rgb(0xE6E6E6), rgb(0x1A1A1A)),
    (rgb(0xD3D3D3), rgb(0x2A2A2A)),
    (rgb(0x111111), rgb(0xEDEDED)),
    (rgb(0x585858), rgb(0x9B9B9B)),
    (rgb(0xA0A0A0), rgb(0x5A5A5A)),
    (rgb(0xDADADA), rgb(0x262626)),
    (rgb(0xFBFBFB), rgb(0x1F1F1F)),
    (rgb(0xD21A1A), rgb(0xD21A1A)),
];
/// Installs LocalWave's light and dark colour tables; call once between frames, beside the themes.
pub fn set_colors(ui: &mut Ui) {
    ui.set_colors(&COLORS);
}
pub(super) fn container(
    ui: &mut Ui,
    key: &'static str,
    axis: Axis,
    w: Size,
    h: Size,
    fill: Rgba,
) -> Handle {
    ui.node(
        key.into(),
        Node {
            axis,
            look: Look::NONE.fill(fill),
            ..Node::column().w(w).h(h)
        },
    )
}
pub(super) fn spacer(ui: &mut Ui, key: &'static str, height: f32) {
    ui.node(key.into(), Node::DEFAULT.h(Size::Px(height)));
}
pub(super) fn rule(ui: &mut Ui, key: &'static str, vertical: bool) {
    container(
        ui,
        key,
        Axis::Column,
        if vertical { Size::Px(1.0) } else { Size::Grow },
        if vertical { Size::Grow } else { Size::Px(1.0) },
        ui.color(LINE),
    );
}

pub(super) fn paint_artwork(
    ui: &mut Ui,
    node: &Handle,
    row: Option<crate::catalog::Row<'_>>,
    points: f32,
    images: Option<&crate::artwork::Images>,
) {
    ui.set_role(node, Role::Image, 0);
    if let (Some(row), Some(images)) = (row, images)
        && let Some(path) = row.cover_art_path
        && let Some(image) = images.get(row.cover_art_id, path, points, Some(node))
    {
        // a descriptor that went bad between frames draws nothing (`Paint::image` refuses it)
        ui.canvas(node, move |paint, rect| {
            let _ = paint.image(rect, &image, Fit::Cover, Rgba::rgb(255, 255, 255));
        });
    }
}

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Play,
    Pause,
    Previous,
    Next,
    Queue,
    Panel,
    Shuffle,
    Repeat,
    Refresh,
    Clear,
    Down,
    More,
    Settings,
    Back,
    Check,
}
pub(super) fn paint_icon(paint: &mut pxui::Paint<'_>, icon: Icon, x: f32, y: f32, color: Rgba) {
    match icon {
        Icon::Play => crate::icons::play(
            paint,
            Rect {
                x,
                y,
                w: 20.0,
                h: 20.0,
            },
            color,
        ),
        Icon::Pause => {
            paint.fill(Rect::new(x + 4.0, y + 3.0, 4.0, 14.0), Fill::new(color));
            paint.fill(Rect::new(x + 12.0, y + 3.0, 4.0, 14.0), Fill::new(color));
        }
        Icon::Queue => {
            for dy in [5.0, 10.0, 15.0] {
                paint.fill(Rect::new(x + 2.0, y + dy, 2.0, 1.5), Fill::new(color));
                paint.fill(Rect::new(x + 6.0, y + dy, 12.0, 1.5), Fill::new(color));
            }
        }
        Icon::Previous | Icon::Next => {
            let next = matches!(icon, Icon::Next);
            paint.fill(Rect::new(x + if next { 15.0 } else { 3.0 }, y + 4.0, 2.0, 12.0), Fill::new(color));
            for row in 0..12 {
                let w = (6.0 - (row as f32 - 5.5).abs()) * 1.5;
                paint.fill(Rect::new(x + if next { 4.0 } else { 15.0 - w }, y + 4.0 + row as f32, w, 1.0), Fill::new(color));
            }
        }
        Icon::Panel => {
            paint.stroke(Rect::new(x + 2.0, y + 3.0, 16.0, 14.0), Stroke::new(color));
            paint.line((x + 12.0, y + 3.0), (x + 12.0, y + 17.0), Stroke::new(color));
        }
        Icon::Shuffle => {
            crate::icons::shuffle(
                paint,
                Rect {
                    x,
                    y,
                    w: 20.0,
                    h: 20.0,
                },
                color,
            );
        }
        Icon::Repeat => crate::icons::repeat(
            paint,
            Rect {
                x,
                y,
                w: 20.0,
                h: 20.0,
            },
            color,
        ),
        Icon::Refresh => {
            paint.line((x + 4.0, y + 5.0), (x + 16.0, y + 5.0), Stroke::new(color));
            paint.line((x + 16.0, y + 5.0), (x + 16.0, y + 15.0), Stroke::new(color));
            paint.line((x + 16.0, y + 15.0), (x + 4.0, y + 15.0), Stroke::new(color));
            paint.line((x + 4.0, y + 15.0), (x + 4.0, y + 10.0), Stroke::new(color));
            paint.line((x + 4.0, y + 5.0), (x + 8.0, y + 2.0), Stroke::new(color));
            paint.line((x + 4.0, y + 5.0), (x + 8.0, y + 8.0), Stroke::new(color));
        }
        Icon::Clear => {
            paint.line((x + 5.0, y + 5.0), (x + 15.0, y + 15.0), Stroke::new(color));
            paint.line((x + 15.0, y + 5.0), (x + 5.0, y + 15.0), Stroke::new(color));
        }
        Icon::More => {
            for dx in [3.0, 9.0, 15.0] {
                paint.fill(Rect::new(x + dx, y + 9.0, 2.5, 2.5), Fill::new(color));
            }
        }
        Icon::Settings => {
            for (dy, knob) in [(5.0, 12.0), (10.0, 6.0), (15.0, 13.0)] {
                paint.fill(Rect::new(x + 2.0, y + dy, 16.0, 1.5), Fill::new(color));
                paint.fill(Rect::new(x + knob - 1.5, y + dy - 2.0, 3.0, 5.5), Fill::new(color));
            }
        }
        Icon::Back => {
            paint.line((x + 12.0, y + 4.0), (x + 6.0, y + 10.0), Stroke::new(color));
            paint.line((x + 6.0, y + 10.0), (x + 12.0, y + 16.0), Stroke::new(color));
        }
        Icon::Check => {
            paint.line((x + 4.0, y + 10.0), (x + 8.0, y + 14.0), Stroke::new(color));
            paint.line((x + 8.0, y + 14.0), (x + 16.0, y + 5.0), Stroke::new(color));
        }
        Icon::Down => {
            paint.line((x + 3.0, y + 7.0), (x + 10.0, y + 14.0), Stroke::new(color));
            paint.line((x + 10.0, y + 14.0), (x + 17.0, y + 7.0), Stroke::new(color));
        }
    }
}
/// Every kit icon, indexed by its discriminant (the word an icon button's drawing carries).
const ICONS: [Icon; 15] = [
    Icon::Play, Icon::Pause, Icon::Previous, Icon::Next, Icon::Queue, Icon::Panel, Icon::Shuffle, Icon::Repeat,
    Icon::Refresh, Icon::Clear, Icon::Down, Icon::More, Icon::Settings, Icon::Back, Icon::Check,
];
// the table is indexed by discriminant: keep it in declaration order
const _: () = {
    let mut i = 0;
    while i < ICONS.len() {
        assert!(ICONS[i] as usize == i);
        i += 1;
    }
};
/// The one drawing of every kit icon button: `word` names the icon.
fn draw_icon(paint: &mut pxui::Paint<'_>, at: pxui::geom::Rect, ink: Rgba, word: u32) {
    if let Some(&icon) = ICONS.get(word as usize) {
        paint_icon(paint, icon, at.x, at.y, ink);
    }
}
/// `icon` as a kit icon button's drawing, painted into the 20-point square it is given in the look's ink.
pub(super) fn icon_drawing(icon: Icon) -> pxui::Icon {
    pxui::Icon::draw(draw_icon, icon as u32)
}

// SwiftUI wraps table labels, then fits their line box to physical pixels.
pub(super) fn heading_size(ui: &mut Ui, value: &str, available: f32, natural: f32) -> (f32, f32) {
    table_text_size(
        ui,
        value,
        available,
        natural,
        &TABLE_HEADING,
        TABLE_LINE,
    )
}
/// A table column heading: 12 points, medium, wrapping at words.
pub(super) const TABLE_HEADING: Label = Label::BODY.size(12.0).line_height(TABLE_LINE).weight(500).wrap_spaces();
/// The line height of 12-point table text (headings and mono cells), in points.
pub(super) const TABLE_LINE: f32 = 16.8;
/// -0.03 em, one f32 step past it so 24 and 40 point text resolve to exactly -0.72 and -1.2 points of tracking.
pub(crate) const TIGHT_3: f32 = -0.030_000_001;
pub(super) fn table_text_size(
    ui: &mut Ui,
    value: &str,
    available: f32,
    natural: f32,
    style: &pxui::Label,
    line_height: f32,
) -> (f32, f32) {
    let (width, lines) = if natural > available {
        let metrics = ui
            .text_service()
            .measure(value, style, available.max(0.01))
            .unwrap();
        (metrics.width, metrics.lines)
    } else {
        (natural, 1)
    };
    let density = ui.density() as f32;
    (
        (width * density).ceil() / density,
        line_height * lines.max(1) as f32,
    )
}

/// `Menu::variant` of a menu whose rows sit on the paper fill, read ink-2 while off and report ENABLED (the folder options).
pub(super) const MENU_PANEL: u16 = 1;
/// `Menu::variant` of a settings choice menu: the current row on the selected fill, every row a checkable toggle.
pub(super) const MENU_CHOICE: u16 = 2;
/// LocalWave's context menu row (registered as `localwave-menu`): 13-point text 12 points in, ink or the danger red; `MENU_PANEL`
/// and `MENU_CHOICE` rows also take the paper (or selected) fill and their own accessible role and states.
pub(crate) fn menu_row_body(ui: &mut Ui, c: &pxui::bodies::MenuRowConfig) {
    let (enabled, selected) = (c.state.disabled == 0, c.state.checked != 0);
    ui.set_node_layout(&c.row, &Node::column().w(Size::Grow).h_px(c.row_h).pad_x(12.0));
    let ink = if c.danger != 0 { DANGER } else { INK };
    if c.variant == 0 {
        let look = MENU_ROW.text(Rgba::token(ink));
        ui.set_look(&c.row, look, State { disabled: !enabled, ..State::default() });
    } else {
        let fill = Rgba::token(if selected { SELECTED } else { PAPER });
        let look = MENU_ROW.fill(fill).text(Rgba::token(if enabled { ink } else { SECONDARY }));
        ui.set_look(&c.row, look, State::default());
    }
    ui.text(&c.row, c.text_of(&c.text));
    use pxui::a11y::state::{CHECKABLE, CHECKED, ENABLED};
    match c.variant {
        MENU_PANEL => ui.set_role(&c.row, Role::MenuItem, if enabled { ENABLED } else { 0 }),
        MENU_CHOICE => ui.set_role(&c.row, Role::Toggle, CHECKABLE | if selected { CHECKED } else { 0 }),
        _ => {}
    }
}
/// A context menu row's 13-point text.
const MENU_ROW: Look = Look::NONE.font(Font::BODY.size(13));
#[cfg(test)]
pub(super) mod button_tests {
    use super::*;
    use pxui::{Event, FrameIn, Platform, Str, ThemeSource};
    use std::cell::Cell;
    #[test]
    fn matrix_digits_match_swift_geometry_stagger_changed_rows_and_stop_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let ink = Rgba {
            r: 12,
            g: 13,
            b: 14,
            a: 255,
        };
        let mut time = 0.0;
        for cycle in 0..4 {
            mem_sys::local_reset();
            for seconds in [0, 1, 59, 60, 3599, 3600, 59999, 60000] {
                time += 1.0;
                for offset in [0.0, 0.069, 0.07, 0.139, 0.14, 0.235, 0.236, 0.3] {
                    crate::testing::frame(
                        &mut ui,
                        &FrameIn {
                            width: 400.0,
                            height: 80.0,
                            time: time + offset,
                            ..Default::default()
                        },
                        &mut |ui| matrix_clock(ui, seconds, 43.0, ink),
                    )
                    .unwrap();
                    let clock = ui
                        .a11y_nodes()
                        .iter()
                        .find(|n| ui.a11y_name_of(n) == "Elapsed time")
                        .unwrap();
                    assert!((clock.rect.h - 43.0).abs() < 0.001);
                    if seconds < 6000 {
                        assert!((clock.rect.w - 152.22).abs() < 0.001);
                    }
                    for dot in ui
                        .layers()
                        .flat_map(|l| l.instances.iter())
                        .filter(|i| i.color == [12, 13, 14, 255])
                    {
                        assert!((dot.rect[2] - dot.rect[0] - 43.0 / 7.0 * 0.78).abs() < 0.001);
                        assert!((dot.rect[3] - dot.rect[1] - 43.0 / 7.0 * 0.78).abs() < 0.001);
                    }
                    if offset >= 0.236 {
                        assert!(
                            ui.repaint_at().is_none(),
                            "completed matrix must remain idle"
                        );
                    }
                }
            }
            if cycle > 0 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "all digit carries, including minute/hour boundaries"
                );
            }
        }
    }

    #[test]
    fn waveform_bars_preview_drag_seek_on_release_and_keys_without_warm_allocations() {
        use pxui::{Event, FrameIn, Platform};
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut peaks = [0.0; 512];
        peaks[256..].fill(1.0);
        let player = PlayerView {
            waveform: Some(&peaks),
            duration: 240.0,
            initialized: true,
            ..Default::default()
        };
        fn frame(
            ui: &mut Ui,
            player: &PlayerView<'_>,
            time: f64,
            events: &[Event],
            intent: &mut Option<Intent>,
        ) {
            crate::testing::frame(
                ui,
                &FrameIn {
                    width: 1440.0,
                    height: 71.0,
                    time,
                    events: events.as_ptr(),
                    event_count: events.len(),
                    ..Default::default()
                },
                &mut |ui| {
                    transport(ui, player, 60.0, true, intent);
                },
            )
            .unwrap();
        }
        for tick in 0..12 {
            frame(&mut ui, &player, tick as f64, &[], &mut None);
        }
        let rect = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Playback position")
            .unwrap()
            .rect;
        let width = rect.w / 800.0;
        let bars: Vec<_> = ui
            .layers()
            .flat_map(|layer| layer.instances.iter())
            .filter(|instance| {
                (instance.rect[2] - instance.rect[0] - width).abs() < 0.001
                    && instance.rect[0] >= rect.x
                    && instance.rect[0] < rect.x + rect.w
            })
            .map(|instance| instance.rect)
            .collect();
        assert_eq!(
            bars.len(),
            200,
            "scrubber={rect:?}; translucent={:?}; first={:?}",
            ui.layers()
                .flat_map(|l| l.instances.iter())
                .filter(|i| i.color[3] == 107)
                .take(4)
                .map(|i| i.rect)
                .collect::<Vec<_>>(),
            ui.layers()
                .flat_map(|l| l.instances.iter())
                .take(8)
                .map(|i| i.rect)
                .collect::<Vec<_>>()
        );
        assert!(
            bars[..100]
                .iter()
                .all(|bar| (bar[3] - bar[1] - 2.0).abs() < 0.001)
        );
        assert!(
            bars[100..]
                .iter()
                .all(|bar| (bar[3] - bar[1] - 36.0).abs() < 0.001)
        );
        for round in 0..24 {
            let pointer = |kind, fraction: f64| Event {
                kind,
                x: rect.x as f64 + rect.w as f64 * fraction,
                y: rect.y as f64 + 20.0,
                ..Default::default()
            };
            let inputs = [
                pointer(input::wire::POINTER_PRESSED, 0.25),
                pointer(input::wire::POINTER_MOVED, 1.2),
                pointer(input::wire::POINTER_RELEASED, 0.75),
                Event {
                    kind: input::wire::KEY_PRESSED,
                    code: input::Key::ArrowRight.code(),
                    ..Default::default()
                },
                Event {
                    kind: input::wire::KEY_RELEASED,
                    code: input::Key::ArrowRight.code(),
                    ..Default::default()
                },
            ];
            mem_sys::local_reset();
            for (step, event) in inputs.iter().enumerate() {
                let mut intent = None;
                frame(
                    &mut ui,
                    &player,
                    20.0 + (round * 5 + step) as f64 * 0.01,
                    std::slice::from_ref(event),
                    &mut intent,
                );
                assert_eq!(
                    intent,
                    match step {
                        2 => Some(Intent::SeekFraction(0.75)),
                        3 => Some(Intent::SeekFraction(65.0 / 240.0)),
                        _ => None,
                    }
                );
                if step == 1 {
                    assert!(
                        ui.layers()
                            .flat_map(|layer| layer.instances.iter())
                            .any(|instance| {
                                (instance.rect[2] - instance.rect[0] - rect.w / 400.0).abs() < 0.001
                                    && (instance.rect[3] - instance.rect[1] - 40.0).abs() < 0.001
                                    && (instance.rect[0] - (rect.x + rect.w - rect.w / 800.0)).abs()
                                        < 0.001
                            }),
                        "held drag previews at the clamped end before release"
                    );
                }
            }
            if round >= 2 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm waveform input/build/draw across the ABI"
                );
            }
        }
    }

    #[test]
    fn row_number_labels_cover_zero_and_the_largest_unsigned_value_on_the_stack() {
        mem_sys::local_reset();
        for (value, expected) in [
            (0, "0"),
            (1, "1"),
            (10000, "10000"),
            (u64::MAX, "18446744073709551615"),
        ] {
            assert_eq!(stack_fmt(&mut [0u8; 24], format_args!("{value}")), expected);
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }

    #[test]
    fn pressed_visual_scales_without_shrinking_hit_area_or_allocating_when_warm() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        ui.set_theme(&ThemeSource {
            source: Str {
                ptr: crate::LIGHT.as_ptr(),
                len: crate::LIGHT.len(),
            },
            ..ThemeSource::default()
        })
        .unwrap();
        let clicked = Cell::new(false);
        let mut build = |ui: &mut Ui| {
            clicked.set(ui.icon_button("test-button", icon_drawing(Icon::Next), "Next", &icon_key(64.0, 40.0).look(&ICON_PLATE_LOOKS)).clicked);
        };
        let input = |time, events: &[Event]| FrameIn {
            width: 320.0,
            height: 240.0,
            time,
            events: events.as_ptr(),
            event_count: events.len(),
            ..FrameIn::default()
        };
        for time in [0.0, 0.1] {
            crate::testing::frame(&mut ui, &input(time, &[]), &mut build).unwrap();
        }
        let key = pxui::ui::key_of(ROOT_KEY, "test-button");
        let button = ui.box_of_key(key).unwrap();
        let surface = ui
            .box_of_key(pxui::ui::key_of(pxui::ui::key_of(key, "visual"), "surface"))
            .unwrap();
        let original = ui.rect(&button);
        let pointer = |kind| Event {
            kind,
            x: 0.5,
            y: 20.0,
            ..Event::default()
        };
        for round in 0..3 {
            let base = 0.2 + round as f64 * 2.0;
            mem_sys::local_reset();
            crate::testing::frame(
                &mut ui,
                &input(
                    base,
                    &[
                        pointer(input::wire::POINTER_MOVED),
                        pointer(input::wire::POINTER_PRESSED),
                    ],
                ),
                &mut build,
            )
            .unwrap();
            assert!(!clicked.get());
            crate::testing::frame(&mut ui, &input(base + 0.08, &[]), &mut build).unwrap();
            let shrunk = ui.rect(&surface);
            assert!((shrunk.w - original.w * 0.96).abs() < 0.001);
            assert!((shrunk.x - original.w * 0.02).abs() < 0.001);
            assert_eq!(ui.rect(&button), original);
            assert_eq!(
                ui.a11y_nodes()
                    .iter()
                    .find(|n| ui.a11y_name_of(n) == "Next")
                    .unwrap()
                    .rect,
                original
            );
            // Release outside the shrunk visual but inside the original button.
            crate::testing::frame(
                &mut ui,
                &input(base + 0.1, &[pointer(input::wire::POINTER_RELEASED)]),
                &mut build,
            )
            .unwrap();
            assert!(clicked.get());
            crate::testing::frame(&mut ui, &input(base + 0.34, &[]), &mut build).unwrap();
            assert_eq!(ui.rect(&surface), original);
            let allocations = mem_sys::local_alloc_count();
            assert!(
                round == 0 || allocations == 0,
                "warm button press allocated {allocations} times"
            );
        }
        let reduce = "tokens.reduced_motion = true";
        ui.set_theme(&ThemeSource {
            flags: ThemeSource::REDUCED_MOTION,
            source: Str {
                ptr: reduce.as_ptr(),
                len: reduce.len(),
            },
            ..ThemeSource::default()
        })
        .unwrap();
        assert!(ui.theme().tokens.reduced_motion);
        crate::testing::frame(
            &mut ui,
            &input(7.0, &[pointer(input::wire::POINTER_PRESSED)]),
            &mut build,
        )
        .unwrap();
        assert!(
            (ui.rect(&surface).w - original.w * 0.96).abs() < 0.001,
            "reduced motion: surface {:?}, button held {}",
            ui.rect(&surface),
            ui.signal(&button).held
        );
        crate::testing::frame(
            &mut ui,
            &input(7.01, &[pointer(input::wire::POINTER_RELEASED)]),
            &mut build,
        )
        .unwrap();
        assert!(clicked.get());
        assert_eq!(ui.rect(&surface), original);
    }
}
