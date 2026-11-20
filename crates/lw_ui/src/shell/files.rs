use pxui::bodies::BodyConfig;
use super::*;
use crate::FilesFilter;
use crate::files::{Folder, Location, Request, Target};

/// The Add music sheet (`localwave-sheet` / `CHOICES`).
const ADD_MUSIC: Dialog = Dialog::DEFAULT.variant(variant::CHOICES);
/// A sheet's paper card: 20-point padding, 12 between its parts.
pub(super) const SHEET: Node = Node::column().w(Size::Grow).h(Size::Fit).gap(12.0).pad(20.0).look(PAPER_FILL);
/// A folder page's Back and summary bar, 16 points under the header; it wraps on a phone.
const BACK_BAR: Node = Node::row().w(Size::Grow).h(Size::Fit).align(Align::Center).padding(Edges::ZERO.t(16.0));
/// A folder page's tool keys, 15 points above the rows; they wrap on a phone.
const FOLDER_TOOLS: Node = Node::row().w(Size::Grow).h(Size::Fit).gap(8.0).align(Align::Center).padding(Edges::ZERO.b(15.0));

pub(super) fn popup(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    target: Target,
    at: (f32, f32),
    intent: &mut Option<Intent>,
) {
    let busy = player.files.is_none_or(|files| files.busy);
    let present = player.files.is_some_and(|files| {
        files.account == Some(target.user)
            && files.sources.iter().any(|folder| folder.id == target.source && folder.root == target.root)
    });
    let rows = [
        MenuItem::new("Open folder").id(OPEN).disabled(!present),
        MenuItem::new("Rescan now").id(RESCAN).disabled(!(present && !busy)),
        MenuItem::new("Reconnect folder…").id(RECONNECT).disabled(!(present && !busy)),
        MenuItem::new("Remove source").id(REMOVE).disabled(!(present && !busy)).danger(),
    ];
    let menu = ui.menu("source-menu", pxui::Point::new(at.0, at.1), &rows, &OPTIONS);
    let location = Location { user: target.user, source: target.source, parent: None };
    match menu.chosen_id {
        Some(OPEN) => *intent = Some(Intent::Files(Request::Browse(location))),
        Some(RESCAN) => *intent = Some(Intent::Files(Request::Rescan(location))),
        Some(RECONNECT) => *intent = Some(Intent::ReconnectSource(target)),
        Some(_) => *intent = Some(Intent::ConfirmRemoveSource(target)),
        None => {}
    }
    if menu.dismissed && intent.is_none() {
        *intent = Some(Intent::CloseSourceMenu);
    }
}

/// The folder options menu: rows on the paper fill (`MENU_PANEL`), focused by a press.
const OPTIONS: Menu<'static> = super::songs::MENU.w_px(220.0).name("Folder options").variant(MENU_PANEL).focus_on_click(true);
// The folder options' and folder actions' row ids.
const OPEN: i64 = 0;
const RESCAN: i64 = 1;
const RECONNECT: i64 = 2;
const REMOVE: i64 = 3;
const PLAY: i64 = 4;
const SHUFFLE: i64 = 5;

pub(super) fn confirm_remove(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    target: Target,
    intent: &mut Option<Intent>,
) {
    let width = sheet_width(ui);
    let folder = player.files.and_then(|files| {
        files.sources.iter().find(|folder| {
            files.account == Some(target.user)
                && folder.id == target.source
                && folder.root == target.root
        })
    });
    let enabled = folder.is_some() && player.files.is_some_and(|files| !files.busy);
    let buttons = [DialogButton::new("Cancel").cancel(), DialogButton::new("Remove").danger().primary().disabled(!enabled)];
    let sheet = ui.dialog("remove-source", "Remove folder?", &buttons, &SHEET_DIALOG.w_px(width), &mut |ui| {
        let name = folder.map_or("This folder changed. Reload folders and try again.", |folder| folder.name.as_str());
        settings::paragraph(ui, "folder-name", name, width - 40.0, DIALOG_NAME);
        let consequences = "Its songs leave the library. Files are not deleted.";
        settings::paragraph(ui, "consequences", consequences, width - 40.0, DIALOG_NOTE);
    });
    // Swift's default action: Return confirms.
    match sheet.chosen {
        Some(1) => *intent = Some(Intent::Files(Request::Remove(target))),
        Some(_) => *intent = Some(Intent::CloseRemoveSource),
        None => {}
    }
}

/// Back (button and ⌘[): the parent folder, or the source list from a source root.
pub(super) fn up(directory: &crate::files::Directory) -> Intent {
    let location = directory.location;
    Intent::Files(directory.up.map_or(Request::Roots, |parent| {
        Request::Browse(Location { parent: Some(parent), ..location })
    }))
}

/// A plate key (`localwave-button` / `KEY`, its 13-point label centred): ink on the panel fill, secondary while disabled; the
/// caller sizes it (`plate_h`: 44 points on a phone, 36 on the desktop).
const PLATE: pxui::Button = pxui::Button::DEFAULT.variant(variant::KEY).centered().label(TEXT_13).look(&PLATE_LOOKS);
const PLATE_LOOKS: States = PLAY_KEY_LOOKS.disabled(PANEL_FILL.text(Rgba::token(SECONDARY)));
/// `PLATE` in paper on ink (Reconnect).
const PLATE_INK: pxui::Button = PLATE.look(&PLATE_INK_LOOKS);
const PLATE_INK_LOOKS: States = PLAY_PRIMARY_LOOKS.disabled(INK_FILL.text(Rgba::token(SECONDARY)));
fn plate_h(ui: &Ui) -> f32 {
    if ui.view_size().width < compact::BREAKPOINT { 44.0 } else { 36.0 }
}
/// LocalWave's confirmation sheet (registered as `localwave-sheet`): on paper, 20 in, the 24-point title, the caller's content, then
/// the kit's buttons (`DialogConfig.built_in`, U2b.4.38), which the button body draws as the sheet's keys (`sheet_key`); `variant::FORM`
/// puts the rows 4 apart instead of 12. `variant::CHOICES` (Swift's Add music sheet): 16 in and 8 apart, the 20-point title, the
/// content, then the buttons as choices.
pub(crate) fn sheet_body(ui: &mut Ui, c: &pxui::bodies::DialogConfig) {
    let choices = c.variant == variant::CHOICES;
    ui.column(if choices { CHOICES_SHEET } else if c.variant == variant::FORM { SHEET.gap(4.0) } else { SHEET }, |ui| {
        ui.label(c.text_of(&c.title), if choices { SHEET_TITLE } else { TEXT_24.grow().h_px(36.0).color(INK) });
        ui.slot(c.content);
        if choices {
            spacer(ui, "gap", 4.0);
        }
        ui.slot(c.built_in);
    });
}
/// A sheet's button (`localwave-button` with its sheet's variant: `SHEET`, `FORM`, `CHOICES`; the kit builds it, its presses, focus and
/// state): the buttons' row 40 high (a `CHOICES` sheet's a column 8 apart); a key 100 points wide (`plate_h` high), its 13-point label
/// centred in ink, the danger red or secondary while disabled; a `CHOICES` button with a detail a full-width choice, its 17-point title
/// over its detail paragraph. Both on the panel fill.
pub(super) fn sheet_key(ui: &mut Ui, c: &pxui::bodies::ButtonConfig) {
    use pxui::bodies::BodyConfig;
    let state = State::from(c.state);
    let (label, detail) = (c.text_of(&c.label), c.text_of(&c.detail));
    let choice = c.variant == variant::CHOICES && !detail.is_empty();
    // the kit's row of the buttons is each button's line
    ui.set_node_layout(&c.line, &if c.variant == variant::CHOICES { Node::column().w(Size::Grow).h(Size::Fit).gap(8.0) } else { Node::row().w(Size::Grow).h_px(40.0) });
    ui.set_node_layout(&c.row, &if choice { CHOICE } else { Node::row().w_px(100.0).h_px(plate_h(ui)).center() });
    ui.style(&c.row, &PANEL_LOOKS, state);
    if choice {
        ui.label(label, OPTION_TITLE.role(Role::Unknown));
        settings::paragraph(ui, "detail", detail, sheet_width(ui) - 56.0, OPTION_DETAIL);
    } else {
        let ink = if state.disabled { SECONDARY } else if c.danger() { DANGER } else { INK };
        ui.label(label, TEXT_13.color(ink).role(Role::Unknown));
    }
}
/// A confirmation sheet (`localwave-sheet` / `SHEET`; its keys `sheet_key`).
pub(super) const SHEET_DIALOG: Dialog = Dialog::DEFAULT.variant(variant::SHEET);
/// The Add music sheet's card: paper, 16 in, 8 apart.
const CHOICES_SHEET: Node = Node::column().w(Size::Grow).h(Size::Fit).gap(8.0).pad(16.0).look(PAPER_FILL);
/// A sheet choice: the sheet's width, 12 by 10 in, its title 2 over its detail.
const CHOICE: Node = Node::column().w(Size::Grow).h(Size::Fit).gap(2.0).pad_x(12.0).pad_y(10.0);
/// A sheet's width: the view less 16 points a side, 240 to 420.
fn sheet_width(ui: &Ui) -> f32 {
    (ui.view_size().width - 32.0).clamp(240.0, 420.0)
}

