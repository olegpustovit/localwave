//! Frames borrow worker-prepared catalogs. Use explicit int_key identities (0 and 1 are reserved),
//! scope navigation scroll by route and virtualize sidebar collections. App-authored accessibility
//! semantics build a tree only for tests or a consuming bridge. Clock digits stay on cached glyphs;
//! virtual slots retain accessibility string capacity.
//! PlayerView holds stable data; the separate position only reaches transport/Now clocks and seek.
//! Keep both desktop and compact library revisions independent of those ticks.
//!
//! Icon buttons retain full semantic/hit bounds and scale only the cached visual child to 0.96,
//! restarting animation only when held changes. Host reduced motion overrides theme flags. Wrapped
//! measurement identities include content; natural/constrained identities also differ by width.
//! Passive external-drop overlays paint in the root canvas with accessible instructions without
//! shifting layout, capturing queue drags or forcing idle redraws.
//!
use crate::browse::{Query, Request, Scope, Sort};
use crate::catalog::{Catalog, Route};
use crate::{Intent, PlayerView, Selection};
use pxui::{KeyCode, Label, Mods};
use pxui::theme::ColorToken;
use pxui::ui::*;

mod albums;
mod artists;
mod byline;
pub(crate) mod commands;
mod compact;
mod detail;
mod files;
mod find;
mod onboarding;
mod settings;
mod songs;

