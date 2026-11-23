//! Compact song lists (Library Songs, album/artist/list pages, Find): Swift's 56 pt rows with
//! worker-prepared "Artist · Album" lines, availability glyph, duration and a "…" menu, UIKit-style
//! two-sided swipe actions (Play next / Add to queue, or Remove inside a list), the A-Z index rail
//! and the artist page's lead-in. Every string is borrowed from the catalog or static.
use super::*;
use crate::browse::Sort;
use crate::catalog::Collection;
use pxui::SwipeActions;

const ROW: f32 = 56.0;
/// Width of the revealed strip on either side; a full swipe of `SWIPE_FULL` commits it.
const SWIPE_ACTION: f32 = 76.0;
const SWIPE_FULL: f32 = 88.0;
/// A row while selecting: no gesture.
const SELECTING_ROW: GestureOpts = GestureOpts::DEFAULT;
/// A row outside selection: long press and swipe actions on both sides.
const SWIPE_ROW: GestureOpts = GestureOpts::DEFAULT.long_press().swipe_actions(
    SwipeActions::DEFAULT.leading(SWIPE_ACTION).trailing(SWIPE_ACTION).full(SWIPE_FULL),
);
/// A drag handle or index strip: the touch is held at once.
const HELD_TOUCH: GestureOpts = GestureOpts::DEFAULT.touch_drag();
/// Pull to refresh at the framework's distance.
const PULL: GestureOpts = GestureOpts::DEFAULT.pull_to_refresh();

struct Cx<'a> {
    catalog: &'a Catalog,
    collection: &'a Collection,
    state: &'a ShellState,
    player: &'a PlayerView<'a>,
    width: f32,
    /// The playlist the rows belong to; its swipe removes instead of queueing.
    list: Option<i64>,
    /// Phone Select mode: checkboxes, tap toggles, no swipe or menu.
    selecting: bool,
    /// Album pages number rows by track instead of showing artwork.
    numbered: bool,
}

pub(super) fn songs(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    intent: &mut Option<Intent>,
) {
    if state.route == Route::Find {
        compact_find(ui, catalog, state, width, intent);
    }
    // Swift's compact Songs is always title-sorted; the worker projects it.
    if state.route == Route::Songs && state.sort == Sort::Original && intent.is_none() {
        *intent = Some(Intent::SortBy(Some((Sort::Title, true))));
    }
    let Some(collection) = state.collection(catalog) else {
        return;
    };
    let artist = match state.route {
        Route::Artist(index) => Some(index),
        _ => None,
    };
    let detail_page = matches!(state.route, Route::Album(_) | Route::Collection(_));
    if collection.rows.is_empty() && artist.is_none() && !detail_page {
        ui.label("No songs here yet.", EMPTY_NOTE);
        return;
    }
    let cx = Cx {
        catalog,
        collection,
        state,
        player,
        width,
        list: match &*collection.context {
            lw_core::context::PlaybackContext::Playlist { id, .. } => Some(*id),
            _ => None,
        },
        selecting: state.selecting,
        numbered: matches!(state.route, Route::Album(_)),
    };
    let lead = match state.route {
        Route::Artist(index) => artist_lead_height(catalog, index, width),
        Route::Album(_) => album_lead_height(),
        Route::Collection(_) => collection_lead_height(),
        _ => 0.0,
    };
    // Swift's Find "Play results" / "Keep results offline" keys close the song list (the list's trailer, one 56 pt row here); past
    // eight songs it lists the first eight over its "Show all N songs" key until that is pressed
    let find_keys = state.route == Route::Find;
    let capped = find_keys && !state.find_all && collection.rows.len() > FIND_FIRST;
    let shown = if capped { FIND_FIRST } else { collection.rows.len() };
    ui.stack(Node::stack().w(Size::Grow).h(Size::Grow), |ui| {
        let reorder = cx.selecting && cx.list.is_some();
        let list = PHONE_SONGS.lead(lead).trail(if find_keys { ROW } else { 0.0 } + if capped { SHOW_ALL } else { 0.0 } + if cx.selecting { BAR } else { 0.0 }).reorder(reorder)
            // Select mode toggles a row a tap; outside it a tap plays (the list selects nothing)
            .mode(if cx.selecting { pxui::SelectMode::Toggle } else { pxui::SelectMode::None }).open_on_click(!cx.selecting);
        let mut song_rows = state.song_rows.borrow_mut();
        // a kept row shows what these hold (U2b.4.13, as the desktop list); a swiped row follows its gesture, so it rebuilds every frame
        song_rows.revision = Some(crate::hash((
            (state.revision, std::sync::Arc::as_ptr(&catalog.library) as usize, collection.rows.as_ptr() as usize),
            (player.current_id, player.artwork.map_or(0, crate::artwork::Images::revision)),
            (cx.selecting, cx.list, cx.numbered, width.to_bits()),
        )));
        super::super::songs::mark_live(state, &mut song_rows);
        let r = ui.selection_list(
            "songs",
            shown,
            &|i| collection.row_keys[i],
            &mut song_rows,
            &list,
            &mut |ui, part, cell| match part {
                SectionPart::Leading => {
                    match state.route {
                        Route::Artist(index) => artist_lead(ui, &cx, index, intent),
                        Route::Album(_) => album_lead(ui, &cx, intent),
                        Route::Collection(_) => collection_lead(ui, &cx, intent),
                        _ => {}
                    }
                }
                SectionPart::Trailing if find_keys => {
                    if capped {
                        show_all(ui, &collection.count, intent);
                    }
                    find_result_keys(ui, catalog, intent);
                }
                SectionPart::Header | SectionPart::Trailing => {}
                SectionPart::Row(index) => row(ui, &cx, index, &cell, intent),
            },
        );
        drop(song_rows);
        if let Some(i) = r.selected
            && intent.is_none()
        {
            // a change that leaves no row marked (Escape) is the app's Clear
            let none = !state.song_rows.borrow().rows.iter().any(|f| f & pxui::ListState::SELECTED != 0);
            *intent = Some(if none { Intent::Select(Selection::Clear) } else { Intent::Selected(i) });
        }
        if let Some(i) = r.activated {
            match state.open_swipe.get() {
                // a tap on a row whose swipe actions show closes them
                Some((open, row)) if open == i => ui.close_swipe(&row),
                _ => *intent = Some(if state.marked_count() > 1 { Intent::PlaySelection } else { Intent::PlayRow(i) }),
            }
        }
        if let Some((from, to)) = r.moved {
            *intent = Some(Intent::MoveListRow { from, to });
        }
        let list = r.handle;
        // the window's default key target: Return plays the keyboard's row; Command-A and Escape mark in Select mode only
        ui.focus_default(&list);
        refresh(ui, &list, state, intent);
        if cx.selecting {
            ui.id("selection-bar", |ui| selection_bar(ui, &cx, intent));
        }
        if state.route == Route::Songs && state.sort == Sort::Title && state.ascending {
            let top = ui.content_rect().y.max(state.top_inset);
            let above = (ui.rect(&list).y - top).max(0.0);
            ui.id("rail", |ui| rail(ui, &list, (&collection.letters, &collection.starts), (ROW, collection.rows.len()), above));
        }
    });
}

