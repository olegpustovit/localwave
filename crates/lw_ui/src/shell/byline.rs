//! The title wraps intrinsically. The byline uses a 15-point strut and baseline-aligned padded
//! 14-point links, trims leading wrapped whitespace and measures only inside its retained metadata
//! view. Natural/constrained measurement identities differ by width. Preserve accessible
//! artist/album routing, empty-line height and native 100/279/400-point fitting references.
//!
use super::*;

// Swift's PlayerBylineFlow uses a 15pt strut and aligns padded 14pt links by baseline.
fn positions(width: f32, sizes: [(f32, f32); 5]) -> ([Rect; 5], f32) {
    let baselines = [16.0, 15.0, 14.0, 15.0, 16.0];
    let mut result = [Rect::default(); 5];
    let (mut x, mut y, mut ascent, mut descent) = (0.0, 0.0, 15.0f32, 6.0f32);
    let mut start = 0;
    for index in 0..=5 {
        if index == 5 || (x > 0.0 && x + sizes[index].0 > width) {
            for item in start..index {
                result[item].y += y + ascent - baselines[item];
            }
            y += ascent + descent;
            (x, ascent, descent) = (0.0, 15.0, 6.0);
            start = index;
        }
        if index == 5 {
            break;
        }
        if x == 0.0 && matches!(index, 1 | 3) {
            start = index + 1;
            continue;
        }
        let (w, h) = sizes[index];
        result[index] = Rect { x, y: 0.0, w, h };
        x += w;
        ascent = ascent.max(baselines[index]);
        descent = descent.max(h - baselines[index]);
    }
    (result, y)
}

