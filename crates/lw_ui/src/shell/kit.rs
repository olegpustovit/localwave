use pxui::bodies::BodyConfig;
use super::*;

// LocalWave's colour tokens, read with `ui.color` and `paint.color`. The accent is the theme's own.
pub(super) const PAPER: ColorToken = ColorToken(0);
pub(super) const PANEL: ColorToken = ColorToken(1);
pub(super) const SELECTED: ColorToken = ColorToken(2);
pub(super) const INK: ColorToken = ColorToken(3);
pub(super) const SECONDARY: ColorToken = ColorToken(4);
pub(super) const TERTIARY: ColorToken = ColorToken(5);
pub(super) const LINE: ColorToken = ColorToken(6);
pub(super) const KEY: ColorToken = ColorToken(7);
pub(super) const DANGER: ColorToken = ColorToken(8);
/// Swift's `key2`: the play, shuffle and option keys' plate.
pub(super) const KEY2: ColorToken = ColorToken(9);
pub(super) const fn rgb(hex: u32) -> Rgba {
    Rgba::rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
/// The (light, dark) pair of each token, in token order.
pub(super) const COLORS: [(Rgba, Rgba); 10] = [
    (rgb(0xF3F3F3), rgb(0x101010)),
    (rgb(0xE6E6E6), rgb(0x1A1A1A)),
    (rgb(0xD3D3D3), rgb(0x2A2A2A)),
    (rgb(0x111111), rgb(0xEDEDED)),
    (rgb(0x585858), rgb(0x9B9B9B)),
    (rgb(0xA0A0A0), rgb(0x5A5A5A)),
    (rgb(0xDADADA), rgb(0x262626)),
    (rgb(0xFBFBFB), rgb(0x1F1F1F)),
    (rgb(0xD21A1A), rgb(0xD21A1A)),
    (rgb(0xDADADA), rgb(0x2E2E2E)),
];
/// Gives a root or scoped theme LocalWave's palette for the selected appearance.
pub fn apply_palette(theme: &mut pxui::ThemeHandle, dark: bool) {
    for (index, (light, night)) in COLORS.into_iter().enumerate() {
        theme.set_color(pxui::theme::ColorToken(index as u16), if dark { night } else { light });
    }
}
pub(super) fn container(
    ui: &mut Ui,
    key: &'static str,
    axis: Axis,
    w: Size,
    h: Size,
    look: &'static Look,
) -> Handle {
    ui.node(
        key,
        Node::column().w(w).h(h).axis(axis).look(*look),
    )
}
pub(super) fn spacer(ui: &mut Ui, key: &'static str, height: f32) {
    ui.node(key, Node::DEFAULT.h(Size::Px(height)));
}
pub(super) fn rule(ui: &mut Ui, key: &'static str, vertical: bool) {
    container(
        ui,
        key,
        Axis::Column,
        if vertical { Size::Px(1.0) } else { Size::Grow },
        if vertical { Size::Grow } else { Size::Px(1.0) },
        &LINE_FILL,
    );
}

pub(super) fn paint_artwork(
    ui: &mut Ui,
    node: &Handle,
    row: Option<crate::catalog::Row<'_>>,
    points: f32,
    images: Option<&crate::artwork::Images>,
) {
    ui.set_role(node, Role::Image, pxui::A11yStates::NONE);
    if let (Some(row), Some(images)) = (row, images)
        && let Some(path) = row.cover_art_path
        && let Some(image) = images.get(row.cover_art_id, path, points, Some(node))
    {
        // a descriptor that went bad between frames draws nothing (`Paint::image` refuses it)
        ui.canvas(node, move |paint, rect| {
            let _ = paint.image(rect, &image, &COVER_ART);
        });
    }
}

/// Cover art fills its square, cropped, untinted.
pub(super) const COVER_ART: pxui::ImageStyle = pxui::ImageStyle::DEFAULT.fit(Fit::Cover);

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
    /// Keep offline (an arrow into a tray).
    Offline,
    /// Kept offline.
    Kept,
    /// Repeat one: the repeat arrows with a mono "1" at their lower right.
    RepeatOne,
    /// Not a favourite: an outlined heart.
    Heart,
    /// A favourite: a filled heart.
    HeartFilled,
    /// A song row's actions: three small dots, Swift's 2.8-point ellipsis drawn at two thirds.
    RowMore,
    /// Add (to a list): a 14-point plus of 1.5-point bars.
    Add,
    /// Swift's LW-search vector (a lens), scaled to the box it is given.
    Search,
    /// Swift's LW-plus vector, scaled to the box it is given (the sidebar's add rows, Add music).
    Plus,
    /// Swift's LW-folder vector, scaled to the box it is given (the onboarding's Choose a folder).
    Folder,
    /// Swift's LW-route vector, scaled to the box it is given (the audio route key).
    Route,
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
        Icon::Queue => crate::icons::queue(paint, Rect::new(x, y, 20.0, 20.0), color),
        Icon::Previous | Icon::Next => {
            let next = matches!(icon, Icon::Next);
            paint.fill(Rect::new(x + if next { 15.0 } else { 3.0 }, y + 4.0, 2.0, 12.0), Fill::new(color));
            for row in 0..12 {
                let w = (6.0 - (row as f32 - 5.5).abs()) * 1.5;
                paint.fill(Rect::new(x + if next { 4.0 } else { 15.0 - w }, y + 4.0 + row as f32, w, 1.0), Fill::new(color));
            }
        }
        Icon::Panel => crate::icons::side(paint, Rect::new(x, y, 20.0, 20.0), color),
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
        Icon::Clear => crate::icons::close(paint, Rect::new(x, y, 20.0, 20.0), color),
        Icon::More => crate::icons::more(paint, Rect::new(x, y, 20.0, 20.0), color),
        Icon::Settings => crate::icons::settings(paint, Rect::new(x, y, 20.0, 20.0), color),
        Icon::Back => crate::icons::back(paint, Rect::new(x, y, 20.0, 20.0), color),
        Icon::Check => {
            paint.line((x + 4.0, y + 10.0), (x + 8.0, y + 14.0), Stroke::new(color));
            paint.line((x + 8.0, y + 14.0), (x + 16.0, y + 5.0), Stroke::new(color));
        }
        Icon::Add => {
            paint.fill(Rect::new(x + 3.0, y + 9.25, 14.0, 1.5), Fill::new(color));
            paint.fill(Rect::new(x + 9.25, y + 3.0, 1.5, 14.0), Fill::new(color));
        }
        Icon::Search => crate::icons::search(paint, Rect::new(x, y, 20.0, 20.0), color),
        Icon::Plus => crate::icons::plus(paint, Rect::new(x, y, 20.0, 20.0), color),
        Icon::Folder => crate::icons::folder(paint, Rect::new(x, y, 20.0, 20.0), color),
        Icon::Route => crate::icons::route(paint, Rect::new(x, y, 20.0, 20.0), color),
        Icon::Offline | Icon::Kept => crate::icons::offline(paint, Rect::new(x, y, 20.0, 20.0), matches!(icon, Icon::Kept), color),
        Icon::Heart | Icon::HeartFilled => crate::icons::heart(paint, Rect::new(x, y, 20.0, 20.0), matches!(icon, Icon::HeartFilled), color),
        Icon::RepeatOne => {
            crate::icons::repeat(paint, Rect { x, y, w: 20.0, h: 20.0 }, color);
            // the "1" in the icon's ink, 19 right and 15 down in an 8 by 12 box (where the key's label sat)
            let one = [pxui::TextSpan { start: 0, end: 1, ..Default::default() }.color(color)];
            let _ = paint.text_spans(pxui::Point::new(x + 19.0, y + 15.0), REPEAT_ONE, 8.0, "1", &one);
        }
        Icon::RowMore => {
            let color = Rgba::token(SECONDARY);
            for dx in [5.5, 12.0, 18.5] {
                paint.circle((x + 2.0 + dx * (2.0 / 3.0), y + 10.0), 1.4 * (2.0 / 3.0), Some(Fill::new(color)), None);
            }
        }
        Icon::Down => {
            paint.line((x + 3.0, y + 7.0), (x + 10.0, y + 14.0), Stroke::new(color));
            paint.line((x + 10.0, y + 14.0), (x + 17.0, y + 7.0), Stroke::new(color));
        }
    }
}
/// Every kit icon, indexed by its discriminant (the word an icon button's drawing carries).
const ICONS: [Icon; 26] = [
    Icon::Play, Icon::Pause, Icon::Previous, Icon::Next, Icon::Queue, Icon::Panel, Icon::Shuffle, Icon::Repeat,
    Icon::Refresh, Icon::Clear, Icon::Down, Icon::More, Icon::Settings, Icon::Back, Icon::Check, Icon::Offline, Icon::Kept,
    Icon::RepeatOne, Icon::Heart, Icon::HeartFilled, Icon::RowMore, Icon::Add, Icon::Search, Icon::Plus, Icon::Folder, Icon::Route,
];
/// The repeat-one mark: Geist Mono 9 in a 12-point line.
const REPEAT_ONE: Label = MONO_9.w_px(8.0).h_px(12.0);
// the table is indexed by discriminant: keep it in declaration order
const _: () = {
    let mut i = 0;
    while i < ICONS.len() {
        assert!(ICONS[i] as usize == i);
        i += 1;
    }
};
/// The one drawing of every kit icon button: `word`'s low byte names the icon, the bits above it a size in quarter points
/// (`icon_sized`). Play, Shuffle, the offline marks and Swift's vectors (lens, plus, side, heart) fill the square they are given
/// (`Button::icon_size`, or the sized square about its centre; close too); the others draw in a 20-point square at its origin.
fn draw_icon(paint: &mut pxui::Paint<'_>, at: pxui::geom::Rect, ink: Rgba, word: u32) {
    let points = (word >> 8) as f32 / 4.0;
    let at = if points > 0.0 {
        pxui::geom::Rect { x: at.x + (at.w - points) / 2.0, y: at.y + (at.h - points) / 2.0, w: points, h: points }
    } else {
        at
    };
    match ICONS.get((word & 0xff) as usize) {
        Some(Icon::More) => crate::icons::more(paint, at, ink),
        Some(Icon::Play) => crate::icons::play(paint, at, ink),
        Some(icon @ (Icon::Previous | Icon::Next)) if points > 0.0 => {
            crate::icons::skip(paint, at, matches!(icon, Icon::Next), ink);
        }
        Some(Icon::Shuffle) => crate::icons::shuffle(paint, at, ink),
        Some(Icon::Repeat) => crate::icons::repeat(paint, at, ink),
        Some(Icon::Offline) => crate::icons::offline(paint, at, false, ink),
        Some(Icon::Kept) => crate::icons::offline(paint, at, true, ink),
        Some(Icon::Search) => crate::icons::search(paint, at, ink),
        Some(Icon::Plus) => crate::icons::plus(paint, at, ink),
        Some(Icon::Folder) => crate::icons::folder(paint, at, ink),
        Some(Icon::Settings) => crate::icons::settings(paint, at, ink),
        Some(Icon::Route) => crate::icons::route(paint, at, ink),
        Some(Icon::Panel) => crate::icons::side(paint, at, ink),
        Some(Icon::Clear) => crate::icons::close(paint, at, ink),
        Some(Icon::Back) => crate::icons::back(paint, at, ink),
        Some(Icon::Heart) => crate::icons::heart(paint, at, false, ink),
        Some(Icon::HeartFilled) => crate::icons::heart(paint, at, true, ink),
        Some(&icon) => paint_icon(paint, icon, at.x, at.y, ink),
        None => {}
    }
}
/// `icon` as a kit icon button's drawing, painted into the 20-point square it is given in the look's ink.
pub(super) const fn icon_drawing(icon: Icon) -> pxui::Icon {
    pxui::Icon::draw(draw_icon, icon as u32)
}
/// `icon` drawn at Swift's `points` (an `LWIcon` size) about the centre of the square it is given.
pub(super) const fn icon_sized(icon: Icon, points: f32) -> pxui::Icon {
    pxui::Icon::draw(draw_icon, icon as u32 | ((points * 4.0) as u32) << 8)
}

