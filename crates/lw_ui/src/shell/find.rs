//! Hide stale-query results, retain editor focus through worker replies and reset scrolling on
//! query changes. Bounded UTF-8 query/chip edits preserve casing and reject overflow visibly
//! without truncating or losing accepted text. Actual playback starts invalidate Unplayed; overview
//! browse clears stale Find projections.
//!
//! The Play all/Keep offline toolbar uses 30-point buttons, 8-point gaps, 10-point side padding,
//! 16/18-point icons and 15-point bottom padding including its full-width rule. Ceil 13-point Host
//! label widths to physical pixels with distinct identities. Animate retained backgrounds for 140
//! ms without key-style scale. Clone prepared eligible ID Arcs into TogglePins; disable
//! stale/empty/blocked actions and leave copies/preferences on the worker.
//!
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Play,
    /// Folder pages: "Play folder".
    Folder,
    Shuffle,
    Offline(bool),
}

fn background(ui: &mut Ui, key: &'static str, color: Rgba, shown: bool, immediate: bool) {
    let view = ui.view_of(key, &0, |ui| {
        ui.node(
            "fill",
            Node::column().w(Size::Grow).h(Size::Grow).look(Look::NONE.fill(color)),
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
pub(super) fn button(ui: &mut Ui, action: Action, enabled: bool) -> bool {
    let (key, label, icon_size, selected) = match action {
        Action::Play => ("play-all", "Play all", 16.0, false),
        Action::Folder => ("play-folder", "Play folder", 16.0, false),
        Action::Shuffle => ("shuffle", "Shuffle", 16.0, false),
        Action::Offline(kept) => (
            "keep-offline",
            if kept { "Kept offline" } else { "Keep offline" },
            18.0,
            kept,
        ),
    };
    const TOOL_LABEL: pxui::Label = pxui::Label::BODY.size(13.0).line_height(18.2).lines(1).ellipsis();
    let density = ui.density();
    let label_width =
        (ui.text_service().width(label, TOOL_LABEL, None).unwrap() * density).ceil() / density;
    let width = 20.0 + icon_size + 6.0 + label_width;
    let tool = pxui::Button::sized(width, 30.0).look(if selected { &SELECTED_LOOKS } else { &PAPER_LOOKS }).disabled(!enabled).name(label);
    // the hover and press washes sit under the copy, so the contents are a stack built with the press state
    let response = ui.button_content(key, &tool, |ui, press| {
        let layers = ui.node("layers", Node::stack().w(Size::Grow).h(Size::Grow));
        let before = ui.state(&layers, -1);
        ui.set_state(&layers, i64::from(selected));
        ui.within(&layers, &mut |ui| {
            background(ui, "hover", ui.color(PANEL), press.hovered && !selected, before != i64::from(selected));
            background(ui, "press", ui.color(PANEL), press.pressed, false);
            ui.view_of("action-copy", &(label, selected), |ui| {
                // a keyed node, not a keyless row: the tests look the icon up under it by key
                let content = ui.node("content", Node::row().w(Size::Grow).h(Size::Grow).gap(6.0).pad_x(10.0).align(Align::Center));
                ui.within(&content, &mut |ui| {
                    let color = if selected { ui.color(INK) } else { ui.color(SECONDARY) };
                    let icon = ui.node("icon", Node::column().w_px(icon_size).h_px(icon_size));
                    ui.canvas(&icon, move |paint, rect| match action {
                        Action::Play | Action::Folder => crate::icons::play(paint, rect, color),
                        Action::Shuffle => crate::icons::shuffle(paint, rect, color),
                        Action::Offline(kept) => crate::icons::offline(paint, rect, kept, color),
                    });
                    let _copy = ui.label(label, TEXT_13.w_px(label_width).color(if selected { INK } else { SECONDARY }).role(Role::Unknown));
                    #[cfg(test)]
                    tests::copied(key, _copy);
                });
            });
        });
    });
    if matches!(action, Action::Offline(_)) {
        ui.a11y_value(&response.handle, if selected { "Kept offline" } else { "Not kept offline" }, (0, 0))
            .expect("valid offline status");
    }
    enabled && response.clicked
}

pub(super) fn toolbar(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    intent: &mut Option<Intent>,
) {
    let collection = state.collection(catalog);
    let summary = collection
        .and(catalog.view.as_ref())
        .and_then(|view| view.find.as_ref());
    let tools = ui.node("find-tools", Node::column().w(Size::Grow).h_px(45.0));
    ui.within(&tools, &mut |ui| {
        // keyed: the tests look the buttons' parts up by key under it
        let buttons = ui.node("buttons", Node::row().w(Size::Grow).h_px(30.0).gap(8.0));
        ui.within(&buttons, &mut |ui| {
            if button(
                ui,
                Action::Play,
                collection.is_some_and(|group| !group.ids.is_empty()),
            ) {
                *intent = Some(Intent::PlayRow(0));
            }
            if button(
                ui,
                Action::Offline(summary.is_some_and(|value| value.pinned)),
                summary.is_some_and(|value| !value.eligible.is_empty()),
            ) {
                *intent = Some(Intent::TogglePins);
            }
        });
    });
    ui.canvas_over(&tools, move |paint, rect| {
        paint.fill(Rect::new(rect.x - 32.0, rect.y + rect.h - 1.0, rect.w + 64.0, 1.0), Fill::new(paint.color(LINE)).fractional())
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
            commands: None,
            scope: Scope::Find,
            query: Query::new("track").unwrap(),
            sort: Sort::Original,
            ascending: true,
        };
        let mut normal = base.clone();
        normal.project(request, vec![0, 1, 2]);
        let mut kept = base.clone();
        kept.prepare_pins(&mut |_| true);
        kept.prepare_tables(&crate::test_table_styles()).unwrap();
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
        let root = ui.node("root", Node::column().w_px(400.0).h_px(45.0).pad_x(32.0));
        ui.within(&root, &mut |ui| {
            toolbar(ui, catalog, state, intent)
        });
        root
    }
    /// A keyed part of a Find action's copy by its key path.
    fn part(ui: &Ui, root: Handle, action: &str, part: &str) -> Handle {
        let mut key = root.box_key;
        for label in ["find-tools", "buttons", action, "layers", "action-copy", "content", part] {
            key = pxui::ui::key_of(key, label);
        }
        ui.box_of_key(pxui::Key::Resolved(key)).unwrap()
    }
    thread_local! {
        /// The copy label each action built last (play-all, keep-offline): a label has no key to look it up by.
        static COPIES: std::cell::Cell<[Option<Handle>; 2]> = const { std::cell::Cell::new([None; 2]) };
    }
    /// Records the copy `button` built for `key`.
    pub(super) fn copied(key: &str, copy: Handle) {
        let Some(slot) = ["play-all", "keep-offline"].iter().position(|k| *k == key) else { return };
        COPIES.with(|copies| {
            let mut all = copies.get();
            all[slot] = Some(copy);
            copies.set(all);
        });
    }
    /// The copy label of `action` (play-all or keep-offline) from the last frame.
    fn copy(action: &str) -> Handle {
        let slot = ["play-all", "keep-offline"].iter().position(|k| *k == action).unwrap();
        COPIES.with(|copies| copies.get()[slot]).expect("the copy was built")
    }
    #[test]
    fn find_tools_match_native_point_geometry_values_and_warm_action_routing() {
        mem_sys::set_peer(pxui::library_counter);
        let catalogs = catalogs();
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
                    let frame = FrameIn::new(400.0, 100.0).scale(scale).reduced_motion(true);
                    crate::testing::frame(&mut ui, &frame, &mut |ui| {
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
                    let enabled = pxui::A11yStates::ENABLED.bits();
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
                        let copy = ui.rect(&copy(action));
                        assert!(
                            (copy.x - left - 10.0 - size - 6.0).abs() < 0.001
                                && (copy.y - 5.9).abs() < 0.001
                                && (copy.h - 18.2).abs() < 0.001,
                            "{action}: the copy sits 6 points after the icon in its 18.2-point line box at 5.9, got {copy:?}"
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
                        crate::testing::frame(&mut ui, &frame, &mut |ui| {
                            build(ui, catalog, &state, &mut intent);
                        })
                        .unwrap();
                        assert_eq!(intent.take(), expected);
                    }
                }
            }
            state.apply(Intent::Search(Query::new("other").unwrap()));
            let frame = FrameIn::new(400.0, 100.0).reduced_motion(true);
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut None);
            })
            .unwrap();
            assert!(
                ui.a11y_nodes()
                    .iter()
                    .all(|node| node.states & pxui::A11yStates::ENABLED.bits() == 0),
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
    fn search_results_page_replaces_the_field_until_find_is_navigated_again() {
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Find));
        state.apply(Intent::Search(Query::new("a").unwrap()));
        assert!(!state.find_results, "typing in the Find field keeps it editable");
        state.apply(Intent::SearchResults(Query::new("cloud").unwrap()));
        assert!(state.find_results && state.query.as_str() == "cloud");
        state.apply(Intent::Navigate(Route::Find));
        assert!(!state.find_results && state.query.as_str() == "cloud");
    }
    #[test]
    fn find_tools_animate_only_backgrounds_and_settle_without_scaling_or_reshaping() {
        mem_sys::set_peer(pxui::library_counter);
        let catalogs = catalogs();
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Find));
        state.apply(Intent::Search(Query::new("track").unwrap()));
        let mut root = None;
        let mut frame = FrameIn::new(400.0, 100.0);
        for _ in 0..3 {
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
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
        let copy = copy("play-all");
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
            frame.wire.time = base;
            frame.wire.events = down.as_ptr();
            frame.wire.event_count = down.len();
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut None);
            })
            .unwrap();
            frame.wire.events = std::ptr::null();
            frame.wire.event_count = 0;
            let mut settled_revision = None;
            for time in [0.07, 0.15, 0.3] {
                frame.wire.time = base + time;
                crate::testing::frame(&mut ui, &frame, &mut |ui| {
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
            frame.wire.events = &up;
            frame.wire.event_count = 1;
            frame.wire.time = base + 0.4;
            let mut intent = None;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut intent);
            })
            .unwrap();
            assert_eq!(intent, Some(Intent::PlayRow(0)));
            frame.wire.events = std::ptr::null();
            frame.wire.event_count = 0;
            for time in [0.47, 0.55] {
                frame.wire.time = base + time;
                crate::testing::frame(&mut ui, &frame, &mut |ui| {
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
            frame.wire.events = &leave;
            frame.wire.event_count = 1;
            frame.wire.time = base + 0.6;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut None);
            })
            .unwrap();
            frame.wire.events = std::ptr::null();
            frame.wire.event_count = 0;
            frame.wire.time = base + 0.8;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
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
