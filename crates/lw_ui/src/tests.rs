use super::*;
use pxui::{FrameIn, Platform, Str, ThemeHandle, Ui};
pub(crate) fn prepared(songs: Vec<lw_core::Song>) -> catalog::Catalog {
    let mut catalog = catalog::Catalog::new(songs);
    catalog.prepare_tables(&crate::test_table_styles()).unwrap();
    catalog
}

#[test]
fn twenty_position_ticks_rebuild_no_song_rows_and_keep_transport_live() {
    mem_sys::set_peer(pxui::library_counter);
    let catalog = prepared(
        (0..10_000)
            .map(|id| lw_core::Song {
                id: Some(id),
                title: format!("Track {id:05}"),
                ..Default::default()
            })
            .collect(),
    );
    let state = ShellState::default();
    let player = PlayerView {
        revision: 7,
        current_id: Some(0),
        initialized: true,
        playing: true,
        title: "Track 00000",
        duration: 240.0,
        ..Default::default()
    };
    for width in [1440.0, 390.0] {
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
        // Counts rebuilt views, and a verify pass rebuilds every kept view.
        ui.verify_views(false);
        ui.set_a11y(true);
        let mut frame = FrameIn::new(width, 872.0).reduced_motion(true);
        let watched = if width == 1440.0 {
            "song-visual"
        } else {
            "compact-library"
        };
        let mut initial_rows = 0;
        for tick in 0..20 {
            frame.wire.time = tick as f64 * 0.05;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, tick as f64, &catalog, &state, &mut None)
            })
            .unwrap();
            let _ = ui.output(); // The native host consumes the ABI output on every frame.
            initial_rows += ui.rebuilt_named(watched);
        }
        assert!(
            initial_rows > 0,
            "the retained-view counter must observe the initial build at width {width}"
        );
        for tick in 1..=20 {
            let position = 12.0 + tick as f64 * 0.05;
            frame.wire.time += 0.05;
            let layouts = ui.text_layout_count();
            mem_sys::local_reset();
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, position, &catalog, &state, &mut None)
            })
            .unwrap();
            for view in [
                "song-visual",
                "song-copy",
                "sidebar",
                "library",
                "compact-library",
                "track-information",
            ] {
                assert_eq!(
                    ui.rebuilt_named(view),
                    0,
                    "{view}, width {width}, tick {tick}"
                );
            }
            if width == 1440.0 {
                assert!(ui.rebuilt_named("transport") > 0);
                assert!(ui.rebuilt_named("playback-clock") > 0);
                let seek = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| ui.a11y_name_of(node) == "Playback position")
                    .unwrap();
                assert!((seek.value.now - position).abs() < 0.001);
                let clock = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| ui.a11y_name_of(node) == "Elapsed time")
                    .unwrap();
                assert_eq!(clock.value.now, position as u64 as f64);
            }
            assert_eq!(
                mem_sys::local_alloc_count(),
                0,
                "width {width}, tick {tick}, own {}, peer {}, layouts {}",
                mem_sys::own(mem_sys::COUNT_LOCAL_ALLOCS),
                pxui::library_counter(mem_sys::COUNT_LOCAL_ALLOCS),
                ui.text_layout_count() - layouts
            );
        }
        let updated = PlayerView {
            revision: player.revision + 1,
            current_id: Some(1),
            playing: false,
            title: "Updated track",
            ..player
        };
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &updated, 13.0, &catalog, &state, &mut None)
        })
        .unwrap();
        assert!(ui.rebuilt_named(watched) > 0);
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Updated track")
        );
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Play")
        );
    }
}

#[test]
fn line_boxes_match_coretext_baselines_at_both_pixel_densities_without_warm_allocations() {
    mem_sys::set_peer(pxui::library_counter);
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    let samples = [
        (0, 34.0, 34.0, 29.0),
        (0, 19.0, 22.8, 17.0),
        (0, 14.0, 19.6, 14.0),
        (0, 13.0, 18.2, 13.0),
        (0, 12.0, 16.8, 12.0),
        (1, 12.0, 16.8, 12.0),
    ];
    for round in 0..5 {
        mem_sys::local_reset();
        for scale in [1.0, 2.0] {
            for (index, (font, size, line, baseline)) in samples.into_iter().enumerate()
            {
                let label = pxui::Label::BODY.font(font).size(size).line_height(line);
                crate::testing::frame(
                    &mut ui,
                    &FrameIn::new(400.0, 100.0).scale(scale).time(round as f64),
                    &mut |ui| {
                        let a = ui.node(
                            "line-box",
                            pxui::ui::Node::column().w_px(200.0).h_px(line).role(pxui::ui::Role::Label).at(20.0, 20.0).look(pxui::ui::Look::NONE.text(pxui::geom::Rgba::rgb(231, 0, 0))),
                        );
                        ui.text_line_box(&a, "Ag01", label, &[]).unwrap();
                    },
                )
                .unwrap();
                // The first glyph ("A") sits on the baseline: its mask's bottom edge is the line box's baseline, which must be
                // CoreText's for this size and line height (box top at y = 20).
                let a = ui
                    .layers()
                    .flat_map(|l| l.instances.iter())
                    .find(|i| i.kind == pxui::draw::kind::MASK && i.color == [231, 0, 0, 255])
                    .expect("the line box draws its glyphs");
                assert_eq!(a.rect[3] - 20.0, baseline, "CoreText line box {index}, scale {scale}");
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    for width in [1440.0, 1280.0, 1024.0] {
        for _ in 0..3 {
            crate::testing::frame(
                &mut ui,
                &FrameIn::new(width, 872.0).reduced_motion(true),
                &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut None),
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let catalog = catalog::Catalog::default();
    let state = ShellState::default();
    let mut time = 0.0;
    for round in 0..6 {
        mem_sys::local_reset();
        for width in [640.0, 1440.0] {
            for dragging in [false, true, true, false] {
                time += 1.0 / 60.0;
                crate::testing::frame(
                    &mut ui,
                    &FrameIn::new(width, 872.0).time(time),
                    &mut |ui| {
                        shell(
                            ui,
                            &PlayerView {
                                file_dragging: dragging,
                                ..Default::default()
                            },
                            0.0,
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
                    .find(|node| node.is_role(pxui::Role::Window))
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
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
            duration,
            ..Default::default()
        };
        crate::testing::frame(
            &mut ui,
            &FrameIn::new(1440.0, 872.0),
            &mut |ui| shell(ui, &player, position, &catalog, &state, &mut None),
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
fn queue_sheet_virtualizes_upcoming_rows_and_saves_bounded_names_without_warm_allocations() {
    let mut position = 0.0;
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
    let mut ui = test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
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
    let mut frame = FrameIn::new(1440.0, 872.0);
    let mut intent = None;
    for _ in 0..3 {
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, position, &catalog, &state, &mut intent)
        })
        .unwrap();
    }
    assert!(
        ui.a11y_nodes().len() < 700,
        "ten thousand occurrences remain virtual"
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
                && node.rect.w > 1000.0
                && node.rect.x == 10.0
        })
        .map(|node| node.rect)
        .collect();
    assert!(rows.len() > 3 && rows.len() < 30);
    assert!(
        rows.windows(2)
            .all(|rows| (rows[1].y - rows[0].y - 56.0).abs() < 0.1),
        "one column of 56-point rows: {rows:?}"
    );
    let grips: Vec<_> = ui
        .a11y_nodes()
        .iter()
        .filter(|node| {
            ui.a11y_name_of(node) == "Reorder queued track"
                && node.rect.x < 1000.0
                && node.rect.y >= rows[0].y
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
        pxui::Event::pointer(input::wire::SCROLLED, 0, 0.0, -30.0),
    ] {
        frame.wire.events = &event;
        frame.wire.event_count = 1;
        frame.wire.time += 0.01;
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, position, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert!(intent.is_none());
    }
    assert_eq!(
        ui.carrying().unwrap().kind,
        "localwave.queue-occurrence:3"
    );
    assert!(
        !ui.a11y_nodes().iter().any(|node| node.id == grips[0].0),
        "the modal's drag survives pruning its source row"
    );
    let release = pointer(input::wire::POINTER_RELEASED, grips[2].1);
    frame.wire.events = &release;
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &catalog, &state, &mut intent)
    })
    .unwrap();
    assert_eq!(
        intent.take(),
        Some(Intent::MoveQueue {
            occurrence: 3,
            before: Some(35)
        })
    );
    let scroll_back = pxui::Event::pointer(input::wire::SCROLLED, 0, 0.0, 30.0);
    frame.wire.events = &scroll_back;
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &catalog, &state, &mut intent)
    })
    .unwrap();
    frame.wire.events = std::ptr::null();
    frame.wire.event_count = 0;
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
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &catalog, &state, &mut intent)
    })
    .unwrap();
    assert_eq!(intent.take(), Some(Intent::NamePlaylist));
    state.apply(Intent::NamePlaylist);
    for _ in 0..3 {
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, position, &catalog, &state, &mut intent)
        })
        .unwrap();
    }
    assert!(
        !ui.a11y_nodes()
            .iter()
            .any(|node| ui.a11y_name_of(node) == "Create playlist from queue")
    );
    activate(&mut ui, "Cancel playlist creation");
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &catalog, &state, &mut intent)
    })
    .unwrap();
    assert_eq!(intent.take(), Some(Intent::CancelPlaylistName));
    state.apply(Intent::CancelPlaylistName);
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &catalog, &state, &mut intent)
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
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &catalog, &state, &mut intent)
    })
    .unwrap();
    activate(&mut ui, "Create playlist from queue");
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &catalog, &state, &mut intent)
    })
    .unwrap();
    assert_eq!(intent.take(), Some(Intent::SaveQueue(name)));
    state.apply(Intent::PlaylistNameTooLong);
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &catalog, &state, &mut intent)
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
            pxui::Event::pointer(input::wire::POINTER_MOVED, 0, (queue_rect.x + 100.0) as f64, (queue_rect.y + 100.0) as f64),
            pxui::Event::pointer(input::wire::SCROLLED, 0, 0.0, if cycle % 2 == 0 { -4.0 } else { 4.0 }),
        ];
        frame.wire.events = events.as_ptr();
        frame.wire.event_count = events.len();
        frame.wire.time += 0.02;
        player.revision += 1;
        position += 1.0;
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, position, &catalog, &state, &mut intent)
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
    let escape = pxui::Event::key(input::wire::KEY_PRESSED, input::Key::Escape.code());
    frame.wire.events = &escape;
    frame.wire.event_count = 1;
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &catalog, &state, &mut intent)
    })
    .unwrap();
    assert_eq!(intent.take(), Some(Intent::CloseQueue));
    state.apply(Intent::CloseQueue);
    frame.wire.events = std::ptr::null();
    frame.wire.event_count = 0;
    state.apply(Intent::ShowQueue);
    frame.wire.width = 390.0;
    frame.wire.height = 778.0;
    for _ in 0..3 {
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, position, &catalog, &state, &mut intent)
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let catalog = prepared(Vec::new());
    let mut state = ShellState::default();
    let player = PlayerView::default();
    for inset in [28.0, f32::NAN, -1.0, 0.0, 44.0] {
        state.set_window_inset(inset);
        for _ in 0..3 {
            crate::testing::frame(
                &mut ui,
                &FrameIn::new(1440.0, 872.0),
                &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut None),
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
        crate::testing::frame(
            &mut ui,
            &FrameIn::new(1000.0, 480.0),
            &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut None),
        )
        .unwrap();
    }
    assert!(
        !ui.a11y_nodes()
            .iter()
            .any(|node| ui.a11y_name_of(node) == "Current track artwork"),
        "the tall player panel collapses in a short window"
    );
    assert!(ui.a11y_nodes().iter().any(
        |node| ui.a11y_name_of(node) == "Show full queue" && node.rect.y + node.rect.h <= 480.0
    ));
}

