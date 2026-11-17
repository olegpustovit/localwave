//! Cards use a 150-point adaptive minimum, 20-point column gap, square artwork and 28-point row
//! gap. The toolbar uses 30-point Artist/Year/Size controls, 8-point gaps, a fractional 1.5-point
//! selected border, 14-point bottom padding and one-point header clearance. Without a window inset
//! first artwork begins at y=172.6; preserve native control widths 52/47/44/100 and fractional row
//! stride. Explicit CoreText column widths remain fractional; intrinsic Text widths ceil to
//! physical pixels.
//!
use super::*;
use crate::browse::AlbumSort;

pub(super) fn toolbar(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    intent: &mut Option<Intent>,
) {
    let row = ui.node("album-tools".into(), Node::row().w(Size::Grow).h_px(30.0).gap(8.0));
    ui.within(&row, &mut |ui| {
        for (value, key, title) in [
            (AlbumSort::Artist, "sort-artist", "Artist"),
            (AlbumSort::Year, "sort-year", "Year"),
            (AlbumSort::Added, "sort-added", "Size"),
        ] {
            let selected = state.album_sort == value;
            let node = Node::column().w(Size::Fit).h_px(30.0).pad_x(10.0).center().role(Role::Button);
            let button = ui.pressable(key, node, &PLAIN.look(if selected { &SELECTED_LOOKS } else { &PAPER_LOOKS }), |ui, _| {
                ui.label(title, TEXT_13.color(if selected { INK } else { SECONDARY }));
            });
            ui.a11y_name(&button.handle, title);
            ui.set_role(
                &button.handle,
                Role::Button,
                if selected {
                    pxui::a11y::state::SELECTED
                } else {
                    0
                },
            );
            if selected {
                ui.draw_over(&button.handle, move |paint, rect| {
                    paint.fill_fractional(rect.x, rect.y, rect.w, 1.5, paint.color(INK));
                    paint.fill_fractional(rect.x, rect.y + rect.h - 1.5, rect.w, 1.5, paint.color(INK));
                    paint.fill_fractional(rect.x, rect.y + 1.5, 1.5, rect.h - 3.0, paint.color(INK));
                    paint.fill_fractional(
                        rect.x + rect.w - 1.5,
                        rect.y + 1.5,
                        1.5,
                        rect.h - 3.0,
                        paint.color(INK),
                    );
                });
            }
            if button.clicked {
                *intent = Some(Intent::SortAlbums(value));
            }
        }
        ui.box_("flex", BoxOpts::default().w(Size::Grow));
        let node = Node::row().w(Size::Fit).h_px(30.0).gap(6.0).pad_x(10.0).align(Align::Center).role(Role::Button);
        let button = ui.pressable("shuffle-all", node, &PLAIN.enabled(!catalog.is_empty()), |ui, _| {
            let icon = ui.node("icon".into(), Node::column().w_px(16.0).h_px(16.0));
            ui.draw(&icon, move |paint, rect| {
                crate::icons::shuffle(paint, rect, paint.color(SECONDARY))
            });
            ui.label("Shuffle all", CAPTION);
        });
        ui.a11y_name(&button.handle, "Shuffle all");
        if button.clicked {
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
    intent: &mut Option<Intent>,
) {
    if catalog.all.is_some() && catalog.is_empty() {
        onboarding::empty_library(ui, catalog, state, width, intent);
        return;
    }
    let row_revision = (
        state.revision,
        player.current_id,
        player.artwork.map_or(0, crate::artwork::Images::revision),
        std::sync::Arc::as_ptr(&catalog.library) as usize,
    );
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
        &SectionList::new(groups.len().div_ceil(columns)).row_h(card_height + 28.0).lead(if compact { 16.0 } else { 172.6 }).trail(12.0).overlay_scrollbar(),
        // a row is a line of cards
        &|line| line as i64,
        &mut |ui, part, _| match part {
            SectionPart::Leading => {
                if !compact {
                    header_view(ui, catalog, state, width, intent);
                    spacer(ui, "gallery-top-gap", 22.0);
                }
            }
            SectionPart::Header => {}
            SectionPart::Row(line) => {
                ui.column("gallery-inset", Node::column().w(Size::Grow).h(Size::Grow).pad_x(inset), |ui| {
                    let row = ui.node("group-row".into(), Node::row().w(Size::Grow).h(Size::Grow).gap(gap));
                    ui.within(&row, &mut |ui| {
                        for (column, index) in order
                            .iter()
                            .copied()
                            .skip(line * columns)
                            .take(columns)
                            .enumerate()
                        {
                            let holder = ui.node(
                                Key::Id(index as i64),
                                Node::column().w_px(cell).h_px(card_height),
                            );
                            ui.within(&holder, &mut |ui| {
                                let group = &groups[index];
                                let visual = ui.view_of(
                                    "group-visual",
                                    &row_revision,
                                    |ui| {
                                        let node = Node {
                                            padding: Edges::ZERO,
                                            ..Node::column().w(Size::Grow).h(Size::Grow).role(Role::Button)
                                        };
                                        let card = ui.pressable("group", node, &PLAIN.look(&PAPER_LOOKS), |ui, _| {
                                            let art = container(
                                                ui,
                                                "artwork",
                                                Axis::Stack,
                                                Size::Grow,
                                                Size::Px(cell),
                                                ui.color(PANEL),
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

                                            ui.label(&group.title, TEXT_14.line_height(17.5).weight(500).grow().h_px(17.5).color(INK));
                                            ui.label(&group.subtitle, CAPTION.grow());
                                        });
                                        ui.a11y_name(&card.handle, &group.title);
                                        if card.clicked {
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
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            reduced_motion: 1,
            ..Default::default()
        };
        for _ in 0..3 {
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, &state, &mut None)
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
                    shell(ui, &player, 0.0, &catalog, &state, &mut None)
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
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
                shell(ui, &player, 0.0, &catalog, &state, &mut None)
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
                    shell(ui, &player, 0.0, &catalog, &state, &mut None)
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
                        shell(ui, &player, 0.0, &catalog, &state, &mut intent)
                    })
                    .unwrap();
                    assert_eq!(intent, Some(Intent::SortAlbums(sort)));
                    assert!(state.apply(intent.unwrap()));
                    ui.frame(&frame, &mut |ui| {
                        shell(ui, &player, 0.0, &catalog, &state, &mut None)
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
                    shell(ui, &player, 0.0, &catalog, &state, &mut intent)
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
