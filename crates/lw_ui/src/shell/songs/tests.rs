use super::*;
use pxui::{FrameIn, Platform};

fn catalog(count: usize) -> Catalog {
    crate::tests::prepared(
        (0..count)
            .map(|index| lw_core::Song {
                id: Some(index as i64),
                track_number: Some(if index == 0 { 9 } else { 12 }),
                title: if index == 0 {
                    "Quiet lights".into()
                } else {
                    "An exceptionally long song title".into()
                },
                artist: if index == 0 {
                    "The National".into()
                } else {
                    "Another artist".into()
                },
                album: if index == 0 {
                    "A midnight album".into()
                } else {
                    "An earlier record".into()
                },
                file_extension: Some(if index == 0 {
                    "FLAC".into()
                } else {
                    "mp3".into()
                }),
                duration: Some(if index == 0 { 240.0 } else { 61.0 }),
                ..Default::default()
            })
            .collect(),
    )
}

#[test]
fn song_table_matches_native_columns_baselines_and_fitted_durations_at_both_densities() {
    mem_sys::set_peer(pxui::library_counter);
    let catalog = catalog(2);
    let state = ShellState::default();
    let player = PlayerView::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    for round in 0..6 {
        mem_sys::local_reset();
        for scale in [1.0, 2.0] {
            for width in [399.0, 400.0, 519.0, 520.0, 679.0, 680.0, 888.0] {
                ui.frame(
                    &FrameIn {
                        width: width + 552.0,
                        height: 872.0,
                        scale,
                        reduced_motion: 1,
                        ..Default::default()
                    },
                    &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut None),
                )
                .unwrap();
                let row = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| {
                        node.is_role(Role::ListItem) && ui.a11y_name_of(node) == "Quiet lights"
                    })
                    .unwrap();
                assert!(
                    (row.rect.y
                        - if matches!(width, 400.0 | 520.0) {
                            159.7
                        } else {
                            142.9
                        })
                    .abs()
                        < 0.001
                        && (row.rect.w - width).abs() < 0.001,
                    "full-width song row: {:?}",
                    row.rect
                );
                let wrapped = matches!(width, 400.0 | 520.0);
                for (name, y, h) in [
                    ("1", if wrapped { 2.2 } else { 10.2 }, 18.2),
                    ("Quiet lights", if wrapped { 1.2 } else { 9.2 }, 19.6),
                    (
                        "4:00",
                        if wrapped { 3.2 } else { 11.2 },
                        if wrapped { 33.6 } else { 16.8 },
                    ),
                ] {
                    let label = ui
                        .a11y_nodes()
                        .iter()
                        .find(|node| node.is_role(Role::Label) && ui.a11y_name_of(node) == name)
                        .unwrap();
                    assert!(
                        (label.rect.y - row.rect.y - y).abs() < 0.001
                            && (label.rect.h - h).abs() < 0.001,
                        "native baseline box {name}: {:?}",
                        label.rect
                    );
                }
                let time = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| node.is_role(Role::Label) && ui.a11y_name_of(node) == "4:00")
                    .unwrap();
                let native_width = if width == 400.0 {
                    if scale == 1.0 { 15.0 } else { 14.5 }
                } else if matches!(width, 520.0 | 364.0 | 364.1) {
                    22.0
                } else {
                    29.0
                };
                assert!(
                    (time.rect.w - native_width).abs() < 0.001
                        && (time.rect.x + time.rect.w - (232.0 + width - 60.0)).abs() < 0.001,
                    "native trailing Text rounds its intrinsic width: {:?}",
                    time.rect
                );
                for (name, visible) in [
                    ("The National", width >= 400.0),
                    ("A midnight album", width >= 520.0),
                    ("flac", width >= 680.0),
                ] {
                    assert_eq!(
                        ui.a11y_nodes()
                            .iter()
                            .any(|node| node.is_role(Role::Label) && ui.a11y_name_of(node) == name),
                        visible
                    );
                }
                let action = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| {
                        node.is_role(Role::Button)
                            && ui.a11y_name_of(node) == "Actions for Quiet lights"
                    })
                    .unwrap();
                assert!(
                    (action.rect.x - (232.0 + width - 52.0)).abs() < 0.001 && action.rect.w == 32.0
                );
                if width == 888.0 {
                    let title = ui
                        .a11y_nodes()
                        .iter()
                        .find(|node| {
                            node.is_role(Role::Button) && ui.a11y_name_of(node) == "Sort by Title"
                        })
                        .unwrap();
                    assert!(
                        (title.rect.x - 289.6).abs() < 0.01 && (title.rect.y - 115.6).abs() < 0.001
                    );
                    assert!((time.rect.x - 1031.0).abs() < 0.001);
                }
            }
        }
        if round >= 3 {
            assert_eq!(
                mem_sys::local_alloc_count(),
                0,
                "warm responsive song table and 1x/2x fitting"
            );
        }
    }
}

#[test]
fn trailing_time_heading_matches_native_rounding_and_fractional_overflow() {
    let catalog = catalog(2);
    let table = catalog.all.as_ref().unwrap().song_table.as_ref().unwrap();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    for scale in [1.0, 2.0] {
        for width in [
            399.0, 400.0, 519.0, 520.0, 679.0, 680.0, 888.0, 364.0, 364.1, 364.26, 364.3, 364.5,
        ] {
            let columns = table.columns(&catalog, width, None, Sort::Original, Route::Songs);
            let mut root = None;
            ui.frame(
                &FrameIn {
                    width,
                    height: 100.0,
                    scale,
                    reduced_motion: 1,
                    ..Default::default()
                },
                &mut |ui| {
                    let component = ui.node(
                        "component",
                        Node {
                            w: Size::Px(width),
                            h: Size::Grow,
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&component, &mut |ui| {
                        let layout = heading_layout(ui, &catalog, columns, None);
                        let p = palette(ui);
                        root = Some(heading(ui, columns, None, layout, p, &mut None));
                    });
                },
            )
            .unwrap();
            let button = ui
                .box_of_key(Ui::key_of(root.unwrap().box_key, "Time"))
                .unwrap();
            let text = ui
                .box_of_key(Ui::key_of(button.box_key, "time-label"))
                .unwrap();
            let rect = ui.rect(&text);
            let native = if matches!(width, 400.0 | 520.0) {
                if scale == 1.0 { 21.0 } else { 20.5 }
            } else if scale == 1.0 {
                28.0
            } else {
                27.5
            };
            assert!(
                (rect.x - (width - 60.0 - native)).abs() < 0.001
                    && (rect.w - native).abs() < 0.001
                    && (rect.y - 10.0).abs() < 0.001,
                "native Time heading {width}/{scale}: {rect:?}"
            );
        }
    }
}

#[test]
fn playback_changes_rebuild_only_affected_song_rows_and_keep_copy_cached() {
    mem_sys::set_peer(pxui::library_counter);
    let catalog = crate::tests::prepared(
        (0..8)
            .map(|id| lw_core::Song {
                id: Some(id),
                title: "Track".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                duration: Some(240.0),
                ..Default::default()
            })
            .collect(),
    );
    let state = ShellState::default();
    let mut player = PlayerView {
        initialized: true,
        title: "Track",
        duration: 240.0,
        ..Default::default()
    };
    let mut ui = pxui::Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut frame = FrameIn {
        width: 1440.0,
        height: 872.0,
        ..Default::default()
    };
    let mut previous = Some(1);
    let mut observed = 0;
    for tick in 0..80 {
        let (id, playing) = [(0, false), (0, true), (1, true), (1, false)][tick % 4];
        player.current_id = Some(id);
        player.playing = playing;
        player.revision = tick as u64;
        frame.time = tick as f64;
        mem_sys::local_reset();
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut None)
        })
        .unwrap();
        let _ = ui.output();
        observed += ui.rebuilt_named("song-visual");
        if tick >= 40 {
            assert_eq!(mem_sys::local_alloc_count(), 0, "playback update {tick}");
            let changed_song = previous != player.current_id;
            assert_eq!(
                ui.rebuilt_named("song-visual"),
                if changed_song { 2 } else { 1 },
                "only old/new current rows, tick {tick}"
            );
            assert_eq!(
                ui.rebuilt_named("song-copy"),
                if changed_song { 2 } else { 0 },
                "play/pause keeps text, tick {tick}"
            );
            let indicator = ui
                .a11y_nodes()
                .iter()
                .find(|node| {
                    node.is_role(Role::Image)
                        && ui.a11y_name_of(node) == if playing { "Playing" } else { "Paused" }
                })
                .unwrap();
            let row = ui
                .a11y_nodes()
                .iter()
                .filter(|node| node.is_role(Role::ListItem) && ui.a11y_name_of(node) == "Track")
                .nth(id as usize)
                .unwrap();
            assert!(
                indicator.rect.y >= row.rect.y
                    && indicator.rect.y + indicator.rect.h <= row.rect.y + row.rect.h
            );
            assert_eq!(
                ui.a11y_nodes()
                    .iter()
                    .filter(|node| node.is_role(Role::Image)
                        && matches!(ui.a11y_name_of(node), "Playing" | "Paused"))
                    .count(),
                1
            );
        }
        previous = player.current_id;
    }
    assert!(
        observed > 0,
        "the rebuild counter must observe actual row builds"
    );
}