/// The phone songs list (`localwave-row` / `PHONE_ROW`): 56-point rows, the overlay scrollbar, no frame.
const PHONE_SONGS: pxui::SelectionList = pxui::SelectionList::DEFAULT.row_h(ROW).overlay_scrollbar().variant(variant::PHONE_ROW).bare();

/// Row `index`'s content in its list row (`cell`, the body's stack on paper, the selected fill while marked): the swipe gestures on
/// the row, its content offset by the swipe over the revealed action, the mark, artwork or track number, labels, state glyph,
/// duration and "…" (the grip in a list's Select mode).
fn row(ui: &mut Ui, cx: &Cx<'_>, index: usize, cell: &pxui::CellState, intent: &mut Option<Intent>) {
    let Cx {
        catalog,
        collection,
        state,
        player,
        width,
        selecting,
        numbered,
        ..
    } = *cx;
    let row = catalog.row(collection.rows[index]);
    let node = cell.handle;
    ui.a11y_name(&node, row.title);
    // Touch only: a mouse never starts a gesture, so desktop rows read rest.
    let g = ui.gesture(&node, &if selecting { SELECTING_ROW } else { SWIPE_ROW });
    let mut content = Node::row().w(Size::Grow).h(Size::Grow).gap(12.0).align(Align::Center).padding(Edges::ZERO.l(16.0).r(6.0));
    if g.swipe != 0.0 {
        // `swipe` is the row's own x offset; LocalWave never mirrors, so positive reveals the leading side.
        content = content.w(Size::Px(width)).h(Size::Px(ROW)).at(g.swipe, 0.0).look(PAPER_FILL);
    }
    let mut more = false;
    let mut build = |ui: &mut Ui| { ui.id("song", |ui| { ui.row(content, |ui| {
        if selecting {
            ui.checkbox("mark", row.title, cell.selected, &MARK);
        }
        if numbered {
            // the track number, else the row's place: both prepared on the worker (`track_label`, the song table's numbers)
            let place = collection.song_table.as_ref().map_or("", |table| table.number_label(index));
            let number = crate::song_text::body_value(&row, crate::catalog::SongColumn::Number, true, place);
            ui.id("track", |ui| { ui.row(Node::row().w_px(22.0).h_px(18.2).justify(Justify::End), |ui| {
                ui.label(number, MONO_13.color(SECONDARY));
            }); });
        } else {
            artwork(ui, catalog, player, Some(collection.rows[index]), 40.0);
        }
        let labels = container(ui, "labels", Axis::Column, Size::Grow, Size::Fit, &PAPER_FILL);
        ui.within(&labels, &mut |ui| {
            ui.label(row.title, if player.current_id == Some(row.id) { ROW_TITLE_NOW } else { ROW_TITLE });
            // a blocked row's prepared line is "<source> needs reconnecting" whatever the page
            let note = if row.blocked {
                row.subtitle
            } else if selecting {
                row.artist
            } else if numbered {
                if row.availability == Some(lw_core::FileAvailability::InCloud) { row.cloud_note } else { row.file_note }
            } else {
                row.subtitle
            };
            ui.label(note, ROW_NOTE);
        });
        if selecting {
            if cx.list.is_some() {
                reorder_grip(ui);
            }
            return;
        }
        let glyph = row.blocked
            || row.pinned
            || row.availability != Some(lw_core::FileAvailability::OnDevice)
            || matches!(row.state, lw_core::FileState::CopyPending | lw_core::FileState::Failed);
        // Swift's state glyph and time share an inner row 8 apart; a blocked row has no time
        if glyph || !row.blocked {
            ui.id("state-time", |ui| { ui.row(Node::row().w(Size::Fit).h(Size::Fit).gap(8.0).align(Align::Center), |ui| {
                if glyph {
                    ui.stack(Node::stack().w_px(if row.pinned { 14.0 } else { 10.0 }).h_px(10.0).center(), |ui| {
                        super::super::songs::row::availability(ui, row, true);
                    });
                }
                if !row.blocked {
                    ui.label(row.duration, MONO_DIM);
                }
            }); });
        }
        // a blank where a blocked row's actions key was
        if row.blocked {
            ui.node("actions", Node::row().w_px(44.0).h_px(44.0));
        } else {
            more = ui.icon_button("actions", icon_sized(Icon::More, 18.0), row.actions_label, &MORE).clicked;
        }
    }); }); };
    if row.blocked {
        // Swift fades the whole blocked row to 0.45; a view's opacity fades its subtree. Its key holds what the row shows.
        let key = (row.id, player.current_id, player.artwork.map_or(0, crate::artwork::Images::revision), width.to_bits(), selecting, numbered, g.swipe.to_bits());
        let faded = ui.view_of("blocked", &key, &mut build);
        ui.set_view(&faded, &pxui::ViewTransform::DEFAULT.opacity(0.45));
    } else {
        build(ui);
    }
    // Swift's hairline over every row but the first, 16 points in
    if index > 0 {
        ui.canvas_over(&node, |paint, rect| paint.fill(Rect::new(rect.x + 16.0, rect.y, rect.w - 16.0, 1.0), Fill::new(Rgba::token(LINE)).fractional()));
    }
    let active = g.swiping || g.swipe_open;
    // a row off rest (swiping, open or settling) rebuilds every frame until it is back: its last settling frame resets `open_swipe`
    if active || g.swipe != 0.0 || state.open_swipe.get().is_some_and(|(open, _)| open == index) {
        state.song_live.borrow_mut().1.push(index);
    }
    #[cfg(test)]
    super::tests::ROW_BUILDS.set(super::tests::ROW_BUILDS.get() + 1);
    match state.open_swipe.get() {
        Some((open, other)) if active && open != index => {
            ui.close_swipe(&other);
            state.open_swipe.set(Some((index, node)));
        }
        Some((open, _)) if open == index && active => {
            state.open_swipe.set(Some((index, node)));
        }
        Some((open, _)) if open == index && g.swipe == 0.0 => state.open_swipe.set(None),
        None if active => state.open_swipe.set(Some((index, node))),
        _ => {}
    }
    if g.highlighted {
        ui.style(&node, &pxui::theme::States::of(SELECTED_FILL), State::default());
    }
    if more {
        let rect = ui.rect(&node);
        *intent = Some(Intent::SongMenu(
            index,
            row.id,
            collection.key,
            (rect.x + rect.w - 44.0, rect.y),
        ));
    }
    let leading = g.swipe > 0.0;
    let (key, label, trailing) = match cx.list {
        Some(list) => (
            "remove",
            "Remove",
            Intent::RemoveFromList { song: row.id, list },
        ),
        None => ("add-to-queue", "Add to queue", Intent::AppendSong(row.id)),
    };
    let (key, label, action) = if leading {
        ("play-next", "Play next", Intent::PlayNext(row.id))
    } else {
        (key, label, trailing)
    };
    if g.swipe != 0.0 {
        ui.id("swipe-actions", |ui| { ui.row(Node::row().w(Size::Grow).h(Size::Grow).justify(if leading { Justify::Start } else { Justify::End }).z(-1),
            |ui| {
                let revealed = g.swipe.abs();
                let style = if revealed >= SWIPE_FULL { SWIPE_ARMED } else { SWIPE };
                if ui.button(key, label, &style.size(Size::Px(revealed.max(SWIPE_ACTION)), Size::Grow)).clicked {
                    *intent = Some(action);
                    ui.close_swipe(&node);
                }
            },
        ); });
    }
    // A full swipe lasts one frame; the row settles closed on its own.
    if intent.is_none() && (g.swipe_commit_leading || g.swipe_commit_trailing) {
        *intent = Some(if g.swipe_commit_leading {
            Intent::PlayNext(row.id)
        } else {
            trailing
        });
    }
    if g.long_pressed && intent.is_none() {
        let rect = ui.rect(&node);
        *intent = Some(Intent::SongMenu(
            index,
            row.id,
            collection.key,
            (rect.x + rect.w * 0.5 - 100.0, rect.y + rect.h),
        ));
    }
}

