//! LocalWave's pxui views. Application state changes are returned as intents.
pub mod artwork;
pub mod browse;
pub mod catalog;
pub mod files;
mod icons;
pub mod motion;
pub mod queue;
mod shell;
pub use shell::{ShellState, shell};

pub const FONT: &[u8] = include_bytes!("../../../localwave/Fonts/HostGrotesk.ttf");
pub const MONO: &[u8] = include_bytes!("../../../localwave/Fonts/GeistMono.ttf");
pub const LIGHT: &str = include_str!("../themes/light.theme");
pub const DARK: &str = include_str!("../themes/dark.theme");

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsMenu {
    Cache,
    Appearance,
}

// Fixed UTF-8 packets avoid heap allocations when handing searches or playlist names to the worker.
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
    AppendSong(i64),
    SongMenu(usize, i64, u64, (f32, f32)),
    CloseSongMenu,
    TogglePanel,
    Shuffle,
    ShuffleAll,
    SortAlbums(browse::AlbumSort),
    Repeat,
    SeekFraction(f64),
    Volume(f32),
    RefreshLibrary,
    Search(browse::Query),
    SearchTooLong,
    Sort(browse::Sort),
    PlayQueue(u64),
    RemoveQueue(u64),
    MoveQueue {
        occurrence: u64,
        before: Option<u64>,
    },
    MoveFullQueue {
        occurrence: u64,
        before: Option<u64>,
    },
    ClearQueue,
    UndoQueue,
    ShowQueue,
    CloseQueue,
    NamePlaylist,
    EditPlaylistName(browse::Query),
    PlaylistNameTooLong,
    CancelPlaylistName,
    SaveQueue(browse::Query),
    ShowSettings,
    CloseSettings,
    ShowPrivacy,
    ClosePrivacy,
    SettingsMenu(SettingsMenu, (f32, f32)),
    CloseSettingsMenu,
    Setting(lw_core::preferences::Change),
    PrivacyPolicy,
    Support,
    Files(files::Request),
    ChooseFolder,
    ChooseAudio,
    SourceMenu(files::Target, (f32, f32)),
    CloseSourceMenu,
    ReconnectSource(files::Target),
    ConfirmRemoveSource(files::Target),
    CloseRemoveSource,
}

