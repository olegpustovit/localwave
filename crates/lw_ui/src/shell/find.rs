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

/// The Keep offline tool key, checked while `kept`.
pub(super) fn keep_offline(kept: bool) -> pxui::Button<'static> {
    TOOL.icon(icon_drawing(if kept { Icon::Kept } else { Icon::Offline }))
        .icon_size(18.0)
        .selected(kept)
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
            if ui
                .button(
                    "play-all",
                    "Play all",
                    &TOOL
                        .icon(icon_drawing(Icon::Play))
                        .icon_size(16.0)
                        .disabled(!collection.is_some_and(|group| !group.ids.is_empty())),
                )
                .clicked
            {
                *intent = Some(Intent::PlayRow(0));
            }
            let kept = summary.is_some_and(|value| value.pinned);
            if ui
                .button(
                    "keep-offline",
                    if kept { "Kept offline" } else { "Keep offline" },
                    &keep_offline(kept)
                        .disabled(!summary.is_some_and(|value| !value.eligible.is_empty())),
                )
                .clicked
            {
                *intent = Some(Intent::TogglePins);
            }
        });
    });
    ui.canvas_over(&tools, move |paint, rect| {
        paint.fill(
            Rect::new(rect.x - 32.0, rect.y + rect.h - 1.0, rect.w + 64.0, 1.0),
            Fill::new(Rgba::token(LINE)).fractional(),
        )
    });
}

#[cfg(test)]
pub(in crate::shell) mod tests {
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
        ui.within(&root, &mut |ui| toolbar(ui, catalog, state, intent));
        root
    }
    thread_local! {
        /// The icon and label the tool body built last for Play all and Keep offline: neither has a key to look it up by.
        static PARTS: std::cell::Cell<[Option<(Handle, Handle)>; 2]> = const { std::cell::Cell::new([None; 2]) };
    }
    /// Records the icon and label `localwave-button` built for the tool named `text`.
    pub(in crate::shell) fn tool_built(text: &str, icon: Handle, copy: Handle) {
        let Some(slot) = ["Play all", "Keep offline", "Kept offline"]
            .iter()
            .position(|k| *k == text)
        else {
            return;
        };
        PARTS.with(|parts| {
            let mut all = parts.get();
            all[slot.min(1)] = Some((icon, copy));
            parts.set(all);
        });
    }
    /// The icon and label of `action` (play-all or keep-offline) from the last frame.
    fn parts(action: &str) -> (Handle, Handle) {
        let slot = ["play-all", "keep-offline"]
            .iter()
            .position(|k| *k == action)
            .unwrap();
        PARTS
            .with(|parts| parts.get()[slot])
            .expect("the tool was built")
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
                    let frame = FrameIn::new(400.0, 100.0).scale(scale).reduced_motion(true);
                    crate::testing::frame(&mut ui, &frame, &mut |ui| {
                        build(ui, catalog, &state, &mut intent);
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
                        offline.states & pxui::A11yStates::SELECTED.bits() != 0,
                        index == 1,
                        "Keep offline is selected while kept (`Button::selected`)"
                    );
                    let (play_id, offline_id) = (play.id, offline.id);
                    let enabled = pxui::A11yStates::ENABLED.bits();
                    assert_eq!(play.states & enabled != 0, index != 3);
                    assert_eq!(offline.states & enabled != 0, index < 2);
                    for (action, left, size) in [
                        ("play-all", 32.0, 16.0),
                        ("keep-offline", 32.0 + play_width + 8.0, 18.0),
                    ] {
                        let (icon, copy) = parts(action);
                        let icon = ui.rect(&icon);
                        assert_eq!(
                            icon,
                            Rect {
                                x: left + 10.0,
                                y: (30.0 - size) * 0.5,
                                w: size,
                                h: size
                            }
                        );
                        let copy = ui.rect(&copy);
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
        assert!(
            !state.find_results,
            "typing in the Find field keeps it editable"
        );
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
        let mut frame = FrameIn::new(400.0, 100.0);
        for _ in 0..3 {
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut None);
            })
            .unwrap();
        }
        let masks = ui
            .layers()
            .flat_map(|layer| layer.instances.iter())
            .filter(|quad| quad.kind == pxui::draw::kind::MASK)
            .copied()
            .collect::<Vec<_>>();
        let copy = parts("play-all").1;
        let original = ui.rect(&copy);
        for round in 0..8 {
            mem_sys::local_reset();
            let base = round as f64;
            let down = [
                pxui::Event::pointer(input::wire::POINTER_MOVED, 0, 70.0, 15.0),
                pxui::Event::pointer(input::wire::POINTER_PRESSED, 0, 70.0, 15.0),
            ];
            frame.wire.time = base;
            let mut frame = frame;
            frame = frame.events(&down[..]);
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut None);
            })
            .unwrap();
            frame = frame.events(&[]);
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
            let up = pxui::Event::pointer(input::wire::POINTER_RELEASED, 0, 70.0, 15.0);
            let mut frame = frame;
            frame = frame.events(std::slice::from_ref(&up));
            frame.wire.time = base + 0.4;
            let mut intent = None;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut intent);
            })
            .unwrap();
            assert_eq!(intent, Some(Intent::PlayRow(0)));
            frame = frame.events(&[]);
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
            let leave = pxui::Event::pointer(input::wire::POINTER_MOVED, 0, 390.0, 90.0);
            let mut frame = frame;
            frame = frame.events(std::slice::from_ref(&leave));
            frame.wire.time = base + 0.6;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                build(ui, &catalogs[0], &state, &mut None);
            })
            .unwrap();
            frame = frame.events(&[]);
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