#[test]
fn up_next_virtualizes_duplicate_occurrences_and_routes_warm_queue_actions_and_drag() {
    let mut position = 0.0;
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let mut player = PlayerView {
        waveform: Some(&[0.5; 512]),
        initialized: true,
        duration: 20_000.0,
        ..Default::default()
    };
    let mut frame = FrameIn::new(1440.0, 872.0);
    let mut intent = None;
    // Capture drag targets after the entrance settles on the injected clock.
    for time in [0.0, 0.5, 1.0] {
        frame.wire.time = time;
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, position, &catalog, &state, &mut intent)
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
            frame.wire.events = e.as_ref().map_or(std::ptr::null(), |e| e);
            frame.wire.event_count = usize::from(e.is_some());
            frame.wire.time += 0.01;
            player.revision += 1;
            position += 1.0;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, position, &catalog, &state, &mut intent)
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
            frame.wire.events = e.as_ref().map_or(std::ptr::null(), |e| e);
            frame.wire.event_count = usize::from(e.is_some());
            frame.wire.time += 0.02;
            player.revision += 1;
            position += 1.0;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, position, &catalog, &state, &mut intent)
            })
            .unwrap();
            if e.is_none() && intent.is_none() && ui.carrying().is_some() {
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
        pxui::Event::new(input::wire::POINTER_CANCELLED),
        pxui::Event::new(input::wire::FOCUS_LOST),
        pxui::Event::key(input::wire::KEY_PRESSED, input::Key::Escape.code()),
    ] {
        for e in [
            event(input::wire::POINTER_PRESSED, first, 10.0),
            event(input::wire::POINTER_MOVED, third, 10.0),
            cancel,
            event(input::wire::POINTER_RELEASED, third, 10.0),
        ] {
            frame.wire.events = &e;
            frame.wire.event_count = 1;
            frame.wire.time += 0.01;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, position, &catalog, &state, &mut intent)
            })
            .unwrap();
            assert!(intent.is_none(), "cancelled drags never reorder");
        }
        assert!(ui.carrying().is_none());
    }
    frame.wire.events = std::ptr::null();
    frame.wire.event_count = 0;
    let visible_first_grip = ui
        .a11y_nodes()
        .iter()
        .find(|node| {
            ui.a11y_name_of(node) == "Reorder queued track" && (node.rect.y - first.y).abs() < 0.1
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
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &next, &state, &mut intent)
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
        frame.wire.time += 0.01;
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, position, &next, &state, &mut intent)
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
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &cleared, &state, &mut intent)
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
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, position, &cleared, &state, &mut intent)
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let player = PlayerView {
        initialized: true,
        ..Default::default()
    };
    let mut frame = FrameIn::new(1440.0, 872.0);
    let mut intent = None;
    for _ in 0..2 {
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
        })
        .unwrap();
    }
    let keyboard = [
        pxui::Event::key(input::wire::KEY_PRESSED, input::Key::Space.code()),
        pxui::Event::key(input::wire::KEY_RELEASED, input::Key::Space.code()),
    ];
    for cycle in 0..24 {
        let chip = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(pxui::ui::Role::Toggle) && ui.a11y_name_of(node) == "flac")
            .unwrap();
        assert_eq!(
            chip.states & pxui::A11yStates::CHECKED.bits() != 0,
            cycle % 2 == 1
        );
        assert_ne!(chip.states & pxui::A11yStates::CHECKABLE.bits(), 0);
        ui.inject_a11y(&[pxui::A11yAction {
            id: chip.id,
            action_code: if cycle % 2 == 0 { 1 } else { 2 },
            ..Default::default()
        }]);
        if cycle % 2 == 1 {
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, &state, &mut intent)
            })
            .unwrap();
            assert!(intent.is_none());
            frame.wire.events = keyboard.as_ptr();
            frame.wire.event_count = keyboard.len();
        }
        mem_sys::local_reset();
        frame.wire.time += 0.01;
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
        })
        .unwrap();
        let expected = if cycle % 2 == 0 {
            "CAFÉ flac"
        } else {
            "CAFÉ"
        };
        assert_eq!(intent, Some(Intent::Search(Query::new(expected).unwrap())));
        state.apply(intent.take().unwrap());
        frame.wire.events = std::ptr::null();
        frame.wire.event_count = 0;
        frame.wire.time += 0.01;
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
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
    for width in [1440.0, 960.0, 650.0, 390.0, 320.0] {
        frame.wire.width = width;
        // A layout switch starts its slide-in on the first frame; the second one is settled.
        for _ in 0..2 {
            frame.wire.time += 1.0;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, &state, &mut intent)
            })
            .unwrap();
        }
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
            // The phone header has 16 pt gutters (Swift): all five chips fit one row.
            assert!(chips.iter().all(|c| c.rect.y == chips[0].rect.y));
        } else if width == 320.0 {
            assert!(chips[4].rect.y > chips[0].rect.y, "{width} wraps");
            assert_eq!(chips[4].rect.y - chips[0].rect.y, 38.0, "6pt row gap");
        } else if width == 1440.0 {
            assert!(
                chips.iter().all(|c| c.rect.y == chips[0].rect.y),
                "{width} {:?}",
                chips.iter().map(|c| c.rect).collect::<Vec<_>>()
            );
        }
    }
    let maximum = "🎵".repeat(browse::MAX_QUERY_BYTES / 4);
    state.apply(Intent::Search(Query::new(&maximum).unwrap()));
    frame.wire.width = 1440.0;
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut intent)
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
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut intent)
    })
    .unwrap();
    assert_eq!(intent, Some(Intent::SearchTooLong));
    state.apply(intent.take().unwrap());
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut intent)
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let player = PlayerView {
        initialized: true,
        ..PlayerView::default()
    };
    let frame = FrameIn::new(1440.0, 872.0);
    let mut intent = None;
    for _ in 0..2 {
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
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
        pxui::Event::pointer(input::wire::POINTER_MOVED, 0, x, y),
        pxui::Event::pointer(input::wire::POINTER_PRESSED, 0, x, y),
        pxui::Event::pointer(input::wire::POINTER_RELEASED, 0, x, y),
    ];
    crate::testing::frame(
        &mut ui,
        &{
            let mut frame = frame;
            frame.wire.events = click.as_ptr();
            frame.wire.event_count = click.len();
            frame
        },
        &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut intent),
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
    crate::testing::frame(
        &mut ui,
        &{
            let mut frame = frame;
            frame.wire.events = &typed;
            frame.wire.event_count = 1;
            frame
        },
        &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut intent),
    )
    .unwrap();
    assert_eq!(intent, Some(Intent::Search(Query::new("track").unwrap())));
    state.apply(intent.take().unwrap());
    let request = state.request(&catalog).unwrap();
    assert_eq!(request.scope, Scope::Find);
    let mut found = catalog.clone();
    found.project(request, (0..400).collect());
    found.prepare_remap(&catalog);
    state.refresh(&catalog, &found);
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &found, &state, &mut intent)
    })
    .unwrap();
    assert!(
        ui.text_input_focused(),
        "result adoption keeps the editor's identity"
    );
    assert_eq!(state.collection(&found).unwrap().ids.len(), 400);
    for tick in 0..120 {
        let wheel = [
            pxui::Event::pointer(input::wire::POINTER_MOVED, 0, 650.0, 400.0),
            pxui::Event::pointer(input::wire::SCROLLED, 0, 0.0, if tick % 40 < 20 { -1.0 } else { 1.0 }),
        ];
        if tick >= 80 {
            mem_sys::local_reset();
        }
        crate::testing::frame(
            &mut ui,
            &{
                let mut frame = frame.time(tick as f64 / 60.0);
                frame.wire.events = wheel.as_ptr();
                frame.wire.event_count = wheel.len();
                frame
            },
            &mut |ui| shell(ui, &player, 0.0, &found, &state, &mut intent),
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
    let frame = frame.time(2.0);
    state.apply(Intent::Search(Query::new("other").unwrap()));
    assert!(
        state.collection(&found).is_none(),
        "old responses never appear under a newer query"
    );
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &found, &state, &mut intent)
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
    next.prepare_remap(&found);
    state.refresh(&found, &next);
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &next, &state, &mut intent)
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
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &next, &state, &mut intent)
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let player = PlayerView::default();
    let frame = FrameIn::new(1440.0, 872.0);
    let mut intent = None;
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut intent)
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
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut intent)
    })
    .unwrap();
    assert_eq!(intent, Some(Intent::Sort(Sort::Duration)));
    state.apply(intent.take().unwrap());
    assert!(state.ascending);
    state.apply(Intent::Sort(Sort::Duration));
    assert!(!state.ascending);
    let mut sorted = catalog.clone();
    sorted.project(state.request(&catalog).unwrap(), (0..30).rev().collect());
    sorted.prepare_remap(&catalog);
    state.refresh(&catalog, &sorted);
    assert_eq!(state.selected, Some(19));
    assert_eq!(state.collection(&sorted).unwrap().ids[19], 10);
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &sorted, &state, &mut intent)
    })
    .unwrap();
    assert!(
        ui.a11y_nodes()
            .iter()
            .any(|node| ui.a11y_name_of(node) == "Sort by Time, descending")
    );
    let enter = pxui::Event::key(input::wire::KEY_PRESSED, input::Key::Enter.code());
    crate::testing::frame(
        &mut ui,
        &{
            let mut frame = frame;
            frame.wire.events = &enter;
            frame.wire.event_count = 1;
            frame
        },
        &mut |ui| shell(ui, &player, 0.0, &sorted, &state, &mut intent),
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
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &sorted, &state, &mut intent)
    })
    .unwrap();
    assert_eq!(intent, Some(Intent::Sort(Sort::Original)));
    assert_eq!(sorted.collection(Route::Songs).unwrap().ids.len(), 30);
}

