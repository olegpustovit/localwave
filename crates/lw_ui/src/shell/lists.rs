//! List management dialogs: name (new/rename), the sidebar list menu and the delete confirmation.
use super::*;
use crate::browse::Query;
use crate::catalog::Catalog;
use crate::{Intent, ListDialog};

/// The delete list sheet: what does not change between frames.
const DELETE_LIST: Overlay = Overlay::DEFAULT.name("Delete list?");

/// The list menu sheet: what does not change between frames.
const LIST_MENU: Overlay = Overlay::DEFAULT.w(200.0).h(72.0).stroke(Rgba::token(LINE)).name("List actions");

/// The playlist name input of the list dialog: Done ends editing.
const LIST_NAME: pxui::TextField = pxui::TextField::DEFAULT
    .action(pxui::ReturnKey::Done)
    .name("List name");

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
    let open = ui.popup("list-menu", &LIST_MENU.at(at.0, at.1), |ui| {
            for (label, action, color) in [
                ("Rename…", Intent::RenameListDialog(list, title), ui.color(INK)),
                ("Delete list", Intent::ConfirmDeleteList(list), ui.color(DANGER)),
            ] {
                let item = ui.node(
                    label.into(),
                    Node {
                        state: State {
                            disabled: !present,
                            ..State::default()
                        },
                        look: Look {
                            text: color,
                            font: Font {
                                size: 13,
                                ..Font::BODY
                            },
                            ..Look::NONE
                        },
                        ..Node::column()
                            .w(Size::Grow)
                            .h_px(36.0)
                            .pad_x(12.0)
                            .role(Role::MenuItem)
                            .flags(if present { Flags::CLICKABLE | Flags::TAB_STOP } else { Flags::NONE })
                    },
                );
                ui.text(&item, label);
                if present && ui.signal(&item).clicked {
                    *intent = Some(action);
                    accepted = true;
                }
            }
        });
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
    intent: &mut Option<Intent>,
) {
    let (width, _) = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let title = playlist_title(catalog, list);
    let open = ui.modal("confirm-delete-list", &DELETE_LIST.w(width), |ui| {
        ui.column(
            "delete-list-body",
            Node {
                look: Look {
                    fill: ui.color(PAPER),
                    ..Look::NONE
                },
                ..Node::column().w(Size::Grow).h(Size::Fit).gap(12.0).pad(20.0)
            },
            |ui| {
                ui.label("Delete list?", TEXT_24.grow().h_px(36.0).color(INK));
                settings::paragraph(ui, "list-name", title.unwrap_or("This list changed. Try again."), width - 40.0, 15.0, ui.color(INK));
                settings::paragraph(ui, "consequences", "The songs stay in your library and folders.", width - 40.0, 14.0, ui.color(SECONDARY));
                let buttons = container(
                    ui,
                    "delete-list-actions",
                    Axis::Row,
                    Size::Grow,
                    Size::Px(40.0),
                    ui.color(PAPER),
                );
                ui.within(&buttons, &mut |ui| {
                    if files::action(ui, "cancel-delete-list", "Cancel", 100.0, true, (PANEL, INK)) {
                        *intent = Some(Intent::CloseDeleteList);
                    }
                    if files::action(
                        ui,
                        "delete-list-confirm",
                        "Delete List",
                        100.0,
                        title.is_some(),
                        (PANEL, DANGER),
                    ) || (title.is_some() && ui.key_pressed(input::Key::Enter.code(), None))
                    {
                        *intent = Some(Intent::DeleteList(list));
                    }
                });
            },
        );
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
    intent: &mut Option<Intent>,
) {
    let (width, _) = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let (heading, confirm) = match dialog {
        ListDialog::New(_) | ListDialog::NewPage => ("New list", "Create"),
        ListDialog::Rename(_) => ("Rename list", "Save"),
    };
    let open = ui.modal("list-name", &Overlay::DEFAULT.w(width).name(heading), |ui| {
        ui.column(
            "list-name-body",
            Node {
                look: Look {
                    fill: ui.color(PAPER),
                    ..Look::NONE
                },
                ..Node::column().w(Size::Grow).h(Size::Fit).gap(12.0).pad(20.0)
            },
            |ui| {
                ui.label(heading, TEXT_24.grow().h_px(36.0).color(INK));
                let field = ui.box_(
                    "list-name-field",
                    BoxOpts::default().w(Size::Grow).h(Size::Px(40.0)),
                );
                ui.within(&field, &mut |ui| {
                    if let Some(value) = ui
                        .text_field("list-name", state.playlist_name.as_str(), &LIST_NAME)
                        .changed
                    {
                        *intent = Some(
                            Query::new(value)
                                .map_or(Intent::PlaylistNameTooLong, Intent::EditPlaylistName),
                        );
                    }
                });
                ui.label(if state.playlist_name_too_long { "Name exceeds 1,024 UTF-8 bytes. Shorten it to save." } else { "" }, TEXT_12.grow().h_px(20.0).color(SECONDARY));
                let valid =
                    !state.playlist_name_too_long && !state.playlist_name.as_str().trim().is_empty();
                let buttons = container(
                    ui,
                    "list-name-actions",
                    Axis::Row,
                    Size::Grow,
                    Size::Px(40.0),
                    ui.color(PAPER),
                );
                ui.within(&buttons, &mut |ui| {
                    if files::action(ui, "cancel-list-name", "Cancel", 100.0, true, (PANEL, INK)) {
                        *intent = Some(Intent::CancelPlaylistName);
                    }
                    if files::action(ui, "confirm-list-name", confirm, 100.0, valid, (PANEL, INK))
                        || (valid && ui.key_pressed(input::Key::Enter.code(), None))
                    {
                        let name = state.playlist_name;
                        *intent = Some(match dialog {
                            ListDialog::New(song) => Intent::CreateList { name, song },
                            ListDialog::NewPage => Intent::CreatePageList(name),
                            ListDialog::Rename(list) => Intent::RenameList { list, name },
                        });
                    }
                });
            },
        );
    });
    if !open && intent.is_none() {
        *intent = Some(Intent::CancelPlaylistName);
    }
}