#[derive(Default)]
pub struct PlayerView<'a> {
    pub files: Option<&'a files::Snapshot>,
    pub file_dragging: bool,
    pub settings: lw_core::preferences::Settings,
    pub cache_usage: &'a str,
    pub settings_message: &'a str,
    pub storage_busy: bool,
    pub artwork: Option<&'a artwork::Images>,
    pub waveform: Option<&'a lw_core::waveform::Peaks>,
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
    pub(crate) fn prepared(songs: Vec<lw_core::Song>) -> catalog::Catalog {
        let mut catalog = catalog::Catalog::new(songs);
        catalog.prepare_tables().unwrap();
        catalog
    }

    #[test]
    fn line_boxes_match_coretext_baselines_at_both_pixel_densities_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        let mut text = pxui::Text::new(&[FONT, MONO]).unwrap();
        let samples = [
            (0, 34.0, 34.0, 29.0),
            (0, 19.0, 22.8, 17.0),
            (0, 14.0, 19.6, 14.0),
            (0, 13.0, 18.2, 13.0),
            (0, 12.0, 16.8, 12.0),
            (1, 12.0, 16.8, 12.0),
        ];
        let metrics = samples.map(|(font, size, line, _)| {
            text.measure(
                font as u64 + size as u64,
                "Ag01",
                &pxui::TextStyle {
                    font,
                    size_points: size,
                    line_height: line,
                    ..Default::default()
                },
                0.0,
            )
            .unwrap()
        });
        for round in 0..5 {
            mem_sys::local_reset();
            for scale in [1.0, 2.0] {
                for (index, ((font, size, line, baseline), metrics)) in
                    samples.into_iter().zip(metrics).enumerate()
                {
                    let style = pxui::TextStyle {
                        font,
                        size_points: size,
                        line_height: line,
                        ..Default::default()
                    };
                    ui.frame(
                        &FrameIn {
                            width: 400.0,
                            height: 100.0,
                            scale,
                            time: round as f64,
                            ..Default::default()
                        },
                        &mut |ui| {
                            let a = ui.node(
                                "line-box",
                                pxui::ui::Node {
                                    role: pxui::ui::Role::Label,
                                    w: pxui::ui::Size::Px(200.0),
                                    h: pxui::ui::Size::Px(line),
                                    at: Some((20.0, 20.0)).into(),
                                    look: pxui::ui::Look {
                                        text: pxui::geom::Rgba {
                                            r: 231,
                                            g: 0,
                                            b: 0,
                                            a: 255,
                                        },
                                        ..pxui::ui::Look::NONE
                                    },
                                    ..pxui::ui::Node::DEFAULT
                                },
                            );
                            ui.text_line_box(&a, "Ag01", &style, &[]).unwrap();
                            let b = ui.node(
                                "reference",
                                pxui::ui::Node {
                                    w: pxui::ui::Size::Px(200.0),
                                    h: pxui::ui::Size::Px(line),
                                    at: Some((20.0, 20.0)).into(),
                                    ..pxui::ui::Node::DEFAULT
                                },
                            );
                            ui.draw(&b, move |paint, rect| {
                                paint
                                    .text_spans(
                                        (rect.x, rect.y + baseline - metrics.baseline),
                                        &style,
                                        200.0,
                                        Some(pxui::geom::Rgba {
                                            r: 0,
                                            g: 0,
                                            b: 231,
                                            a: 255,
                                        }),
                                        "Ag01",
                                        &[],
                                    )
                                    .unwrap();
                            });
                        },
                    )
                    .unwrap();
                    let actual = ui.layers().flat_map(|l| l.instances.iter()).filter(|i| {
                        i.kind == pxui::draw::kind::MASK && i.color == [231, 0, 0, 255]
                    });
                    let reference = ui.layers().flat_map(|l| l.instances.iter()).filter(|i| {
                        i.kind == pxui::draw::kind::MASK && i.color == [0, 0, 231, 255]
                    });
                    let mut matched = 0;
                    for (a, b) in actual.zip(reference) {
                        matched += 1;
                        assert_eq!(a.rect, b.rect, "CoreText line box {index}, scale {scale}");
                    }
                    assert_eq!(matched, 4);
                }
            }
            if round >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "baseline placement must reuse shaping and glyphs"
                );
            }
        }
    }

    #[test]
    fn desktop_album_cards_and_player_artwork_use_swift_point_dimensions() {
        let catalog = prepared(
            (0..24)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Song {id}"),
                    album: format!("Album {id:02}"),
                    artist: "Artist".into(),
                    release_year: Some(2021),
                    ..Default::default()
                })
                .collect(),
        );
        assert_eq!(catalog.albums[0].subtitle, "Artist · 2021");
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(catalog::Route::Albums));
        let player = PlayerView {
            initialized: true,
            current_id: Some(0),
            ..Default::default()
        };
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        for width in [1440.0, 1280.0, 1024.0] {
            for _ in 0..3 {
                ui.frame(
                    &FrameIn {
                        width,
                        height: 872.0,
                        reduced_motion: 1,
                        ..Default::default()
                    },
                    &mut |ui| shell(ui, &player, &catalog, &state, &mut None),
                )
                .unwrap();
            }
            let available = width - 232.0 - 320.0 - 64.0;
            let columns = ((available + 20.0) / 170.0).floor().max(1.0);
            let side = (available - (columns - 1.0) * 20.0) / columns;
            let mut cards = ui
                .a11y_nodes()
                .iter()
                .filter(|n| ui.a11y_name_of(n) == "Album artwork");
            let first = cards.next().unwrap().rect;
            let second = cards.next().unwrap().rect;
            assert!((first.x - 264.0).abs() < 0.001, "32-point content inset");
            assert!(
                (first.w - side).abs() < 0.001 && (first.h - side).abs() < 0.001,
                "adaptive square {first:?}"
            );
            assert!(
                (second.x - first.x - side - 20.0).abs() < 0.001,
                "20-point column gap"
            );
            assert!(
                (first.y - 172.6).abs() < 0.001,
                "native SwiftUI album header/tool origin"
            );
            let next_row = ui
                .a11y_nodes()
                .iter()
                .filter(|n| ui.a11y_name_of(n) == "Album artwork")
                .nth(columns as usize)
                .unwrap()
                .rect;
            assert!(
                (next_row.y - first.y - side - 43.7 - 28.0).abs() < 0.001,
                "native SwiftUI fractional row stride"
            );
            let player = ui
                .a11y_nodes()
                .iter()
                .find(|n| ui.a11y_name_of(n) == "Current track artwork")
                .unwrap()
                .rect;
            assert!((player.x - (width - 319.0 + 20.0)).abs() < 0.001);
            assert!((player.y - 48.0).abs() < 0.001 && player.w == 200.0 && player.h == 200.0);
        }
    }

    #[test]
    fn external_drop_overlay_retains_accessible_instructions_and_has_no_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let catalog = catalog::Catalog::default();
        let state = ShellState::default();
        let mut time = 0.0;
        for round in 0..6 {
            mem_sys::local_reset();
            for width in [640.0, 1440.0] {
                for dragging in [false, true, true, false] {
                    time += 1.0 / 60.0;
                    ui.frame(
                        &FrameIn {
                            width,
                            height: 872.0,
                            time,
                            ..Default::default()
                        },
                        &mut |ui| {
                            shell(
                                ui,
                                &PlayerView {
                                    file_dragging: dragging,
                                    ..Default::default()
                                },
                                &catalog,
                                &state,
                                &mut None,
                            );
                        },
                    )
                    .unwrap();
                    let root = ui
                        .a11y_nodes()
                        .iter()
                        .find(|node| node.is_role(pxui::a11y::Role::Window))
                        .unwrap();
                    assert_eq!(ui.a11y_name_of(root).contains("Drop to play"), dragging);
                    for x in [12.0, 24.0] {
                        let dash =
                            ui.layers()
                                .flat_map(|layer| layer.instances.iter())
                                .any(|instance| {
                                    instance.kind == pxui::draw::kind::RECT
                                        && instance.rect == [x, 12.0, x + 6.0, 14.0]
                                });
                        assert_eq!(dash, dragging, "six-point dash and gap at {x}");
                    }
                }
            }
            if round >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0);
            }
        }
    }

    #[test]
    fn unknown_playback_length_keeps_clock_and_progress_accessibility_valid() {
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let catalog = prepared(Vec::new());
        let state = ShellState::default();
        for (revision, (position, duration)) in [
            (23.0, 0.0),
            (2.0, 1.0),
            (-1.0, 2.0),
            (f64::NAN, f64::INFINITY),
        ]
        .into_iter()
        .enumerate()
        {
            let player = PlayerView {
                revision: revision as u64,
                position,
                duration,
                ..Default::default()
            };
            ui.frame(
                &FrameIn {
                    width: 1440.0,
                    height: 872.0,
                    ..Default::default()
                },
                &mut |ui| shell(ui, &player, &catalog, &state, &mut None),
            )
            .unwrap();
            let progress = ui
                .a11y_nodes()
                .iter()
                .find(|node| ui.a11y_name_of(node) == "Playback position")
                .unwrap();
            assert!(progress.is_role(if duration.is_finite() && duration > 0.0 {
                pxui::ui::Role::Slider
            } else {
                pxui::ui::Role::Progress
            }));
        }
    }

    #[test]
    fn full_queue_sheet_virtualizes_marks_current_and_saves_bounded_names_without_warm_allocations()
    {
        use lw_core::{context::PlaybackContext, queue::Entry};
        use std::sync::Arc;
        mem_sys::set_peer(pxui::library_counter);
        let mut catalog = prepared(vec![lw_core::Song {
            id: Some(7),
            title: "Repeated queue song".into(),
            artist: "Artist".into(),
            ..Default::default()
        }]);
        catalog.queue = Some(Arc::new(queue::Snapshot::new(
            1,
            (1..=10_000).map(|id| Entry { id, song_id: 7 }).collect(),
            Some(1),
            Arc::new(PlaybackContext::Library),
            false,
        )));
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        state.apply(Intent::SelectRow(0));
        state.apply(Intent::ShowQueue);
        let mut player = PlayerView {
            initialized: true,
            title: "Repeated queue song",
            duration: 30_000.0,
            ..Default::default()
        };
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..Default::default()
        };
        let mut intent = None;
        for _ in 0..3 {
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
        }
        assert!(
            ui.a11y_nodes().len() < 700,
            "ten thousand occurrences remain virtual"
        );
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Currently playing")
        );
        let queue_rect = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Full playback queue")
            .unwrap()
            .rect;
        let rows: Vec<_> = ui
            .a11y_nodes()
            .iter()
            .filter(|node| {
                node.is_role(pxui::ui::Role::ListItem)
                    && ui.a11y_name_of(node) == "Repeated queue song"
                    && node.rect.x >= queue_rect.x
                    && node.rect.x < queue_rect.x + queue_rect.w
            })
            .map(|node| node.rect)
            .collect();
        assert!(rows.len() > 3 && rows.len() < 30);
        assert!(
            rows.windows(2)
                .all(|rows| (rows[1].y - rows[0].y - 44.0).abs() < 0.1),
            "one column of 44-point rows: {rows:?}"
        );
        let current_remove = ui.a11y_nodes().iter().any(|node| {
            ui.a11y_name_of(node) == "Remove from queue"
                && node.rect.x >= queue_rect.x
                && node.rect.x < queue_rect.x + queue_rect.w
                && (node.rect.y - rows[1].y).abs() < 0.1
        });
        assert!(!current_remove, "the current occurrence cannot be removed");
        let grips: Vec<_> = ui
            .a11y_nodes()
            .iter()
            .filter(|node| {
                ui.a11y_name_of(node) == "Reorder queued track"
                    && node.rect.x >= queue_rect.x
                    && node.rect.x < queue_rect.x + queue_rect.w
                    && node.rect.y > rows[1].y
            })
            .map(|node| (node.id, node.rect))
            .collect();
        let pointer = |kind, rect: pxui::geom::Rect| pxui::Event {
            kind,
            x: (rect.x + 10.0) as f64,
            y: (rect.y + 10.0) as f64,
            ..Default::default()
        };
        for event in [
            pointer(input::wire::POINTER_PRESSED, grips[0].1),
            pointer(input::wire::POINTER_MOVED, grips[2].1),
            pxui::Event {
                kind: input::wire::SCROLLED,
                y: -30.0,
                ..Default::default()
            },
        ] {
            frame.events = &event;
            frame.event_count = 1;
            frame.time += 0.01;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
            assert!(intent.is_none());
        }
        assert_eq!(
            ui.carrying_ref().unwrap().kind,
            "localwave.queue-occurrence:3"
        );
        assert!(
            !ui.a11y_nodes().iter().any(|node| node.id == grips[0].0),
            "the modal's drag survives pruning its source row"
        );
        let release = pointer(input::wire::POINTER_RELEASED, grips[2].1);
        frame.events = &release;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(
            intent.take(),
            Some(Intent::MoveFullQueue {
                occurrence: 3,
                before: Some(35)
            })
        );
        let scroll_back = pxui::Event {
            kind: input::wire::SCROLLED,
            y: 30.0,
            ..Default::default()
        };
        frame.events = &scroll_back;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        frame.events = std::ptr::null();
        frame.event_count = 0;
        let activate = |ui: &mut Ui, name: &str| {
            let id = ui
                .a11y_nodes()
                .iter()
                .find(|node| node.is_role(pxui::ui::Role::Button) && ui.a11y_name_of(node) == name)
                .unwrap()
                .id;
            ui.inject_a11y(&[pxui::A11yAction {
                id,
                action_code: 1,
                ..Default::default()
            }]);
        };
        activate(&mut ui, "Save queue as playlist");
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent.take(), Some(Intent::NamePlaylist));
        state.apply(Intent::NamePlaylist);
        for _ in 0..3 {
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
        }
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Create playlist from queue")
        );
        activate(&mut ui, "Cancel playlist creation");
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent.take(), Some(Intent::CancelPlaylistName));
        state.apply(Intent::CancelPlaylistName);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Playlist name")
        );
        state.apply(Intent::NamePlaylist);
        let name = browse::Query::new("  Air 🎵  ").unwrap();
        state.apply(Intent::EditPlaylistName(name));
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        activate(&mut ui, "Create playlist from queue");
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent.take(), Some(Intent::SaveQueue(name)));
        state.apply(Intent::PlaylistNameTooLong);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Create playlist from queue")
        );
        state.apply(Intent::EditPlaylistName(name));
        for cycle in 0..24 {
            mem_sys::local_reset();
            let events = [
                pxui::Event {
                    kind: input::wire::POINTER_MOVED,
                    x: (queue_rect.x + 100.0) as f64,
                    y: (queue_rect.y + 100.0) as f64,
                    ..Default::default()
                },
                pxui::Event {
                    kind: input::wire::SCROLLED,
                    y: if cycle % 2 == 0 { -4.0 } else { 4.0 },
                    ..Default::default()
                },
            ];
            frame.events = events.as_ptr();
            frame.event_count = events.len();
            frame.time += 0.02;
            player.revision += 1;
            player.position += 1.0;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
            if cycle > 8 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm modal/editor/queue scroll and live clock"
                );
            }
            assert!(intent.is_none());
        }
        let escape = pxui::Event {
            kind: input::wire::KEY_PRESSED,
            code: input::Key::Escape.code(),
            ..Default::default()
        };
        frame.events = &escape;
        frame.event_count = 1;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent.take(), Some(Intent::CloseQueue));
        state.apply(Intent::CloseQueue);
        frame.events = std::ptr::null();
        frame.event_count = 0;
        state.apply(Intent::ShowQueue);
        frame.width = 390.0;
        frame.height = 778.0;
        for _ in 0..3 {
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
        }
        let list = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Full playback queue")
            .unwrap();
        assert!(
            list.rect.x >= 0.0
                && list.rect.x + list.rect.w <= 390.0
                && list.rect.y + list.rect.h <= 778.0,
            "{list:?}"
        );
    }

    #[test]
    fn window_titlebar_inset_keeps_controls_clear_and_rejects_invalid_geometry() {
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let catalog = prepared(Vec::new());
        let mut state = ShellState::default();
        let player = PlayerView::default();
        for inset in [28.0, f32::NAN, -1.0, 0.0, 44.0] {
            state.set_window_inset(inset);
            for _ in 0..3 {
                ui.frame(
                    &FrameIn {
                        width: 1440.0,
                        height: 872.0,
                        ..Default::default()
                    },
                    &mut |ui| shell(ui, &player, &catalog, &state, &mut None),
                )
                .unwrap();
            }
            let expected = if inset.is_nan() || inset < 0.0 {
                28.0
            } else {
                inset
            };
            assert!(
                ui.a11y_nodes()
                    .iter()
                    .filter(|node| node.is_role(pxui::ui::Role::Button))
                    .all(|node| node.rect.y >= expected),
                "controls overlap native title bar at {inset}"
            );
        }
        for _ in 0..3 {
            ui.frame(
                &FrameIn {
                    width: 1000.0,
                    height: 480.0,
                    ..Default::default()
                },
                &mut |ui| shell(ui, &player, &catalog, &state, &mut None),
            )
            .unwrap();
        }
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Current track artwork"),
            "the tall player panel collapses in a short window"
        );
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Show full queue"
                    && node.rect.y + node.rect.h <= 480.0)
        );
    }

    #[test]
    fn up_next_virtualizes_duplicate_occurrences_and_routes_warm_queue_actions_and_drag() {
        use lw_core::{context::PlaybackContext, queue::Entry};
        use std::sync::Arc;
        mem_sys::set_peer(pxui::library_counter);
        let mut catalog = prepared(vec![lw_core::Song {
            id: Some(7),
            title: "Duplicate queued song".into(),
            artist: "Artist".into(),
            ..Default::default()
        }]);
        let entries: Arc<[Entry]> = (1..=10_000).map(|id| Entry { id, song_id: 7 }).collect();
        let context = Arc::new(PlaybackContext::Library);
        catalog.queue = Some(Arc::new(queue::Snapshot::new(
            1,
            entries.clone(),
            Some(0),
            context.clone(),
            false,
        )));
        let mut state = ShellState::default();
        state.apply(Intent::SelectRow(0));
        let mut ui = Ui::create(Platform::Apple, Some(15), &[FONT, MONO]);
        ui.set_a11y(true);
        let mut player = PlayerView {
            waveform: Some(&[0.5; 512]),
            initialized: true,
            duration: 20_000.0,
            ..Default::default()
        };
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..Default::default()
        };
        let mut intent = None;
        // Capture drag targets after the entrance settles on the injected clock.
        for time in [0.0, 0.5, 1.0] {
            frame.time = time;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut intent)
            })
            .unwrap();
        }
        let grips: Vec<_> = ui
            .a11y_nodes()
            .iter()
            .filter(|node| ui.a11y_name_of(node) == "Reorder queued track")
            .map(|node| (node.id, node.rect))
            .collect();
        assert!(
            grips.len() >= 3 && grips.len() < 100,
            "only the visible queue is built: {}",
            grips.len()
        );
        let first = grips[0].1;
        let third = grips[2].1;
        let event = |kind, rect: pxui::geom::Rect, dy| pxui::Event {
            kind,
            x: (rect.x + 10.0) as f64,
            y: (rect.y + dy) as f64,
            ..Default::default()
        };
        for cycle in 0..24 {
            mem_sys::local_reset();
            for e in [
                Some(event(input::wire::POINTER_PRESSED, first, 10.0)),
                Some(event(input::wire::POINTER_MOVED, third, 10.0)),
                Some(event(input::wire::POINTER_RELEASED, third, 10.0)),
                None,
            ] {
                frame.events = e.as_ref().map_or(std::ptr::null(), |e| e);
                frame.event_count = usize::from(e.is_some());
                frame.time += 0.01;
                player.revision += 1;
                player.position += 1.0;
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut intent)
                })
                .unwrap();
                if e.is_some_and(|e| e.kind == input::wire::POINTER_RELEASED) {
                    assert_eq!(
                        intent.take(),
                        Some(Intent::MoveQueue {
                            occurrence: 2,
                            before: Some(4)
                        })
                    );
                } else {
                    assert!(intent.is_none());
                }
            }
            if cycle > 5 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm queue gesture and clock update"
                );
            }
        }
        for cycle in 0..16 {
            mem_sys::local_reset();
            let scroll = |y| pxui::Event {
                kind: input::wire::SCROLLED,
                y,
                ..Default::default()
            };
            for e in [
                Some(event(input::wire::POINTER_PRESSED, first, 10.0)),
                Some(event(input::wire::POINTER_MOVED, third, 10.0)),
                Some(scroll(-30.0)),
                None,
                Some(event(input::wire::POINTER_RELEASED, third, 10.0)),
                Some(scroll(30.0)),
                None,
            ] {
                frame.events = e.as_ref().map_or(std::ptr::null(), |e| e);
                frame.event_count = usize::from(e.is_some());
                frame.time += 0.02;
                player.revision += 1;
                player.position += 1.0;
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut intent)
                })
                .unwrap();
                if e.is_none() && intent.is_none() && ui.carrying_ref().is_some() {
                    assert!(
                        !ui.a11y_nodes().iter().any(|node| node.id == grips[0].0),
                        "the original grip is virtualized away"
                    );
                }
                if e.is_some_and(|e| e.kind == input::wire::POINTER_RELEASED) {
                    assert_eq!(
                        intent.take(),
                        Some(Intent::MoveQueue {
                            occurrence: 2,
                            before: Some(34)
                        })
                    );
                } else {
                    assert!(intent.is_none());
                }
            }
            if cycle > 4 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm cross-page queue drag with live clocks"
                );
            }
        }
        for cancel in [
            pxui::Event {
                kind: input::wire::POINTER_CANCELLED,
                ..Default::default()
            },
            pxui::Event {
                kind: input::wire::FOCUS_LOST,
                ..Default::default()
            },
            pxui::Event {
                kind: input::wire::KEY_PRESSED,
                code: input::Key::Escape.code(),
                ..Default::default()
            },
        ] {
            for e in [
                event(input::wire::POINTER_PRESSED, first, 10.0),
                event(input::wire::POINTER_MOVED, third, 10.0),
                cancel,
                event(input::wire::POINTER_RELEASED, third, 10.0),
            ] {
                frame.events = &e;
                frame.event_count = 1;
                frame.time += 0.01;
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut intent)
                })
                .unwrap();
                assert!(intent.is_none(), "cancelled drags never reorder");
            }
            assert!(ui.carrying_ref().is_none());
        }
        frame.events = std::ptr::null();
        frame.event_count = 0;
        let visible_first_grip = ui
            .a11y_nodes()
            .iter()
            .find(|node| {
                ui.a11y_name_of(node) == "Reorder queued track"
                    && (node.rect.y - first.y).abs() < 0.1
            })
            .unwrap()
            .id;
        let mut next = catalog.clone();
        let mut reordered = entries.to_vec();
        reordered.swap(1, 2);
        next.queue = Some(Arc::new(queue::Snapshot::new(
            2,
            reordered.into(),
            Some(0),
            context.clone(),
            false,
        )));
        mem_sys::local_reset();
        state.refresh(&catalog, &next);
        assert_eq!(
            mem_sys::local_alloc_count(),
            0,
            "queue-only adoption does not scan library selection"
        );
        assert_eq!(state.selected, Some(0));
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &next, &state, &mut intent)
        })
        .unwrap();
        let moved = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.id == visible_first_grip)
            .unwrap();
        assert!(
            (moved.rect.y - first.y - 44.0).abs() < 0.1,
            "occurrence identity follows the reordered row"
        );
        for (name, expected) in [
            ("Duplicate queued song", Intent::PlayQueue(3)),
            ("Remove from queue", Intent::RemoveQueue(3)),
            ("Clear up next", Intent::ClearQueue),
        ] {
            let node = ui
                .a11y_nodes()
                .iter()
                .find(|node| node.is_role(pxui::ui::Role::Button) && ui.a11y_name_of(node) == name)
                .unwrap()
                .id;
            ui.inject_a11y(&[pxui::A11yAction {
                id: node,
                action_code: 1,
                ..Default::default()
            }]);
            frame.time += 0.01;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &next, &state, &mut intent)
            })
            .unwrap();
            assert_eq!(intent.take(), Some(expected));
        }
        let mut cleared = next.clone();
        cleared.queue = Some(Arc::new(queue::Snapshot::new(
            3,
            Arc::from([entries[0]]),
            Some(0),
            context,
            true,
        )));
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &cleared, &state, &mut intent)
        })
        .unwrap();
        let undo = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Undo clear up next")
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id: undo,
            action_code: 1,
            ..Default::default()
        }]);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &cleared, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent, Some(Intent::UndoQueue));
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Remove from queue")
        );
    }

    #[test]
    fn find_token_chips_toggle_without_warm_allocations_wrap_and_reject_oversized_queries() {
        use browse::Query;
        use catalog::Route;
        mem_sys::set_peer(pxui::library_counter);
        let catalog = prepared(Vec::new());
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
        use catalog::Route;
        mem_sys::set_peer(pxui::library_counter);
        let catalog = prepared(
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
        use catalog::Route;
        let catalog = prepared(
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
        let catalog = prepared(Vec::new());
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
        use catalog::Route;
        use lw_core::{Song, context::PlaybackContext};
        let song = |id, title: &str, artist: &str, album: &str| Song {
            id: Some(id),
            title: title.into(),
            artist: artist.into(),
            album: album.into(),
            ..Song::default()
        };
        let mut before = prepared(vec![song(1, "Before", "B", "Z")]);
        before.add_collection(
            "Old playlist name",
            vec![0, 0],
            PlaybackContext::Playlist {
                id: 7,
                name: "Old playlist name".into(),
            },
        );
        let mut after = prepared(vec![
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
        state.refresh(&before, &prepared(Vec::new()));
        assert_eq!(state.route, Route::Albums);
        assert_eq!(state.selected, None);
        state.apply(Intent::Navigate(Route::Collection(2)));
        state.selected = Some(1);
        state.refresh(&before, &prepared(Vec::new()));
        assert_eq!(state.route, Route::Songs);
        assert_eq!(state.selected, None);
    }

    #[test]
    fn catalog_refresh_keeps_collection_scroll_after_indices_and_names_change() {
        use catalog::Route;
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
        let mut before = prepared(songs());
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
        let mut after = prepared(inserted);
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
            assert!((initial - scrolled - 76.0).abs() < 0.001);
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
        let catalog = prepared(Vec::new());
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
        let catalog = prepared(Vec::new());
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
                waveform: Some(&[0.5; 512]),
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
        let catalog = prepared(
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
        let mut catalog = prepared(Vec::new());
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
        let catalog = prepared(
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
        assert!((before.y - row.rect.y - 38.0).abs() < 0.001);
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
        assert!((find(&ui, "Track 001").rect.y - (before.y - 10.0 * 38.0)).abs() < 0.001);
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Track 000")
        );
    }
}
