use super::*;
use crate::browse;

pub(in crate::shell) fn popup(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    target: (usize, i64, u64),
    at: (f32, f32),
    sharing: bool,
    intent: &mut Option<Intent>,
) {
    let click = at;
    // A folder page row has no collection; its saved song only has to exist.
    let files = state.route == Route::Files;
    let present = if files {
        catalog.row_index(target.1).is_some()
    } else {
        state.collection(catalog).is_some_and(|collection| {
            collection.key == target.2 && collection.ids.get(target.0) == Some(&target.1)
        })
    };
    let row = catalog.row_index(target.1).map(|index| catalog.row(index));
    let favorite = row.is_some_and(|row| row.favorite);
    let list = state
        .collection(catalog)
        .and_then(|collection| match &*collection.context {
            lw_core::context::PlaybackContext::Playlist { id, .. } => Some(*id),
            _ => None,
        });
    let routes = catalog.byline_routes(target.1);
    if state.song_lists {
        return lists(ui, catalog, target.1, at, present, list, intent);
    }
    let sourced = row.is_some_and(|row| row.source_id.is_some());
    let edit = row.and_then(|row| edit(&row));
    // Swift's LWTrackMenu order.
    let candidates = [
        Some(MenuItem::new("Play").id(PLAY)),
        Some(MenuItem::new("Play next").id(PLAY_NEXT)),
        Some(MenuItem::new("Add to queue").id(QUEUE)),
        Some(MenuItem::new("Add to list").id(LISTS)),
        routes.map(|_| MenuItem::new("Go to album").id(ALBUM)),
        routes.map(|_| MenuItem::new("Go to artist").id(ARTIST)),
        cfg!(not(target_os = "ios")).then_some(MenuItem::new("Show in Finder").id(REVEAL).enabled(sourced)),
        Some(MenuItem::new(if favorite { "Unfavorite" } else { "Favorite" }).id(FAVORITE)),
        Some(MenuItem::new(if row.is_some_and(|row| row.pinned) { "Stop keeping offline" } else { "Keep offline" }).id(OFFLINE)),
        row.filter(|row| row.state == lw_core::FileState::Failed).map(|_| MenuItem::new("Retry download").id(RETRY)),
        Some(MenuItem::new("Share…").id(SHARE).enabled(sourced && !sharing)),
        edit.map(|_| MenuItem::new("Edit info…").id(EDIT)),
        Some(MenuItem::new("Delete…").id(DELETE)),
        list.map(|_| MenuItem::new("Remove from list").id(REMOVE)),
    ];
    let mut rows = [MenuItem::new(""); 14];
    let mut count = 0;
    for item in candidates.into_iter().flatten() {
        rows[count] = item.enabled(item.enabled && present);
        count += 1;
    }
    let menu = ui.menu("song-menu", pxui::Point::new(at.0, at.1), &rows[..count], &MENU.name("Song actions"));
    let song = target.1;
    // "Add to list" closes this menu and opens the list panel.
    if let Some(id) = menu.chosen_id {
        *intent = match id {
            PLAY if files => Some(Intent::PlayFile(song)),
            PLAY => Some(Intent::PlayRow(target.0)),
            PLAY_NEXT => Some(Intent::PlayNext(song)),
            QUEUE => Some(Intent::AppendSong(song)),
            LISTS => Some(Intent::SongMenuLists),
            ALBUM => routes.map(|routes| Intent::Navigate(routes[1])),
            ARTIST => routes.map(|routes| Intent::Navigate(routes[0])),
            REVEAL => Some(Intent::RevealSong(song)),
            FAVORITE => Some(Intent::SetFavorite { song, on: !favorite }),
            OFFLINE => Some(Intent::KeepOffline(song)),
            RETRY => Some(Intent::RetryDownload(song)),
            SHARE => Some(Intent::ShareSong(song, click)),
            EDIT => edit.map(Intent::OpenMetadata),
            DELETE => Some(Intent::ConfirmDeleteSong(song)),
            _ => list.map(|list| Intent::RemoveFromList { song, list }),
        };
    }
    if menu.dismissed && intent.is_none() {
        *intent = Some(Intent::CloseSongMenu);
    }
}