#[test]
fn retained_bars_match_native_phases_and_pending_copy_pulses_without_warm_builds() {
    mem_sys::set_peer(pxui::library_counter);
    let catalog = catalog(1);
    for scale in [1.0, 2.0] {
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        // Counts rebuilt views, and a verify pass rebuilds every kept view.
        ui.verify_views(false);
        let mut frame = FrameIn {
            width: 100.0,
            height: 100.0,
            scale,
            ..Default::default()
        };
        let mut song = catalog.row(0);
        song.state = lw_core::FileState::CopyPending;
        let mut build = |ui: &mut Ui| {
            let p = palette(ui);
            ui.view("indicator", 0, Size::Px(14.0), Size::Px(11.0), &mut |ui| {
                row::equalizer(ui, true, true, song.id, p)
            });
            ui.view("file-state", 0, Size::Px(10.0), Size::Px(10.0), &mut |ui| {
                row::availability(ui, song, true, p)
            });
        };
        ui.frame(&frame, &mut build).unwrap();
        ui.frame(&frame, &mut build).unwrap();
        let indicator = Ui::key_of(pxui::ui::ROOT_KEY, "indicator");
        let equalizer = Ui::key_of(indicator, "equalizer");
        let bars = ["bar-0", "bar-1", "bar-2"].map(|key| {
            let position = Ui::key_of(equalizer, key);
            let visual = Ui::key_of(position, "bar");
            ui.box_of_key(Ui::key_of(visual, "fill")).unwrap()
        });
        let instances: Vec<_> = ui
            .layers()
            .flat_map(|layer| layer.instances.iter().copied())
            .collect();
        for tick in 1..61 {
            frame.time = tick as f64 / 60.0;
            mem_sys::local_reset();
            ui.frame(&frame, &mut build).unwrap();
            for (index, bar) in bars.iter().enumerate() {
                let phase = (frame.time / 0.9 + index as f64 / 3.0).fract() as f32;
                let progress = if phase < 0.5 {
                    phase * 2.0
                } else {
                    (1.0 - phase) * 2.0
                };
                let curve = Curve::Bezier {
                    duration: 1.0,
                    points: [0.42, 0.0, 0.58, 1.0],
                };
                let rect = ui.rect(bar);
                assert!((rect.h - 11.0 * (0.4 + 0.6 * curve.at(progress))).abs() < 0.001);
                assert!(
                    (rect.x - index as f32 * 4.0).abs() < 0.001 && (rect.w - 2.0).abs() < 0.001
                );
                assert!((rect.y + rect.h - 11.0).abs() < 0.001);
            }
            assert_eq!(ui.output().rebuilt_views, 0);
            assert!(
                ui.layers()
                    .flat_map(|layer| layer.instances.iter().copied())
                    .eq(instances.iter().copied())
            );
            assert_eq!(mem_sys::local_alloc_count(), 0);
        }
        // Exercise the pending-state loop on its own, including removal and restart.
        let mut state_revision = 0;
        let mut last_draw = 0;
        for (time, pending, reduced, changed) in [
            (2.0, false, 0, true),
            (3.0, true, 0, true),
            (3.25, true, 0, false),
            (3.5, true, 0, true),
            (3.75, true, 0, false),
            (4.0, true, 0, true),
            (4.25, true, 1, false),
            (5.0, true, 1, false),
        ] {
            if !pending || time == 3.0 {
                state_revision += 1;
            }
            song.state = if pending {
                lw_core::FileState::CopyPending
            } else {
                lw_core::FileState::BookmarkOnly
            };
            frame.time = time;
            frame.reduced_motion = reduced;
            ui.frame(&frame, &mut |ui| {
                let p = palette(ui);
                ui.view(
                    "file-state",
                    state_revision,
                    Size::Px(10.0),
                    Size::Px(10.0),
                    &mut |ui| row::availability(ui, song, true, p),
                );
            })
            .unwrap();
            assert_eq!(
                ui.draw_revision() != last_draw,
                changed,
                "pending pulse at {time}"
            );
            last_draw = ui.draw_revision();
            if reduced != 0 {
                assert!(ui.output().repaint_at.is_infinite());
            }
        }
    }
}

#[test]
fn playing_song_bars_animate_cached_text_scroll_and_stop_without_warm_allocations() {
    mem_sys::set_peer(pxui::library_counter);
    let catalog = catalog(100);
    let state = ShellState::default();
    let mut player = PlayerView {
        current_id: Some(0),
        playing: true,
        ..Default::default()
    };
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut frame = FrameIn {
        width: 1440.0,
        height: 872.0,
        ..Default::default()
    };
    for time in [0.0, 1.0, 2.0] {
        frame.time = time;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut None)
        })
        .unwrap();
    }
    let list = ui
        .a11y_nodes()
        .iter()
        .find(|node| node.is_role(Role::List) && node.rect.x >= 232.0)
        .unwrap()
        .id;
    ui.inject_a11y(&[pxui::A11yAction {
        id: list,
        action_code: 8,
        payload: 0.0,
        ..Default::default()
    }]);
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut None)
    })
    .unwrap();
    // Settle the viewport once; animation frames must keep its content build count unchanged.
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut None)
    })
    .unwrap();
    let viewport_rebuilds = ui.output().rebuilt_views;
    let glyphs: Vec<_> = ui
        .layers()
        .flat_map(|layer| layer.instances.iter())
        .filter(|i| i.kind == pxui::draw::kind::MASK)
        .copied()
        .collect();
    let mut previous = ui.draw_revision();
    let mut changes = 0;
    for tick in 1..121 {
        frame.time = 2.0 + tick as f64 / 60.0;
        mem_sys::local_reset();
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut None)
        })
        .unwrap();
        let revision = ui.draw_revision();
        changes += usize::from(revision != previous);
        previous = revision;
        assert!(
            ui.layers()
                .flat_map(|layer| layer.instances.iter())
                .filter(|i| i.kind == pxui::draw::kind::MASK)
                .copied()
                .eq(glyphs.iter().copied()),
            "animation preserves all cached glyph instances"
        );
        assert!(
            ui.output().rebuilt_views == viewport_rebuilds,
            "animation must keep the shell content build count constant: {}",
            ui.output().rebuilt_views
        );
        assert!(ui.output().repaint_at.is_finite());
        if tick >= 10 {
            assert_eq!(mem_sys::local_alloc_count(), 0, "warm playing indicator");
        }
    }
    assert!(changes > 60);
    player.playing = false;
    frame.time += 1.0;
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut None)
    })
    .unwrap();
    assert!(
        ui.output().repaint_at.is_infinite(),
        "paused indicator stops requesting frames"
    );
    player.playing = true;
    frame.time += 1.0;
    frame.reduced_motion = 1;
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut None)
    })
    .unwrap();
    assert!(
        ui.output().repaint_at.is_infinite(),
        "reduced motion stops the indicator"
    );
    frame.reduced_motion = 0;
    frame.time += 1.0;
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut None)
    })
    .unwrap();
    let list = ui
        .a11y_nodes()
        .iter()
        .find(|node| node.is_role(Role::List) && node.rect.x >= 232.0)
        .unwrap()
        .id;
    for delta in [130.0, -130.0] {
        ui.inject_a11y(&[pxui::A11yAction {
            id: list,
            action_code: 8,
            payload: delta,
            ..Default::default()
        }]);
        frame.time += 1.0;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut None)
        })
        .unwrap();
        let heading = ui
            .a11y_nodes()
            .iter()
            .find(|node| {
                node.is_role(Role::Label)
                    && node.rect.x >= 232.0
                    && ui.a11y_name_of(node) == "Songs"
            })
            .unwrap();
        let column = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == "Sort by Title")
            .unwrap();
        assert!((heading.rect.y - if delta > 0.0 { -102.0 } else { 28.0 }).abs() < 0.001);
        assert!((column.rect.y - if delta > 0.0 { 10.0 } else { 115.6 }).abs() < 0.001);
        assert_eq!(
            ui.output().repaint_at.is_finite(),
            delta < 0.0,
            "the retained indicator restarts after leaving the opaque heading"
        );
    }
    ui.inject_a11y(&[pxui::A11yAction {
        id: list,
        action_code: 8,
        payload: 650.0,
        ..Default::default()
    }]);
    frame.time += 1.0;
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut None)
    })
    .unwrap();
    frame.time += 1.0;
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut None)
    })
    .unwrap();
    assert!(
        ui.output().repaint_at.is_infinite(),
        "an offscreen song cannot keep its animation timer awake"
    );
}

