use super::*;
use crate::Intent;
use crate::catalog::{Catalog, Route};
use lw_core::context::PlaybackContext;

/// A Swift group label.
const GROUP: Label = TEXT_12.grow().h_px(30.0).color(SECONDARY);
/// The Library group's rows: title and route.
const LIBRARY: [(&str, Route); 3] = [("Albums", Route::Albums), ("Artists", Route::Artists), ("Songs", Route::Songs)];
/// The Library group: 32-point sidebar rows, as tall as its three rows.
const SIDE: pxui::SelectionList = pxui::SelectionList::DEFAULT.row_h(32.0).variant(variant::SIDE_ROW).bare().fit();
/// The Lists group: the sidebar rows, scrolling, "+ New list" after the last. A song carried over a playlist row lights it (in the
/// cell: only playlists take a song, so no list-wide drop look).
const SIDE_LISTS: pxui::SelectionList = pxui::SelectionList::DEFAULT.row_h(32.0).variant(variant::SIDE_ROW).bare().trail(32.0);
/// A playlist row while a carried song is over it.
const DROPPING: States = States::of(SELECTED_FILL);
/// A sidebar row's title (ink while its row is selected) and its count.
const SIDE_TITLE: Label = TEXT_14.grow().h_px(20.0).color(SECONDARY);
const SIDE_COUNT: Label = MONO_12.w_px(44.0).h_px(18.0).color(SECONDARY);
/// A source row: a navigation button, never selected.
const SOURCE_ROW: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Grow, Size::Px(32.0)).align(Justify::Start).pad(Edges::new(8.0, 0.0, 8.0, 0.0)).look(&SIDE_ROW_LOOKS);
/// A source that needs reconnecting.
const RECONNECT: pxui::Badge = pxui::Badge::DEFAULT.variant(variant::BADGE);
/// A "+ label" action row (Swift's Add folder or files, New list): the sidebar's width, 32 points high, its text at the row's start.
const ADD_ROW: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Grow, Size::Px(32.0)).align(Justify::Start).look(&BARE);
/// The Search row: the panel fill, "⌘K" at its end.
const SEARCH: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Grow, Size::Px(34.0)).align(Justify::Start).pad(Edges::new(10.0, 0.0, 10.0, 0.0))
    .look(&SEARCH_ROW_LOOKS).label(TEXT_14.grow().h_px(20.0).color(SECONDARY)).accessory("⌘K").accessory_label(MONO_11.h_px(16.0).color(SECONDARY));
/// The shortcuts row: "?" in Geist Mono, then "shortcuts", 10 points under the meter.
const SHORTCUTS: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Grow, Size::Px(28.0)).align(Justify::Start).pad(Edges::new(8.0, 10.0, 0.0, 2.0))
    .look(&BARE).icon(pxui::Icon::glyph('?')).label(TEXT_12.h_px(16.0).color(SECONDARY)).name("Keyboard shortcuts");
/// The offline space meter.
const METER: pxui::Progress = pxui::Progress::DEFAULT.variant(variant::METER);