// The song menu's row ids.
const PLAY: i64 = 0;
const PLAY_NEXT: i64 = 1;
const QUEUE: i64 = 2;
const LISTS: i64 = 3;
const ALBUM: i64 = 4;
const ARTIST: i64 = 5;
const REVEAL: i64 = 6;
const FAVORITE: i64 = 7;
const OFFLINE: i64 = 8;
const RETRY: i64 = 9;
const SHARE: i64 = 10;
const EDIT: i64 = 11;
const DELETE: i64 = 12;
const REMOVE: i64 = 13;

/// LocalWave's context menus: 36-point rows from 200 points wide, no spacer above the first row or below the last.
pub(in crate::shell) const MENU: Menu<'static> = Menu::DEFAULT.row_h(36.0).w_px(200.0).padding(0.0);
/// An "Add to list" panel: at most eight rows tall, its edges included; the rest scroll.
pub(in crate::shell) const LISTS_MENU: Menu<'static> = MENU.name("Add to list").max_h(8.0 * 36.0);
/// The row id of "New list…" in an "Add to list" panel, whose other rows are keyed by playlist id.
pub(in crate::shell) const NEW_LIST: i64 = -1;
/// The most rows an "Add to list" panel holds; lists past it are not offered.
pub(in crate::shell) const LIST_ROWS: usize = 256;

/// The song menu's "Add to list" panel: "New list…", then the other lists (or a note that there are none).
fn lists(
    ui: &mut Ui,
    catalog: &Catalog,
    song: i64,
    at: (f32, f32),
    present: bool,
    list: Option<i64>,
    intent: &mut Option<Intent>,
) {
    let mut rows = [MenuItem::new(""); LIST_ROWS];
    let count = list_rows(&mut rows, catalog, list);
    for row in &mut rows[..count] {
        *row = row.enabled(row.enabled && present);
    }
    let menu = ui.menu("song-lists", pxui::Point::new(at.0, at.1), &rows[..count], &LISTS_MENU);
    match menu.chosen_id {
        Some(NEW_LIST) => *intent = Some(Intent::NewList(Some(song))),
        Some(list) => *intent = Some(Intent::AddToList { song, list }),
        None => {}
    }
    if menu.dismissed && intent.is_none() {
        *intent = Some(Intent::CloseSongMenu);
    }
}

/// Fills `rows` with an "Add to list" panel: "New list…", then every list but `except` keyed by its id, or "No other lists" (disabled)
/// when there is none. Returns the number of rows.
pub(in crate::shell) fn list_rows<'a>(rows: &mut [MenuItem<'a>; LIST_ROWS], catalog: &'a Catalog, except: Option<i64>) -> usize {
    rows[0] = MenuItem::new("New list…").id(NEW_LIST);
    let mut count = 1;
    for collection in &catalog.collections {
        let lw_core::context::PlaybackContext::Playlist { id, .. } = &*collection.context else { continue };
        if Some(*id) != except && count < LIST_ROWS {
            rows[count] = MenuItem::new(&collection.title).id(*id);
            count += 1;
        }
    }
    if count == 1 {
        rows[1] = MenuItem::new("No other lists").enabled(false);
        count = 2;
    }
    count
}

/// Prefill from the row; `None` when a field exceeds the packet, so saving cannot truncate it.
fn edit(row: &crate::catalog::Row<'_>) -> Option<crate::MetadataEdit> {
    let number = |n: Option<i64>| {
        let mut digits = [0u8; 20];
        let mut at = digits.len();
        if let Some(mut n) = n.map(|n| n.max(0) as u64) {
            while n > 0 || at == digits.len() {
                at -= 1;
                digits[at] = b'0' + (n % 10) as u8;
                n /= 10;
            }
        }
        browse::Query::new(std::str::from_utf8(&digits[at..]).ok()?)
    };
    Some(crate::MetadataEdit {
        song: row.id,
        title: browse::Query::new(row.title)?,
        artist: browse::Query::new(row.artist)?,
        album: browse::Query::new(row.album)?,
        album_artist: browse::Query::new(row.album_artist)?,
        year: number(row.release_year)?,
        track: number(row.track_number)?,
    })
}
