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
    // Swift's Find "Play results" / "Keep results offline" keys close the song list (one 56 pt row here).
    let find_keys = state.route == Route::Find;
    ui.stack(Node::stack().w(Size::Grow).h(Size::Grow), |ui| {
        let list = ui.section_list(
            "compact-songs",
            &SectionList::new(collection.rows.len() + usize::from(find_keys)).row_h(ROW).lead(lead).trail(if cx.selecting { BAR } else { 0.0 }).overlay_scrollbar(),
            // the find keys row after the songs has no song
            &|i| collection.row_keys.get(i).copied().unwrap_or(0),
            &mut |ui, part, _| match part {
                SectionPart::Leading => {
                    match state.route {
                        Route::Artist(index) => artist_lead(ui, &cx, index, intent),
                        Route::Album(_) => album_lead(ui, &cx, intent),
                        Route::Collection(_) => collection_lead(ui, &cx, intent),
                        _ => {}
                    }
                }
                SectionPart::Header | SectionPart::Trailing => {}
                SectionPart::Row(index) if index == collection.rows.len() => {
                    find_result_keys(ui, catalog, intent)
                }
                SectionPart::Row(index) => row(ui, &cx, index, intent),
            },
        );
        let list = list.handle;
        refresh(ui, &list, state, intent);
        if cx.selecting {
            ui.id("selection-bar", |ui| selection_bar(ui, &cx, intent));
        }
        if state.route == Route::Songs && state.sort == Sort::Title && state.ascending {
            ui.id("rail", |ui| rail(ui, &list, collection));
        }
    });
}

