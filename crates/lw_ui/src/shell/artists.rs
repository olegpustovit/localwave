//! The desktop table uses a 37.3-point heading, estimated 38-point rows, 28-point artwork, 12-point
//! artwork/text gap, 32/8/20-point column insets and 140 ms hover. Scale four worker-prepared
//! preferred widths and borrow visible rows; wrapped numeric cells/headings use measured rows and
//! baseline alignment.
//!
use super::*;

fn label(ui: &mut Ui, key: &'static str, value: &str, rect: Rect, style: Label) {
    ui.label_at(key.into(), Rect { w: rect.w.max(0.0), ..rect }, value, style).unwrap();
}

/// An artist row's name and figures: one ellipsized line; counts and minutes in the mono face, wrapping.
const CELL: Label = Label::BODY.line_height(19.6).lines(1).ellipsis();
const CELL_MONO: Label = Label::BODY.font(1).size(12.0).line_height(super::kit::TABLE_LINE).wrap_spaces();

fn row_layout(
    ui: &mut Ui,
    columns: [f32; 4],
    summary: &crate::catalog::ArtistRow,
    count: &str,
) -> (f32, [(f32, f32); 4], f32) {
    let mut sizes = [
        ((columns[0] - 80.0).max(0.0), 19.6),
        ((columns[1] - 16.0).max(0.0), 19.6),
        (0.0, 0.0),
        (0.0, 0.0),
    ];
    let density = ui.density();
    for (index, value) in [count, summary.minutes.as_str()].into_iter().enumerate() {
        sizes[index + 2] = table_text_size(
            ui,
            density,
            value,
            (columns[index + 2] - if index == 0 { 16.0 } else { 28.0 }).max(0.0),
            summary.widths[index],
            &CELL_MONO,
            super::kit::TABLE_LINE,
        );
    }
    let descent = 9.8f32.max(sizes[2].1 - 12.0).max(sizes[3].1 - 12.0);
    let height = 38.0f32.max(18.2 + descent);
    (height, sizes, (height - 18.2 - descent) * 0.5)
}

