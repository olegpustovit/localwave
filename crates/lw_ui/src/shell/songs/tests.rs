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
    let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
    let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
fn retained_bars_match_native_phases_and_pending_copy_pulses_without_warm_builds() {
    mem_sys::set_peer(pxui::library_counter);
    let catalog = catalog(1);
    for scale in [1.0, 2.0] {
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
    let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
    let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
    let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