/// Swift's edit-mode drag handle: the list's handle (a press lifts the row at once, touch too; the list reports the move, and each
/// row offers Move up and Move down to assistive technology).
fn reorder_grip(ui: &mut Ui) {
    let grip = ui.part(pxui::Part::HANDLE, Node::column().w_px(44.0).h_px(44.0));
    ui.a11y_name(&grip, "Reorder track");
    ui.canvas(&grip, move |paint, rect| {
        for y in [-5.0, 0.0, 5.0] {
            paint.fill(Rect::new(rect.x + 13.0, rect.y + rect.h * 0.5 + y - 1.0, 18.0, 2.0), Fill::new(Rgba::token(SECONDARY)));
        }
    });
}

/// A swipe action key (`localwave-button` / `ROW`, centred): secondary on the panel fill; `SWIPE_ARMED` past a full swipe, paper on
/// ink. The caller sizes it to what the swipe has revealed.
const SWIPE: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).centered().label(TEXT_13.color(SECONDARY)).look(&PANEL_LOOKS);
const SWIPE_ARMED: pxui::Button = SWIPE.label(TEXT_13.color(PAPER)).look(&INK_LOOKS);

/// Swift's LWMenuButton: a secondary "…" in a 44 pt target.
pub(in crate::shell) const MORE: pxui::IconButton = ICON_44.look(&ICON_DIM_LOOKS);

