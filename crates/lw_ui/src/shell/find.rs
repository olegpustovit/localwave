use super::*;

#[derive(Clone, Copy)]
enum Action {
    Play,
    Offline(bool),
}

fn background(ui: &mut Ui, key: &str, color: Rgba, shown: bool, immediate: bool) {
    let view = ui.view(key, 0, Size::Grow, Size::Grow, &mut |ui| {
        ui.node(
            "fill",
            Node {
                w: Size::Grow,
                h: Size::Grow,
                look: Look {
                    fill: color,
                    ..Look::NONE
                },
                ..Node::DEFAULT
            },
        );
    });
    let previous = ui.state(&view, -1);
    let next = i64::from(shown);
    if previous != next {
        ui.set_state(&view, next);
        ui.animate(
            &view,
            Target::Opacity(if shown { 1.0 } else { 0.0 }),
            if previous < 0 || immediate {
                crate::motion::steps(1, 0.0)
            } else {
                crate::motion::out(crate::motion::QUICK)
            },
        );
    }
}
fn button(ui: &mut Ui, action: Action, enabled: bool, p: Palette) -> bool {
    let (key, label, icon_size, selected) = match action {
        Action::Play => ("play-all", "Play all", 16.0, false),
        Action::Offline(kept) => (
            "keep-offline",
            if kept { "Kept offline" } else { "Keep offline" },
            18.0,
            kept,
        ),
    };
    let mut key_hash = DefaultHasher::new();
    ("find-action-label", label).hash(&mut key_hash);
    let style = pxui::TextStyle {
        size_points: 13.0,
        line_height: 18.2,
        max_lines: 1,
        ellipsis: pxui::TextStyle::ELLIPSIS_END,
        ..Default::default()
    };
    let density = ui.density() as f32;
    let label_width = (ui
        .text_service()
        .width(key_hash.finish(), label, &style, 0.0)
        .unwrap()
        * density)
        .ceil()
        / density;
    let root = ui.node(
        key,
        Node {
            flags: if enabled { CLICKABLE | TAB_STOP } else { 0 },
            role: Role::Button,
            axis: Axis::Stack,
            w: Size::Px(20.0 + icon_size + 6.0 + label_width),
            h: Size::Px(30.0),
            state: State {
                disabled: !enabled,
                ..State::default()
            },
            look: Look {
                fill: if selected { p.selected } else { p.paper },
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.a11y_name(&root, label);
    if matches!(action, Action::Offline(_)) {
        ui.a11y_value(
            &root,
            if selected {
                "Kept offline"
            } else {
                "Not kept offline"
            },
            (0, 0),
        );
    }
    let signal = ui.signal_state(&root);
    let before = ui.state(&root, -1);
    ui.set_state(&root, i64::from(selected));
    ui.within(&root, &mut |ui| {
        background(
            ui,
            "hover",
            p.panel,
            signal.hovered && !selected,
            before != i64::from(selected),
        );
        background(ui, "press", p.panel, signal.held, false);
        let mut revision = DefaultHasher::new();
        (label, selected).hash(&mut revision);
        ui.view(
            "action-copy",
            revision.finish(),
            Size::Grow,
            Size::Grow,
            &mut |ui| {
                let row = ui.node(
                    "content",
                    Node {
                        axis: Axis::Row,
                        align: Align::Center,
                        gap: 6.0,
                        w: Size::Grow,
                        h: Size::Grow,
                        padding: Edges {
                            l: 10.0,
                            r: 10.0,
                            ..Edges::ZERO
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&row, &mut |ui| {
                    let color = if selected { p.ink } else { p.secondary };
                    let icon = ui.node(
                        "icon",
                        Node {
                            w: Size::Px(icon_size),
                            h: Size::Px(icon_size),
                            ..Node::DEFAULT
                        },
                    );
                    ui.draw(&icon, move |paint, rect| match action {
                        Action::Play => crate::icons::play(paint, rect, color),
                        Action::Offline(kept) => crate::icons::offline(paint, rect, kept, color),
                    });
                    let copy = text(
                        ui,
                        "label",
                        label,
                        13,
                        color,
                        (Size::Px(label_width), 18.2),
                        false,
                    );
                    ui.set_role(&copy, Role::Unknown, 0);
                });
            },
        );
    });
    enabled && signal.clicked
}

pub(super) fn toolbar(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let collection = state.collection(catalog);
    let summary = collection
        .and(catalog.view.as_ref())
        .and_then(|view| view.find.as_ref());
    let tools = ui.node(
        "find-tools",
        Node {
            axis: Axis::Column,
            w: Size::Grow,
            h: Size::Px(45.0),
            ..Node::DEFAULT
        },
    );
    ui.within(&tools, &mut |ui| {
        let row = ui.node(
            "buttons",
            Node {
                axis: Axis::Row,
                gap: 8.0,
                w: Size::Grow,
                h: Size::Px(30.0),
                ..Node::DEFAULT
            },
        );
        ui.within(&row, &mut |ui| {
            if button(
                ui,
                Action::Play,
                collection.is_some_and(|group| !group.ids.is_empty()),
                p,
            ) {
                *intent = Some(Intent::PlayRow(0));
            }
            if button(
                ui,
                Action::Offline(summary.is_some_and(|value| value.pinned)),
                summary.is_some_and(|value| !value.eligible.is_empty()),
                p,
            ) {
                *intent = Some(Intent::TogglePins);
            }
        });
    });
    ui.draw_over(&tools, move |paint, rect| {
        paint.fill_fractional(
            rect.x - 32.0,
            rect.y + rect.h - 1.0,
            rect.w + 64.0,
            1.0,
            p.line,
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::{A11yAction, FrameIn, Platform};

    fn catalogs() -> [Catalog; 4] {
        let base = crate::tests::prepared(
            (1..=3)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Track {id}"),
                    ..Default::default()
                })
                .collect(),
        );
        let request = Request {
            scope: Scope::Find,
            query: Query::new("track").unwrap(),
            sort: Sort::Original,
            ascending: true,
        };
        let mut normal = base.clone();
        normal.project(request, vec![0, 1, 2]);
        let mut kept = base.clone();
        kept.prepare_pins(&mut |_| true);
        kept.prepare_tables().unwrap();
        kept.project(request, vec![0, 1, 2]);
        let mut blocked = normal.clone();
        std::sync::Arc::make_mut(blocked.view.as_mut().unwrap())
            .find
            .as_mut()
            .unwrap()
            .eligible = std::sync::Arc::from([]);
        let mut empty = base;
        empty.project(request, Vec::new());
        [normal, kept, blocked, empty]
    }
    fn build(
        ui: &mut Ui,
        catalog: &Catalog,
        state: &ShellState,
        intent: &mut Option<Intent>,
    ) -> Handle {
        let root = ui.node(
            "root",
            Node {
                w: Size::Px(400.0),
                h: Size::Px(45.0),
                padding: Edges {
                    l: 32.0,
                    r: 32.0,
                    ..Edges::ZERO
                },
                ..Node::DEFAULT
            },
        );
        ui.within(&root, &mut |ui| {
            toolbar(ui, catalog, state, palette(ui), intent)
        });
        root
    }
    fn part(ui: &Ui, root: Handle, action: &str, part: &str) -> Handle {
        let mut key = root.box_key;
        for label in [
            "find-tools",
            "buttons",
            action,
            "action-copy",
            "content",
            part,
        ] {
            key = Ui::key_of(key, label);
        }
        ui.box_of_key(key).unwrap()
    }
    #[test]
    fn find_tools_match_native_point_geometry_values_and_warm_action_routing() {
        mem_sys::set_peer(pxui::library_counter);
        let catalogs = catalogs();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Find));
        state.apply(Intent::Search(Query::new("track").unwrap()));
        for round in 0..8 {
            mem_sys::local_reset();
            for scale in [1.0, 2.0] {
                for (index, catalog) in catalogs.iter().enumerate() {
                    let mut intent = None;
                    let mut root = None;
                    let frame = FrameIn {
                        width: 400.0,
                        height: 100.0,
                        scale,
                        reduced_motion: 1,
                        ..Default::default()
                    };
                    ui.frame(&frame, &mut |ui| {
                        root = Some(build(ui, catalog, &state, &mut intent))
                    })
                    .unwrap();
                    assert_eq!(
                        ui.a11y_nodes().len(),
                        2,
                        "button labels and vector icons are not duplicate accessible controls"
                    );
                    let play = ui
                        .a11y_nodes()
                        .iter()
                        .find(|node| ui.a11y_name_of(node) == "Play all")
                        .unwrap();
                    let label = if index == 1 {
                        "Kept offline"
                    } else {
                        "Keep offline"
                    };
                    let offline = ui
                        .a11y_nodes()
                        .iter()
                        .find(|node| ui.a11y_name_of(node) == label)
                        .unwrap();
                    let play_width = if scale == 1.0 { 84.0 } else { 83.5 };
                    let offline_width = if index == 1 {
                        if scale == 1.0 { 112.0 } else { 111.5 }
                    } else if scale == 1.0 {
                        115.0
                    } else {
                        114.5
                    };
                    assert_eq!(
                        play.rect,
                        Rect {
                            x: 32.0,
                            y: 0.0,
                            w: play_width,
                            h: 30.0
                        }
                    );
                    assert_eq!(
                        offline.rect,
                        Rect {
                            x: 32.0 + play_width + 8.0,
                            y: 0.0,
                            w: offline_width,
                            h: 30.0
                        }
                    );
                    assert_eq!(
                        ui.a11y_value_of(offline),
                        Some(if index == 1 {
                            "Kept offline"
                        } else {
                            "Not kept offline"
                        })
                    );
                    let (play_id, offline_id) = (play.id, offline.id);
                    let enabled = pxui::a11y::state::ENABLED;
                    assert_eq!(play.states & enabled != 0, index != 3);
                    assert_eq!(offline.states & enabled != 0, index < 2);
                    for (action, left, size) in [
                        ("play-all", 32.0, 16.0),
                        ("keep-offline", 32.0 + play_width + 8.0, 18.0),
                    ] {
                        let icon = ui.rect(&part(&ui, root.unwrap(), action, "icon"));
                        assert_eq!(
                            icon,
                            Rect {
                                x: left + 10.0,
                                y: (30.0 - size) * 0.5,
                                w: size,
                                h: size
                            }
                        );
                        let copy = ui.rect(&part(&ui, root.unwrap(), action, "label"));
                        assert!(
                            (copy.x - left - 10.0 - size - 6.0).abs() < 0.001
                                && (copy.y - 5.9).abs() < 0.001
                                && (copy.h - 18.2).abs() < 0.001
                        );
                    }
                    assert!(
                        ui.layers()
                            .flat_map(|layer| layer.instances.iter())
                            .any(|quad| quad.rect == [0.0, 44.0, 400.0, 45.0]
                                && quad.color == [0xDA, 0xDA, 0xDA, 255]),
                        "full-width rule is inside the 15-point bottom padding"
                    );
                    for (id, expected) in [
                        (
                            play_id,
                            if index == 3 {
                                None
                            } else {
                                Some(Intent::PlayRow(0))
                            },
                        ),
                        (
                            offline_id,
                            if index < 2 {
                                Some(Intent::TogglePins)
                            } else {
                                None
                            },
                        ),
                    ] {
                        ui.inject_a11y(&[A11yAction {
                            id,
                            action_code: 1,
                            ..Default::default()
                        }]);
                        ui.frame(&frame, &mut |ui| {
                            build(ui, catalog, &state, &mut intent);
                        })
                        .unwrap();
                        assert_eq!(intent.take(), expected);
                    }
                }
            }
            state.apply(Intent::Search(Query::new("other").unwrap()));
            let frame = FrameIn {
                width: 400.0,
                height: 100.0,
                reduced_motion: 1,
                ..Default::default()
            };
            ui.frame(&frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut None);
            })
            .unwrap();
            assert!(
                ui.a11y_nodes()
                    .iter()
                    .all(|node| node.states & pxui::a11y::state::ENABLED == 0),
                "stale query responses cannot play or pin a newer search"
            );
            state.apply(Intent::Search(Query::new("track").unwrap()));
            if round >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm Find tools, kept/blocked/empty states, densities and accessible actions"
                );
            }
        }
    }
    #[test]
    fn find_tools_animate_only_backgrounds_and_settle_without_scaling_or_reshaping() {
        mem_sys::set_peer(pxui::library_counter);
        let catalogs = catalogs();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Find));
        state.apply(Intent::Search(Query::new("track").unwrap()));
        let mut root = None;
        let mut frame = FrameIn {
            width: 400.0,
            height: 100.0,
            ..Default::default()
        };
        for _ in 0..3 {
            ui.frame(&frame, &mut |ui| {
                root = Some(build(ui, &catalogs[0], &state, &mut None))
            })
            .unwrap();
        }
        let masks = ui
            .layers()
            .flat_map(|layer| layer.instances.iter())
            .filter(|quad| quad.kind == pxui::draw::kind::MASK)
            .copied()
            .collect::<Vec<_>>();
        let copy = part(&ui, root.unwrap(), "play-all", "label");
        let original = ui.rect(&copy);
        for round in 0..8 {
            mem_sys::local_reset();
            let base = round as f64;
            let down = [
                pxui::Event {
                    kind: input::wire::POINTER_MOVED,
                    x: 70.0,
                    y: 15.0,
                    ..Default::default()
                },
                pxui::Event {
                    kind: input::wire::POINTER_PRESSED,
                    x: 70.0,
                    y: 15.0,
                    ..Default::default()
                },
            ];
            frame.time = base;
            frame.events = down.as_ptr();
            frame.event_count = down.len();
            ui.frame(&frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut None);
            })
            .unwrap();
            frame.events = std::ptr::null();
            frame.event_count = 0;
            let mut settled_revision = None;
            for time in [0.07, 0.15, 0.3] {
                frame.time = base + time;
                ui.frame(&frame, &mut |ui| {
                    build(ui, &catalogs[0], &state, &mut None);
                })
                .unwrap();
                assert_eq!(
                    ui.rect(&copy),
                    original,
                    "native Find actions keep their text at full size while held"
                );
                assert!(
                    ui.layers()
                        .flat_map(|layer| layer.instances.iter())
                        .filter(|quad| quad.kind == pxui::draw::kind::MASK)
                        .copied()
                        .eq(masks.iter().copied()),
                    "background motion preserves cached glyph instances"
                );
                if let Some(previous) = settled_revision {
                    assert_eq!(
                        ui.draw_revision(),
                        previous,
                        "the 140ms native background transition stops changing after settling"
                    );
                }
                if time >= 0.14 {
                    settled_revision = Some(ui.draw_revision());
                }
            }
            let up = pxui::Event {
                kind: input::wire::POINTER_RELEASED,
                x: 70.0,
                y: 15.0,
                ..Default::default()
            };
            frame.events = &up;
            frame.event_count = 1;
            frame.time = base + 0.4;
            let mut intent = None;
            ui.frame(&frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut intent);
            })
            .unwrap();
            assert_eq!(intent, Some(Intent::PlayRow(0)));
            frame.events = std::ptr::null();
            frame.event_count = 0;
            for time in [0.47, 0.55] {
                frame.time = base + time;
                ui.frame(&frame, &mut |ui| {
                    build(ui, &catalogs[0], &state, &mut None);
                })
                .unwrap();
                assert_eq!(
                    ui.repaint_at().is_some(),
                    time < 0.54,
                    "the release transition draws for 140ms, then idles while hovered"
                );
            }
            let leave = pxui::Event {
                kind: input::wire::POINTER_MOVED,
                x: 390.0,
                y: 90.0,
                ..Default::default()
            };
            frame.events = &leave;
            frame.event_count = 1;
            frame.time = base + 0.6;
            ui.frame(&frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut None);
            })
            .unwrap();
            frame.events = std::ptr::null();
            frame.event_count = 0;
            frame.time = base + 0.8;
            ui.frame(&frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut None);
            })
            .unwrap();
            assert!(ui.output().repaint_at.is_infinite());
            if round >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm Find press/hover transitions"
                );
            }
        }
    }
}
