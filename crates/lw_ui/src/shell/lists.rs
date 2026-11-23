//! List management dialogs: name (new/rename), the sidebar list menu and the delete confirmation.
use super::*;
use crate::browse::Query;
use crate::catalog::Catalog;
use crate::{Intent, ListDialog};

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
    let title = playlist_title(catalog, list);
    let present = title.is_some();
    let rows = [
        MenuItem::new("Rename…").id(RENAME).disabled(!present),
        MenuItem::new("Delete list")
            .id(DELETE)
            .disabled(!present)
            .danger(),
    ];
    let menu = ui.menu(
        "list-menu",
        pxui::Point::new(at.0, at.1),
        &rows,
        &songs::MENU.name("List actions"),
    );
    match menu.chosen_id {
        Some(RENAME) => {
            *intent = Some(Intent::RenameListDialog(
                list,
                title.and_then(Query::new).unwrap_or_default(),
            ))
        }
        Some(_) => *intent = Some(Intent::ConfirmDeleteList(list)),
        None => {}
    }
    if menu.dismissed && intent.is_none() {
        *intent = Some(Intent::CloseListMenu);
    }
}
// The list menu's row ids.
const RENAME: i64 = 0;
const DELETE: i64 = 1;

pub(super) fn confirm_delete(
    ui: &mut Ui,
    catalog: &Catalog,
    list: i64,
    intent: &mut Option<Intent>,
) {
    let pxui::ViewSize { width, .. } = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let title = playlist_title(catalog, list);
    let buttons = [
        DialogButton::new("Cancel").cancel(),
        DialogButton::new("Delete List")
            .danger()
            .primary()
            .disabled(!title.is_some()),
    ];
    let sheet = ui.dialog(
        "delete-list",
        "Delete list?",
        &buttons,
        &super::files::SHEET_DIALOG.w_px(width),
        &mut |ui| {
            settings::paragraph(
                ui,
                "list-name",
                title.unwrap_or("This list changed. Try again."),
                width - 40.0,
                DIALOG_NAME,
            );
            settings::paragraph(
                ui,
                "consequences",
                "The songs stay in your library and folders.",
                width - 40.0,
                DIALOG_NOTE,
            );
        },
    );
    match sheet.chosen {
        Some(1) => *intent = Some(Intent::DeleteList(list)),
        Some(_) => *intent = Some(Intent::CloseDeleteList),
        None => {}
    }
}

/// New list / rename. The name text is shared with the queue sheet's save dialog.
pub(super) fn name_dialog(
    ui: &mut Ui,
    state: &ShellState,
    dialog: ListDialog,
    intent: &mut Option<Intent>,
) {
    let pxui::ViewSize { width, .. } = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let (heading, confirm) = match dialog {
        ListDialog::New(_) | ListDialog::NewPage => ("New list", "Create"),
        ListDialog::Rename(_) => ("Rename list", "Save"),
    };
    let valid = !state.playlist_name_too_long && !state.playlist_name.as_str().trim().is_empty();
    let buttons = [
        DialogButton::new("Cancel").cancel(),
        DialogButton::new(confirm).primary().disabled(!valid),
    ];
    let sheet = ui.dialog(
        "list-name",
        heading,
        &buttons,
        &super::files::SHEET_DIALOG.w_px(width),
        &mut |ui| {
            let field = ui.node(
                "list-name-field",
                Node::DEFAULT.w(Size::Grow).h(Size::Px(40.0)),
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
            ui.label(
                if state.playlist_name_too_long {
                    "Name exceeds 1,024 UTF-8 bytes. Shorten it to save."
                } else {
                    ""
                },
                TEXT_12.grow().h_px(20.0).color(SECONDARY),
            );
        },
    );
    match sheet.chosen {
        Some(1) => {
            let name = state.playlist_name;
            *intent = Some(match dialog {
                ListDialog::New(song) => Intent::CreateList { name, song },
                ListDialog::NewPage => Intent::CreatePageList(name),
                ListDialog::Rename(list) => Intent::RenameList { list, name },
            });
        }
        Some(_) => *intent = Some(Intent::CancelPlaylistName),
        None => {}
    }
}