#[test]
fn transport_buttons_return_intents_and_update_retained_labels() {
    let catalog = prepared(Vec::new());
    let state = ShellState::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
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
    let frame = FrameIn::new(1440.0, 872.0);
    for (name, expected) in [
        ("Refresh library", Intent::RefreshLibrary),
        ("Play", Intent::Play),
        ("Pause", Intent::Pause),
        ("Previous", Intent::Previous),
        ("Next", Intent::Next),
    ] {
        player.playing = name == "Pause";
        player.revision += 1;
        let mut intent = None;
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
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
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
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
    after.prepare_remap(&before);
    let mut state = ShellState::default();
    for (route, selected, expected_route, expected_selected) in [
        (Route::Songs, Some(0), Route::Songs, Some(1)),
        (Route::Album(0), Some(0), Route::Album(1), Some(0)),
        (Route::Artist(0), Some(0), Route::Artist(1), Some(0)),
        (Route::Collection(6), Some(1), Route::Collection(7), Some(2)),
    ] {
        state.apply(Intent::Navigate(route));
        state.selected = selected;
        mem_sys::local_reset();
        state.refresh(&before, &after);
        assert_eq!(mem_sys::local_alloc_count(), 0);
        assert_eq!(state.route, expected_route);
        assert_eq!(state.selected, expected_selected);
    }
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let player = PlayerView {
        initialized: true,
        ..PlayerView::default()
    };
    let frame = FrameIn::new(1440.0, 872.0);
    let mut intent = None;
    state.apply(Intent::Navigate(Route::Songs));
    state.selected = Some(0);
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &before, &state, &mut intent)
    })
    .unwrap();
    assert!(
        ui.a11y_nodes()
            .iter()
            .any(|node| ui.a11y_name_of(node) == "Before")
    );
    state.refresh(&before, &after);
    crate::testing::frame(&mut ui, &frame, &mut |ui| {
        shell(ui, &player, 0.0, &after, &state, &mut intent)
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
    let mut empty = prepared(Vec::new());
    empty.prepare_remap(&before);
    state.apply(Intent::Navigate(Route::Album(0)));
    state.selected = Some(0);
    state.refresh(&before, &empty);
    assert_eq!(state.route, Route::Albums);
    assert_eq!(state.selected, None);
    state.apply(Intent::Navigate(Route::Collection(6)));
    state.selected = Some(1);
    state.refresh(&before, &empty);
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
    after.prepare_remap(&before);
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let player = PlayerView::default();
    let frame = FrameIn::new(1440.0, 872.0);
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
        (Route::Collection(6), Route::Collection(7)),
    ] {
        assert_eq!(
            before.collection(old).unwrap().key,
            after.collection(new).unwrap().key
        );
        state.apply(Intent::Navigate(old));
        for _ in 0..2 {
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, 0.0, &before, &state, &mut intent)
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
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &before, &state, &mut intent)
        })
        .unwrap();
        let scrolled = row_y(&ui);
        assert!((initial - scrolled - 76.0).abs() < 0.001);
        state.refresh(&before, &after);
        assert_eq!(state.route, new);
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &after, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(row_y(&ui), scrolled);
    }
}

#[test]
fn accessible_seek_volume_and_panel_controls_return_intents() {
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let catalog = prepared(Vec::new());
    let state = ShellState::default();
    let mut player = PlayerView {
        duration: 240.0,
        initialized: true,
        ..PlayerView::default()
    };
    let frame = FrameIn::new(1440.0, 872.0);
    let mut intent = None;
    for (name, action, payload, expected) in [
        ("Playback position", 4, 60.0, Intent::SeekFraction(0.25)),
        ("Toggle now playing panel", 1, 0.0, Intent::TogglePanel),
        ("Shuffle off", 1, 0.0, Intent::Shuffle),
        ("Repeat off", 1, 0.0, Intent::Repeat),
    ] {
        player.revision += 1;
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
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
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent.take(), Some(expected), "{name}");
    }
}

