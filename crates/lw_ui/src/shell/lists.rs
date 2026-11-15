//! List management dialogs: name (new/rename), the sidebar list menu and the delete confirmation.
use super::*;
use crate::browse::Query;
use crate::catalog::Catalog;
use crate::{Intent, ListDialog};

/// The playlist behind a collection, by id; `None` when it was removed meanwhile.
pub(super) fn playlist_title(catalog: &Catalog, list: i64) -> Option<&str> {
    catalog
        .collections
        .iter()
        .find_map(|collection| match &*collection.context {
            lw_core::context::PlaybackContext::Playlist { id, .. } if *id == list => {
                Some(collection.title.as_str())
            }
            _ => None,
        })
}

pub(super) fn menu(
    ui: &mut Ui,
    catalog: &Catalog,
    list: i64,
    at: (f32, f32),
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let (w, h) = ui.view_size();
    let at = (
        at.0.clamp(0.0, (w - 200.0).max(0.0)),
        at.1.clamp(0.0, (h - 72.0).max(0.0)),
    );
    let (present, title) = playlist_title(catalog, list).map_or((false, Query::default()), |t| {
        (true, Query::new(t).unwrap_or_default())
    });
    let mut accepted = false;
    let open = ui.popup(
        Some(p.line),
        None,
        false,
        false,
        at,
        200.0,
        72.0,
        "List actions",
        &mut |ui| {
            for (label, action, color) in [
                ("Rename…", Intent::RenameListDialog(list, title), p.ink),
                ("Delete list", Intent::ConfirmDeleteList(list), p.danger),
            ] {
                let item = ui.node(
                    label,
                    Node {
                        flags: if present { CLICKABLE | TAB_STOP } else { 0 },
                        role: Role::MenuItem,
                        state: State {
                            disabled: !present,
                            ..State::default()
                        },
                        w: Size::Grow,
                        h: Size::Px(36.0),
                        padding: Edges {
                            l: 12.0,
                            r: 12.0,
                            ..Edges::ZERO
                        },
                        look: Look {
                            text: color,
                            font: Font {
                                size: 13,
                                ..Font::BODY
                            },
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.text(&item, label);
                if present && ui.signal_state(&item).clicked {
                    *intent = Some(action);
                    accepted = true;
                }
            }
        },
    );
    if accepted {
        ui.dismiss_popup();
    }
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseListMenu);
    }
}

pub(super) fn confirm_delete(
    ui: &mut Ui,
    catalog: &Catalog,
    list: i64,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let (width, _) = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let title = playlist_title(catalog, list);
    let open = ui.modal(Some(width), "Delete list?", &mut |ui| {
        let body = ui.node(
            "delete-list-body",
            Node {
                w: Size::Grow,
                h: Size::Fit,
                padding: Edges::all(20.0),
                gap: 12.0,
                look: Look {
                    fill: p.paper,
                    ..Look::NONE
                },
                ..Node::DEFAULT
            },
        );
        ui.within(&body, &mut |ui| {
            text(
                ui,
                "title",
                "Delete list?",
                24,
                p.ink,
                (Size::Grow, 36.0),
                false,
            );
            settings::paragraph(
                ui,
                "list-name",
                title.unwrap_or("This list changed. Try again."),
                width - 40.0,
                15.0,
                p.ink,
            );
            settings::paragraph(
                ui,
                "consequences",
                "The songs stay in your library and folders.",
                width - 40.0,
                14.0,
                p.secondary,
            );
            let buttons = container(
                ui,
                "delete-list-actions",
                Axis::Row,
                Size::Grow,
                Size::Px(40.0),
                p.paper,
            );
            ui.within(&buttons, &mut |ui| {
                if files::action(ui, "cancel-delete-list", "Cancel", 100.0, true, p) {
                    *intent = Some(Intent::CloseDeleteList);
                }
                if files::action(
                    ui,
                    "delete-list-confirm",
                    "Delete List",
                    100.0,
                    title.is_some(),
                    Palette { ink: p.danger, ..p },
                ) {
                    *intent = Some(Intent::DeleteList(list));
                }
            });
        });
    });
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseDeleteList);
    }
}

/// New list / rename. The name text is shared with the queue sheet's save dialog.
pub(super) fn name_dialog(
    ui: &mut Ui,
    state: &ShellState,
    dialog: ListDialog,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let (width, _) = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let (heading, confirm) = match dialog {
        ListDialog::New(_) => ("New list", "Create"),
        ListDialog::Rename(_) => ("Rename list", "Save"),
    };
    let open = ui.modal(Some(width), heading, &mut |ui| {
        let body = ui.node(
            "list-name-body",
            Node {
                w: Size::Grow,
                h: Size::Fit,
                padding: Edges::all(20.0),
                gap: 12.0,
                look: Look {
                    fill: p.paper,
                    ..Look::NONE
                },
                ..Node::DEFAULT
            },
        );
        ui.within(&body, &mut |ui| {
            text(ui, "title", heading, 24, p.ink, (Size::Grow, 36.0), false);
            let field = ui.box_(
                "list-name-field",
                BoxOpts::default().w(Size::Grow).h(Size::Px(40.0)),
            );
            ui.within(&field, &mut |ui| {
                if let Some(value) = ui.text_field_with_options(
                    "List name",
                    state.playlist_name.as_str(),
                    pxui::TextInputOptions {
                        action: pxui::TextInputOptions::DONE,
                        ..Default::default()
                    },
                ) {
                    *intent = Some(
                        Query::new(value)
                            .map_or(Intent::PlaylistNameTooLong, Intent::EditPlaylistName),
                    );
                }
            });
            text(
                ui,
                "list-name-error",
                if state.playlist_name_too_long {
                    "Name exceeds 1,024 UTF-8 bytes. Shorten it to save."
                } else {
                    ""
                },
                12,
                p.secondary,
                (Size::Grow, 20.0),
                false,
            );
            let valid =
                !state.playlist_name_too_long && !state.playlist_name.as_str().trim().is_empty();
            let buttons = container(
                ui,
                "list-name-actions",
                Axis::Row,
                Size::Grow,
                Size::Px(40.0),
                p.paper,
            );
            ui.within(&buttons, &mut |ui| {
                if files::action(ui, "cancel-list-name", "Cancel", 100.0, true, p) {
                    *intent = Some(Intent::CancelPlaylistName);
                }
                if files::action(ui, "confirm-list-name", confirm, 100.0, valid, p) {
                    let name = state.playlist_name;
                    *intent = Some(match dialog {
                        ListDialog::New(song) => Intent::CreateList { name, song },
                        ListDialog::Rename(list) => Intent::RenameList { list, name },
                    });
                }
            });
        });
    });
    if !open && intent.is_none() {
        *intent = Some(Intent::CancelPlaylistName);
    }
}