#[test]
fn song_actions_route_play_and_append_reject_stale_rows_and_reopen_without_allocations() {
    mem_sys::set_peer(pxui::library_counter);
    let catalog = catalog(2);
    let mut state = ShellState::default();
    let player = PlayerView::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut frame = FrameIn {
        width: 1440.0,
        height: 872.0,
        reduced_motion: 1,
        ..Default::default()
    };
    let mut intent = None;
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalog, &state, &mut intent)
    })
    .unwrap();
    for cycle in 0..8 {
        mem_sys::local_reset();
        for (label, expected) in [
            ("Play", Intent::PlayRow(0)),
            ("Add to queue", Intent::AppendSong(0)),
            ("Favorite", Intent::SetFavorite { song: 0, on: true }),
        ] {
            let actions = ui
                .a11y_nodes()
                .iter()
                .find(|node| {
                    node.is_role(Role::Button)
                        && ui.a11y_name_of(node) == "Actions for Quiet lights"
                })
                .unwrap()
                .id;
            ui.inject_a11y(&[pxui::A11yAction {
                id: actions,
                action_code: 1,
                ..Default::default()
            }]);
            frame.time += 0.01;
            intent = None;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, &state, &mut intent)
            })
            .unwrap();
            assert!(matches!(intent, Some(Intent::SongMenu(0, 0, _, _))));
            state.apply(intent.take().unwrap());
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, &state, &mut intent)
            })
            .unwrap();
            let item = ui
                .a11y_nodes()
                .iter()
                .find(|node| node.is_role(Role::MenuItem) && ui.a11y_name_of(node) == label)
                .unwrap()
                .id;
            ui.inject_a11y(&[pxui::A11yAction {
                id: item,
                action_code: 1,
                ..Default::default()
            }]);
            frame.time += 0.01;
            intent = None;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, &state, &mut intent)
            })
            .unwrap();
            assert_eq!(intent, Some(expected));
            state.apply(Intent::CloseSongMenu);
            intent = None;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, &state, &mut intent)
            })
            .unwrap();
        }
        if cycle >= 3 {
            assert_eq!(mem_sys::local_alloc_count(), 0, "warm row action menus");
        }
    }
    let key = catalog.all.as_ref().unwrap().key;
    let mut changed = catalog.clone();
    changed.project(
        crate::browse::Request {
            commands: None,
            scope: Scope::Collection(key),
            query: Query::default(),
            sort: Sort::Title,
            ascending: true,
        },
        vec![1, 0],
    );
    state.apply(Intent::SongMenu(0, 0, key, (500.0, 200.0)));
    intent = None;
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &changed, &state, &mut intent)
    })
    .unwrap();
    let item = ui
        .a11y_nodes()
        .iter()
        .find(|node| node.is_role(Role::MenuItem) && ui.a11y_name_of(node) == "Add to queue")
        .unwrap();
    assert_eq!(item.states & pxui::a11y::state::ENABLED, 0);
    ui.inject_a11y(&[pxui::A11yAction {
        id: item.id,
        action_code: 1,
        ..Default::default()
    }]);
    frame.time += 0.01;
    intent = None;
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &changed, &state, &mut intent)
    })
    .unwrap();
    assert_eq!(intent, None, "stale menu cannot append a different row");
    changed.prepare_remap(&catalog);
    state.refresh(&catalog, &changed);
    assert!(state.song_menu.is_none());
}
#[test]
fn availability_state_pop_matches_native_scale_and_preserves_song_identity_without_warm_allocations()
 {
    use lw_core::{FileState, Song};
    mem_sys::set_peer(pxui::library_counter);
    let catalogs = [FileState::BookmarkOnly, FileState::Copied].map(|state| {
        super::super::super::tests::prepared(vec![
            Song {
                id: Some(1),
                title: "First".into(),
                file_state: state,
                ..Default::default()
            },
            Song {
                id: Some(2),
                title: "Second".into(),
                file_state: FileState::Failed,
                ..Default::default()
            },
        ])
    });
    let mut catalogs = catalogs;
    let previous = catalogs[0].clone();
    catalogs[1].prepare_remap(&previous);
    let previous = catalogs[1].clone();
    catalogs[0].prepare_remap(&previous);
    let mut state = ShellState::default();
    let player = PlayerView::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut frame = FrameIn {
        width: 1440.0,
        height: 872.0,
        ..Default::default()
    };
    let glyph = |ui: &Ui| {
        ui.a11y_nodes()
            .iter()
            .find(|node| {
                node.is_role(Role::Image) && ui.a11y_name_of(node) == "Checking availability"
            })
            .unwrap()
            .rect
    };
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalogs[0], &state, &mut None)
    })
    .unwrap();
    assert_eq!(glyph(&ui).w, 10.0, "first appearance does not pop");
    for cycle in 0..8 {
        mem_sys::local_reset();
        for next in [1, 0] {
            state.refresh(&catalogs[1 - next], &catalogs[next]);
            frame.time += 1.0;
            let start = frame.time;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalogs[next], &state, &mut None)
            })
            .unwrap();
            for progress in [0.0, 0.044, 0.205, 0.245, 0.472, 1.0] {
                frame.time = start + f64::from(crate::motion::BASE * progress);
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, 0.0, &catalogs[next], &state, &mut None)
                })
                .unwrap();
                let expected = 10.0
                    * (0.3
                        + 0.7
                            * crate::motion::spring(crate::motion::BASE)
                                .at(crate::motion::BASE * progress));
                assert!(
                    (glyph(&ui).w - expected).abs() < 0.002,
                    "native state pop at {progress}: {:?}, expected {expected}",
                    glyph(&ui)
                );
            }
        }
        if cycle >= 3 {
            assert_eq!(
                mem_sys::local_alloc_count(),
                0,
                "warm availability state transitions"
            );
        }
    }
    state.refresh(&catalogs[0], &catalogs[1]);
    frame.reduced_motion = 1;
    frame.time += 1.0;
    ui.frame(&frame, &mut |ui| {
        shell(ui, &player, 0.0, &catalogs[1], &state, &mut None)
    })
    .unwrap();
    assert_eq!(
        glyph(&ui).w,
        10.0,
        "reduced motion keeps the natural glyph size"
    );
    assert!(ui.output().repaint_at.is_infinite());
}

#[test]
fn prepared_song_cells_match_normal_wrapping_density_current_weight_and_album_numbers() {
    let catalog = catalog(2);
    for scale in [1.0, 1.5, 2.0] {
        for width in [400.0, 888.0] {
            for (route, current) in [
                (Route::Songs, None),
                (Route::Songs, Some(0)),
                (Route::Album(0), None),
            ] {
                let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
                let mut reference =
                    crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
                let input = FrameIn {
                    width: 1200.0,
                    height: 300.0,
                    scale,
                    reduced_motion: 1,
                    ..Default::default()
                };
                ui.frame(&input, &mut |_| {}).unwrap();
                reference.frame(&input, &mut |_| {}).unwrap();
                let collection = catalog.collection(route).unwrap();
                let widths = collection.song_table.as_ref().unwrap().columns(
                    &catalog,
                    width,
                    current,
                    Sort::Original,
                    route,
                );
                let window = crate::song_text::Window::new(
                    &catalog,
                    route,
                    widths,
                    ui.density() as f32,
                    current.map(|index| catalog.row(index).id),
                    0,
                    collection.rows.len(),
                )
                .unwrap();
                let mut text = ui.text_service().fork().unwrap();
                let source = catalog.clone();
                let records = std::thread::spawn(move || {
                    window
                        .rows()
                        .flat_map(|row| {
                            window
                                .prepare_row(&source, row, &mut text)
                                .unwrap()
                                .into_iter()
                                .flatten()
                        })
                        .collect::<Vec<_>>()
                })
                .join()
                .unwrap();
                let mut retired = Vec::with_capacity(records.len());
                for (_, record) in records {
                    match ui.text_service().install(record) {
                        Ok(record) => retired.push(record),
                        Err((code, _)) => panic!("prepared song cell refused ({code})"),
                    }
                }
                let layouts = ui.text_layout_count();
                let mut build = |ui: &mut Ui| {
                    for (position, &index) in collection.rows.iter().enumerate() {
                        let row = catalog.row(index);
                        let current = current == Some(index);
                        let numbered = matches!(route, Route::Album(_));
                        let layout = row_layout(
                            ui,
                            &catalog,
                            index,
                            position + 1,
                            numbered,
                            widths,
                            current,
                        );
                        let root = ui.node_keyed(
                            int_key(91, position as i64),
                            Node {
                                w: Size::Px(width),
                                h: Size::Px(layout.height),
                                ..Node::DEFAULT
                            },
                        );
                        ui.within(&root, &mut |ui| {
                            let mut digits = [0u8; 24];
                            let number = stack_fmt(&mut digits, format_args!("{}", position + 1));
                            for column in Column::ALL {
                                if widths[column as usize] <= 0.0
                                    || matches!(column, Column::Actions | Column::State)
                                    || (column == Column::Number && current)
                                {
                                    continue;
                                }
                                let (w, h) = layout.sizes[column as usize];
                                label(
                                    ui,
                                    column.title(),
                                    body_value(
                                        &row,
                                        column,
                                        numbered,
                                        number,
                                    ),
                                    Rect {
                                        x: column as usize as f32 * 100.0,
                                        y: layout.y(baseline(column)),
                                        w,
                                        h,
                                    },
                                    body_style(column, current),
                                    Rgba {
                                        r: 17,
                                        g: 33,
                                        b: 55,
                                        a: 255,
                                    },
                                    Role::Label,
                                );
                            }
                        });
                    }
                };
                ui.frame(&input, &mut build).unwrap();
                assert_eq!(
                    ui.text_layout_count(),
                    layouts,
                    "prepared widths/styles must serve both measurement and node painting: {route:?} {width} {scale}x"
                );
                reference.frame(&input, &mut build).unwrap();
                let actual = ui
                    .layers()
                    .flat_map(|layer| layer.instances.iter().copied())
                    .collect::<Vec<_>>();
                let expected = reference
                    .layers()
                    .flat_map(|layer| layer.instances.iter().copied())
                    .collect::<Vec<_>>();
                assert!(!actual.is_empty());
                assert_eq!(actual, expected);
                std::thread::spawn(move || drop(retired)).join().unwrap();
            }
        }
    }
    let window =
        crate::song_text::Window::new(&catalog, Route::Songs, [100.0; 8], 1.0, None, 0, usize::MAX)
            .unwrap();
    assert!(window.rows().len() <= crate::song_text::MAX_ROWS);
    assert!(window.matches(&catalog.clone(), Route::Songs));
    assert!(!window.matches(&catalog, Route::Find));
    assert!(!window.matches(&self::catalog(2), Route::Songs));
}