// SwiftUI wraps table labels, then fits their line box to physical pixels.
pub(super) fn heading_size(ui: &mut Ui, value: &str, available: f32, natural: f32) -> (f32, f32) {
    let density = ui.density();
    table_text_size(
        ui,
        density,
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
    density: f32,
    value: &str,
    available: f32,
    natural: f32,
    style: &pxui::Label,
    line_height: f32,
) -> (f32, f32) {
    let (width, lines) = if natural > available {
        let metrics = ui
            .text_service()
            .measure(value, *style, Some(available.max(0.01)))
            .unwrap();
        (metrics.width, metrics.lines)
    } else {
        (natural, 1)
    };
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
    let state = State::from(c.state);
    let (enabled, selected) = (!state.disabled, state.checked);
    ui.set_node_layout(&c.row, &Node::column().w(Size::Grow).h_px(c.row_h).pad_x(12.0));
    let ink = if c.danger != 0 { DANGER } else { INK };
    if c.variant == 0 {
        let look = MENU_ROW.text(Rgba::token(ink));
        ui.style(&c.row, &pxui::theme::States::of(look), State::default().disabled(!enabled));
    } else {
        let fill = Rgba::token(if selected { SELECTED } else { PAPER });
        let look = MENU_ROW.fill(fill).text(Rgba::token(if enabled { ink } else { SECONDARY }));
        ui.style(&c.row, &pxui::theme::States::of(look), State::default());
    }
    ui.text(&c.row, c.text_of(&c.text));
    use pxui::A11yStates;
    match c.variant {
        MENU_PANEL => ui.set_role(&c.row, Role::MenuItem, if enabled { A11yStates::ENABLED } else { A11yStates::NONE }),
        MENU_CHOICE => ui.set_role(&c.row, Role::Toggle, A11yStates::CHECKABLE.with(if selected { A11yStates::CHECKED } else { A11yStates::NONE })),
        _ => {}
    }
}
/// A context menu row's 13-point text.
const MENU_ROW: Look = Look::NONE.font(Font::BODY.size(13.0));
#[cfg(test)]
pub(super) mod button_tests {
    use super::*;
    use pxui::{Event, FrameIn, Platform, ThemeHandle};
    use std::cell::Cell;
    #[test]
    fn matrix_clock_reuses_rows_when_only_the_seconds_digit_advances() {
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.verify_views(false);
        for (index, seconds) in [753, 754, 755].into_iter().enumerate() {
            let input = FrameIn::new(400.0 + index as f32, 80.0).time(index as f64);
            crate::testing::frame(&mut ui, &input, &mut |ui| matrix_clock(ui, seconds, 43.0, INK)).unwrap();
        }
        assert_eq!(ui.rebuilt_named("stable"), 0);
        assert!(ui.rebuilt_named("changed") > 0);
    }

    #[test]
    fn matrix_digits_match_swift_geometry_stagger_changed_rows_and_stop_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let ink = Rgba::rgb(12, 13, 14);
        // The clock takes a colour token: INK is this test's ink, so its dots stay findable by colour.
        let mut theme = ThemeHandle::parse(include_str!("../../themes/app-light.theme")).unwrap();
        crate::apply_palette(&mut theme, false);
        theme.set_color(INK, ink);
        ui.set_theme(&theme).unwrap();
        let mut time = 0.0;
        for cycle in 0..4 {
            mem_sys::local_reset();
            for seconds in [0, 1, 59, 60, 3599, 3600, 59999, 60000] {
                time += 1.0;
                for offset in [0.0, 0.069, 0.07, 0.139, 0.14, 0.235, 0.236, 0.3] {
                    crate::testing::frame(
                        &mut ui,
                        &FrameIn::new(400.0, 80.0).time(time + offset),
                        &mut |ui| matrix_clock(ui, seconds, 43.0, INK),
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
    fn waveform_bars_preview_drag_seek_on_release_without_warm_allocations() {
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
                &{
                    let mut frame = FrameIn::new(1440.0, 71.0).time(time);
                    frame = frame.events(&events[..]);
                    frame
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
            ];
            mem_sys::local_reset();
            for (step, event) in inputs.iter().enumerate() {
                let mut intent = None;
                frame(
                    &mut ui,
                    &player,
                    20.0 + (round * 3 + step) as f64 * 0.01,
                    std::slice::from_ref(event),
                    &mut intent,
                );
                assert_eq!(
                    intent,
                    match step {
                        2 => Some(Intent::SeekFraction(0.75)),
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
        let mut light = ThemeHandle::parse(crate::LIGHT).unwrap();
        apply_palette(&mut light, false);
        ui.set_theme(&light).unwrap();
        let clicked = Cell::new(false);
        let mut build = |ui: &mut Ui| {
            clicked.set(ui.icon_button("test-button", icon_drawing(Icon::Next), "Next", &icon_key(64.0, 40.0).look(&ICON_PLATE_LOOKS)).clicked);
        };
        fn input(time: f64, events: &[Event]) -> FrameIn<'_> { FrameIn::new(320.0, 240.0).time(time).events(events) }
        for time in [0.0, 0.1] {
            crate::testing::frame(&mut ui, &input(time, &[]), &mut build).unwrap();
        }
        let key = pxui::ui::key_of(ROOT_KEY, "test-button");
        let button = ui.box_of_key(pxui::Key::Resolved(key)).unwrap();
        let surface = ui
            .box_of_key(pxui::Key::Resolved(pxui::ui::key_of(pxui::ui::key_of(key, "visual"), "surface")))
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
        let mut reduced = ThemeHandle::default();
        reduced.reduced_motion(true);
        apply_palette(&mut reduced, pxui::theme::Tokens::DEFAULT.is_dark());
        ui.set_theme(&reduced).unwrap();
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