#[test]
fn shell_themes_and_warm_build_allocate_nothing() {
    let mut position;
    mem_sys::set_peer(pxui::library_counter);
    let catalog = prepared(Vec::new());
    let state = ShellState::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    for (source, dark) in [(LIGHT, false), (DARK, true)] {
        let mut theme = ThemeHandle::parse(source).unwrap();
        crate::apply_palette(&mut theme, dark);
        ui.set_theme(&theme).unwrap();
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
            position = frame as f64;
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
            crate::testing::frame(
                &mut ui,
                // Suppress transitions so the verification rebuild has the same geometry.
                &FrameIn::new(1440.0, 872.0).time(frame as f64).reduced_motion(true),
                &mut |ui| shell(ui, &player, position, &catalog, &state, &mut intent),
            )
            .unwrap();
        }
        mem_sys::local_reset();
        for frame in 21..121 {
            player.revision = frame;
            position = frame as f64;
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
            crate::testing::frame(
                &mut ui,
                &FrameIn::new(1440.0, 872.0).time(frame as f64).reduced_motion(true),
                &mut |ui| shell(ui, &player, position, &catalog, &state, &mut intent),
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
    let mut position = 0.0;
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let mut player = PlayerView {
        initialized: true,
        duration: 240.0,
        ..PlayerView::default()
    };
    let mut intent = None;
    // Settle initial geometry before sending the repeatable wheel trace.
    for _ in 0..2 {
        crate::testing::frame(
            &mut ui,
            &FrameIn::new(1440.0, 872.0),
            &mut |ui| shell(ui, &player, position, &catalog, &state, &mut intent),
        )
        .unwrap();
    }
    for frame in 0..160 {
        let events = [
            pxui::Event::pointer(input::wire::POINTER_MOVED, 0, 650.0, 400.0),
            pxui::Event::pointer(input::wire::SCROLLED, 0, 0.0, if frame % 40 < 20 { -1.0 } else { 1.0 }),
        ];
        position = frame as f64;
        player.revision = frame;
        if frame == 80 {
            mem_sys::local_reset();
        }
        let before = mem_sys::local_alloc_count();
        crate::testing::frame(
            &mut ui,
            &{
                let mut frame = FrameIn::new(1440.0, 872.0).time(frame as f64 / 60.0);
                frame.wire.events = events.as_ptr();
                frame.wire.event_count = events.len();
                frame
            },
            &mut |ui| shell(ui, &player, position, &catalog, &state, &mut intent),
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
    crate::testing::frame(
        &mut ui,
        &FrameIn::new(1440.0, 872.0),
        &mut |ui| shell(ui, &player, position, &catalog, &state, &mut intent),
    )
    .unwrap();
    assert_eq!(intent, Some(Intent::Navigate(catalog::Route::Albums)));
    assert!(state.apply(intent.take().unwrap()));
    crate::testing::frame(
        &mut ui,
        &FrameIn::new(1440.0, 872.0),
        &mut |ui| shell(ui, &player, position, &catalog, &state, &mut intent),
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let state = ShellState::default();
    let player = PlayerView::default();
    let mut intent = None;
    let frame = FrameIn::new(1440.0, 872.0);
    for _ in 0..2 {
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
        })
        .unwrap();
    }
    let mut moved = false;
    for tick in 0..160 {
        let events = [
            pxui::Event::pointer(input::wire::POINTER_MOVED, 0, 100.0, 450.0),
            pxui::Event::pointer(input::wire::SCROLLED, 0, 0.0, if tick % 40 < 20 { -1.0 } else { 1.0 }),
        ];
        if tick == 80 {
            mem_sys::local_reset();
        }
        crate::testing::frame(
            &mut ui,
            &{
                let mut frame = frame.time(tick as f64 / 60.0);
                frame.wire.events = events.as_ptr();
                frame.wire.event_count = events.len();
                frame
            },
            &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut intent),
        )
        .unwrap();
        if tick >= 80 {
            assert_eq!(mem_sys::local_alloc_count(), 0, "frame {tick} allocated");
        }
        let rows = ui.a11y_nodes().iter().filter(|node| {
            node.is_role(pxui::ui::Role::Button) && ui.a11y_name_of(node).starts_with("Playlist ")
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
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let mut state = ShellState::default();
    let player = PlayerView::default();
    let frame = FrameIn::new(1440.0, 872.0);
    let mut intent = None;
    let build = |ui: &mut Ui, state: &ShellState, frame: &FrameIn, intent: &mut Option<Intent>| {
        crate::testing::frame(ui, frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, state, intent)
        })
        .unwrap();
    };
    for _ in 0..2 {
        build(&mut ui, &state, &frame, &mut intent);
    }
    let find = |ui: &Ui, name: &str| {
        *ui.a11y_nodes()
            .iter()
            .find(|node| node.is_role(pxui::ui::Role::ListItem) && ui.a11y_name_of(node) == name)
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
    let enter = pxui::Event::key(input::wire::KEY_PRESSED, input::Key::Enter.code());
    build(
        &mut ui,
        &state,
        &{
            let mut frame = frame;
            frame.wire.events = &enter;
            frame.wire.event_count = 1;
            frame
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

#[test]
fn global_shortcuts_emit_swift_intents_and_stay_quiet_for_fields_and_modals() {
    use catalog::Route;
    let catalog = prepared(vec![lw_core::Song {
        id: Some(1),
        title: "Track".into(),
        ..Default::default()
    }]);
    let mut state = ShellState::default();
    let player = PlayerView {
        initialized: true,
        duration: 100.0,
        ..PlayerView::default()
    };
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let mut time = 0.0;
    let width = std::cell::Cell::new(1440.0_f32);
    let mut run = |ui: &mut Ui, state: &ShellState, events: &[pxui::Event]| {
        time += 0.02;
        let frame = {
            let mut frame = FrameIn::new(width.get(), 872.0).time(time);
            frame.wire.events = events.as_ptr();
            frame.wire.event_count = events.len();
            frame
        };
        let mut intent = None;
        crate::testing::frame(ui, &frame, &mut |ui| {
            shell(ui, &player, 10.0, &catalog, state, &mut intent)
        })
        .unwrap();
        intent
    };
    let ev = |kind: u32, code: u32| pxui::Event {
        kind,
        code,
        ..Default::default()
    };
    let tap = |ui: &mut Ui,
               state: &ShellState,
               run: &mut dyn FnMut(&mut Ui, &ShellState, &[pxui::Event]) -> Option<Intent>,
               code: u32| {
        let intent = run(ui, state, &[ev(input::wire::KEY_PRESSED, code)]);
        run(ui, state, &[ev(input::wire::KEY_RELEASED, code)]);
        intent
    };
    run(&mut ui, &state, &[]);
    run(&mut ui, &state, &[]);
    let ch = |c: char| c as u32;
    for (code, expected) in [
        (input::Key::Space.code(), Intent::Play),
        (ch('s'), Intent::Shuffle),
        (ch('r'), Intent::Repeat),
        (ch('i'), Intent::TogglePanel),
        (input::Key::ArrowRight.code(), Intent::SeekFraction(0.15)),
        (input::Key::ArrowLeft.code(), Intent::SeekFraction(0.05)),
    ] {
        assert_eq!(tap(&mut ui, &state, &mut run, code), Some(expected));
    }
    for c in [
        commands::Command::Shuffle,
        commands::Command::Network,
        commands::Command::Settings,
    ] {
        assert_eq!(c.key().is_some(), !c.hint().is_empty());
    }
    let alt = input::Key::Alt.code();
    run(&mut ui, &state, &[ev(input::wire::KEY_PRESSED, alt)]);
    assert_eq!(
        tap(&mut ui, &state, &mut run, input::Key::ArrowRight.code()),
        Some(Intent::SeekFraction(0.4))
    );
    run(&mut ui, &state, &[ev(input::wire::KEY_RELEASED, alt)]);
    let shift = input::Key::Shift.code();
    run(&mut ui, &state, &[ev(input::wire::KEY_PRESSED, shift)]);
    assert_eq!(
        tap(&mut ui, &state, &mut run, input::Key::ArrowRight.code()),
        Some(Intent::Next)
    );
    run(&mut ui, &state, &[ev(input::wire::KEY_RELEASED, shift)]);
    let meta = input::Key::Meta.code();
    run(&mut ui, &state, &[ev(input::wire::KEY_PRESSED, meta)]);
    for (code, expected) in [
        (ch('o'), Intent::ChooseFolder),
        (ch('f'), Intent::Navigate(Route::Find)),
        (ch('n'), Intent::NewList(None)),
        (ch('1'), Intent::Navigate(Route::Albums)),
        (ch('2'), Intent::Navigate(Route::Artists)),
        (ch('3'), Intent::Navigate(Route::Songs)),
        (input::Key::ArrowLeft.code(), Intent::Previous),
        (input::Key::ArrowRight.code(), Intent::Next),
        (input::Key::ArrowUp.code(), Intent::Volume(1.0 / 12.0)),
        (input::Key::ArrowDown.code(), Intent::Volume(0.0)),
    ] {
        assert_eq!(tap(&mut ui, &state, &mut run, code), Some(expected));
    }
    assert_eq!(tap(&mut ui, &state, &mut run, ch('s')), None);
    run(&mut ui, &state, &[ev(input::wire::KEY_RELEASED, meta)]);
    // Swift's Add Files (⇧⌘O) and exact-modifier ⌘ shortcuts.
    run(&mut ui, &state, &[ev(input::wire::KEY_PRESSED, shift)]);
    run(&mut ui, &state, &[ev(input::wire::KEY_PRESSED, meta)]);
    assert_eq!(
        tap(&mut ui, &state, &mut run, ch('o')),
        Some(Intent::ChooseAudio)
    );
    assert_eq!(tap(&mut ui, &state, &mut run, ch('n')), None);
    run(&mut ui, &state, &[ev(input::wire::KEY_RELEASED, meta)]);
    run(&mut ui, &state, &[ev(input::wire::KEY_RELEASED, shift)]);
    state.apply(Intent::ShowShortcuts);
    assert_eq!(
        tap(&mut ui, &state, &mut run, input::Key::Space.code()),
        None
    );
    state.apply(Intent::CloseShortcuts);
    state.apply(Intent::Navigate(Route::Find));
    run(&mut ui, &state, &[]);
    let rect = ui
        .a11y_nodes()
        .iter()
        .find(|node| node.is_role(pxui::ui::Role::TextField))
        .unwrap()
        .rect;
    let (x, y) = (
        f64::from(rect.x + rect.w * 0.8),
        f64::from(rect.y + rect.h * 0.5),
    );
    let at = |kind| pxui::Event {
        kind,
        x,
        y,
        ..Default::default()
    };
    run(
        &mut ui,
        &state,
        &[
            at(input::wire::POINTER_MOVED),
            at(input::wire::POINTER_PRESSED),
            at(input::wire::POINTER_RELEASED),
        ],
    );
    assert!(ui.text_input_focused());
    for code in [
        input::Key::Space.code(),
        ch('s'),
        input::Key::ArrowLeft.code(),
    ] {
        assert_eq!(tap(&mut ui, &state, &mut run, code), None);
    }
    // Compact Now Playing: Esc closes it, but only with no menu or sheet owning Esc.
    width.set(390.0);
    state.apply(Intent::TogglePlayer);
    run(&mut ui, &state, &[]);
    assert_eq!(
        tap(&mut ui, &state, &mut run, input::Key::Escape.code()),
        Some(Intent::TogglePlayer)
    );
}

#[test]
fn shortcuts_keep_press_modifiers_raw_case_and_intents() {
    use catalog::Route;
    use input::Key::{Alt, ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Ctrl, Enter, Escape, Meta, Shift, Space};
    use pxui::Event;

    let catalog = prepared(vec![lw_core::Song { id: Some(1), title: "Track".into(), ..Default::default() }]);
    let mut player = PlayerView { initialized: true, current_id: Some(1), volume: 0.5, duration: 100.0, ..PlayerView::default() };
    let mut state = ShellState::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    let mut tick = 0;
    // Keep these extra input cases independent of the established frame fingerprints.
    let mut run = |state: &ShellState, player: &PlayerView<'_>, events: &[Event], carry: bool| {
        let mut frame = FrameIn::new(1440.0, 872.0).time(tick as f64 / 60.0);
        tick += 1;
        frame.wire.events = events.as_ptr();
        frame.wire.event_count = events.len();
        let mut intent = None;
        ui.frame(&frame, &mut |ui| {
            if carry { ui.carry(None, "song", "1"); }
            shell(ui, player, 40.0, &catalog, state, &mut intent);
        }).unwrap();
        intent
    };
    run(&state, &player, &[], false);
    let edge = |kind, code| Event { kind, code, ..Default::default() };
    let mut press = |state: &ShellState, player: &PlayerView<'_>, mods: &[input::Key], key, release_in_frame, carry| {
        let releases: Vec<_> = mods.iter().map(|key| edge(Event::KEY_RELEASED, key.code())).collect();
        let mut events: Vec<_> = mods.iter().map(|key| edge(Event::KEY_PRESSED, key.code())).collect();
        events.extend([edge(Event::KEY_PRESSED, key), edge(Event::KEY_RELEASED, key)]);
        if release_in_frame { events.extend_from_slice(&releases); }
        let intent = run(state, player, &events, carry);
        if !release_in_frame && !releases.is_empty() { run(state, player, &releases, false); }
        intent
    };
    let ch = |key| key as u32;
    for (mods, key, expected) in [
        (&[][..], Space.code(), Intent::Play),
        (&[][..], ch('s'), Intent::Shuffle),
        (&[][..], ch('r'), Intent::Repeat),
        (&[][..], ch('i'), Intent::TogglePanel),
        (&[][..], ch('l'), Intent::SetFavorite { song: 1, on: true }),
        (&[][..], ArrowLeft.code(), Intent::SeekFraction(0.35)),
        (&[][..], ArrowRight.code(), Intent::SeekFraction(0.45)),
        (&[Alt][..], ArrowLeft.code(), Intent::SeekFraction(0.10)),
        (&[Alt][..], ArrowRight.code(), Intent::SeekFraction(0.70)),
        (&[Shift][..], ArrowLeft.code(), Intent::Previous),
        (&[Shift][..], ArrowRight.code(), Intent::Next),
        (&[Meta][..], ArrowLeft.code(), Intent::Previous),
        (&[Meta][..], ArrowRight.code(), Intent::Next),
        (&[Meta][..], ArrowUp.code(), Intent::Volume(0.5 + 1.0 / 12.0)),
        (&[Meta][..], ArrowDown.code(), Intent::Volume(0.5 - 1.0 / 12.0)),
        (&[Meta][..], ch('O'), Intent::ChooseFolder),
        (&[Meta, Shift][..], ch('O'), Intent::ChooseAudio),
        (&[Meta][..], ch('F'), Intent::Navigate(Route::Find)),
        (&[Meta][..], ch('N'), Intent::NewList(None)),
        (&[Meta][..], ch('1'), Intent::Navigate(Route::Albums)),
        (&[Meta][..], ch('2'), Intent::Navigate(Route::Artists)),
        (&[Meta][..], ch('3'), Intent::Navigate(Route::Songs)),
        (&[Meta][..], ch('K'), Intent::ShowCommands),
        (&[Ctrl][..], ch('/'), Intent::ShowCommands),
        (&[Alt][..], ch('?'), Intent::ShowShortcuts),
        (&[Meta][..], ch(','), Intent::ShowSettings),
        (&[Meta][..], ch('A'), Intent::Select(Selection::All(1))),
    ] {
        assert_eq!(press(&state, &player, mods, key, false, false), Some(expected), "key {key} with {mods:?}");
    }
    assert_eq!(press(&state, &player, &[Meta], ch('O'), true, false), Some(Intent::ChooseFolder));
    assert_eq!(press(&state, &player, &[Meta, Shift], ch('n'), false, false), None);
    assert_eq!(press(&state, &player, &[], ch('L'), false, false), None);

    player.playing = true;
    player.revision += 1;
    assert_eq!(press(&state, &player, &[], Space.code(), false, false), Some(Intent::Pause));
    state.apply(Intent::Select(Selection::All(1)));
    for (mods, key, delta, extend) in [(&[][..], ArrowUp, -1, false), (&[Shift][..], ArrowDown, 1, true)] {
        assert_eq!(press(&state, &player, mods, key.code(), false, false), Some(Intent::Select(Selection::Move { delta, extend, len: 1 })));
    }
    state.apply(Intent::SelectRow(0));
    assert_eq!(press(&state, &player, &[Alt], Enter.code(), false, false), Some(Intent::PlayRow(0)));
    assert_eq!(press(&state, &player, &[], Escape.code(), false, true), None);
    assert_eq!(press(&state, &player, &[], Escape.code(), false, false), Some(Intent::Select(Selection::Clear)));
    state.apply(Intent::ShowSettings);
    assert_eq!(press(&state, &player, &[Meta], ch('['), false, false), Some(Intent::CloseSettings));
    assert_eq!(press(&state, &player, &[], Escape.code(), false, false), Some(Intent::CloseSettings));
}

#[test]
fn back_new_list_delete_return_and_dialog_enter_follow_swift_with_gating() {
    use catalog::Route;
    use lw_core::context::PlaybackContext;
    let mut catalog = prepared(
        (1..=3)
            .map(|id| lw_core::Song {
                id: Some(id),
                title: format!("Track {id}"),
                album: "Album".into(),
                ..Default::default()
            })
            .collect(),
    );
    catalog.add_collection(
        "Mix",
        vec![0, 1, 2],
        PlaybackContext::Playlist {
            id: 7,
            name: "Mix".into(),
        },
    );
    let mut files = files::Snapshot {
        account: Some(1),
        sources: vec![files::Folder {
            id: 2,
            root: 3,
            name: "Music".into(),
            path: "/Music".into(),
            current: true,
            ..Default::default()
        }]
        .into(),
        ..Default::default()
    };
    let location = files::Location {
        user: 1,
        source: 2,
        parent: Some(9),
    };
    files.directory = Some(std::sync::Arc::new(files::Directory {
        location,
        source_root: 3,
        up: Some(8),
        title: "Folder".into(),
        ..Default::default()
    }));
    let player = PlayerView {
        initialized: true,
        files: Some(&files),
        ..PlayerView::default()
    };
    let mut state = ShellState::default();
    // Automatic collections (Favorites) come first.
    let mix = Route::Collection(
        catalog
            .collections
            .iter()
            .position(|c| c.title == "Mix")
            .unwrap(),
    );
    // The worker projects the opened list's rows into the catalog view.
    state.apply(Intent::Navigate(mix));
    catalog.project(state.request(&catalog).unwrap(), vec![0, 1, 2]);
    state.apply(Intent::Navigate(Route::Songs));
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let mut time = 0.0;
    let mut run = |ui: &mut Ui, state: &ShellState, events: &[pxui::Event]| {
        time += 0.02;
        let frame = {
            let mut frame = FrameIn::new(1440.0, 872.0).time(time);
            frame.wire.events = events.as_ptr();
            frame.wire.event_count = events.len();
            frame
        };
        let mut intent = None;
        crate::testing::frame(ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, state, &mut intent)
        })
        .unwrap();
        intent
    };
    let ev = |kind: u32, code: u32| pxui::Event {
        kind,
        code,
        ..Default::default()
    };
    // Taps `code` with the given modifiers held.
    let mut press = |ui: &mut Ui, state: &ShellState, mods: &[input::Key], code: u32| {
        for m in mods {
            run(ui, state, &[ev(input::wire::KEY_PRESSED, m.code())]);
        }
        if code == 0 {
            return run(ui, state, &[]);
        }
        let intent = run(ui, state, &[ev(input::wire::KEY_PRESSED, code)]);
        run(ui, state, &[ev(input::wire::KEY_RELEASED, code)]);
        for m in mods {
            run(ui, state, &[ev(input::wire::KEY_RELEASED, m.code())]);
        }
        intent
    };
    use input::Key::{Backspace, Enter, Escape, Meta, Shift};
    let (open, new_list) = ('[' as u32, 'n' as u32);
    press(&mut ui, &state, &[], 0);
    // ⌘[ from details goes to their index; plain Songs has nowhere to go.
    assert_eq!(press(&mut ui, &state, &[Meta], open), None);
    state.apply(Intent::Navigate(Route::Album(0)));
    assert_eq!(
        press(&mut ui, &state, &[Meta], open),
        Some(Intent::Navigate(Route::Albums))
    );
    state.apply(Intent::Navigate(mix));
    assert_eq!(
        press(&mut ui, &state, &[Meta], open),
        Some(Intent::Navigate(Route::Lists))
    );
    assert_eq!(press(&mut ui, &state, &[Meta, Shift], open), None);
    // Nothing selected: ⇧⌘N, Delete and Return stay quiet.
    for (mods, code) in [
        (&[Meta, Shift][..], new_list),
        (&[][..], Backspace.code()),
        (&[][..], Enter.code()),
    ] {
        assert_eq!(press(&mut ui, &state, mods, code), None);
    }
    state.apply(Intent::Select(Selection::All(3)));
    assert_eq!(
        press(&mut ui, &state, &[Meta, Shift], new_list),
        Some(Intent::NewList(Some(1)))
    );
    assert_eq!(
        press(&mut ui, &state, &[], Backspace.code()),
        Some(Intent::RemoveFromList { song: 1, list: 7 })
    );
    assert_eq!(
        press(&mut ui, &state, &[], Enter.code()),
        Some(Intent::PlaySelection)
    );
    state.apply(Intent::SelectRow(1));
    assert_eq!(
        press(&mut ui, &state, &[], Enter.code()),
        Some(Intent::PlayRow(1))
    );
    // Delete removes only from a list route.
    state.apply(Intent::Navigate(Route::Songs));
    state.apply(Intent::SelectRow(1));
    assert_eq!(press(&mut ui, &state, &[], Backspace.code()), None);
    assert_eq!(
        press(&mut ui, &state, &[Meta, Shift], new_list),
        Some(Intent::NewList(Some(2)))
    );
    // Folders go up one level; settings go back from privacy, then settings.
    state.apply(Intent::Navigate(Route::Files));
    assert_eq!(
        press(&mut ui, &state, &[Meta], open),
        Some(Intent::Files(files::Request::Browse(files::Location {
            parent: Some(8),
            ..location
        })))
    );
    state.apply(Intent::ShowSettings);
    state.apply(Intent::ShowPrivacy);
    assert_eq!(
        press(&mut ui, &state, &[Meta], open),
        Some(Intent::ClosePrivacy)
    );
    state.apply(Intent::ClosePrivacy);
    assert_eq!(
        press(&mut ui, &state, &[Meta], open),
        Some(Intent::CloseSettings)
    );
    state.apply(Intent::CloseSettings);
    // Return confirms the dialogs and Esc cancels; an invalid name stays open.
    state.apply(Intent::NewList(None));
    press(&mut ui, &state, &[], 0);
    assert_eq!(press(&mut ui, &state, &[], Enter.code()), None);
    assert_eq!(press(&mut ui, &state, &[Meta], open), None);
    assert_eq!(
        press(&mut ui, &state, &[], Escape.code()),
        Some(Intent::CancelPlaylistName)
    );
    let name = browse::Query::new("Mix").unwrap();
    state.apply(Intent::NewList(Some(1)));
    state.apply(Intent::EditPlaylistName(name));
    press(&mut ui, &state, &[], 0);
    assert_eq!(
        press(&mut ui, &state, &[], Enter.code()),
        Some(Intent::CreateList {
            name,
            song: Some(1)
        })
    );
    state.apply(Intent::CancelPlaylistName);
    state.apply(Intent::ConfirmDeleteList(7));
    press(&mut ui, &state, &[], 0);
    assert_eq!(
        press(&mut ui, &state, &[], Enter.code()),
        Some(Intent::DeleteList(7))
    );
    state.apply(Intent::CloseDeleteList);
    let target = files::Target {
        user: 1,
        source: 2,
        root: 3,
    };
    state.apply(Intent::ConfirmRemoveSource(target));
    press(&mut ui, &state, &[], 0);
    assert_eq!(
        press(&mut ui, &state, &[], Enter.code()),
        Some(Intent::Files(files::Request::Remove(target)))
    );
}

#[test]
fn favorite_key_heart_and_menu_emit_set_favorite_from_the_current_state() {
    use pxui::ui::Role;
    let catalog = prepared(
        (1..=2)
            .map(|id| lw_core::Song {
                id: Some(id),
                title: format!("Track {id}"),
                is_favorite: (id == 1) as i64,
                ..Default::default()
            })
            .collect(),
    );
    let mut state = ShellState::default();
    let mut player = PlayerView {
        initialized: true,
        current_id: Some(1),
        title: "Track 1",
        ..PlayerView::default()
    };
    let mut ui = test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let mut time = 0.0;
    let mut run =
        |ui: &mut Ui, state: &ShellState, player: &PlayerView<'_>, events: &[pxui::Event]| {
            time += 0.02;
            let frame = {
                let mut frame = FrameIn::new(1440.0, 872.0).time(time);
                frame.wire.events = events.as_ptr();
                frame.wire.event_count = events.len();
                frame
            };
            let mut intent = None;
            crate::testing::frame(ui, &frame, &mut |ui| {
                shell(ui, player, 0.0, &catalog, state, &mut intent)
            })
            .unwrap();
            intent
        };
    let key = |kind, code| pxui::Event {
        kind,
        code,
        ..Default::default()
    };
    let l = 'l' as u32;
    let off = Some(Intent::SetFavorite { song: 1, on: false });
    run(&mut ui, &state, &player, &[]);
    run(&mut ui, &state, &player, &[]);
    let intent = run(
        &mut ui,
        &state,
        &player,
        &[key(input::wire::KEY_PRESSED, l)],
    );
    run(
        &mut ui,
        &state,
        &player,
        &[key(input::wire::KEY_RELEASED, l)],
    );
    assert_eq!(intent, off);
    assert_eq!(commands::Command::Favorite.hint(), "L");
    // The heart is a named button; the menu item reads the row's state.
    let click = |ui: &mut Ui,
                 state: &ShellState,
                 player: &PlayerView<'_>,
                 run: &mut dyn FnMut(
        &mut Ui,
        &ShellState,
        &PlayerView<'_>,
        &[pxui::Event],
    ) -> Option<Intent>,
                 role,
                 name: &str| {
        let id = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(role) && ui.a11y_name_of(node) == name)
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id,
            action_code: 1,
            ..Default::default()
        }]);
        run(ui, state, player, &[])
    };
    assert_eq!(
        click(
            &mut ui,
            &state,
            &player,
            &mut run,
            Role::Button,
            "Unfavorite current song"
        ),
        off
    );
    let group = catalog.all.as_ref().unwrap().key;
    state.apply(Intent::SongMenu(0, 1, group, (500.0, 200.0)));
    run(&mut ui, &state, &player, &[]);
    assert_eq!(
        click(
            &mut ui,
            &state,
            &player,
            &mut run,
            Role::MenuItem,
            "Unfavorite"
        ),
        off
    );
    state.apply(Intent::SongMenu(1, 2, group, (500.0, 200.0)));
    run(&mut ui, &state, &player, &[]);
    assert_eq!(
        click(
            &mut ui,
            &state,
            &player,
            &mut run,
            Role::MenuItem,
            "Favorite"
        ),
        Some(Intent::SetFavorite { song: 2, on: true })
    );
    state.apply(Intent::CloseSongMenu);
    // Nothing current: L is a no-op. The controller bumps the stable revision
    // whenever the current entry changes, so the test does too.
    player.current_id = None;
    player.revision += 1;
    run(&mut ui, &state, &player, &[]);
    assert_eq!(
        run(
            &mut ui,
            &state,
            &player,
            &[key(input::wire::KEY_PRESSED, l)]
        ),
        None
    );
    run(
        &mut ui,
        &state,
        &player,
        &[key(input::wire::KEY_RELEASED, l)],
    );
    // A focused Find field types the letter instead.
    player.current_id = Some(1);
    player.revision += 1;
    state.apply(Intent::Navigate(catalog::Route::Find));
    run(&mut ui, &state, &player, &[]);
    let rect = ui
        .a11y_nodes()
        .iter()
        .find(|node| node.is_role(Role::TextField))
        .unwrap()
        .rect;
    let at = |kind| pxui::Event {
        kind,
        x: f64::from(rect.x + rect.w * 0.8),
        y: f64::from(rect.y + rect.h * 0.5),
        ..Default::default()
    };
    run(
        &mut ui,
        &state,
        &player,
        &[
            at(input::wire::POINTER_MOVED),
            at(input::wire::POINTER_PRESSED),
            at(input::wire::POINTER_RELEASED),
        ],
    );
    assert!(ui.text_input_focused());
    assert_eq!(
        run(
            &mut ui,
            &state,
            &player,
            &[key(input::wire::KEY_PRESSED, l)]
        ),
        None
    );
}

