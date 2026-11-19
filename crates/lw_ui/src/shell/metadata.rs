//! Song metadata editor sheet and the song delete confirmation (library entry only).
use super::*;
use crate::catalog::Catalog;
use crate::{Intent, MetadataEdit};

/// The edit metadata sheet: `localwave-sheet`'s form, its fields 4 apart.
const EDIT_METADATA: Dialog = Dialog::DEFAULT.variant(variant::FORM);

/// A metadata text input: Done ends editing.
const META_FIELD: pxui::TextField = pxui::TextField::DEFAULT.action(pxui::ReturnKey::Done);

/// Label, field key, input. Swift edits disc numbers too; the sheet keeps them untouched.
const FIELDS: [(&str, &str, pxui::TextField); 6] = [
    ("Title", "meta-title", META_FIELD.name("Title")),
    ("Artist", "meta-artist", META_FIELD.name("Artist")),
    ("Album", "meta-album", META_FIELD.name("Album")),
    ("Album artist", "meta-album-artist", META_FIELD.name("Album artist")),
    ("Release year", "meta-year", META_FIELD.name("Release year")),
    ("Track number", "meta-track", META_FIELD.name("Track number")),
];

pub(super) fn editor(
    ui: &mut Ui,
    state: &ShellState,
    mut edit: MetadataEdit,
    intent: &mut Option<Intent>,
) {
    let pxui::ViewSize { width, .. } = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let valid = edit.valid() && !state.metadata_too_long;
    let buttons = [DialogButton::new("Cancel").cancel(), DialogButton::new("Save").primary().disabled(!valid)];
    let sheet = ui.dialog("edit-metadata", "Edit Metadata", &buttons, &EDIT_METADATA.w_px(width), &mut |ui| {
        for (index, (label, key, input)) in FIELDS.into_iter().enumerate() {
            ui.label(label, TEXT_12.grow().h_px(16.0).color(SECONDARY));
            let field = ui.node(key, Node::DEFAULT.w(Size::Grow).h(Size::Px(32.0)));
            ui.within(&field, &mut |ui| {
                let current = *edit.field_mut(index).expect("field index is in range");
                if let Some(value) = ui.text_field(key, current.as_str(), &input).changed {
                    *intent = Some(Query::new(value).map_or(Intent::MetadataTooLong, |q| Intent::MetadataField(index, q)));
                }
            });
        }
        ui.label(if state.metadata_too_long { "A field exceeds 1,024 UTF-8 bytes. Shorten it to save." } else if edit.numbers().is_none() { "Year and track must be whole numbers from 0 to 9999." } else if edit.title.as_str().trim().is_empty() { "Title is required." } else { "" }, TEXT_12.grow().h_px(20.0).color(SECONDARY));
    });
    match sheet.chosen {
        Some(1) => *intent = Some(Intent::SaveMetadata(edit)),
        Some(_) => *intent = Some(Intent::CloseMetadata),
        None => {}
    }
}

pub(super) fn confirm_delete(
    ui: &mut Ui,
    catalog: &Catalog,
    song: i64,
    intent: &mut Option<Intent>,
) {
    let pxui::ViewSize { width, .. } = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let title = catalog
        .row_index(song)
        .map(|index| catalog.row(index).title);
    let buttons = [DialogButton::new("Cancel").cancel(), DialogButton::new("Delete Song").danger().disabled(!title.is_some())];
    let sheet = ui.dialog("delete-song", "Delete song?", &buttons, &Dialog::DEFAULT.w_px(width), &mut |ui| {
        settings::paragraph(ui, "song-name", title.unwrap_or("This song changed. Try again."), width - 40.0, DIALOG_NAME);
        let consequences = "It leaves your library and lists. The audio file is not deleted.";
        settings::paragraph(ui, "consequences", consequences, width - 40.0, DIALOG_NOTE);
    });
    match sheet.chosen {
        Some(1) => *intent = Some(Intent::DeleteSong(song)),
        Some(_) => *intent = Some(Intent::CloseDeleteSong),
        None => {}
    }
}
