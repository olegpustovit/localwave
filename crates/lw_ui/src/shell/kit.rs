use super::*;

#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub(super) paper: Rgba,
    pub(super) panel: Rgba,
    pub(super) selected: Rgba,
    pub(super) ink: Rgba,
    pub(super) secondary: Rgba,
    pub(super) tertiary: Rgba,
    pub(super) line: Rgba,
    pub(super) key: Rgba,
    pub(super) signal: Rgba,
    pub(super) danger: Rgba,
}
pub(super) const fn rgb(hex: u32) -> Rgba {
    Rgba {
        r: (hex >> 16) as u8,
        g: (hex >> 8) as u8,
        b: hex as u8,
        a: 255,
    }
}
pub(super) fn palette(ui: &Ui) -> Palette {
    let dark = ui.theme().tokens.foreground.r > 128;
    if dark {
        Palette {
            paper: rgb(0x101010),
            panel: rgb(0x1A1A1A),
            selected: rgb(0x2A2A2A),
            ink: rgb(0xEDEDED),
            secondary: rgb(0x9B9B9B),
            tertiary: rgb(0x5A5A5A),
            line: rgb(0x262626),
            key: rgb(0x1F1F1F),
            signal: ui.theme().tokens.accent,
            danger: rgb(0xD21A1A),
        }
    } else {
        Palette {
            paper: rgb(0xF3F3F3),
            panel: rgb(0xE6E6E6),
            selected: rgb(0xD3D3D3),
            ink: rgb(0x111111),
            secondary: rgb(0x585858),
            tertiary: rgb(0xA0A0A0),
            line: rgb(0xDADADA),
            key: rgb(0xFBFBFB),
            signal: ui.theme().tokens.accent,
            danger: rgb(0xD21A1A),
        }
    }
}
pub(super) fn container(
    ui: &mut Ui,
    key: &str,
    axis: Axis,
    w: Size,
    h: Size,
    fill: Rgba,
) -> Handle {
    ui.node(
        key,
        Node {
            axis,
            w,
            h,
            look: Look { fill, ..Look::NONE },
            ..Node::DEFAULT
        },
    )
}
pub(super) fn text(
    ui: &mut Ui,
    key: &str,
    value: &str,
    size: i64,
    color: Rgba,
    dimensions: (Size, f32),
    mono: bool,
) -> Handle {
    let style = pxui::TextStyle {
        font: u16::from(mono),
        size_points: size as f32,
        line_height: size as f32 * 1.4,
        max_lines: 1,
        ellipsis: pxui::TextStyle::ELLIPSIS_END,
        ..Default::default()
    };
    let node = ui.node(
        key,
        Node {
            flags: FIT_TEXT | FIT_TEXT_PIXEL,
            w: dimensions.0,
            h: Size::Px(dimensions.1),
            role: Role::Label,
            look: Look {
                text: color,
                font: Font {
                    family: mono as u16,
                    size,
                    ..Font::BODY
                },
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.text_line_box(&node, value, &style, &[]).unwrap();
    node
}
pub(super) fn spacer(ui: &mut Ui, key: &str, height: f32) {
    ui.box_(key, BoxOpts::default().h(Size::Px(height)));
}
pub(super) fn rule(ui: &mut Ui, key: &str, vertical: bool, p: Palette) {
    container(
        ui,
        key,
        Axis::Column,
        if vertical { Size::Px(1.0) } else { Size::Grow },
        if vertical { Size::Grow } else { Size::Px(1.0) },
        p.line,
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
        && let Some(image) = images.get_at(row.cover_art_id, path, points, node.box_key)
    {
        ui.draw(node, move |paint, rect| {
            paint.image_fit(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                &image,
                Fit::Cover,
                Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            )
        });
    }
}

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Play,
    Pause,
    Previous,
    Next,
    Stop,
    Queue,
    Panel,
    Shuffle,
    Repeat,
    Refresh,
    Clear,
}
pub(super) fn icon_button(
    ui: &mut Ui,
    key: &str,
    label: &str,
    icon: Icon,
    dimensions: (f32, f32),
    inverse: bool,
    p: Palette,
) -> bool {
    let node = ui.node(
        key,
        Node {
            flags: CLICKABLE | TAB_STOP,
            w: Size::Px(dimensions.0),
            h: Size::Px(dimensions.1),
            role: Role::Button,
            ..Node::DEFAULT
        },
    );
    let signal = ui.signal_state(&node);
    ui.a11y_name(&node, label);
    let fill = if inverse {
        p.ink
    } else if signal.hovered || signal.held {
        p.panel
    } else {
        p.key
    };
    let color = if inverse { p.paper } else { p.ink };
    let revision = pxui::hash!((icon as u8, inverse, signal.hovered, signal.held));
    ui.within(&node, &mut |ui| {
        let visual = ui.view("visual", revision, Size::Grow, Size::Grow, &mut |ui| {
            let surface = ui.node(
                "surface",
                Node {
                    w: Size::Grow,
                    h: Size::Grow,
                    ..Node::DEFAULT
                },
            );
            ui.set_look(
                &surface,
                Look { fill, ..Look::NONE },
                State {
                    hovered: signal.hovered,
                    pressed: signal.held,
                    ..State::default()
                },
            );
            ui.draw(&surface, move |paint, rect| {
                let x = rect.x + (rect.w - 20.0) / 2.0;
                let y = rect.y + (rect.h - 20.0) / 2.0;
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
                        paint.fill(x + 4.0, y + 3.0, 4.0, 14.0, color);
                        paint.fill(x + 12.0, y + 3.0, 4.0, 14.0, color);
                    }
                    Icon::Stop => paint.fill(x + 5.0, y + 5.0, 10.0, 10.0, color),
                    Icon::Queue => {
                        for dy in [5.0, 10.0, 15.0] {
                            paint.fill(x + 2.0, y + dy, 2.0, 1.5, color);
                            paint.fill(x + 6.0, y + dy, 12.0, 1.5, color);
                        }
                    }
                    Icon::Previous | Icon::Next => {
                        let next = matches!(icon, Icon::Next);
                        paint.fill(x + if next { 15.0 } else { 3.0 }, y + 4.0, 2.0, 12.0, color);
                        for row in 0..12 {
                            let w = (6.0 - (row as f32 - 5.5).abs()) * 1.5;
                            paint.fill(
                                x + if next { 4.0 } else { 15.0 - w },
                                y + 4.0 + row as f32,
                                w,
                                1.0,
                                color,
                            );
                        }
                    }
                    Icon::Panel => {
                        paint.stroke(x + 2.0, y + 3.0, 16.0, 14.0, color);
                        paint.line((x + 12.0, y + 3.0), (x + 12.0, y + 17.0), color);
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
                    Icon::Repeat => {
                        paint.line((x + 3.0, y + 5.0), (x + 17.0, y + 5.0), color);
                        paint.line((x + 17.0, y + 5.0), (x + 17.0, y + 15.0), color);
                        paint.line((x + 17.0, y + 15.0), (x + 3.0, y + 15.0), color);
                        paint.line((x + 3.0, y + 15.0), (x + 3.0, y + 9.0), color);
                    }
                    Icon::Refresh => {
                        paint.line((x + 4.0, y + 5.0), (x + 16.0, y + 5.0), color);
                        paint.line((x + 16.0, y + 5.0), (x + 16.0, y + 15.0), color);
                        paint.line((x + 16.0, y + 15.0), (x + 4.0, y + 15.0), color);
                        paint.line((x + 4.0, y + 15.0), (x + 4.0, y + 10.0), color);
                        paint.line((x + 4.0, y + 5.0), (x + 8.0, y + 2.0), color);
                        paint.line((x + 4.0, y + 5.0), (x + 8.0, y + 8.0), color);
                    }
                    Icon::Clear => {
                        paint.line((x + 5.0, y + 5.0), (x + 15.0, y + 15.0), color);
                        paint.line((x + 15.0, y + 5.0), (x + 5.0, y + 15.0), color);
                    }
                }
            });
        });
        if ui.state(&visual, 0) != i64::from(signal.held) {
            ui.set_state(&visual, i64::from(signal.held));
            let scale = if signal.held { 0.96 } else { 1.0 };
            ui.animate(
                &visual,
                Target::Scale {
                    x: scale,
                    y: scale,
                    anchor: (0.5, 0.5),
                },
                crate::motion::spring(if signal.held {
                    crate::motion::INSTANT
                } else {
                    crate::motion::BASE
                }),
            );
        }
    });
    signal.clicked
}

// SwiftUI wraps table labels, then fits their line box to physical pixels.
pub(super) fn heading_size(ui: &mut Ui, value: &str, available: f32, natural: f32) -> (f32, f32) {
    table_text_size(
        ui,
        value,
        available,
        natural,
        &pxui::TextStyle {
            size_points: 12.0,
            line_height: 16.8,
            weight: 500,
            wrap: pxui::TextStyle::WRAP_WORD_SPACES,
            ..Default::default()
        },
    )
}
pub(super) fn table_text_size(
    ui: &mut Ui,
    value: &str,
    available: f32,
    natural: f32,
    style: &pxui::TextStyle,
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
        style.line_height * lines.max(1) as f32,
    )
}

#[cfg(test)]
pub(super) mod button_tests {
    use super::*;
    use pxui::{Event, FrameIn, Platform, Str, ThemeSource};
    use std::cell::Cell;
    #[test]
    fn matrix_digits_match_swift_geometry_stagger_changed_rows_and_stop_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
                    ui.frame(
                        &FrameIn {
                            width: 400.0,
                            height: 80.0,
                            time: time + offset,
                            ..Default::default()
                        },
                        &mut |ui| matrix_clock(ui, seconds, ink),
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
    fn volume_cells_preview_and_adjust_by_reference_steps_without_warm_allocations() {
        use pxui::{Event, FrameIn, Platform};
        mem_sys::set_peer(pxui::library_counter);
        let player = PlayerView {
            volume: 0.5,
            ..Default::default()
        };
        fn frame(
            ui: &mut Ui,
            player: &PlayerView<'_>,
            scale: f32,
            events: &[Event],
            intent: &mut Option<Intent>,
        ) {
            ui.frame(
                &FrameIn {
                    width: 320.0,
                    height: 44.0,
                    scale,
                    events: events.as_ptr(),
                    event_count: events.len(),
                    ..Default::default()
                },
                &mut |ui| {
                    volume(ui, player, Size::Px(82.0), palette(ui), intent);
                },
            )
            .unwrap();
        }
        for scale in [1.0, 2.0] {
            let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
            ui.set_a11y(true);
            frame(&mut ui, &player, scale, &[], &mut None);
            let p = palette(&ui);
            let cells: Vec<_> = ui
                .layers()
                .flat_map(|layer| layer.instances.iter())
                .filter(|quad| {
                    (quad.rect[2] - quad.rect[0] - 5.0).abs() < 0.001
                        && (quad.rect[3] - quad.rect[1] - 10.0).abs() < 0.001
                })
                .copied()
                .collect();
            assert_eq!(cells.len(), 12);
            for (index, cell) in cells.iter().enumerate() {
                assert_eq!(
                    cell.rect,
                    [index as f32 * 7.0, 17.0, index as f32 * 7.0 + 5.0, 27.0]
                );
                let color = if index < 6 { p.ink } else { p.selected };
                assert_eq!(cell.color, [color.r, color.g, color.b, color.a]);
            }
            let id = ui
                .a11y_nodes()
                .iter()
                .find(|node| ui.a11y_name_of(node) == "Volume")
                .unwrap()
                .id;
            let pointer = |kind, x| Event {
                kind,
                x,
                y: 22.0,
                ..Default::default()
            };
            let key = |key: input::Key| Event {
                kind: input::wire::KEY_PRESSED,
                code: key.code(),
                ..Default::default()
            };
            for round in 0..8 {
                mem_sys::local_reset();
                for (event, expected) in [
                    (pointer(input::wire::POINTER_PRESSED, 20.5), 0.25),
                    (pointer(input::wire::POINTER_MOVED, 61.5), 0.75),
                    (pointer(input::wire::POINTER_RELEASED, 82.0), 1.0),
                    (key(input::Key::ArrowRight), 0.5 + 1.0 / 12.0),
                    (key(input::Key::ArrowLeft), 0.5 - 1.0 / 12.0),
                    (key(input::Key::Home), 0.0),
                    (key(input::Key::End), 1.0),
                    (key(input::Key::PageUp), 1.0),
                    (key(input::Key::PageDown), 0.0),
                ] {
                    let mut intent = None;
                    frame(
                        &mut ui,
                        &player,
                        scale,
                        std::slice::from_ref(&event),
                        &mut intent,
                    );
                    assert_eq!(intent, Some(Intent::Volume(expected)));
                }
                let mut intent = None;
                frame(
                    &mut ui,
                    &player,
                    scale,
                    &[pointer(input::wire::POINTER_PRESSED, 20.5)],
                    &mut intent,
                );
                let events = [
                    pointer(input::wire::POINTER_RELEASED, 61.5),
                    pointer(input::wire::POINTER_PRESSED, 20.5),
                ];
                intent = None;
                frame(&mut ui, &player, scale, &events, &mut intent);
                assert_eq!(
                    intent,
                    Some(Intent::Volume(0.25)),
                    "live preview wins over an older release in the same batch"
                );
                intent = None;
                frame(
                    &mut ui,
                    &player,
                    scale,
                    &[pointer(input::wire::POINTER_CANCELLED, 20.5)],
                    &mut intent,
                );
                assert!(intent.is_none());
                ui.inject_a11y(&[pxui::A11yAction {
                    id,
                    action_code: 4,
                    payload: 0.25,
                    ..Default::default()
                }]);
                let mut intent = None;
                frame(&mut ui, &player, scale, &[], &mut intent);
                assert_eq!(intent, Some(Intent::Volume(0.25)));
                if round > 1 {
                    assert_eq!(
                        mem_sys::local_alloc_count(),
                        0,
                        "warm volume input and cells"
                    );
                }
            }
        }
    }

    #[test]
    fn waveform_bars_preview_drag_seek_on_release_and_keys_without_warm_allocations() {
        use pxui::{Event, FrameIn, Platform};
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut peaks = [0.0; 512];
        peaks[256..].fill(1.0);
        let player = PlayerView {
            waveform: Some(&peaks),
            position: 60.0,
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
            ui.frame(
                &FrameIn {
                    width: 1440.0,
                    height: 71.0,
                    time,
                    events: events.as_ptr(),
                    event_count: events.len(),
                    ..Default::default()
                },
                &mut |ui| {
                    let p = palette(ui);
                    transport(ui, player, p, intent);
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
            assert_eq!(
                std::str::from_utf8(ClockLabel::integer(value).bytes()).unwrap(),
                expected
            );
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }

    #[test]
    fn pressed_visual_scales_without_shrinking_hit_area_or_allocating_when_warm() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
            let p = palette(ui);
            clicked.set(icon_button(
                ui,
                "test-button",
                "Next",
                Icon::Next,
                (64.0, 40.0),
                false,
                p,
            ));
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
            ui.frame(&input(time, &[]), &mut build).unwrap();
        }
        let key = Ui::key_of(ROOT_KEY, "test-button");
        let button = ui.box_of_key(key).unwrap();
        let surface = ui
            .box_of_key(Ui::key_of(Ui::key_of(key, "visual"), "surface"))
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
            ui.frame(
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
            ui.frame(&input(base + 0.08, &[]), &mut build).unwrap();
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
            ui.frame(
                &input(base + 0.1, &[pointer(input::wire::POINTER_RELEASED)]),
                &mut build,
            )
            .unwrap();
            assert!(clicked.get());
            ui.frame(&input(base + 0.34, &[]), &mut build).unwrap();
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
        ui.frame(
            &input(7.0, &[pointer(input::wire::POINTER_PRESSED)]),
            &mut build,
        )
        .unwrap();
        assert!(
            (ui.rect(&surface).w - original.w * 0.96).abs() < 0.001,
            "reduced motion: surface {:?}, button held {}",
            ui.rect(&surface),
            ui.signal_state(&button).held
        );
        ui.frame(
            &input(7.01, &[pointer(input::wire::POINTER_RELEASED)]),
            &mut build,
        )
        .unwrap();
        assert!(clicked.get());
        assert_eq!(ui.rect(&surface), original);
    }
}