fn row(ui: &mut Ui, cx: &Cx<'_>, index: usize, intent: &mut Option<Intent>) {
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
    let cell = ui.node("swipe-row", Node::stack().w(Size::Grow).h(Size::Grow).flags(Flags::CLIP));
    ui.within(&cell, &mut |ui| {
        let selected = state.selected == Some(index);
        let row_node = Node::row().w(Size::Grow).h(Size::Grow).gap(12.0).align(Align::Center).role(Role::ListItem).padding(Edges::ZERO.l(16.0).r(6.0)).look(if selected { SELECTED_FILL } else { PAPER_FILL });
        // Touch gestures are taken on the row after it is declared; `more` is read once the row exists.
        let mut more = false;
        let node = ui.pressable("song", row_node, &PLAIN, |ui, _| {
            if selecting {
                ui.checkbox("mark", row.title, state.is_marked(index), &MARK);
            }
            if numbered {
                let mut digits = [0u8; 24];
                let digits = stack_fmt(&mut digits, format_args!("{}", row.track_number.map_or(index as u64 + 1, |n| n.max(0) as u64)));
                ui.id("track", |ui| { ui.row(Node::row().w_px(22.0).h_px(18.2).justify(Justify::End), |ui| {
                    ui.label(digits, MONO_13.color(SECONDARY));
                }); });
            } else {
                artwork(ui, catalog, player, Some(collection.rows[index]), 40.0);
            }
            let labels = container(ui, "labels", Axis::Column, Size::Grow, Size::Fit, &PAPER_FILL);
            ui.within(&labels, &mut |ui| {
                ui.label(row.title, if player.current_id == Some(row.id) { ROW_TITLE_NOW } else { ROW_TITLE });
                let note = if selecting {
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
                    reorder_grip(ui, index, row.title);
                }
                return;
            }
            if row.pinned
                || row.availability != Some(lw_core::FileAvailability::OnDevice)
                || matches!(
                    row.state,
                    lw_core::FileState::CopyPending | lw_core::FileState::Failed
                )
            {
                let holder = container(
                    ui,
                    "state",
                    Axis::Row,
                    Size::Px(if row.pinned { 14.0 } else { 10.0 }),
                    Size::Px(10.0),
                    &PAPER_FILL,
                );
                ui.within(&holder, &mut |ui| {
                    super::super::songs::row::glyph(
                        ui,
                        (row.state, row.availability, row.pinned),
                        true,
                    );
                });
            }
            ui.label(row.duration, MONO_DIM);
            more = ui.icon_button("actions", icon_drawing(Icon::More), row.actions_label, &MORE).clicked;
        });
        // Touch only: a mouse never starts a gesture, so desktop rows read rest.
        let g = ui.gesture(
            &node.handle,
            &if selecting { SELECTING_ROW } else { SWIPE_ROW },
        );
        let active = g.swiping || g.swipe_open;
        match state.open_swipe.get() {
            Some((open, other)) if active && open != index => {
                ui.close_swipe(&other);
                state.open_swipe.set(Some((index, node.handle)));
            }
            Some((open, _)) if open == index && active => {
                state.open_swipe.set(Some((index, node.handle)));
            }
            Some((open, _)) if open == index && g.swipe == 0.0 => state.open_swipe.set(None),
            None if active => state.open_swipe.set(Some((index, node.handle))),
            _ => {}
        }
        if g.swipe != 0.0 {
            // `swipe` is the row's own x offset; LocalWave never mirrors, so positive reveals the leading side.
            ui.set_node_layout(
                &node.handle,
                &ui.node_layout(&node.handle).w(Size::Px(width)).h(Size::Px(ROW)).at(g.swipe, 0.0),
            );
        }
        if g.highlighted {
            ui.style(
                &node.handle,
                &pxui::theme::States::of(row_node.look.fill(Rgba::token(SELECTED))),
                State::default(),
            );
        }
        ui.a11y_name(&node.handle, row.title);
        let clicked = node.clicked;
        let node = node.handle;
        if more {
            let rect = ui.rect(&node);
            *intent = Some(Intent::SongMenu(
                index,
                row.id,
                collection.key,
                (rect.x + rect.w - 44.0, rect.y),
            ));
        }
        if selecting && cx.list.is_some() {
            super::super::songs::row::drop_target(ui, &node, index, intent);
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
        if clicked && intent.is_none() {
            if selecting {
                *intent = Some(Intent::Select(Selection::Toggle(index)));
            } else if g.swipe_open {
                ui.close_swipe(&node);
            } else {
                *intent = Some(Intent::PlayRow(index));
            }
        }
    });
}

/// Swift's edit-mode drag handle: dragging it reorders the playlist (`MoveListRow` on drop).
fn reorder_grip(ui: &mut Ui, index: usize, title: &str) {
    use super::super::songs::row::LIST_ROW;
    let grip = ui.pressable("reorder", Node::column().w_px(44.0).h_px(44.0).role(Role::Button), &PLAIN, |_, _| {});
    ui.a11y_name(&grip.handle, "Reorder track");
    // The finger takes the handle at once as a held press, so the list neither scrolls nor refreshes.
    ui.gesture(&grip.handle, &HELD_TOUCH);
    ui.canvas(&grip.handle, move |paint, rect| {
        for y in [-5.0, 0.0, 5.0] {
            paint.fill(Rect::new(rect.x + 13.0, rect.y + rect.h * 0.5 + y - 1.0, 18.0, 2.0), Fill::new(Rgba::token(SECONDARY)));
        }
    });
    if grip.held || grip.clicked {
        let mut digits = [0u8; 24];
        let id = stack_fmt(&mut digits, format_args!("{}", index)).as_bytes();
        let mut kind = [0u8; 48];
        kind[..LIST_ROW.len()].copy_from_slice(LIST_ROW.as_bytes());
        let len = LIST_ROW.len() + id.len();
        kind[LIST_ROW.len()..len].copy_from_slice(id);
        ui.carry(
            if grip.clicked { None } else { Some(&grip.handle) },
            std::str::from_utf8(&kind[..len]).expect("list drag kind is ASCII"),
            title,
        );
    }
}

/// A swipe action key (`localwave-button` / `ROW`, centred): secondary on the panel fill; `SWIPE_ARMED` past a full swipe, paper on
/// ink. The caller sizes it to what the swipe has revealed.
const SWIPE: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).centered().label(TEXT_13.color(SECONDARY)).look(&PANEL_LOOKS);
const SWIPE_ARMED: pxui::Button = SWIPE.label(TEXT_13.color(PAPER)).look(&INK_LOOKS);

/// Swift's LWMenuButton: a secondary "…" in a 44 pt target.
const MORE: pxui::IconButton = ICON_44.look(&ICON_DIM_LOOKS);

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
fn jump(ui: &mut Ui, list: &Handle, target: usize, count: usize) {
    let visible = ((ui.rect(list).h / ROW) as usize).max(1);
    let top = (ui.scroll(list).painted / ROW) as usize;
    ui.reveal(
        list,
        if target < top {
            target
        } else {
            (target + visible - 1).min(count - 1)
        },
    );
}

/// Swift's trailing index (`section_index`, drawn by `localwave-index`): a press or drag over a letter jumps to its first song.
fn rail(ui: &mut Ui, list: &Handle, collection: &Collection) {
    ui.row(Node::row().w(Size::Grow).h(Size::Grow).align(Align::Center).justify(Justify::End), |ui| {
        let index = ui.section_index("index", &collection.letters, &RAIL);
        // A finger on the rail is a held press-and-drag, not a scroll or a row swipe underneath.
        ui.gesture(&index.response.handle, &HELD_TOUCH);
        if let Some(letter) = index.chosen {
            jump(ui, list, collection.starts[letter], collection.rows.len());
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
        + 8.0
        + PLAY_BAR
        + 16.0
        + rows as f32 * album_card(width)
        + rows.saturating_sub(1) as f32 * 20.0
        + 20.0
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
                spacer(ui, "keys-gap", 8.0);
                play_keys(ui, None, intent);
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
                spacer(ui, "songs-gap", 20.0);
                ui.column(Node::column().w(Size::Grow).h_px(SECTION_LABEL).padding(Edges::ZERO.l(16.0).t(20.0).b(8.0)),
                    |ui| {
                        ui.label("Songs", SECTION_CAPTION);
                    },
                );
            });
        }); }); });
}