#[test]
fn track_menu_follows_swift_order_enablement_submenu_and_right_click_without_allocations() {
    use lw_core::context::PlaybackContext;
    mem_sys::set_peer(pxui::library_counter);
    let mut catalog = crate::tests::prepared(vec![
        lw_core::Song {
            id: Some(1),
            title: "Quiet lights".into(),
            artist: "The National".into(),
            album: "A midnight album".into(),
            duration: Some(240.0),
            ..Default::default()
        },
        lw_core::Song {
            id: Some(2),
            title: "Second".into(),
            source_id: Some(5),
            relative_path: Some("second.flac".into()),
            file_state: lw_core::FileState::Failed,
            ..Default::default()
        },
    ]);
    for (id, name, rows) in [(7, "Morning", vec![0]), (8, "Night", vec![1])] {
        catalog.add_collection(
            name,
            rows,
            PlaybackContext::Playlist {
                id,
                name: name.into(),
            },
        );
    }
    let all = catalog.all.as_ref().unwrap().key;
    let slot = catalog
        .collections
        .iter()
        .position(|group| matches!(&*group.context, PlaybackContext::Playlist { id: 7, .. }))
        .unwrap();
    let morning = catalog.collections[slot].key;
    let player = PlayerView::default();
    let mut state = ShellState::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut frame = FrameIn {
        width: 1440.0,
        height: 872.0,
        reduced_motion: 1,
        ..Default::default()
    };
    let mut intent = None;
    let run =
        |ui: &mut Ui, state: &ShellState, frame: &mut FrameIn, intent: &mut Option<Intent>| {
            frame.time += 0.01;
            *intent = None;
            ui.frame(frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, state, intent)
            })
            .unwrap();
        };
    let items = |ui: &Ui| -> Vec<(String, bool)> {
        ui.a11y_nodes()
            .iter()
            .filter(|node| node.is_role(Role::MenuItem))
            .map(|node| {
                (
                    ui.a11y_name_of(node).to_string(),
                    node.states & pxui::a11y::state::ENABLED != 0,
                )
            })
            .collect()
    };
    let activate = |ui: &mut Ui, name: &str| {
        let id = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::MenuItem) && ui.a11y_name_of(node) == name)
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id,
            action_code: 1,
            ..Default::default()
        }]);
    };
    // Plain library row without a source: no Retry, no Remove, Show in Finder disabled.
    run(&mut ui, &state, &mut frame, &mut intent);
    state.apply(Intent::SongMenu(0, 1, all, (500.0, 200.0)));
    run(&mut ui, &state, &mut frame, &mut intent);
    let expected = [
        ("Play", true),
        ("Play next", true),
        ("Add to queue", true),
        ("Add to list", true),
        ("Go to album", true),
        ("Go to artist", true),
        ("Show in Finder", false),
        ("Favorite", true),
        ("Keep offline", true),
        ("Share…", false),
        ("Edit info…", true),
        ("Delete…", true),
    ];
    let names = |items: &[(String, bool)]| {
        items
            .iter()
            .map(|(name, on)| (name.clone(), *on))
            .collect::<Vec<_>>()
    };
    let got = names(&items(&ui));
    let want: Vec<_> = expected
        .iter()
        .map(|(name, on)| (name.to_string(), *on && !cfg!(target_os = "ios")))
        .collect();
    assert_eq!(
        got.iter().map(|item| item.0.as_str()).collect::<Vec<_>>(),
        expected.iter().map(|item| item.0).collect::<Vec<_>>()
    );
    assert_eq!(got.iter().map(|item| item.1).collect::<Vec<_>>(), {
        let mut on = want.iter().map(|item| item.1).collect::<Vec<_>>();
        on[6] = false;
        on
    });
    let routes = catalog.byline_routes(1).unwrap();
    for (label, want) in [
        ("Play", Intent::PlayRow(0)),
        ("Play next", Intent::PlayNext(1)),
        ("Add to queue", Intent::AppendSong(1)),
        ("Go to album", Intent::Navigate(routes[1])),
        ("Go to artist", Intent::Navigate(routes[0])),
        ("Keep offline", Intent::KeepOffline(1)),
    ] {
        activate(&mut ui, label);
        run(&mut ui, &state, &mut frame, &mut intent);
        assert_eq!(intent, Some(want), "{label}");
        state.apply(Intent::SongMenu(0, 1, all, (500.0, 200.0)));
        run(&mut ui, &state, &mut frame, &mut intent);
    }
    // Submenu: playlists from the catalog, then the add intent.
    activate(&mut ui, "Add to list");
    run(&mut ui, &state, &mut frame, &mut intent);
    assert_eq!(intent, Some(Intent::SongMenuLists));
    state.apply(Intent::SongMenuLists);
    run(&mut ui, &state, &mut frame, &mut intent);
    assert_eq!(
        items(&ui)
            .iter()
            .map(|item| item.0.as_str())
            .collect::<Vec<_>>(),
        ["New list…", "Morning", "Night"]
    );
    activate(&mut ui, "Night");
    run(&mut ui, &state, &mut frame, &mut intent);
    assert_eq!(intent, Some(Intent::AddToList { song: 1, list: 8 }));
    state.apply(Intent::CloseSongMenu);
    // A failed, sourced row offers Retry and an enabled Show in Finder.
    state.apply(Intent::SongMenu(1, 2, all, (500.0, 200.0)));
    run(&mut ui, &state, &mut frame, &mut intent);
    let failed = items(&ui);
    assert!(failed.iter().any(|item| item.0 == "Retry download"));
    assert!(
        failed
            .iter()
            .any(|item| item.0 == "Show in Finder" && item.1)
            || cfg!(target_os = "ios")
    );
    assert!(!failed.iter().any(|item| item.0 == "Remove from list"));
    activate(&mut ui, "Retry download");
    run(&mut ui, &state, &mut frame, &mut intent);
    assert_eq!(intent, Some(Intent::RetryDownload(2)));
    state.apply(Intent::SongMenu(1, 2, all, (500.0, 200.0)));
    run(&mut ui, &state, &mut frame, &mut intent);
    let order: Vec<_> = items(&ui).into_iter().map(|item| item.0).collect();
    let (retry, share) = (
        order.iter().position(|name| name == "Retry download"),
        order.iter().position(|name| name == "Share…"),
    );
    assert_eq!(share, retry.map(|index| index + 1), "Share follows Retry");
    assert!(items(&ui).iter().any(|item| item.0 == "Share…" && item.1));
    activate(&mut ui, "Share…");
    run(&mut ui, &state, &mut frame, &mut intent);
    assert_eq!(intent, Some(Intent::ShareSong(2, (500.0, 200.0))));
    // A share in flight disables the item, as Swift's `sharing` flag does.
    frame.time += 0.01;
    ui.frame(&frame, &mut |ui| {
        let busy = PlayerView {
            sharing: true,
            ..Default::default()
        };
        shell(ui, &busy, 0.0, &catalog, &state, &mut None)
    })
    .unwrap();
    assert!(items(&ui).iter().any(|item| item.0 == "Share…" && !item.1));
    state.apply(Intent::CloseSongMenu);
    // Inside a playlist: Remove from list, and that list is not offered as a target.
    state.apply(Intent::SongMenu(0, 1, morning, (500.0, 200.0)));
    run(&mut ui, &state, &mut frame, &mut intent);
    assert_eq!(state.song_menu.map(|menu| menu.2), Some(morning));
    state.apply(Intent::CloseSongMenu);
    state.apply(Intent::Navigate(Route::Collection(slot)));
    state.apply(Intent::SongMenu(0, 1, morning, (500.0, 200.0)));
    run(&mut ui, &state, &mut frame, &mut intent);
    assert_eq!(items(&ui).last().unwrap().0, "Remove from list");
    activate(&mut ui, "Remove from list");
    run(&mut ui, &state, &mut frame, &mut intent);
    assert_eq!(intent, Some(Intent::RemoveFromList { song: 1, list: 7 }));
    state.apply(Intent::SongMenu(0, 1, morning, (500.0, 200.0)));
    state.apply(Intent::SongMenuLists);
    run(&mut ui, &state, &mut frame, &mut intent);
    assert_eq!(
        items(&ui)
            .iter()
            .map(|item| item.0.as_str())
            .collect::<Vec<_>>(),
        ["New list…", "Night"]
    );
    state.apply(Intent::CloseSongMenu);
    // Secondary click on a row opens the menu at the pointer.
    run(&mut ui, &state, &mut frame, &mut intent);
    let row = ui
        .a11y_nodes()
        .iter()
        .find(|node| {
            node.is_role(Role::Button) && ui.a11y_name_of(node) == "Actions for Quiet lights"
        })
        .unwrap()
        .rect;
    let (x, y) = (f64::from(row.x - 200.0), f64::from(row.y + 16.0));
    let press = [
        pxui::Event {
            kind: input::wire::POINTER_MOVED,
            x,
            y,
            ..Default::default()
        },
        pxui::Event {
            kind: input::wire::POINTER_PRESSED,
            code: 1,
            x,
            y,
            ..Default::default()
        },
        pxui::Event {
            kind: input::wire::POINTER_RELEASED,
            code: 1,
            x,
            y,
            ..Default::default()
        },
    ];
    frame.time += 0.01;
    intent = None;
    ui.frame(
        &FrameIn {
            events: press.as_ptr(),
            event_count: press.len(),
            ..frame
        },
        &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut intent),
    )
    .unwrap();
    assert!(
        matches!(intent, Some(Intent::SongMenu(_, 1, _, _))),
        "{intent:?}"
    );
    // Warm menu frames, including the playlist panel, allocate nothing.
    state.apply(intent.take().unwrap());
    for cycle in 0..8 {
        mem_sys::local_reset();
        state.apply(Intent::SongMenuLists);
        run(&mut ui, &state, &mut frame, &mut intent);
        state.apply(Intent::SongMenu(0, 1, all, (500.0, 200.0)));
        run(&mut ui, &state, &mut frame, &mut intent);
        if cycle >= 3 {
            assert_eq!(mem_sys::local_alloc_count(), 0, "warm track menu");
        }
    }
}