pub(super) fn table(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    intent: &mut Option<Intent>,
) {
    if catalog.artist_table.is_some() && catalog.is_empty() {
        onboarding::empty_library(ui, catalog, state, width, intent);
        return;
    }
    let Some(table) = &catalog.artist_table else {
        header_view(ui, catalog, state, width, intent);
        ui.id("artists-loading-inset", |ui| { ui.column(Node::column().w(Size::Grow).h(Size::Fit).pad_x(32.0), |ui| {
            ui.label("Loading artists…", TEXT_14.grow().h_px(40.0).color(SECONDARY));
        }); });
        return;
    };
    let images = player.artwork;
    let total = table.preferred.iter().sum::<f32>().max(1.0);
    let columns = table.preferred.map(|preferred| preferred * width / total);
    let heading_sizes = std::array::from_fn::<_, 4, _>(|index| {
        heading_size(
            ui,
            ["Name", "Albums", "Songs", "Time"][index],
            (columns[index]
                - if index == 0 {
                    40.0
                } else if index == 3 {
                    28.0
                } else {
                    16.0
                })
            .max(0.0),
            table.headings[index],
        )
    });
    let heading_height = heading_sizes
        .iter()
        .map(|size| size.1)
        .fold(16.8f32, f32::max)
        + 20.5;
    let entry = crate::motion::Page::new(ui, state.entry_revision);
    let revision = crate::hash((
        state.revision,
        columns.map(f32::to_bits),
        images.map_or(0, crate::artwork::Images::revision),
        std::sync::Arc::as_ptr(&catalog.library) as usize,
    ));
    ui.section_list(
        "artists",
        &SectionList::new(catalog.artists.len()).row_h(38.0).lead(105.6).header(heading_height).overlay_scrollbar().measured(),
        &|i| i as i64,
        &mut |ui, part, _| match part {
            SectionPart::Leading => header_view(ui, catalog, state, width, intent),
            SectionPart::Trailing => {}
            SectionPart::Header => {
                let head = ui.node(
                    "artist-columns",
                    Node::column().w(Size::Grow).h_px(heading_height).look(PAPER_FILL),
                );
                ui.within(&head, &mut |ui| {
                    let mut x = 0.0;
                    for (index, title) in
                        ["Name", "Albums", "Songs", "Time"].into_iter().enumerate()
                    {
                        let left = if index == 0 { 32.0 } else { 8.0 };
                        let right = if index == 3 { 20.0 } else { 8.0 };
                        let rect = Rect {
                            x: x + if index == 3 { columns[index] - right - heading_sizes[index].0 } else { left },
                            y: 10.0 + (heading_height - 20.5 - heading_sizes[index].1) * 0.5,
                            w: heading_sizes[index].0,
                            h: heading_sizes[index].1,
                        };
                        let style = if index == 3 { TABLE_HEADING.align(pxui::TextAlign::End) } else { TABLE_HEADING }.color(SECONDARY);
                        ui.id(Key::Id(index as i64), |ui| label(ui, "heading", title, rect, style));
                        x += columns[index];
                    }
                });
                ui.canvas_over(&head, move |paint, rect| {
                    paint.fill(Rect::new(rect.x, (rect.y + rect.h).floor(), rect.w, 1.0), Fill::new(Rgba::token(LINE)).fractional())
                });
            }
            SectionPart::Row(index) => {
                let group = &catalog.artists[index];
                let summary = &table.rows[index];
                let name = if group.title.is_empty() {
                    "Unknown artist"
                } else {
                    group.title.as_str()
                };
                let (row_height, body_sizes, top) = row_layout(ui, columns, summary, &group.count);
                let visual = ui.view_of(
                    "artist-visual",
                    &revision,
                    |ui| {
                        let node = Node::column().w(Size::Grow).h_px(row_height).role(Role::Button).transition(Some(FADE));
                        let row = ui.pressable("artist", node, &PLAIN.look(&SONG_ROW_LOOKS), |ui, _| {
                            let art = ui.node(
                                "artist-artwork",
                                Node::column().w_px(28.0).h_px(28.0).at(32.0, top).look(PANEL_FILL),
                            );
                            paint_artwork(
                                ui,
                                &art,
                                group.artwork_row.map(|index| catalog.row(index)),
                                28.0,
                                images,
                            );
                            ui.a11y_name(&art, "Artist artwork");
                            let mut x = 0.0;
                            for (column, value) in [
                                name,
                                summary.albums.as_str(),
                                group.count.as_str(),
                                summary.minutes.as_str(),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                let mono = column >= 2;
                                let left = if column == 0 { 72.0 } else { 8.0 };
                                let right = if column == 3 { 20.0 } else { 8.0 };
                                label(
                                    ui,
                                    ["name", "albums", "songs", "time"][column],
                                    value,
                                    Rect {
                                        x: x + if column == 3 {
                                            columns[column] - right - body_sizes[column].0
                                        } else {
                                            left
                                        },
                                        y: top + 18.2 - if mono { 12.0 } else { 14.0 },
                                        w: body_sizes[column].0,
                                        h: body_sizes[column].1,
                                    },
                                    {
                                        let style = if mono { CELL_MONO } else { CELL };
                                        let style = if column == 3 { style.align(pxui::TextAlign::End) } else { style };
                                        style.color(if column == 0 { INK } else { SECONDARY })
                                    },
                                );
                                x += columns[column];
                            }
                        });
                        ui.a11y_name(&row.handle, name);
                        let last = index + 1 == catalog.artists.len();
                        ui.canvas_over(&row.handle, move |paint, rect| {
                            paint.fill(Rect::new(rect.x, rect.y.floor(), rect.w, 1.0), Fill::new(Rgba::token(LINE)).fractional());
                            if last {
                                paint.fill(Rect::new(rect.x, (rect.y + rect.h).floor(), rect.w, 1.0), Fill::new(Rgba::token(LINE)).fractional());
                            }
                        });
                        if row.clicked {
                            *intent = Some(Intent::Navigate(Route::Artist(index)));
                        }
                    },
                );
                entry.row(ui, &visual, index);
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::{FrameIn, Platform};

    #[test]
    fn artist_table_matches_swift_columns_virtualizes_and_routes_warm_input() {
        mem_sys::set_peer(pxui::library_counter);
        let mut catalog = Catalog::new(
            (0..4000)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Track {id}"),
                    artist: format!("Artist {:04}", id / 4),
                    album: ["Alpha", "Beta", "Alpha", "Gamma"][(id % 4) as usize].into(),
                    duration: Some([15.0, 30.0, 0.0, 45.0][(id % 4) as usize]),
                    ..Default::default()
                })
                .collect(),
        );
        catalog.prepare_tables(&crate::test_table_styles()).unwrap();
        let table = catalog.artist_table.as_ref().unwrap().clone();
        assert_eq!(table.rows[0].albums, "Alpha, Beta, Gamma");
        assert_eq!(table.rows[0].minutes, "2 min");
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Artists));
        let player = PlayerView::default();
        let mut frame = FrameIn::new(1440.0, 872.0).reduced_motion(true);
        let pending = Catalog::new(Vec::new());
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &pending, &state, &mut None)
        })
        .unwrap();
        let loading = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Loading artists…")
            .unwrap();
        assert_eq!(loading.rect.x, 264.0);
        assert!(ui.a11y_nodes().iter().any(|node| node.is_role(Role::Label)
            && node.rect.x >= 232.0
            && ui.a11y_name_of(node) == "Artists"));
        catalog.prepare_remap(&pending);
        state.refresh(&pending, &catalog);
        for _ in 0..3 {
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, &state, &mut None)
            })
            .unwrap();
        }
        let artist = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == "Artist 0000")
            .unwrap()
            .rect;
        assert!(
            (artist.x - 232.0).abs() < 0.001 && (artist.y - 142.9).abs() < 0.001,
            "full-bleed table: {artist:?}"
        );
        assert_eq!(artist.w, 888.0);
        assert_eq!(artist.h, 38.0);
        let art = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "Artist artwork")
            .unwrap()
            .rect;
        assert!(
            (art.x - 264.0).abs() < 0.001
                && (art.y - 147.9).abs() < 0.001
                && art.w == 28.0
                && art.h == 28.0
        );
        let mut x = 232.0;
        let total = table.preferred.iter().sum::<f32>();
        for (index, title) in ["Name", "Albums", "Songs", "Time"].into_iter().enumerate() {
            let header = ui
                .a11y_nodes()
                .iter()
                .find(|node| {
                    node.is_role(Role::Label)
                        && ui.a11y_name_of(node) == title
                        && node.rect.x >= 232.0
                })
                .unwrap()
                .rect;
            let width = table.preferred[index] * 888.0 / total;
            let left = if index == 0 { 32.0 } else { 8.0 };
            let right = if index == 3 { 20.0 } else { 8.0 };
            assert!(
                (header.x
                    - x
                    - if index == 3 {
                        width - right - table.headings[index].ceil()
                    } else {
                        left
                    })
                .abs()
                    < 0.001
                    && (header.w - table.headings[index].ceil()).abs() < 0.001,
                "{title}: {header:?}"
            );
            x += width;
        }
        let count = ui
            .a11y_nodes()
            .iter()
            .filter(|node| {
                node.is_role(Role::Button) && ui.a11y_name_of(node).starts_with("Artist 0")
            })
            .count();
        assert!(
            (16..40).contains(&count),
            "only visible artists plus overscan: {count}"
        );
        let list = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::List) && node.rect.x > 200.0 && node.rect.w > 500.0)
            .unwrap()
            .id;
        for cycle in 0..24 {
            mem_sys::local_reset();
            ui.inject_a11y(&[pxui::A11yAction {
                id: list,
                action_code: 8,
                payload: if cycle % 2 == 0 { 380.0 } else { -380.0 },
                ..Default::default()
            }]);
            frame.wire.time += 0.02;
            crate::testing::frame(&mut ui, &frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, &state, &mut None)
            })
            .unwrap();
            let heading = ui
                .a11y_nodes()
                .iter()
                .find(|node| {
                    node.is_role(Role::Label)
                        && ui.a11y_name_of(node) == "Artists"
                        && node.rect.x >= 232.0
                })
                .unwrap();
            let column = ui
                .a11y_nodes()
                .iter()
                .find(|node| {
                    node.is_role(Role::Label)
                        && ui.a11y_name_of(node) == "Name"
                        && node.rect.x >= 232.0
                })
                .unwrap();
            let offset = if cycle % 2 == 0 { 380.0 } else { 0.0 };
            assert!(
                (heading.rect.y - 28.0 + offset).abs() < 0.001,
                "page heading scrolls with the section: {:?}",
                heading.rect
            );
            assert!(
                (column.rect.y - if offset > 0.0 { 10.0 } else { 115.6 }).abs() < 0.001,
                "column heading pins independently: {:?}",
                column.rect
            );
            if cycle >= 6 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm artist scrolling");
            }
        }
        frame.wire.reduced_motion = 0;
        frame.wire.time += 1.0;
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut None)
        })
        .unwrap();
        let entered = frame.wire.time + 0.01;
        let pointer = pxui::Event::pointer(input::wire::POINTER_MOVED, 0, (artist.x + 100.0) as f64, (artist.y + 20.0) as f64);
        crate::testing::frame(
            &mut ui,
            &{
                let mut frame = frame.time(entered);
                frame = frame.events(std::slice::from_ref(&pointer));
                frame
            },
            &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut None),
        )
        .unwrap();
        assert!(ui.repaint_at().is_some());
        crate::testing::frame(
            &mut ui,
            &frame.time(entered + 0.07),
            &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut None),
        )
        .unwrap();
        assert!(
            ui.repaint_at().is_some(),
            "hover is still moving halfway through Swift's quick duration"
        );
        frame.wire.time = entered + 0.15;
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut None)
        })
        .unwrap();
        assert!(ui.repaint_at().is_none(), "hover settles after 140 ms");
        let mut intent = None;
        let target = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == "Artist 0000")
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id: target,
            action_code: 1,
            ..Default::default()
        }]);
        crate::testing::frame(&mut ui, &frame, &mut |ui| {
            shell(ui, &player, 0.0, &catalog, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent, Some(Intent::Navigate(Route::Artist(0))));
    }
    #[test]
    fn narrow_artist_rows_match_native_wrapping_baselines_and_growing_height() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = super::super::super::tests::prepared(
            (0..4)
                .map(|index| lw_core::Song {
                    id: Some(index),
                    title: format!("Track {index}"),
                    artist: "Artist 0000".into(),
                    album: ["Alpha", "Beta", "Alpha", "Gamma"][index as usize].into(),
                    duration: Some([15.0, 30.0, 0.0, 45.0][index as usize]),
                    ..Default::default()
                })
                .collect(),
        );
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Artists));
        let player = PlayerView::default();
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        for cycle in 0..8 {
            mem_sys::local_reset();
            for scale in [1.0, 2.0] {
                for width in [888.0, 520.0, 400.0, 364.0] {
                    crate::testing::frame(
                        &mut ui,
                        &FrameIn::new(width + 552.0, 872.0).scale(scale).reduced_motion(true),
                        &mut |ui| shell(ui, &player, 0.0, &catalog, &state, &mut None),
                    )
                    .unwrap();
                    let rect = |name: &str, role: Role| {
                        ui.a11y_nodes()
                            .iter()
                            .find(|node| node.is_role(role) && ui.a11y_name_of(node) == name)
                            .unwrap()
                            .rect
                    };
                    let row = rect("Artist 0000", Role::Button);
                    let wrapped = width < 520.0;
                    assert!((row.y - if wrapped { 159.7 } else { 142.9 }).abs() < 0.001);
                    assert!((row.h - if wrapped { 39.8 } else { 38.0 }).abs() < 0.001);
                    let time = rect("2 min", Role::Label);
                    let art = ui
                        .a11y_nodes()
                        .iter()
                        .find(|node| ui.a11y_name_of(node) == "Artist artwork")
                        .unwrap()
                        .rect;
                    assert!((time.y - row.y - if wrapped { 6.2 } else { 11.2 }).abs() < 0.001);
                    assert!((time.h - if wrapped { 33.6 } else { 16.8 }).abs() < 0.001);
                    assert!((time.w - if wrapped { 22.0 } else { 36.0 }).abs() < 0.001);
                    assert!((time.x + time.w - (232.0 + width - 20.0)).abs() < 0.001);
                    assert!((art.y - row.y - if wrapped { 0.0 } else { 5.0 }).abs() < 0.001);
                }
            }
            if cycle >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm native artist wrapping/resize"
                );
            }
        }
    }
}