pub(super) fn draw(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let values = [player.artist, " ", "—", " ", player.album];
    let styles = std::array::from_fn::<_, 5, _>(|index| pxui::TextStyle {
        size_points: if matches!(index, 1 | 3) { 15.0 } else { 14.0 },
        line_height: if matches!(index, 1 | 3) { 21.0 } else { 19.6 },
        wrap: pxui::TextStyle::WRAP_WORD_SPACES,
        ..Default::default()
    });
    let density = ui.density() as f32;
    let sizes = std::array::from_fn(|index| {
        let service = &mut ui.text_service();
        let natural = service.measure(values[index], &styles[index], 0.0).unwrap();
        let metrics = if natural.width > width {
            service
                .measure(values[index], &styles[index], width)
                .unwrap()
        } else {
            natural
        };
        let padding = if matches!(index, 0 | 4) { 2.0 } else { 0.0 };
        (
            ((metrics.width * density).ceil() / density).min(width),
            styles[index].line_height * metrics.lines.max(1) as f32 + padding,
        )
    });
    let (rects, height) = positions(width, sizes);
    let root = ui.node("byline", Node::column().w_px(width).h_px(height));
    ui.within(&root, &mut |ui| {
        for index in [0, 2, 4] {
            let rect = rects[index];
            let link = index != 2;
            let node = ui.node(
                ["artist", "space-one", "separator", "space-two", "album"][index],
                Node {
                    at: Some((rect.x, rect.y)).into(),
                    inset: Edges {
                        t: if link { 2.0 } else { 0.0 },
                        ..Edges::ZERO
                    },
                    look: Look {
                        text: ui.color(SECONDARY),
                        ..Look::NONE
                    },
                    ..Node::column()
                        .w_px(rect.w)
                        .h_px(rect.h)
                        .role(if link { Role::Button } else { Role::Label })
                        .flags(if link { Flags::CLICKABLE | Flags::TAB_STOP } else { Flags::NONE })
                },
            );
            ui.text_line_box(&node, values[index], &styles[index], &[])
                .unwrap();
            if link && ui.signal_state(&node).clicked {
                let route = player.current_id.and_then(|id| {
                    let row = catalog.row(catalog.row_index(id)?);
                    let matches = if index == 0 {
                        row.artist == player.artist
                    } else {
                        row.album == player.album
                    };
                    if !matches {
                        return None;
                    }
                    catalog
                        .byline_routes(id)
                        .map(|routes| routes[usize::from(index == 4)])
                });
                if let Some(route) = route {
                    *intent = Some(Intent::Navigate(route));
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_matches_swift_baselines_and_trims_wrapped_spaces() {
        let sizes = [
            (81.0, 21.6),
            (5.0, 21.0),
            (12.0, 19.6),
            (5.0, 21.0),
            (206.0, 21.6),
        ];
        let (rects, height) = positions(400.0, sizes);
        assert_eq!(height, 22.0);
        assert_eq!(rects.map(|r| r.y), [0.0, 1.0, 2.0, 1.0, 0.0]);
        assert!((rects[4].x - 103.0).abs() < 0.001);
        let (rects, height) = positions(279.0, sizes);
        assert_eq!(height, 44.0);
        assert_eq!((rects[4].x, rects[4].y), (0.0, 22.0));
        let (rects, height) = positions(
            80.192,
            [
                (80.192, 41.2),
                (3.75, 21.0),
                (14.0, 19.6),
                (3.75, 21.0),
                (80.192, 60.8),
            ],
        );
        assert_eq!(rects[1], Rect::default());
        assert_eq!((rects[2].x, rects[2].y), (0.0, 42.2));
        assert!((height - 123.0).abs() < 0.001, "{height}");
    }

    #[test]
    fn byline_wraps_like_swift_and_routes_links_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = crate::tests::prepared(vec![lw_core::Song {
            id: Some(1),
            title: "Track".into(),
            artist: "The National".into(),
            album: "High Violet, Trouble Will Find Me".into(),
            album_artist: "Album artist".into(),
            ..Default::default()
        }]);
        let mut player = PlayerView {
            current_id: Some(1),
            artist: "The National",
            album: "High Violet, Trouble Will Find Me",
            ..Default::default()
        };
        let mut ui = crate::test_ui(pxui::Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut frame = pxui::FrameIn {
            width: 400.0,
            height: 400.0,
            ..Default::default()
        };
        let mut intent = None;
        ui.frame(&frame, &mut |ui| {
            draw(ui, &player, &catalog, 279.0, &mut intent)
        })
        .unwrap();
        let find = |ui: &Ui, value: &str| {
            *ui.a11y_nodes()
                .iter()
                .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == value)
                .unwrap()
        };
        let artist = find(&ui, player.artist);
        let album = find(&ui, player.album);
        let dash = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == "—")
            .unwrap();
        assert_eq!((dash.rect.x, dash.rect.y, dash.rect.w), (86.0, 2.0, 12.0));
        assert_eq!(artist.rect.w, 81.0);
        assert_eq!(
            (artist.rect.x, artist.rect.y, album.rect.x, album.rect.y),
            (0.0, 0.0, 0.0, 22.0)
        );
        for cycle in 0..8 {
            mem_sys::local_reset();
            for scale in [1.0, 2.0] {
                frame.scale = scale;
                for width in [100.0, 279.0, 400.0] {
                    for (value, route) in [
                        (player.artist, Route::Artist(0)),
                        (player.album, Route::Album(0)),
                    ] {
                        let id = find(&ui, value).id;
                        ui.inject_a11y(&[pxui::A11yAction {
                            id,
                            action_code: 1,
                            ..Default::default()
                        }]);
                        frame.time += 0.1;
                        intent = None;
                        ui.frame(&frame, &mut |ui| {
                            draw(ui, &player, &catalog, width, &mut intent)
                        })
                        .unwrap();
                        assert_eq!(intent, Some(Intent::Navigate(route)));
                        let artist = find(&ui, player.artist);
                        let album = find(&ui, player.album);
                        assert_eq!(artist.rect.w, if scale == 1.0 { 81.0 } else { 80.5 });
                        assert!(artist.rect.w <= width && album.rect.w <= width);
                        let (x, y, w, h) = if width == 100.0 {
                            (0.0, 22.0, 78.0, 60.8)
                        } else if width == 279.0 {
                            (0.0, 22.0, 206.0, 21.6)
                        } else {
                            (if scale == 1.0 { 103.0 } else { 102.5 }, 0.0, 206.0, 21.6)
                        };
                        assert!(
                            (album.rect.x - x).abs() < 0.001
                                && (album.rect.y - y).abs() < 0.001
                                && (album.rect.w - w).abs() < 0.001
                                && (album.rect.h - h).abs() < 0.001,
                            "native PlayerBylineFlow at {width}: {:?}",
                            album.rect
                        );
                    }
                }
            }
            if cycle >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm measured byline and accessible links"
                );
            }
        }
        for artist in [true, false] {
            let label = if artist {
                "Unadopted artist"
            } else {
                "Unadopted album"
            };
            if artist {
                player.artist = label;
            } else {
                player.album = label;
            }
            ui.frame(&frame, &mut |ui| {
                draw(ui, &player, &catalog, 279.0, &mut intent)
            })
            .unwrap();
            let id = find(&ui, label).id;
            ui.inject_a11y(&[pxui::A11yAction {
                id,
                action_code: 1,
                ..Default::default()
            }]);
            intent = None;
            ui.frame(&frame, &mut |ui| {
                draw(ui, &player, &catalog, 279.0, &mut intent)
            })
            .unwrap();
            assert_eq!(
                intent, None,
                "metadata awaiting catalog adoption cannot route to an old group"
            );
        }
    }
}
