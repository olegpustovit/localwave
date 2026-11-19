use super::*;
use crate::Intent;
use crate::catalog::{Catalog, Route};
use crate::files::Location;

pub(super) fn side_row(
    ui: &mut Ui,
    key: &'static str,
    label: (&str, &str),
    route: Route,
    state: &ShellState,
    intent: &mut Option<Intent>,
) {
    side_item(ui, (key, 0), label, Go::Route(route, None), "", state, intent)
}

/// Where a sidebar row leads; list rows also take song drops and a menu.
#[derive(Clone, Copy)]
enum Go {
    Route(Route, Option<i64>),
    Source(Location),
}

/// A sidebar row; a `list` id adds Swift's list menu on secondary press.
#[allow(clippy::too_many_arguments)]
fn side_item(
    ui: &mut Ui,
    (key, id): (&'static str, i64),
    label: (&str, &str),
    go: Go,
    badge: &str,
    state: &ShellState,
    intent: &mut Option<Intent>,
) {
    let (title, count) = label;
    let (route, list) = match go {
        Go::Route(route, list) => (Some(route), list),
        Go::Source(_) => (None, None),
    };
    let selected = !state.settings_visible && route.is_some_and(|route| state.route == route);
    let spec = Node::row().w(Size::Grow).h_px(32.0).pad_x(8.0).align(Align::Center).role(Role::Button);
    let row = PLAIN.look(if selected { &SELECTED_LOOKS } else { &SIDE_ROW_LOOKS });
    let key: Key = if id == 0 { key.into() } else { Key::Id(int_key(pxui::ui::key_of(0, key), id)) };
    let node = ui.pressable(key, spec, &row, |ui, _| {
        ui.label(title, TEXT_14.w(if badge.is_empty() { Size::Grow } else { Size::Fit }).h_px(20.0).color(if selected { INK } else { SECONDARY }));
        if !badge.is_empty() {
            let tag = ui.label(badge, TEXT_11.h_px(16.0).color(PAPER));
            ui.style(&tag, &pxui::theme::States::of(BADGE), State::default());
            ui.node("badge-gap", Node::DEFAULT.w(Size::Grow));
        }
        ui.label(count, MONO_12.w_px(44.0).h_px(18.0).color(SECONDARY));
    });
    ui.a11y_name(&node.handle, title);
    let dropping = list.and_then(|_| super::songs::song_drop(ui, &node.handle));
    if dropping.is_some() {
        ui.style(&node.handle, &pxui::theme::States::of(SELECTED_LOOKS.idle), State::default());
    }
    if let (Some(list), Some((true, song))) = (list, dropping) {
        *intent = Some(Intent::AddToList { song, list });
    } else if node.clicked {
        *intent = Some(match go {
            Go::Route(route, _) => Intent::Navigate(route),
            Go::Source(location) => Intent::Files(crate::files::Request::Browse(location)),
        });
    }
    if let Some(list) = list
        && node.secondary
    {
        *intent = Some(Intent::ListMenu(list, ui.signal(&node.handle).pointer));
    }
}

/// A Swift group label.
const GROUP: Label = TEXT_12.grow().h_px(30.0).color(SECONDARY);

/// A plain "+ label" action row (Swift's Add folder or files, New list).
fn add_row(ui: &mut Ui, key: &'static str, title: &str, name: &str) -> bool {
    let row = ui.button_content(key, &ADD_ROW, |ui, _| {
        // the text starts 8 points in, as Swift's does; `button_with` centres it
        let copy = ui.node("label", Node::column().w(Size::Grow).h(Size::Grow).padding(Edges::ZERO.l(8.0)).look(LINK_TEXT));
        ui.text(&copy, title);
    });
    ui.a11y_name(&row.handle, name);
    row.clicked
}
/// A "+ label" action row: the sidebar's width, 32 points high.
const ADD_ROW: pxui::Button = pxui::Button::DEFAULT.size(Size::Grow, Size::Px(32.0)).centered().look(&BARE);

pub(super) fn sidebar(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    intent: &mut Option<Intent>,
) {
    ui.column(Node::column().w(Size::Grow).h(Size::Grow).pad(10.0), |ui| {
        spacer(ui, "sidebar-top", 28.0);
        let node = Node::row().w(Size::Grow).h_px(34.0).pad_x(10.0).align(Align::Center).role(Role::Button);
        let search = ui.pressable("search", node, &PLAIN.look(&SEARCH_ROW_LOOKS), |ui, _| {
            ui.label("Search", TEXT_14.grow().h_px(20.0).color(SECONDARY));
            ui.label("⌘K", MONO_11.h_px(16.0).color(SECONDARY));
        });
        ui.a11y_name(&search.handle, "Search");
        if search.clicked {
            *intent = Some(Intent::ShowCommands);
        }
        spacer(ui, "search-gap", 12.0);
        ui.label("Library", GROUP);
        side_row(ui, "albums", ("Albums", &catalog.album_count), Route::Albums, state, intent);
        side_row(ui, "artists", ("Artists", &catalog.artist_count), Route::Artists, state, intent);
        side_row(ui, "songs", ("Songs", &catalog.song_count), Route::Songs, state, intent);
        ui.label("Sources", GROUP);
        for source in &catalog.source_rows {
            side_item(
                ui,
                ("source", source.location.source),
                (&source.name, &source.count),
                Go::Source(source.location),
                if source.reconnect { "Reconnect" } else { "" },
                state,
                intent,
            );
        }
        if add_row(ui, "add-music", "+  Add folder or files", "Add folder or files") {
            *intent = Some(Intent::ChooseAudio);
        }
        ui.label("Lists", GROUP);
        let collections = catalog.collections.len();
        ui.grid_list("collections", collections + 1, &pxui::GridList::DEFAULT.w_px((width - 20.0).max(1.0)).h_px(32.0), &mut |ui, index| {
            let Some(collection) = catalog.collections.get(index) else {
                if add_row(ui, "new-list", "+  New list", "New list") {
                    *intent = Some(Intent::NewList(None));
                }
                return;
            };
            let list = match &*collection.context {
                lw_core::context::PlaybackContext::Playlist { id, .. } => Some(*id),
                _ => None,
            };
            side_item(
                ui,
                ("collection", 0),
                (&collection.title, &collection.count),
                Go::Route(Route::Collection(index), list),
                "",
                state,
                intent,
            );
        });
        ui.row(Node::row().w(Size::Grow).h_px(30.0).padding(Edges::ZERO.t(14.0).l(8.0).r(8.0)),
            |ui| {
                ui.label("Offline", TEXT_12.grow().h_px(16.0).color(SECONDARY));
                ui.label(&catalog.offline_usage, MONO_12.h_px(16.0).color(SECONDARY));
            },
        );
        ui.row(Node::row().w(Size::Grow).h_px(4.0).margin(Edges::ZERO.t(10.0).l(8.0).r(8.0)).look(LINE_FILL),
            |ui| {
                container(ui, "fill", Axis::Column, Size::Pct(catalog.offline_fraction), Size::Grow, &INK_FILL);
            },
        );
        let node = Node::row().w(Size::Grow).h_px(28.0).role(Role::Button).padding(Edges::ZERO.t(10.0).l(8.0));
        let shortcuts = ui.pressable("shortcuts", node, &PLAIN, |ui, _| {
            ui.label("?", MONO_11.w_px(14.0).h_px(16.0).color(SECONDARY));
            ui.label("shortcuts", TEXT_12.h_px(16.0).color(SECONDARY));
        });
        ui.a11y_name(&shortcuts.handle, "Keyboard shortcuts");
        if shortcuts.clicked {
            *intent = Some(Intent::ShowShortcuts);
        }
    });
}