/// The back key (`localwave-button` / `KEY`): 96 by 44, the chevron and the parent's name from its start, secondary with no plate;
/// named "Back".
const BACK_KEY: pxui::Button =
    pxui::Button::DEFAULT.variant(variant::KEY).size(Size::Px(96.0), Size::Px(44.0)).icon(icon_drawing(Icon::Back)).label(LEAD_DIM).look(&DIM_LOOKS).name("Back");
/// Swift's detail back row: chevron and the parent's name, 44 pt tall.
fn back_row(
    ui: &mut Ui,
    label: &'static str,
    parent: Route,
    intent: &mut Option<Intent>,
    extras: &mut dyn FnMut(&mut Ui, &mut Option<Intent>),
) {
    ui.id("back-row", |ui| { ui.row(Node::row().w(Size::Grow).h_px(BACK_ROW).align(Align::Center).padding(Edges::ZERO.t(10.0).l(6.0)),
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
fn play_keys(ui: &mut Ui, offline: Option<bool>, intent: &mut Option<Intent>) {
    ui.row(Node::row().w(Size::Grow).h_px(PLAY_BAR).gap(8.0).pad_x(16.0), |ui| {
        if ui.button("play", "Play", &PLAY_PRIMARY.icon(icon_drawing(Icon::Play))).clicked {
            *intent = Some(Intent::PlayRow(0));
        }
        if ui.button("shuffle", "Shuffle", &PLAY_KEY.icon(icon_drawing(Icon::Shuffle))).clicked {
            *intent = Some(Intent::ShuffleCollection);
        }
        if let Some(kept) = offline
            && ui.button("keep-offline", if kept { "Kept offline" } else { "Keep offline" }, &offline_key(kept)).clicked
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
            // Playlists edit (select and remove); the others select.
            if !(playlist && collection.rows.is_empty())
                && ui.button("select", if playlist { "Edit" } else { "Select" }, &WORD).clicked
            {
                *intent = Some(Intent::SelectMode(true));
            }
            if !matches!(state.route, Route::Artist(_))
                && ui.icon_button("page-actions", icon_drawing(Icon::More), if playlist { "List actions" } else { "Album actions" }, &MORE).clicked
            {
                *intent = Some(Intent::PageMenu((cx.width - 8.0, BACK_ROW)));
            }
        });
        return;
    }
    selection_header(ui, state.marked_count(), collection.rows.len(), collection.rows.len(), intent);
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
/// disabled; `DECK_REMOVE` in the danger colour.
const DECK: pxui::Button = pxui::Button::DEFAULT.variant(variant::KEY).size(Size::Grow, Size::Grow).axis(Axis::Column).gap(4.0).centered().label(TEXT_12).look(&DECK_LOOKS);
const DECK_REMOVE: pxui::Button = DECK.look(&DECK_REMOVE_LOOKS);
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

/// Compact album page lead-in: back, 150 pt artwork beside title, artist and summary, Play/Shuffle.
const ALBUM_ART: f32 = 150.0;
fn album_lead_height() -> f32 {
    BACK_ROW + ALBUM_ART + 16.0 + PLAY_BAR + 16.0
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
            ui.row(Node::row().w(Size::Grow).h_px(ALBUM_ART).gap(16.0).pad_x(16.0), |ui| {
                let art = container(ui, "artwork", Axis::Stack, Size::Px(ALBUM_ART), Size::Px(ALBUM_ART), &PANEL_FILL);
                paint_artwork(ui, &art, collection.artwork_row.map(|row| catalog.row(row)), ALBUM_ART, player.artwork);
                ui.a11y_name(&art, "Album artwork");
                ui.column(Node::column().w(Size::Grow).h(Size::Grow).gap(6.0).justify(Justify::End), |ui| {
                    ui.label(&collection.title, PARAGRAPH_24.color(INK));
                    if let lw_core::context::PlaybackContext::Album { artist, .. } = collection.context.as_ref()
                        && !artist.is_empty()
                    {
                        let link = detail::link(ui, "artist", crate::clip(artist), (17.0, 23.8, 400, 0.0));
                        if link.clicked
                            && let Some(index) = catalog.library.artists.iter().position(|a| a.title == crate::clip(artist))
                        {
                            *intent = Some(Intent::Navigate(Route::Artist(index)));
                        }
                    }
                    ui.label(&collection.summary, MONO_13.grow().color(SECONDARY));
                });
            });
            spacer(ui, "keys-gap", 16.0);
            play_keys(ui, offline_state(cx), intent);
        });
    }); }); });
}

/// Compact list page lead-in (playlists, Favorites and the other automatic lists).
fn collection_lead_height() -> f32 {
    BACK_ROW + TITLE_BLOCK + 8.0 + PLAY_BAR + 16.0
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
            titles(ui, &collection.title, &collection.summary);
            spacer(ui, "keys-gap", 8.0);
            play_keys(ui, offline_state(cx), intent);
            spacer(ui, "list-gap", 16.0);
        });
    }); }); });
}