#[test]
fn favorite_burst_runs_only_on_the_off_to_on_transition_and_respects_reduced_motion() {
    let make = |on: i64, id: i64| {
        prepared(
            (1..=2)
                .map(|n| lw_core::Song {
                    id: Some(n),
                    title: format!("Track {n}"),
                    is_favorite: if n == id { on } else { 1 - on },
                    ..Default::default()
                })
                .collect(),
        )
    };
    for (reduced, expect_burst) in [(0, true), (1, false)] {
        let state = ShellState::default();
        let mut ui = test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
        let mut time = 0.0;
        // The controller bumps the stable revision whenever the current entry changes.
        let (mut revision, mut last_id) = (1, None);
        let mut run = |catalog: &catalog::Catalog, id: i64, seconds: f64| {
            time += seconds;
            revision += u64::from(last_id != Some(id));
            last_id = Some(id);
            let player = PlayerView {
                initialized: true,
                current_id: Some(id),
                revision,
                ..PlayerView::default()
            };
            let frame = {
                let mut frame = FrameIn::new(1440.0, 872.0).time(time);
                frame.wire.reduced_motion = reduced;
                frame
            };
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, 0.0, catalog, &state, &mut None)
            })
            .unwrap();
            ui.output().repaint_at.is_finite()
        };
        let (off, on) = (make(0, 1), make(1, 1));
        for _ in 0..40 {
            run(&off, 1, 0.5);
        }
        assert!(!run(&off, 1, 0.5), "idle heart never animates");
        assert_eq!(
            run(&on, 1, 0.01),
            expect_burst,
            "off to on starts the burst"
        );
        for _ in 0..4 {
            run(&on, 1, 0.2);
        }
        assert!(!run(&on, 1, 0.2), "the burst settles");
        // Switching to a song that is already a favourite is not a toggle.
        let other = make(0, 2);
        run(&other, 2, 0.5);
        assert!(!run(&other, 1, 0.01), "track change never bursts");
    }
}