#[test]
fn list_management_dialogs_menu_confirmation_reorder_and_scrolling_panel() {
    use lw_core::context::PlaybackContext;
    mem_sys::set_peer(pxui::library_counter);
    let mut catalog = titled(4);
    let list = |id: i64| PlaybackContext::Playlist {
        id,
        name: String::new(),
    };
    catalog.add_collection("Mix", vec![0, 1, 2], list(7));
    for n in 0..20 {
        catalog.add_collection(&format!("List {n}"), vec![3], list(100 + n));
    }
    let all = catalog.all.as_ref().unwrap().key;
    let player = PlayerView::default();
    let mut state = ShellState::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut time = 0.0;
    let mut run = |ui: &mut Ui, state: &ShellState, events: &[pxui::Event]| {
        time += 0.02;
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            time,
            reduced_motion: 1,
            events: events.as_ptr(),
            event_count: events.len(),
            ..Default::default()
        };
        let mut intent = None;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, state, &mut intent)
        })
        .unwrap();
        intent
    };
    let find = |ui: &Ui, role: Role, name: &str| {
        ui.a11y_nodes()
            .iter()
            .find(|node| node.is_role(role) && ui.a11y_name_of(node) == name)
            .map(|node| (node.id, node.rect))
    };
    let activate = |ui: &mut Ui, role: Role, name: &str| {
        let (id, _) = find(ui, role, name).unwrap_or_else(|| panic!("no {name}"));
        ui.inject_a11y(&[pxui::A11yAction {
            id,
            action_code: 1,
            ..Default::default()
        }]);
    };
    let pointer = |kind, x: f32, y: f32, code| pxui::Event {
        kind,
        code,
        x: f64::from(x),
        y: f64::from(y),
        ..Default::default()
    };
    run(&mut ui, &state, &[]);
    // The sidebar lists scroll; New list ends them. Then the dialog refuses a blank name.
    for _ in 0..30 {
        run(
            &mut ui,
            &state,
            &[
                pointer(input::wire::POINTER_MOVED, 100.0, 450.0, 0),
                pxui::Event { kind: input::wire::SCROLLED, y: -1.0, ..Default::default() },
            ],
        );
    }
    activate(&mut ui, Role::Button, "New list");
    assert_eq!(run(&mut ui, &state, &[]), Some(Intent::NewList(None)));
    state.apply(Intent::NewList(None));
    run(&mut ui, &state, &[]);
    activate(&mut ui, Role::Button, "Create");
    assert_eq!(run(&mut ui, &state, &[]), None, "blank name");
    let name = Query::new("  Road trip  ").unwrap();
    state.apply(Intent::EditPlaylistName(name));
    run(&mut ui, &state, &[]);
    activate(&mut ui, Role::Button, "Create");
    assert_eq!(
        run(&mut ui, &state, &[]),
        Some(Intent::CreateList { name, song: None })
    );
    activate(&mut ui, Role::Button, "Cancel");
    assert_eq!(run(&mut ui, &state, &[]), Some(Intent::CancelPlaylistName));
    state.apply(Intent::CancelPlaylistName);
    run(&mut ui, &state, &[]);
    assert!(find(&ui, Role::Button, "Create").is_none());

    // Track menu: New list… carries the song; every list is reachable by scrolling the panel.
    state.apply(Intent::SongMenu(1, 1, all, (500.0, 200.0)));
    state.apply(Intent::SongMenuLists);
    run(&mut ui, &state, &[]);
    let (_, first) = find(&ui, Role::MenuItem, "New list…").unwrap();
    let bottom = first.y + 8.0 * 36.0 + 2.0;
    let last = |ui: &Ui| find(ui, Role::MenuItem, "List 19").map(|item| item.1);
    assert!(
        last(&ui).is_none_or(|rect| rect.y >= bottom),
        "starts below the fold"
    );
    let wheel = |y| pxui::Event {
        kind: input::wire::SCROLLED,
        x: f64::from(first.x + 20.0),
        y,
        ..Default::default()
    };
    let mut reached = None;
    for _ in 0..40 {
        run(
            &mut ui,
            &state,
            &[
                pointer(
                    input::wire::POINTER_MOVED,
                    first.x + 20.0,
                    first.y + 40.0,
                    0,
                ),
                wheel(-60.0),
            ],
        );
        reached = last(&ui).filter(|rect| rect.y + rect.h <= bottom && rect.y >= first.y);
        if reached.is_some() {
            break;
        }
    }
    reached.expect("the last list scrolls into the panel");
    for _ in 0..30 {
        run(&mut ui, &state, &[]);
    }
    let rect = last(&ui).unwrap();
    let (x, y) = (rect.x + 20.0, rect.y + rect.h * 0.5);
    let click = [
        pointer(input::wire::POINTER_MOVED, x, y, 0),
        pointer(input::wire::POINTER_PRESSED, x, y, 0),
        pointer(input::wire::POINTER_RELEASED, x, y, 0),
    ];
    run(&mut ui, &state, &click[..1]);
    run(&mut ui, &state, &click[1..2]);
    assert_eq!(
        run(&mut ui, &state, &click[2..]),
        Some(Intent::AddToList { song: 1, list: 119 })
    );
    // Back up to New list… with the same panel.
    for _ in 0..40 {
        run(
            &mut ui,
            &state,
            &[
                pointer(
                    input::wire::POINTER_MOVED,
                    first.x + 20.0,
                    first.y + 40.0,
                    0,
                ),
                wheel(60.0),
            ],
        );
    }
    activate(&mut ui, Role::MenuItem, "New list…");
    assert_eq!(run(&mut ui, &state, &[]), Some(Intent::NewList(Some(1))));
    state.apply(Intent::NewList(Some(1)));
    state.apply(Intent::EditPlaylistName(name));
    run(&mut ui, &state, &[]);
    activate(&mut ui, Role::Button, "Create");
    assert_eq!(
        run(&mut ui, &state, &[]),
        Some(Intent::CreateList {
            name,
            song: Some(1)
        })
    );
    state.apply(Intent::CancelPlaylistName);

    // Sidebar list menu: secondary press, Rename prefilled, Delete behind a confirmation.
    for _ in 0..30 {
        run(
            &mut ui,
            &state,
            &[
                pointer(input::wire::POINTER_MOVED, 100.0, 450.0, 0),
                pxui::Event { kind: input::wire::SCROLLED, y: 1.0, ..Default::default() },
            ],
        );
    }
    run(&mut ui, &state, &[]);
    let (_, row) = find(&ui, Role::Button, "Mix").unwrap();
    let (x, y) = (row.x + 10.0, row.y + 10.0);
    let secondary = [
        pointer(input::wire::POINTER_MOVED, x, y, 0),
        pointer(input::wire::POINTER_PRESSED, x, y, 1),
    ];
    run(&mut ui, &state, &secondary[..1]);
    let opened = run(&mut ui, &state, &secondary[1..]);
    assert!(matches!(opened, Some(Intent::ListMenu(7, _))), "{opened:?}");
    state.apply(opened.unwrap());
    run(&mut ui, &state, &[]);
    activate(&mut ui, Role::MenuItem, "Rename…");
    let rename = Intent::RenameListDialog(7, Query::new("Mix").unwrap());
    assert_eq!(run(&mut ui, &state, &[]), Some(rename));
    state.apply(rename);
    run(&mut ui, &state, &[]);
    activate(&mut ui, Role::Button, "Save");
    assert_eq!(
        run(&mut ui, &state, &[]),
        Some(Intent::RenameList {
            list: 7,
            name: Query::new("Mix").unwrap()
        })
    );
    state.apply(Intent::CancelPlaylistName);
    state.apply(Intent::ListMenu(7, (300.0, 300.0)));
    run(&mut ui, &state, &[]);
    activate(&mut ui, Role::MenuItem, "Delete list");
    assert_eq!(
        run(&mut ui, &state, &[]),
        Some(Intent::ConfirmDeleteList(7))
    );
    state.apply(Intent::ConfirmDeleteList(7));
    run(&mut ui, &state, &[]);
    activate(&mut ui, Role::Button, "Cancel");
    assert_eq!(run(&mut ui, &state, &[]), Some(Intent::CloseDeleteList));
    run(&mut ui, &state, &[]);
    activate(&mut ui, Role::Button, "Delete List");
    assert_eq!(run(&mut ui, &state, &[]), Some(Intent::DeleteList(7)));
    state.apply(Intent::CloseDeleteList);

    // Drag reorder in the list route emits the new position; sorted views offer no grips.
    let slot = catalog
        .collections
        .iter()
        .position(|group| matches!(&*group.context, PlaybackContext::Playlist { id: 7, .. }))
        .unwrap();
    state.apply(Intent::Navigate(Route::Collection(slot)));
    for _ in 0..3 {
        run(&mut ui, &state, &[]);
    }
    let grips: Vec<_> = ui
        .a11y_nodes()
        .iter()
        .filter(|node| ui.a11y_name_of(node) == "Reorder track")
        .map(|node| node.rect)
        .collect();
    assert_eq!(grips.len(), 3);
    let (from, to) = (grips[0], grips[2]);
    let (x, y0, y1) = (from.x + 10.0, from.y + 10.0, to.y + to.h * 0.8);
    assert_eq!(
        run(
            &mut ui,
            &state,
            &[pointer(input::wire::POINTER_MOVED, x, y0, 0)]
        ),
        None
    );
    assert_eq!(
        run(
            &mut ui,
            &state,
            &[pointer(input::wire::POINTER_PRESSED, x, y0, 0)]
        ),
        None
    );
    assert_eq!(
        run(
            &mut ui,
            &state,
            &[pointer(input::wire::POINTER_MOVED, x, y1, 0)]
        ),
        None
    );
    assert_eq!(run(&mut ui, &state, &[]), None);
    assert_eq!(
        run(
            &mut ui,
            &state,
            &[pointer(input::wire::POINTER_RELEASED, x, y1, 0)]
        ),
        Some(Intent::MoveListRow { from: 0, to: 2 })
    );
    state.apply(Intent::Sort(crate::browse::Sort::Title));
    for _ in 0..3 {
        run(&mut ui, &state, &[]);
    }
    assert!(find(&ui, Role::Button, "Reorder track").is_none());
    state.apply(Intent::Navigate(Route::Songs));

    // Warm frames with each dialog open allocate nothing.
    for cycle in 0..8 {
        mem_sys::local_reset();
        for open in [
            Intent::NewList(None),
            Intent::ListMenu(7, (300.0, 300.0)),
            Intent::ConfirmDeleteList(7),
        ] {
            state.apply(open);
            run(&mut ui, &state, &[]);
            state.apply(Intent::CancelPlaylistName);
            state.apply(Intent::CloseListMenu);
            state.apply(Intent::CloseDeleteList);
            run(&mut ui, &state, &[]);
        }
        if cycle >= 3 {
            assert_eq!(mem_sys::local_alloc_count(), 0, "warm list dialogs");
        }
    }
}

