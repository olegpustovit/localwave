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

pub(super) fn gallery(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let mut revision = DefaultHasher::new();
    (
        state.revision,
        player.current_id,
        player.artwork.map_or(0, crate::artwork::Images::revision),
        std::sync::Arc::as_ptr(&catalog.library) as usize,
    )
        .hash(&mut revision);
    let row_revision = revision.finish();
    let entry = crate::motion::Page::new(ui, state.entry_revision);
    let groups = &catalog.albums;
    let order = &catalog.album_orders[state.album_sort as usize];
    let compact = ui.view_size().0 < compact::BREAKPOINT;
    let inset = if compact { 16.0 } else { 32.0 };
    let gap = if compact { 16.0 } else { 20.0 };
    let available = (width - inset * 2.0).max(1.0);
    let columns = ((available + gap) / if compact { 156.0 } else { 170.0 })
        .floor()
        .max(1.0) as usize;
    let cell = (available - (columns - 1) as f32 * gap) / columns as f32;
    let card_height = cell + 43.7;
    ui.section_list(
        "groups",
        &SectionList {
            count: groups.len().div_ceil(columns),
            row_height: card_height + 28.0,
            leading_height: if compact { 16.0 } else { 172.6 },
            header_height: 0.0,
            trailing_height: 12.0,
            overlay_scrollbar: true,
        },
        &mut |ui, part| match part {
            SectionPart::Leading => {
                if !compact {
                    header_view(ui, catalog, state, width, p, intent);
                    spacer(ui, "gallery-top-gap", 22.0);
                }
            }
            SectionPart::Header => {}
            SectionPart::Row(line) => {
                let inset = ui.node(
                    "gallery-inset",
                    Node {
                        w: Size::Grow,
                        h: Size::Grow,
                        padding: Edges {
                            l: inset,
                            r: inset,
                            ..Edges::ZERO
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&inset, &mut |ui| {
                    let row = ui.node(
                        "group-row",
                        Node {
                            axis: Axis::Row,
                            gap,
                            w: Size::Grow,
                            h: Size::Grow,
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&row, &mut |ui| {
                        for (column, index) in order
                            .iter()
                            .copied()
                            .skip(line * columns)
                            .take(columns)
                            .enumerate()
                        {
                            let holder = ui.node_keyed(
                                int_key(row.box_key, index as i64),
                                Node {
                                    w: Size::Px(cell),
                                    h: Size::Px(card_height),
                                    ..Node::DEFAULT
                                },
                            );
                            ui.within(&holder, &mut |ui| {
                                let group = &groups[index];
                                let visual = ui.view(
                                    "group-visual",
                                    row_revision,
                                    Size::Grow,
                                    Size::Grow,
                                    &mut |ui| {
                                        let node = ui.node(
                                            "group",
                                            Node {
                                                flags: CLICKABLE | TAB_STOP,
                                                w: Size::Grow,
                                                h: Size::Grow,
                                                role: Role::Button,
                                                padding: Edges::ZERO,
                                                ..Node::DEFAULT
                                            },
                                        );
                                        ui.a11y_name(&node, &group.title);
                                        let signal = ui.signal_state(&node);
                                        ui.set_look(
                                            &node,
                                            Look {
                                                fill: p.paper,
                                                ..Look::NONE
                                            },
                                            State::default(),
                                        );
                                        ui.within(&node, &mut |ui| {
                                            let art = container(
                                                ui,
                                                "artwork",
                                                Axis::Stack,
                                                Size::Grow,
                                                Size::Px(cell),
                                                p.panel,
                                            );
                                            paint_artwork(
                                                ui,
                                                &art,
                                                group.artwork_row.map(|index| catalog.row(index)),
                                                cell,
                                                player.artwork,
                                            );
                                            ui.a11y_name(&art, "Album artwork");
                                            spacer(ui, "artwork-label-gap", 8.0);

                                            let name = text(
                                                ui,
                                                "name",
                                                &group.title,
                                                14,
                                                p.ink,
                                                (Size::Grow, 17.5),
                                                false,
                                            );
                                            ui.text_line_box(
                                                &name,
                                                &group.title,
                                                &pxui::TextStyle {
                                                    size_points: 14.0,
                                                    line_height: 17.5,
                                                    weight: 500,
                                                    max_lines: 1,
                                                    ellipsis: pxui::TextStyle::ELLIPSIS_END,
                                                    ..Default::default()
                                                },
                                                &[],
                                            )
                                            .unwrap();
                                            text(
                                                ui,
                                                "artist",
                                                &group.subtitle,
                                                13,
                                                p.secondary,
                                                (Size::Grow, 18.2),
                                                false,
                                            );
                                        });
                                        if signal.clicked {
                                            *intent = Some(Intent::Navigate(Route::Album(index)));
                                        }
                                    },
                                );
                                entry.row(ui, &visual, line * columns + column);
                            });
                        }
                    });
                });
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::{FrameIn, Platform};

    #[test]
    fn album_heading_and_toolbar_scroll_with_virtual_cards_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = crate::tests::prepared(
            (0..10_000)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Track {id}"),
                    artist: "Artist".into(),
                    album: format!("Album {id:05}"),
                    ..Default::default()
                })
                .collect(),
        );
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Albums));
        let player = PlayerView::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            reduced_motion: 1,
            ..Default::default()
        };
        for _ in 0..3 {
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut None)
            })
            .unwrap();
        }
        let list = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::List) && node.rect.x >= 232.0 && node.rect.w > 500.0)
            .unwrap();
        assert_eq!(
            (list.rect.x, list.rect.y, list.rect.w, list.rect.h),
            (232.0, 0.0, 888.0, 800.0)
        );
        let id = list.id;
        for cycle in 0..10 {
            mem_sys::local_reset();
            for delta in [200.0, -200.0] {
                ui.inject_a11y(&[pxui::A11yAction {
                    id,
                    action_code: 8,
                    payload: delta,
                    ..Default::default()
                }]);
                frame.time += 0.1;
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut None)
                })
                .unwrap();
                let offset = if delta > 0.0 { 200.0 } else { 0.0 };
                let title = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| {
                        node.is_role(Role::Label)
                            && ui.a11y_name_of(node) == "Albums"
                            && node.rect.x >= 232.0
                    })
                    .unwrap();
                assert!(
                    (title.rect.y - 28.0 + offset).abs() < 0.001,
                    "scrolling page title: {:?}",
                    title.rect
                );
                let sort = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == "Year")
                    .unwrap();
                assert!(
                    (sort.rect.y - 105.6 + offset).abs() < 0.001,
                    "toolbar must scroll out: {:?}",
                    sort.rect
                );
                let art = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| ui.a11y_name_of(node) == "Album artwork")
                    .unwrap();
                assert!(
                    (art.rect.y - 172.6 + offset).abs() < 0.001,
                    "cards share the same scroll offset: {:?}",
                    art.rect
                );
                let drawn = ui
                    .a11y_nodes()
                    .iter()
                    .filter(|node| ui.a11y_name_of(node) == "Album artwork")
                    .count();
                assert!(
                    (8..40).contains(&drawn),
                    "only visible cards plus overscan: {drawn}"
                );
            }
            if cycle >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm 10,000-album page/header/card scrolling"
                );
            }
        }
    }

    #[test]
    fn album_toolbar_positions_sorts_routes_and_reuses_warm_frames() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = crate::tests::prepared(vec![
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
