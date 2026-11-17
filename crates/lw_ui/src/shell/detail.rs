//! Album and artist detail headers, 8/24-point padded 240-point artwork bottom-aligned with the
//! metadata block (stacked under 696 points). Summaries are prepared with the collection; frames
//! only borrow strings. Both pages lead with Swift's 40-point Back row and 52-point Play/Shuffle
//! (+ Keep offline on albums) keys; the artist page adds its album grid (180-point cards, 20/28-point
//! gaps, prepared per artist) before the scoped song table. Albums add Swift's mono format line after the
//! summary and, once the library worker resolved it, an underlined "Folder:" link that opens that Files folder.
use super::compact::list::{key, offline_key};
use super::*;
use lw_core::context::PlaybackContext;

const BACK: f32 = 40.0;
const KEYS: f32 = 52.0;
const ALBUM_META: f32 = 18.2 + 40.8 + 6.0 + 25.2 + 12.0 + 18.2 + 20.0 + KEYS;
/// Swift's "Folder:" line (4 pt gap + 18.2 pt line) when the album's source folder is known.
const FOLDER: f32 = 4.0 + 18.2;
const ARTIST_META: f32 = 6.0 + 18.2 + 48.0 + 6.0 + 19.6;
const ARTIST_KEYS: f32 = 300.0;
const ALBUM_KEYS: f32 = 3.0 * 114.0 + 16.0;
const CARD_GAP: f32 = 20.0;
const ROW_GAP: f32 = 28.0;

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
fn stacked(width: f32) -> bool {
    width < 32.0 + 260.0 + 24.0 + ARTIST_KEYS + 32.0
}
/// Artist album grid columns and card width (Swift's native Grid, 150-point minimum, 180 maximum).
fn grid(width: f32) -> (usize, f32) {
    let available = (width - 64.0).max(1.0);
    let columns = (((available + CARD_GAP) / (150.0 + CARD_GAP)) as usize).max(1);
    let cell = ((available - (columns - 1) as f32 * CARD_GAP) / columns as f32).min(180.0);
    (columns, cell)
}
fn card_height(cell: f32) -> f32 {
    cell + 8.0 + 17.5 + 18.2
}
/// Height of the artist album grid with its 40-point tail; zero without albums.
fn grid_height(width: f32, albums: usize) -> f32 {
    let (columns, cell) = grid(width);
    let rows = albums.div_ceil(columns);
    if rows == 0 {
        0.0
    } else {
        rows as f32 * card_height(cell) + (rows - 1) as f32 * ROW_GAP + 40.0
    }
}
/// Height of the header body under the Back row: artwork/title block with its padding.
fn body_height(width: f32, route: Route, folder: bool) -> f32 {
    match route {
        Route::Album(_) if wide(width) => 8.0 + 240.0 + 24.0,
        Route::Album(_) => 8.0 + art_size(width) + 24.0 + ALBUM_META + if folder { FOLDER } else { 0.0 } + 24.0,
        _ => 28.0 + ARTIST_META + if stacked(width) { 20.0 + KEYS } else { 0.0 } + 18.0,
    }
}
/// Fixed leading height of the section list for a detail route (`albums`: the artist's album cards).
pub(super) fn height(width: f32, route: Route, albums: usize, folder: bool) -> f32 {
    BACK + body_height(width, route, folder)
        + if matches!(route, Route::Artist(_)) { grid_height(width, albums) } else { 0.0 }
}
fn back_row(ui: &mut Ui, parent: Route, intent: &mut Option<Intent>) {
    let row = ui.node("back-row".into(), Node::column().w(Size::Grow).h_px(BACK));
    ui.within(&row, &mut |ui| {
        const BUTTON: pxui::Button = pxui::Button::sized(96.0, BACK).look(&BARE).name("Back");
        let back = ui.button_content("back", &BUTTON, |ui, _| {
            ui.row(
                "content",
                Node { padding: Edges { l: 26.0, ..Edges::ZERO }, ..Node::row().w(Size::Grow).h(Size::Grow).gap(4.0).align(Align::End) },
                |ui| {
                    let glyph = ui.node("icon".into(), Node::column().w_px(20.0).h_px(20.0));
                    let color = ui.color(SECONDARY);
                    ui.draw(&glyph, move |paint, rect| kit::paint_icon(paint, Icon::Back, rect.x, rect.y, color));
                    ui.label("Back", TEXT_14.h(19.6).color(SECONDARY));
                },
            );
        });
        if back.clicked {
            *intent = Some(Intent::Navigate(parent));
        }
    });
}
pub(super) fn line(
    ui: &mut Ui,
    key: &'static str,
    value: &str,
    // tracking in ems
    (size, line_height, weight, tracking): (f32, f32, u16, f32),
    color: Rgba,
    flags: Flags,
) -> Handle {
    let node = ui.node(
        key.into(),
        Node {
            flags,
            look: Look {
                text: color,
                ..Look::NONE
            },
            ..Node::column()
                .w(Size::Grow)
                .h_px(line_height)
                .role(if flags == Flags::NONE {
                    Role::Label
                } else {
                    Role::Button
                })
        },
    );
    ui.text_line_box(
        &node,
        value,
        &Label::BODY.size(size).line_height(line_height).weight(weight).tracking(tracking).lines(1).ellipsis(),
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
    intent: &mut Option<Intent>,
) {
    let Some(group) = state.collection(catalog) else {
        return;
    };
    let route = state.route;
    let album = matches!(route, Route::Album(_));
    let cards: &[usize] = match route {
        Route::Artist(index) => catalog.artist_albums.get(index).map_or(&[], Vec::as_slice),
        _ => &[],
    };
    // Albums are small; the artist page has no kept state.
    let folder = group.folder.is_some();
    let kept = album && !group.rows.is_empty() && group.rows.iter().all(|&row| catalog.row(row).pinned);
    let revision = crate::hash((
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        width.to_bits(),
        kept,
        folder,
        player.artwork.map_or(0, crate::artwork::Images::revision),
    ));
    ui.view(
        "page-heading",
        revision,
        Size::Grow,
        Size::Px(height(width, route, cards.len(), folder)),
        &mut |ui| {
            let column = container(ui, "detail-page", Axis::Column, Size::Grow, Size::Grow, ui.color(PAPER));
            ui.within(&column, &mut |ui| {
                back_row(ui, if album { Route::Albums } else { Route::Artists }, intent);
                let row = album && wide(width) || !album && !stacked(width);
                let root = ui.node(
                    "detail-header".into(),
                    Node {
                        axis: if row { Axis::Row } else { Axis::Column },
                        padding: if album {
                            Edges { l: 32.0, r: 32.0, t: 8.0, b: 24.0 }
                        } else {
                            Edges { l: 32.0, r: 32.0, t: 28.0, b: 18.0 }
                        },
                        ..Node::column()
                            .w(Size::Grow)
                            .h_px(body_height(width, route, folder))
                            .gap(if album {
                                if row { 32.0 } else { 24.0 }
                            } else if row {
                                24.0
                            } else {
                                20.0
                            })
                            .align(if row { Align::End } else { Align::Start })
                    },
                );
                ui.within(&root, &mut |ui| {
                    header_body(ui, catalog, player, group, album, kept, width, intent);
                });
                if !cards.is_empty() {
                    album_grid(ui, catalog, player, cards, width, intent);
                }
            });
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn header_body(
    ui: &mut Ui,
    catalog: &Catalog,
    player: &PlayerView<'_>,
    group: &crate::catalog::Collection,
    album: bool,
    kept: bool,
    width: f32,
    intent: &mut Option<Intent>,
) {
    if album {
        let size = art_size(width);
        let art = container(ui, "artwork", Axis::Stack, Size::Px(size), Size::Px(size), ui.color(PANEL));
        paint_artwork(ui, &art, group.artwork_row.map(|index| catalog.row(index)), size, player.artwork);
        ui.a11y_name(&art, "Album artwork");
    }
    ui.column(
        "meta",
        Node::column()
            .w(Size::Grow)
            .h_px(if album { ALBUM_META + if group.folder.is_some() { FOLDER } else { 0.0 } } else { ARTIST_META }),
        |ui| {
            if !album {
                spacer(ui, "kind-gap", 6.0);
            }
            line(ui, "kind", if album { "Album" } else { "Artist" }, (13.0, 18.2, 400, 0.0), ui.color(SECONDARY), Flags::NONE);
            let title = match (album, group.title.is_empty()) {
                (false, true) => "Unknown artist",
                _ => &group.title,
            };
            line(
                ui,
                "title",
                title,
                if album { (40.0, 40.8, 500, TIGHT_3) } else { (48.0, 48.0, 500, -0.025) },
                ui.color(INK),
                Flags::NONE,
            );
            if let PlaybackContext::Album { artist, .. } = group.context.as_ref()
                && !artist.is_empty()
            {
                spacer(ui, "artist-gap", 6.0);
                let link = line(ui, "artist", artist, (18.0, 25.2, 400, 0.0), ui.color(SECONDARY), Flags::CLICKABLE | Flags::TAB_STOP);
                if ui.signal(&link).clicked
                    && let Some(index) = catalog.library.artists.iter().position(|a| a.title == *artist)
                {
                    *intent = Some(Intent::Navigate(Route::Artist(index)));
                }
            }
            spacer(ui, "summary-gap", if album { 12.0 } else { 6.0 });
            if album && !group.format.is_empty() {
                // Swift: "2019 · 8 songs · 37 min · " in the text face, then the format in mono.
                let row = container(ui, "summary", Axis::Row, Size::Grow, Size::Px(18.2), ui.color(PAPER));
                ui.within(&row, &mut |ui| {
                    ui.label(&group.lead, TEXT_13.h(18.2).color(SECONDARY));
                    ui.label(&group.format, MONO_12.w(Size::Grow).h(16.8).color(SECONDARY));
                });
            } else {
                line(
                    ui,
                    "summary",
                    &group.summary,
                    if album { (13.0, 18.2, 400, 0.0) } else { (14.0, 19.6, 400, 0.0) },
                    ui.color(SECONDARY),
                    Flags::NONE,
                );
            }
            if let Some(target) = &group.folder {
                spacer(ui, "folder-gap", 4.0);
                ui.row("folder", Node::row().w(Size::Grow).h_px(18.2).gap(4.0), |ui| {
                    ui.label("Folder:", TEXT_13.h(18.2).color(SECONDARY));
                    let link = ui.node(
                        "link".into(),
                        Node::column().w(Size::Fit).h_px(18.2).role(Role::Button).flags(Flags::CLICKABLE | Flags::TAB_STOP),
                    );
                    ui.a11y_name(&link, &target.label);
                    ui.within(&link, &mut |ui| {
                        ui.label(&target.label, TEXT_13.h(17.2).color(INK));
                        container(ui, "underline", Axis::Column, Size::Grow, Size::Px(1.0), ui.color(INK));
                    });
                    if ui.signal(&link).clicked {
                        *intent = Some(Intent::Files(crate::files::Request::Browse(target.location)));
                    }
                });
            }
            if album {
                spacer(ui, "buttons-gap", 20.0);
                keys(ui, group, true, kept, ALBUM_KEYS.min((width - 64.0).max(1.0)), intent);
            }
        },
    );
    if !album {
        keys(ui, group, false, false, if stacked(width) { (width - 64.0).max(1.0) } else { ARTIST_KEYS }, intent);
    }
}

/// Swift's playKeys: Play (ink), Shuffle and, on albums, Keep offline.
fn keys(
    ui: &mut Ui,
    group: &crate::catalog::Collection,
    album: bool,
    kept: bool,
    width: f32,
    intent: &mut Option<Intent>,
) {
    ui.row("buttons", Node::row().w_px(width).h_px(KEYS).gap(8.0), |ui| {
        let enabled = !group.ids.is_empty();
        if key(ui, "play", "Play", Icon::Play, true) && enabled {
            *intent = Some(Intent::PlayRow(0));
        }
        if key(ui, "shuffle", "Shuffle", Icon::Shuffle, false) && enabled {
            *intent = Some(Intent::ShuffleCollection);
        }
        if album && offline_key(ui, kept) && enabled {
            *intent = Some(Intent::Page(crate::PageAction::KeepOffline));
        }
    });
}

/// The artist's album cards: artwork, name and year, in rows of `grid(width)` columns.
fn album_grid(
    ui: &mut Ui,
    catalog: &Catalog,
    player: &PlayerView<'_>,
    cards: &[usize],
    width: f32,
    intent: &mut Option<Intent>,
) {
    let (columns, cell) = grid(width);
    let grid = ui.node(
        "album-grid".into(),
        Node {
            padding: Edges { l: 32.0, r: 32.0, b: 40.0, ..Edges::ZERO },
            ..Node::column().w(Size::Grow).h_px(grid_height(width, cards.len())).gap(ROW_GAP)
        },
    );
    ui.within(&grid, &mut |ui| {
        for (line_index, chunk) in cards.chunks(columns).enumerate() {
            let cards_row = ui.node(
                Key::Id(line_index as i64),
                Node::row().w(Size::Grow).h_px(card_height(cell)).gap(CARD_GAP),
            );
            ui.within(&cards_row, &mut |ui| {
                for &index in chunk {
                    let group = &catalog.albums[index];
                    let card = ui.node(
                        Key::Id(index as i64),
                        Node::column().w_px(cell).h(Size::Grow).role(Role::Button).flags(Flags::CLICKABLE | Flags::TAB_STOP),
                    );
                    ui.a11y_name(&card, &group.title);
                    ui.within(&card, &mut |ui| {
                        let art = container(ui, "artwork", Axis::Stack, Size::Px(cell), Size::Px(cell), ui.color(PANEL));
                        paint_artwork(ui, &art, group.artwork_row.map(|row| catalog.row(row)), cell, player.artwork);
                        spacer(ui, "art-gap", 8.0);
                        line(ui, "name", &group.title, (14.0, 17.5, 500, 0.0), ui.color(INK), Flags::NONE);
                        let year = group.rows.first().and_then(|row| catalog.row(*row).release_year);
                        let mut digits = [0u8; 24];
                        let year = stack_fmt(&mut digits, format_args!("{}", year.unwrap_or(0).max(0)));
                        let year = if year == "0" { "\u{2014}" } else { year };
                        line(ui, "year", year, (13.0, 18.2, 400, 0.0), ui.color(SECONDARY), Flags::NONE);
                    });
                    if ui.signal(&card).clicked {
                        *intent = Some(Intent::Navigate(Route::Album(index)));
                    }
                }
            });
        }
    });
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
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
            frame.time += 0.1;
            assert_eq!(
                run(&mut ui, &state, &frame, Some("Back")),
                Some(Intent::Navigate(if link { Route::Albums } else { Route::Artists }))
            );
            frame.time += 0.1;
            if link {
                assert_eq!(
                    run(&mut ui, &state, &frame, Some("Keep offline")),
                    Some(Intent::Page(crate::PageAction::KeepOffline))
                );
            } else {
                assert!(matches!(
                    run(&mut ui, &state, &frame, Some("B")),
                    Some(Intent::Navigate(Route::Album(_)))
                ), "album card opens its album");
            }
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
