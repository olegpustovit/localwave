//! The phone page "…" menu (Swift's albumMenuGroups, automaticMenuGroups and listMenuGroups): whole-page
//! actions on the displayed collection. Labels are static; the controller applies `Intent::Page`.
use super::*;
use crate::PageAction;
use lw_core::context::PlaybackContext;
use super::super::songs::{list_rows, LISTS_MENU, LIST_ROWS, MENU, NEW_LIST};

pub(in crate::shell) fn menu(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    at: (f32, f32),
    intent: &mut Option<Intent>,
) {
    // A folder page only gets the deck's list choices; its songs are the marked rows.
    let folder = state.route == Route::Files;
    let collection = state.collection(catalog);
    if collection.is_none() && !(folder && state.page_lists) {
        *intent = Some(Intent::ClosePageMenu);
        return;
    }
    let context = collection.map(|collection| &*collection.context);
    let (list, album) = match context {
        Some(PlaybackContext::Playlist { id, .. }) => (Some(*id), false),
        Some(PlaybackContext::Album { .. }) => (None, true),
        _ => (None, false),
    };
    let artist = match context {
        Some(PlaybackContext::Album { artist, .. }) => catalog.library.artists.iter().position(|a| a.title == crate::clip(artist)),
        _ => None,
    };
    let filled = collection.is_some_and(|collection| !collection.ids.is_empty());
    let kept = filled && collection.is_some_and(|collection| collection.rows.iter().all(|&row| catalog.row(row).pinned));
    if state.page_lists {
        return lists(ui, catalog, at, intent);
    }
    let candidates = [
        Some(MenuItem::new("Play").id(PLAY).enabled(filled)),
        Some(MenuItem::new("Shuffle").id(SHUFFLE).enabled(filled)),
        album.then_some(MenuItem::new("Play next").id(PLAY_NEXT).enabled(filled)),
        Some(MenuItem::new("Add to queue").id(QUEUE).enabled(filled)),
        album.then_some(MenuItem::new("Add to list").id(LISTS).enabled(filled)),
        artist.map(|_| MenuItem::new("Go to artist").id(ARTIST)),
        list.map(|_| MenuItem::new("Rename…").id(RENAME)),
        list.map(|_| MenuItem::new("Duplicate").id(DUPLICATE)),
        album.then_some(MenuItem::new("Show in folder").id(FOLDER).enabled(filled)),
        Some(MenuItem::new(if kept { "Stop keeping offline" } else { "Keep offline" }).id(OFFLINE).enabled(filled)),
        list.map(|_| MenuItem::new("Delete list").id(DELETE).danger()),
    ];
    let mut rows = [MenuItem::new(""); 11];
    let mut count = 0;
    for item in candidates.into_iter().flatten() {
        rows[count] = item;
        count += 1;
    }
    // The menu opens leftward from the "…" key.
    let menu = ui.menu("page-menu", pxui::Point::new(at.0 - 200.0, at.1), &rows[..count], &MENU.name("Page actions"));
    // "Add to list" closes this menu and opens the list panel.
    if let Some(id) = menu.chosen_id {
        *intent = match id {
            PLAY => Some(Intent::Page(PageAction::Play)),
            SHUFFLE => Some(Intent::Page(PageAction::Shuffle)),
            PLAY_NEXT => Some(Intent::Page(PageAction::PlayNext)),
            QUEUE => Some(Intent::Page(PageAction::AddToQueue)),
            LISTS => Some(Intent::PageLists),
            ARTIST => artist.map(|index| Intent::Navigate(Route::Artist(index))),
            RENAME => list.map(|list| {
                let title = collection.map_or("", |collection| &collection.title);
                Intent::RenameListDialog(list, crate::browse::Query::new(title).unwrap_or_default())
            }),
            DUPLICATE => Some(Intent::Page(PageAction::Duplicate)),
            FOLDER => Some(Intent::Page(PageAction::ShowInFolder)),
            OFFLINE => Some(Intent::Page(PageAction::KeepOffline)),
            _ => list.map(Intent::ConfirmDeleteList),
        };
    }
    if menu.dismissed && intent.is_none() {
        *intent = Some(Intent::ClosePageMenu);
    }
}

// The page menu's row ids.
const PLAY: i64 = 0;
const SHUFFLE: i64 = 1;
const PLAY_NEXT: i64 = 2;
const QUEUE: i64 = 3;
const LISTS: i64 = 4;
const ARTIST: i64 = 5;
const RENAME: i64 = 6;
const DUPLICATE: i64 = 7;
const FOLDER: i64 = 8;
const OFFLINE: i64 = 9;
const DELETE: i64 = 10;

/// The page menu's "Add to list" panel: "New list…", then every list.
fn lists(ui: &mut Ui, catalog: &Catalog, at: (f32, f32), intent: &mut Option<Intent>) {
    let mut rows = [MenuItem::new(""); LIST_ROWS];
    let count = list_rows(&mut rows, catalog, None);
    let menu = ui.menu("page-lists", pxui::Point::new(at.0 - 200.0, at.1), &rows[..count], &LISTS_MENU);
    match menu.chosen_id {
        Some(NEW_LIST) => *intent = Some(Intent::NewPageList),
        Some(list) => *intent = Some(Intent::Page(PageAction::AddToList(list))),
        None => {}
    }
    if menu.dismissed && intent.is_none() {
        *intent = Some(Intent::ClosePageMenu);
    }
}
