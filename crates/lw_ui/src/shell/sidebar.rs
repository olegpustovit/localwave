use super::*;
use crate::Intent;
use crate::catalog::{Catalog, Route};

pub(super) fn side_row(
    ui: &mut Ui,
    key: &str,
    label: (&str, &str),
    route: Route,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let (title, count) = label;
    let node = ui.node(
        key,
        Node {
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
        },
    );
    let signal = ui.signal_state(&node);
    ui.set_look(
        &node,
        Look {
            fill: if !state.settings_visible && state.route == route {
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
            if !state.settings_visible && state.route == route {
                p.ink
            } else {
                p.secondary
            },
            (Size::Grow, 20.0),
            false,
        );
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
    if signal.clicked {
        *intent = Some(Intent::Navigate(route));
    }
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
        Node {
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges::all(10.0),
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        spacer(ui, "sidebar-top", 28.0);
        text(
            ui,
            "brand",
            "LOCALWAVE",
            12,
            p.secondary,
            (Size::Grow, 34.0),
            true,
        );
        spacer(ui, "sidebar-brand-gap", 12.0);
        text(
            ui,
            "library-heading",
            "Library",
            12,
            p.secondary,
            (Size::Grow, 30.0),
            false,
        );
        side_row(
            ui,
            "albums",
            ("Albums", &catalog.album_count),
            Route::Albums,
            state,
            p,
            intent,
        );
        side_row(
            ui,
            "artists",
            ("Artists", &catalog.artist_count),
            Route::Artists,
            state,
            p,
            intent,
        );
        side_row(
            ui,
            "songs",
            ("Songs", &catalog.song_count),
            Route::Songs,
            state,
            p,
            intent,
        );
        side_row(ui, "find", ("Find", ""), Route::Find, state, p, intent);
        side_row(ui, "files", ("Files", ""), Route::Files, state, p, intent);
        spacer(ui, "lists-gap", 14.0);
        text(
            ui,
            "lists-heading",
            "Lists",
            12,
            p.secondary,
            (Size::Grow, 30.0),
            false,
        );
        ui.grid_list(
            "collections",
            catalog.collections.len(),
            (width - 20.0).max(1.0),
            32.0,
            &mut |ui, index| {
                let collection = &catalog.collections[index];
                side_row(
                    ui,
                    "collection",
                    (&collection.title, &collection.count),
                    Route::Collection(index),
                    state,
                    p,
                    intent,
                );
            },
        );
        text(
            ui,
            "storage",
            "Local music, your files",
            12,
            p.secondary,
            (Size::Grow, 26.0),
            false,
        );
        let button = ui.node(
            "settings",
            Node {
                flags: CLICKABLE | FOCUSABLE,
                role: Role::Button,
                w: Size::Grow,
                h: Size::Px(36.0),
                look: Look {
                    fill: if state.settings_visible {
                        p.selected
                    } else {
                        Rgba::default()
                    },
                    text: p.ink,
                    font: Font {
                        size: 15,
                        ..Font::BODY
                    },
                    ..Look::NONE
                },
                ..Node::DEFAULT
            },
        );
        ui.text(&button, "Settings");
        ui.a11y_name(&button, "Settings");
        if ui.signal_state(&button).clicked {
            *intent = Some(Intent::ShowSettings);
        }
    });
}