fn titled(count: i64) -> Catalog {
    crate::tests::prepared(
        (0..count)
            .map(|id| lw_core::Song {
                id: Some(id),
                title: format!("T{id}"),
                artist: "Artist".into(),
                album: "Album".into(),
                duration: Some(60.0),
                ..Default::default()
            })
            .collect(),
    )
}

#[test]
fn multi_selection_toggles_ranges_in_sort_order_and_rebuilds_only_changed_rows() {
    use crate::Selection;
    mem_sys::set_peer(pxui::library_counter);
    let mut catalog = titled(6);
    let key = catalog.all.as_ref().unwrap().key;
    // Displayed order is T5..T0, so index ranges differ from id ranges.
    catalog.project(
        crate::browse::Request {
            commands: None,
            scope: Scope::Collection(key),
            query: Query::default(),
            sort: Sort::Title,
            ascending: false,
        },
        vec![5, 4, 3, 2, 1, 0],
    );
    let mut state = ShellState::default();
    let player = PlayerView::default();
    let mut ui = pxui::Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut time = 0.0;
    let mut run = |ui: &mut Ui, state: &mut ShellState, events: &[pxui::Event]| {
        time += 0.02;
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            time,
            reduced_motion: 1,
            events: events.as_ptr(),
            event_count: events.len(),
            ..Default::default()
        };
        let mut intent = None;
        mem_sys::local_reset();
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, state, &mut intent)
        })
        .unwrap();
        if let Some(intent) = intent {
            state.apply(intent);
        }
        intent
    };
    let key_event = |kind, code: u32| pxui::Event {
        kind,
        code,
        ..Default::default()
    };
    let (meta, shift) = (input::Key::Meta.code(), input::Key::Shift.code());
    let tap = |ui: &mut Ui,
               state: &mut ShellState,
               run: &mut dyn FnMut(&mut Ui, &mut ShellState, &[pxui::Event]) -> Option<Intent>,
               code: u32| {
        let intent = run(ui, state, &[key_event(input::wire::KEY_PRESSED, code)]);
        run(ui, state, &[key_event(input::wire::KEY_RELEASED, code)]);
        intent
    };
    let click =
        |ui: &mut Ui,
         state: &mut ShellState,
         run: &mut dyn FnMut(&mut Ui, &mut ShellState, &[pxui::Event]) -> Option<Intent>,
         name: &str| {
            let id = ui
                .a11y_nodes()
                .iter()
                .find(|n| n.is_role(Role::ListItem) && ui.a11y_name_of(n) == name)
                .unwrap()
                .id;
            ui.inject_a11y(&[pxui::A11yAction {
                id,
                action_code: 1,
                ..Default::default()
            }]);
            run(ui, state, &[])
        };
    let ids = |state: &ShellState| state.selected_song_ids(&catalog).collect::<Vec<_>>();
    run(&mut ui, &mut state, &[]);
    run(&mut ui, &mut state, &[]);

    // Plain click selects one row; only the old and new rows rebuild.
    assert_eq!(
        click(&mut ui, &mut state, &mut run, "T4"),
        Some(Intent::SelectRow(1))
    );
    assert_eq!(ids(&state), [4]);
    run(&mut ui, &mut state, &[]);
    assert_eq!(
        ui.rebuilt_named("song-visual"),
        1,
        "only the newly selected row"
    );
    run(&mut ui, &mut state, &[]);
    assert_eq!(
        mem_sys::local_alloc_count(),
        0,
        "warm frame with a selection"
    );

    // Cmd-click toggles on and off.
    run(
        &mut ui,
        &mut state,
        &[key_event(input::wire::KEY_PRESSED, meta)],
    );
    assert_eq!(
        click(&mut ui, &mut state, &mut run, "T1"),
        Some(Intent::Select(Selection::Toggle(4)))
    );
    assert_eq!(ids(&state), [4, 1]);
    run(&mut ui, &mut state, &[]);
    assert_eq!(ui.rebuilt_named("song-visual"), 1);
    click(&mut ui, &mut state, &mut run, "T1");
    assert_eq!(ids(&state), [4]);
    run(
        &mut ui,
        &mut state,
        &[key_event(input::wire::KEY_RELEASED, meta)],
    );

    click(&mut ui, &mut state, &mut run, "T4");
    // Shift-click spans anchor (T4) to the target in the displayed order: T4, T3, T2.
    run(
        &mut ui,
        &mut state,
        &[key_event(input::wire::KEY_PRESSED, shift)],
    );
    click(&mut ui, &mut state, &mut run, "T2");
    run(
        &mut ui,
        &mut state,
        &[key_event(input::wire::KEY_RELEASED, shift)],
    );
    assert_eq!(ids(&state), [4, 3, 2]);

    // Shift+Down extends by one displayed row; plain Up moves a single selection.
    run(
        &mut ui,
        &mut state,
        &[key_event(input::wire::KEY_PRESSED, shift)],
    );
    tap(&mut ui, &mut state, &mut run, input::Key::ArrowDown.code());
    run(
        &mut ui,
        &mut state,
        &[key_event(input::wire::KEY_RELEASED, shift)],
    );
    assert_eq!(ids(&state), [4, 3, 2, 1]);
    tap(&mut ui, &mut state, &mut run, input::Key::ArrowUp.code());
    assert_eq!(ids(&state), [2]);
    assert_eq!(state.selected, Some(3));

    // Selection survives scrolling away and back; Cmd-A selects the collection; Esc clears.
    run(
        &mut ui,
        &mut state,
        &[key_event(input::wire::KEY_PRESSED, meta)],
    );
    assert_eq!(
        tap(&mut ui, &mut state, &mut run, 'a' as u32),
        Some(Intent::Select(Selection::All(6)))
    );
    run(
        &mut ui,
        &mut state,
        &[key_event(input::wire::KEY_RELEASED, meta)],
    );
    assert_eq!(ids(&state), [5, 4, 3, 2, 1, 0]);
    state.apply(Intent::Select(Selection::Toggle(0)));
    for _ in 0..3 {
        run(&mut ui, &mut state, &[]);
    }
    assert_eq!(mem_sys::local_alloc_count(), 0);
    assert_eq!(ids(&state), [4, 3, 2, 1, 0]);
    assert_eq!(
        tap(&mut ui, &mut state, &mut run, input::Key::Escape.code()),
        Some(Intent::Select(Selection::Clear))
    );
    assert!(ids(&state).is_empty());
}

#[test]
fn select_all_is_a_no_op_while_find_is_focused() {
    mem_sys::set_peer(pxui::library_counter);
    let mut catalog = titled(3);
    catalog.project(
        crate::browse::Request {
            commands: None,
            scope: Scope::Find,
            query: Query::default(),
            sort: Sort::Original,
            ascending: true,
        },
        vec![0, 1, 2],
    );
    let mut state = ShellState::default();
    state.apply(Intent::Navigate(Route::Find));
    let player = PlayerView::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut time = 0.0;
    let mut run = |ui: &mut Ui, events: &[pxui::Event]| {
        time += 0.02;
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            time,
            events: events.as_ptr(),
            event_count: events.len(),
            ..Default::default()
        };
        let mut intent = None;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
        })
        .unwrap();
        intent
    };
    run(&mut ui, &[]);
    run(&mut ui, &[]);
    let field = ui
        .a11y_nodes()
        .iter()
        .find(|node| node.is_role(Role::TextField))
        .unwrap()
        .rect;
    let at = |kind| pxui::Event {
        kind,
        x: f64::from(field.x + field.w * 0.8),
        y: f64::from(field.y + field.h * 0.5),
        ..Default::default()
    };
    run(
        &mut ui,
        &[
            at(input::wire::POINTER_MOVED),
            at(input::wire::POINTER_PRESSED),
            at(input::wire::POINTER_RELEASED),
        ],
    );
    assert!(ui.text_input_focused());
    let key = |code: u32| pxui::Event {
        kind: input::wire::KEY_PRESSED,
        code,
        ..Default::default()
    };
    run(&mut ui, &[key(input::Key::Meta.code())]);
    assert_eq!(run(&mut ui, &[key('a' as u32)]), None);
}