/// `refreshing` holds from release until `end_refresh`, so its first frame starts the load; it can
/// read once more after the end, which `refresh_ending` keeps from starting another.
fn refresh(ui: &mut Ui, list: &Handle, state: &ShellState, intent: &mut Option<Intent>) {
    let pull = ui.gesture(list, &PULL);
    if !pull.refreshing {
        state.refresh_ending.set(false);
        state.refresh_wait.set(None);
    } else if let Some((epoch, since)) = state.refresh_wait.get() {
        // Ends on the next catalog delivery; the deadline keeps a refresh that changed nothing from holding.
        if epoch != state.refresh_epoch || ui.now() - since > 5.0 {
            ui.end_refresh(list);
            state.refresh_wait.set(None);
            state.refresh_ending.set(true);
        } else {
            ui.repaint_after((5.0 - (ui.now() - since)).max(0.0));
        }
    } else if !state.refresh_ending.get() {
        state
            .refresh_wait
            .set(Some((state.refresh_epoch, ui.now())));
        if intent.is_none() {
            *intent = Some(Intent::RefreshLibrary);
        }
    }
}

/// Scroll so `target` starts the viewport (reveal only scrolls the minimum, so aim at the row that
/// ends it when moving down).
fn jump(ui: &mut Ui, list: &Handle, target: usize, count: usize, row_height: f32) {
    let visible = ((ui.rect(list).h / row_height) as usize).max(1);
    let top = (ui.scroll(list).painted / row_height) as usize;
    ui.reveal(
        list,
        if target < top {
            target
        } else {
            (target + visible - 1).min(count - 1)
        },
    );
}

/// A press or drag over a letter jumps to the first row in that section.
/// Swift overlays the page's scroll view, header and list: the rail also accounts for the header above the list.
pub(super) fn rail(
    ui: &mut Ui,
    list: &Handle,
    (letters, starts): (&[&str], &[usize]),
    (row_height, count): (f32, usize),
    above: f32,
) {
    ui.row(Node::row().w(Size::Grow).h(Size::Grow).align(Align::Center).justify(Justify::End).margin(Edges::ZERO.t(-above)), |ui| {
        let index = ui.section_index("index", letters, &RAIL);
        // A finger on the rail is a held press-and-drag, not a scroll or a row swipe underneath.
        ui.gesture(&index.response.handle, &HELD_TOUCH);
        if let Some(letter) = index.chosen {
            jump(ui, list, starts[letter], count, row_height);
        }
    });
}
const RAIL: pxui::SectionIndex = pxui::SectionIndex::DEFAULT.name("Section index");

const BACK_ROW: f32 = 54.0;
const TITLE_BLOCK: f32 = 33.6 + 4.0 + 18.2;
const PLAY_BAR: f32 = 52.0;
const SECTION_LABEL: f32 = 20.0 + 18.2 + 8.0;
const CARD_GAP: f32 = 12.0;

fn album_cell(width: f32) -> f32 {
    ((width - 32.0 - CARD_GAP) / 2.0).floor()
}
fn album_card(width: f32) -> f32 {
    album_cell(width) + 8.0 + 17.5 + 18.2
}
fn artist_lead_height(catalog: &Catalog, artist: usize, width: f32) -> f32 {
    let albums = catalog.artist_albums.get(artist).map_or(0, Vec::len);
    let rows = albums.div_ceil(2);
    BACK_ROW
        + TITLE_BLOCK
        + 16.0
        + PLAY_BAR
        + 16.0
        + rows as f32 * album_card(width)
        + rows.saturating_sub(1) as f32 * 20.0
        + 16.0
        + SECTION_LABEL
}