/// The desktop folder page's Back (`localwave-button` / `ROW`): Swift's plain secondary "‹ Back", 64 by 40, named "Back".
const FILES_BACK: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Px(64.0), Size::Px(40.0)).align(Justify::Start).label(TEXT_14.color(SECONDARY).role(Role::Unknown)).name("Back");
/// The Folders page's "+" (Add music): the 24-point plus in ink.
const ADD_MUSIC_KEY: pxui::IconButton = ICON_44.look(&ICON_LOOKS);
/// A phone folder's 52-point keys (Shuffle, Keep offline): ink on the panel fill, paper on ink while checked (kept).
const KEY_52: pxui::IconButton = icon_key(52.0, 52.0).look(&KEY_52_LOOKS);
const KEY_52_LOOKS: States = PLAY_KEY_LOOKS.checked(INK_FILL.text(Rgba::token(PAPER)));
/// A folder page's meters (`localwave-progress` / `BAR`).
const BAR: pxui::Progress = pxui::Progress::DEFAULT.variant(variant::BAR);
/// A phone folder's All / Offline / In cloud filters (`localwave-tabs` / `MODE`, as the library modes).
const FILTER: pxui::Tabs = pxui::Tabs::DEFAULT.name("Availability filter").variant(variant::MODE);
/// "show" after the hidden files' count (`localwave-button` / `ROW`, Swift's word key): its secondary word, 44 high.
const SHOW: pxui::Button = super::compact::list::WORD.pad(Edges::ZERO).label(BODY_DIM).name("Show non-audio files");

/// Swift's folder/file marks in the # column, drawn as outlines.
fn mark(ui: &mut Ui, directory: bool, color: Rgba) {
    let node = ui.node("mark", Node::column().w_px(16.0).h_px(16.0));
    ui.canvas(&node, move |paint, rect| {
        let (x, y, w, h) = if directory { (rect.x + 1.0, rect.y + 3.0, 14.0, 10.0) } else { (rect.x + 3.0, rect.y + 1.0, 10.0, 14.0) };
        paint.fill(Rect::new(x, y, w, 1.5), Fill::new(color).fractional());
        paint.fill(Rect::new(x, y + h - 1.5, w, 1.5), Fill::new(color).fractional());
        paint.fill(Rect::new(x, y, 1.5, h), Fill::new(color).fractional());
        paint.fill(Rect::new(x + w - 1.5, y, 1.5, h), Fill::new(color).fractional());
    });
}

const ADD_STEPS: [&str; 3] = [
    "Choose a folder in Files",
    "LocalWave indexes its audio files",
    "Play your music from the library",
];

/// The desktop folder table's columns: "#" and Time point-wide, the others sharing the rest by weight.
const FOLDER_COLUMNS: [pxui::Column; 6] = [
    pxui::Column::new("#").w_px(56.0).grip(false),
    pxui::Column::new("Name").share(2.2),
    pxui::Column::new("Kind").share(1.5),
    pxui::Column::new("Size").share(1.0),
    pxui::Column::new("On device").share(1.2),
    pxui::Column::new("Time").w_px(84.0).grip(false),
];
/// The Time column's index: its heading and cells end 20 points before the column's end.
const TIME: usize = 5;
/// The desktop folder table: no frame, nothing selected, a press or Return opens a row (the arrows only move the keyboard's row);
/// 38-point rows (`localwave-row` / `FOLDER_ROW`) under 38-point headings (`localwave-heading` / `FOLDER_HEADING`).
const FOLDER_TABLE: pxui::Table = pxui::Table::DEFAULT.mode(pxui::SelectMode::None).open_on_click(true).bare().row_h(38.0).head(pxui::TableHead::DEFAULT.h_px(38.0).variant(variant::FOLDER_HEADING)).variant(variant::FOLDER_ROW);
/// A folder table heading (`localwave-heading` / `FOLDER_HEADING`): Swift's 12-point secondary title; Time's ends 20 points in.
pub(super) fn heading_body(ui: &mut Ui, c: &pxui::bodies::TableHeadConfig) {
    if c.index == TIME {
        let layout = ui.node_layout(&c.node).justify(Justify::End).padding(Edges::ZERO.l(8.0).r(20.0));
        ui.set_node_layout(&c.node, &layout);
    }
    ui.label(c.text_of(&c.title), FOOTNOTE.role(Role::Unknown));
}

pub(super) fn view(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    state: &ShellState,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let compact = ui.view_size().width < compact::BREAKPOINT;
    let directory = player.files.and_then(|files| files.directory.as_ref());
    // Swift's source list pads 16 points; its folder page 32.
    let inset = if compact || directory.is_none() { 16.0 } else { 32.0 };
    ui.column(Node::column().w(Size::Grow).h(Size::Grow).flags(if directory.is_none() { Flags::SCROLL | Flags::CLIP } else { Flags::NONE }).padding(Edges::new(inset, if directory.is_none() { if compact { 6.0 } else { 10.0 } } else { 0.0 }, inset, 12.0)),
        |ui| {
            let Some(files) = player.files else {
                ui.label("Loading folders…", TEXT_18.grow().h_px(40.0).color(SECONDARY));
                return;
            };
            let content = (width - inset * 2.0).max(1.0);
            match &files.directory {
                // the two pages share this column: each keeps its own identity
                Some(directory) if files.account.is_some() => ui.id("folder-page", |ui| folder_page(ui, files, directory, (state, player.settings.show_non_audio), compact, intent)),
                _ => ui.id("source-page", |ui| source_page(ui, files, content, compact, intent)),
            }
        },
    );
}

/// Worker progress and the last message, shared by both pages.
fn status(ui: &mut Ui, files: &crate::files::Snapshot, compact: bool, intent: &mut Option<Intent>) {
    if files.busy {
        let progress = container(ui, "files-progress", Axis::Row, Size::Grow, Size::Px(if compact { 44.0 } else { 40.0 }), &PANEL_FILL);
        ui.within(&progress, &mut |ui| {
            ui.label(&files.progress, MONO_13.grow().h_px(32.0).color(INK).role(Role::Progress));
            if ui.button("cancel", "Cancel", &PLATE.size(Size::Px(80.0), Size::Px(if compact { 44.0 } else { 36.0 }))).clicked {
                *intent = Some(Intent::Files(Request::Cancel));
            }
        });
    }
    if !files.message.is_empty() {
        ui.label(&files.message, PARAGRAPH_13.color(SECONDARY));
    }
}

fn source_page(ui: &mut Ui, files: &crate::files::Snapshot, width: f32, compact: bool, intent: &mut Option<Intent>) {
    let enabled = files.account.is_some() && !files.busy;
    ui.row(Node::row().w(Size::Grow).h(Size::Fit).margin(Edges::ZERO.b(16.0)),
        |ui| {
            ui.column(Node::column().w(Size::Grow).h(Size::Fit).gap(4.0), |ui| {
                ui.label("Folders", TEXT_32.grow().h_px(if compact { 44.0 } else { 33.6 }).color(INK));
                ui.label(&files.source_count, CAPTION.grow());
            });
            // Swift's "+" opens an Add music sheet; its two choices are the picker intents.
            if ui.icon_button("add-music", icon_drawing(Icon::Plus), "Add music", &ADD_MUSIC_KEY.disabled(!enabled)).clicked {
                *intent = Some(Intent::ShowAddMusic);
            }
        },
    );
    status(ui, files, compact, intent);
    let Some(user) = files.account else {
        if files.message.is_empty() {
            ui.label("Loading folders…", TEXT_14.grow().h_px(40.0).color(SECONDARY));
        }
        return;
    };
    if files.sources.is_empty() {
        ui.label("Point it at a folder.", TEXT_28.grow().h_px(48.0).color(INK));
        ui.label("Your files stay where they are. Nothing is uploaded.", PARAGRAPH_14.color(SECONDARY));
        for (index, step) in ADD_STEPS.iter().enumerate() {
            let row = ui.node(Key::Id(index as i64), Node::row().w(Size::Grow).h_px(44.0).gap(16.0).align(Align::Center));
            ui.within(&row, &mut |ui| {
                ui.label(&["1", "2", "3"][index], MONO_13.w_px(12.0).color(SECONDARY));
                ui.label(step, TEXT_15.grow().h_px(24.0).color(INK));
            });
            ui.canvas_over(&row, move |paint, rect| paint.fill(Rect::new(rect.x, rect.y + rect.h - 1.0, rect.w, 1.0), Fill::new(Rgba::token(LINE)).fractional()));
        }
    }
    for folder in files.sources.iter() {
        source_card(ui, user, folder, width, intent);
    }
    ui.column(Node::column().w(Size::Grow).h(Size::Fit).margin(Edges::ZERO.t(16.0)), |ui| {
        rule(ui, "offline-rule", false);
        spacer(ui, "offline-gap", 16.0);
        let label = container(ui, "offline-label", Axis::Row, Size::Grow, Size::Px(19.6), &PAPER_FILL);
        ui.within(&label, &mut |ui| {
            ui.label("Offline space", BODY_INK.grow());
            ui.label(&files.offline_usage, MONO_12.color(INK));
        });
        spacer(ui, "offline-bar-gap", 10.0);
        ui.progress("offline", "Offline space", Some(files.offline_fraction), &BAR);
        spacer(ui, "offline-note-gap", 8.0);
        ui.label("Cloud songs download when played and the next two download ahead. Pinned folders stay.", PARAGRAPH_13.color(SECONDARY));
    });
}

/// A source folder's card: 17 in, 8 under; the folder card's line border (ink while the folder needs reconnecting).
const SOURCE_CARD: Node = Node::column().w(Size::Grow).h(Size::Fit).pad(17.0).margin(Edges::ZERO.b(8.0));
const FOLDER_CARD_LOOKS: States = States::of(FOLDER_CARD);
const FOLDER_CARD_ERROR_LOOKS: States = States::of(FOLDER_CARD_ERROR);

