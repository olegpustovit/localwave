use crate::browse::{Query, Request, Scope, Sort};
use crate::catalog::{Catalog, Route};
use crate::{Intent, PlayerView};
use pxui::ui::*;
use std::hash::{DefaultHasher, Hash, Hasher};

mod albums;
mod artists;
mod byline;
mod files;
mod settings;

pub struct ShellState {
    pub route: Route,
    pub selected: Option<usize>,
    pub panel_visible: bool,
    pub query: Query,
    pub sort: Sort,
    pub ascending: bool,
    pub album_sort: crate::browse::AlbumSort,
    queue_visible: bool,
    playlist_editing: bool,
    playlist_name: Query,
    playlist_name_too_long: bool,
    top_inset: f32,
    query_too_long: bool,
    query_revision: u64,
    revision: u64,
    entry_revision: u64,
    settings_visible: bool,
    privacy_visible: bool,
    settings_menu: Option<(crate::SettingsMenu, (f32, f32))>,
    source_menu: Option<(crate::files::Target, (f32, f32))>,
    remove_source: Option<crate::files::Target>,
}
impl Default for ShellState {
    fn default() -> Self {
        Self {
            route: Route::Songs,
            selected: None,
            panel_visible: true,
            query: Query::default(),
            sort: Sort::Original,
            ascending: true,
            album_sort: crate::browse::AlbumSort::Artist,
            queue_visible: false,
            playlist_editing: false,
            playlist_name: Query::default(),
            playlist_name_too_long: false,
            top_inset: 0.0,
            query_too_long: false,
            query_revision: 0,
            revision: 0,
            entry_revision: 1,
            settings_visible: false,
            privacy_visible: false,
            settings_menu: None,
            source_menu: None,
            remove_source: None,
        }
    }
}
impl ShellState {
    pub fn files_visible(&self) -> bool {
        self.route == Route::Files && !self.settings_visible
    }
    pub fn set_window_inset(&mut self, top: f32) {
        if top.is_finite() && top >= 0.0 && self.top_inset != top {
            self.top_inset = top;
            self.revision = self.revision.wrapping_add(1);
        }
    }
    pub fn collection<'a>(&self, catalog: &'a Catalog) -> Option<&'a crate::catalog::Collection> {
        if self.route == Route::Find {
            catalog
                .view
                .as_ref()
                .filter(|view| {
                    view.request.scope == Scope::Find && view.request.query == self.query
                })
                .map(|view| &view.collection)
        } else {
            catalog.collection(self.route)
        }
    }
    pub fn request(&self, catalog: &Catalog) -> Option<Request> {
        Some(Request {
            scope: catalog.scope(self.route)?,
            query: if self.route == Route::Find {
                self.query
            } else {
                Query::default()
            },
            sort: self.sort,
            ascending: self.ascending,
        })
    }
    pub fn refresh(&mut self, previous: &Catalog, next: &Catalog) {
        let same_view = match (&previous.view, &next.view) {
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if std::sync::Arc::ptr_eq(&previous.library, &next.library) && same_view {
            return;
        }
        let old = self.collection(previous);
        let selected = old.and_then(|group| {
            self.selected
                .and_then(|index| group.ids.get(index).copied())
        });
        let occurrence = old.zip(self.selected).map_or(0, |(group, index)| {
            group.ids[..index.min(group.ids.len())]
                .iter()
                .filter(|id| Some(**id) == selected)
                .count()
        });
        let same = |group: &crate::catalog::Collection| {
            old.is_some_and(|old| match (&*old.context, &*group.context) {
                (
                    lw_core::context::PlaybackContext::Playlist { id: a, .. },
                    lw_core::context::PlaybackContext::Playlist { id: b, .. },
                ) => a == b,
                (a, b) => a == b,
            })
        };
        self.route = match self.route {
            Route::Album(_) => next
                .albums
                .iter()
                .position(same)
                .map_or(Route::Albums, Route::Album),
            Route::Artist(_) => next
                .artists
                .iter()
                .position(same)
                .map_or(Route::Artists, Route::Artist),
            Route::Collection(_) => next
                .collections
                .iter()
                .position(same)
                .map_or(Route::Songs, Route::Collection),
            route => route,
        };
        self.selected = self
            .collection(next)
            .filter(|group| same(group))
            .and_then(|group| {
                selected.and_then(|selected| {
                    group
                        .ids
                        .iter()
                        .enumerate()
                        .filter(|(_, id)| **id == selected)
                        .nth(occurrence)
                        .map(|(index, _)| index)
                })
            });
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn apply(&mut self, intent: Intent) -> bool {
        if matches!(
            intent,
            Intent::Navigate(_)
                | Intent::ShowSettings
                | Intent::CloseSettings
                | Intent::ShowPrivacy
                | Intent::ClosePrivacy
        ) {
            self.entry_revision = self.entry_revision.wrapping_add(1);
        }
        match intent {
            Intent::Navigate(route) => {
                self.source_menu = None;
                self.remove_source = None;
                self.settings_visible = false;
                self.privacy_visible = false;
                self.settings_menu = None;
                self.route = route;
                self.selected = None;
                self.sort = Sort::Original;
                self.ascending = true;
            }
            Intent::SortAlbums(sort) => self.album_sort = sort,
            Intent::SelectRow(index) => self.selected = Some(index),
            Intent::TogglePanel => self.panel_visible = !self.panel_visible,
            Intent::ShowQueue => {
                self.source_menu = None;
                self.remove_source = None;
                self.queue_visible = true;
                self.settings_menu = None;
            }
            Intent::CloseQueue => {
                self.queue_visible = false;
                self.playlist_editing = false;
            }
            Intent::NamePlaylist => self.playlist_editing = true,
            Intent::EditPlaylistName(name) => {
                self.playlist_name = name;
                self.playlist_name_too_long = false;
            }
            Intent::PlaylistNameTooLong => self.playlist_name_too_long = true,
            Intent::CancelPlaylistName => self.playlist_editing = false,
            Intent::Search(query) => {
                self.query = query;
                self.selected = None;
                self.query_too_long = false;
                self.query_revision = self.query_revision.wrapping_add(1);
            }
            Intent::SearchTooLong => {
                self.query_too_long = true;
                self.query_revision = self.query_revision.wrapping_add(1);
            }
            Intent::Sort(sort) => {
                self.ascending = self.sort != sort || !self.ascending;
                self.sort = sort;
            }
            Intent::ShowSettings => {
                self.source_menu = None;
                self.remove_source = None;
                self.settings_visible = true;
                self.privacy_visible = false;
                self.queue_visible = false;
                self.settings_menu = None;
            }
            Intent::CloseSettings => {
                self.settings_visible = false;
                self.privacy_visible = false;
                self.settings_menu = None;
            }
            Intent::ShowPrivacy => {
                self.privacy_visible = true;
                self.settings_menu = None;
            }
            Intent::ClosePrivacy => self.privacy_visible = false,
            Intent::SettingsMenu(menu, at) => {
                if at.0.is_finite() && at.1.is_finite() {
                    self.settings_menu = Some((menu, at));
                }
            }
            Intent::CloseSettingsMenu => self.settings_menu = None,
            Intent::SourceMenu(target, at) => {
                if at.0.is_finite() && at.1.is_finite() {
                    self.source_menu = Some((target, at));
                    self.settings_menu = None;
                }
            }
            Intent::CloseSourceMenu => self.source_menu = None,
            Intent::ConfirmRemoveSource(target) => {
                self.remove_source = Some(target);
                self.source_menu = None;
            }
            Intent::CloseRemoveSource => self.remove_source = None,
            Intent::Setting(_) => {
                self.settings_menu = None;
                self.revision = self.revision.wrapping_add(1);
                return false;
            }
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
    danger: Rgba,
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
            danger: rgb(0xD21A1A),
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
            danger: rgb(0xD21A1A),
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
    let style = pxui::TextStyle {
        font: u16::from(mono),
        size_points: size as f32,
        line_height: size as f32 * 1.4,
        max_lines: 1,
        ellipsis: pxui::TextStyle::ELLIPSIS_END,
        ..Default::default()
    };
    let width = if matches!(dimensions.0, Size::Fit) {
        let mut key = DefaultHasher::new();
        (value, mono, size).hash(&mut key);
        // SwiftUI rounds intrinsic Text widths to physical pixels; CoreText widths stay fractional.
        let density = ui.density() as f32;
        Size::Px(
            (ui.text_service()
                .width(key.finish(), value, &style, 0.0)
                .unwrap()
                * density)
                .ceil()
                / density,
        )
    } else {
        dimensions.0
    };
    let node = ui.node(
        key,
        Node {
            flags: FIT_TEXT,
            w: width,
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

fn paint_artwork(
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
enum Icon {
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
                        Icon::Queue => {
                            for dy in [5.0, 10.0, 15.0] {
                                paint.fill(x + 2.0, y + dy, 2.0, 1.5, color);
                                paint.fill(x + 6.0, y + dy, 12.0, 1.5, color);
                            }
                        }
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

pub fn shell(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    state: &ShellState,
    intent: &mut Option<Intent>,
) {
    if ui.key_pressed(input::Key::Escape.code()) {
        ui.cancel_carry();
    }
    let p = palette(ui);
    if state.queue_visible {
        queue_sheet(ui, player, catalog, state, p, intent);
    }
    if let Some((menu, at)) = state.settings_menu {
        settings::popup(ui, player, menu, at, p, intent);
    }
    if let Some((target, at)) = state.source_menu {
        files::popup(ui, player, target, at, p, intent);
    }
    if let Some(target) = state.remove_source {
        files::confirm_remove(ui, player, target, p, intent);
    }
    let (width, height) = ui.view_size();
    let panel = !state.settings_visible
        && state.route != Route::Files
        && state.panel_visible
        && width >= 900.0
        && height - state.top_inset >= 640.0;
    let sidebar_width = if width >= 900.0 { 231.0 } else { 180.0 };
    let content_width = (width - sidebar_width - 1.0 - if panel { 320.0 } else { 0.0 }).max(100.0);
    let root = container(ui, "shell", Axis::Column, Size::Grow, Size::Grow, p.paper);
    ui.set_role(&root, Role::Window, 0);
    ui.a11y_name(
        &root,
        if player.file_dragging {
            "LocalWave — Drop to play. Folders are indexed. Audio files join Added Files."
        } else {
            "LocalWave"
        },
    );
    ui.within(&root, &mut |ui| {
        spacer(ui, "titlebar-clearance", state.top_inset);
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
            (
                revision,
                player.current_id,
                player.artwork.map_or(0, crate::artwork::Images::revision),
                player.settings,
                player.cache_usage.as_ptr() as usize,
                player.cache_usage.len(),
                player.storage_busy,
                player.settings_message.as_ptr() as usize,
                player.settings_message.len(),
                player.message.as_ptr() as usize,
                player.message.len(),
                if state.files_visible() {
                    player.files.map_or(0, |files| files.revision)
                } else {
                    0
                },
            )
                .hash(&mut library_revision);
            let page = ui.view(
                "library",
                library_revision.finish(),
                Size::Grow,
                Size::Grow,
                &mut |ui| {
                    let mut route_key = DefaultHasher::new();
                    if let Some(group) = catalog
                        .collection(state.route)
                        .filter(|_| state.route != Route::Find)
                    {
                        (true, group.key).hash(&mut route_key);
                    } else {
                        (false, state.route).hash(&mut route_key);
                    }
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
                        if state.settings_visible {
                            settings::view(ui, player, state, content_width, p, intent);
                        } else if state.route == Route::Files {
                            files::view(ui, player, content_width, p, intent);
                        } else {
                            content(ui, catalog, state, player, content_width, p, intent);
                        }
                    });
                },
            );
            ui.view_clip(
                &page,
                Some(Rect {
                    x: sidebar_width + 1.0,
                    y: state.top_inset,
                    w: content_width,
                    h: (height - state.top_inset - 72.0).max(0.0),
                }),
            );
            crate::motion::enter(
                ui,
                &page,
                state.entry_revision,
                (if width < 900.0 { 32.0 } else { 0.0 }, 0.0),
                crate::motion::BASE,
                0.0,
            );
            if panel {
                rule(ui, "panel-divider", true, p);
                let mut panel_revision = DefaultHasher::new();
                (
                    player.revision,
                    player.artwork.map_or(0, crate::artwork::Images::revision),
                    catalog.queue.as_ref().map(|queue| queue.revision),
                    std::sync::Arc::as_ptr(&catalog.library) as usize,
                )
                    .hash(&mut panel_revision);
                let panel = ui.view(
                    "now-panel",
                    panel_revision.finish(),
                    Size::Px(319.0),
                    Size::Grow,
                    &mut |ui| now_panel(ui, player, catalog, p, intent),
                );
                ui.view_clip(
                    &panel,
                    Some(Rect {
                        x: width - 319.0,
                        y: state.top_inset,
                        w: 319.0,
                        h: (height - state.top_inset - 72.0).max(0.0),
                    }),
                );
                crate::motion::enter(ui, &panel, 1, (32.0, 0.0), crate::motion::SLOW, 0.0);
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
    if player.file_dragging {
        ui.draw_over(&root, move |paint, rect| {
            let (x, y, w, h) = (
                rect.x + 12.0,
                rect.y + 12.0,
                (rect.w - 24.0).max(0.0),
                (rect.h - 24.0).max(0.0),
            );
            paint.fill(x, y, w, h, Rgba { a: 240, ..p.paper });
            // Swift's drop border uses six-point dashes and gaps.
            for vertical in [false, true] {
                let length = if vertical { h } else { w };
                for offset in (0..length.ceil() as usize).step_by(12) {
                    let dash = 6.0f32.min(length - offset as f32);
                    if vertical {
                        paint.fill(x, y + offset as f32, 2.0, dash, p.ink);
                        paint.fill(x + w - 2.0, y + offset as f32, 2.0, dash, p.ink);
                    } else {
                        paint.fill(x + offset as f32, y, dash, 2.0, p.ink);
                        paint.fill(x + offset as f32, y + h - 2.0, dash, 2.0, p.ink);
                    }
                }
            }
            for (text, size, at, color) in [
                ("Drop to play", 40, rect.y + rect.h * 0.5 - 34.0, p.ink),
                (
                    "Folders are indexed. Audio files join Added Files.",
                    14,
                    rect.y + rect.h * 0.5 + 22.0,
                    p.secondary,
                ),
            ] {
                let width = paint.text_width(Some(size), text);
                paint.text_s((rect.x + (rect.w - width) * 0.5, at), size, color, text);
            }
        });
    }
    if intent.is_none()
        && !state.queue_visible
        && !state.settings_visible
        && ui.key_pressed(input::Key::Enter)
        && !ui.text_input_focused()
        && let Some(index) = state.selected
        && state
            .collection(catalog)
            .is_some_and(|collection| index < collection.rows.len())
    {
        *intent = Some(Intent::PlayRow(index));
    }
    let settings_shortcut = ui.key_pressed(',') && ui.key_down(input::Key::Meta.code());
    if settings_shortcut {
        *intent = Some(Intent::ShowSettings);
    }
    if state.settings_visible && !state.queue_visible && ui.key_pressed(input::Key::Escape.code()) {
        *intent = Some(if state.settings_menu.is_some() {
            Intent::CloseSettingsMenu
        } else if state.privacy_visible {
            Intent::ClosePrivacy
        } else {
            Intent::CloseSettings
        });
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
    let signal = ui.signal_state(&node);
    ui.set_look(
        &node,
        Look {
            fill: if !state.settings_visible && state.route == route {
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
            if !state.settings_visible && state.route == route {
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
        side_row(ui, "find", ("Find", ""), Route::Find, state, p, intent);
        side_row(ui, "files", ("Files", ""), Route::Files, state, p, intent);
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
        let button = ui.node(
            "settings",
            Node {
                flags: CLICKABLE | FOCUSABLE,
                role: Role::Button,
                w: Size::Grow,
                h: Size::Px(36.0),
                look: Look {
                    fill: if state.settings_visible {
                        p.selected
                    } else {
                        Rgba::default()
                    },
                    text: p.ink,
                    font: Font {
                        size: 15,
                        ..Font::BODY
                    },
                    ..Look::NONE
                },
                ..Node::DEFAULT
            },
        );
        ui.text(&button, "Settings");
        ui.a11y_name(&button, "Settings");
        if ui.signal_state(&button).clicked {
            *intent = Some(Intent::ShowSettings);
        }
    });
}

fn header(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let collection = state.collection(catalog);
    let root = ui.node(
        "library-header",
        Node {
            w: Size::Grow,
            h: Size::Fit,
            padding: Edges {
                l: 32.0,
                r: 32.0,
                t: 28.0,
                b: 0.0,
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let title = match state.route {
            Route::Albums => "Albums",
            Route::Artists => "Artists",
            Route::Find => "Find",
            _ => collection.map_or("Songs", |group| group.title.as_str()),
        };
        let heading = container(
            ui,
            "heading-row",
            Axis::Row,
            Size::Grow,
            Size::Px(34.0),
            p.paper,
        );
        ui.within(&heading, &mut |ui| {
            let heading = text(ui, "heading", title, 34, p.ink, (Size::Grow, 34.0), false);
            ui.text_line_box(
                &heading,
                title,
                &pxui::TextStyle {
                    size_points: 34.0,
                    line_height: 34.0,
                    weight: 500,
                    tracking: -0.85,
                    max_lines: 1,
                    ..Default::default()
                },
                &[],
            )
            .unwrap();
            if !matches!(state.route, Route::Albums | Route::Artists)
                && icon_button(
                    ui,
                    "refresh",
                    "Refresh library",
                    Icon::Refresh,
                    (36.0, 36.0),
                    false,
                    p,
                )
            {
                *intent = Some(Intent::RefreshLibrary);
            }
        });
        if state.route == Route::Find {
            ui.view(
                "find-query",
                state.query_revision,
                Size::Grow,
                Size::Px(44.0),
                &mut |ui| {
                    let row =
                        container(ui, "query-row", Axis::Row, Size::Grow, Size::Grow, p.paper);
                    ui.within(&row, &mut |ui| {
                        let field = ui.box_(
                            "query-field",
                            BoxOpts::default().w(Size::Grow).h(Size::Grow),
                        );
                        ui.within(&field, &mut |ui| {
                            if let Some(value) = ui.text_field("Find songs", state.query.as_str()) {
                                *intent = Some(
                                    Query::new(value).map_or(Intent::SearchTooLong, Intent::Search),
                                );
                            }
                        });
                        if icon_button(
                            ui,
                            "clear-query",
                            "Clear search",
                            Icon::Clear,
                            (36.0, 36.0),
                            false,
                            p,
                        ) {
                            *intent = Some(Intent::Search(Query::default()));
                        }
                    });
                },
            );
            find_tokens(ui, state, (width - 64.0).max(1.0), p, intent);
        }
        spacer(ui, "heading-subtitle-gap", 6.0);
        text(
            ui,
            "subtitle",
            if state.query_too_long && state.route == Route::Find {
                "Search text is too long. Shorten it."
            } else if state.route == Route::Find && state.query.as_str().trim().is_empty() {
                "Search titles, artists, albums or add a filter"
            } else if state.route == Route::Find && collection.is_none() {
                "Searching…"
            } else if state.route == Route::Songs {
                "Click a header to sort. Double-click or Return plays."
            } else if state.route == Route::Artists {
                "Sorted ignoring “The”"
            } else if state.route == Route::Albums {
                &catalog.library_subtitle
            } else {
                collection.map_or("Browse your library", |group| group.subtitle.as_str())
            },
            14,
            p.secondary,
            (Size::Grow, 19.6),
            false,
        );
        spacer(ui, "heading-bottom-gap", 18.0);
        if state.route == Route::Albums {
            albums::toolbar(ui, catalog, state, p, intent);
        }
        if state.route == Route::Albums {
            ui.node(
                "heading-divider",
                Node {
                    w: Size::Px(width),
                    h: Size::Px(1.0),
                    margin: Edges {
                        l: -32.0,
                        r: -32.0,
                        ..Edges::ZERO
                    },
                    look: Look {
                        fill: p.line,
                        ..Look::NONE
                    },
                    ..Node::DEFAULT
                },
            );
            spacer(ui, "album-header-bottom", 1.0);
        } else if state.route != Route::Artists {
            rule(ui, "heading-divider", false, p);
        }
    });
}

fn header_view(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let mut revision = DefaultHasher::new();
    (
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        catalog
            .view
            .as_ref()
            .map_or(0, |view| std::sync::Arc::as_ptr(view) as usize),
    )
        .hash(&mut revision);
    ui.view(
        "page-heading",
        revision.finish(),
        Size::Grow,
        Size::Fit,
        &mut |ui| header(ui, catalog, state, width, p, intent),
    );
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
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        if state.route == Route::Albums {
            albums::gallery(ui, catalog, state, player, width, p, intent);
            return;
        }
        if state.route == Route::Artists {
            artists::table(ui, catalog, state, player, width, p, intent);
            return;
        }
        header(ui, catalog, state, width, p, intent);
        let body = ui.node(
            "content-body",
            Node {
                w: Size::Grow,
                h: Size::Grow,
                padding: Edges {
                    l: 32.0,
                    r: 32.0,
                    ..Edges::ZERO
                },
                ..Node::DEFAULT
            },
        );
        ui.within(&body, &mut |ui| {
            let mut row_revision = DefaultHasher::new();
            (
                state.revision,
                player.current_id,
                player.artwork.map_or(0, crate::artwork::Images::revision),
                std::sync::Arc::as_ptr(&catalog.library) as usize,
            )
                .hash(&mut row_revision);
            let row_revision = row_revision.finish();
            let collection = state.collection(catalog);
            if let Some(collection) = collection {
                let entry = crate::motion::Page::new(ui, state.entry_revision);
                let header = container(
                    ui,
                    "columns",
                    Axis::Row,
                    Size::Grow,
                    Size::Px(32.0),
                    p.paper,
                );
                ui.within(&header, &mut |ui| {
                    let applied = catalog
                        .view
                        .as_ref()
                        .filter(|view| Some(view.request.scope) == catalog.scope(state.route))
                        .map(|view| (view.request.sort, view.request.ascending));
                    for (key, column, value, up, down, name, size) in [
                        (
                            "number",
                            Sort::Original,
                            "#",
                            "#",
                            "#",
                            "Sort by original order",
                            Size::Px(44.0),
                        ),
                        (
                            "title",
                            Sort::Title,
                            "Title",
                            "Title ↑",
                            "Title ↓",
                            "Sort by Title",
                            Size::Grow,
                        ),
                        (
                            "artist",
                            Sort::Artist,
                            "Artist",
                            "Artist ↑",
                            "Artist ↓",
                            "Sort by Artist",
                            Size::Pct(0.24),
                        ),
                        (
                            "album",
                            Sort::Album,
                            "Album",
                            "Album ↑",
                            "Album ↓",
                            "Sort by Album",
                            Size::Pct(0.26),
                        ),
                        (
                            "time",
                            Sort::Duration,
                            "Time",
                            "Time ↑",
                            "Time ↓",
                            "Sort by Time",
                            Size::Px(48.0),
                        ),
                    ] {
                        let shown = match applied.filter(|(sort, _)| *sort == column) {
                            Some((_, true)) => up,
                            Some(_) => down,
                            None => value,
                        };
                        let heading = ui.node(
                            key,
                            Node {
                                flags: CLICKABLE | TAB_STOP | FIT_TEXT,
                                w: size,
                                h: Size::Px(28.0),
                                role: Role::Button,
                                look: Look {
                                    text: p.secondary,
                                    font: Font {
                                        size: 12,
                                        ..Font::BODY
                                    },
                                    ..Look::NONE
                                },
                                ..Node::DEFAULT
                            },
                        );
                        ui.text(&heading, shown);
                        let action = match applied.filter(|(sort, _)| *sort == column) {
                            None => name,
                            Some((sort, ascending)) => match (sort, ascending) {
                                (Sort::Title, true) => "Sort by Title, ascending",
                                (Sort::Title, false) => "Sort by Title, descending",
                                (Sort::Artist, true) => "Sort by Artist, ascending",
                                (Sort::Artist, false) => "Sort by Artist, descending",
                                (Sort::Album, true) => "Sort by Album, ascending",
                                (Sort::Album, false) => "Sort by Album, descending",
                                (Sort::Duration, true) => "Sort by Time, ascending",
                                (Sort::Duration, false) => "Sort by Time, descending",
                                _ => name,
                            },
                        };
                        ui.a11y_name(&heading, action);
                        if ui.signal_state(&heading).clicked {
                            *intent = Some(Intent::Sort(column));
                        }
                    }
                });
                rule(ui, "columns-divider", false, p);
                let results = ui.node_keyed(
                    int_key(
                        root.box_key,
                        if state.route == Route::Find {
                            state.query_revision as i64
                        } else {
                            0
                        },
                    ),
                    Node {
                        w: Size::Grow,
                        h: Size::Grow,
                        ..Node::DEFAULT
                    },
                );
                ui.within(&results, &mut |ui| {
                    let row_width = (width - 64.0).max(1.0);
                    ui.grid_list(
                        "songs",
                        collection.rows.len(),
                        row_width,
                        38.0,
                        &mut |ui, index| {
                            let row = catalog.row(collection.rows[index]);
                            let visual = ui.view(
                                "song-visual",
                                row_revision,
                                Size::Grow,
                                Size::Grow,
                                &mut |ui| {
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
                                    let signal = ui.signal_state(&node);
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
                                        let number = ClockLabel::integer((index + 1) as u64);
                                        text(
                                            ui,
                                            "number",
                                            std::str::from_utf8(number.bytes()).unwrap(),
                                            12,
                                            p.secondary,
                                            (Size::Px(44.0), 20.0),
                                            true,
                                        );
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
                            entry.row(ui, &visual, index);
                        },
                    );
                });
            }
        });
    });
}

fn find_tokens(
    ui: &mut Ui,
    state: &ShellState,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    const TOKENS: [&str; 5] = ["flac", "cloud", "offline", "year:2019", "unplayed"];
    let widths = TOKENS.map(|token| (ui.text_width(Some(13), token) + 24.0).min(width));
    let positions = {
        let (mut x, mut y) = (0.0, 0.0);
        widths.map(|w| {
            if x > 0.0 && x + w > width {
                x = 0.0;
                y += 38.0;
            }
            let at = (x, y);
            x += w + 6.0;
            at
        })
    };
    let mut revision = DefaultHasher::new();
    (state.query_revision, width.to_bits()).hash(&mut revision);
    ui.view(
        "find-tokens",
        revision.finish(),
        Size::Grow,
        Size::Px(positions[4].1 + 44.0),
        &mut |ui| {
            for (index, token) in TOKENS.into_iter().enumerate() {
                let selected = state.query.has_token(token);
                let chip = ui.node(
                    token,
                    Node {
                        flags: CLICKABLE | TAB_STOP | CLIP,
                        role: Role::Toggle,
                        w: Size::Px(widths[index]),
                        h: Size::Px(32.0),
                        at: Some(positions[index]).into(),
                        look: Look {
                            fill: if selected { p.ink } else { p.panel },
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.set_role(
                    &chip,
                    Role::Toggle,
                    pxui::a11y::state::CHECKABLE
                        | if selected {
                            pxui::a11y::state::CHECKED
                        } else {
                            0
                        },
                );
                ui.a11y_name(&chip, token);
                ui.within(&chip, &mut |ui| {
                    let label = ui.node(
                        "label",
                        Node {
                            flags: FIT_TEXT,
                            w: Size::Grow,
                            h: Size::Grow,
                            inset: Edges {
                                l: 12.0,
                                ..Edges::ZERO
                            },
                            look: Look {
                                text: if selected { p.paper } else { p.secondary },
                                font: Font {
                                    size: 13,
                                    ..Font::BODY
                                },
                                ..Look::NONE
                            },
                            ..Node::DEFAULT
                        },
                    );
                    ui.text(&label, token);
                });
                if ui.signal_state(&chip).clicked {
                    *intent = Some(
                        state
                            .query
                            .toggle_token(token)
                            .map_or(Intent::SearchTooLong, Intent::Search),
                    );
                }
            }
        },
    );
}

fn now_panel(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let root = ui.node(
        "now",
        Node {
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges {
                l: 20.0,
                r: 20.0,
                t: 8.0,
                b: 0.0,
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let head = ui.node(
            "header",
            Node {
                axis: Axis::Row,
                align: Align::Center,
                w: Size::Px(289.0),
                h: Size::Px(36.0),
                ..Node::DEFAULT
            },
        );
        ui.within(&head, &mut |ui| {
            text(
                ui,
                "name",
                "Now playing",
                13,
                p.secondary,
                (Size::Grow, 18.2),
                false,
            );
            if icon_button(
                ui,
                "hide-panel",
                "Minimise to small player",
                Icon::Panel,
                (36.0, 36.0),
                false,
                p,
            ) {
                *intent = Some(Intent::TogglePanel);
            }
        });
        spacer(ui, "artwork-top-gap", 4.0);
        let art = container(
            ui,
            "artwork",
            Axis::Stack,
            Size::Px(200.0),
            Size::Px(200.0),
            p.panel,
        );
        paint_artwork(
            ui,
            &art,
            player
                .current_id
                .and_then(|id| catalog.row_index(id))
                .map(|index| catalog.row(index)),
            200.0,
            player.artwork,
        );
        ui.a11y_name(&art, "Current track artwork");
        spacer(ui, "artwork-bottom-gap", 14.0);
        let mut information_revision = DefaultHasher::new();
        (
            player.initialized,
            player.current_id,
            player.title,
            player.artist,
            player.album,
        )
            .hash(&mut information_revision);
        let information = ui.view(
            "track-information",
            information_revision.finish(),
            Size::Grow,
            Size::Fit,
            &mut |ui| {
                let title = ui.node(
                    "current-track",
                    Node {
                        w: Size::Grow,
                        h: Size::Fit,
                        min_h: 22.8,
                        role: Role::Label,
                        ..Node::DEFAULT
                    },
                );
                let value = if !player.initialized {
                    "Loading library"
                } else if player.title.is_empty() {
                    "Nothing playing"
                } else {
                    player.title
                };
                ui.text_line_box(
                    &title,
                    value,
                    &pxui::TextStyle {
                        size_points: 19.0,
                        weight: 500,
                        tracking: -0.285,
                        line_height: 22.8,
                        wrap: 1,
                        ..Default::default()
                    },
                    &[],
                )
                .unwrap();
                if player.current_id.is_some() {
                    byline::draw(ui, player, catalog, 279.0, p, intent);
                }
            },
        );
        crate::motion::enter(
            ui,
            &information,
            1,
            (0.0, 10.0),
            crate::motion::BASE,
            crate::motion::QUICK + 0.045,
        );
        spacer(ui, "title-clock-gap", 14.0);
        let clock_group = ui.view(
            "playback-clock",
            player.revision,
            Size::Grow,
            Size::Px(43.0),
            &mut |ui| {
                let clock = ui.node(
                    "clock",
                    Node {
                        axis: Axis::Row,
                        align: Align::End,
                        w: Size::Grow,
                        h: Size::Px(43.0),
                        ..Node::DEFAULT
                    },
                );
                ui.within(&clock, &mut |ui| {
                    matrix_clock(ui, player.position.max(0.0) as u64, p.ink);
                    ui.box_("clock-flex", BoxOpts::default().w(Size::Grow));
                    small_clock(
                        ui,
                        "panel-remaining",
                        (player.duration - player.position).max(0.0) as u64,
                        true,
                        50.0,
                        p.secondary,
                    );
                });
            },
        );
        crate::motion::enter(
            ui,
            &clock_group,
            1,
            (0.0, 10.0),
            crate::motion::BASE,
            crate::motion::QUICK + 2.0 * 0.045,
        );
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
        // Keep the queue viewport stable when playback reports a notice.
        text(
            ui,
            "playback-notice",
            player.message,
            13,
            p.secondary,
            (Size::Grow, 44.0),
            false,
        );
        spacer(ui, "queue-gap", 14.0);
        rule(ui, "queue-divider", false, p);
        let mut queue_revision = DefaultHasher::new();
        (
            catalog.queue.as_ref().map(|queue| queue.revision),
            std::sync::Arc::as_ptr(&catalog.library) as usize,
            player.artwork.map_or(0, crate::artwork::Images::revision),
        )
            .hash(&mut queue_revision);
        ui.view(
            "up-next",
            queue_revision.finish(),
            Size::Grow,
            Size::Grow,
            &mut |ui| queue_rows(ui, catalog, false, 279.0, player.artwork, p, intent),
        );
    });
}

fn queue_action(ui: &mut Ui, key: &str, label: &str, name: &str, width: f32, p: Palette) -> bool {
    let button = ui.node(
        key,
        Node {
            flags: CLICKABLE | TAB_STOP | FIT_TEXT,
            role: Role::Button,
            w: Size::Px(width),
            h: Size::Px(28.0),
            look: Look {
                text: p.secondary,
                font: Font {
                    size: 12,
                    ..Font::BODY
                },
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.text(&button, label);
    ui.a11y_name(&button, name);
    ui.signal_state(&button).clicked
}

fn queue_sheet(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let (window_width, window_height) = ui.view_size();
    let width = (window_width - 32.0).clamp(240.0, 680.0);
    let height = (window_height - 64.0).clamp(160.0, 620.0);
    let open = ui.modal(Some(width), "Playback queue", &mut |ui| {
        let mut revision = DefaultHasher::new();
        (
            state.revision,
            player.artwork.map_or(0, crate::artwork::Images::revision),
            catalog.queue.as_ref().map(|queue| queue.revision),
            std::sync::Arc::as_ptr(&catalog.library) as usize,
            player.message.as_ptr() as usize,
            player.message.len(),
            width.to_bits(),
            height.to_bits(),
        )
            .hash(&mut revision);
        ui.view(
            "queue-sheet-content",
            revision.finish(),
            Size::Grow,
            Size::Px(height),
            &mut |ui| {
                let body = ui.node(
                    "queue-sheet-body",
                    Node {
                        w: Size::Grow,
                        h: Size::Px(height),
                        padding: Edges::all(20.0),
                        look: Look {
                            fill: p.paper,
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&body, &mut |ui| {
                    let header = container(
                        ui,
                        "full-queue-header",
                        Axis::Row,
                        Size::Grow,
                        Size::Px(40.0),
                        p.paper,
                    );
                    ui.within(&header, &mut |ui| {
                        text(
                            ui,
                            "full-queue-title",
                            "Queue",
                            24,
                            p.ink,
                            (Size::Grow, 36.0),
                            false,
                        );
                        if catalog
                            .queue
                            .as_ref()
                            .is_some_and(|queue| !queue.entries.is_empty())
                            && queue_action(
                                ui,
                                "save-playlist",
                                "Save as playlist",
                                "Save queue as playlist",
                                128.0,
                                p,
                            )
                        {
                            *intent = Some(Intent::NamePlaylist);
                        }
                        if icon_button(
                            ui,
                            "close-queue",
                            "Close queue",
                            Icon::Clear,
                            (28.0, 28.0),
                            false,
                            p,
                        ) {
                            *intent = Some(Intent::CloseQueue);
                        }
                    });
                    if state.playlist_editing {
                        text(
                            ui,
                            "playlist-prompt",
                            "Save all tracks in queue order; duplicates are included once.",
                            12,
                            p.secondary,
                            (Size::Grow, 32.0),
                            false,
                        );
                        let row = container(
                            ui,
                            "playlist-editor",
                            Axis::Row,
                            Size::Grow,
                            Size::Px(44.0),
                            p.paper,
                        );
                        ui.within(&row, &mut |ui| {
                            let field = ui.box_(
                                "playlist-name-field",
                                BoxOpts::default().w(Size::Grow).h(Size::Grow),
                            );
                            ui.within(&field, &mut |ui| {
                                if let Some(value) =
                                    ui.text_field("Playlist name", state.playlist_name.as_str())
                                {
                                    *intent = Some(Query::new(value).map_or(
                                        Intent::PlaylistNameTooLong,
                                        Intent::EditPlaylistName,
                                    ));
                                }
                            });
                            let valid = !state.playlist_name_too_long
                                && !state.playlist_name.as_str().trim().is_empty()
                                && catalog
                                    .queue
                                    .as_ref()
                                    .is_some_and(|queue| !queue.entries.is_empty());
                            if valid {
                                if queue_action(
                                    ui,
                                    "create-playlist",
                                    "Create",
                                    "Create playlist from queue",
                                    60.0,
                                    p,
                                ) {
                                    *intent = Some(Intent::SaveQueue(state.playlist_name));
                                }
                            } else {
                                text(
                                    ui,
                                    "create-unavailable",
                                    "Create",
                                    12,
                                    p.secondary,
                                    (Size::Px(60.0), 28.0),
                                    false,
                                );
                            }
                            if queue_action(
                                ui,
                                "cancel-playlist",
                                "Cancel",
                                "Cancel playlist creation",
                                60.0,
                                p,
                            ) {
                                *intent = Some(Intent::CancelPlaylistName);
                            }
                        });
                        text(
                            ui,
                            "playlist-name-error",
                            if state.playlist_name_too_long {
                                "Name exceeds 1,024 UTF-8 bytes. Shorten it to save."
                            } else {
                                ""
                            },
                            12,
                            p.secondary,
                            (Size::Grow, 24.0),
                            false,
                        );
                    }
                    text(
                        ui,
                        "queue-notice",
                        player.message,
                        12,
                        p.secondary,
                        (Size::Grow, 28.0),
                        false,
                    );
                    rule(ui, "full-queue-divider", false, p);
                    let mut revision = DefaultHasher::new();
                    (
                        catalog.queue.as_ref().map(|queue| queue.revision),
                        std::sync::Arc::as_ptr(&catalog.library) as usize,
                        width.to_bits(),
                        player.artwork.map_or(0, crate::artwork::Images::revision),
                    )
                        .hash(&mut revision);
                    ui.view(
                        "full-queue-rows",
                        revision.finish(),
                        Size::Grow,
                        Size::Grow,
                        &mut |ui| {
                            queue_rows(ui, catalog, true, width - 42.0, player.artwork, p, intent)
                        },
                    );
                });
            },
        );
    });
    if !open {
        *intent = Some(Intent::CloseQueue);
    }
}

fn queue_rows(
    ui: &mut Ui,
    catalog: &Catalog,
    full: bool,
    width: f32,
    images: Option<&crate::artwork::Images>,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let Some(queue) = &catalog.queue else { return };
    let entries = if full {
        &queue.entries[..]
    } else {
        queue.upcoming()
    };
    let header = container(
        ui,
        "queue-heading",
        Axis::Row,
        Size::Grow,
        Size::Px(34.0),
        p.paper,
    );
    ui.within(&header, &mut |ui| {
        text(
            ui,
            "queue-context",
            if full {
                queue.context.title()
            } else {
                &queue.heading
            },
            13,
            p.secondary,
            (Size::Grow, 28.0),
            false,
        );
        text(
            ui,
            "queue-count",
            if full { &queue.total } else { &queue.count },
            12,
            p.secondary,
            (Size::Px(56.0), 28.0),
            true,
        );
        if !full
            && icon_button(
                ui,
                "show-queue",
                "Show full queue",
                Icon::Queue,
                (28.0, 28.0),
                false,
                p,
            )
        {
            *intent = Some(Intent::ShowQueue);
        }
        if !queue.upcoming().is_empty()
            && queue_action(ui, "clear", "Clear", "Clear up next", 44.0, p)
        {
            *intent = Some(Intent::ClearQueue);
        }
        if queue.can_undo && queue_action(ui, "undo", "Undo", "Undo clear up next", 44.0, p) {
            *intent = Some(Intent::UndoQueue);
        }
    });
    if entries.is_empty() {
        text(
            ui,
            "queue-empty",
            if queue.can_undo {
                "Up next cleared."
            } else {
                "Queue is empty."
            },
            14,
            p.secondary,
            (Size::Grow, 44.0),
            false,
        );
        return;
    }
    let list = ui.grid_list(
        "queue-entries",
        entries.len(),
        width,
        44.0,
        &mut |ui, index| {
            let entry = entries[index];
            let current = queue
                .current
                .and_then(|index| queue.entries.get(index))
                .map(|entry| entry.id)
                == Some(entry.id);
            let row = catalog
                .row_index(entry.song_id)
                .map(|index| catalog.row(index));
            let title = row.map_or("Unavailable track", |row| {
                if row.title.is_empty() {
                    "Untitled track"
                } else {
                    row.title
                }
            });
            let artist = row.map_or("", |row| row.artist);
            let item = ui.node_keyed(
                int_key(
                    Ui::key_of(
                        0,
                        if full {
                            "localwave-full-queue-entry"
                        } else {
                            "localwave-queue-entry"
                        },
                    ),
                    entry.id as i64,
                ),
                Node {
                    w: Size::Grow,
                    h: Size::Px(44.0),
                    axis: Axis::Row,
                    flags: CLICKABLE,
                    role: Role::ListItem,
                    look: Look {
                        fill: if current { p.selected } else { p.paper },
                        ..Look::NONE
                    },
                    ..Node::DEFAULT
                },
            );
            ui.a11y_name(&item, title);
            if current {
                ui.set_role(&item, Role::ListItem, pxui::a11y::state::SELECTED);
            }
            ui.within(&item, &mut |ui| {
                let grip = ui.node(
                    "reorder-queued",
                    Node {
                        flags: CLICKABLE | TAB_STOP,
                        role: Role::Button,
                        w: Size::Px(24.0),
                        h: Size::Px(44.0),
                        ..Node::DEFAULT
                    },
                );
                ui.a11y_name(&grip, "Reorder queued track");
                ui.draw(&grip, move |paint, rect| {
                    for x in [8.0, 12.0] {
                        for y in [17.0, 21.0, 25.0] {
                            paint.fill(rect.x + x, rect.y + y, 2.0, 2.0, p.secondary);
                        }
                    }
                });
                let signal = ui.signal_state(&grip);
                if signal.held || signal.clicked {
                    let id = ClockLabel::integer(entry.id);
                    const PREFIX: &[u8] = b"localwave.queue-occurrence:";
                    let mut kind = [0u8; 64];
                    kind[..PREFIX.len()].copy_from_slice(PREFIX);
                    let len = PREFIX.len() + id.bytes().len();
                    kind[PREFIX.len()..len].copy_from_slice(id.bytes());
                    ui.carry(
                        if signal.clicked { None } else { Some(&grip) },
                        std::str::from_utf8(&kind[..len]).expect("queue drag kind is ASCII"),
                        title,
                    );
                }
                let art = container(
                    ui,
                    "queue-art",
                    Axis::Stack,
                    Size::Px(30.0),
                    Size::Px(30.0),
                    p.panel,
                );
                paint_artwork(ui, &art, row, 30.0, images);
                ui.a11y_name(
                    &art,
                    if current {
                        "Currently playing"
                    } else {
                        "Track artwork"
                    },
                );
                if current {
                    ui.draw(&art, move |paint, rect| {
                        for (x, height) in [(9.0, 8.0), (14.0, 14.0), (19.0, 10.0)] {
                            paint.fill(
                                rect.x + x,
                                rect.y + (30.0 - height) / 2.0,
                                2.0,
                                height,
                                p.ink,
                            );
                        }
                    });
                }
                let play = ui.node(
                    "play-queued",
                    Node {
                        flags: CLICKABLE | TAB_STOP,
                        role: Role::Button,
                        w: Size::Grow,
                        h: Size::Grow,
                        padding: Edges {
                            l: 10.0,
                            ..Edges::ZERO
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.a11y_name(&play, title);
                ui.within(&play, &mut |ui| {
                    text(
                        ui,
                        "queue-title",
                        title,
                        13,
                        p.ink,
                        (Size::Grow, 19.0),
                        false,
                    );
                    text(
                        ui,
                        "queue-artist",
                        artist,
                        12,
                        p.secondary,
                        (Size::Grow, 17.0),
                        false,
                    );
                });
                if ui.signal_state(&play).clicked {
                    *intent = Some(Intent::PlayQueue(entry.id));
                }
                if !current
                    && icon_button(
                        ui,
                        "remove-queued",
                        "Remove from queue",
                        Icon::Clear,
                        (28.0, 44.0),
                        false,
                        p,
                    )
                {
                    *intent = Some(Intent::RemoveQueue(entry.id));
                }
            });
            let drop = ui.drop_target_ref(&item).and_then(|drop| {
                let (dropped, payload) = match drop {
                    pxui::DropRef::Hover(p) => (false, p),
                    pxui::DropRef::Dropped(p) => (true, p),
                };
                payload
                    .kind
                    .strip_prefix("localwave.queue-occurrence:")?
                    .parse::<u64>()
                    .ok()
                    .map(|id| (dropped, id))
            });
            if let Some((dropped, occurrence)) = drop {
                let after = ui.signal_state(&item).pointer.1 >= ui.rect(&item).y + 22.0;
                let before = if after {
                    entries.get(index + 1).map(|entry| entry.id)
                } else {
                    Some(entry.id)
                };
                if before != Some(occurrence) {
                    ui.draw(&item, move |paint, rect| {
                        paint.fill(
                            rect.x,
                            if after { rect.y + rect.h - 2.0 } else { rect.y },
                            rect.w,
                            2.0,
                            p.ink,
                        );
                    });
                    if dropped {
                        *intent = Some(if full {
                            Intent::MoveFullQueue { occurrence, before }
                        } else {
                            Intent::MoveQueue { occurrence, before }
                        });
                    }
                }
            }
        },
    );
    ui.a11y_name(
        &list,
        if full {
            "Full playback queue"
        } else {
            "Up next queue"
        },
    );
    ui.transfer_carry(&list);
}

#[derive(Clone, Copy)]
struct ClockLabel {
    bytes: [u8; 24],
    start: usize,
}
impl ClockLabel {
    fn integer(mut number: u64) -> Self {
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
fn matrix_clock(ui: &mut Ui, seconds: u64, color: Rgba) {
    use pxui::kit::{AnimationTarget, Keyframe};
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
    let unit = 43.0 / 7.0;
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
            h: Size::Px(43.0),
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
        let mut revision = DefaultHasher::new();
        (shown.bytes(), before.bytes()).hash(&mut revision);
        let revision = revision.finish();
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
        let duration = if player.duration.is_finite() {
            player.duration.max(0.0)
        } else {
            0.0
        };
        let position = if player.position.is_finite() {
            player.position.clamp(0.0, duration)
        } else {
            0.0
        };
        small_clock(ui, "elapsed", elapsed, false, 44.0, p.secondary);
        let scrubber = ui.node(
            "scrubber",
            Node {
                flags: CLIP
                    | if duration > 0.0 {
                        CLICKABLE | FOCUSABLE | FOCUS_MARK
                    } else {
                        0
                    },
                w: Size::Grow,
                h: Size::Px(40.0),
                role: if duration > 0.0 {
                    Role::Slider
                } else {
                    Role::Progress
                },
                ..Node::DEFAULT
            },
        );
        ui.a11y_name(&scrubber, "Playback position");
        ui.a11y_number(&scrubber, position, 0.0, duration, 5.0);
        let signal = ui.signal_state(&scrubber);
        let rect = ui.rect(&scrubber);
        let mut fraction = if duration > 0.0 {
            (position / duration) as f32
        } else {
            0.0
        };
        if duration > 0.0 && rect.w > 0.0 {
            if signal.held || signal.released {
                let x = if signal.released {
                    signal.release_point.0
                } else {
                    signal.pointer.0
                };
                fraction = ((x - rect.x) / rect.w).clamp(0.0, 1.0);
            }
            if signal.released && signal.button == Some(0) {
                *intent = Some(Intent::SeekFraction(fraction as f64));
            }
            if let Some(value) = signal.a11y_value.filter(|value| value.is_finite()) {
                *intent = Some(Intent::SeekFraction((value / duration).clamp(0.0, 1.0)));
            }
            if ui.focused(&scrubber) {
                let seconds = if ui.key_pressed(input::Key::ArrowLeft) {
                    Some(position - 5.0)
                } else if ui.key_pressed(input::Key::ArrowRight) {
                    Some(position + 5.0)
                } else if ui.key_pressed(input::Key::Home) {
                    Some(0.0)
                } else if ui.key_pressed(input::Key::End) {
                    Some(duration)
                } else {
                    None
                };
                if let Some(seconds) = seconds {
                    *intent = Some(Intent::SeekFraction((seconds / duration).clamp(0.0, 1.0)));
                }
            }
        }
        let peaks = player.waveform.copied();
        ui.draw(&scrubber, move |paint, rect| {
            const BARS: usize = 200;
            let baseline = (rect.h - 4.0).max(0.0);
            let step = rect.w / BARS as f32;
            if let Some(peaks) = &peaks {
                for index in 0..BARS {
                    let lower = index * peaks.len() / BARS;
                    let upper = ((index + 1) * peaks.len() / BARS).max(lower + 1);
                    let peak = peaks[lower..upper]
                        .iter()
                        .copied()
                        .fold(0.0, f32::max)
                        .clamp(0.0, 1.0);
                    let height = (peak * baseline).max(2.0);
                    paint.fill_fractional(
                        rect.x + index as f32 * step,
                        rect.y + baseline - height,
                        step / 4.0,
                        height,
                        Rgba {
                            a: (p.ink.a as f32 * 0.42).round() as u8,
                            ..p.ink
                        },
                    );
                }
            }
            paint.fill(rect.x, rect.y + baseline + 1.0, rect.w, 1.0, p.line);
            paint.fill_fractional(rect.x, rect.y + baseline, rect.w * fraction, 3.0, p.ink);
            paint.fill_fractional(
                rect.x + rect.w * fraction - step / 4.0,
                rect.y,
                step / 2.0,
                rect.h,
                p.signal,
            );
        });
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
        let signal = ui.signal_state(&volume);
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
            "queue",
            "Show full queue",
            Icon::Queue,
            (36.0, 40.0),
            false,
            p,
        ) {
            *intent = Some(Intent::ShowQueue);
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
