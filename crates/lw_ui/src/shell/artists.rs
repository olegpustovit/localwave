use super::*;

fn label(ui: &mut Ui, key: &str, value: &str, rect: Rect, style: pxui::TextStyle, color: Rgba) {
    let node = ui.node(
        key,
        Node {
            role: Role::Label,
            w: Size::Px(rect.w.max(0.0)),
            h: Size::Px(rect.h),
            at: Some((rect.x, rect.y)).into(),
            look: Look {
                text: color,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.text_line_box(&node, value, &style, &[]).unwrap();
}

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
    let style = pxui::TextStyle {
        font: 1,
        size_points: 12.0,
        line_height: 16.8,
        wrap: pxui::TextStyle::WRAP_WORD_SPACES,
        ..Default::default()
    };
    for (index, value) in [count, summary.minutes.as_str()].into_iter().enumerate() {
        sizes[index + 2] = table_text_size(
            ui,
            value,
            (columns[index + 2] - if index == 0 { 16.0 } else { 28.0 }).max(0.0),
            summary.widths[index],
            &style,
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
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let Some(table) = &catalog.artist_table else {
        header_view(ui, catalog, state, width, p, intent);
        let inset = ui.node(
            "artists-loading-inset",
            Node {
                w: Size::Grow,
                h: Size::Fit,
                padding: Edges {
                    l: 32.0,
                    r: 32.0,
                    ..Edges::ZERO
                },
                ..Node::DEFAULT
            },
        );
        ui.within(&inset, &mut |ui| {
            text(
                ui,
                "artists-loading",
                "Loading artists…",
                14,
                p.secondary,
                (Size::Grow, 40.0),
                false,
            );
        });
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
    let mut revision = DefaultHasher::new();
    (
        state.revision,
        columns.map(f32::to_bits),
        images.map_or(0, crate::artwork::Images::revision),
        std::sync::Arc::as_ptr(&catalog.library) as usize,
    )
        .hash(&mut revision);
    let revision = revision.finish();
    ui.section_list_measured(
        "artists",
        &SectionList {
            count: catalog.artists.len(),
            row_height: 38.0,
            leading_height: 105.6,
            header_height: heading_height,
            trailing_height: 0.0,
            overlay_scrollbar: true,
        },
        &mut |ui, part, _| match part {
            SectionPart::Leading => header_view(ui, catalog, state, width, p, intent),
            SectionPart::Header => {
                let head = ui.node(
                    "artist-columns",
                    Node {
                        w: Size::Grow,
                        h: Size::Px(heading_height),
                        look: Look {
                            fill: p.paper,
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&head, &mut |ui| {
                    let mut x = 0.0;
                    for (index, title) in
                        ["Name", "Albums", "Songs", "Time"].into_iter().enumerate()
                    {
                        let left = if index == 0 { 32.0 } else { 8.0 };
                        let right = if index == 3 { 20.0 } else { 8.0 };
                        label(
                            ui,
                            title,
                            title,
                            Rect {
                                x: x + if index == 3 {
                                    columns[index] - right - heading_sizes[index].0
                                } else {
                                    left
                                },
                                y: 10.0 + (heading_height - 20.5 - heading_sizes[index].1) * 0.5,
                                w: heading_sizes[index].0,
                                h: heading_sizes[index].1,
                            },
                            pxui::TextStyle {
                                size_points: 12.0,
                                line_height: 16.8,
                                weight: 500,
                                wrap: pxui::TextStyle::WRAP_WORD_SPACES,
                                align: if index == 3 {
                                    pxui::TextStyle::ALIGN_RIGHT
                                } else {
                                    pxui::TextStyle::ALIGN_LEFT
                                },
                                ..Default::default()
                            },
                            p.secondary,
                        );
                        x += columns[index];
                    }
                });
                ui.draw_over(&head, move |paint, rect| {
                    paint.fill_fractional(rect.x, (rect.y + rect.h).floor(), rect.w, 1.0, p.line)
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
                let visual = ui.view(
                    "artist-visual",
                    revision,
                    Size::Grow,
                    Size::Fit,
                    &mut |ui| {
                        let row = ui.node(
                            "artist",
                            Node {
                                flags: CLICKABLE | TAB_STOP,
                                role: Role::Button,
                                w: Size::Grow,
                                h: Size::Px(row_height),
                                ..Node::DEFAULT
                            },
                        );
                        ui.a11y_name(&row, name);
                        let signal = ui.signal_state(&row);
                        ui.set_look(
                            &row,
                            Look {
                                fill: p.paper,
                                ..Look::NONE
                            },
                            State::default(),
                        );
                        ui.within(&row, &mut |ui| {
                            let hover = ui.view("hover", 0, Size::Grow, Size::Grow, &mut |ui| {
                                container(
                                    ui,
                                    "fill",
                                    Axis::Stack,
                                    Size::Grow,
                                    Size::Grow,
                                    Rgba { a: 153, ..p.panel },
                                );
                            });
                            let previous = ui.state(&hover, -1);
                            let shown = i64::from(signal.hovered);
                            if previous != shown {
                                ui.set_state(&hover, shown);
                                ui.animate(
                                    &hover,
                                    Target::Opacity(shown as f32),
                                    if previous < 0 {
                                        crate::motion::steps(1, 0.0)
                                    } else {
                                        crate::motion::out(crate::motion::QUICK)
                                    },
                                );
                            }

                            let art = ui.node(
                                "artist-artwork",
                                Node {
                                    w: Size::Px(28.0),
                                    h: Size::Px(28.0),
                                    at: Some((32.0, top)).into(),
                                    look: Look {
                                        fill: p.panel,
                                        ..Look::NONE
                                    },
                                    ..Node::DEFAULT
                                },
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
                                    pxui::TextStyle {
                                        font: u16::from(mono),
                                        size_points: if mono { 12.0 } else { 14.0 },
                                        line_height: if mono { 16.8 } else { 19.6 },
                                        max_lines: if mono { 0 } else { 1 },
                                        ellipsis: if mono {
                                            pxui::TextStyle::ELLIPSIS_NONE
                                        } else {
                                            pxui::TextStyle::ELLIPSIS_END
                                        },
                                        wrap: if mono {
                                            pxui::TextStyle::WRAP_WORD_SPACES
                                        } else {
                                            pxui::TextStyle::WRAP_NONE
                                        },
                                        align: if column == 3 {
                                            pxui::TextStyle::ALIGN_RIGHT
                                        } else {
                                            pxui::TextStyle::ALIGN_LEFT
                                        },
                                        ..Default::default()
                                    },
                                    if column == 0 { p.ink } else { p.secondary },
                                );
                                x += columns[column];
                            }
                        });
                        let last = index + 1 == catalog.artists.len();
                        ui.draw_over(&row, move |paint, rect| {
                            paint.fill_fractional(rect.x, rect.y.floor(), rect.w, 1.0, p.line);
                            if last {
                                paint.fill_fractional(
                                    rect.x,
                                    (rect.y + rect.h).floor(),
                                    rect.w,
                                    1.0,
                                    p.line,
                                );
                            }
                        });
                        if signal.clicked {
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
        catalog.prepare_tables().unwrap();
        let table = catalog.artist_table.as_ref().unwrap();
        assert_eq!(table.rows[0].albums, "Alpha, Beta, Gamma");
        assert_eq!(table.rows[0].minutes, "2 min");
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Artists));
        let player = PlayerView::default();
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            reduced_motion: 1,
            ..Default::default()
        };
        let pending = Catalog::new(Vec::new());
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &pending, &state, &mut None)
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
        state.refresh(&pending, &catalog);
        for _ in 0..3 {
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut None)
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
            frame.time += 0.02;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut None)
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
        frame.reduced_motion = 0;
        frame.time += 1.0;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut None)
        })
        .unwrap();
        let entered = frame.time + 0.01;
        let pointer = pxui::Event {
            kind: input::wire::POINTER_MOVED,
            x: (artist.x + 100.0) as f64,
            y: (artist.y + 20.0) as f64,
            ..Default::default()
        };
        ui.frame(
            &FrameIn {
                time: entered,
                events: &pointer,
                event_count: 1,
                ..frame
            },
            &mut |ui| shell(ui, &player, &catalog, &state, &mut None),
        )
        .unwrap();
        assert!(ui.repaint_at().is_some());
        ui.frame(
            &FrameIn {
                time: entered + 0.07,
                ..frame
            },
            &mut |ui| shell(ui, &player, &catalog, &state, &mut None),
        )
        .unwrap();
        assert!(
            ui.repaint_at().is_some(),
            "hover is still moving halfway through Swift's quick duration"
        );
        frame.time = entered + 0.15;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut None)
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
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
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
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        for cycle in 0..8 {
            mem_sys::local_reset();
            for scale in [1.0, 2.0] {
                for width in [888.0, 520.0, 400.0, 364.0] {
                    ui.frame(
                        &FrameIn {
                            width: width + 552.0,
                            height: 872.0,
                            scale,
                            reduced_motion: 1,
                            ..Default::default()
                        },
                        &mut |ui| shell(ui, &player, &catalog, &state, &mut None),
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