fn source_card(ui: &mut Ui, user: i64, folder: &Folder, width: f32, intent: &mut Option<Intent>) {
    let target = Target { user, source: folder.id, root: folder.root };
    let error = folder.error.is_some();
    let looks = if error { &FOLDER_CARD_ERROR_LOOKS } else { &FOLDER_CARD_LOOKS };
    let card = ui.pressable(Key::Id(folder.id), SOURCE_CARD.role(if error { Role::Group } else { Role::Button }), &PLAIN.look(looks), |ui, _| {
        let top = container(ui, "source-top", Axis::Row, Size::Grow, Size::Px(44.0), &PAPER_FILL);
        ui.within(&top, &mut |ui| {
            ui.column(Node::column().w(Size::Grow).h(Size::Grow).gap(2.0), |ui| {
                ui.label(&folder.name, TEXT_17.grow().color(INK));
                ui.label(&folder.path, CAPTION.grow());
            });
            ui.label(&folder.count, CAPTION);
        });
        if let Some(message) = &folder.error {
            spacer(ui, "error-gap", 12.0);
            let row = ui.node("source-error", Node::row().w(Size::Grow).h_px(20.0).gap(8.0).align(Align::Center));
            ui.within(&row, &mut |ui| {
                super::songs::row::glyph(ui, (lw_core::FileState::Failed, Some(lw_core::FileAvailability::Unknown), false), true);
                ui.label(message, TEXT_13.grow().h_px(18.9).color(INK));
            });
            spacer(ui, "buttons-gap", 12.0);
            ui.id("source-buttons", |ui| { ui.row(Node::row().w(Size::Grow).h_px(44.0).gap(2.0), |ui| {
                let half = ((width - 34.0 - 2.0) / 2.0).floor().max(1.0);
                if ui.button("reconnect", "Reconnect folder", &PLATE_INK.size(Size::Px(half), Size::Px(44.0))).clicked {
                    *intent = Some(Intent::ReconnectSource(target));
                }
                let options = ui.button("source-options", "Options", &PLATE.size(Size::Px(half), Size::Px(44.0)));
                if options.clicked {
                    let at = ui.rect(&options.handle);
                    *intent = Some(Intent::SourceMenu(target, (at.x, at.y + at.h)));
                }
            }); });
        } else {
            spacer(ui, "source-bar-gap", 14.0);
            ui.progress("source-bar", "On this device", Some(folder.local), &BAR);
            spacer(ui, "source-bar-end", 10.0);
            let footer = container(ui, "source-footer", Axis::Row, Size::Grow, Size::Px(18.2), &PAPER_FILL);
            ui.within(&footer, &mut |ui| {
                ui.label(&folder.stats, CAPTION.grow());
                ui.label(&folder.scanned, CAPTION);
            });
        }
    });
    ui.a11y_name(&card.handle, &folder.name);
    if card.secondary {
        *intent = Some(Intent::SourceMenu(target, ui.signal(&card.handle).pointer));
    } else if !error && intent.is_none() && card.clicked {
        *intent = Some(Intent::Files(Request::Browse(Location { user, source: folder.id, parent: None })));
    }
}

fn folder_page(ui: &mut Ui, files: &crate::files::Snapshot, directory: &crate::files::Directory, (state, show_non_audio): (&ShellState, bool), compact: bool, intent: &mut Option<Intent>) {
    if compact {
        return ui.id("compact-folder", |ui| compact_folder(ui, files, directory, (state, show_non_audio), intent));
    }
    let location = directory.location;
    let target = Target { user: location.user, source: location.source, root: directory.source_root };
    ui.row(if compact { BACK_BAR.wrap() } else { BACK_BAR }, |ui| {
        if ui.button("back", "‹ Back", &FILES_BACK).clicked {
            *intent = Some(up(directory));
        }
        ui.node("top-space", Node::DEFAULT.w(Size::Grow).h(Size::Px(1.0)));
        if ui.button("import", "Import folder", &PLATE.size(Size::Px(108.0), Size::Px(36.0)).disabled(files.busy)).clicked {
            *intent = Some(Intent::Files(Request::Import(location)));
        }
        let options = ui.button("source-options", "Folder options", &PLATE.size(Size::Px(118.0), Size::Px(36.0)));
        if options.clicked {
            let at = ui.rect(&options.handle);
            *intent = Some(Intent::SourceMenu(target, (at.x, at.y + at.h)));
        }
    });
    if let Some(message) = &directory.error {
        let row = ui.node("folder-error", Node::row().w(Size::Grow).h_px(56.0).gap(12.0).pad_x(16.0).align(Align::Center).margin(Edges::ZERO.t(20.0)).look(INK_OUTLINE));
        ui.within(&row, &mut |ui| {
            super::songs::row::glyph(ui, (lw_core::FileState::Failed, Some(lw_core::FileAvailability::Unknown), false), true);
            ui.label(message, TEXT_14.grow().h_px(20.0).color(INK));
            if ui.button("reconnect", "Reconnect folder", &PLATE_INK.size(Size::Px(140.0), Size::Px(36.0)).disabled(files.busy)).clicked {
                *intent = Some(Intent::ReconnectSource(target));
            }
        });
    }
    ui.column(Node::column().w(Size::Grow).h(Size::Fit).padding(Edges::ZERO.t(28.0).b(18.0)), |ui| {
        if !directory.breadcrumb.is_empty() {
            ui.label(&directory.breadcrumb, BODY_DIM.grow());
            spacer(ui, "breadcrumb-gap", 8.0);
        }
        ui.label(&directory.title, TEXT_34.grow().h_px(36.0).color(INK));
        spacer(ui, "summary-gap", 6.0);
        ui.label(&directory.summary, BODY_DIM.grow());
    });
    let tools = ui.node("folder-tools", if compact { FOLDER_TOOLS.wrap() } else { FOLDER_TOOLS });
    ui.within(&tools, &mut |ui| {
        let ready = !directory.songs.is_empty() && !files.busy;
        if ui.button("play-folder", "Play folder", &TOOL.icon(icon_drawing(Icon::Play)).icon_size(16.0).disabled(!ready)).clicked {
            *intent = Some(Intent::PlayFolder);
        }
        if ui.button("shuffle", "Shuffle", &TOOL.icon(icon_drawing(Icon::Shuffle)).icon_size(16.0).disabled(!ready)).clicked {
            *intent = Some(Intent::ShuffleFolder);
        }
        if ui.button("keep-offline", if directory.kept { "Kept offline" } else { "Keep offline" }, &super::find::keep_offline(directory.kept).disabled(!ready)).clicked {
            *intent = Some(Intent::KeepFolder);
        }
        ui.node("tools-space", Node::DEFAULT.w(Size::Grow).h(Size::Px(1.0)));
        if !compact {
            legend(ui);
        }
    });
    ui.canvas_over(&tools, move |paint, rect| paint.fill(Rect::new(rect.x - 32.0, rect.y + rect.h - 1.0, rect.w + 64.0, 1.0), Fill::new(Rgba::token(LINE)).fractional()));
    status(ui, files, compact, intent);
    if directory.entries.is_empty() {
        ui.label("Nothing here.", TEXT_14.grow().h_px(48.0).color(SECONDARY));
        return;
    }
    // The parent participates in the identity, giving each destination its own scroll state.
    let key = int_key(int_key(pxui::ui::key_of(0, "files-directory"), location.source), location.parent.unwrap_or(0));
    let list = ui.node(Key::Id(key), Node::column().w(Size::Grow).h(Size::Grow));
    let entries = &directory.entries;
    let mut table = state.folder_table.borrow_mut();
    // the rows keep their builds while the snapshot holds; its flags (plain files disabled) are written once a snapshot
    if table.selection.revision != Some(files.revision) {
        table.selection.rows.resize(entries.len(), 0);
        for (flags, entry) in table.selection.rows.iter_mut().zip(entries) {
            let plain = !entry.directory && !entry.audio;
            *flags = *flags & pxui::ListState::SELECTED | if plain { pxui::ListState::DISABLED } else { 0 };
        }
        table.selection.revision = Some(files.revision);
    }
    // Desktop lists every file, the plain ones greyed (Swift).
    let mut r = None;
    ui.within(&list, &mut |ui| {
        r = Some(ui.table("directory-entries", &FOLDER_COLUMNS, entries.len(), &|i| entries[i].id, &mut table, &FOLDER_TABLE, &mut |ui, index, column, cell| {
            let entry = &entries[index];
            let other = !entry.directory && !entry.audio;
            let tone = if other { TERTIARY } else { SECONDARY };
            match column {
                0 if entry.audio => { ui.label(&entry.number, TEXT_13.color(tone)); }
                0 => mark(ui, entry.directory, Rgba::token(tone)),
                1 => {
                    ui.a11y_name(&cell.handle, &entry.name);
                    ui.label(&entry.name, TEXT_14.grow().color(if other { TERTIARY } else { INK }));
                }
                2 => { ui.label(&entry.kind, TEXT_14.grow().color(tone)); }
                3 => { ui.label(&entry.size, MONO_12.grow().color(tone)); }
                4 => match entry.glyph {
                    Some(glyph) => super::songs::row::glyph(ui, (glyph.state, glyph.availability, glyph.pinned), true),
                    None if entry.audio => {
                        // Not imported yet: Swift imports on play; here the row offers Import.
                        if ui.button("import-file", "Import", &PLATE.size(Size::Px(72.0), Size::Px(36.0)).disabled(files.busy)).clicked {
                            *intent = Some(Intent::Files(Request::ImportPath { user: location.user, source: location.source, path: entry.id }));
                        }
                    }
                    None => {}
                },
                _ => { ui.row(TIME_CELL, |ui| { ui.label(&entry.time, MONO_12.color(tone)); }); }
            }
        }));
    });
    drop(table);
    // a folder opens on a press or Return (`activated`), never on an arrow key
    let open = r.and_then(|r| r.rows.activated).filter(|&i| entries.get(i).is_some_and(|entry| entry.directory));
    if let Some(i) = open && intent.is_none() {
        *intent = Some(Intent::Files(Request::Browse(Location { parent: Some(entries[i].id), ..location })));
    }
}
/// A folder table's Time cell: its text ends 12 points before the kit's 8-point inset (20 in all, as its heading).
const TIME_CELL: Node = Node::row().w(Size::Grow).h(Size::Grow).align(Align::Center).justify(Justify::End).padding(Edges::ZERO.r(12.0));

