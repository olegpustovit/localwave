//! LocalWave's pxui views. Application state changes are returned as intents.
pub mod catalog;
pub mod motion;
mod shell;
pub use shell::{ShellState, shell};

pub const FONT: &[u8] = include_bytes!("../../../localwave/Fonts/HostGrotesk.ttf");
pub const MONO: &[u8] = include_bytes!("../../../localwave/Fonts/GeistMono.ttf");
pub const LIGHT: &str = include_str!("../themes/light.theme");
pub const DARK: &str = include_str!("../themes/dark.theme");

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Intent {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Navigate(catalog::Route),
    SelectRow(usize),
    PlayRow(usize),
    TogglePanel,
    Shuffle,
    Repeat,
    SeekFraction(f64),
    Volume(f32),
}

#[derive(Default)]
pub struct PlayerView<'a> {
    pub revision: u64,
    pub title: &'a str,
    pub context: &'a str,
    pub message: &'a str,
    pub playing: bool,
    pub initialized: bool,
    pub current_id: Option<i64>,
    pub artist: &'a str,
    pub album: &'a str,
    pub position: f64,
    pub duration: f64,
    pub volume: f32,
    pub shuffled: bool,
    pub repeat: lw_core::queue::RepeatMode,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::{FrameIn, Platform, Str, ThemeSource, Ui};

    #[test]
    fn transport_buttons_return_intents_and_update_retained_labels() {
        let catalog = catalog::Catalog::new(Vec::new());
        let state = ShellState::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let mut player = PlayerView {
            revision: 0,
            title: "Example track",
            context: "Library",
            message: "",
            playing: false,
            initialized: true,
            duration: 300.0,
            ..PlayerView::default()
        };
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..FrameIn::default()
        };
        for (name, expected) in [
            ("Play", Intent::Play),
            ("Pause", Intent::Pause),
            ("Previous", Intent::Previous),
            ("Next", Intent::Next),
            ("Stop", Intent::Stop),
        ] {
            player.playing = name == "Pause";
            player.revision += 1;
            let mut intent = None;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
            let id = ui
                .a11y_nodes()
                .iter()
                .find(|node| ui.a11y_name_of(node) == name)
                .unwrap()
                .id;
            ui.inject_a11y(&[pxui::A11yAction {
                id,
                action_code: 1,
                ..pxui::A11yAction::default()
            }]);
            player.revision += 1;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
            assert_eq!(intent, Some(expected));
        }
    }

    #[test]
    fn accessible_seek_volume_and_panel_controls_return_intents() {
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let catalog = catalog::Catalog::new(Vec::new());
        let state = ShellState::default();
        let mut player = PlayerView {
            duration: 240.0,
            initialized: true,
            ..PlayerView::default()
        };
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..FrameIn::default()
        };
        let mut intent = None;
        for (name, action, payload, expected) in [
            ("Volume", 4, 0.25, Intent::Volume(0.25)),
            ("Playback position", 4, 60.0, Intent::SeekFraction(0.25)),
            ("Toggle player panel", 1, 0.0, Intent::TogglePanel),
            ("Shuffle off", 1, 0.0, Intent::Shuffle),
            ("Repeat off", 1, 0.0, Intent::Repeat),
        ] {
            player.revision += 1;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
            let id = ui
                .a11y_nodes()
                .iter()
                .find(|node| ui.a11y_name_of(node) == name)
                .unwrap()
                .id;
            ui.inject_a11y(&[pxui::A11yAction {
                id,
                action_code: action,
                payload,
                ..pxui::A11yAction::default()
            }]);
            player.revision += 1;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
            assert_eq!(intent.take(), Some(expected), "{name}");
        }
    }

    #[test]
    fn shell_themes_and_warm_build_allocate_nothing() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog::Catalog::new(Vec::new());
        let state = ShellState::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        for theme in [LIGHT, DARK] {
            ui.set_theme(&ThemeSource {
                source: Str {
                    ptr: theme.as_ptr(),
                    len: theme.len(),
                },
                ..ThemeSource::default()
            })
            .unwrap();
            let mut player = PlayerView {
                revision: 1,
                title: "Example track",
                context: "Library",
                message: "",
                playing: false,
                initialized: true,
                duration: 300.0,
                ..PlayerView::default()
            };
            let mut intent = None;
            for frame in 0..20 {
                player.revision = frame;
                player.position = frame as f64;
                player.playing = frame % 2 == 0;
                player.context = if player.playing {
                    "Favorites"
                } else {
                    "Library"
                };
                player.message = if player.playing {
                    "Unavailable track skipped"
                } else {
                    ""
                };
                ui.frame(
                    &FrameIn {
                        width: 1440.0,
                        height: 872.0,
                        time: frame as f64,
                        ..FrameIn::default()
                    },
                    &mut |ui| shell(ui, &player, &catalog, &state, &mut intent),
                )
                .unwrap();
            }
            mem_sys::local_reset();
            for frame in 21..121 {
                player.revision = frame;
                player.position = frame as f64;
                player.playing = frame % 2 == 0;
                player.context = if player.playing {
                    "Favorites"
                } else {
                    "Library"
                };
                player.message = if player.playing {
                    "Unavailable track skipped"
                } else {
                    ""
                };
                ui.frame(
                    &FrameIn {
                        width: 1440.0,
                        height: 872.0,
                        time: frame as f64,
                        ..FrameIn::default()
                    },
                    &mut |ui| shell(ui, &player, &catalog, &state, &mut intent),
                )
                .unwrap();
            }
            let allocations = mem_sys::local_alloc_count();
            assert_eq!(allocations, 0);
            assert!(
                ui.a11y_nodes()
                    .iter()
                    .any(|node| ui.a11y_name_of(node) == "Favorites")
            );
            assert!(
                ui.a11y_nodes()
                    .iter()
                    .any(|node| ui.a11y_name_of(node) == "LocalWave")
            );
            assert!(
                ui.a11y_nodes()
                    .iter()
                    .any(|node| ui.a11y_name_of(node) == "Pause")
            );
        }
    }
    #[test]
    fn ten_thousand_song_table_virtualizes_and_reuses_warm_scrolled_rows() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog::Catalog::new(
            (0..10_000)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Track {id:05}"),
                    artist: "Example artist".into(),
                    album: "Example album".into(),
                    duration: Some(240.0),
                    ..lw_core::Song::default()
                })
                .collect(),
        );
        let mut state = ShellState::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let mut player = PlayerView {
            initialized: true,
            duration: 240.0,
            ..PlayerView::default()
        };
        let mut intent = None;
        // Settle initial geometry before sending the repeatable wheel trace.
        for _ in 0..2 {
            ui.frame(
                &FrameIn {
                    width: 1440.0,
                    height: 872.0,
                    ..FrameIn::default()
                },
                &mut |ui| shell(ui, &player, &catalog, &state, &mut intent),
            )
            .unwrap();
        }
        for frame in 0..160 {
            let events = [
                pxui::Event {
                    kind: input::wire::POINTER_MOVED,
                    x: 650.0,
                    y: 400.0,
                    ..pxui::Event::default()
                },
                pxui::Event {
                    kind: input::wire::SCROLLED,
                    y: if frame % 40 < 20 { -1.0 } else { 1.0 },
                    ..pxui::Event::default()
                },
            ];
            player.position = frame as f64;
            player.revision = frame;
            if frame == 80 {
                mem_sys::local_reset();
            }
            let before = mem_sys::local_alloc_count();
            ui.frame(
                &FrameIn {
                    width: 1440.0,
                    height: 872.0,
                    time: frame as f64 / 60.0,
                    events: events.as_ptr(),
                    event_count: events.len(),
                    ..FrameIn::default()
                },
                &mut |ui| shell(ui, &player, &catalog, &state, &mut intent),
            )
            .unwrap();
            if frame >= 80 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    before,
                    "frame {frame} allocated"
                );
            }
            let rows = ui
                .a11y_nodes()
                .iter()
                .filter(|node| ui.a11y_name_of(node).starts_with("Track "))
                .count();
            assert!(
                (1..100).contains(&rows),
                "table must only build visible rows: {rows}"
            );
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);

        let id = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Albums")
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id,
            action_code: 1,
            ..pxui::A11yAction::default()
        }]);
        ui.frame(
            &FrameIn {
                width: 1440.0,
                height: 872.0,
                ..FrameIn::default()
            },
            &mut |ui| shell(ui, &player, &catalog, &state, &mut intent),
        )
        .unwrap();
        assert_eq!(intent, Some(Intent::Navigate(catalog::Route::Albums)));
        assert!(state.apply(intent.take().unwrap()));
        ui.frame(
            &FrameIn {
                width: 1440.0,
                height: 872.0,
                ..FrameIn::default()
            },
            &mut |ui| shell(ui, &player, &catalog, &state, &mut intent),
        )
        .unwrap();
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Example album")
        );
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node).starts_with("Track "))
        );
    }
    #[test]
    fn ten_thousand_sidebar_collections_only_build_visible_rows() {
        mem_sys::set_peer(pxui::library_counter);
        let mut catalog = catalog::Catalog::new(Vec::new());
        for id in 0..10_000 {
            let name = format!("Playlist {id:05}");
            catalog.add_collection(
                &name,
                Vec::new(),
                lw_core::context::PlaybackContext::Playlist {
                    id,
                    name: name.clone(),
                },
            );
        }
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let state = ShellState::default();
        let player = PlayerView::default();
        let mut intent = None;
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..FrameIn::default()
        };
        for _ in 0..2 {
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
        }
        let mut moved = false;
        for tick in 0..160 {
            let events = [
                pxui::Event {
                    kind: input::wire::POINTER_MOVED,
                    x: 100.0,
                    y: 450.0,
                    ..pxui::Event::default()
                },
                pxui::Event {
                    kind: input::wire::SCROLLED,
                    y: if tick % 40 < 20 { -1.0 } else { 1.0 },
                    ..pxui::Event::default()
                },
            ];
            if tick == 80 {
                mem_sys::local_reset();
            }
            ui.frame(
                &FrameIn {
                    time: tick as f64 / 60.0,
                    events: events.as_ptr(),
                    event_count: events.len(),
                    ..frame
                },
                &mut |ui| shell(ui, &player, &catalog, &state, &mut intent),
            )
            .unwrap();
            if tick >= 80 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "frame {tick} allocated");
            }
            let rows = ui.a11y_nodes().iter().filter(|node| {
                node.is_role(pxui::ui::Role::Button)
                    && ui.a11y_name_of(node).starts_with("Playlist ")
            });
            assert!((1..100).contains(&rows.count()));
            moved |= ui.a11y_nodes().iter().any(|node| {
                node.is_role(pxui::ui::Role::Button) && ui.a11y_name_of(node) == "Playlist 00030"
            });
        }
        assert!(moved, "sidebar must scroll to later collections");
    }

    #[test]
    fn song_rows_follow_scroll_selection_navigation_and_resize() {
        let catalog = catalog::Catalog::new(
            (0..100)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Track {id:03}"),
                    is_favorite: id % 2,
                    ..lw_core::Song::default()
                })
                .collect(),
        );
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        let player = PlayerView::default();
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..FrameIn::default()
        };
        let mut intent = None;
        let build =
            |ui: &mut Ui, state: &ShellState, frame: &FrameIn, intent: &mut Option<Intent>| {
                ui.frame(frame, &mut |ui| shell(ui, &player, &catalog, state, intent))
                    .unwrap();
            };
        for _ in 0..2 {
            build(&mut ui, &state, &frame, &mut intent);
        }
        let find = |ui: &Ui, name: &str| {
            *ui.a11y_nodes()
                .iter()
                .find(|node| {
                    node.is_role(pxui::ui::Role::ListItem) && ui.a11y_name_of(node) == name
                })
                .unwrap()
        };
        let before = find(&ui, "Track 010").rect;
        let list = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(pxui::ui::Role::List) && node.rect.x > 200.0)
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id: list,
            action_code: 8,
            payload: 38.0,
            ..pxui::A11yAction::default()
        }]);
        build(&mut ui, &state, &frame, &mut intent);
        let row = find(&ui, "Track 010");
        assert_eq!(before.y - row.rect.y, 38.0);
        ui.inject_a11y(&[pxui::A11yAction {
            id: row.id,
            action_code: 1,
            ..pxui::A11yAction::default()
        }]);
        build(&mut ui, &state, &frame, &mut intent);
        assert_eq!(intent, Some(Intent::SelectRow(10)));
        state.apply(intent.take().unwrap());
        build(&mut ui, &state, &frame, &mut intent);
        let enter = pxui::Event {
            kind: input::wire::KEY_PRESSED,
            code: input::Key::Enter.code(),
            ..pxui::Event::default()
        };
        build(
            &mut ui,
            &state,
            &FrameIn {
                events: &enter,
                event_count: 1,
                ..frame
            },
            &mut intent,
        );
        assert_eq!(intent.take(), Some(Intent::PlayRow(10)));
        state.apply(Intent::TogglePanel);
        build(&mut ui, &state, &frame, &mut intent);
        assert_eq!(find(&ui, "Track 010").rect.w - before.w, 320.0);
        state.apply(Intent::Navigate(catalog::Route::Collection(0)));
        build(&mut ui, &state, &frame, &mut intent);
        assert_eq!(find(&ui, "Track 001").rect.y, before.y - 10.0 * 38.0);
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Track 000")
        );
    }
}