#[test]
fn sidebar_follows_swift_order_and_smart_lists_count_from_worker_snapshots() {
    use lw_core::{FileAvailability, Song};
    mem_sys::set_peer(pxui::library_counter);
    const GIB: i64 = 1 << 30;
    let song = |id: i64, availability, size: i64| Song {
        id: Some(id),
        title: format!("Song {id}"),
        source_availability: availability,
        file_size: Some(size),
        ..Song::default()
    };
    let mut catalog = prepared(vec![
        song(1, Some(FileAvailability::OnDevice), GIB * 3 / 2),
        song(2, Some(FileAvailability::OnDevice), GIB),
        song(3, Some(FileAvailability::InCloud), GIB),
        song(4, None, 0),
    ]);
    // Played once: song 1; kept: songs 2 and 3.
    catalog.prepare_smart_lists(&mut |id| i64::from(id == 1), &mut |id| id == 2 || id == 3, 8);
    let counts = catalog.collections.iter().map(|c| (c.title.as_str(), c.count.as_str())).collect::<Vec<_>>();
    assert_eq!(
        counts[3..],
        [("Kept offline", "2"), ("On this device now", "2"), ("Never played", "3")]
    );
    assert_eq!(catalog.offline_usage, "2.5 / 8 GB");
    assert!((catalog.offline_fraction - 0.3125).abs() < 1e-6);
    let before = std::sync::Arc::as_ptr(&catalog.library);
    catalog.prepare_smart_lists(&mut |id| i64::from(id == 1), &mut |id| id == 2 || id == 3, 8);
    assert_eq!(before, std::sync::Arc::as_ptr(&catalog.library), "unchanged lists keep the page");
    let location = |source| files::Location { user: 1, source, parent: None };
    std::sync::Arc::make_mut(&mut catalog.library).source_rows = vec![
        catalog::SourceRow { location: location(5), name: "iCloud Drive".into(), count: "34".into(), reconnect: false },
        catalog::SourceRow { location: location(6), name: "Dropbox".into(), count: "6".into(), reconnect: true },
    ];

    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let state = ShellState::default();
    let player = PlayerView::default();
    let mut intent = None;
    let frame = FrameIn::new(1440.0, 872.0);
    let run = |ui: &mut Ui, intent: &mut Option<Intent>| {
        crate::testing::frame(ui, &frame, &mut |ui| shell(ui, &player, 0.0, &catalog, &state, intent)).unwrap();
    };
    for _ in 0..2 {
        run(&mut ui, &mut intent);
    }
    let mut buttons = ui
        .a11y_nodes()
        .iter()
        .filter(|node| node.is_role(pxui::ui::Role::Button) && node.rect.x < 231.0 && (1.0..800.0).contains(&node.rect.y))
        .map(|node| (node.rect.y, ui.a11y_name_of(node).to_owned()))
        .collect::<Vec<_>>();
    buttons.sort_by(|a, b| a.0.total_cmp(&b.0));
    let names = buttons.iter().map(|(_, name)| name.as_str()).collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "Search", "Albums", "Artists", "Songs", "iCloud Drive", "Dropbox", "Add folder or files",
            "Favorites", "Recently added", "Most played", "Kept offline", "On this device now",
            "Never played", "New list", "Keyboard shortcuts",
        ]
    );
    for (name, expected) in [
        ("Search", Intent::ShowCommands),
        ("Dropbox", Intent::Files(files::Request::Browse(location(6)))),
        ("Add folder or files", Intent::ChooseAudio),
        ("Keyboard shortcuts", Intent::ShowShortcuts),
        ("Never played", Intent::Navigate(catalog::Route::Collection(5))),
    ] {
        let id = ui.a11y_nodes().iter().find(|n| ui.a11y_name_of(n) == name).unwrap().id;
        ui.inject_a11y(&[pxui::A11yAction { id, action_code: 1, ..pxui::A11yAction::default() }]);
        run(&mut ui, &mut intent);
        assert_eq!(intent.take(), Some(expected), "{name}");
    }
    mem_sys::local_reset();
    for _ in 0..20 {
        run(&mut ui, &mut intent);
    }
    assert_eq!(mem_sys::local_alloc_count(), 0, "warm sidebar frames allocate");
}

