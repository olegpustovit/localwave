//! Album and artist detail headers, 8/24-point padded 240-point artwork bottom-aligned with the
//! metadata block (stacked under 696 points). Summaries are prepared with the collection; frames
//! only borrow strings. Play/Shuffle reuse the Find toolbar buttons and the scoped song table is
//! the body. Not yet ported: album format/folder lines and the artist album grid.
use super::find::{Action, button};
use super::*;
use lw_core::context::PlaybackContext;

const ALBUM_META: f32 = 18.2 + 40.8 + 6.0 + 25.2 + 12.0 + 18.2 + 20.0 + 30.0;
const ARTIST_META: f32 = 18.2 + 48.0 + 6.0 + 19.6 + 20.0 + 30.0;

fn wide(width: f32) -> bool {
    width >= 32.0 + 240.0 + 32.0 + 360.0 + 32.0
}
fn art_size(width: f32) -> f32 {
    if wide(width) {
        240.0
    } else {
        (width - 64.0).clamp(120.0, 240.0)
    }
}
/// Fixed leading height of the section list for a detail route.
pub(super) fn height(width: f32, route: Route) -> f32 {
    match route {
        Route::Album(_) if wide(width) => 8.0 + 240.0 + 24.0,
        Route::Album(_) => 8.0 + art_size(width) + 24.0 + ALBUM_META + 24.0,
        _ => 28.0 + ARTIST_META + 18.0,
    }
}
fn line(
    ui: &mut Ui,
    key: &str,
    value: &str,
    (size, line_height, weight, tracking): (f32, f32, u16, f32),
    color: Rgba,
    flags: i64,
) -> Handle {
    let node = ui.node(
        key,
        Node {
            flags,
            role: if flags == 0 {
                Role::Label
            } else {
                Role::Button
            },
            w: Size::Grow,
            h: Size::Px(line_height),
            look: Look {
                text: color,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.text_line_box(
        &node,
        value,
        &pxui::TextStyle {
            size_points: size,
            line_height,
            weight,
            tracking,
            max_lines: 1,
            ellipsis: pxui::TextStyle::ELLIPSIS_END,
            ..Default::default()
        },
        &[],
    )
    .unwrap();
    node
}

pub(super) fn header(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let Some(group) = state.collection(catalog) else {
        return;
    };
    let route = state.route;
    let revision = pxui::hash!((
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        width.to_bits(),
        player.artwork.map_or(0, crate::artwork::Images::revision),
    ));
    ui.view(
        "page-heading",
        revision,
        Size::Grow,
        Size::Px(height(width, route)),
        &mut |ui| {
            let album = matches!(route, Route::Album(_));
            let row = album && wide(width) || !album;
            let root = ui.node(
                "detail-header",
                Node {
                    axis: if row { Axis::Row } else { Axis::Column },
                    align: if row { Align::End } else { Align::Start },
                    gap: if album {
                        if row { 32.0 } else { 24.0 }
                    } else {
                        0.0
                    },
                    w: Size::Grow,
                    h: Size::Grow,
                    padding: if album {
                        Edges {
                            l: 32.0,
                            r: 32.0,
                            t: 8.0,
                            b: 24.0,
                        }
                    } else {
                        Edges {
                            l: 32.0,
                            r: 32.0,
                            t: 28.0,
                            b: 18.0,
                        }
                    },
                    ..Node::DEFAULT
                },
            );
            ui.within(&root, &mut |ui| {
                if album {
                    let size = art_size(width);
                    let art = container(
                        ui,
                        "artwork",
                        Axis::Stack,
                        Size::Px(size),
                        Size::Px(size),
                        p.panel,
                    );
                    paint_artwork(
                        ui,
                        &art,
                        group.artwork_row.map(|index| catalog.row(index)),
                        size,
                        player.artwork,
                    );
                    ui.a11y_name(&art, "Album artwork");
                }
                let meta = ui.node(
                    "meta",
                    Node {
                        axis: Axis::Column,
                        w: Size::Grow,
                        h: Size::Px(if album { ALBUM_META } else { ARTIST_META }),
                        ..Node::DEFAULT
                    },
                );
                ui.within(&meta, &mut |ui| {
                    line(
                        ui,
                        "kind",
                        if album { "Album" } else { "Artist" },
                        (13.0, 18.2, 400, 0.0),
                        p.secondary,
                        0,
                    );
                    let title = match (album, group.title.is_empty()) {
                        (false, true) => "Unknown artist",
                        _ => &group.title,
                    };
                    line(
                        ui,
                        "title",
                        title,
                        if album {
                            (40.0, 40.8, 500, -1.2)
                        } else {
                            (48.0, 48.0, 500, -1.2)
                        },
                        p.ink,
                        0,
                    );
                    if let PlaybackContext::Album { artist, .. } = group.context.as_ref()
                        && !artist.is_empty()
                    {
                        spacer(ui, "artist-gap", 6.0);
                        let link = line(
                            ui,
                            "artist",
                            artist,
                            (18.0, 25.2, 400, 0.0),
                            p.secondary,
                            CLICKABLE | TAB_STOP,
                        );
                        if ui.signal_state(&link).clicked
                            && let Some(index) = catalog
                                .library
                                .artists
                                .iter()
                                .position(|a| a.title == *artist)
                        {
                            *intent = Some(Intent::Navigate(Route::Artist(index)));
                        }
                    }
                    spacer(ui, "summary-gap", if album { 12.0 } else { 6.0 });
                    line(
                        ui,
                        "summary",
                        &group.summary,
                        if album {
                            (13.0, 18.2, 400, 0.0)
                        } else {
                            (14.0, 19.6, 400, 0.0)
                        },
                        p.secondary,
                        0,
                    );
                    spacer(ui, "buttons-gap", 20.0);
                    let buttons = ui.node(
                        "buttons",
                        Node {
                            axis: Axis::Row,
                            gap: 8.0,
                            w: Size::Grow,
                            h: Size::Px(30.0),
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&buttons, &mut |ui| {
                        let enabled = !group.ids.is_empty();
                        if button(ui, Action::Start, enabled, p) {
                            *intent = Some(Intent::PlayRow(0));
                        }
                        if button(ui, Action::Shuffle, enabled, p) {
                            *intent = Some(Intent::ShuffleCollection);
                        }
                    });
                });
            });
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::{FrameIn, Platform};

    fn song(id: i64, title: &str, album: &str, seconds: f64) -> lw_core::Song {
        lw_core::Song {
            id: Some(id),
            title: title.into(),
            artist: "Alice".into(),
            album: album.into(),
            release_year: Some(2019),
            duration: Some(seconds),
            ..Default::default()
        }
    }

    #[test]
    fn prepared_summaries_match_swift_wording() {
        let catalog = crate::tests::prepared(vec![
            song(1, "One", "A", 1800.0),
            song(2, "Two", "A", 1050.0),
            song(3, "Three", "B", 0.0),
        ]);
        let library = &catalog.library;
        assert_eq!(library.albums[0].summary, "2019 · 2 songs · 48 min");
        assert_eq!(library.albums[1].summary, "2019 · 1 songs · 0 min");
        assert_eq!(library.artists[0].summary, "2 albums · 3 songs");
        assert_eq!(library.all.as_ref().unwrap().summary, "");
    }

    #[test]
    fn album_and_artist_headers_render_actions_and_scoped_rows_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = crate::tests::prepared(vec![
            song(1, "Opener", "A", 1800.0),
            song(2, "Closer", "A", 1080.0),
            song(3, "Other Album Song", "B", 60.0),
        ]);
        let player = PlayerView::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            reduced_motion: 1,
            ..Default::default()
        };
        let mut state = ShellState::default();
        let run = |ui: &mut Ui, state: &ShellState, frame: &FrameIn, click: Option<&str>| {
            if let Some(name) = click {
                let id = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| ui.a11y_name_of(node) == name)
                    .unwrap_or_else(|| panic!("missing {name}"))
                    .id;
                ui.inject_a11y(&[pxui::A11yAction {
                    id,
                    action_code: 1,
                    ..Default::default()
                }]);
            }
            let mut intent = None;
            ui.frame(frame, &mut |ui| {
                shell(ui, &player, 0.0, &catalog, state, &mut intent)
            })
            .unwrap();
            intent
        };
        let has = |ui: &Ui, name: &str| ui.a11y_nodes().iter().any(|n| ui.a11y_name_of(n) == name);
        for (route, kind, title, link, summary, rows, other) in [
            (
                Route::Album(0),
                "Album",
                "A",
                true,
                "2019 · 2 songs · 48 min",
                "Closer",
                "Other Album Song",
            ),
            (
                Route::Artist(0),
                "Artist",
                "Alice",
                false,
                "2 albums · 3 songs",
                "Other Album Song",
                "Nope",
            ),
        ] {
            state.apply(Intent::Navigate(route));
            for _ in 0..3 {
                run(&mut ui, &state, &frame, None);
            }
            assert!(
                has(&ui, kind) && has(&ui, title) && has(&ui, summary),
                "{kind} header"
            );
            assert_eq!(has(&ui, "Alice"), true);
            assert!(has(&ui, rows) && !has(&ui, other), "scoped rows");
            frame.time += 0.1;
            assert_eq!(
                run(&mut ui, &state, &frame, Some("Play")),
                Some(Intent::PlayRow(0))
            );
            frame.time += 0.1;
            assert_eq!(
                run(&mut ui, &state, &frame, Some("Shuffle")),
                Some(Intent::ShuffleCollection)
            );
            if link {
                frame.time += 0.1;
                assert_eq!(
                    run(&mut ui, &state, &frame, Some("Alice")),
                    Some(Intent::Navigate(Route::Artist(0)))
                );
            }
            for cycle in 0..8 {
                mem_sys::local_reset();
                frame.time += 0.1;
                run(&mut ui, &state, &frame, None);
                if cycle >= 3 {
                    assert_eq!(mem_sys::local_alloc_count(), 0, "warm {kind} detail frames");
                }
            }
        }
    }
}
