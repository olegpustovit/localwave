use super::*;
use crate::Intent;
use crate::catalog::{Catalog, Route};
use crate::files::Location;

pub(super) fn side_row(
    ui: &mut Ui,
    key: &str,
    label: (&str, &str),
    route: Route,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    side_item(ui, (key, 0), label, Go::Route(route, None), "", state, p, intent)
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
    (key, id): (&str, i64),
    label: (&str, &str),
    go: Go,
    badge: &str,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let (title, count) = label;
    let (route, list) = match go {
        Go::Route(route, list) => (Some(route), list),
        Go::Source(_) => (None, None),
    };
    let selected = !state.settings_visible && route.is_some_and(|route| state.route == route);
    let spec = Node {
            flags: CLICKABLE | TAB_STOP,
            axis: Axis::Row,
            w: Size::Grow,
            h: Size::Px(32.0),
            role: Role::Button,
            padding: Edges {
                l: 8.0,
                r: 8.0,
                ..Edges::ZERO
            },
            align: Align::Center,
            ..Node::DEFAULT
    };
    let node = if id == 0 {
        ui.node(key, spec)
    } else {
        ui.node_keyed(int_key(Ui::key_of(0, key), id), spec)
    };
    let signal = ui.signal_state(&node);
    let dropping = list.and_then(|_| super::songs::song_drop(ui, &node));
    ui.set_look(
        &node,
        Look {
            fill: if dropping.is_some() || selected {
                p.selected
            } else if signal.hovered {
                p.panel
            } else {
                p.paper
            },
            ..Look::NONE
        },
        State::default(),
    );
    ui.a11y_name(&node, title);
    ui.within(&node, &mut |ui| {
        text(
            ui,
            "title",
            title,
            14,
            if selected { p.ink } else { p.secondary },
            (if badge.is_empty() { Size::Grow } else { Size::Fit }, 20.0),
            false,
        );
        if !badge.is_empty() {
            let tag = text(ui, "badge", badge, 11, p.paper, (Size::Fit, 16.0), false);
            ui.set_look(&tag, Look { fill: p.ink, text: p.paper, font: Font { size: 11, ..Font::BODY }, ..Look::NONE }, State::default());
            ui.box_("badge-gap", BoxOpts::default().w(Size::Grow));
        }
        text(
            ui,
            "count",
            count,
            12,
            p.secondary,
            (Size::Px(44.0), 18.0),
            true,
        );
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
fn group(ui: &mut Ui, key: &str, title: &str, p: Palette) {
    text(ui, key, title, 12, p.secondary, (Size::Grow, 30.0), false);
}

/// A plain "+ label" action row (Swift's Add folder or files, New list).
fn add_row(ui: &mut Ui, key: &str, title: &str, name: &str, p: Palette) -> bool {
    let row = ui.node(
        key,
        Node {
            flags: CLICKABLE | TAB_STOP,
            role: Role::Button,
            w: Size::Grow,
            h: Size::Px(32.0),
            padding: Edges { l: 8.0, ..Edges::ZERO },
            look: Look { text: p.secondary, font: Font { size: 14, ..Font::BODY }, ..Look::NONE },
            ..Node::DEFAULT
        },
    );
    ui.text(&row, title);
    ui.a11y_name(&row, name);
    ui.signal_state(&row).clicked
}

pub(super) fn sidebar(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let root = ui.node(
        "sidebar-content",
        Node { w: Size::Grow, h: Size::Grow, padding: Edges::all(10.0), ..Node::DEFAULT },
    );
    ui.within(&root, &mut |ui| {
        spacer(ui, "sidebar-top", 28.0);
        let search = ui.node(
            "search",
            Node {
                flags: CLICKABLE | TAB_STOP,
                axis: Axis::Row,
                role: Role::Button,
                w: Size::Grow,
                h: Size::Px(34.0),
                padding: Edges { l: 10.0, r: 10.0, ..Edges::ZERO },
                align: Align::Center,
                ..Node::DEFAULT
            },
        );
        let signal = ui.signal_state(&search);
        ui.set_look(&search, Look { fill: if signal.hovered { p.selected } else { p.panel }, ..Look::NONE }, State::default());
        ui.a11y_name(&search, "Search");
        ui.within(&search, &mut |ui| {
            text(ui, "label", "Search", 14, p.secondary, (Size::Grow, 20.0), false);
            text(ui, "hint", "⌘K", 11, p.secondary, (Size::Fit, 16.0), true);
        });
        if signal.clicked {
            *intent = Some(Intent::ShowCommands);
        }
        spacer(ui, "search-gap", 12.0);
        group(ui, "library-heading", "Library", p);
        side_row(ui, "albums", ("Albums", &catalog.album_count), Route::Albums, state, p, intent);
        side_row(ui, "artists", ("Artists", &catalog.artist_count), Route::Artists, state, p, intent);
        side_row(ui, "songs", ("Songs", &catalog.song_count), Route::Songs, state, p, intent);
        group(ui, "sources-heading", "Sources", p);
        for source in &catalog.source_rows {
            side_item(
                ui,
                ("source", source.location.source),
                (&source.name, &source.count),
                Go::Source(source.location),
                if source.reconnect { "Reconnect" } else { "" },
                state,
                p,
                intent,
            );
        }
        if add_row(ui, "add-music", "+  Add folder or files", "Add folder or files", p) {
            *intent = Some(Intent::ChooseAudio);
        }
        group(ui, "lists-heading", "Lists", p);
        let collections = catalog.collections.len();
        ui.grid_list("collections", collections + 1, (width - 20.0).max(1.0), 32.0, &mut |ui, index| {
            let Some(collection) = catalog.collections.get(index) else {
                if add_row(ui, "new-list", "+  New list", "New list", p) {
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
                p,
                intent,
            );
        });
        let footer = ui.node(
            "offline",
            Node { axis: Axis::Row, w: Size::Grow, h: Size::Px(30.0), padding: Edges { t: 14.0, l: 8.0, r: 8.0, ..Edges::ZERO }, ..Node::DEFAULT },
        );
        ui.within(&footer, &mut |ui| {
            text(ui, "label", "Offline", 12, p.secondary, (Size::Grow, 16.0), false);
            text(ui, "usage", &catalog.offline_usage, 12, p.secondary, (Size::Fit, 16.0), true);
        });
        let meter = ui.node(
            "offline-meter",
            Node {
                axis: Axis::Row,
                w: Size::Grow,
                h: Size::Px(4.0),
                margin: Edges { t: 10.0, l: 8.0, r: 8.0, ..Edges::ZERO },
                look: Look { fill: p.line, ..Look::NONE },
                ..Node::DEFAULT
            },
        );
        ui.within(&meter, &mut |ui| {
            container(ui, "fill", Axis::Column, Size::Pct(catalog.offline_fraction), Size::Grow, p.ink);
        });
        let shortcuts = ui.node(
            "shortcuts",
            Node {
                flags: CLICKABLE | TAB_STOP,
                role: Role::Button,
                axis: Axis::Row,
                w: Size::Grow,
                h: Size::Px(28.0),
                padding: Edges { t: 10.0, l: 8.0, ..Edges::ZERO },
                ..Node::DEFAULT
            },
        );
        ui.a11y_name(&shortcuts, "Keyboard shortcuts");
        ui.within(&shortcuts, &mut |ui| {
            text(ui, "key", "?", 11, p.secondary, (Size::Px(14.0), 16.0), true);
            text(ui, "label", "shortcuts", 12, p.secondary, (Size::Fit, 16.0), false);
        });
        if ui.signal_state(&shortcuts).clicked {
            *intent = Some(Intent::ShowShortcuts);
        }
    });
}