fn press(ui: &mut Ui, name: &str, nth: usize) {
    let id = ui
        .a11y_nodes()
        .iter()
        .filter(|node| ui.a11y_name_of(node) == name)
        .nth(nth)
        .unwrap_or_else(|| panic!("no control named {name}"))
        .id;
    ui.inject_a11y(&[pxui::A11yAction {
        id,
        action_code: 1,
        ..Default::default()
    }]);
}

fn y_of(ui: &Ui, name: &str, nth: usize) -> f32 {
    ui.a11y_nodes()
        .iter()
        .filter(|node| ui.a11y_name_of(node) == name)
        .nth(nth)
        .unwrap_or_else(|| panic!("no node named {name}"))
        .rect
        .y
}

#[test]
fn empty_palette_lists_swift_commands_over_the_live_page_without_a_worker_round_trip() {
    let catalog = prepared(Vec::new());
    let mut state = ShellState::default();
    state.apply(Intent::ShowCommands);
    assert!(
        state.command_results(&catalog).is_none(),
        "no worker results yet"
    );
    let items = state.command_items(&catalog);
    let rows: Vec<_> = items
        .iter()
        .map(|item| (item.group, item.label.as_str(), item.hint.as_str()))
        .collect();
    // Swift's order and labels; hints only for bound keys (Rescan needs a source, so it is worker-only).
    let swift = [
        ("Play / pause", "Space"),
        ("Shuffle on or off", "S"),
        ("Cycle repeat", "R"),
        ("Add folder or files…", "⌘O"),
        ("Simulate no network on / off", ""),
        ("Switch light / dark", ""),
        ("Minimise or restore the player", "I"),
        ("Show keyboard shortcuts", "?"),
    ];
    assert_eq!(
        rows,
        swift.map(|(label, hint)| ("Commands", label, hint)),
        "{rows:?}"
    );
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let player = PlayerView {
        initialized: true,
        ..Default::default()
    };
    let frame = FrameIn::new(1440.0, 872.0);
    for _ in 0..3 {
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut None)
        })
        .unwrap();
    }
    let ys: Vec<f32> = swift.iter().map(|(label, _)| y_of(&ui, label, 0)).collect();
    assert!(ys.windows(2).all(|pair| pair[0] < pair[1]), "{ys:?}");
    assert!(
        ui.a11y_nodes()
            .iter()
            .any(|node| ui.a11y_name_of(node) == "Current track artwork"),
        "the page stays under the dim"
    );
    let key = |label: &str| items.iter().find(|item| item.label == label).unwrap().key;
    assert_eq!(
        state.activate_command(&catalog, key("Shuffle on or off"), false, false),
        Some(Intent::Shuffle)
    );
    assert_eq!(
        state.activate_command(&catalog, key("Play / pause"), true, false),
        Some(Intent::Pause)
    );
}