pub(super) fn sidebar(ui: &mut Ui, catalog: &Catalog, state: &ShellState, intent: &mut Option<Intent>) {
    ui.column(Node::column().w(Size::Grow).h(Size::Grow).pad(10.0), |ui| {
        spacer(ui, "sidebar-top", 28.0);
        if ui.button("search", "Search", &SEARCH).clicked {
            *intent = Some(Intent::ShowCommands);
        }
        spacer(ui, "search-gap", 12.0);
        ui.label("Library", GROUP);
        let route = if state.settings_visible { None } else { Some(state.route) };
        // a new route (or list count) selects its row in either group and none in the other; otherwise the rows hold, so every
        // build of the frame a click selects a row builds the same rows until its route follows
        let shown = Some((route, catalog.collections.len()));
        if state.side_route.replace(shown) != shown {
            let mut library = state.side_library.borrow_mut();
            library.rows.clear();
            library.rows.extend(LIBRARY.iter().map(|&(_, r)| if route == Some(r) { pxui::ListState::SELECTED } else { 0 }));
            let mut lists = state.side_lists.borrow_mut();
            lists.rows.clear();
            lists.rows.resize(catalog.collections.len(), 0);
            if let Some(Route::Collection(i)) = route
                && let Some(flags) = lists.rows.get_mut(i)
            {
                *flags = pxui::ListState::SELECTED;
            }
        }
        let counts = [&catalog.album_count, &catalog.artist_count, &catalog.song_count];
        let library = ui.selection_list("library", LIBRARY.len(), &|i| i as i64, &mut state.side_library.borrow_mut(), &SIDE, &mut |ui, part, cell| {
            if let SectionPart::Row(i) = part {
                ui.a11y_name(&cell.handle, LIBRARY[i].0);
                ui.label(LIBRARY[i].0, SIDE_TITLE.color(if cell.selected { INK } else { SECONDARY }));
                ui.label(counts[i], SIDE_COUNT);
            }
        });
        ui.a11y_name(&library.handle, "Library");
        if let Some(i) = library.selected.or(library.activated) {
            *intent = Some(Intent::Navigate(LIBRARY[i].1));
        }
        ui.label("Sources", GROUP);
        for source in &catalog.source_rows {
            let row = ui.button_content(Key::Id(source.location.source), &SOURCE_ROW.name(&source.name), |ui, _| {
                ui.label(&source.name, SIDE_TITLE.w(if source.reconnect { Size::Fit } else { Size::Grow }));
                if source.reconnect {
                    ui.badge("reconnect", "Reconnect", "Reconnect", &RECONNECT);
                    ui.node("badge-gap", Node::DEFAULT.w(Size::Grow));
                }
                ui.label(&source.count, SIDE_COUNT);
            });
            if row.clicked {
                *intent = Some(Intent::Files(crate::files::Request::Browse(source.location)));
            }
        }
        if ui.button("add-music", "+  Add folder or files", &ADD_ROW.name("Add folder or files")).clicked {
            *intent = Some(Intent::ChooseAudio);
        }
        ui.label("Lists", GROUP);
        let playlist = |i: usize| match &*catalog.collections[i].context {
            PlaybackContext::Playlist { id, .. } => Some(*id),
            _ => None,
        };
        let collections = &catalog.collections;
        let lists = ui.selection_list("lists", collections.len(), &|i| collections[i].key as i64, &mut state.side_lists.borrow_mut(), &SIDE_LISTS, &mut |ui, part, cell| match part {
            SectionPart::Row(i) => {
                let collection = &collections[i];
                ui.a11y_name(&cell.handle, &collection.title);
                ui.label(&collection.title, SIDE_TITLE.color(if cell.selected { INK } else { SECONDARY }));
                ui.label(&collection.count, SIDE_COUNT);
                if let Some(list) = playlist(i)
                    && let Some((dropped, song)) = super::songs::song_drop(ui, &cell.handle)
                {
                    if dropped {
                        *intent = Some(Intent::AddToList { song, list });
                    }
                    ui.style(&cell.handle, &DROPPING, State::default());
                }
            }
            SectionPart::Trailing => {
                if ui.button("new-list", "+  New list", &ADD_ROW.name("New list")).clicked {
                    *intent = Some(Intent::NewList(None));
                }
            }
            _ => {}
        });
        ui.a11y_name(&lists.handle, "Lists");
        if let Some(i) = lists.selected.or(lists.activated) {
            *intent = Some(Intent::Navigate(Route::Collection(i)));
        }
        if let (Some(i), Some(row)) = (lists.secondary, lists.row)
            && let Some(list) = playlist(i)
        {
            *intent = Some(Intent::ListMenu(list, ui.signal(&row).pointer));
        }
        ui.row(Node::row().w(Size::Grow).h_px(30.0).padding(Edges::ZERO.t(14.0).l(8.0).r(8.0)), |ui| {
            ui.label("Offline", TEXT_12.grow().h_px(16.0).color(SECONDARY));
            ui.label(&catalog.offline_usage, MONO_12.h_px(16.0).color(SECONDARY));
        });
        ui.progress("offline", "Offline space", Some(catalog.offline_fraction), &METER);
        if ui.button("shortcuts", "shortcuts", &SHORTCUTS).clicked {
            *intent = Some(Intent::ShowShortcuts);
        }
    });
}
