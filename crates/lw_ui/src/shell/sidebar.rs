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
    let spec = Node::row()
        .w(Size::Grow)
        .h_px(32.0)
        .pad_x(8.0)
        .align(Align::Center)
        .role(Role::Button)
        .flags(Flags::CLICKABLE | Flags::TAB_STOP);
    let node = if id == 0 {
        ui.node(key.into(), spec)
    } else {
        ui.node(Key::Id(int_key(Ui::key_of(0, key), id)), spec)
    };
    let signal = ui.signal(&node);
    let dropping = list.and_then(|_| super::songs::song_drop(ui, &node));
    ui.set_look(
        &node,
        Look {
            fill: if dropping.is_some() || selected {
                ui.color(SELECTED)
            } else if signal.hovered {
                ui.color(PANEL)
            } else {
                ui.color(PAPER)
            },
            ..Look::NONE
        },
        State::default(),
    );
    ui.a11y_name(&node, title);
    ui.within(&node, &mut |ui| {
        ui.label(title, TEXT_14.w(if badge.is_empty() { Size::Grow } else { Size::Fit }).h(20.0).color(if selected { INK } else { SECONDARY }));
        if !badge.is_empty() {
            let tag = ui.label(badge, TEXT_11.h(16.0).color(PAPER));
            ui.set_look(&tag, Look { fill: ui.color(INK), text: ui.color(PAPER), font: Font { size: 11, ..Font::BODY }, ..Look::NONE }, State::default());
            ui.box_("badge-gap", BoxOpts::default().w(Size::Grow));
        }
        ui.label(count, MONO_12.w(Size::Px(44.0)).h(18.0).color(SECONDARY));
    });
    if let (Some(list), Some((true, song))) = (list, dropping) {
        *intent = Some(Intent::AddToList { song, list });
    } else if signal.clicked {
        *intent = Some(match go {
            Go::Route(route, _) => Intent::Navigate(route),
            Go::Source(location) => Intent::Files(crate::files::Request::Browse(location)),
        });
    }
    if let Some(list) = list
        && signal.pressed
        && signal.button == Some(1)
    {
        *intent = Some(Intent::ListMenu(list, signal.pointer));
    }
}

/// A Swift group label.
const GROUP: Label = TEXT_12.w(Size::Grow).h(30.0).color(SECONDARY);

/// A plain "+ label" action row (Swift's Add folder or files, New list).
fn add_row(ui: &mut Ui, key: &'static str, title: &str, name: &str) -> bool {
    let theme = looks(ui);
    let row = ui.button_content(key, &ADD_ROW, |ui, _| {
        // the text starts 8 points in, as Swift's does; `button_with` centres it
        let copy = ui.node("label".into(), Node { padding: Edges { l: 8.0, ..Edges::ZERO }, look: theme.add_row, ..Node::column().w(Size::Grow).h(Size::Grow) });
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
    ui.column("sidebar-content", Node::column().w(Size::Grow).h(Size::Grow).pad(10.0), |ui| {
        spacer(ui, "sidebar-top", 28.0);
        let search = ui.node(
            "search".into(),
            Node::row()
                .w(Size::Grow)
                .h_px(34.0)
                .pad_x(10.0)
                .align(Align::Center)
                .role(Role::Button)
                .flags(Flags::CLICKABLE | Flags::TAB_STOP),
        );
        let signal = ui.signal(&search);
        ui.set_look(&search, Look { fill: if signal.hovered { ui.color(SELECTED) } else { ui.color(PANEL) }, ..Look::NONE }, State::default());
        ui.a11y_name(&search, "Search");
        ui.within(&search, &mut |ui| {
            ui.label("Search", TEXT_14.w(Size::Grow).h(20.0).color(SECONDARY));
            ui.label("⌘K", MONO_11.h(16.0).color(SECONDARY));
        });
        if signal.clicked {
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
        ui.grid_list("collections", collections + 1, (width - 20.0).max(1.0), 32.0, &mut |ui, index| {
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
        ui.row(
            "offline",
            Node { padding: Edges { t: 14.0, l: 8.0, r: 8.0, ..Edges::ZERO }, ..Node::row().w(Size::Grow).h_px(30.0) },
            |ui| {
                ui.label("Offline", TEXT_12.w(Size::Grow).h(16.0).color(SECONDARY));
                ui.label(&catalog.offline_usage, MONO_12.h(16.0).color(SECONDARY));
            },
        );
        ui.row(
            "offline-meter",
            Node {
                margin: Edges { t: 10.0, l: 8.0, r: 8.0, ..Edges::ZERO },
                look: Look { fill: ui.color(LINE), ..Look::NONE },
                ..Node::row().w(Size::Grow).h_px(4.0)
            },
            |ui| {
                container(ui, "fill", Axis::Column, Size::Pct(catalog.offline_fraction), Size::Grow, ui.color(INK));
            },
        );
        let shortcuts = ui.node(
            "shortcuts".into(),
            Node {
                padding: Edges { t: 10.0, l: 8.0, ..Edges::ZERO },
                ..Node::row().w(Size::Grow).h_px(28.0).role(Role::Button).flags(Flags::CLICKABLE | Flags::TAB_STOP)
            },
        );
        ui.a11y_name(&shortcuts, "Keyboard shortcuts");
        ui.within(&shortcuts, &mut |ui| {
            ui.label("?", MONO_11.w(Size::Px(14.0)).h(16.0).color(SECONDARY));
            ui.label("shortcuts", TEXT_12.h(16.0).color(SECONDARY));
        });
        if ui.signal(&shortcuts).clicked {
            *intent = Some(Intent::ShowShortcuts);
        }
    });
}