pub struct ShellState {
    /// The compact song row whose swipe actions are showing, so swiping another closes it.
    open_swipe: std::cell::Cell<Option<(usize, pxui::Handle)>>,
    /// The queue sheet's drag-to-dismiss offset from the previous frame.
    /// Bumped by every catalog delivery; a pull to refresh ends when it moves past the value it saw.
    refresh_epoch: u64,
    refresh_wait: std::cell::Cell<Option<(u64, f64)>>,
    refresh_ending: std::cell::Cell<bool>,
    song_window: std::cell::Cell<Option<crate::song_text::Window>>,
    /// A carry was in flight when the last frame ended, so its drop-target views rebuild once more when it ends.
    carried: std::cell::Cell<bool>,
    /// Set by an arrow-key move; the song list reveals the cursor row once, like AppKit `scrollRowToVisible`.
    reveal_selected: std::cell::Cell<bool>,
    /// The sidebar's Library and Lists groups, borrowed mutably only for the list call, and the route (`None` in settings) and list
    /// count their rows last showed: a change selects the route's row; a click's own selection holds until the route follows it.
    side_library: std::cell::RefCell<pxui::ListState>,
    side_lists: std::cell::RefCell<pxui::ListState>,
    side_route: std::cell::Cell<Option<(Option<Route>, usize)>>,
    pub route: Route,
    pub selected: Option<usize>,
    /// Sorted row positions of the multi-selection; grows only in `apply`/`refresh`, never in frames.
    marked: Vec<usize>,
    anchor: Option<usize>,
    /// Phone Select mode: rows show checkboxes and taps toggle marks.
    selecting: bool,
    page_menu: Option<(f32, f32)>,
    page_lists: bool,
    /// A folder row's "…" menu: entry id and anchor.
    folder_menu: Option<(i64, (f32, f32))>,
    files_filter: crate::FilesFilter,
    add_music: bool,
    selection_revision: u64,
    pub panel_visible: bool,
    pub query: Query,
    pub sort: Sort,
    pub ascending: bool,
    pub album_sort: crate::browse::AlbumSort,
    player_expanded: bool,
    queue_visible: bool,
    playlist_editing: bool,
    list_dialog: Option<crate::ListDialog>,
    list_menu: Option<(i64, (f32, f32))>,
    delete_list: Option<i64>,
    metadata: Option<crate::MetadataEdit>,
    metadata_too_long: bool,
    delete_song: Option<i64>,
    playlist_name: Query,
    playlist_name_too_long: bool,
    top_inset: f32,
    query_too_long: bool,
    find_results: bool,
    query_revision: u64,
    revision: u64,
    entry_revision: u64,
    settings_visible: bool,
    privacy_visible: bool,
    settings_menu: Option<(crate::SettingsMenu, (f32, f32))>,
    source_menu: Option<(crate::files::Target, (f32, f32))>,
    song_menu: Option<(usize, i64, u64, (f32, f32))>,
    song_lists: bool,
    explored: bool,
    remove_source: Option<crate::files::Target>,
    commands_visible: bool,
    command_query: Query,
    command_selected: usize,
    command_too_long: bool,
    shortcuts_visible: bool,
}
impl Default for ShellState {
    fn default() -> Self {
        Self {
            open_swipe: std::cell::Cell::new(None),
            refresh_epoch: 0,
            refresh_wait: std::cell::Cell::new(None),
            refresh_ending: std::cell::Cell::new(false),
            song_window: std::cell::Cell::new(None),
            carried: std::cell::Cell::new(false),
            reveal_selected: std::cell::Cell::new(false),
            side_library: std::cell::RefCell::new(pxui::ListState::new()),
            side_lists: std::cell::RefCell::new(pxui::ListState::new()),
            side_route: std::cell::Cell::new(None),
            route: Route::Songs,
            selected: None,
            marked: Vec::new(),
            anchor: None,
            selecting: false,
            page_menu: None,
            page_lists: false,
            folder_menu: None,
            files_filter: crate::FilesFilter::All,
            add_music: false,
            selection_revision: 0,
            panel_visible: true,
            query: Query::default(),
            sort: Sort::Original,
            ascending: true,
            album_sort: crate::browse::AlbumSort::Artist,
            player_expanded: false,
            queue_visible: false,
            playlist_editing: false,
            list_dialog: None,
            list_menu: None,
            delete_list: None,
            metadata: None,
            metadata_too_long: false,
            delete_song: None,
            playlist_name: Query::default(),
            playlist_name_too_long: false,
            top_inset: 0.0,
            query_too_long: false,
            find_results: false,
            query_revision: 0,
            revision: 0,
            entry_revision: 1,
            settings_visible: false,
            privacy_visible: false,
            settings_menu: None,
            source_menu: None,
            song_menu: None,
            song_lists: false,
            explored: false,
            remove_source: None,
            commands_visible: false,
            command_query: Query::default(),
            command_selected: 0,
            command_too_long: false,
            shortcuts_visible: false,
        }
    }
}
impl ShellState {
    fn open_list_dialog(&mut self, dialog: crate::ListDialog, name: Query) {
        self.list_dialog = Some(dialog);
        self.playlist_name = name;
        self.playlist_name_too_long = false;
        (self.song_menu, self.list_menu, self.source_menu) = (None, None, None);
        self.song_lists = false;
        self.settings_menu = None;
        self.commands_visible = false;
        self.shortcuts_visible = false;
    }
    pub fn song_text_window(&self, catalog: &Catalog) -> Option<crate::song_text::Window> {
        self.song_window.get().filter(|window| {
            !self.settings_visible
                && self.collection(catalog).is_some()
                && window.matches(catalog, self.route)
        })
    }
    /// Song ids of the selected rows in displayed order, borrowed for context-menu actions.
    pub fn selected_song_ids<'a>(&'a self, catalog: &'a Catalog) -> impl Iterator<Item = i64> + 'a {
        let ids = self.collection(catalog).map_or(&[][..], |c| &c.ids[..]);
        self.marked.iter().filter_map(move |&i| ids.get(i).copied())
    }
    /// Saved songs of the marked rows of a folder page, in row order.
    pub fn selected_folder_songs(&self, directory: &crate::files::Directory) -> Vec<i64> {
        self.marked
            .iter()
            .filter_map(|&index| directory.entry(self.files_filter, index)?.song)
            .collect()
    }
    pub fn selecting(&self) -> bool {
        self.selecting
    }
    /// Swift leaves selection mode once a bar action ran; the controller calls this after taking its targets.
    pub fn finish_selecting(&mut self) {
        if self.selecting {
            self.selecting = false;
            self.clear_selection();
            self.selection_revision = self.selection_revision.wrapping_add(1);
        }
    }
    fn is_marked(&self, index: usize) -> bool {
        self.marked.binary_search(&index).is_ok()
    }
    fn clear_selection(&mut self) {
        (self.selected, self.anchor) = (None, None);
        self.marked.clear();
    }
    fn select(&mut self, selection: Selection) {
        match selection {
            Selection::Toggle(i) => {
                match self.marked.binary_search(&i) {
                    Ok(at) => {
                        self.marked.remove(at);
                    }
                    Err(at) => self.marked.insert(at, i),
                }
                self.selected = self.is_marked(i).then_some(i);
                self.anchor = Some(i);
            }
            Selection::Range(i) => {
                let from = *self.anchor.get_or_insert(i);
                self.marked.clear();
                self.marked.extend(from.min(i)..=from.max(i));
                self.selected = Some(i);
            }
            Selection::All(len) => {
                self.marked.clear();
                self.marked.reserve_exact(len);
                self.marked.extend(0..len);
            }
            Selection::Clear => self.clear_selection(),
            Selection::Move { delta, extend, len } => {
                let Some(&first) = self.marked.first().filter(|_| len > 0) else {
                    return;
                };
                let at = (self.selected.unwrap_or(first) as isize + delta)
                    .clamp(0, len as isize - 1) as usize;
                if extend {
                    if let Err(pos) = self.marked.binary_search(&at) {
                        self.marked.insert(pos, at);
                    }
                } else {
                    self.marked.clear();
                    self.marked.push(at);
                    self.anchor = Some(at);
                }
                self.selected = Some(at);
                self.reveal_selected.set(true);
            }
        }
        self.selection_revision = self.selection_revision.wrapping_add(1);
    }
    pub fn files_visible(&self) -> bool {
        self.route == Route::Files && !self.settings_visible
    }
    pub fn set_window_inset(&mut self, top: f32) {
        if top.is_finite() && top >= 0.0 && self.top_inset != top {
            self.top_inset = top;
            self.revision = self.revision.wrapping_add(1);
        }
    }
    pub fn collection<'a>(&self, catalog: &'a Catalog) -> Option<&'a crate::catalog::Collection> {
        if self.route == Route::Find {
            catalog
                .view
                .as_ref()
                .filter(|view| {
                    view.request.scope == Scope::Find && view.request.query == self.query
                })
                .map(|view| &view.collection)
        } else {
            catalog.collection(self.route)
        }
    }
    pub fn request(&self, catalog: &Catalog) -> Option<Request> {
        Some(Request {
            commands: self.commands_visible.then_some(self.command_query),
            scope: catalog
                .scope(self.route)
                .or_else(|| self.commands_visible.then_some(Scope::Overview))?,
            query: if self.route == Route::Find {
                self.query
            } else {
                Query::default()
            },
            sort: self.sort,
            ascending: self.ascending,
        })
    }
    pub fn refresh(&mut self, previous: &Catalog, next: &Catalog) {
        self.refresh_epoch = self.refresh_epoch.wrapping_add(1);
        let same_commands = match (&previous.commands, &next.commands) {
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same_commands {
            self.command_selected = self.command_selected.min(
                self.command_results(next)
                    .map_or(0, |r| r.items.len().saturating_sub(1)),
            );
            self.revision = self.revision.wrapping_add(1);
        }
        if previous.same_page(next) {
            return;
        }
        let remap = next.remap.as_ref().expect("worker-prepared catalog remap");
        assert!(
            remap.matches(previous, next),
            "catalog remap must match both pages"
        );
        self.song_menu = None;
        let old = self.route;
        (self.route, self.selected) = remap.resolve(old, self.selected, self.query);
        if self.route != Route::Files && (!self.marked.is_empty() || self.anchor.is_some()) {
            let map = |i: &usize| remap.resolve(old, Some(*i), self.query).1;
            self.anchor = self.anchor.as_ref().and_then(map);
            let mut marked: Vec<usize> = self.marked.iter().filter_map(map).collect();
            marked.sort_unstable();
            self.marked = marked;
            self.selection_revision = self.selection_revision.wrapping_add(1);
        }
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn apply(&mut self, intent: Intent) -> bool {
        // Selection changes rebuild only affected rows, so they skip the shared revision.
        match intent {
            Intent::SelectRow(index) => {
                self.marked.clear();
                self.marked.push(index);
                (self.selected, self.anchor) = (Some(index), Some(index));
                self.selection_revision = self.selection_revision.wrapping_add(1);
                return true;
            }
            Intent::Select(selection) => {
                self.select(selection);
                return true;
            }
            Intent::SelectMode(on) => {
                self.selecting = on;
                self.clear_selection();
                self.selection_revision = self.selection_revision.wrapping_add(1);
                self.revision = self.revision.wrapping_add(1);
                return true;
            }
            _ => {}
        }
        if matches!(intent, Intent::Navigate(_) | Intent::Files(_)) {
            self.files_filter = crate::FilesFilter::All;
            self.folder_menu = None;
        }
        // Marks index the folder's rows, which another folder replaces.
        if matches!(intent, Intent::Files(_) | Intent::FilesFilter(_)) && self.selecting && self.route == Route::Files {
            self.selecting = false;
            self.clear_selection();
            self.selection_revision = self.selection_revision.wrapping_add(1);
        }
        if matches!(
            intent,
            Intent::Navigate(_)
                | Intent::TogglePlayer
                | Intent::ShowSettings
                | Intent::CloseSettings
                | Intent::ShowPrivacy
                | Intent::ClosePrivacy
        ) {
            self.entry_revision = self.entry_revision.wrapping_add(1);
        }
        if matches!(
            intent,
            Intent::Navigate(_)
                | Intent::ShowSettings
                | Intent::ShowQueue
                | Intent::Search(_)
                | Intent::SearchResults(_)
                | Intent::Sort(_)
        ) {
            self.song_menu = None;
        }
        match intent {
            Intent::ShowCommands => {
                self.commands_visible = true;
                self.command_query = Query::default();
                self.command_selected = 0;
                self.command_too_long = false;
                self.shortcuts_visible = false;
                self.queue_visible = false;
                self.settings_menu = None;
                self.source_menu = None;
                self.song_menu = None;
                self.remove_source = None;
            }
            Intent::CloseCommands => self.commands_visible = false,
            Intent::CommandQuery(query) => {
                self.command_query = query;
                self.command_selected = 0;
                self.command_too_long = false;
            }
            Intent::CommandTooLong => self.command_too_long = true,
            Intent::CommandSelect(index) => self.command_selected = index,
            Intent::ShowShortcuts => {
                self.commands_visible = false;
                self.shortcuts_visible = true;
                self.queue_visible = false;
                self.settings_menu = None;
                self.source_menu = None;
                self.song_menu = None;
                self.remove_source = None;
            }
            Intent::CloseShortcuts => self.shortcuts_visible = false,
            Intent::SongMenu(index, id, key, at) => {
                if at.0.is_finite() && at.1.is_finite() {
                    self.song_menu = Some((index, id, key, at));
                    self.song_lists = false;
                    self.settings_menu = None;
                    self.source_menu = None;
                }
            }
            Intent::SongMenuLists => self.song_lists = self.song_menu.is_some(),
            Intent::SelectionLists(index, id, key, at) => {
                if at.0.is_finite() && at.1.is_finite() {
                    self.song_menu = Some((index, id, key, at));
                    self.song_lists = true;
                    (self.settings_menu, self.source_menu) = (None, None);
                }
            }
            Intent::PageMenu(at) => {
                if at.0.is_finite() && at.1.is_finite() {
                    self.page_menu = Some(at);
                    self.page_lists = false;
                    (self.song_menu, self.list_menu, self.source_menu) = (None, None, None);
                    self.settings_menu = None;
                }
            }
            Intent::PageLists => self.page_lists = self.page_menu.is_some(),
            Intent::PageListChoices(at) => {
                if at.0.is_finite() && at.1.is_finite() {
                    self.page_menu = Some(at);
                    self.page_lists = true;
                    (self.song_menu, self.list_menu, self.source_menu) = (None, None, None);
                    self.settings_menu = None;
                }
            }
            Intent::NewPageList => {
                (self.page_menu, self.page_lists) = (None, false);
                self.open_list_dialog(crate::ListDialog::NewPage, Query::default());
            }
            Intent::FolderMenu(entry, at) => {
                if at.0.is_finite() && at.1.is_finite() {
                    self.folder_menu = Some((entry, at));
                    (self.song_menu, self.source_menu) = (None, None);
                }
            }
            Intent::CloseFolderMenu => self.folder_menu = None,
            Intent::ClosePageMenu => {
                self.page_menu = None;
                self.page_lists = false;
            }
            Intent::Page(_) => {
                self.page_menu = None;
                self.page_lists = false;
                self.revision = self.revision.wrapping_add(1);
                return false;
            }
            Intent::FilesFilter(filter) => {
                self.files_filter = filter;
                self.revision = self.revision.wrapping_add(1);
                return true;
            }
            Intent::ShowAddMusic => {
                self.add_music = true;
                self.source_menu = None;
            }
            Intent::CloseSongMenu => {
                self.song_menu = None;
                self.song_lists = false;
            }
            Intent::Navigate(route) => {
                self.explored = true;
                self.commands_visible = false;
                self.shortcuts_visible = false;
                self.source_menu = None;
                self.remove_source = None;
                self.settings_visible = false;
                self.privacy_visible = false;
                self.settings_menu = None;
                self.player_expanded = false;
                self.route = route;
                self.find_results = false;
                (self.selecting, self.page_menu) = (false, None);
                self.clear_selection();
                self.selection_revision = self.selection_revision.wrapping_add(1);
                self.sort = Sort::Original;
                self.ascending = true;
            }
            Intent::SortAlbums(sort) => self.album_sort = sort,
            Intent::TogglePanel => self.panel_visible = !self.panel_visible,
            Intent::TogglePlayer => self.player_expanded = !self.player_expanded,
            Intent::ShowQueue => {
                self.source_menu = None;
                self.remove_source = None;
                self.queue_visible = true;
                self.settings_menu = None;
            }
            Intent::CloseQueue => {
                self.queue_visible = false;
                self.playlist_editing = false;
            }
            Intent::NamePlaylist => self.playlist_editing = true,
            Intent::NewList(song) => {
                self.open_list_dialog(crate::ListDialog::New(song), Query::default())
            }
            Intent::RenameListDialog(list, name) => {
                self.open_list_dialog(crate::ListDialog::Rename(list), name)
            }
            Intent::ListMenu(list, at) => {
                if at.0.is_finite() && at.1.is_finite() {
                    self.list_menu = Some((list, at));
                    self.song_menu = None;
                    self.source_menu = None;
                    self.settings_menu = None;
                }
            }
            Intent::CloseListMenu => self.list_menu = None,
            Intent::ConfirmDeleteList(list) => {
                self.delete_list = Some(list);
                self.list_menu = None;
            }
            Intent::CloseDeleteList => self.delete_list = None,
            Intent::OpenMetadata(edit) => {
                self.metadata = Some(edit);
                self.metadata_too_long = false;
                self.song_menu = None;
                self.song_lists = false;
            }
            Intent::MetadataField(index, value) => {
                if let Some(field) = self
                    .metadata
                    .as_mut()
                    .and_then(|edit| edit.field_mut(index))
                {
                    *field = value;
                }
                self.metadata_too_long = false;
            }
            Intent::MetadataTooLong => self.metadata_too_long = true,
            Intent::CloseMetadata => self.metadata = None,
            Intent::ConfirmDeleteSong(song) => {
                self.delete_song = Some(song);
                self.song_menu = None;
                self.song_lists = false;
            }
            Intent::CloseDeleteSong => self.delete_song = None,
            Intent::EditPlaylistName(name) => {
                self.playlist_name = name;
                self.playlist_name_too_long = false;
            }
            Intent::PlaylistNameTooLong => self.playlist_name_too_long = true,
            Intent::CancelPlaylistName => {
                self.playlist_editing = false;
                self.list_dialog = None;
            }
            Intent::Search(query) => {
                self.query = query;
                self.clear_selection();
                self.selection_revision = self.selection_revision.wrapping_add(1);
                self.query_too_long = false;
                self.query_revision = self.query_revision.wrapping_add(1);
            }
            Intent::SearchResults(query) => {
                self.apply(Intent::Search(query));
                self.find_results = true;
            }
            Intent::SearchTooLong => {
                self.query_too_long = true;
                self.query_revision = self.query_revision.wrapping_add(1);
            }
            Intent::Sort(sort) => {
                self.ascending = self.sort != sort || !self.ascending;
                self.sort = sort;
            }
            Intent::ShowSettings => {
                self.player_expanded = false;
                self.source_menu = None;
                self.remove_source = None;
                self.settings_visible = true;
                self.privacy_visible = false;
                self.queue_visible = false;
                self.settings_menu = None;
            }
            Intent::CloseSettings => {
                self.settings_visible = false;
                self.privacy_visible = false;
                self.settings_menu = None;
            }
            Intent::ShowPrivacy => {
                self.privacy_visible = true;
                self.settings_menu = None;
            }
            Intent::ClosePrivacy => self.privacy_visible = false,
            Intent::SettingsMenu(menu, at) => {
                self.song_menu = None;
                if at.0.is_finite() && at.1.is_finite() {
                    self.settings_menu = Some((menu, at));
                }
            }
            Intent::CloseSettingsMenu => self.settings_menu = None,
            Intent::SourceMenu(target, at) => {
                self.song_menu = None;
                if at.0.is_finite() && at.1.is_finite() {
                    self.source_menu = Some((target, at));
                    self.settings_menu = None;
                }
            }
            Intent::CloseSourceMenu => {
                self.source_menu = None;
                self.add_music = false;
            }
            Intent::ConfirmRemoveSource(target) => {
                self.remove_source = Some(target);
                self.source_menu = None;
            }
            Intent::CloseRemoveSource => self.remove_source = None,
            Intent::Setting(_) => {
                self.settings_menu = None;
                self.revision = self.revision.wrapping_add(1);
                return false;
            }
            _ => return false,
        }
        self.revision = self.revision.wrapping_add(1);
        true
    }
}

fn library_revision(player: &PlayerView<'_>, catalog: &Catalog, state: &ShellState) -> u64 {
    crate::hash((
        state.revision,
        (
            state.selection_revision,
            // A held pull to refresh must see the delivery that ends it even when the library is unchanged.
            state.refresh_wait.get().map_or(0, |_| state.refresh_epoch),
        ),
        (
            std::sync::Arc::as_ptr(&catalog.library) as usize,
            catalog.all.is_some()
        ),
        player.revision,
        (player.current_id, player.playing),
        player.artwork.map_or(0, crate::artwork::Images::revision),
        player.settings,
        (
            player.cache_usage.as_ptr() as usize,
            player.cache_usage.len()
        ),
        player.storage_busy,
        (
            player.settings_message.as_ptr() as usize,
            player.settings_message.len()
        ),
        (player.message.as_ptr() as usize, player.message.len()),
        if state.files_visible() {
            player.files.map_or(0, |files| files.revision)
        } else {
            0
        },
    ))
}

/// Swift's global shortcuts. Raw scalar queries: routed key vectors allocate.
fn global_shortcut(
    ui: &Ui,
    player: &PlayerView<'_>,
    catalog: &Catalog,
    position: f64,
) -> Option<Intent> {
    use crate::commands::Command;
    // Extra modifiers held at the end of the frame select a different shortcut.
    let pressed = |key: KeyCode, mods: Mods| {
        let exact = [
            (KeyCode::Meta, Mods::META), (KeyCode::Alt, Mods::ALT),
            (KeyCode::Shift, Mods::SHIFT), (KeyCode::Ctrl, Mods::CTRL),
        ].into_iter().all(|(key, modifier)| modifier == mods || !ui.key_down(key));
        exact && (ui.key_pressed(key, mods)
            || mods != Mods::ANY && matches!(key, KeyCode::Char(c) if ui.key_pressed(KeyCode::Char(c.to_ascii_uppercase()), mods)))
    };
    // ⇧⌘O is Add Files. Shift uses the held state, and Command uses the press.
    if ui.key_down(KeyCode::Shift)
        && !ui.key_down(KeyCode::Alt)
        && !ui.key_down(KeyCode::Ctrl)
        && (ui.key_pressed(KeyCode::Char('o'), Mods::META) || ui.key_pressed(KeyCode::Char('O'), Mods::META))
    {
        return Some(Intent::ChooseAudio);
    }
    for command in [
        Command::PlayPause,
        Command::Shuffle,
        Command::Repeat,
        Command::AddMusic,
        Command::Panel,
        Command::Favorite,
    ] {
        let (key, modifier) = command.key()?;
        if pressed(key, modifier) {
            if command == Command::Favorite {
                // No current song: the key is consumed by nothing.
                let id = player.current_id?;
                let on = catalog
                    .row_index(id)
                    .is_some_and(|i| catalog.row(i).favorite);
                return Some(Intent::SetFavorite { song: id, on: !on });
            }
            return Some(match command {
                Command::PlayPause if player.playing => Intent::Pause,
                Command::PlayPause => Intent::Play,
                Command::Shuffle => Intent::Shuffle,
                Command::Repeat => Intent::Repeat,
                // Swift's Command-O is "Add Folder…".
                Command::AddMusic => Intent::ChooseFolder,
                _ => Intent::TogglePanel,
            });
        }
    }
    for (key, intent) in [
        ('f', Intent::Navigate(Route::Find)),
        ('n', Intent::NewList(None)),
        ('1', Intent::Navigate(Route::Albums)),
        ('2', Intent::Navigate(Route::Artists)),
        ('3', Intent::Navigate(Route::Songs)),
    ] {
        if pressed(KeyCode::Char(key), Mods::META) {
            return Some(intent);
        }
    }
    for (key, modifier, intent) in [
        (KeyCode::ArrowLeft, Mods::META, Intent::Previous),
        (KeyCode::ArrowRight, Mods::META, Intent::Next),
        (KeyCode::ArrowLeft, Mods::SHIFT, Intent::Previous),
        (KeyCode::ArrowRight, Mods::SHIFT, Intent::Next),
        (
            KeyCode::ArrowUp,
            Mods::META,
            Intent::Volume((player.volume + 1.0 / 12.0).min(1.0)),
        ),
        (
            KeyCode::ArrowDown,
            Mods::META,
            Intent::Volume((player.volume - 1.0 / 12.0).max(0.0)),
        ),
    ] {
        if pressed(key, modifier) {
            return Some(intent);
        }
    }
    if player.duration > 0.0 {
        for (key, modifier, seconds) in [
            (KeyCode::ArrowLeft, Mods::ANY, -5.0),
            (KeyCode::ArrowRight, Mods::ANY, 5.0),
            (KeyCode::ArrowLeft, Mods::ALT, -30.0),
            (KeyCode::ArrowRight, Mods::ALT, 30.0),
        ] {
            if pressed(key, modifier) {
                let to = ((position + seconds) / player.duration).clamp(0.0, 1.0);
                return Some(Intent::SeekFraction(to));
            }
        }
    }
    None
}

fn back_pressed(ui: &Ui) -> bool {
    ui.key_pressed(KeyCode::Char('['), Mods::META)
        && ![KeyCode::Shift, KeyCode::Alt, KeyCode::Ctrl].into_iter().any(|key| ui.key_down(key))
}

/// ⌘[ Back: detail to its index, folder to its parent (source root to the source list).
fn back_shortcut(ui: &Ui, player: &PlayerView<'_>, state: &ShellState) -> Option<Intent> {
    if !back_pressed(ui) {
        return None;
    }
    match state.route {
        Route::Album(_) => Some(Intent::Navigate(Route::Albums)),
        Route::Artist(_) => Some(Intent::Navigate(Route::Artists)),
        Route::Collection(_) => Some(Intent::Navigate(Route::Lists)),
        Route::Files => Some(files::up(player.files?.directory.as_ref()?)),
        _ => None,
    }
}

/// ⌘A, Esc and arrows over the song selection; raw scalar key queries only.
fn selection_shortcut(
    ui: &Ui,
    catalog: &Catalog,
    state: &ShellState,
    carrying: bool,
) -> Option<Intent> {
    let len = state.collection(catalog)?.rows.len();
    if ui.key_pressed(KeyCode::Char('a'), Mods::META) || ui.key_pressed(KeyCode::Char('A'), Mods::META) {
        return (len > 0).then_some(Intent::Select(Selection::All(len)));
    }
    if state.marked.is_empty() {
        return None;
    }
    let down = |key| ui.key_down(key);
    // ⇧⌘N: New List from Selection (the name dialog carries the selected songs).
    if down(KeyCode::Shift)
        && !down(KeyCode::Alt)
        && !down(KeyCode::Ctrl)
        && (ui.key_pressed(KeyCode::Char('n'), Mods::META) || ui.key_pressed(KeyCode::Char('N'), Mods::META))
    {
        return state
            .selected_song_ids(catalog)
            .next()
            .map(|song| Intent::NewList(Some(song)));
    }
    if ui.key_pressed(KeyCode::Backspace, Mods::ANY)
        && ![KeyCode::Meta, KeyCode::Shift, KeyCode::Alt, KeyCode::Ctrl]
            .into_iter()
            .any(down)
        && let Some(lw_core::context::PlaybackContext::Playlist { id, .. }) =
            state.collection(catalog).map(|c| &*c.context)
    {
        return state
            .selected_song_ids(catalog)
            .next()
            .map(|song| Intent::RemoveFromList { song, list: *id });
    }
    // Esc that cancels a drag is spent on the drag.
    if !carrying && ui.key_pressed(KeyCode::Escape, Mods::ANY) {
        return Some(Intent::Select(Selection::Clear));
    }
    if ui.view_size().width < compact::BREAKPOINT {
        return None;
    }
    let plain = ![KeyCode::Meta, KeyCode::Shift, KeyCode::Alt, KeyCode::Ctrl].into_iter().any(down);
    for (key, delta) in [(KeyCode::ArrowUp, -1), (KeyCode::ArrowDown, 1)] {
        let extend = ui.key_pressed(key, Mods::SHIFT);
        if extend || (plain && ui.key_pressed(key, Mods::ANY)) {
            return Some(Intent::Select(Selection::Move { delta, extend, len }));
        }
    }
    None
}

pub fn shell(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    catalog: &Catalog,
    state: &ShellState,
    intent: &mut Option<Intent>,
) {
    let input_was_focused = ui.text_input_focused();
    // a carry, or a list row lifted by its grip: Escape cancels it (a plain held press leaves Escape to the app)
    let carrying = ui.carrying().is_some() || ui.reordering();
    if ui.key_pressed(KeyCode::Escape, Mods::ANY) {
        ui.cancel_carry();
    }
    if state.commands_visible {
        commands::palette_view(ui, player, catalog, state, intent);
    } else if state.shortcuts_visible {
        commands::shortcuts(ui, state.top_inset, intent);
    }
    if state.queue_visible {
        queue_sheet(ui, player, catalog, state, intent);
    }
    if let Some((menu, at)) = state.settings_menu {
        settings::popup(ui, player, menu, at, intent);
    }
    if let Some((target, at)) = state.source_menu {
        files::popup(ui, player, target, at, intent);
    }
    if let Some((index, id, key, at)) = state.song_menu {
        songs::popup(
            ui,
            catalog,
            state,
            (index, id, key),
            at,
            player.sharing,
            intent,
        );
    }
    if let Some(target) = state.remove_source {
        files::confirm_remove(ui, player, target, intent);
    }
    if let Some(at) = state.page_menu {
        compact::page::menu(ui, catalog, state, at, intent);
    }
    if let Some((entry, at)) = state.folder_menu {
        files::folder_menu(ui, player, entry, at, intent);
    }
    if state.add_music {
        files::add_music(ui, player, intent);
    }
    if let Some((list, at)) = state.list_menu {
        lists::menu(ui, catalog, list, at, intent);
    }
    if let Some(list) = state.delete_list {
        lists::confirm_delete(ui, catalog, list, intent);
    }
    if let Some(dialog) = state.list_dialog {
        lists::name_dialog(ui, state, dialog, intent);
    }
    if let Some(edit) = state.metadata {
        metadata::editor(ui, state, edit, intent);
    }
    if let Some(song) = state.delete_song {
        metadata::confirm_delete(ui, catalog, song, intent);
    }
    if intent.is_none() && onboarding::needs_start_flag(catalog, player.settings.did_start) {
        *intent = Some(Intent::Setting(lw_core::preferences::Change::DidStart));
    }
    let root = if onboarding::wanted(catalog, state, player.settings.did_start) {
        onboarding::view(ui, state.top_inset, intent)
    } else if ui.view_size().width < compact::BREAKPOINT {
        state.song_window.set(None);
        compact::view(ui, player, position, catalog, state, intent)
    } else {
        desktop(ui, player, position, catalog, state, intent)
    };
    // Swift's long-press menu blurs the page behind it on a phone.
    if state.song_menu.is_some() && ui.view_size().width < compact::BREAKPOINT {
        let scrim = ui.view_of("song-menu-backdrop", &0, |ui| {
            ui.node(
                "scrim",
                Node::column().w(Size::Grow).h(Size::Grow).look(SCRIM),
            );
        });
        ui.set_view(&scrim, &pxui::ViewTransform::DEFAULT.blur(14.0));
    }
    // Swift's scrims: 0.28 behind the palette and help, 0.32 behind the queue sheet.
    let dim = if state.queue_visible {
        82
    } else if state.commands_visible || state.shortcuts_visible {
        71
    } else {
        0
    };
    if dim > 0 {
        ui.canvas_over(&root, move |paint, rect| {
            paint.fill(rect, Fill::new(SCRIM_INK.alpha(dim)));
        });
    }
    if player.file_dragging {
        const TITLE: pxui::Label = pxui::Label::BODY.size(40.0).color(INK);
        const NOTE: pxui::Label = pxui::Label::BODY.size(14.0).color(SECONDARY);
        ui.canvas_over(&root, move |paint, rect| {
            let (x, y, w, h) = (
                rect.x + 12.0,
                rect.y + 12.0,
                (rect.w - 24.0).max(0.0),
                (rect.h - 24.0).max(0.0),
            );
            paint.fill(Rect::new(x, y, w, h), Fill::new(Rgba::token(PAPER).alpha(240)));
            // Swift's drop border uses six-point dashes and gaps.
            for vertical in [false, true] {
                let length = if vertical { h } else { w };
                for offset in (0..length.ceil() as usize).step_by(12) {
                    let dash = 6.0f32.min(length - offset as f32);
                    if vertical {
                        paint.fill(Rect::new(x, y + offset as f32, 2.0, dash), Fill::new(Rgba::token(INK)));
                        paint.fill(Rect::new(x + w - 2.0, y + offset as f32, 2.0, dash), Fill::new(Rgba::token(INK)));
                    } else {
                        paint.fill(Rect::new(x + offset as f32, y, dash, 2.0), Fill::new(Rgba::token(INK)));
                        paint.fill(Rect::new(x + offset as f32, y + h - 2.0, dash, 2.0), Fill::new(Rgba::token(INK)));
                    }
                }
            }
            // the first baselines where Swift puts them: 7 pt under the centre (the 40 pt title) and 36.3 pt under it (the note);
            // `text` places the layout top, which the painter's own measure in the drawn style puts `baseline` above them
            for (text, baseline, style) in [
                ("Drop to play", rect.y + rect.h * 0.5 + 7.0, TITLE),
                ("Folders are indexed. Audio files join Added Files.", rect.y + rect.h * 0.5 + 36.3, NOTE),
            ] {
                let Ok(m) = paint.text_service().measure(text, style, None) else { continue };
                paint.text(pxui::Point::new(rect.x + (rect.w - m.width) * 0.5, baseline - m.baseline), style, 0.0, text);
            }
        });
    }
    if intent.is_none()
        && !state.commands_visible
        && !state.shortcuts_visible
        && !state.queue_visible
        && !state.settings_visible
        && state.song_menu.is_none()
        && state.list_dialog.is_none()
        && state.delete_list.is_none()
        && state.metadata.is_none()
        && state.delete_song.is_none()
        && state.remove_source.is_none()
        && ui.key_pressed(KeyCode::Enter, Mods::ANY)
        && !input_was_focused
        && !ui.text_input_focused()
        && let Some(collection) = state.collection(catalog)
    {
        // Swift plays the whole selection; one row keeps its playback position.
        if state.marked.len() > 1 {
            *intent = Some(Intent::PlaySelection);
        } else if let Some(index) = state.selected.filter(|&i| i < collection.rows.len()) {
            *intent = Some(Intent::PlayRow(index));
        }
    }
    let settings_shortcut = ui.key_pressed(KeyCode::Char(','), Mods::META);
    if !state.commands_visible && !state.shortcuts_visible && settings_shortcut {
        *intent = Some(Intent::ShowSettings);
    }
    if intent.is_none()
        && !input_was_focused
        && !ui.text_input_focused()
        && !state.commands_visible
        && !state.shortcuts_visible
        && !state.queue_visible
        && !state.playlist_editing
        && state.list_dialog.is_none()
        && state.list_menu.is_none()
        && state.delete_list.is_none()
        && state.metadata.is_none()
        && state.delete_song.is_none()
        && state.song_menu.is_none()
        && state.source_menu.is_none()
        && state.remove_source.is_none()
    {
        *intent = if state.settings_visible {
            // Swift's settings page: ⌘[ goes back, to the settings list from privacy.
            (state.settings_menu.is_none() && back_pressed(ui)).then_some(if state.privacy_visible {
                Intent::ClosePrivacy
            } else {
                Intent::CloseSettings
            })
        } else {
            global_shortcut(ui, player, catalog, position)
                .or_else(|| selection_shortcut(ui, catalog, state, carrying))
                .or_else(|| back_shortcut(ui, player, state))
        };
    }
    if !state.commands_visible && !state.shortcuts_visible && !state.queue_visible {
        if ui.key_pressed(KeyCode::Char('k'), Mods::META)
            || ui.key_pressed(KeyCode::Char('K'), Mods::META)
            || (!ui.text_input_focused() && ui.key_pressed(KeyCode::Char('/'), Mods::ANY))
        {
            *intent = Some(Intent::ShowCommands);
        } else if !ui.text_input_focused() && ui.key_pressed(KeyCode::Char('?'), Mods::ANY) {
            *intent = Some(Intent::ShowShortcuts);
        }
    }
    // Swift's Esc closes the compact Now Playing panel (wide layouts keep it docked).
    if intent.is_none()
        && state.player_expanded
        && !state.settings_visible
        && !state.commands_visible
        && !state.shortcuts_visible
        && !state.queue_visible
        && state.settings_menu.is_none()
        && state.source_menu.is_none()
        && state.remove_source.is_none()
        && state.song_menu.is_none()
        && state.list_menu.is_none()
        && state.list_dialog.is_none()
        && state.delete_list.is_none()
        && ui.view_size().width < compact::BREAKPOINT
        && ui.key_pressed(KeyCode::Escape, Mods::ANY)
    {
        *intent = Some(Intent::TogglePlayer);
    }
    if !state.commands_visible
        && !state.shortcuts_visible
        && state.settings_visible
        && !state.queue_visible
        && ui.key_pressed(KeyCode::Escape, Mods::ANY)
    {
        *intent = Some(if state.settings_menu.is_some() {
            Intent::CloseSettingsMenu
        } else if state.privacy_visible {
            Intent::ClosePrivacy
        } else {
            Intent::CloseSettings
        });
    }
}
mod kit;
pub use kit::apply_palette;
pub(crate) use theme::register_bodies;
#[cfg(test)]
pub(crate) use kit::TIGHT_3;
use kit::*;
mod theme;
use theme::*;
mod sidebar;
use sidebar::*;
mod header;
mod lists;
mod metadata;
use header::*;
mod now;
use now::*;
mod queue_sheet;
use queue_sheet::*;
mod clock;
use clock::*;
pub(crate) use clock::stack_fmt;
mod transport;
use transport::*;