/// Swift's compact artistDetail: back, name and summary, Play/Shuffle keys, album cards, "Songs".
fn artist_lead(ui: &mut Ui, cx: &Cx<'_>, artist: usize, intent: &mut Option<Intent>) {
    let Cx {
        catalog,
        collection,
        state,
        player,
        width,
        ..
    } = *cx;
    let albums = catalog
        .artist_albums
        .get(artist)
        .map_or(&[][..], Vec::as_slice);
    let revision = crate::hash((
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        width.to_bits(),
        player.artwork.map_or(0, crate::artwork::Images::revision),
        select_revision(state),
    ));
    ui.id("artist-lead", |ui| { ui.column(Node::column().h_px(artist_lead_height(catalog, artist, width)), |ui| { ui.view_of("artist-lead", &revision, |ui| {
            let column = container(ui, "lead", Axis::Column, Size::Grow, Size::Grow, &PAPER_FILL);
            ui.within(&column, &mut |ui| {
                nav_row(ui, cx, "Library", Route::Artists, intent);
                let name = if collection.title.is_empty() { "Unknown artist" } else { &collection.title };
                titles(ui, name, &collection.summary);
                spacer(ui, "keys-gap", 16.0);
                play_keys(ui, None, !collection.rows.is_empty(), intent);
                spacer(ui, "albums-gap", 16.0);
                let cell = album_cell(width);
                for (line, pair) in albums.chunks(2).enumerate() {
                    if line > 0 {
                        spacer(ui, "album-row-gap", 20.0);
                    }
                    let cards = ui.node(
                        Key::Id(line as i64),
                        Node::row().w(Size::Grow).h_px(album_card(width)).gap(CARD_GAP).pad_x(16.0),
                    );
                    ui.within(&cards, &mut |ui| {
                        for index in pair.iter().copied() {
                            let group = &catalog.albums[index];
                            let node = Node::column().w_px(cell).h(Size::Grow).role(Role::Button);
                            let card = ui.pressable(Key::Id(index as i64), node, &PLAIN, |ui, _| {
                                let art = container(
                                    ui,
                                    "artwork",
                                    Axis::Stack,
                                    Size::Px(cell),
                                    Size::Px(cell),
                                    &PANEL_FILL,
                                );
                                paint_artwork(
                                    ui,
                                    &art,
                                    group.artwork_row.map(|row| catalog.row(row)),
                                    cell,
                                    player.artwork,
                                );
                                spacer(ui, "art-gap", 8.0);
                                ui.label(&group.title, CARD_TITLE);
                                let year = group.rows.first().and_then(|row| catalog.row(*row).release_year);
                                let mut digits = [0u8; 24];
                                let year = stack_fmt(&mut digits, format_args!("{}", year.unwrap_or(0).max(0)));
                                ui.label(if year == "0" { "" } else { year }, SUMMARY);
                            });
                            ui.a11y_name(&card.handle, &group.title);
                            if card.clicked {
                                *intent = Some(Intent::Navigate(Route::Album(index)));
                            }
                        }
                    });
                }
                // Swift's album grid pads 16 under its cards
                spacer(ui, "songs-gap", 16.0);
                ui.column(Node::column().w(Size::Grow).h_px(SECTION_LABEL).padding(Edges::ZERO.l(16.0).t(20.0).b(8.0)),
                    |ui| {
                        ui.label("Songs", SECTION_CAPTION);
                    },
                );
            });
        }); }); });
}

/// A 44-point Back key with an 18-point chevron and the parent's name.
pub(in crate::shell) const BACK_KEY: pxui::Button =
    pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Fit, Size::Px(44.0)).align(Justify::Start)
        .pad(Edges::ZERO.l(10.0).r(12.0)).gap(4.0).icon(icon_drawing(Icon::Back)).icon_size(18.0)
        .label(LEAD_DIM).look(&DIM_LOOKS).name("Back");
/// Swift's detail back row: chevron and the parent's name, 44 pt tall.
fn back_row(
    ui: &mut Ui,
    label: &'static str,
    parent: Route,
    intent: &mut Option<Intent>,
    extras: &mut dyn FnMut(&mut Ui, &mut Option<Intent>),
) {
    ui.id("back-row", |ui| { ui.row(Node::row().w(Size::Grow).h_px(BACK_ROW).gap(4.0).align(Align::Center).padding(Edges::ZERO.t(10.0).r(16.0)),
        |ui| {
            if ui.button("back", label, &BACK_KEY).clicked {
                *intent = Some(Intent::Navigate(parent));
            }
            extras(ui, intent);
        },
    ); });
}

/// Name (32 pt) over a 13 pt summary.
fn titles(ui: &mut Ui, title: &str, summary: &str) {
    ui.column(Node::column().w(Size::Grow).h_px(TITLE_BLOCK).pad_x(16.0), |ui| {
        ui.label(title, PAGE_TITLE);
        spacer(ui, "gap", 4.0);
        ui.label(summary, SUMMARY);
    });
}

/// The songs Find lists before its "Show all" key.
const FIND_FIRST: usize = 8;
/// Swift's "Show all N songs" key: 8 points down, 56 high on the panel, 16 in at each side.
const SHOW_ALL: f32 = 8.0 + 56.0;
const SHOW_ALL_KEY: pxui::Button = pxui::Button::DEFAULT.variant(variant::KEY).size(Size::Grow, Size::Px(56.0)).centered().label(TEXT_15.color(INK)).look(&PANEL_LOOKS).name("Show all songs");
fn show_all(ui: &mut Ui, count: &str, intent: &mut Option<Intent>) {
    ui.row(Node::row().w(Size::Grow).h_px(SHOW_ALL).padding(Edges::ZERO.l(16.0).r(16.0).t(8.0)), |ui| {
        // the count is worker-prepared; the words around it are fixed: one label per part, no formatting a frame
        ui.id("show-all", |ui| {
            if ui.button_content("show-all", &SHOW_ALL_KEY, |ui, _| {
                ui.row(Node::row().w(Size::Fit).h(Size::Fit).gap(4.0), |ui| {
                    ui.label("Show all", TEXT_15.color(INK).role(Role::Unknown));
                    ui.label(count, TEXT_15.color(INK).role(Role::Unknown));
                    ui.label("songs", TEXT_15.color(INK).role(Role::Unknown));
                });
            }).clicked {
                *intent = Some(Intent::FindShowAll);
            }
        });
    });
}

