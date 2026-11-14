use super::*;
use crate::browse::AlbumSort;

pub(super) fn toolbar(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let row = ui.node(
        "album-tools",
        Node {
            axis: Axis::Row,
            gap: 8.0,
            w: Size::Grow,
            h: Size::Px(30.0),
            ..Node::DEFAULT
        },
    );
    ui.within(&row, &mut |ui| {
        for (value, title) in [
            (AlbumSort::Artist, "Artist"),
            (AlbumSort::Year, "Year"),
            (AlbumSort::Added, "Size"),
        ] {
            let selected = state.album_sort == value;
            let button = ui.node(
                title,
                Node {
                    flags: CLICKABLE | TAB_STOP,
                    role: Role::Button,
                    w: Size::Fit,
                    h: Size::Px(30.0),
                    align: Align::Center,
                    justify: Justify::Center,
                    padding: Edges {
                        l: 10.0,
                        r: 10.0,
                        ..Edges::ZERO
                    },
                    look: Look {
                        fill: if selected { p.selected } else { p.paper },
                        ..Look::NONE
                    },
                    ..Node::DEFAULT
                },
            );
            ui.a11y_name(&button, title);
            ui.set_role(
                &button,
                Role::Button,
                if selected {
                    pxui::a11y::state::SELECTED
                } else {
                    0
                },
            );
            ui.within(&button, &mut |ui| {
                text(
                    ui,
                    "label",
                    title,
                    13,
                    if selected { p.ink } else { p.secondary },
                    (Size::Fit, 18.2),
                    false,
                );
            });
            if selected {
                ui.draw_over(&button, move |paint, rect| {
                    paint.fill_fractional(rect.x, rect.y, rect.w, 1.5, p.ink);
                    paint.fill_fractional(rect.x, rect.y + rect.h - 1.5, rect.w, 1.5, p.ink);
                    paint.fill_fractional(rect.x, rect.y + 1.5, 1.5, rect.h - 3.0, p.ink);
                    paint.fill_fractional(
                        rect.x + rect.w - 1.5,
                        rect.y + 1.5,
                        1.5,
                        rect.h - 3.0,
                        p.ink,
                    );
                });
            }
            if ui.signal_state(&button).clicked {
                *intent = Some(Intent::SortAlbums(value));
            }
        }
        ui.box_("flex", BoxOpts::default().w(Size::Grow));
        let button = ui.node(
            "shuffle-all",
            Node {
                axis: Axis::Row,
                align: Align::Center,
                gap: 6.0,
                flags: if catalog.is_empty() {
                    0
                } else {
                    CLICKABLE | TAB_STOP
                },
                state: State {
                    disabled: catalog.is_empty(),
                    ..State::default()
                },
                role: Role::Button,
                w: Size::Fit,
                h: Size::Px(30.0),
                padding: Edges {
                    l: 10.0,
                    r: 10.0,
                    ..Edges::ZERO
                },
                ..Node::DEFAULT
            },
        );
        ui.a11y_name(&button, "Shuffle all");
        ui.within(&button, &mut |ui| {
            let icon = ui.node(
                "icon",
                Node {
                    w: Size::Px(16.0),
                    h: Size::Px(16.0),
                    ..Node::DEFAULT
                },
            );
            ui.draw(&icon, move |paint, rect| {
                crate::icons::shuffle(paint, rect, p.secondary)
            });
            text(
                ui,
                "label",
                "Shuffle all",
                13,
                p.secondary,
                (Size::Fit, 18.2),
                false,
            );
        });
        if ui.signal_state(&button).clicked {
            *intent = Some(Intent::ShuffleAll);
        }
    });
    spacer(ui, "album-tools-bottom", 13.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::{FrameIn, Platform};

    #[test]
    fn album_toolbar_positions_sorts_routes_and_reuses_warm_frames() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = Catalog::new(vec![
            lw_core::Song {
                id: Some(1),
                artist: "Alice".into(),
                album: "B".into(),
                release_year: Some(2019),
                created_at: 300.0,
                ..Default::default()
            },
            lw_core::Song {
                id: Some(2),
                artist: "The Bee".into(),
                album: "A".into(),
                release_year: Some(2022),
                created_at: 100.0,
                ..Default::default()
            },
            lw_core::Song {
                id: Some(3),
                artist: "Cedar".into(),
                album: "C".into(),
                created_at: 200.0,
                ..Default::default()
            },
        ]);
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Albums));
        let generation = state.entry_revision;
        let player = PlayerView::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            reduced_motion: 1,
            ..Default::default()
        };
        let find = |ui: &Ui, name: &str| {
            *ui.a11y_nodes()
                .iter()
                .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == name)
                .unwrap()
        };
        for _ in 0..3 {
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut None)
            })
            .unwrap();
        }
        let mut right = 256.0;
        for (name, expected_width) in [("Artist", 52.0), ("Year", 47.0), ("Size", 44.0)] {
            let button = find(&ui, name);
            assert!(
                (button.rect.x - right - 8.0).abs() < 0.001,
                "8-point gap and 32-point content inset: {name} {:?}",
                button.rect
            );
            assert!(
                (button.rect.y - 105.6).abs() < 0.001 && button.rect.h == 30.0,
                "{name} {:?}",
                button.rect
            );
            assert!(
                (button.rect.w - expected_width).abs() < 0.001,
                "native SwiftUI control width {name}: {:?}",
                button.rect
            );
            right = button.rect.x + button.rect.w;
        }
        let shuffle = find(&ui, "Shuffle all");
        assert!(
            (shuffle.rect.w - 100.0).abs() < 0.001,
            "native SwiftUI shuffle width: {:?}",
            shuffle.rect
        );
        assert!((shuffle.rect.x + shuffle.rect.w - 1088.0).abs() < 0.001);
        let art = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Album artwork")
            .unwrap()
            .rect;
        assert!(
            (art.y - 172.6).abs() < 0.001,
            "toolbar plus header bottom line/gap: {art:?}"
        );
        assert!(
            !ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Refresh library")
        );
        for cycle in 0..10 {
            mem_sys::local_reset();
            for scale in [1.0, 2.0] {
                frame.scale = scale;
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut None)
                })
                .unwrap();
                let widths = if scale == 1.0 {
                    [52.0, 47.0, 44.0]
                } else {
                    [52.0, 46.5, 44.0]
                };
                let xs = if scale == 1.0 {
                    [264.0, 324.0, 379.0]
                } else {
                    [264.0, 324.0, 378.5]
                };
                for (index, name) in ["Artist", "Year", "Size"].into_iter().enumerate() {
                    let rect = find(&ui, name).rect;
                    assert!(
                        (rect.x - xs[index]).abs() < 0.001
                            && (rect.w - widths[index]).abs() < 0.001,
                        "native SwiftUI at {scale}x: {name} {rect:?}"
                    );
                }
                for (name, sort, order) in [
                    ("Artist", AlbumSort::Artist, ["B", "A", "C"]),
                    ("Year", AlbumSort::Year, ["A", "B", "C"]),
                    ("Size", AlbumSort::Added, ["B", "C", "A"]),
                ] {
                    ui.inject_a11y(&[pxui::A11yAction {
                        id: find(&ui, name).id,
                        action_code: 1,
                        ..Default::default()
                    }]);
                    frame.time += 0.1;
                    let mut intent = None;
                    ui.frame(&frame, &mut |ui| {
                        shell(ui, &player, &catalog, &state, &mut intent)
                    })
                    .unwrap();
                    assert_eq!(intent, Some(Intent::SortAlbums(sort)));
                    assert!(state.apply(intent.unwrap()));
                    ui.frame(&frame, &mut |ui| {
                        shell(ui, &player, &catalog, &state, &mut None)
                    })
                    .unwrap();
                    assert_ne!(find(&ui, name).states & pxui::a11y::state::SELECTED, 0);
                    assert!(
                        find(&ui, order[0]).rect.x < find(&ui, order[1]).rect.x
                            && find(&ui, order[1]).rect.x < find(&ui, order[2]).rect.x
                    );
                    assert_eq!(
                        state.entry_revision, generation,
                        "sorting must not restart page entrances"
                    );
                }
                ui.inject_a11y(&[pxui::A11yAction {
                    id: find(&ui, "Shuffle all").id,
                    action_code: 1,
                    ..Default::default()
                }]);
                let mut intent = None;
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut intent)
                })
                .unwrap();
                assert_eq!(intent, Some(Intent::ShuffleAll));
            }
            if cycle >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm album toolbar sorting/navigation"
                );
            }
        }
        state.apply(Intent::Navigate(Route::Songs));
        state.apply(Intent::Navigate(Route::Albums));
        assert_eq!(
            state.album_sort,
            AlbumSort::Added,
            "the root's album sort survives navigation"
        );
    }
}
