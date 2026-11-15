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
    song_window: std::cell::Cell<Option<crate::song_text::Window>>,
    pub route: Route,
    pub selected: Option<usize>,
    /// Sorted row positions of the multi-selection; grows only in `apply`/`refresh`, never in frames.
    marked: Vec<usize>,
    anchor: Option<usize>,
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
            song_window: std::cell::Cell::new(None),
            route: Route::Songs,
            selected: None,
            marked: Vec::new(),
            anchor: None,
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
        if !self.marked.is_empty() || self.anchor.is_some() {
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
            _ => {}
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
            Intent::CloseSourceMenu => self.source_menu = None,
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
    pxui::hash!((
        state.revision,
        state.selection_revision,
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
    let (meta, alt) = (input::Key::Meta.code(), input::Key::Alt.code());
    let (shift, ctrl) = (input::Key::Shift.code(), input::Key::Ctrl.code());
    // Swift matches exact modifiers, so ⌘N must not also fire on ⇧⌘N.
    let pressed = |key: u32, modifier: u32| {
        let upper = char::from_u32(key).map_or(key, |c| c.to_ascii_uppercase() as u32);
        let exact = [meta, alt, shift, ctrl]
            .iter()
            .all(|&m| m == modifier || !ui.key_down(m));
        if modifier == 0 {
            ui.key_pressed(key) && exact
        } else {
            exact && (ui.key_pressed_with(key, modifier) || ui.key_pressed_with(upper, modifier))
        }
    };
    // ⇧⌘O is Add Files; Add Folder (⌘O) stays exact above.
    if ui.key_down(shift)
        && !ui.key_down(alt)
        && !ui.key_down(ctrl)
        && (ui.key_pressed_with('o' as u32, meta) || ui.key_pressed_with('O' as u32, meta))
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
    for (key, modifier, intent) in [
        ('f', meta, Intent::Navigate(Route::Find)),
        ('n', meta, Intent::NewList(None)),
        ('1', meta, Intent::Navigate(Route::Albums)),
        ('2', meta, Intent::Navigate(Route::Artists)),
        ('3', meta, Intent::Navigate(Route::Songs)),
    ] {
        if pressed(key as u32, modifier) {
            return Some(intent);
        }
    }
    let shift = input::Key::Shift.code();
    for (key, modifier, intent) in [
        (input::Key::ArrowLeft, meta, Intent::Previous),
        (input::Key::ArrowRight, meta, Intent::Next),
        (input::Key::ArrowLeft, shift, Intent::Previous),
        (input::Key::ArrowRight, shift, Intent::Next),
        (
            input::Key::ArrowUp,
            meta,
            Intent::Volume((player.volume + 1.0 / 12.0).min(1.0)),
        ),
        (
            input::Key::ArrowDown,
            meta,
            Intent::Volume((player.volume - 1.0 / 12.0).max(0.0)),
        ),
    ] {
        if pressed(key.code(), modifier) {
            return Some(intent);
        }
    }
    if player.duration > 0.0 {
        for (key, modifier, seconds) in [
            (input::Key::ArrowLeft, 0, -5.0),
            (input::Key::ArrowRight, 0, 5.0),
            (input::Key::ArrowLeft, alt, -30.0),
            (input::Key::ArrowRight, alt, 30.0),
        ] {
            if pressed(key.code(), modifier) {
                let to = ((position + seconds) / player.duration).clamp(0.0, 1.0);
                return Some(Intent::SeekFraction(to));
            }
        }
    }
    None
}

/// ⌘A, Esc and arrows over the song selection; raw scalar key queries only.
fn selection_shortcut(
    ui: &Ui,
    catalog: &Catalog,
    state: &ShellState,
    carrying: bool,
) -> Option<Intent> {
    let len = state.collection(catalog)?.rows.len();
    let (meta, shift) = (input::Key::Meta.code(), input::Key::Shift.code());
    if ui.key_pressed_with('a' as u32, meta) || ui.key_pressed_with('A' as u32, meta) {
        return (len > 0).then_some(Intent::Select(Selection::All(len)));
    }
    if state.marked.is_empty() {
        return None;
    }
    // Esc that cancels a drag is spent on the drag.
    if !carrying && ui.key_pressed(input::Key::Escape.code()) {
        return Some(Intent::Select(Selection::Clear));
    }
    if ui.view_size().0 < compact::BREAKPOINT {
        return None;
    }
    let plain = ![meta, shift, input::Key::Alt.code(), input::Key::Ctrl.code()]
        .iter()
        .any(|&m| ui.key_down(m));
    for (key, delta) in [(input::Key::ArrowUp, -1), (input::Key::ArrowDown, 1)] {
        let extend = ui.key_pressed_with(key.code(), shift);
        if extend || (plain && ui.key_pressed(key.code())) {
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
    let carrying = ui.carrying_ref().is_some();
    if ui.key_pressed(input::Key::Escape.code()) {
        ui.cancel_carry();
    }
    let p = palette(ui);
    if state.commands_visible {
        commands::palette_view(ui, player, catalog, state, p, intent);
    } else if state.shortcuts_visible {
        commands::shortcuts(ui, p, intent);
    }
    if state.queue_visible {
        queue_sheet(ui, player, catalog, state, p, intent);
    }
    if let Some((menu, at)) = state.settings_menu {
        settings::popup(ui, player, menu, at, p, intent);
    }
    if let Some((target, at)) = state.source_menu {
        files::popup(ui, player, target, at, p, intent);
    }
    if let Some((index, id, key, at)) = state.song_menu {
        songs::popup(
            ui,
            catalog,
            state,
            (index, id, key),
            at,
            player.sharing,
            p,
            intent,
        );
    }
    if let Some(target) = state.remove_source {
        files::confirm_remove(ui, player, target, p, intent);
    }
    if let Some((list, at)) = state.list_menu {
        lists::menu(ui, catalog, list, at, p, intent);
    }
    if let Some(list) = state.delete_list {
        lists::confirm_delete(ui, catalog, list, p, intent);
    }
    if let Some(dialog) = state.list_dialog {
        lists::name_dialog(ui, state, dialog, p, intent);
    }
    if let Some(edit) = state.metadata {
        metadata::editor(ui, state, edit, p, intent);
    }
    if let Some(song) = state.delete_song {
        metadata::confirm_delete(ui, catalog, song, p, intent);
    }
    let root = if onboarding::wanted(catalog, state) {
        onboarding::view(ui, p, intent)
    } else if ui.view_size().0 < compact::BREAKPOINT {
        state.song_window.set(None);
        compact::view(ui, player, position, catalog, state, p, intent)
    } else {
        desktop(ui, player, position, catalog, state, p, intent)
    };
    if player.file_dragging {
        ui.draw_over(&root, move |paint, rect| {
            let (x, y, w, h) = (
                rect.x + 12.0,
                rect.y + 12.0,
                (rect.w - 24.0).max(0.0),
                (rect.h - 24.0).max(0.0),
            );
            paint.fill(x, y, w, h, Rgba { a: 240, ..p.paper });
            // Swift's drop border uses six-point dashes and gaps.
            for vertical in [false, true] {
                let length = if vertical { h } else { w };
                for offset in (0..length.ceil() as usize).step_by(12) {
                    let dash = 6.0f32.min(length - offset as f32);
                    if vertical {
                        paint.fill(x, y + offset as f32, 2.0, dash, p.ink);
                        paint.fill(x + w - 2.0, y + offset as f32, 2.0, dash, p.ink);
                    } else {
                        paint.fill(x + offset as f32, y, dash, 2.0, p.ink);
                        paint.fill(x + offset as f32, y + h - 2.0, dash, 2.0, p.ink);
                    }
                }
            }
            for (text, size, at, color) in [
                ("Drop to play", 40, rect.y + rect.h * 0.5 - 34.0, p.ink),
                (
                    "Folders are indexed. Audio files join Added Files.",
                    14,
                    rect.y + rect.h * 0.5 + 22.0,
                    p.secondary,
                ),
            ] {
                let width = paint.text_width(Some(size), text);
                paint.text_s((rect.x + (rect.w - width) * 0.5, at), size, color, text);
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
        && ui.key_pressed(input::Key::Enter)
        && !input_was_focused
        && !ui.text_input_focused()
        && let Some(index) = state.selected
        && state
            .collection(catalog)
            .is_some_and(|collection| index < collection.rows.len())
    {
        *intent = Some(Intent::PlayRow(index));
    }
    let settings_shortcut = ui.key_pressed_with(',' as u32, input::Key::Meta.code());
    if !state.commands_visible && !state.shortcuts_visible && settings_shortcut {
        *intent = Some(Intent::ShowSettings);
    }
    if intent.is_none()
        && !input_was_focused
        && !ui.text_input_focused()
        && !state.commands_visible
        && !state.shortcuts_visible
        && !state.queue_visible
        && !state.settings_visible
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
        *intent = global_shortcut(ui, player, catalog, position)
            .or_else(|| selection_shortcut(ui, catalog, state, carrying));
    }
    if !state.commands_visible && !state.shortcuts_visible && !state.queue_visible {
        if ui.key_pressed_with('k' as u32, input::Key::Meta.code())
            || ui.key_pressed_with('K' as u32, input::Key::Meta.code())
            || (!ui.text_input_focused() && ui.key_pressed('/'))
        {
            *intent = Some(Intent::ShowCommands);
        } else if !ui.text_input_focused() && ui.key_pressed('?') {
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
        && ui.view_size().0 < compact::BREAKPOINT
        && ui.key_pressed(input::Key::Escape.code())
    {
        *intent = Some(Intent::TogglePlayer);
    }
    if !state.commands_visible
        && !state.shortcuts_visible
        && state.settings_visible
        && !state.queue_visible
        && ui.key_pressed(input::Key::Escape.code())
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
use kit::*;
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
mod transport;
use transport::*;
