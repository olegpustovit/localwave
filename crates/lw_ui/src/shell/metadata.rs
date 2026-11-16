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
        let body = ui.node(
            "metadata-body",
            Node {
                w: Size::Grow,
                h: Size::Fit,
                padding: Edges::all(20.0),
                gap: 4.0,
                look: Look {
                    fill: ui.color(PAPER),
                    ..Look::NONE
                },
                ..Node::DEFAULT
            },
        );
        ui.within(&body, &mut |ui| {
            text(
                ui,
                "title",
                "Edit Metadata",
                24,
                ui.color(INK),
                (Size::Grow, 36.0),
                false,
            );
            for (index, (label, key)) in FIELDS.into_iter().enumerate() {
                text(ui, label, label, 12, ui.color(SECONDARY), (Size::Grow, 16.0), false);
                let field = ui.box_(key, BoxOpts::default().w(Size::Grow).h(Size::Px(32.0)));
                ui.within(&field, &mut |ui| {
                    let current = *edit.field_mut(index).expect("field index is in range");
                    if let Some(value) = ui.text_field_with_options(
                        label,
                        current.as_str(),
                        pxui::TextInputOptions {
                            action: pxui::TextInputOptions::DONE,
                            ..Default::default()
                        },
                    ) {
                        *intent =
                            Some(Query::new(value).map_or(Intent::MetadataTooLong, |q| {
                                Intent::MetadataField(index, q)
                            }));
                    }
                });
            }
            let valid = edit.valid() && !state.metadata_too_long;
            text(
                ui,
                "meta-error",
                if state.metadata_too_long {
                    "A field exceeds 1,024 UTF-8 bytes. Shorten it to save."
                } else if edit.numbers().is_none() {
                    "Year and track must be whole numbers from 0 to 9999."
                } else if edit.title.as_str().trim().is_empty() {
                    "Title is required."
                } else {
                    ""
                },
                12,
                ui.color(SECONDARY),
                (Size::Grow, 20.0),
                false,
            );
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
        });
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
        let body = ui.node(
            "delete-song-body",
            Node {
                w: Size::Grow,
                h: Size::Fit,
                padding: Edges::all(20.0),
                gap: 12.0,
                look: Look {
                    fill: ui.color(PAPER),
                    ..Look::NONE
                },
                ..Node::DEFAULT
            },
        );
        ui.within(&body, &mut |ui| {
            text(
                ui,
                "title",
                "Delete song?",
                24,
                ui.color(INK),
                (Size::Grow, 36.0),
                false,
            );
            settings::paragraph(
                ui,
                "song-name",
                title.unwrap_or("This song changed. Try again."),
                width - 40.0,
                15.0,
                ui.color(INK),
            );
            settings::paragraph(
                ui,
                "consequences",
                "It leaves your library and lists. The audio file is not deleted.",
                width - 40.0,
                14.0,
                ui.color(SECONDARY),
            );
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
        });
    });
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseDeleteSong);
    }
}