fn find_result_keys(ui: &mut Ui, catalog: &Catalog, intent: &mut Option<Intent>) {
    let kept = catalog.view.as_ref().and_then(|view| view.find.as_ref()).is_some_and(|find| find.pinned);
    ui.row(Node::row().w(Size::Grow).h_px(ROW).gap(8.0).padding(Edges::ZERO.l(16.0).r(16.0).t(2.0)), |ui| {
        if ui.button("play-results", "Play results", &PLAY_KEY.icon(icon_drawing(Icon::Play))).clicked {
            *intent = Some(Intent::PlayRow(0));
        }
        let (label, icon) = if kept { ("Kept offline", Icon::Check) } else { ("Keep results offline", Icon::Down) };
        if ui.button("keep-results-offline", label, &PLAY_KEY.icon(icon_drawing(icon)).selected(kept)).clicked {
            *intent = Some(Intent::TogglePins);
        }
    });
}

/// Play and Shuffle keys.
fn play_keys(ui: &mut Ui, offline: Option<bool>, enabled: bool, intent: &mut Option<Intent>) {
    ui.row(Node::row().w(Size::Grow).h_px(PLAY_BAR).gap(8.0).pad_x(16.0), |ui| {
        if ui.button("play", "Play", &PLAY_PRIMARY.icon(icon_drawing(Icon::Play)).icon_size(18.0).disabled(!enabled)).clicked {
            *intent = Some(Intent::PlayRow(0));
        }
        if ui.button("shuffle", "Shuffle", &PLAY_KEY.icon(icon_drawing(Icon::Shuffle)).icon_size(18.0).disabled(!enabled)).clicked {
            *intent = Some(Intent::ShuffleCollection);
        }
        if let Some(kept) = offline
            && ui.button("keep-offline", if kept { "Kept offline" } else { "Keep offline" }, &offline_key(kept).disabled(!enabled)).clicked
        {
            *intent = Some(Intent::Page(crate::PageAction::KeepOffline));
        }
    });
}

/// Swift's third play key on albums only (its list page has Play and Shuffle).
fn offline_state(cx: &Cx<'_>) -> Option<bool> {
    matches!(&*cx.collection.context, lw_core::context::PlaybackContext::Album { .. })
        .then(|| !cx.collection.rows.is_empty() && cx.collection.rows.iter().all(|&row| cx.catalog.row(row).pinned))
}
/// The Keep offline key: the offline icon (Kept while kept) and checked while kept.
pub(in crate::shell) const fn offline_key(kept: bool) -> pxui::Button<'static> {
    PLAY_KEY.icon(icon_drawing(if kept { Icon::Kept } else { Icon::Offline })).selected(kept)
}
/// Swift's playKeyButtons (`localwave-button` / `KEY`): its slot's width, `PLAY_BAR` high, the icon and 15-point label centred 8
/// apart, ink on the panel fill; `PLAY_PRIMARY` paper on ink (Play).
pub(in crate::shell) const PLAY_KEY: pxui::Button = pxui::Button::DEFAULT.variant(variant::KEY).size(Size::Grow, Size::Px(PLAY_BAR)).gap(8.0).centered().label(TEXT_15).look(&PLAY_KEY_LOOKS);
pub(in crate::shell) const PLAY_PRIMARY: pxui::Button = PLAY_KEY.look(&PLAY_PRIMARY_LOOKS);

/// Select-mode part of a lead's cache key: the marks only matter while selecting.
fn select_revision(state: &ShellState) -> (bool, u64) {
    (state.selecting, if state.selecting { state.selection_revision } else { 0 })
}

const BAR: f32 = 64.0;

/// Back row, or Swift's selection header (Select all, "N selected", Done) in Select mode.
/// Albums and lists add the page "…" menu; artists only Select.
fn nav_row(ui: &mut Ui, cx: &Cx<'_>, label: &'static str, parent: Route, intent: &mut Option<Intent>) {
    let Cx { collection, state, .. } = *cx;
    let playlist = cx.list.is_some();
    if !state.selecting {
        back_row(ui, label, parent, intent, &mut |ui, intent| {
            container(ui, "nav-space", Axis::Row, Size::Grow, Size::Px(1.0), &PAPER_FILL);
            // Playlists edit (select and remove); albums and automatic lists select; Swift's artist page has Back alone.
            if !(playlist && collection.rows.is_empty()) && !matches!(state.route, Route::Artist(_))
                && ui.button("select", if playlist { "Edit" } else { "Select" }, &WORD).clicked
            {
                *intent = Some(Intent::SelectMode(true));
            }
            if !matches!(state.route, Route::Artist(_))
                && ui.icon_button("page-actions", icon_sized(Icon::More, 22.0), if playlist { "List actions" } else { "Album actions" }, &ICON_44.look(&ICON_LOOKS)).clicked
            {
                *intent = Some(Intent::PageMenu((cx.width - 16.0, BACK_ROW)));
            }
        });
        return;
    }
    selection_header(ui, state.marked_count(), collection.rows.len(), collection.rows.len(), intent);
}