#[test]
fn onboarding_and_route_empty_states_show_their_text_without_warm_allocations() {
    use lw_core::context::PlaybackContext;
    mem_sys::set_peer(pxui::library_counter);
    let mut empty = titled(0);
    std::sync::Arc::make_mut(&mut empty.library).set_source_count(0);
    let mut sourced = titled(0);
    std::sync::Arc::make_mut(&mut sourced.library).set_source_count(1);
    let mut filled = titled(2);
    filled.add_collection(
        "Quiet",
        Vec::new(),
        PlaybackContext::Playlist {
            id: 9,
            name: "Quiet".into(),
        },
    );
    filled.prepare_tables().unwrap();
    let list = filled
        .collections
        .iter()
        .position(|group| matches!(&*group.context, PlaybackContext::Playlist { id: 9, .. }))
        .unwrap();
    let folders = crate::files::Snapshot {
        account: Some(1),
        ..Default::default()
    };
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let time = std::cell::Cell::new(0.0);
    // Renders three warm frames; the last must not allocate. Returns the accessibility strings.
    let show = |ui: &mut Ui, catalog: &Catalog, state: &ShellState, player: &PlayerView<'_>| {
        let mut intent = None;
        for cycle in 0..4 {
            time.set(time.get() + 0.02);
            mem_sys::local_reset();
            ui.frame(
                &FrameIn {
                    width: 1440.0,
                    height: 872.0,
                    time: time.get(),
                    reduced_motion: 1,
                    ..Default::default()
                },
                &mut |ui| shell(ui, player, 0.0, catalog, state, &mut intent),
            )
            .unwrap();
            if cycle == 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm empty state");
            }
        }
        (ui.a11y_nodes().iter())
            .map(|node| ui.a11y_name_of(node).to_string())
            .collect::<Vec<_>>()
    };
    let has = |names: &[String], text: &str| names.iter().any(|name| name == text);
    let player = PlayerView::default();
    let mut state = ShellState::default();

    // No sources and no songs: onboarding with its three steps; unloaded sources never show it.
    let names = show(&mut ui, &empty, &state, &player);
    for text in [
        "Choose a folder",
        "Pick some files",
        "Explore the library",
        "0 songs so far",
        "v2",
        "Google Drive not in the list?",
        "Point it at a folder.",
        "Choose a folder in Files",
        "Your files stay where they are. Nothing is uploaded.",
    ] {
        assert!(has(&names, text), "onboarding: {text}");
    }
    assert!(!has(
        &show(&mut ui, &titled(0), &state, &player),
        "Choose a folder"
    ));
    // Its primary action is the add-folder flow, shortcut and button alike.
    show(&mut ui, &empty, &state, &player);
    let button = ui
        .a11y_nodes()
        .iter()
        .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == "Choose a folder")
        .unwrap()
        .id;
    ui.inject_a11y(&[pxui::A11yAction {
        id: button,
        action_code: 1,
        ..Default::default()
    }]);
    let mut intent = None;
    ui.frame(
        &FrameIn {
            width: 1440.0,
            height: 872.0,
            time: 9.0,
            reduced_motion: 1,
            ..Default::default()
        },
        &mut |ui| shell(ui, &player, 0.0, &empty, &state, &mut intent),
    )
    .unwrap();
    assert_eq!(intent, Some(Intent::ChooseFolder));
    // The other actions and the Drive disclosure (collapsed until pressed).
    let press = |ui: &mut Ui, name: &str, time: f64| {
        show(ui, &empty, &ShellState::default(), &player);
        let id = (ui.a11y_nodes().iter())
            .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == name)
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction { id, action_code: 1, ..Default::default() }]);
        let mut intent = None;
        let frame = FrameIn { width: 1440.0, height: 872.0, time, reduced_motion: 1, ..Default::default() };
        ui.frame(&frame, &mut |ui| shell(ui, &player, 0.0, &empty, &ShellState::default(), &mut intent))
            .unwrap();
        intent
    };
    assert_eq!(press(&mut ui, "Pick some files", 10.0), Some(Intent::ChooseAudio));
    assert_eq!(press(&mut ui, "Explore the library", 11.0), Some(Intent::Navigate(Route::Albums)));
    let help = "Install the Google Drive app, then in Files tap Browse → ⋯ → Edit and turn on Google Drive.";
    assert!(!has(&show(&mut ui, &empty, &state, &player), help));
    assert_eq!(press(&mut ui, "Google Drive not in the list?", 12.0), None);
    assert!(has(&show(&mut ui, &empty, &state, &player), help));
    // The persisted `lwDidStart` flag suppresses onboarding for an empty library; a library that
    // gained a source asks once for the flag to be saved.
    let started = PlayerView {
        settings: lw_core::preferences::Settings { did_start: true, ..Default::default() },
        ..Default::default()
    };
    assert!(!has(&show(&mut ui, &empty, &state, &started), "Choose a folder"));
    let mut intent = None;
    let frame = FrameIn { width: 1440.0, height: 872.0, time: 13.0, reduced_motion: 1, ..Default::default() };
    ui.frame(&frame, &mut |ui| shell(ui, &player, 0.0, &sourced, &state, &mut intent)).unwrap();
    assert_eq!(intent, Some(Intent::Setting(lw_core::preferences::Change::DidStart)));
    // Exploring leaves onboarding.
    state.apply(Intent::Navigate(Route::Albums));
    assert!(!has(
        &show(&mut ui, &empty, &state, &player),
        "Choose a folder"
    ));

    // A source without songs gets the library message on Songs, Albums and Artists.
    for route in [Route::Songs, Route::Albums, Route::Artists] {
        state.apply(Intent::Navigate(route));
        let names = show(&mut ui, &sourced, &state, &player);
        assert!(has(&names, onboarding::LIBRARY_EMPTY), "{route:?}");
    }
    // Find without a query hints; a list without songs has its own text.
    let find = &mut state;
    find.apply(Intent::Navigate(Route::Find));
    let request = find.request(&filled).unwrap();
    filled.project(request, Vec::new());
    assert!(has(
        &show(&mut ui, &filled, &*find, &player),
        onboarding::FIND_HINT
    ));
    let lists = &mut state;
    lists.apply(Intent::Navigate(Route::Collection(list)));
    assert!(has(
        &show(&mut ui, &filled, &*lists, &player),
        "No songs here yet."
    ));
    // Queue and Files.
    let queue = &mut state;
    queue.apply(Intent::ShowQueue);
    filled.queue = Some(std::sync::Arc::new(crate::queue::Snapshot::new(
        1,
        std::sync::Arc::from([]),
        None,
        std::sync::Arc::new(PlaybackContext::Library),
        false,
    )));
    assert!(has(
        &show(&mut ui, &filled, &*queue, &player),
        "Queue is empty."
    ));
    let files = &mut state;
    files.apply(Intent::CloseQueue);
    files.apply(Intent::Navigate(Route::Files));
    let names = show(
        &mut ui,
        &filled,
        &*files,
        &PlayerView {
            files: Some(&folders),
            ..Default::default()
        },
    );
    assert!(has(&names, "Point it at a folder."));
}

#[test]
fn song_drag_carries_row_or_selection_and_drops_on_lists_and_up_next() {
    use lw_core::{context::PlaybackContext, queue::Entry};
    use std::{cell::Cell, sync::Arc};
    mem_sys::set_peer(pxui::library_counter);
    let mut catalog = titled(4);
    catalog.add_collection(
        "Mix",
        vec![0],
        PlaybackContext::Playlist {
            id: 7,
            name: String::new(),
        },
    );
    catalog.queue = Some(Arc::new(crate::queue::Snapshot::new(
        1,
        (1..=3).map(|id| Entry { id, song_id: 0 }).collect(),
        Some(0),
        Arc::new(PlaybackContext::Library),
        false,
    )));
    let player = PlayerView {
        initialized: true,
        title: "T0",
        duration: 60_000.0,
        ..Default::default()
    };
    let mut state = ShellState::default();
    state.apply(Intent::Navigate(Route::Songs));
    state.apply(Intent::Select(crate::Selection::Toggle(1)));
    state.apply(Intent::Select(crate::Selection::Toggle(3)));
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut time = 0.0;
    let damage = Cell::new([0.0f32; 4]);
    let mut run = |ui: &mut Ui, events: &[pxui::Event]| {
        time += 0.02;
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            time,
            reduced_motion: 1,
            events: events.as_ptr(),
            event_count: events.len(),
            ..Default::default()
        };
        let mut intent = None;
        mem_sys::local_reset();
        let out = ui
            .frame(&frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, &state, &mut intent)
            })
            .unwrap();
        damage.set(if out.has_damage != 0 { out.damage } else { [0.0; 4] });
        intent
    };
    let find = |ui: &Ui, role: Role, name: &str| {
        ui.a11y_nodes()
            .iter()
            .find(|node| node.is_role(role) && ui.a11y_name_of(node) == name)
            .map(|node| node.rect)
            .unwrap_or_else(|| panic!("no {name}"))
    };
    let pointer = |kind, x: f32, y: f32| pxui::Event {
        kind,
        x: f64::from(x),
        y: f64::from(y),
        ..Default::default()
    };
    for _ in 0..3 {
        run(&mut ui, &[]);
    }
    let touches = |rect: Rect, damage: [f32; 4]| {
        damage[2] > 0.0
            && damage[0] < rect.x + rect.w
            && damage[0] + damage[2] > rect.x
            && damage[1] < rect.y + rect.h
            && damage[1] + damage[3] > rect.y
    };
    let centre = |rect: Rect| (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5);
    // Press `row`, drag onto `target` and hover there, then release or cancel with Esc.
    macro_rules! drag {
        ($row:expr, $target:expr, $end:expr) => {{
            let (x, y) = centre(find(&ui, Role::ListItem, $row));
            let (tx, ty) = centre($target);
            run(&mut ui, &[pointer(input::wire::POINTER_MOVED, x, y)]);
            run(&mut ui, &[pointer(input::wire::POINTER_PRESSED, x, y)]);
            run(&mut ui, &[pointer(input::wire::POINTER_MOVED, x, y + 8.0)]);
            let kind = ui.carrying_ref().map(|p| p.kind.to_owned());
            damage.set([0.0; 4]);
            run(&mut ui, &[pointer(input::wire::POINTER_MOVED, tx, ty)]);
            let mut repainted = touches($target, damage.get());
            run(&mut ui, &[]);
            repainted |= touches($target, damage.get());
            let intent = match $end {
                "drop" => run(&mut ui, &[pointer(input::wire::POINTER_RELEASED, tx, ty)]),
                _ => {
                    run(&mut ui, &[pxui::Event {
                        kind: input::wire::KEY_PRESSED,
                        code: input::Key::Escape.code(),
                        ..Default::default()
                    }]);
                    assert!(ui.carrying_ref().is_none(), "Esc cancels the carry");
                    run(&mut ui, &[pointer(input::wire::POINTER_RELEASED, tx, ty)])
                }
            };
            run(&mut ui, &[]);
            (kind, repainted, intent)
        }};
    }
    let mix = find(&ui, Role::Button, "Mix");
    let up_next = find(&ui, Role::List, "Up next queue");
    // An unselected row carries just itself.
    let (kind, repainted, intent) = drag!("T2", mix, "drop");
    assert_eq!(kind.as_deref(), Some("localwave.song:2"));
    assert!(repainted, "hovered list repaints");
    assert_eq!(intent, Some(Intent::AddToList { song: 2, list: 7 }));
    // A selected row carries the id the drop resolves against the whole selection.
    let (kind, repainted, intent) = drag!("T1", up_next, "drop");
    assert_eq!(kind.as_deref(), Some("localwave.song:1"));
    assert!(repainted, "hovered Up next repaints");
    assert_eq!(intent, Some(Intent::AppendSong(1)));
    // Esc cancels; dropping on a library row does nothing.
    let (_, _, intent) = drag!("T2", mix, "esc");
    assert_eq!(intent, None);
    let t0 = find(&ui, Role::ListItem, "T0");
    let (_, _, intent) = drag!("T2", t0, "drop");
    assert_eq!(intent, None);
    // Warm frames during a drag allocate nothing.
    let (x, y) = centre(find(&ui, Role::ListItem, "T2"));
    run(&mut ui, &[pointer(input::wire::POINTER_PRESSED, x, y)]);
    run(&mut ui, &[pointer(input::wire::POINTER_MOVED, x, y + 8.0)]);
    let (mx, my) = centre(up_next);
    run(&mut ui, &[pointer(input::wire::POINTER_MOVED, mx, my)]);
    for _ in 0..4 {
        run(&mut ui, &[]);
        assert_eq!(mem_sys::local_alloc_count(), 0, "warm frame during a drag");
    }
}

