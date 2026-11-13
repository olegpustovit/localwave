//! LocalWave's pxui views. Application state changes are returned as intents.
pub mod browse;
pub mod catalog;
pub mod motion;
mod shell;
pub use shell::{ShellState, shell};

pub const FONT: &[u8] = include_bytes!("../../../localwave/Fonts/HostGrotesk.ttf");
pub const MONO: &[u8] = include_bytes!("../../../localwave/Fonts/GeistMono.ttf");
pub const LIGHT: &str = include_str!("../themes/light.theme");
pub const DARK: &str = include_str!("../themes/dark.theme");

// The fixed UTF-8 search packet avoids a heap allocation when handing an edit to the worker.
#[allow(clippy::large_enum_variant)]
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
    RefreshLibrary,
    Search(browse::Query),
    SearchTooLong,
    Sort(browse::Sort),
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
    fn find_token_chips_toggle_without_warm_allocations_wrap_and_reject_oversized_queries() {
        use browse::Query;
        use catalog::{Catalog, Route};
        mem_sys::set_peer(pxui::library_counter);
        let catalog = Catalog::new(Vec::new());
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Find));
        state.apply(Intent::Search(Query::new("CAFÉ").unwrap()));
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let player = PlayerView {
            initialized: true,
            ..Default::default()
        };
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..Default::default()
        };
        let mut intent = None;
        for _ in 0..2 {
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
        }
        let keyboard = [
            pxui::Event {
                kind: input::wire::KEY_PRESSED,
                code: input::Key::Space.code(),
                ..Default::default()
            },
            pxui::Event {
                kind: input::wire::KEY_RELEASED,
                code: input::Key::Space.code(),
                ..Default::default()
            },
        ];
        for cycle in 0..24 {
            let chip = ui
                .a11y_nodes()
                .iter()
                .find(|node| {
                    node.is_role(pxui::ui::Role::Toggle) && ui.a11y_name_of(node) == "flac"
                })
                .unwrap();
            assert_eq!(
                chip.states & pxui::a11y::state::CHECKED != 0,
                cycle % 2 == 1
            );
            assert_ne!(chip.states & pxui::a11y::state::CHECKABLE, 0);
            ui.inject_a11y(&[pxui::A11yAction {
                id: chip.id,
                action_code: if cycle % 2 == 0 { 1 } else { 2 },
                ..Default::default()
            }]);
            if cycle % 2 == 1 {
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut intent)
                })
                .unwrap();
                assert!(intent.is_none());
                frame.events = keyboard.as_ptr();
                frame.event_count = keyboard.len();
            }
            mem_sys::local_reset();
            frame.time += 0.01;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
            let expected = if cycle % 2 == 0 {
                "CAFÉ flac"
            } else {
                "CAFÉ"
            };
            assert_eq!(intent, Some(Intent::Search(Query::new(expected).unwrap())));
            state.apply(intent.take().unwrap());
            frame.events = std::ptr::null();
            frame.event_count = 0;
            frame.time += 0.01;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
            if cycle >= 6 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm filter-chip interaction"
                );
            }
        }
        for width in [960.0, 650.0, 390.0] {
            frame.width = width;
            frame.time += 0.01;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
            let chips: Vec<_> = ui
                .a11y_nodes()
                .iter()
                .filter(|node| {
                    node.is_role(pxui::ui::Role::Toggle)
                        && matches!(
                            ui.a11y_name_of(node),
                            "flac" | "cloud" | "offline" | "year:2019" | "unplayed"
                        )
                })
                .collect();
            assert_eq!(chips.len(), 5);
            for (index, chip) in chips.iter().enumerate() {
                assert!(
                    chip.rect.x >= 0.0
                        && chip.rect.x + chip.rect.w <= width + 0.5
                        && chip.rect.h == 32.0,
                    "chip {:?} outside {width}",
                    chip.rect
                );
                for other in &chips[index + 1..] {
                    assert!(
                        chip.rect.x + chip.rect.w <= other.rect.x
                            || other.rect.x + other.rect.w <= chip.rect.x
                            || chip.rect.y + chip.rect.h <= other.rect.y
                            || other.rect.y + other.rect.h <= chip.rect.y
                    );
                }
            }
            if width == 390.0 {
                assert!(chips[4].rect.y > chips[0].rect.y);
            }
        }
        let maximum = "🎵".repeat(browse::MAX_QUERY_BYTES / 4);
        state.apply(Intent::Search(Query::new(&maximum).unwrap()));
        frame.width = 1440.0;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        let id = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(pxui::ui::Role::Toggle) && ui.a11y_name_of(node) == "flac")
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id,
            action_code: 1,
            ..Default::default()
        }]);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent, Some(Intent::SearchTooLong));
        state.apply(intent.take().unwrap());
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(state.query.as_str(), maximum);
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Search text is too long. Shorten it.")
        );
        state.apply(Intent::Navigate(Route::Albums));
        assert_eq!(
            state.request(&catalog).unwrap().scope,
            browse::Scope::Overview
        );
        assert!(state.request(&catalog).unwrap().query.as_str().is_empty());
    }

    #[test]
    fn find_keeps_focus_hides_stale_results_and_scrolls_warm_without_allocations() {
        use browse::{Query, Scope, Sort};
        use catalog::{Catalog, Route};
        mem_sys::set_peer(pxui::library_counter);
        let catalog = Catalog::new(
            (0..10000)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Track {id:05}"),
                    artist: "Artist".into(),
                    album: "Album".into(),
                    ..Default::default()
                })
                .collect(),
        );
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Find));
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let player = PlayerView {
            initialized: true,
            ..PlayerView::default()
        };
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..Default::default()
        };
        let mut intent = None;
        for _ in 0..2 {
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
        }
        let field = ui
            .a11y_nodes()
            .iter()
            .find(|node| {
                node.is_role(pxui::ui::Role::TextField) && ui.a11y_name_of(node) == "Find songs"
            })
            .unwrap()
            .rect;
        let (x, y) = (
            f64::from(field.x + field.w * 0.8),
            f64::from(field.y + field.h * 0.5),
        );
        let click = [
            pxui::Event {
                kind: input::wire::POINTER_MOVED,
                x,
                y,
                ..Default::default()
            },
            pxui::Event {
                kind: input::wire::POINTER_PRESSED,
                code: 0,
                x,
                y,
                ..Default::default()
            },
            pxui::Event {
                kind: input::wire::POINTER_RELEASED,
                code: 0,
                x,
                y,
                ..Default::default()
            },
        ];
        ui.frame(
            &FrameIn {
                events: click.as_ptr(),
                event_count: click.len(),
                ..frame
            },
            &mut |ui| shell(ui, &player, &catalog, &state, &mut intent),
        )
        .unwrap();
        assert!(ui.text_input_focused());
        let typed = pxui::Event {
            kind: input::wire::TEXT_INPUT,
            text: Str {
                ptr: "track".as_ptr(),
                len: 5,
            },
            ..Default::default()
        };
        ui.frame(
            &FrameIn {
                events: &typed,
                event_count: 1,
                ..frame
            },
            &mut |ui| shell(ui, &player, &catalog, &state, &mut intent),
        )
        .unwrap();
        assert_eq!(intent, Some(Intent::Search(Query::new("track").unwrap())));
        state.apply(intent.take().unwrap());
        let request = state.request(&catalog).unwrap();
        assert_eq!(request.scope, Scope::Find);
        let mut found = catalog.clone();
        found.project(request, (0..400).collect());
        state.refresh(&catalog, &found);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &found, &state, &mut intent)
        })
        .unwrap();
        assert!(
            ui.text_input_focused(),
            "result adoption keeps the editor's identity"
        );
        assert_eq!(state.collection(&found).unwrap().ids.len(), 400);
        for tick in 0..120 {
            let wheel = [
                pxui::Event {
                    kind: input::wire::POINTER_MOVED,
                    x: 650.0,
                    y: 400.0,
                    ..Default::default()
                },
                pxui::Event {
                    kind: input::wire::SCROLLED,
                    y: if tick % 40 < 20 { -1.0 } else { 1.0 },
                    ..Default::default()
                },
            ];
            if tick >= 80 {
                mem_sys::local_reset();
            }
            ui.frame(
                &FrameIn {
                    time: tick as f64 / 60.0,
                    events: wheel.as_ptr(),
                    event_count: wheel.len(),
                    ..frame
                },
                &mut |ui| shell(ui, &player, &found, &state, &mut intent),
            )
            .unwrap();
            if tick >= 80 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "focused Find warm scroll at tick {tick}"
                );
            }
            assert!(ui.text_input_focused());
        }
        let frame = FrameIn { time: 2.0, ..frame };
        state.apply(Intent::Search(Query::new("other").unwrap()));
        assert!(
            state.collection(&found).is_none(),
            "old responses never appear under a newer query"
        );
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &found, &state, &mut intent)
        })
        .unwrap();
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| node.is_role(pxui::ui::Role::ListItem)
                    && ui.a11y_name_of(node).starts_with("Track "))
        );
        let mut next = catalog.clone();
        next.project(state.request(&catalog).unwrap(), (100..400).collect());
        state.refresh(&found, &next);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &next, &state, &mut intent)
        })
        .unwrap();
        let first = ui
            .a11y_nodes()
            .iter()
            .find(|node| {
                node.is_role(pxui::ui::Role::ListItem) && ui.a11y_name_of(node) == "Track 00100"
            })
            .unwrap();
        assert!(
            first.rect.y > 100.0,
            "a new query starts at the top of its own results"
        );
        assert!(ui.text_input_focused());
        assert_eq!(next.view.as_ref().unwrap().request.sort, Sort::Original);
        let clear = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Clear search")
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id: clear,
            action_code: 1,
            ..Default::default()
        }]);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &next, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent, Some(Intent::Search(Query::default())));
    }

    #[test]
    fn sort_headers_request_orders_and_selection_follows_the_prepared_playback_rows() {
        use browse::Sort;
        use catalog::{Catalog, Route};
        let catalog = Catalog::new(
            (0..30)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Track {id:03}"),
                    duration: Some(id as f64),
                    ..Default::default()
                })
                .collect(),
        );
        let mut state = ShellState::default();
        state.apply(Intent::SelectRow(10));
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let player = PlayerView::default();
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..Default::default()
        };
        let mut intent = None;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        let heading = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Sort by Time")
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id: heading,
            action_code: 1,
            ..Default::default()
        }]);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent, Some(Intent::Sort(Sort::Duration)));
        state.apply(intent.take().unwrap());
        assert!(state.ascending);
        state.apply(Intent::Sort(Sort::Duration));
        assert!(!state.ascending);
        let mut sorted = catalog.clone();
        sorted.project(state.request(&catalog).unwrap(), (0..30).rev().collect());
        state.refresh(&catalog, &sorted);
        assert_eq!(state.selected, Some(19));
        assert_eq!(state.collection(&sorted).unwrap().ids[19], 10);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &sorted, &state, &mut intent)
        })
        .unwrap();
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Sort by Time, descending")
        );
        let enter = pxui::Event {
            kind: input::wire::KEY_PRESSED,
            code: input::Key::Enter.code(),
            ..Default::default()
        };
        ui.frame(
            &FrameIn {
                events: &enter,
                event_count: 1,
                ..frame
            },
            &mut |ui| shell(ui, &player, &sorted, &state, &mut intent),
        )
        .unwrap();
        assert_eq!(intent.take(), Some(Intent::PlayRow(19)));
        let reset = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Sort by original order")
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id: reset,
            action_code: 1,
            ..Default::default()
        }]);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &sorted, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent, Some(Intent::Sort(Sort::Original)));
        assert_eq!(sorted.collection(Route::Songs).unwrap().ids.len(), 30);
    }

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
            ("Refresh library", Intent::RefreshLibrary),
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
    fn refreshed_catalog_preserves_identity_and_rebuilds_retained_content() {
        use catalog::{Catalog, Route};
        use lw_core::{Song, context::PlaybackContext};
        let song = |id, title: &str, artist: &str, album: &str| Song {
            id: Some(id),
            title: title.into(),
            artist: artist.into(),
            album: album.into(),
            ..Song::default()
        };
        let mut before = Catalog::new(vec![song(1, "Before", "B", "Z")]);
        before.add_collection(
            "Old playlist name",
            vec![0, 0],
            PlaybackContext::Playlist {
                id: 7,
                name: "Old playlist name".into(),
            },
        );
        let mut after = Catalog::new(vec![
            song(2, "Inserted", "A", "A"),
            song(1, "After", "B", "Z"),
        ]);
        after.add_collection(
            "Another playlist",
            vec![0],
            PlaybackContext::Playlist {
                id: 8,
                name: "Another playlist".into(),
            },
        );
        after.add_collection(
            "Renamed playlist",
            vec![0, 1, 1],
            PlaybackContext::Playlist {
                id: 7,
                name: "Renamed playlist".into(),
            },
        );
        let mut state = ShellState::default();
        for (route, selected, expected_route, expected_selected) in [
            (Route::Songs, Some(0), Route::Songs, Some(1)),
            (Route::Album(0), Some(0), Route::Album(1), Some(0)),
            (Route::Artist(0), Some(0), Route::Artist(1), Some(0)),
            (Route::Collection(2), Some(1), Route::Collection(3), Some(2)),
        ] {
            state.apply(Intent::Navigate(route));
            state.selected = selected;
            mem_sys::local_reset();
            state.refresh(&before, &after);
            assert_eq!(mem_sys::local_alloc_count(), 0);
            assert_eq!(state.route, expected_route);
            assert_eq!(state.selected, expected_selected);
        }
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let player = PlayerView {
            initialized: true,
            ..PlayerView::default()
        };
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..FrameIn::default()
        };
        let mut intent = None;
        state.apply(Intent::Navigate(Route::Songs));
        state.selected = Some(0);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &before, &state, &mut intent)
        })
        .unwrap();
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Before")
        );
        state.refresh(&before, &after);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &after, &state, &mut intent)
        })
        .unwrap();
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "After")
        );
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Inserted")
        );
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Before")
        );
        state.apply(Intent::Navigate(Route::Album(0)));
        state.selected = Some(0);
        state.refresh(&before, &Catalog::new(Vec::new()));
        assert_eq!(state.route, Route::Albums);
        assert_eq!(state.selected, None);
        state.apply(Intent::Navigate(Route::Collection(2)));
        state.selected = Some(1);
        state.refresh(&before, &Catalog::new(Vec::new()));
        assert_eq!(state.route, Route::Songs);
        assert_eq!(state.selected, None);
    }

    #[test]
    fn catalog_refresh_keeps_collection_scroll_after_indices_and_names_change() {
        use catalog::{Catalog, Route};
        use lw_core::{Song, context::PlaybackContext};
        let songs = || {
            (0..100)
                .map(|id| Song {
                    id: Some(id),
                    title: format!("Track {id:03}"),
                    artist: "B".into(),
                    album: "Z".into(),
                    ..Song::default()
                })
                .collect::<Vec<_>>()
        };
        let mut before = Catalog::new(songs());
        before.add_collection(
            "Old name",
            (0..100).collect(),
            PlaybackContext::Playlist {
                id: 7,
                name: "Old name".into(),
            },
        );
        let mut inserted = songs();
        inserted.insert(
            0,
            Song {
                id: Some(100),
                title: "Inserted".into(),
                artist: "A".into(),
                album: "A".into(),
                ..Song::default()
            },
        );
        let mut after = Catalog::new(inserted);
        after.add_collection(
            "Other list",
            vec![0],
            PlaybackContext::Playlist {
                id: 8,
                name: "Other list".into(),
            },
        );
        after.add_collection(
            "Renamed",
            (1..101).collect(),
            PlaybackContext::Playlist {
                id: 7,
                name: "Renamed".into(),
            },
        );
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let player = PlayerView::default();
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..FrameIn::default()
        };
        let mut intent = None;
        let mut state = ShellState::default();
        let row_y = |ui: &Ui| {
            ui.a11y_nodes()
                .iter()
                .find(|node| {
                    node.is_role(pxui::ui::Role::ListItem) && ui.a11y_name_of(node) == "Track 010"
                })
                .unwrap()
                .rect
                .y
        };
        for (old, new) in [
            (Route::Album(0), Route::Album(1)),
            (Route::Collection(2), Route::Collection(3)),
        ] {
            assert_eq!(
                before.collection(old).unwrap().key,
                after.collection(new).unwrap().key
            );
            state.apply(Intent::Navigate(old));
            for _ in 0..2 {
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &before, &state, &mut intent)
                })
                .unwrap();
            }
            let initial = row_y(&ui);
            let list = ui
                .a11y_nodes()
                .iter()
                .find(|node| node.is_role(pxui::ui::Role::List) && node.rect.x > 200.0)
                .unwrap()
                .id;
            ui.inject_a11y(&[pxui::A11yAction {
                id: list,
                action_code: 8,
                payload: 76.0,
                ..Default::default()
            }]);
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &before, &state, &mut intent)
            })
            .unwrap();
            let scrolled = row_y(&ui);
            assert_eq!(initial - scrolled, 76.0);
            state.refresh(&before, &after);
            assert_eq!(state.route, new);
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &after, &state, &mut intent)
            })
            .unwrap();
            assert_eq!(row_y(&ui), scrolled);
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