/// Swift's phone Songs and Find reach select mode only through an accessibility action ("Select songs" on its sort button): a
/// 1-point unpainted button named "Select" that assistive technologies press and the page never shows.
pub(in crate::shell) fn select_for_assistive(ui: &mut Ui) -> bool {
    const SELECT: pxui::Button = pxui::Button::DEFAULT.variant(variant::ASSISTIVE);
    ui.button("select", "Select", &SELECT).clicked
}

/// Swift's WaveMobileSelectionHeader: Select all, "N selected", Done. `selectable` rows count toward "all";
/// "Select all" marks `rows` positions.
pub(in crate::shell) fn selection_header(ui: &mut Ui, selected: usize, selectable: usize, rows: usize, intent: &mut Option<Intent>) {
    ui.id("selection-header", |ui| { ui.row(Node::row().w(Size::Grow).h_px(BACK_ROW).align(Align::Center).padding(Edges::ZERO.t(10.0).l(12.0).r(12.0)), |ui| {
        let all = selected > 0 && selected == selectable;
        if ui.button("select-all", if all { "Deselect all" } else { "Select all" }, &WORD).clicked {
            *intent = Some(Intent::Select(if all { Selection::Clear } else { Selection::All(rows) }));
        }
        container(ui, "left-space", Axis::Row, Size::Grow, Size::Px(1.0), &PAPER_FILL);
        let mut buffer = [0u8; 40];
        ui.label(selected_label(&mut buffer, selected), LEAD_INK);
        container(ui, "right-space", Axis::Row, Size::Grow, Size::Px(1.0), &PAPER_FILL);
        if ui.button("done", "Done", &WORD).clicked {
            *intent = Some(Intent::SelectMode(false));
        }
    }); });
}

/// "N selected" without a heap allocation.
fn selected_label(buffer: &mut [u8; 40], count: usize) -> &str {
    use std::io::Write;
    let mut rest = &mut buffer[..];
    let _ = write!(rest, "{count} selected");
    let used = 40 - rest.len();
    std::str::from_utf8(&buffer[..used]).unwrap_or("")
}

/// Swift's 44 pt text button (Select, Done, Select all at 16 pt, `localwave-button` / `ROW`): as wide as its word plus 4 points
/// either side, ink.
pub(in crate::shell) const WORD: pxui::Button =
    pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Fit, Size::Px(44.0)).centered().pad(Edges::ZERO.l(4.0).r(4.0)).label(TEXT_16.color(INK));

/// Swift's 20 pt selection mark (`localwave-chip` / `MARK`): a passive checkbox, the row carries the action.
pub(in crate::shell) const MARK: pxui::Checkbox = pxui::Checkbox::DEFAULT.size(20.0).passive(true).variant(variant::MARK);

/// A selection deck key (`localwave-button` / `KEY`, its icon over its 12-point label): ink on the key plate, tertiary while
/// disabled; `DECK_REMOVE` destructive (`Button::danger`: the danger colour).
const DECK: pxui::Button = pxui::Button::DEFAULT.variant(variant::KEY).size(Size::Grow, Size::Grow).axis(Axis::Column).gap(4.0).centered().label(TEXT_12).look(&DECK_LOOKS);
const DECK_REMOVE: pxui::Button = DECK.danger();
/// Swift's selection deck: Play next, Add to list, Offline, then Queue (Remove inside a list). Returns the pressed
/// key ("play-next", "add-to-list", "offline", "queue" or "remove") and the key's rectangle.
pub(in crate::shell) fn deck(ui: &mut Ui, enabled: bool, remove: bool) -> Option<(&'static str, pxui::geom::Rect)> {
    let mut pressed = None;
    ui.column(Node::column().w(Size::Grow).h(Size::Grow).justify(Justify::End), |ui| {
        let deck = ui.node("selection-bar", Node::row().w(Size::Grow).h_px(BAR).gap(2.0).padding(Edges::ZERO.l(2.0).r(2.0).t(2.0)).look(PANEL_FILL));
        ui.set_role(&deck, Role::Group, pxui::A11yStates::NONE);
        ui.a11y_name(&deck, "Selection actions");
        ui.within(&deck, &mut |ui| {
            for (key, label, glyph) in [
                ("play-next", "Play next", Icon::Next),
                ("add-to-list", "Add to list", Icon::Add),
                ("offline", "Offline", Icon::Offline),
                (if remove { "remove" } else { "queue" }, if remove { "Remove" } else { "Queue" }, if remove { Icon::Clear } else { Icon::Queue }),
            ] {
                let style = if key == "remove" { DECK_REMOVE } else { DECK };
                let r = ui.button(key, label, &style.icon(icon_drawing(glyph)).disabled(!enabled));
                if r.clicked {
                    pressed = Some((key, ui.rect(&r.handle)));
                }
            }
        });
    });
    pressed
}