/// The phone folder list (`localwave-row` / `PHONE_ROW`): 56-point rows, the overlay scrollbar, no frame.
const PHONE_FOLDER: pxui::SelectionList = pxui::SelectionList::DEFAULT.row_h(56.0).overlay_scrollbar().variant(variant::PHONE_ROW).bare();
/// A phone folder row: 56 points, 12 between its parts, 6 in from the end.
const ROW_56: Node = Node::row().w(Size::Grow).h_px(56.0).gap(12.0).align(Align::Center).padding(Edges::ZERO.r(6.0));

/// Swift's mobileFolder: back and item count, title with options, breadcrumb, Play folder / Shuffle / Keep offline
/// keys, All / Offline / In cloud tabs and 56 pt file rows (tap plays, "…" opens the song menu). Gap: no Select mode.
fn compact_folder(ui: &mut Ui, files: &crate::files::Snapshot, directory: &crate::files::Directory, (state, show_non_audio): (&ShellState, bool), intent: &mut Option<Intent>) {
    use super::compact::list::PLAY_PRIMARY;
    let files_filter = state.files_filter;
    let selecting = state.selecting;
    // Marks index the shown rows; "all" covers the saved songs among them.
    let shown = directory.shown(files_filter, show_non_audio);
    // the rows keep their builds while the snapshot, the filter, the mode and the marks hold; their flags (a plain file, and in Select
    // mode any row without a saved song, disabled) and the header's counts are prepared then in one pass, never on a warm frame
    let revision = pxui::hash_of(&(files.revision, files_filter as u8, selecting, show_non_audio, state.selection_revision));
    let mut marks = state.song_rows.borrow_mut();
    if marks.revision != Some(revision) {
        marks.rows.resize(shown, 0);
        let (mut selectable, mut selected) = (0, 0);
        for (i, flags) in marks.rows.iter_mut().enumerate() {
            let entry = directory.entry(files_filter, i);
            let song = entry.is_some_and(|entry| entry.song.is_some());
            let off = entry.is_none_or(|entry| if selecting { !song } else { !entry.directory && !entry.audio });
            *flags = *flags & pxui::ListState::SELECTED | if off { pxui::ListState::DISABLED } else { 0 };
            selectable += usize::from(song);
            selected += usize::from(song && *flags & pxui::ListState::SELECTED != 0);
        }
        marks.revision = Some(revision);
        state.folder_counts.set((selectable, selected));
        #[cfg(test)]
        tests::FOLDER_PASSES.set(tests::FOLDER_PASSES.get() + 1);
    }
    drop(marks);
    let (selectable, selected) = state.folder_counts.get();
    use super::compact::list::{BACK_KEY, MORE, WORD};
    let location = directory.location;
    let target = Target { user: location.user, source: location.source, root: directory.source_root };
    let top = ui.node("folder-top", Node::row().w(Size::Grow).h_px(44.0).align(Align::Center));
    if selecting {
        // Swift's selection header replaces the back row and the title.
        ui.within(&top, &mut |ui| {
            super::compact::list::selection_header(ui, selected, selectable, shown, intent);
        });
    } else {
    ui.within(&top, &mut |ui| {
        if ui.button("back", if directory.up.is_some() { "Back" } else { "Folders" }, &BACK_KEY.size(Size::Fit, Size::Px(44.0))).clicked {
            *intent = Some(up(directory));
        }
        ui.node("top-space", Node::DEFAULT.w(Size::Grow).h(Size::Px(1.0)));
        ui.label(&directory.detail, MONO_DIM);
        if directory.songs.is_empty() {
            return;
        }
        if ui.button("select", "Select", &WORD.pad(Edges::ZERO.l(12.0))).clicked {
            *intent = Some(Intent::SelectMode(true));
        }
    });
    }
    if !selecting {
    ui.id("folder-titles", |ui| { ui.row(Node::row().w(Size::Grow).h_px(48.0).align(Align::Center), |ui| {
        ui.id("name", |ui| { ui.column(Node::column().w(Size::Grow).h(Size::Fit), |ui| { ui.label(&directory.title, PAGE_TITLE); }); });
        let more = ui.icon_button("source-options", icon_drawing(Icon::More), "Folder options", &ICON_44.look(&ICON_LOOKS));
        if more.clicked {
            let at = ui.rect(&more.handle);
            *intent = Some(Intent::SourceMenu(target, (at.x, at.y + at.h)));
        }
    }); });
    }
    if !directory.breadcrumb.is_empty() {
        ui.label(&directory.breadcrumb, TEXT_13.grow().h_px(24.0).color(SECONDARY));
    }
    if let Some(message) = &directory.error {
        let row = ui.node("folder-error", Node::row().w(Size::Grow).h_px(56.0).gap(8.0).pad_x(12.0).align(Align::Center).margin(Edges::ZERO.t(8.0)).look(INK_OUTLINE));
        ui.within(&row, &mut |ui| {
            super::songs::row::glyph(ui, (lw_core::FileState::Failed, Some(lw_core::FileAvailability::Unknown), false), true);
            ui.label(message, TEXT_13.grow().h_px(18.9).color(INK));
            if ui.button("reconnect", "Reconnect", &PLATE_INK.size(Size::Px(88.0), Size::Px(44.0)).disabled(files.busy)).clicked {
                *intent = Some(Intent::ReconnectSource(target));
            }
        });
    }
    let ready = !directory.songs.is_empty() && !files.busy;
    ui.row(Node::row().w(Size::Grow).h_px(52.0).gap(8.0).margin(Edges::ZERO.t(12.0).b(8.0)), |ui| {
        if ui.button("play-folder", "Play folder", &PLAY_PRIMARY.icon(icon_drawing(Icon::Play)).disabled(!ready)).clicked {
            *intent = Some(Intent::PlayFolder);
        }
        if ui.icon_button("shuffle", icon_drawing(Icon::Shuffle), "Shuffle folder", &KEY_52.disabled(!ready)).clicked {
            *intent = Some(Intent::ShuffleFolder);
        }
        let (icon, name) = if directory.kept { (Icon::Kept, "Kept offline") } else { (Icon::Offline, "Keep folder offline") };
        if ui.icon_button("keep-offline", icon_drawing(icon), name, &KEY_52.selected(directory.kept).disabled(!ready)).clicked {
            *intent = Some(Intent::KeepFolder);
        }
    });
    status(ui, files, true, intent);
    if directory.entries.is_empty() {
        ui.label("No files in this folder.", TEXT_14.grow().h_px(48.0).color(SECONDARY));
        return;
    }
    // Swift's All / Offline / In cloud tabs; the worker listed what each shows.
    const FILTERS: [FilesFilter; 3] = [FilesFilter::All, FilesFilter::Offline, FilesFilter::Cloud];
    let filters = [
        pxui::Segment::new("All").id(0).detail(&directory.filter_counts[0]),
        pxui::Segment::new("Offline").id(1).detail(&directory.filter_counts[1]),
        pxui::Segment::new("In cloud").id(2).detail(&directory.filter_counts[2]),
    ];
    let chosen = FILTERS.iter().position(|f| *f == files_filter);
    let tabs = ui.tabs("filter-tabs", &filters, chosen, &FILTER);
    if tabs.response.clicked && let Some(i) = tabs.chosen {
        *intent = Some(Intent::FilesFilter(FILTERS[i]));
    }
    let key = int_key(int_key(pxui::ui::key_of(0, "files-directory"), location.source), location.parent.unwrap_or(0));
    let list = ui.node(Key::Id(key), Node::column().w(Size::Grow).h(Size::Grow));
    // Swift's "N non-audio files hidden · show" closes the list (its trailer) while the setting is off.
    let footer = !show_non_audio && directory.non_audio > 0;
    // Select mode marks a saved song a tap (the list toggles its row; the marks are the songs list's flags); outside it a tap opens,
    // plays or imports the row (the list selects nothing)
    let rows = PHONE_FOLDER.trail(if footer { 56.0 } else { 0.0 })
        .mode(if selecting { pxui::SelectMode::Toggle } else { pxui::SelectMode::None }).open_on_click(!selecting);
    let mut marks = state.song_rows.borrow_mut();
    let mut r = None;
    ui.within(&list, &mut |ui| {
        r = Some(ui.selection_list("directory-entries", shown, &|i| directory.entry(files_filter, i).map_or(i as i64, |entry| entry.id), &mut marks, &rows, &mut |ui, part, cell| {
            let index = match part {
                SectionPart::Row(index) => index,
                SectionPart::Trailing if footer => {
                    ui.row(Node::row().w(Size::Grow).h_px(56.0).gap(4.0).align(Align::Center), |ui| {
                        ui.label(&directory.hidden_label, BODY_DIM);
                        if ui.button("show", "show", &SHOW).clicked {
                            *intent = Some(Intent::Setting(lw_core::preferences::Change::ToggleShowNonAudio));
                        }
                    });
                    return;
                }
                _ => return,
            };
            let Some(entry) = directory.entry(files_filter, index) else { return };
            let other = !entry.directory && !entry.audio;
            ui.a11y_name(&cell.handle, &entry.name);
            let row = ui.node("content", ROW_56);
            ui.within(&row, &mut |ui| {
                ui.column(Node::column().w_px(16.0).h_px(16.0), |ui| match entry.glyph {
                    Some(glyph) if entry.audio => super::songs::row::glyph(ui, (glyph.state, glyph.availability, glyph.pinned), true),
                    _ => mark(ui, entry.directory, Rgba::token(SECONDARY)),
                });
                ui.column(Node::column().w(Size::Grow).h(Size::Fit), |ui| {
                    ui.label(&entry.name, TEXT_15.grow().color(if other { TERTIARY } else { INK }));
                    ui.label(&entry.detail, MONO_DIM.grow());
                });
                if selecting {
                    // Swift's trailing 44 pt checkbox, on saved songs only.
                    if entry.song.is_some() {
                        ui.id("select-cell", |ui| { ui.row(Node::row().w_px(44.0).h_px(44.0).center(), |ui| { ui.checkbox("mark", &entry.name, cell.selected, &super::compact::list::MARK); }); });
                    }
                } else if entry.directory {
                    // Folder rows: Open folder / Play folder / Shuffle folder.
                    let more = ui.icon_button("actions", icon_drawing(Icon::More), "Actions for folder", &MORE);
                    if more.clicked {
                        let at = ui.rect(&more.handle);
                        *intent = Some(Intent::FolderMenu(entry.id, (at.x + at.w - 44.0, at.y)));
                    }
                // Swift's per-file "…"; only saved songs have actions here.
                } else if let Some(song) = entry.song {
                    let more = ui.icon_button("actions", icon_drawing(Icon::More), "Actions for file", &MORE);
                    if more.clicked {
                        let at = ui.rect(&more.handle);
                        *intent = Some(Intent::SongMenu(index, song, 0, (at.x + at.w - 44.0, at.y)));
                    }
                }
            });
            ui.canvas_over(&row, move |paint, rect| paint.fill(Rect::new(rect.x + 28.0, rect.y + rect.h - 1.0, rect.w - 28.0, 1.0), Fill::new(Rgba::token(LINE)).fractional()));
        }));
    });
    drop(marks);
    let Some(r) = r else { return };
    if let Some(i) = r.selected
        && intent.is_none()
    {
        // a change that leaves no row marked (Escape) is the app's Clear
        let none = !state.song_rows.borrow().rows.iter().any(|f| f & pxui::ListState::SELECTED != 0);
        *intent = Some(if none { Intent::Select(crate::Selection::Clear) } else { Intent::Selected(i) });
    }
    if let Some(entry) = r.activated.and_then(|i| directory.entry(files_filter, i))
        && intent.is_none()
    {
        *intent = if entry.directory {
            Some(Intent::Files(Request::Browse(Location { parent: Some(entry.id), ..location })))
        } else if let Some(song) = entry.song {
            Some(Intent::PlayFile(song))
        } else if entry.audio && !files.busy {
            // Not imported yet: Swift imports on play; here a tap imports the file.
            Some(Intent::Files(Request::ImportPath { user: location.user, source: location.source, path: entry.id }))
        } else {
            None
        };
    }
    // the window's default key target: Return opens the keyboard's row; Command-A and Escape mark in Select mode only
    ui.focus_default(&r.handle);
    if selecting {
        ui.id("selection-deck", |ui| { ui.column(Node::column().w(Size::Grow).h_px(64.0).margin(Edges::ZERO.l(-16.0).r(-16.0).b(-12.0)), |ui| {
            if let Some((key, rect)) = super::compact::list::deck(ui, selected > 0, false) {
                *intent = Some(match key {
                    "play-next" => Intent::Page(crate::PageAction::PlayNext),
                    "add-to-list" => Intent::PageListChoices((rect.x, rect.y - 36.0 * 8.0)),
                    "offline" => Intent::Page(crate::PageAction::KeepOffline),
                    _ => Intent::Page(crate::PageAction::AddToQueue),
                });
            }
        }); });
    }
}