#[test]
fn now_panel_draws_title_and_worker_formatted_metadata_without_warm_allocations() {
    mem_sys::set_peer(pxui::library_counter);
    let song = lw_core::Song {
        id: Some(7),
        title: "Field Day".into(),
        artist: "Juno Abara".into(),
        album: "Weather Systems".into(),
        file_extension: Some("flac".into()),
        sample_rate: Some(96_000.0),
        bit_depth: Some(24),
        file_size: Some(108_113_920),
        file_state: lw_core::FileState::Copied,
        relative_path: Some("Weather Systems/Field Day.flac".into()),
        ..Default::default()
    };
    let info = now_info::NowInfo::new(&song, Some("Google Drive"), 16);
    let catalog = prepared(vec![song]);
    let state = ShellState::default();
    let player = PlayerView {
        initialized: true,
        revision: 1,
        current_id: Some(7),
        title: "Field Day",
        artist: "Juno Abara",
        album: "Weather Systems",
        duration: 224.0,
        info: Some(&info),
        ..Default::default()
    };
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let frame = FrameIn::new(1440.0, 872.0);
    for round in 0..10 {
        mem_sys::local_reset();
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 101.0, &catalog, &state, &mut None)
        })
        .unwrap();
        if round > 5 {
            assert_eq!(mem_sys::local_alloc_count(), 0, "warm Now panel frame");
        }
    }
    let panel = ui
        .a11y_nodes()
        .iter()
        .find(|node| ui.a11y_name_of(node) == "Current track artwork")
        .unwrap()
        .rect;
    let below = |name: &str| {
        let node = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == name && node.rect.x >= panel.x)
            .unwrap_or_else(|| panic!("no node named {name} in the panel"));
        node.rect.y
    };
    // Title, then the metadata rows in Swift's order: Format, Size, File, Plays.
    let order = [
        below("Field Day"),
        below("flac · 96 kHz · 24 bit"),
        below("103.1 MB · on device"),
        below("Google Drive / Weather Systems"),
        below("16"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
}

#[test]
fn clip_keeps_char_boundaries_and_oversized_metadata_shows_clipped_everywhere() {
    use crate::catalog::Route;
    use lw_core::{context::PlaybackContext, queue::Entry};
    use std::sync::Arc;
    let limit = crate::DISPLAY_LIMIT;
    assert_eq!(crate::clip("short"), "short");
    assert_eq!(crate::clip(&"a".repeat(limit)).len(), limit);
    // A 2-byte character straddling the limit is dropped whole.
    let straddle = format!("{}é{}", "a".repeat(limit - 1), "b".repeat(50));
    assert_eq!(crate::clip(&straddle).len(), limit - 1);
    assert_eq!(crate::clip_owned("é".repeat(limit)).len(), limit);

    mem_sys::set_peer(pxui::library_counter);
    // Tag values of 100 KiB, multi-byte so a clip lands inside a character.
    let huge = |c: &str| c.repeat(100 * 1024 / c.len());
    let (title, artist, album) = (huge("é"), huge("名"), huge("x"));
    let song = lw_core::Song {
        id: Some(7),
        title: title.clone(),
        artist: artist.clone(),
        album: album.clone(),
        album_artist: huge("y"),
        file_extension: Some("flac".into()),
        relative_path: Some(format!("{}/a.flac", huge("z"))),
        ..Default::default()
    };
    let info = now_info::NowInfo::new(&song, Some(&huge("s")), 1);
    let mut catalog = prepared(vec![song]);
    catalog.add_collection(
        &huge("L"),
        vec![0],
        PlaybackContext::Playlist { id: 3, name: huge("L") },
    );
    catalog.queue = Some(Arc::new(queue::Snapshot::new(
        1,
        Arc::from([Entry { id: 1, song_id: 7 }, Entry { id: 2, song_id: 7 }]),
        Some(0),
        Arc::new(PlaybackContext::Playlist { id: 3, name: huge("L") }),
        false,
    )));
    let row = catalog.row(0);
    for text in [row.title, row.artist, row.album, row.album_artist, row.actions_label, row.subtitle] {
        assert!(!text.is_empty() && text.len() <= limit, "{}", text.len());
    }
    for group in catalog.albums.iter().chain(&catalog.artists).chain(&catalog.collections) {
        assert!(group.title.len() <= limit && group.subtitle.len() <= limit && group.summary.len() <= limit);
    }
    assert!(catalog.queue.as_ref().unwrap().heading.len() <= limit + 16);
    assert!([&info.format, &info.file, &info.folder].iter().all(|text| text.len() <= limit));
    // The playback context keeps the full name: only display strings are clipped.
    assert!(matches!(&*catalog.collections.last().unwrap().context, PlaybackContext::Playlist { name, .. } if name.len() == 100 * 1024));

    let player = PlayerView {
        initialized: true,
        revision: 1,
        current_id: Some(7),
        title: crate::clip(&title),
        artist: crate::clip(&artist),
        album: crate::clip(&album),
        duration: 30.0,
        info: Some(&info),
        ..Default::default()
    };
    let routes = [
        Route::Songs,
        Route::Albums,
        Route::Album(0),
        Route::Artists,
        Route::Artist(0),
        Route::Lists,
        Route::Collection(catalog.collections.len() - 1),
    ];
    for (width, height) in [(1440.0, 872.0), (390.0, 844.0)] {
        for route in routes {
            let mut ui = test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
            ui.set_a11y(true);
            let mut state = ShellState::default();
            state.apply(Intent::Navigate(route));
            state.apply(Intent::ShowQueue);
            let frame = FrameIn::new(width, height);
            for round in 0..8 {
                mem_sys::local_reset();
                crate::testing::frame(&mut ui, &frame, &mut |ui| shell(ui, &player, 101.0, &catalog, &state, &mut None))
                    .unwrap();
                if round > 5 {
                    assert_eq!(mem_sys::local_alloc_count(), 0, "warm frame {route:?} {width}");
                }
            }
            for node in ui.a11y_nodes() {
                let name = ui.a11y_name_of(node);
                assert!(name.len() <= limit + 64, "{route:?} {width}: {} bytes", name.len());
            }
        }
    }
}

#[test]
fn queue_sheet_follows_swift_structure_order_and_actions() {
    use lw_core::{context::PlaybackContext, queue::Entry};
    use std::sync::Arc;
    let mut catalog = prepared(
        (1..=4)
            .map(|id| lw_core::Song {
                id: Some(id),
                title: format!("Song {id}"),
                artist: "Artist".into(),
                ..Default::default()
            })
            .collect(),
    );
    catalog.queue = Some(Arc::new(queue::Snapshot::new(
        1,
        (1..=4)
            .map(|id| Entry {
                id,
                song_id: id as i64,
            })
            .collect(),
        Some(0),
        Arc::new(PlaybackContext::Library),
        false,
    )));
    let mut state = ShellState::default();
    state.apply(Intent::ShowQueue);
    let player = PlayerView {
        initialized: true,
        context: "Library",
        playing: true,
        ..Default::default()
    };
    let frame = FrameIn::new(1440.0, 872.0);
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[FONT, MONO]);
    ui.set_a11y(true);
    let mut intent = None;
    let run = |ui: &mut Ui, intent: &mut Option<Intent>| {
        crate::testing::frame(ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, intent)
        })
        .unwrap();
    };
    for _ in 0..3 {
        run(&mut ui, &mut intent);
    }
    // A full-width bottom sheet (Swift's sheet host has no macOS variant).
    let sheet = ui
        .a11y_nodes()
        .iter()
        .find(|node| ui.a11y_name_of(node) == "Playback queue" && node.rect.w > 1000.0)
        .expect("queue sheet")
        .rect;
    assert_eq!((sheet.x, sheet.w), (0.0, 1440.0));
    assert!((sheet.y + sheet.h - 872.0).abs() < 0.5, "{sheet:?}");
    // Header, Now row (current song), Up next rows (upcoming only), Clear up next.
    let order = [
        y_of(&ui, "Close queue", 0),
        y_of(&ui, "Song 1", 0),
        y_of(&ui, "Song 2", 0),
        y_of(&ui, "Song 3", 0),
        y_of(&ui, "Song 4", 0),
        y_of(&ui, "Clear up next", 0),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
    assert_eq!(
        ui.a11y_nodes()
            .iter()
            .filter(|node| ui.a11y_name_of(node) == "Remove from queue" && node.rect.h == 56.0)
            .count(),
        3,
        "the playing song is not removable"
    );
    assert!(
        ui.a11y_nodes()
            .iter()
            .any(|node| ui.a11y_name_of(node) == "Save queue as playlist")
    );
    for (name, nth, expected) in [
        ("Remove from queue", 0, Intent::RemoveQueue(2)),
        ("Clear up next", 0, Intent::ClearQueue),
        ("Close queue", 0, Intent::CloseQueue),
    ] {
        press(&mut ui, name, nth);
        run(&mut ui, &mut intent);
        assert_eq!(intent.take(), Some(expected), "{name}");
    }
}
