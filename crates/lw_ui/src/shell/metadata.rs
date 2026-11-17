//! Song metadata editor sheet and the song delete confirmation (library entry only).
use super::*;
use crate::catalog::Catalog;
use crate::{Intent, MetadataEdit};

/// Label, field key. Swift edits disc numbers too; the sheet keeps them untouched.
const FIELDS: [(&str, &str); 6] = [
    ("Title", "meta-title"),
    ("Artist", "meta-artist"),
    ("Album", "meta-album"),
    ("Album artist", "meta-album-artist"),
    ("Release year", "meta-year"),
    ("Track number", "meta-track"),
];

pub(super) fn editor(
    ui: &mut Ui,
    state: &ShellState,
    mut edit: MetadataEdit,
    intent: &mut Option<Intent>,
) {
    let (width, _) = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let open = ui.modal(Some(width), "Edit Metadata", &mut |ui| {
        ui.column(
            "metadata-body",
            Node {
                look: Look {
                    fill: ui.color(PAPER),
                    ..Look::NONE
                },
                ..Node::column().w(Size::Grow).h(Size::Fit).gap(4.0).pad(20.0)
            },
            |ui| {
                ui.label("Edit Metadata", TEXT_24.w(Size::Grow).h(36.0).color(INK));
                for (index, (label, key)) in FIELDS.into_iter().enumerate() {
                    ui.label(label, TEXT_12.w(Size::Grow).h(16.0).color(SECONDARY));
                    let field = ui.box_(key, BoxOpts::default().w(Size::Grow).h(Size::Px(32.0)));
                    ui.within(&field, &mut |ui| {
                        let current = *edit.field_mut(index).expect("field index is in range");
                        if let Some(value) = ui.text_field_with_options(
                            label,
                            current.as_str(),
                            pxui::TextInputOptions::DEFAULT.action(pxui::TextInputOptions::DONE),
                        ) {
                            *intent =
                                Some(Query::new(value).map_or(Intent::MetadataTooLong, |q| {
                                    Intent::MetadataField(index, q)
                                }));
                        }
                    });
                }
                let valid = edit.valid() && !state.metadata_too_long;
                ui.label(if state.metadata_too_long { "A field exceeds 1,024 UTF-8 bytes. Shorten it to save." } else if edit.numbers().is_none() { "Year and track must be whole numbers from 0 to 9999." } else if edit.title.as_str().trim().is_empty() { "Title is required." } else { "" }, TEXT_12.w(Size::Grow).h(20.0).color(SECONDARY));
                let buttons = container(
                    ui,
                    "metadata-actions",
                    Axis::Row,
                    Size::Grow,
                    Size::Px(40.0),
                    ui.color(PAPER),
                );
                ui.within(&buttons, &mut |ui| {
                    if files::action(ui, "cancel-metadata", "Cancel", 100.0, true, (PANEL, INK)) {
                        *intent = Some(Intent::CloseMetadata);
                    }
                    if files::action(ui, "save-metadata", "Save", 100.0, valid, (PANEL, INK)) {
                        *intent = Some(Intent::SaveMetadata(edit));
                    }
                });
            },
        );
    });
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseMetadata);
    }
}

pub(super) fn confirm_delete(
    ui: &mut Ui,
    catalog: &Catalog,
    song: i64,
    intent: &mut Option<Intent>,
) {
    let (width, _) = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let title = catalog
        .row_index(song)
        .map(|index| catalog.row(index).title);
    let open = ui.modal(Some(width), "Delete song?", &mut |ui| {
        ui.column(
            "delete-song-body",
            Node {
                look: Look {
                    fill: ui.color(PAPER),
                    ..Look::NONE
                },
                ..Node::column().w(Size::Grow).h(Size::Fit).gap(12.0).pad(20.0)
            },
            |ui| {
                ui.label("Delete song?", TEXT_24.w(Size::Grow).h(36.0).color(INK));
                settings::paragraph(ui, "song-name", title.unwrap_or("This song changed. Try again."), width - 40.0, 15.0, ui.color(INK));
                settings::paragraph(ui, "consequences", "It leaves your library and lists. The audio file is not deleted.", width - 40.0, 14.0, ui.color(SECONDARY));
                let buttons = container(
                    ui,
                    "delete-song-actions",
                    Axis::Row,
                    Size::Grow,
                    Size::Px(40.0),
                    ui.color(PAPER),
                );
                ui.within(&buttons, &mut |ui| {
                    if files::action(ui, "cancel-delete-song", "Cancel", 100.0, true, (PANEL, INK)) {
                        *intent = Some(Intent::CloseDeleteSong);
                    }
                    if files::action(
                        ui,
                        "delete-song-confirm",
                        "Delete Song",
                        100.0,
                        title.is_some(),
                        (PANEL, DANGER),
                    ) {
                        *intent = Some(Intent::DeleteSong(song));
                    }
                });
            },
        );
    });
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseDeleteSong);
    }
}