/// A folder row's menu: Open folder, Play folder, Shuffle folder.
pub(super) fn folder_menu(ui: &mut Ui, player: &PlayerView<'_>, entry: i64, at: (f32, f32), intent: &mut Option<Intent>) {
    let Some(directory) = player.files.and_then(|files| files.directory.as_ref()) else {
        *intent = Some(Intent::CloseFolderMenu);
        return;
    };
    let Some(found) = directory.entries.iter().find(|candidate| candidate.id == entry && candidate.directory) else {
        *intent = Some(Intent::CloseFolderMenu);
        return;
    };
    let filled = !found.songs.is_empty();
    let rows = [
        MenuItem::new("Open folder").id(OPEN),
        MenuItem::new("Play folder").id(PLAY).disabled(!filled),
        MenuItem::new("Shuffle folder").id(SHUFFLE).disabled(!filled),
    ];
    // Swift's folder menu ends 160 points short of the view's end, opening leftward from the row's "…" key.
    let x = at.0.min(ui.view_size().width - 200.0) - 160.0;
    let menu = ui.menu("folder-menu", pxui::Point::new(x, at.1), &rows, &super::songs::MENU.name("Folder actions"));
    match menu.chosen_id {
        Some(OPEN) => *intent = Some(Intent::Files(Request::Browse(Location { parent: Some(entry), ..directory.location }))),
        Some(PLAY) => *intent = Some(Intent::PlayEntryFolder(entry)),
        Some(_) => *intent = Some(Intent::ShuffleEntryFolder(entry)),
        None => {}
    }
    if menu.dismissed && intent.is_none() {
        *intent = Some(Intent::CloseFolderMenu);
    }
}

/// Swift's Add music sheet: a folder or some files (`localwave-sheet` / `CHOICES`); Close, Escape or a press outside closes it.
pub(super) fn add_music(ui: &mut Ui, player: &PlayerView<'_>, intent: &mut Option<Intent>) {
    let enabled = player.files.is_some_and(|files| files.account.is_some() && !files.busy);
    let buttons = [
        DialogButton::new("A folder").detail("Watched for changes. iCloud Drive, Google Drive, Dropbox, On My iPhone, USB drives, SMB servers.").disabled(!enabled),
        DialogButton::new("Some files").detail("Play them now; they join the library too.").disabled(!enabled),
        DialogButton::new("Close").cancel(),
    ];
    let sheet = ui.dialog("add-music", "Add music", &buttons, &ADD_MUSIC.w_px(sheet_width(ui)), &mut |ui| {
        ui.label("Files stay where they are", CAPTION.grow());
    });
    match sheet.chosen {
        Some(0) => *intent = Some(Intent::ChooseFolder),
        Some(1) => *intent = Some(Intent::ChooseAudio),
        Some(_) => *intent = Some(Intent::CloseSourceMenu),
        None if sheet.dismissed => *intent = Some(Intent::CloseSourceMenu),
        None => {}
    }
}