fn selection_bar(ui: &mut Ui, cx: &Cx<'_>, intent: &mut Option<Intent>) {
    let Cx { collection, state, .. } = *cx;
    let first = state.first_marked().and_then(|index| collection.ids.get(index).map(|&id| (index, id)));
    let remove = cx.list;
    if let Some((key, rect)) = deck(ui, first.is_some(), remove.is_some()) {
        let (index, id) = first.unwrap_or_default();
        *intent = Some(match key {
            "play-next" => Intent::PlayNext(id),
            "add-to-list" => Intent::SelectionLists(index, id, collection.key, (rect.x, rect.y - 36.0 * 8.0)),
            "offline" => Intent::KeepOffline(id),
            "remove" => Intent::RemoveFromList { song: id, list: remove.unwrap_or_default() },
            _ => Intent::AppendSong(id),
        });
    }
}

/// Compact album page lead-in (Swift's compact albumDetail): back, 4 points down the 152 pt artwork beside title, artist,
/// summary and format, bottom-aligned, then Play/Shuffle 16 points around.
const ALBUM_ART: f32 = 152.0;
fn album_lead_height() -> f32 {
    BACK_ROW + 4.0 + ALBUM_ART + 16.0 + PLAY_BAR + 16.0
}
fn album_lead(ui: &mut Ui, cx: &Cx<'_>, intent: &mut Option<Intent>) {
    let Cx { catalog, collection, state, player, width, .. } = *cx;
    let revision = crate::hash((
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        width.to_bits(),
        player.artwork.map_or(0, crate::artwork::Images::revision),
        select_revision(state),
    ));
    ui.id("album-lead", |ui| { ui.column(Node::column().h_px(album_lead_height()), |ui| { ui.view_of("album-lead", &revision, |ui| {
        let column = container(ui, "lead", Axis::Column, Size::Grow, Size::Grow, &PAPER_FILL);
        ui.within(&column, &mut |ui| {
            nav_row(ui, cx, "Library", Route::Albums, intent);
            spacer(ui, "art-gap", 4.0);
            ui.row(Node::row().w(Size::Grow).h_px(ALBUM_ART).gap(16.0).pad_x(16.0), |ui| {
                let art = container(ui, "artwork", Axis::Stack, Size::Px(ALBUM_ART), Size::Px(ALBUM_ART), &PANEL_FILL);
                paint_artwork(ui, &art, collection.artwork_row.map(|row| catalog.row(row)), ALBUM_ART, player.artwork);
                ui.a11y_name(&art, "Album artwork");
                ui.column(Node::column().w(Size::Grow).h(Size::Grow).justify(Justify::End), |ui| {
                    ui.label(&collection.title, PHONE_ALBUM_TITLE);
                    if let lw_core::context::PlaybackContext::Album { artist, .. } = collection.context.as_ref()
                        && !artist.is_empty()
                    {
                        spacer(ui, "artist-gap", 2.0);
                        if ui.button("artist", crate::clip(artist), &detail::link(17.0, 23.8)).clicked
                            && let Some(index) = catalog.library.artists.iter().position(|a| a.title == crate::clip(artist))
                        {
                            *intent = Some(Intent::Navigate(Route::Artist(index)));
                        }
                    }
                    spacer(ui, "summary-gap", 8.0);
                    ui.label(&collection.summary, SUMMARY);
                    spacer(ui, "format-gap", 2.0);
                    ui.label(&collection.format, MONO_12.color(SECONDARY));
                });
            });
            spacer(ui, "keys-gap", 16.0);
            play_keys(ui, offline_state(cx), !collection.rows.is_empty(), intent);
        });
    }); }); });
}

/// Compact list page lead-in (playlists, Favorites and the other automatic lists).
fn collection_lead_height() -> f32 {
    BACK_ROW + TITLE_BLOCK + 16.0 + PLAY_BAR + 16.0
}
fn collection_lead(ui: &mut Ui, cx: &Cx<'_>, intent: &mut Option<Intent>) {
    let Cx { catalog, collection, state, .. } = *cx;
    let revision = crate::hash((
        state.revision,
        collection.key,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        select_revision(state),
    ));
    ui.id("collection-lead", |ui| { ui.column(Node::column().h_px(collection_lead_height()), |ui| { ui.view_of("collection-lead", &revision, |ui| {
        let column = container(ui, "lead", Axis::Column, Size::Grow, Size::Grow, &PAPER_FILL);
        ui.within(&column, &mut |ui| {
            nav_row(ui, cx, "Lists", Route::Lists, intent);
            // Swift's phone list page counts songs only; automatic lists keep "updates itself"
            let summary = match collection.context.as_ref() {
                lw_core::context::PlaybackContext::Playlist { .. } => collection.summary.split(" · ").next().unwrap_or_default(),
                _ => collection.summary.as_str(),
            };
            titles(ui, &collection.title, summary);
            spacer(ui, "keys-gap", 16.0);
            play_keys(ui, offline_state(cx), !collection.rows.is_empty(), intent);
            spacer(ui, "list-gap", 16.0);
        });
    }); }); });
}
