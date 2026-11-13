use crate::catalog::{Catalog, Route};
use crate::{Intent, PlayerView};
use pxui::ui::*;
use std::hash::{DefaultHasher, Hash, Hasher};

pub struct ShellState {
    pub route: Route,
    pub selected: Option<usize>,
    pub panel_visible: bool,
    revision: u64,
}
impl Default for ShellState {
    fn default() -> Self {
        Self {
            route: Route::Songs,
            selected: None,
            panel_visible: true,
            revision: 0,
        }
    }
}
impl ShellState {
    pub fn apply(&mut self, intent: Intent) -> bool {
        match intent {
            Intent::Navigate(route) => {
                self.route = route;
                self.selected = None;
            }
            Intent::SelectRow(index) => self.selected = Some(index),
            Intent::TogglePanel => self.panel_visible = !self.panel_visible,
            _ => return false,
        }
        self.revision = self.revision.wrapping_add(1);
        true
    }
}

#[derive(Clone, Copy)]
struct Palette {
    paper: Rgba,
    panel: Rgba,
    selected: Rgba,
    ink: Rgba,
    secondary: Rgba,
    line: Rgba,
    key: Rgba,
    signal: Rgba,
}
const fn rgb(hex: u32) -> Rgba {
    Rgba {
        r: (hex >> 16) as u8,
        g: (hex >> 8) as u8,
        b: hex as u8,
        a: 255,
    }
}
fn palette(ui: &Ui) -> Palette {
    let dark = ui.theme().tokens.foreground.r > 128;
    if dark {
        Palette {
            paper: rgb(0x101010),
            panel: rgb(0x1A1A1A),
            selected: rgb(0x2A2A2A),
            ink: rgb(0xEDEDED),
            secondary: rgb(0x9B9B9B),
            line: rgb(0x262626),
            key: rgb(0x1F1F1F),
            signal: ui.theme().tokens.accent,
        }
    } else {
        Palette {
            paper: rgb(0xF3F3F3),
            panel: rgb(0xE6E6E6),
            selected: rgb(0xD3D3D3),
            ink: rgb(0x111111),
            secondary: rgb(0x585858),
            line: rgb(0xDADADA),
            key: rgb(0xFBFBFB),
            signal: ui.theme().tokens.accent,
        }
    }
}
fn container(ui: &mut Ui, key: &str, axis: Axis, w: Size, h: Size, fill: Rgba) -> Handle {
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
fn text(
    ui: &mut Ui,
    key: &str,
    value: &str,
    size: i64,
    color: Rgba,
    dimensions: (Size, f32),
    mono: bool,
) -> Handle {
    let node = ui.node(
        key,
        Node {
            flags: FIT_TEXT,
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
    ui.text(&node, value);
    node
}
fn spacer(ui: &mut Ui, key: &str, height: f32) {
    ui.box_(key, BoxOpts::default().h(Size::Px(height)));
}
fn rule(ui: &mut Ui, key: &str, vertical: bool, p: Palette) {
    container(
        ui,
        key,
        Axis::Column,
        if vertical { Size::Px(1.0) } else { Size::Grow },
        if vertical { Size::Grow } else { Size::Px(1.0) },
        p.line,
    );
}

#[derive(Clone, Copy)]
enum Icon {
    Play,
    Pause,
    Previous,
    Next,
    Stop,
    Panel,
    Shuffle,
    Repeat,
}
fn icon_button(
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
    let signal = ui.signal(&node);
    ui.a11y_name(&node, label);
    let fill = if inverse {
        p.ink
    } else if signal.hovered || signal.held {
        p.panel
    } else {
        p.key
    };
    let color = if inverse { p.paper } else { p.ink };
    let mut revision = DefaultHasher::new();
    (icon as u8, inverse, signal.hovered, signal.held).hash(&mut revision);
    ui.within(&node, &mut |ui| {
        let visual = ui.view(
            "visual",
            revision.finish(),
            Size::Grow,
            Size::Grow,
            &mut |ui| {
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
                        Icon::Play => {
                            for row in 0..16 {
                                let w = (8.0 - (row as f32 - 7.5).abs()) * 1.5;
                                paint.fill(x + 5.0, y + 2.0 + row as f32, w, 1.0, color);
                            }
                        }
                        Icon::Pause => {
                            paint.fill(x + 4.0, y + 3.0, 4.0, 14.0, color);
                            paint.fill(x + 12.0, y + 3.0, 4.0, 14.0, color);
                        }
                        Icon::Stop => paint.fill(x + 5.0, y + 5.0, 10.0, 10.0, color),
                        Icon::Previous | Icon::Next => {
                            let next = matches!(icon, Icon::Next);
                            paint.fill(
                                x + if next { 15.0 } else { 3.0 },
                                y + 4.0,
                                2.0,
                                12.0,
                                color,
                            );
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
                            paint.line((x + 2.0, y + 4.0), (x + 17.0, y + 16.0), color);
                            paint.line((x + 2.0, y + 16.0), (x + 17.0, y + 4.0), color);
                        }
                        Icon::Repeat => {
                            paint.line((x + 3.0, y + 5.0), (x + 17.0, y + 5.0), color);
                            paint.line((x + 17.0, y + 5.0), (x + 17.0, y + 15.0), color);
                            paint.line((x + 17.0, y + 15.0), (x + 3.0, y + 15.0), color);
                            paint.line((x + 3.0, y + 15.0), (x + 3.0, y + 9.0), color);
                        }
                    }
                });
            },
        );
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

#[cfg(test)]
mod button_tests {
    use super::*;
    use pxui::{Event, FrameIn, Platform, Str, ThemeSource};
    use std::cell::Cell;

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
            ui.signal(&button).held
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

pub fn shell(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    state: &ShellState,
    intent: &mut Option<Intent>,
) {
    let p = palette(ui);
    let (width, _) = ui.view_size();
    let panel = state.panel_visible && width >= 900.0;
    let sidebar_width = if width >= 900.0 { 231.0 } else { 180.0 };
    let content_width = (width - sidebar_width - 1.0 - if panel { 320.0 } else { 0.0 }).max(100.0);
    let root = container(ui, "shell", Axis::Column, Size::Grow, Size::Grow, p.paper);
    ui.set_role(&root, Role::Window, 0);
    ui.a11y_name(&root, "LocalWave");
    ui.within(&root, &mut |ui| {
        let work = container(ui, "workspace", Axis::Row, Size::Grow, Size::Grow, p.paper);
        ui.within(&work, &mut |ui| {
            let revision = state.revision.wrapping_mul(2) | catalog.all.is_some() as u64;
            ui.view(
                "sidebar",
                revision,
                Size::Px(sidebar_width),
                Size::Grow,
                &mut |ui| sidebar(ui, catalog, state, sidebar_width, p, intent),
            );
            rule(ui, "sidebar-divider", true, p);
            let mut library_revision = DefaultHasher::new();
            (revision, player.current_id).hash(&mut library_revision);
            ui.view(
                "library",
                library_revision.finish(),
                Size::Grow,
                Size::Grow,
                &mut |ui| {
                    let mut route_key = DefaultHasher::new();
                    state.route.hash(&mut route_key);
                    let route = ui.node_keyed(
                        int_key(
                            Ui::key_of(0, "localwave-library-route"),
                            route_key.finish() as i64,
                        ),
                        Node {
                            w: Size::Grow,
                            h: Size::Grow,
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&route, &mut |ui| {
                        content(ui, catalog, state, player, content_width, p, intent)
                    });
                },
            );
            if panel {
                rule(ui, "panel-divider", true, p);
                ui.view(
                    "now-panel",
                    player.revision,
                    Size::Px(319.0),
                    Size::Grow,
                    &mut |ui| now_panel(ui, player, p, intent),
                );
            }
        });
        rule(ui, "transport-divider", false, p);
        ui.view(
            "transport",
            player.revision,
            Size::Grow,
            Size::Px(71.0),
            &mut |ui| transport(ui, player, p, intent),
        );
    });
    if intent.is_none()
        && ui.key_pressed(input::Key::Enter)
        && !ui.text_input_focused()
        && let Some(index) = state.selected
        && catalog
            .collection(state.route)
            .is_some_and(|collection| index < collection.rows.len())
    {
        *intent = Some(Intent::PlayRow(index));
    }
}

fn side_row(
    ui: &mut Ui,
    key: &str,
    label: (&str, &str),
    route: Route,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let (title, count) = label;
    let node = ui.node(
        key,
        Node {
            flags: CLICKABLE | TAB_STOP,
            axis: Axis::Row,
            w: Size::Grow,
            h: Size::Px(32.0),
            role: Role::Button,
            padding: Edges {
                l: 8.0,
                r: 8.0,
                ..Edges::ZERO
            },
            align: Align::Center,
            ..Node::DEFAULT
        },
    );
    let signal = ui.signal(&node);
    ui.set_look(
        &node,
        Look {
            fill: if state.route == route {
                p.selected
            } else if signal.hovered {
                p.panel
            } else {
                p.paper
            },
            ..Look::NONE
        },
        State::default(),
    );
    ui.a11y_name(&node, title);
    ui.within(&node, &mut |ui| {
        text(
            ui,
            "title",
            title,
            14,
            if state.route == route {
                p.ink
            } else {
                p.secondary
            },
            (Size::Grow, 20.0),
            false,
        );
        text(
            ui,
            "count",
            count,
            12,
            p.secondary,
            (Size::Px(44.0), 18.0),
            true,
        );
    });
    if signal.clicked {
        *intent = Some(Intent::Navigate(route));
    }
}
fn sidebar(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let root = ui.node(
        "sidebar-content",
        Node {
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges::all(10.0),
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        spacer(ui, "sidebar-top", 28.0);
        text(
            ui,
            "brand",
            "LOCALWAVE",
            12,
            p.secondary,
            (Size::Grow, 34.0),
            true,
        );
        spacer(ui, "sidebar-brand-gap", 12.0);
        text(
            ui,
            "library-heading",
            "Library",
            12,
            p.secondary,
            (Size::Grow, 30.0),
            false,
        );
        side_row(
            ui,
            "albums",
            ("Albums", &catalog.album_count),
            Route::Albums,
            state,
            p,
            intent,
        );
        side_row(
            ui,
            "artists",
            ("Artists", &catalog.artist_count),
            Route::Artists,
            state,
            p,
            intent,
        );
        side_row(
            ui,
            "songs",
            ("Songs", &catalog.song_count),
            Route::Songs,
            state,
            p,
            intent,
        );
        spacer(ui, "lists-gap", 14.0);
        text(
            ui,
            "lists-heading",
            "Lists",
            12,
            p.secondary,
            (Size::Grow, 30.0),
            false,
        );
        ui.grid_list(
            "collections",
            catalog.collections.len(),
            (width - 20.0).max(1.0),
            32.0,
            &mut |ui, index| {
                let collection = &catalog.collections[index];
                side_row(
                    ui,
                    "collection",
                    (&collection.title, &collection.count),
                    Route::Collection(index),
                    state,
                    p,
                    intent,
                );
            },
        );
        text(
            ui,
            "storage",
            "Local music, your files",
            12,
            p.secondary,
            (Size::Grow, 26.0),
            false,
        );
    });
}

fn content(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let root = ui.node(
        "content",
        Node {
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges {
                l: 28.0,
                r: 28.0,
                t: 28.0,
                b: 12.0,
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let collection = catalog.collection(state.route);
        let title = match state.route {
            Route::Albums => "Albums",
            Route::Artists => "Artists",
            _ => collection.map_or("Songs", |group| group.title.as_str()),
        };
        text(ui, "heading", title, 34, p.ink, (Size::Grow, 44.0), false);
        text(
            ui,
            "subtitle",
            collection.map_or("Browse your library", |group| group.subtitle.as_str()),
            14,
            p.secondary,
            (Size::Grow, 34.0),
            false,
        );
        rule(ui, "heading-divider", false, p);
        if matches!(state.route, Route::Albums | Route::Artists) {
            let albums = state.route == Route::Albums;
            let groups = if albums {
                &catalog.albums
            } else {
                &catalog.artists
            };
            let cell = if albums {
                180.0
            } else {
                (width - 56.0).max(1.0)
            };
            ui.grid_list(
                "groups",
                groups.len(),
                cell,
                if albums { 216.0 } else { 54.0 },
                &mut |ui, index| {
                    let group = &groups[index];
                    let node = ui.node(
                        "group",
                        Node {
                            flags: CLICKABLE | TAB_STOP,
                            w: Size::Grow,
                            h: Size::Grow,
                            role: Role::Button,
                            padding: Edges::all(10.0),
                            ..Node::DEFAULT
                        },
                    );
                    ui.a11y_name(&node, &group.title);
                    let signal = ui.signal(&node);
                    ui.set_look(
                        &node,
                        Look {
                            fill: if signal.hovered { p.panel } else { p.paper },
                            ..Look::NONE
                        },
                        State::default(),
                    );
                    ui.within(&node, &mut |ui| {
                        if albums {
                            let art = container(
                                ui,
                                "artwork",
                                Axis::Stack,
                                Size::Grow,
                                Size::Px(146.0),
                                p.panel,
                            );
                            ui.a11y_name(&art, "Album artwork");
                        }
                        text(
                            ui,
                            "name",
                            &group.title,
                            14,
                            p.ink,
                            (Size::Grow, 23.0),
                            false,
                        );
                        text(
                            ui,
                            "artist",
                            &group.subtitle,
                            12,
                            p.secondary,
                            (Size::Grow, 18.0),
                            false,
                        );
                    });
                    if signal.clicked {
                        *intent = Some(Intent::Navigate(if albums {
                            Route::Album(index)
                        } else {
                            Route::Artist(index)
                        }));
                    }
                },
            );
        } else if let Some(collection) = collection {
            let header = container(
                ui,
                "columns",
                Axis::Row,
                Size::Grow,
                Size::Px(32.0),
                p.paper,
            );
            ui.within(&header, &mut |ui| {
                for (key, value, size) in [
                    ("title", "Title", Size::Grow),
                    ("artist", "Artist", Size::Pct(0.24)),
                    ("album", "Album", Size::Pct(0.26)),
                    ("time", "Time", Size::Px(48.0)),
                ] {
                    text(ui, key, value, 12, p.secondary, (size, 28.0), false);
                }
            });
            rule(ui, "columns-divider", false, p);
            let row_width = (width - 56.0).max(1.0);
            ui.grid_list(
                "songs",
                collection.rows.len(),
                row_width,
                38.0,
                &mut |ui, index| {
                    let row = catalog.row(collection.rows[index]);
                    let node = ui.node(
                        "song",
                        Node {
                            flags: CLICKABLE | TAB_STOP,
                            axis: Axis::Row,
                            align: Align::Center,
                            w: Size::Grow,
                            h: Size::Px(38.0),
                            role: Role::ListItem,
                            ..Node::DEFAULT
                        },
                    );
                    let signal = ui.signal(&node);
                    ui.a11y_name(&node, row.title);
                    ui.set_look(
                        &node,
                        Look {
                            fill: if state.selected == Some(index) {
                                p.selected
                            } else if signal.hovered {
                                p.panel
                            } else {
                                p.paper
                            },
                            ..Look::NONE
                        },
                        State::default(),
                    );
                    ui.within(&node, &mut |ui| {
                        text(
                            ui,
                            "title",
                            row.title,
                            14,
                            if player.current_id == Some(row.id) {
                                p.signal
                            } else {
                                p.ink
                            },
                            (Size::Grow, 20.0),
                            false,
                        );
                        text(
                            ui,
                            "artist",
                            row.artist,
                            14,
                            p.secondary,
                            (Size::Pct(0.24), 20.0),
                            false,
                        );
                        text(
                            ui,
                            "album",
                            row.album,
                            14,
                            p.secondary,
                            (Size::Pct(0.26), 20.0),
                            false,
                        );
                        text(
                            ui,
                            "duration",
                            row.duration,
                            12,
                            p.secondary,
                            (Size::Px(48.0), 20.0),
                            true,
                        );
                    });
                    if signal.clicked {
                        *intent = Some(Intent::SelectRow(index));
                    }
                    if signal.double_clicked
                        || (signal.clicked && ui.key_pressed(input::Key::Enter))
                    {
                        *intent = Some(Intent::PlayRow(index));
                    }
                },
            );
        }
    });
}

fn now_panel(ui: &mut Ui, player: &PlayerView<'_>, p: Palette, intent: &mut Option<Intent>) {
    let root = ui.node(
        "now",
        Node {
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges::all(20.0),
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let head = container(ui, "header", Axis::Row, Size::Grow, Size::Px(36.0), p.paper);
        ui.within(&head, &mut |ui| {
            text(
                ui,
                "name",
                "Now playing",
                13,
                p.secondary,
                (Size::Grow, 28.0),
                false,
            );
            if icon_button(
                ui,
                "hide-panel",
                "Minimise to small player",
                Icon::Panel,
                (28.0, 28.0),
                false,
                p,
            ) {
                *intent = Some(Intent::TogglePanel);
            }
        });
        spacer(ui, "artwork-top-gap", 12.0);
        let art = container(
            ui,
            "artwork",
            Axis::Stack,
            Size::Px(184.0),
            Size::Px(184.0),
            p.panel,
        );
        ui.a11y_name(&art, "Current track artwork");
        spacer(ui, "artwork-bottom-gap", 14.0);
        text(
            ui,
            "current-track",
            if !player.initialized {
                "Loading library"
            } else if player.title.is_empty() {
                "Nothing playing"
            } else {
                player.title
            },
            19,
            p.ink,
            (Size::Grow, 28.0),
            false,
        );
        text(
            ui,
            "artist",
            player.artist,
            14,
            p.secondary,
            (Size::Grow, 22.0),
            false,
        );
        text(
            ui,
            "album",
            player.album,
            14,
            p.secondary,
            (Size::Grow, 22.0),
            false,
        );
        spacer(ui, "title-clock-gap", 14.0);
        let clock = container(
            ui,
            "clock",
            Axis::Column,
            Size::Grow,
            Size::Px(51.0),
            p.paper,
        );
        ui.a11y_name(&clock, "Elapsed time");
        ui.a11y_number(&clock, player.position, 0.0, player.duration.max(0.0), 1.0);
        let seconds = player.position.max(0.0) as u64;
        ui.draw(&clock, move |paint, rect| {
            matrix_clock(paint, rect, seconds, p.ink)
        });
        spacer(ui, "clock-bottom-gap", 14.0);
        rule(ui, "context-divider", false, p);
        spacer(ui, "context-top-gap", 13.0);
        text(
            ui,
            "playback-context",
            player.context,
            12,
            p.secondary,
            (Size::Grow, 24.0),
            false,
        );
        text(
            ui,
            "playback-notice",
            player.message,
            13,
            p.secondary,
            (Size::Grow, 44.0),
            false,
        );
    });
}

#[derive(Clone, Copy)]
struct ClockLabel {
    bytes: [u8; 24],
    start: usize,
}
impl ClockLabel {
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
    fn bytes(&self) -> &[u8] {
        &self.bytes[self.start..]
    }
}
fn small_clock(ui: &mut Ui, key: &str, seconds: u64, remaining: bool, width: f32, color: Rgba) {
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
    ui.a11y_number(&node, seconds as f64, 0.0, f64::MAX, 1.0);
    let label = ClockLabel::new(seconds, remaining);
    ui.draw(&node, move |paint, rect| {
        let mut x = rect.x;
        for byte in label.bytes() {
            x += paint.icon((x, rect.y + 4.0), 1, 12.0, *byte as char, color);
        }
    });
}
fn matrix_clock(paint: &mut Paint<'_>, rect: Rect, seconds: u64, color: Rgba) {
    const DIGITS: [[u8; 7]; 10] = [
        [14, 17, 19, 21, 25, 17, 14],
        [4, 12, 4, 4, 4, 4, 14],
        [14, 17, 1, 2, 4, 8, 31],
        [30, 1, 1, 14, 1, 1, 30],
        [2, 6, 10, 18, 31, 2, 2],
        [31, 16, 16, 30, 1, 1, 30],
        [14, 16, 16, 30, 17, 17, 14],
        [31, 1, 2, 4, 8, 8, 8],
        [14, 17, 17, 14, 17, 17, 14],
        [14, 17, 17, 15, 1, 1, 14],
    ];
    let label = ClockLabel::new(seconds, false);
    let unit = (43.0f32 / 7.0).min(rect.w / (label.bytes().len() as f32 * 6.0));
    for (digit, byte) in label.bytes().iter().copied().enumerate() {
        let rows = if byte == b':' {
            [0, 0, 4, 0, 4, 0, 0]
        } else {
            DIGITS[(byte - b'0') as usize]
        };
        for (y, row) in rows.iter().enumerate() {
            for x in 0..5 {
                if row & (1 << (4 - x)) != 0 {
                    paint.fill(
                        rect.x + (digit * 6 + x) as f32 * unit,
                        rect.y + y as f32 * unit,
                        (unit - 1.0).max(0.5),
                        (unit - 1.0).max(0.5),
                        color,
                    );
                }
            }
        }
    }
}
fn transport(ui: &mut Ui, player: &PlayerView<'_>, p: Palette, intent: &mut Option<Intent>) {
    let root = ui.node(
        "transport-content",
        Node {
            axis: Axis::Row,
            align: Align::Center,
            w: Size::Grow,
            h: Size::Grow,
            gap: 16.0,
            padding: Edges {
                r: 20.0,
                ..Edges::ZERO
            },
            look: Look {
                fill: p.panel,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let keys = ui.node(
            "keys",
            Node {
                axis: Axis::Row,
                gap: 2.0,
                w: Size::Px(210.0),
                h: Size::Px(71.0),
                ..Node::DEFAULT
            },
        );
        ui.within(&keys, &mut |ui| {
            if icon_button(
                ui,
                "previous",
                "Previous",
                Icon::Previous,
                (68.0, 71.0),
                false,
                p,
            ) {
                *intent = Some(Intent::Previous);
            }
            if icon_button(
                ui,
                "toggle",
                if player.playing { "Pause" } else { "Play" },
                if player.playing {
                    Icon::Pause
                } else {
                    Icon::Play
                },
                (68.0, 71.0),
                true,
                p,
            ) {
                *intent = Some(if player.playing {
                    Intent::Pause
                } else {
                    Intent::Play
                });
            }
            if icon_button(ui, "next", "Next", Icon::Next, (68.0, 71.0), false, p) {
                *intent = Some(Intent::Next);
            }
        });
        let elapsed = player.position.max(0.0) as u64;
        let remaining = (player.duration - player.position).max(0.0) as u64;
        small_clock(ui, "elapsed", elapsed, false, 44.0, p.secondary);
        let scrubber = ui.node(
            "scrubber",
            Node {
                flags: CLICKABLE | TAB_STOP,
                w: Size::Grow,
                h: Size::Px(40.0),
                role: Role::Slider,
                ..Node::DEFAULT
            },
        );
        ui.a11y_name(&scrubber, "Playback position");
        ui.a11y_number(
            &scrubber,
            player.position,
            0.0,
            player.duration.max(0.0),
            5.0,
        );
        let fraction = if player.duration > 0.0 {
            (player.position / player.duration).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        ui.draw(&scrubber, move |paint, rect| {
            paint.fill(rect.x, rect.y + rect.h / 2.0 - 1.0, rect.w, 2.0, p.line);
            paint.fill(
                rect.x,
                rect.y + rect.h / 2.0 - 1.0,
                rect.w * fraction,
                2.0,
                p.ink,
            );
        });
        let signal = ui.signal(&scrubber);
        let rect = ui.rect(&scrubber);
        if (signal.clicked || signal.dragging) && rect.w > 0.0 {
            *intent = Some(Intent::SeekFraction(
                ((signal.pointer.0 - rect.x) / rect.w).clamp(0.0, 1.0) as f64,
            ));
        }
        if let Some(value) = signal.a11y_value
            && player.duration > 0.0
        {
            *intent = Some(Intent::SeekFraction(
                (value / player.duration).clamp(0.0, 1.0),
            ));
        }
        small_clock(ui, "remaining", remaining, true, 50.0, p.secondary);
        if icon_button(
            ui,
            "shuffle",
            if player.shuffled {
                "Shuffle on"
            } else {
                "Shuffle off"
            },
            Icon::Shuffle,
            (36.0, 40.0),
            player.shuffled,
            p,
        ) {
            *intent = Some(Intent::Shuffle);
        }
        if icon_button(
            ui,
            "repeat",
            match player.repeat {
                lw_core::queue::RepeatMode::None => "Repeat off",
                lw_core::queue::RepeatMode::All => "Repeat all",
                lw_core::queue::RepeatMode::One => "Repeat one",
            },
            Icon::Repeat,
            (36.0, 40.0),
            player.repeat != lw_core::queue::RepeatMode::None,
            p,
        ) {
            *intent = Some(Intent::Repeat);
        }
        let volume = ui.node(
            "volume",
            Node {
                flags: CLICKABLE | TAB_STOP,
                w: Size::Px(80.0),
                h: Size::Px(40.0),
                role: Role::Slider,
                ..Node::DEFAULT
            },
        );
        ui.a11y_name(&volume, "Volume");
        ui.a11y_number(&volume, player.volume as f64, 0.0, 1.0, 0.05);
        let fraction = player.volume.clamp(0.0, 1.0);
        ui.draw(&volume, move |paint, rect| {
            paint.fill(rect.x, rect.y + 19.0, rect.w, 2.0, p.line);
            paint.fill(rect.x, rect.y + 19.0, rect.w * fraction, 2.0, p.ink);
            paint.fill(
                rect.x + (rect.w - 2.0) * fraction,
                rect.y + 14.0,
                2.0,
                12.0,
                p.ink,
            );
        });
        let signal = ui.signal(&volume);
        let rect = ui.rect(&volume);
        if (signal.clicked || signal.dragging) && rect.w > 0.0 {
            *intent = Some(Intent::Volume(
                ((signal.pointer.0 - rect.x) / rect.w).clamp(0.0, 1.0),
            ));
        }
        if let Some(value) = signal.a11y_value {
            *intent = Some(Intent::Volume(value.clamp(0.0, 1.0) as f32));
        }
        if icon_button(ui, "stop", "Stop", Icon::Stop, (36.0, 40.0), false, p) {
            *intent = Some(Intent::Stop);
        }
        if icon_button(
            ui,
            "panel",
            "Toggle player panel",
            Icon::Panel,
            (36.0, 40.0),
            false,
            p,
        ) {
            *intent = Some(Intent::TogglePanel);
        }
    });
}