/// The right-hand key of the folder toolbar.
fn legend(ui: &mut Ui) {
    use lw_core::{FileAvailability::{InCloud, OnDevice}, FileState::Copied};
    ui.row(Node::row().w(Size::Fit).h_px(30.0).gap(6.0).align(Align::Center).padding(Edges::ZERO.l(4.0)), |ui| {
        super::songs::row::glyph(ui, (Copied, Some(OnDevice), true), true);
        ui.label("kept offline", FOOTNOTE);
        super::songs::row::glyph(ui, (Copied, Some(OnDevice), false), true);
        ui.label("on this Mac", FOOTNOTE);
        super::songs::row::glyph(ui, (Copied, Some(InCloud), false), true);
        ui.label("in cloud", FOOTNOTE);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::{Directory, Entry, Folder, Snapshot};

    thread_local! {
        /// How many times the phone folder list prepared its row flags and counts (an O(rows) pass), for the warm-frame check.
        pub(super) static FOLDER_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    use pxui::{Event, FrameIn, Platform};
    use std::sync::Arc;

    fn frame(
        ui: &mut Ui,
        files: &Snapshot,
        catalog: &Catalog,
        state: &ShellState,
        time: &mut f64,
        events: &[Event],
        intent: &mut Option<Intent>,
    ) {
        frame_at((1440.0, 872.0), ui, files, catalog, state, time, events, intent)
    }
    #[allow(clippy::too_many_arguments)]
    fn frame_at(
        size: (f32, f32),
        ui: &mut Ui,
        files: &Snapshot,
        catalog: &Catalog,
        state: &ShellState,
        time: &mut f64,
        events: &[Event],
        intent: &mut Option<Intent>,
    ) {
        *time += 1.0 / 60.0;
        crate::testing::frame(
            ui,
            &{
                let mut frame = FrameIn::new(size.0, size.1).time(*time);
                frame.wire.events = events.as_ptr();
                frame.wire.event_count = events.len();
                frame
            },
            &mut |ui| {
                shell(
                    ui,
                    &PlayerView {
                        files: Some(files),
                        ..Default::default()
                    },
                    0.0,
                    catalog,
                    state,
                    intent,
                )
            },
        )
        .unwrap();
    }
    fn click(
        ui: &mut Ui,
        files: &Snapshot,
        catalog: &Catalog,
        state: &ShellState,
        time: &mut f64,
        name: &str,
    ) -> Option<Intent> {
        click_at((1440.0, 872.0), ui, files, catalog, state, time, name)
    }
    fn click_at(
        size: (f32, f32),
        ui: &mut Ui,
        files: &Snapshot,
        catalog: &Catalog,
        state: &ShellState,
        time: &mut f64,
        name: &str,
    ) -> Option<Intent> {
        let frame = |ui: &mut Ui, files: &Snapshot, catalog: &Catalog, state: &ShellState, time: &mut f64, events: &[Event], intent: &mut Option<Intent>| frame_at(size, ui, files, catalog, state, time, events, intent);
        let rect = ui
            .a11y_nodes()
            .iter()
            .find(|node| ui.a11y_name_of(node) == name)
            .unwrap_or_else(|| panic!("missing {name}"))
            .rect;
        let (x, y) = (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5);
        let mut intent = None;
        frame(
            ui,
            files,
            catalog,
            state,
            time,
            &[
                Event::pointer(input::wire::POINTER_MOVED, 0, x as f64, y as f64),
                Event::pointer(input::wire::POINTER_PRESSED, 0, x as f64, y as f64),
            ],
            &mut intent,
        );
        frame(
            ui,
            files,
            catalog,
            state,
            time,
            &[Event::pointer(input::wire::POINTER_RELEASED, 0, x as f64, y as f64)],
            &mut intent,
        );
        intent
    }

    fn names(ui: &Ui) -> Vec<String> {
        ui.a11y_nodes().iter().map(|node| ui.a11y_name_of(node).to_string()).collect()
    }

    #[test]
    fn source_cards_and_folder_pages_show_swift_labels_route_actions_and_stay_allocation_free() {
        use crate::files::Glyph;
        use lw_core::{FileAvailability, FileState};
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let card = |id: i64, name: &str, error: Option<&str>| Folder {
            id,
            root: id + 10,
            name: name.into(),
            path: "/Music/Library".into(),
            count: "34 songs".into(),
            stats: "0 kept · 28 on device · 6 in cloud".into(),
            scanned: "2 min. ago".into(),
            error: error.map(Into::into),
            local: 0.82,
            current: id == 1,
        };
        let mut files = Snapshot {
            revision: 1,
            account: Some(42),
            source_count: "2 sources, watched for changes".into(),
            offline_usage: "0.0 of 8 GB".into(),
            sources: Arc::from([card(1, "iCloud Drive", None), card(2, "Dropbox", Some("Access expired."))]),
            ..Default::default()
        };
        let catalog = Catalog::default();
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Files));
        let mut time = 0.0;
        for _ in 0..3 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        let seen = names(&ui);
        for expected in [
            "Folders", "2 sources, watched for changes", "iCloud Drive", "34 songs",
            "0 kept · 28 on device · 6 in cloud", "2 min. ago", "Access expired.",
            "Reconnect folder", "Options", "Offline space", "0.0 of 8 GB",
            "Cloud songs download when played and the next two download ahead. Pinned folders stay.",
        ] {
            assert!(seen.iter().any(|name| name == expected), "missing {expected}: {seen:?}");
        }
        for round in 0..5 {
            mem_sys::local_reset();
            for _ in 0..10 {
                frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
            }
            if round >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm source cards allocate");
            }
        }
        let location = Location { user: 42, source: 1, parent: Some(5) };
        let glyph = |state, availability, pinned| Some(Glyph { state, availability: Some(availability), pinned });
        let entry = |id: i64, name: &str, audio: bool, glyph| Entry {
            id,
            name: name.into(),
            number: if audio { id.to_string() } else { String::new() },
            kind: if audio { "flac audio".into() } else { "Not audio".into() },
            size: "20.3 MB".into(),
            time: "2:29".into(),
            glyph,
            audio,
            ..Default::default()
        };
        files.directory = Some(Arc::new(Directory {
            location,
            source_root: 5,
            up: Some(4),
            title: "Field Recordings 04".into(),
            breadcrumb: "iCloud Drive / Halden Marsh / Field Recordings 04".into(),
            summary: "8 songs · 310.9 MB · 7 offline".into(),
            detail: "3 items".into(),
            songs: Arc::from([1, 2]),
            entries: vec![
                entry(1, "01 Blue Hour.flac", true, glyph(FileState::Copied, FileAvailability::OnDevice, true)),
                entry(2, "02 Sodium.flac", true, None),
                entry(3, "cover.jpg", false, None),
            ],
            ..Default::default()
        }));
        files.revision += 1;
        for _ in 0..3 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        let seen = names(&ui);
        for expected in [
            "Back", "iCloud Drive / Halden Marsh / Field Recordings 04", "Field Recordings 04",
            "8 songs · 310.9 MB · 7 offline", "Play folder", "Shuffle", "Keep offline",
            "kept offline", "on this Mac", "in cloud", "#", "Name", "Kind", "Size", "On device",
            "Time", "01 Blue Hour.flac", "flac audio", "Not audio", "cover.jpg", "20.3 MB", "2:29",
            "Import folder", "Folder options",
        ] {
            assert!(seen.iter().any(|name| name == expected), "missing {expected}: {seen:?}");
        }
        assert!(seen.iter().any(|name| name == "Kept offline"), "the pinned on-device row's glyph says so");
        for (label, intent) in [
            ("Play folder", Intent::PlayFolder),
            ("Shuffle", Intent::ShuffleFolder),
            ("Keep offline", Intent::KeepFolder),
        ] {
            assert_eq!(click(&mut ui, &files, &catalog, &state, &mut time, label), Some(intent));
        }
        assert_eq!(
            click(&mut ui, &files, &catalog, &state, &mut time, "Import"),
            Some(Intent::Files(Request::ImportPath { user: 42, source: 1, path: 2 })),
            "only the unimported audio row offers Import"
        );
        for round in 0..5 {
            mem_sys::local_reset();
            for _ in 0..10 {
                frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
            }
            if round >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm folder page allocates");
            }
        }
        // U4b.4 #73 / U2b.4.37: the table's folder row opens on a press, again on the next press (nothing is selected), and on Return,
        // never on an arrow key; a plain file's row is disabled and takes none
        let mut with_folder = Directory { location, source_root: 5, ..Default::default() };
        with_folder.entries = vec![
            Entry { id: 9, name: "Takes".into(), directory: true, ..Default::default() },
            Entry { id: 10, name: "Mixes".into(), directory: true, ..Default::default() },
            entry(3, "cover.jpg", false, None),
        ];
        files.directory = Some(Arc::new(with_folder));
        files.revision += 1;
        for _ in 0..3 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        let browse = Intent::Files(Request::Browse(Location { parent: Some(9), ..location }));
        assert_eq!(click(&mut ui, &files, &catalog, &state, &mut time, "Takes"), Some(browse));
        state.apply(browse);
        assert_eq!(click(&mut ui, &files, &catalog, &state, &mut time, "Takes"), Some(browse), "a second press opens again");
        for (key, want) in [(input::Key::ArrowDown, None), (input::Key::ArrowUp, None), (input::Key::Enter, Some(browse))] {
            let mut intent = None;
            let (down, up) = (Event::key(Event::KEY_PRESSED, key.code()), Event::key(Event::KEY_RELEASED, key.code()));
            frame(&mut ui, &files, &catalog, &state, &mut time, &[down, up], &mut intent);
            assert_eq!(intent, want, "{key:?}: arrows never open, Return opens the keyboard's row");
        }
        assert_eq!(click(&mut ui, &files, &catalog, &state, &mut time, "cover.jpg"), None);
        let row = |name: &str| ui.a11y_nodes().iter().find(|node| node.role == Role::Row as u32 && ui.a11y_name_of(node) == name).map(|node| node.states & pxui::A11yStates::ENABLED.bits() != 0);
        assert_eq!((row("Takes"), row("cover.jpg")), (Some(true), Some(false)), "the folder's row is enabled, the plain file's disabled");
        // Nothing saved: Play folder, Shuffle and Keep offline are disabled; kept shows Kept offline.
        let mut empty = Directory { location, source_root: 5, ..Default::default() };
        empty.entries = vec![entry(3, "cover.jpg", false, None)];
        files.directory = Some(Arc::new(empty));
        files.revision += 1;
        for _ in 0..3 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        assert_eq!(click(&mut ui, &files, &catalog, &state, &mut time, "Play folder"), None);
        assert_eq!(click(&mut ui, &files, &catalog, &state, &mut time, "Keep offline"), None);
    }

    #[test]
    fn files_virtualize_borrowed_rows_route_actions_and_scroll_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut files = Snapshot {
            revision: 1,
            account: Some(42),
            source_count: "1 source".into(),
            sources: Arc::from([Folder {
                id: 7,
                root: 11,
                name: "Original music".into(),
                path: "/Music/Original".into(),
                count: "10000 indexed items".into(),
                current: true,
                ..Default::default()
            }]),
            ..Default::default()
        };
        let catalog = Catalog::default();
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Files));
        let mut time = 0.0;
        for _ in 0..3 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        assert_eq!(
            click(&mut ui, &files, &catalog, &state, &mut time, "Add music"),
            Some(Intent::ShowAddMusic)
        );
        let target = Target {
            user: 42,
            source: 7,
            root: 11,
        };
        // An expired source's card offers Reconnect folder and Options instead of opening.
        let sources = files.sources.clone();
        files.sources = Arc::from([Folder {
            id: 7,
            root: 11,
            name: "Original music".into(),
            path: "/Music/Original".into(),
            count: "10000 indexed items".into(),
            error: Some("Access expired.".into()),
            current: true,
            ..Default::default()
        }]);
        files.revision += 1;
        for _ in 0..2 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        assert_eq!(
            click(&mut ui, &files, &catalog, &state, &mut time, "Reconnect folder"),
            Some(Intent::ReconnectSource(target))
        );
        let menu = click(&mut ui, &files, &catalog, &state, &mut time, "Options").unwrap();
        assert!(
            matches!(menu, Intent::SourceMenu(value, (x, y)) if value == target && x > 0.0 && y > 0.0)
        );
        files.sources = sources;
        files.revision += 1;
        for _ in 0..2 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        state.apply(Intent::SourceMenu(target, (500.0, 200.0)));
        for _ in 0..2 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        assert_eq!(
            click(
                &mut ui,
                &files,
                &catalog,
                &state,
                &mut time,
                "Reconnect folder…"
            ),
            Some(Intent::ReconnectSource(target))
        );
        state.apply(Intent::CloseSourceMenu);
        for _ in 0..2 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        state.apply(Intent::SourceMenu(target, (500.0, 200.0)));
        for _ in 0..2 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        let remove = click(
            &mut ui,
            &files,
            &catalog,
            &state,
            &mut time,
            "Remove source",
        )
        .unwrap();
        assert_eq!(remove, Intent::ConfirmRemoveSource(target));
        state.apply(remove);
        for _ in 0..2 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        assert!(ui.a11y_nodes().iter().any(|node| node.is_role(Role::Dialog) && ui.a11y_name_of(node) == "Remove folder?"));
        assert_eq!(
            click(&mut ui, &files, &catalog, &state, &mut time, "Cancel"),
            Some(Intent::CloseRemoveSource)
        );
        state.apply(Intent::CloseRemoveSource);
        for _ in 0..2 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        for round in 0..6 {
            mem_sys::local_reset();
            for phase in 0..3 {
                state.source_menu = (phase == 1).then_some((target, (500.0, 200.0)));
                state.remove_source = (phase == 2).then_some(target);
                state.revision += 1;
                for _ in 0..3 {
                    frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
                }
            }
            if round >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm source options and confirmation must retain their layouts"
                );
            }
        }
        assert_eq!(
            click(&mut ui, &files, &catalog, &state, &mut time, "Remove"),
            Some(Intent::Files(Request::Remove(target)))
        );
        files.busy = true;
        files.revision += 1;
        for _ in 0..2 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        assert_eq!(
            click(&mut ui, &files, &catalog, &state, &mut time, "Remove"),
            None
        );
        files.busy = false;
        files.revision += 1;
        let sources = files.sources.clone();
        files.sources = Arc::from([Folder {
            id: 7,
            root: 12,
            name: "Reconnected elsewhere".into(),
            path: "/Different music".into(),
            current: true,
            ..Default::default()
        }]);
        for _ in 0..2 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        assert_eq!(
            click(&mut ui, &files, &catalog, &state, &mut time, "Remove"),
            None,
            "an old confirmation cannot remove a reconnected root"
        );
        files.sources = sources;
        files.revision += 1;
        state.apply(Intent::CloseRemoveSource);
        for _ in 0..2 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        let location = Location {
            user: 42,
            source: 7,
            parent: None,
        };
        assert_eq!(
            click(
                &mut ui,
                &files,
                &catalog,
                &state,
                &mut time,
                "Original music"
            ),
            Some(Intent::Files(Request::Browse(location)))
        );
        files.revision += 1;
        files.directory = Some(Arc::new(Directory {
            source_root: 11,
            location: Location {
                parent: Some(11),
                ..location
            },
            up: None,
            title: "Original music".into(),
            detail: "10000 items".into(),
            entries: (0..10_000)
                .map(|id| Entry {
                    id,
                    name: format!("Track {id:05}.flac"),
                    kind: "flac audio".into(),
                    size: "0.0 MB".into(),
                    directory: false,
                    audio: true,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }));
        for _ in 0..3 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        assert!(
            ui.a11y_nodes().len() < 1000,
            "all 10000 paths must not become UI nodes"
        );
        assert_eq!(
            click(
                &mut ui,
                &files,
                &catalog,
                &state,
                &mut time,
                "Import folder"
            ),
            Some(Intent::Files(Request::Import(Location {
                parent: Some(11),
                ..location
            })))
        );
        for round in 0..6 {
            mem_sys::local_reset();
            for step in 0..20 {
                let events = [
                    Event::pointer(input::wire::POINTER_MOVED, 0, 650.0, 400.0),
                    Event::pointer(input::wire::SCROLLED, 0, 0.0, if step < 10 { -1.0 } else { 1.0 }),
                ];
                let mut intent = None;
                frame(
                    &mut ui,
                    &files,
                    &catalog,
                    &state,
                    &mut time,
                    &events,
                    &mut intent,
                );
                assert_eq!(intent, None);
            }
            if round >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0);
            }
        }
        files.revision += 1;
        files.busy = true;
        files.progress = "Indexing: 300 items".into();
        for _ in 0..3 {
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        assert_eq!(
            click(&mut ui, &files, &catalog, &state, &mut time, "Cancel"),
            Some(Intent::Files(Request::Cancel))
        );
        assert_eq!(
            click(&mut ui, &files, &catalog, &state, &mut time, "Import folder"),
            None,
            "a running job disables Import"
        );
        assert_eq!(
            click(&mut ui, &files, &catalog, &state, &mut time, "Back"),
            Some(Intent::Files(Request::Roots))
        );
        state.apply(Intent::Navigate(Route::Songs));
        assert!(!state.files_visible());
        for step in 0..3 {
            // Reach idle after navigation using the injected clock, without real waits.
            if step == 1 {
                time += 1.0;
            }
            frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        let unchanged = ui.draw_revision();
        files.revision += 1;
        frame(&mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        assert_eq!(
            ui.draw_revision(),
            unchanged,
            "hidden Files progress must not redraw the library"
        );
        files.busy = false;
        files.progress.clear();
        files.revision += 1;
        state.apply(Intent::Navigate(Route::Files));
        for step in 0..3 {
            time += if step == 1 { 1.0 } else { 1.0 / 60.0 };
            crate::testing::frame(
                &mut ui,
                &FrameIn::new(640.0, 480.0).time(time),
                &mut |ui| {
                    shell(
                        ui,
                        &PlayerView {
                            files: Some(&files),
                            ..Default::default()
                        },
                        0.0,
                        &catalog,
                        &state,
                        &mut None,
                    );
                },
            )
            .unwrap();
        }
        let mut right = 0.0;
        for name in ["Back", "Import folder", "Folder options"] {
            let rect = ui
                .a11y_nodes()
                .iter()
                .find(|node| ui.a11y_name_of(node) == name)
                .unwrap()
                .rect;
            assert!(
                rect.x >= right && rect.x + rect.w <= 640.0 && rect.y + rect.h <= 480.0,
                "{name}: {rect:?}"
            );
            right = rect.x + rect.w;
        }
    }

    #[test]
    fn compact_folder_page_has_swift_keys_rows_and_routes_actions_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let size = (390.0, 778.0);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let location = Location { user: 42, source: 1, parent: Some(5) };
        let entry = |id: i64, name: &str, detail: &str, audio: bool, directory: bool| Entry {
            id, name: name.into(), detail: detail.into(), audio, directory,
            number: if audio { id.to_string() } else { String::new() },
            ..Default::default()
        };
        let directory = Directory {
            location, source_root: 5, up: Some(4), title: "Field Recordings".into(), breadcrumb: "iCloud / Field Recordings".into(),
            detail: "3 items".into(), songs: Arc::from([9_i64]),
            entries: vec![entry(1, "Takes", "Folder", false, true), entry(2, "01 Blue Hour.flac", "FLAC · 20.3 MB · 2:29", true, false), entry(3, "cover.jpg", "not audio", false, false)],
            non_audio: 1,
            ..Default::default()
        };
        let files = Snapshot { revision: 1, account: Some(42), directory: Some(Arc::new(directory)), ..Default::default() };
        let (catalog, mut state, mut time) = (Catalog::default(), ShellState::default(), 0.0);
        state.apply(Intent::Navigate(Route::Files));
        for _ in 0..3 {
            frame_at(size, &mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        let seen = names(&ui);
        for expected in ["Back", "Folder options", "Play folder", "Shuffle folder", "Keep folder offline", "Takes", "01 Blue Hour.flac"] {
            assert!(seen.iter().any(|name| name == expected), "missing {expected}: {seen:?}");
        }
        assert!(!seen.iter().any(|name| name == "cover.jpg"), "non-audio files stay hidden without the setting");
        for (label, intent) in [
            ("Play folder", Intent::PlayFolder),
            ("Shuffle folder", Intent::ShuffleFolder),
            ("Keep folder offline", Intent::KeepFolder),
            ("Back", Intent::Files(Request::Browse(Location { parent: Some(4), ..location }))),
            ("Takes", Intent::Files(Request::Browse(Location { parent: Some(1), ..location }))),
        ] {
            assert_eq!(click_at(size, &mut ui, &files, &catalog, &state, &mut time, label), Some(intent), "{label}");
        }
        for round in 0..5 {
            mem_sys::local_reset();
            for _ in 0..10 {
                frame_at(size, &mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
            }
            if round >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm compact folder page allocates");
            }
        }
        // the rows' flags and the header's counts (one pass over every shown row) are prepared per snapshot, filter, mode and marks:
        // a warm frame reads them and walks no row (U4b review ruling 3)
        let passes = FOLDER_PASSES.get();
        for _ in 0..10 {
            frame_at(size, &mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        assert_eq!(FOLDER_PASSES.get(), passes, "a warm frame prepares nothing");
        state.apply(Intent::SelectMode(true));
        frame_at(size, &mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        assert_eq!(FOLDER_PASSES.get(), passes + 1, "a mode change prepares once");
    }

    #[test]
    fn compact_folder_select_mode_folder_menus_and_hidden_footer_route_intents_without_warm_allocations() {
        use lw_core::FileAvailability::OnDevice;
        mem_sys::set_peer(pxui::library_counter);
        let size = (390.0, 778.0);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let location = Location { user: 42, source: 1, parent: Some(5) };
        let glyph = Some(crate::files::Glyph { state: lw_core::FileState::Copied, availability: Some(OnDevice), pinned: false });
        let audio = |id: i64, name: &str, song: i64| Entry { id, name: name.into(), detail: "FLAC".into(), audio: true, song: Some(song), glyph, ..Default::default() };
        let directory = Directory {
            location, source_root: 5, up: Some(4), title: "Field".into(), detail: "4 items".into(), songs: Arc::from([9_i64, 10, 11]),
            entries: vec![
                Entry { id: 1, name: "Takes".into(), detail: "Folder".into(), directory: true, songs: Arc::from([11_i64]), ..Default::default() },
                audio(2, "01 Here.flac", 9),
                audio(3, "02 There.flac", 10),
                Entry { id: 4, name: "cover.jpg".into(), detail: "not audio".into(), ..Default::default() },
            ],
            non_audio: 1,
            hidden_label: "1 non-audio file hidden ·".into(),
            filter_counts: ["2".into(), "2".into(), "0".into()],
            ..Default::default()
        };
        let files = Snapshot { revision: 1, account: Some(42), directory: Some(Arc::new(directory)), ..Default::default() };
        let (catalog, mut state, mut time) = (Catalog::default(), ShellState::default(), 0.0);
        state.apply(Intent::Navigate(Route::Files));
        let settle = |ui: &mut Ui, state: &ShellState, time: &mut f64| {
            for _ in 0..3 {
                frame_at(size, ui, &files, &catalog, state, time, &[], &mut None);
            }
        };
        settle(&mut ui, &state, &mut time);
        let seen = names(&ui);
        for want in ["Select", "Actions for folder", "show"] {
            assert!(seen.iter().any(|name| name == want), "missing {want}: {seen:?}");
        }
        assert_eq!(
            click_at(size, &mut ui, &files, &catalog, &state, &mut time, "show"),
            Some(Intent::Setting(lw_core::preferences::Change::ToggleShowNonAudio))
        );
        // The folder row's menu opens, then Open / Play / Shuffle folder emit their intents.
        let menu = click_at(size, &mut ui, &files, &catalog, &state, &mut time, "Actions for folder");
        assert!(matches!(menu, Some(Intent::FolderMenu(1, _))), "{menu:?}");
        for (label, want) in [
            ("Open folder", Intent::Files(Request::Browse(Location { parent: Some(1), ..location }))),
            ("Play folder", Intent::PlayEntryFolder(1)),
            ("Shuffle folder", Intent::ShuffleEntryFolder(1)),
        ] {
            state.apply(menu.unwrap());
            settle(&mut ui, &state, &mut time);
            assert_eq!(click_at(size, &mut ui, &files, &catalog, &state, &mut time, label), Some(want), "{label}");
            state.apply(Intent::CloseFolderMenu);
            settle(&mut ui, &state, &mut time);
        }
        // Select mode: header, checkboxes on saved songs, the deck acts on the marked songs.
        assert_eq!(click_at(size, &mut ui, &files, &catalog, &state, &mut time, "Select"), Some(Intent::SelectMode(true)));
        state.apply(Intent::SelectMode(true));
        settle(&mut ui, &state, &mut time);
        let seen = names(&ui);
        for want in ["Select all", "Done", "Play next", "Add to list", "Offline", "Queue", "0 selected"] {
            assert!(seen.iter().any(|name| name == want), "missing {want}: {seen:?}");
        }
        assert!(!seen.iter().any(|name| name == "Actions for folder"), "menus hide while selecting");
        // ruling 3 (U4b): the list toggles a saved song's row a tap (`SelectMode::Toggle`); a folder's row takes none while selecting
        assert_eq!(click_at(size, &mut ui, &files, &catalog, &state, &mut time, "Takes"), None);
        let tap = click_at(size, &mut ui, &files, &catalog, &state, &mut time, "02 There.flac");
        assert_eq!(tap, Some(Intent::Selected(2)));
        state.apply(tap.unwrap());
        settle(&mut ui, &state, &mut time);
        assert!(names(&ui).iter().any(|name| name == "1 selected"));
        assert_eq!(state.selected_folder_songs(files.directory.as_ref().unwrap()), vec![10]);
        assert_eq!(click_at(size, &mut ui, &files, &catalog, &state, &mut time, "Play next"), Some(Intent::Page(crate::PageAction::PlayNext)));
        assert_eq!(click_at(size, &mut ui, &files, &catalog, &state, &mut time, "Queue"), Some(Intent::Page(crate::PageAction::AddToQueue)));
        let lists = click_at(size, &mut ui, &files, &catalog, &state, &mut time, "Add to list");
        assert!(matches!(lists, Some(Intent::PageListChoices(_))), "{lists:?}");
        // Another folder leaves Select mode.
        state.apply(Intent::Files(Request::Browse(Location { parent: Some(1), ..location })));
        assert!(!state.selecting() && (state.marked_count() == 0));
        state.apply(Intent::SelectMode(true));
        settle(&mut ui, &state, &mut time);
        for round in 0..5 {
            mem_sys::local_reset();
            for _ in 0..10 {
                frame_at(size, &mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
            }
            if round >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm folder select frame allocates");
            }
        }
        // the folder list's disabled rows stay on the Files page (the songs lists share only the marks): a plain file's row, shown
        // with Show non-audio files on outside Select mode, goes with the selection a navigation clears
        state.apply(Intent::SelectMode(false));
        state.song_rows.borrow_mut().rows = vec![0, pxui::ListState::DISABLED];
        state.apply(Intent::Navigate(Route::Songs));
        assert!(!state.song_rows.borrow().rows.iter().any(|f| f & pxui::ListState::DISABLED != 0), "the songs lists never inherit disabled rows");
    }

    #[test]
    fn compact_folder_tabs_filter_rows_tap_plays_and_the_plus_opens_the_add_music_sheet() {
        use lw_core::FileAvailability::{InCloud, OnDevice};
        mem_sys::set_peer(pxui::library_counter);
        let size = (390.0, 778.0);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let location = Location { user: 42, source: 1, parent: Some(5) };
        let glyph = |availability| Some(crate::files::Glyph { state: lw_core::FileState::Copied, availability: Some(availability), pinned: false });
        let audio = |id: i64, name: &str, song: Option<i64>, availability| Entry {
            id, name: name.into(), detail: "FLAC · 1.0 MB · 2:29".into(), audio: true, song,
            glyph: song.and_then(|_| glyph(availability)),
            number: id.to_string(), ..Default::default()
        };
        let directory = Directory {
            location, source_root: 5, up: Some(4), title: "Field".into(), detail: "4 items".into(), songs: Arc::from([9_i64, 10]),
            entries: vec![
                Entry { id: 1, name: "Takes".into(), detail: "Folder".into(), directory: true, ..Default::default() },
                audio(2, "01 Here.flac", Some(9), OnDevice),
                audio(3, "02 Cloud.flac", Some(10), InCloud),
                audio(4, "03 New.flac", None, OnDevice),
            ],
            filters: [vec![0, 1], vec![0, 2]],
            filter_counts: ["3".into(), "1".into(), "1".into()],
            ..Default::default()
        };
        let files = Snapshot { revision: 1, account: Some(42), directory: Some(Arc::new(directory)), ..Default::default() };
        let (catalog, mut state, mut time) = (Catalog::default(), ShellState::default(), 0.0);
        state.apply(Intent::Navigate(Route::Files));
        for _ in 0..3 {
            frame_at(size, &mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        let seen = names(&ui);
        for want in ["All", "Offline", "In cloud", "01 Here.flac", "02 Cloud.flac", "03 New.flac"] {
            assert!(seen.iter().any(|name| name == want), "missing {want}: {seen:?}");
        }
        assert_eq!(click_at(size, &mut ui, &files, &catalog, &state, &mut time, "01 Here.flac"), Some(Intent::PlayFile(9)));
        assert_eq!(
            click_at(size, &mut ui, &files, &catalog, &state, &mut time, "03 New.flac"),
            Some(Intent::Files(Request::ImportPath { user: 42, source: 1, path: 4 })),
            "an unsaved file imports on tap"
        );
        let more = click_at(size, &mut ui, &files, &catalog, &state, &mut time, "Actions for file");
        assert!(matches!(more, Some(Intent::SongMenu(_, 9, 0, _))), "{more:?}");
        let tab = click_at(size, &mut ui, &files, &catalog, &state, &mut time, "In cloud");
        assert_eq!(tab, Some(Intent::FilesFilter(crate::FilesFilter::Cloud)));
        state.apply(tab.unwrap());
        for _ in 0..3 {
            frame_at(size, &mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
        }
        let seen = names(&ui);
        assert!(seen.iter().any(|name| name == "02 Cloud.flac") && seen.iter().any(|name| name == "Takes"), "{seen:?}");
        assert!(!seen.iter().any(|name| name == "01 Here.flac" || name == "03 New.flac"), "{seen:?}");
        // A folder row's song menu plays inside the folder's context.
        let library = crate::tests::prepared(vec![lw_core::Song { id: Some(9), title: "Here".into(), ..Default::default() }]);
        state.apply(Intent::SongMenu(0, 9, 0, (100.0, 100.0)));
        for _ in 0..3 {
            frame_at(size, &mut ui, &files, &library, &state, &mut time, &[], &mut None);
        }
        let play = ui.a11y_nodes().iter().find(|node| node.is_role(Role::MenuItem) && ui.a11y_name_of(node) == "Play").unwrap().id;
        ui.inject_a11y(&[pxui::A11yAction { id: play, action_code: 1, ..Default::default() }]);
        let mut menu = None;
        frame_at(size, &mut ui, &files, &library, &state, &mut time, &[], &mut menu);
        assert_eq!(menu, Some(Intent::PlayFile(9)));
        state.apply(Intent::CloseSongMenu);
        state.apply(Intent::Files(Request::Roots));
        assert_eq!(state.files_filter, crate::FilesFilter::All, "changing folder resets the tab");
        for round in 0..5 {
            mem_sys::local_reset();
            for _ in 0..10 {
                frame_at(size, &mut ui, &files, &catalog, &state, &mut time, &[], &mut None);
            }
            if round >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm filtered folder page allocates");
            }
        }
        // The Folders "+" opens the sheet; its two choices start the pickers.
        let roots = Snapshot { revision: 2, account: Some(42), ..Default::default() };
        for _ in 0..3 {
            frame_at(size, &mut ui, &roots, &catalog, &state, &mut time, &[], &mut None);
        }
        let plus = click_at(size, &mut ui, &roots, &catalog, &state, &mut time, "Add music");
        assert_eq!(plus, Some(Intent::ShowAddMusic));
        state.apply(Intent::ShowAddMusic);
        for _ in 0..3 {
            frame_at(size, &mut ui, &roots, &catalog, &state, &mut time, &[], &mut None);
        }
        assert_eq!(click_at(size, &mut ui, &roots, &catalog, &state, &mut time, "A folder"), Some(Intent::ChooseFolder));
        assert_eq!(click_at(size, &mut ui, &roots, &catalog, &state, &mut time, "Some files"), Some(Intent::ChooseAudio));
        state.apply(Intent::CloseSourceMenu);
        assert!(!state.add_music);
    }
}
