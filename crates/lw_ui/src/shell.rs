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
use crate::{Intent, PlayerView};
use pxui::ui::*;

mod albums;
mod artists;
mod byline;
pub(crate) mod commands;
mod compact;
mod files;
mod find;
mod settings;
mod songs;

pub struct ShellState {
    pub route: Route,
    pub selected: Option<usize>,
    pub panel_visible: bool,
    pub query: Query,
    pub sort: Sort,
    pub ascending: bool,
    pub album_sort: crate::browse::AlbumSort,
    player_expanded: bool,
    queue_visible: bool,
    playlist_editing: bool,
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
            route: Route::Songs,
            selected: None,
            panel_visible: true,
            query: Query::default(),
            sort: Sort::Original,
            ascending: true,
            album_sort: crate::browse::AlbumSort::Artist,
            player_expanded: false,
            queue_visible: false,
            playlist_editing: false,
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
        let same_view = match (&previous.view, &next.view) {
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if std::sync::Arc::ptr_eq(&previous.library, &next.library) && same_view {
            return;
        }
        self.song_menu = None;
        let old = self.collection(previous);
        let selected = old.and_then(|group| {
            self.selected
                .and_then(|index| group.ids.get(index).copied())
        });
        let occurrence = old.zip(self.selected).map_or(0, |(group, index)| {
            group.ids[..index.min(group.ids.len())]
                .iter()
                .filter(|id| Some(**id) == selected)
                .count()
        });
        let same = |group: &crate::catalog::Collection| {
            old.is_some_and(|old| match (&*old.context, &*group.context) {
                (
                    lw_core::context::PlaybackContext::Playlist { id: a, .. },
                    lw_core::context::PlaybackContext::Playlist { id: b, .. },
                ) => a == b,
                (a, b) => a == b,
            })
        };
        self.route = match self.route {
            Route::Album(_) => next
                .albums
                .iter()
                .position(same)
                .map_or(Route::Albums, Route::Album),
            Route::Artist(_) => next
                .artists
                .iter()
                .position(same)
                .map_or(Route::Artists, Route::Artist),
            Route::Collection(_) => next
                .collections
                .iter()
                .position(same)
                .map_or(Route::Songs, Route::Collection),
            route => route,
        };
        self.selected = self
            .collection(next)
            .filter(|group| same(group))
            .and_then(|group| {
                selected.and_then(|selected| {
                    group
                        .ids
                        .iter()
                        .enumerate()
                        .filter(|(_, id)| **id == selected)
                        .nth(occurrence)
                        .map(|(index, _)| index)
                })
            });
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn apply(&mut self, intent: Intent) -> bool {
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
                    self.settings_menu = None;
                    self.source_menu = None;
                }
            }
            Intent::CloseSongMenu => self.song_menu = None,
            Intent::Navigate(route) => {
                self.commands_visible = false;
                self.shortcuts_visible = false;
                self.source_menu = None;
                self.remove_source = None;
                self.settings_visible = false;
                self.privacy_visible = false;
                self.settings_menu = None;
                self.player_expanded = false;
                self.route = route;
                self.selected = None;
                self.sort = Sort::Original;
                self.ascending = true;
            }
            Intent::SortAlbums(sort) => self.album_sort = sort,
            Intent::SelectRow(index) => self.selected = Some(index),
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
            Intent::EditPlaylistName(name) => {
                self.playlist_name = name;
                self.playlist_name_too_long = false;
            }
            Intent::PlaylistNameTooLong => self.playlist_name_too_long = true,
            Intent::CancelPlaylistName => self.playlist_editing = false,
            Intent::Search(query) => {
                self.query = query;
                self.selected = None;
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

pub fn shell(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    catalog: &Catalog,
    state: &ShellState,
    intent: &mut Option<Intent>,
) {
    let input_was_focused = ui.text_input_focused();
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
        songs::popup(ui, catalog, state, (index, id, key), at, p, intent);
    }
    if let Some(target) = state.remove_source {
        files::confirm_remove(ui, player, target, p, intent);
    }
    let root = if ui.view_size().0 < compact::BREAKPOINT {
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
use header::*;
mod now;
use now::*;
mod queue_sheet;
use queue_sheet::*;
mod clock;
use clock::*;
mod transport;
use transport::*;