#[test]
fn metadata_editor_opens_prefilled_saves_edited_fields_and_delete_confirms_without_allocations() {
    mem_sys::set_peer(pxui::library_counter);
    let catalog = crate::tests::prepared(vec![lw_core::Song {
        id: Some(1),
        title: "Quiet lights".into(),
        artist: "The National".into(),
        album: "A midnight album".into(),
        album_artist: "Various".into(),
        release_year: Some(2010),
        track_number: Some(3),
        ..Default::default()
    }]);
    let all = catalog.all.as_ref().unwrap().key;
    let player = PlayerView::default();
    let mut state = ShellState::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut time = 0.0;
    let mut run = |ui: &mut Ui, state: &ShellState| {
        time += 0.02;
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            time,
            reduced_motion: 1,
            ..Default::default()
        };
        let mut intent = None;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, state, &mut intent)
        })
        .unwrap();
        intent
    };
    let activate = |ui: &mut Ui, role: Role, name: &str| {
        let id = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(role) && ui.a11y_name_of(node) == name)
            .unwrap_or_else(|| panic!("no {name}"))
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id,
            action_code: 1,
            ..Default::default()
        }]);
    };
    let q = |text: &str| Query::new(text).unwrap();
    run(&mut ui, &state);
    state.apply(Intent::SongMenu(0, 1, all, (500.0, 200.0)));
    run(&mut ui, &state);
    activate(&mut ui, Role::MenuItem, "Edit info…");
    let opened = run(&mut ui, &state);
    let prefilled = crate::MetadataEdit {
        song: 1,
        title: q("Quiet lights"),
        artist: q("The National"),
        album: q("A midnight album"),
        album_artist: q("Various"),
        year: q("2010"),
        track: q("3"),
    };
    assert_eq!(opened, Some(Intent::OpenMetadata(prefilled)));
    state.apply(opened.unwrap());
    run(&mut ui, &state);
    run(&mut ui, &state); // the tall sheet settles to its centred position
    activate(&mut ui, Role::Button, "Cancel");
    assert_eq!(
        run(&mut ui, &state),
        Some(Intent::CloseMetadata),
        "Cancel emits no save"
    );
    state.apply(Intent::CloseMetadata);
    run(&mut ui, &state);

    // Save carries the edited fields; invalid numbers or a blank title refuse it.
    state.apply(Intent::OpenMetadata(prefilled));
    state.apply(Intent::MetadataField(0, q("Brand new")));
    state.apply(Intent::MetadataField(4, q("1999")));
    run(&mut ui, &state);
    activate(&mut ui, Role::Button, "Save");
    assert_eq!(
        run(&mut ui, &state),
        Some(Intent::SaveMetadata(crate::MetadataEdit {
            title: q("Brand new"),
            year: q("1999"),
            ..prefilled
        }))
    );
    for (field, text) in [(4, "20x0"), (5, "10000"), (0, "  ")] {
        state.apply(Intent::OpenMetadata(prefilled));
        state.apply(Intent::MetadataField(field, q(text)));
        run(&mut ui, &state);
        activate(&mut ui, Role::Button, "Save");
        assert_eq!(run(&mut ui, &state), None, "{text:?}");
    }
    state.apply(Intent::CloseMetadata);

    // Delete asks first; only the confirmation emits the command.
    state.apply(Intent::SongMenu(0, 1, all, (500.0, 200.0)));
    run(&mut ui, &state);
    activate(&mut ui, Role::MenuItem, "Delete…");
    assert_eq!(run(&mut ui, &state), Some(Intent::ConfirmDeleteSong(1)));
    state.apply(Intent::ConfirmDeleteSong(1));
    run(&mut ui, &state);
    activate(&mut ui, Role::Button, "Cancel");
    assert_eq!(run(&mut ui, &state), Some(Intent::CloseDeleteSong));
    run(&mut ui, &state);
    activate(&mut ui, Role::Button, "Delete Song");
    assert_eq!(run(&mut ui, &state), Some(Intent::DeleteSong(1)));
    state.apply(Intent::CloseDeleteSong);

    for cycle in 0..8 {
        mem_sys::local_reset();
        for open in [
            Intent::OpenMetadata(prefilled),
            Intent::ConfirmDeleteSong(1),
        ] {
            state.apply(open);
            run(&mut ui, &state);
            state.apply(Intent::CloseMetadata);
            state.apply(Intent::CloseDeleteSong);
            run(&mut ui, &state);
        }
        if cycle >= 3 {
            assert_eq!(
                mem_sys::local_alloc_count(),
                0,
                "warm editor and delete dialogs"
            );
        }
    }
}

#[test]
fn arrow_selection_past_the_bottom_edge_scrolls_by_one_row_without_warm_allocations() {
    mem_sys::set_peer(pxui::library_counter);
    let catalog = titled(100);
    let mut state = ShellState::default();
    let player = PlayerView::default();
    let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
    ui.set_a11y(true);
    let mut time = 0.0;
    let mut run = |ui: &mut Ui, state: &mut ShellState, events: &[pxui::Event]| {
        time += 0.02;
        let frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            time,
            reduced_motion: 1,
            events: events.as_ptr(),
            event_count: events.len(),
            ..Default::default()
        };
        let mut intent = None;
        mem_sys::local_reset();
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, state, &mut intent)
        })
        .unwrap();
        if let Some(intent) = intent {
            state.apply(intent);
        }
    };
    let key = |kind, code: u32| pxui::Event {
        kind,
        code,
        ..Default::default()
    };
    let top_of = |ui: &Ui, row: usize| {
        let name = format!("T{row}");
        ui.a11y_nodes()
            .iter()
            .find(|n| n.is_role(Role::ListItem) && ui.a11y_name_of(n) == name)
            .map(|n| n.rect.y)
    };
    run(&mut ui, &mut state, &[]);
    run(&mut ui, &mut state, &[]);
    state.apply(Intent::SelectRow(0));
    let down = input::Key::ArrowDown.code();
    let mut tops = Vec::new();
    for row in 1..40 {
        run(&mut ui, &mut state, &[key(input::wire::KEY_PRESSED, down)]);
        run(&mut ui, &mut state, &[key(input::wire::KEY_RELEASED, down)]);
        run(&mut ui, &mut state, &[]);
        run(&mut ui, &mut state, &[]);
        assert_eq!(mem_sys::local_alloc_count(), 0, "warm frame at row {row}");
        assert_eq!(state.selected, Some(row));
        tops.push(top_of(&ui, row).unwrap_or_else(|| panic!("row {row} is not on screen")));
    }
    // The cursor walks down one row height per press, then stays at the bottom edge while the list scrolls.
    let last = *tops.last().unwrap();
    assert!(last + 38.0 <= 872.0 && tops[..10].windows(2).all(|w| w[1] == w[0] + 38.0), "{tops:?}");
    assert!(tops[20..].iter().all(|&y| y == last), "{tops:?}");

    // Arrowing back up keeps the cursor row visible too, without moving the list while it is inside.
    for _ in 0..39 {
        run(&mut ui, &mut state, &[key(input::wire::KEY_PRESSED, input::Key::ArrowUp.code())]);
        run(&mut ui, &mut state, &[key(input::wire::KEY_RELEASED, input::Key::ArrowUp.code())]);
        run(&mut ui, &mut state, &[]);
        run(&mut ui, &mut state, &[]);
    }
    assert_eq!(state.selected, Some(0));
    assert!(top_of(&ui, 0).is_some_and(|y| y >= 0.0), "row 0 visible after ArrowUp");
}
