use super::*;
use crate::files::{Folder, Location, Request, Target};

fn options(ui: &mut Ui, target: Target, p: Palette, intent: &mut Option<Intent>) {
    let button = button(ui, "source-options", "Folder options", 118.0, true, p);
    if ui.signal_state(&button).clicked {
        let at = ui.rect(&button);
        *intent = Some(Intent::SourceMenu(target, (at.x, at.y + at.h)));
    }
}

pub(super) fn popup(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    target: Target,
    at: (f32, f32),
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let (width, height) = ui.view_size();
    let at = (
        at.0.clamp(0.0, (width - 220.0).max(0.0)),
        at.1.clamp(0.0, (height - 144.0).max(0.0)),
    );
    let busy = player.files.is_none_or(|files| files.busy);
    let present = player.files.is_some_and(|files| {
        files.account == Some(target.user)
            && files
                .sources
                .iter()
                .any(|folder| folder.id == target.source && folder.root == target.root)
    });
    let mut accepted = false;
    let open = ui.popup(
        Some(p.line),
        None,
        false,
        false,
        at,
        220.0,
        144.0,
        "Folder options",
        &mut |ui| {
            for (index, label) in [
                "Open folder",
                "Rescan now",
                "Reconnect folder…",
                "Remove source",
            ]
            .into_iter()
            .enumerate()
            {
                let enabled = present && (index == 0 || !busy);
                let item = ui.node(
                    label,
                    Node {
                        flags: if enabled { CLICKABLE | FOCUSABLE } else { 0 },
                        role: Role::MenuItem,
                        w: Size::Grow,
                        h: Size::Px(36.0),
                        padding: Edges {
                            l: 12.0,
                            r: 12.0,
                            ..Edges::ZERO
                        },
                        look: Look {
                            fill: p.paper,
                            text: if !enabled {
                                p.secondary
                            } else if index == 3 {
                                p.danger
                            } else {
                                p.ink
                            },
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
                ui.set_role(
                    &item,
                    Role::MenuItem,
                    if enabled {
                        pxui::a11y::state::ENABLED
                    } else {
                        0
                    },
                );
                if enabled && ui.signal_state(&item).clicked {
                    let location = Location {
                        user: target.user,
                        source: target.source,
                        parent: None,
                    };
                    *intent = Some(match index {
                        0 => Intent::Files(Request::Browse(location)),
                        1 => Intent::Files(Request::Rescan(location)),
                        2 => Intent::ReconnectSource(target),
                        _ => Intent::ConfirmRemoveSource(target),
                    });
                    accepted = true;
                }
            }
        },
    );
    if accepted {
        ui.dismiss_popup();
    }
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseSourceMenu);
    }
}

pub(super) fn confirm_remove(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    target: Target,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let (width, _) = ui.view_size();
    let width = (width - 32.0).clamp(240.0, 420.0);
    let folder = player.files.and_then(|files| {
        files.sources.iter().find(|folder| {
            files.account == Some(target.user)
                && folder.id == target.source
                && folder.root == target.root
        })
    });
    let enabled = folder.is_some() && player.files.is_some_and(|files| !files.busy);
    let open = ui.modal(Some(width), "Remove folder?", &mut |ui| {
        let body = ui.node(
            "remove-source-body",
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
                "Remove folder?",
                24,
                p.ink,
                (Size::Grow, 36.0),
                false,
            );
            settings::paragraph(
                ui,
                "folder-name",
                folder.map_or(
                    "This folder changed. Reload folders and try again.",
                    |folder| folder.name.as_str(),
                ),
                width - 40.0,
                15.0,
                p.ink,
            );
            settings::paragraph(
                ui,
                "consequences",
                "Its songs leave the library. Files are not deleted.",
                width - 40.0,
                14.0,
                p.secondary,
            );
            let buttons = container(
                ui,
                "remove-source-actions",
                Axis::Row,
                Size::Grow,
                Size::Px(40.0),
                p.paper,
            );
            ui.within(&buttons, &mut |ui| {
                if action(ui, "cancel-remove", "Cancel", 100.0, true, p) {
                    *intent = Some(Intent::CloseRemoveSource);
                }
                // Swift's default action: Return confirms.
                if action(
                    ui,
                    "remove-confirm",
                    "Remove",
                    100.0,
                    enabled,
                    Palette { ink: p.danger, ..p },
                ) || (enabled && ui.key_pressed(input::Key::Enter.code()))
                {
                    *intent = Some(Intent::Files(Request::Remove(target)));
                }
            });
        });
    });
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseRemoveSource);
    }
}

/// Back (button and ⌘[): the parent folder, or the source list from a source root.
pub(super) fn up(directory: &crate::files::Directory) -> Intent {
    let location = directory.location;
    Intent::Files(directory.up.map_or(Request::Roots, |parent| {
        Request::Browse(Location { parent: Some(parent), ..location })
    }))
}

fn button(ui: &mut Ui, key: &str, label: &str, width: f32, enabled: bool, p: Palette) -> Handle {
    let height = if ui.view_size().0 < compact::BREAKPOINT { 44.0 } else { 36.0 };
    sized_button(ui, key, label, width, height, enabled, p)
}
fn sized_button(ui: &mut Ui, key: &str, label: &str, width: f32, height: f32, enabled: bool, p: Palette) -> Handle {
    let button = ui.node(
        key,
        Node {
            flags: if enabled { CLICKABLE | FOCUSABLE } else { 0 },
            role: Role::Button,
            w: Size::Px(width),
            h: Size::Px(height),
            look: Look {
                fill: p.panel,
                text: if enabled { p.ink } else { p.secondary },
                font: Font {
                    size: 13,
                    ..Font::BODY
                },
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.text(&button, label);
    ui.a11y_name(&button, label);
    ui.set_role(
        &button,
        Role::Button,
        if enabled {
            pxui::a11y::state::ENABLED
        } else {
            0
        },
    );
    button
}
pub(super) fn action(
    ui: &mut Ui,
    key: &str,
    label: &str,
    width: f32,
    enabled: bool,
    p: Palette,
) -> bool {
    let button = button(ui, key, label, width, enabled, p);
    ui.signal_state(&button).clicked && enabled
}

/// Swift's plain secondary text link (Back).
fn link(ui: &mut Ui, key: &str, label: &str, width: f32, p: Palette) -> bool {
    let height = if ui.view_size().0 < compact::BREAKPOINT { 44.0 } else { 40.0 };
    let link = ui.node(
        key,
        Node {
            flags: CLICKABLE | FOCUSABLE,
            role: Role::Button,
            w: Size::Px(width),
            h: Size::Px(height),
            look: Look { text: p.secondary, font: Font { size: 14, ..Font::BODY }, ..Look::NONE },
            ..Node::DEFAULT
        },
    );
    ui.text(&link, label);
    ui.a11y_name(&link, key);
    ui.signal_state(&link).clicked
}

/// A table cell: fixed `Some(width)` or a weighted share of the remaining row.
fn cell(ui: &mut Ui, key: &str, fixed: Option<f32>, share: f32, end: bool, build: &mut dyn FnMut(&mut Ui)) {
    let node = ui.node(
        key,
        Node {
            axis: Axis::Row,
            align: Align::Center,
            justify: if end { Justify::End } else { Justify::Start },
            w: fixed.map_or(Size::Grow, Size::Px),
            h: Size::Grow,
            grow: share,
            padding: Edges { l: 8.0, r: if end { 20.0 } else { 8.0 }, ..Edges::ZERO },
            ..Node::DEFAULT
        },
    );
    ui.within(&node, build);
}

/// Swift's folder/file marks in the # column, drawn as outlines.
fn mark(ui: &mut Ui, directory: bool, color: Rgba) {
    let node = ui.node("mark", Node { w: Size::Px(16.0), h: Size::Px(16.0), ..Node::DEFAULT });
    ui.draw(&node, move |paint, rect| {
        let (x, y, w, h) = if directory { (rect.x + 1.0, rect.y + 3.0, 14.0, 10.0) } else { (rect.x + 3.0, rect.y + 1.0, 10.0, 14.0) };
        paint.fill_fractional(x, y, w, 1.5, color);
        paint.fill_fractional(x, y + h - 1.5, w, 1.5, color);
        paint.fill_fractional(x, y, 1.5, h, color);
        paint.fill_fractional(x + w - 1.5, y, 1.5, h, color);
    });
}

const ADD_STEPS: [&str; 3] = [
    "Choose a folder in Files",
    "LocalWave indexes its audio files",
    "Play your music from the library",
];

/// A column of the folder table: header label, fixed width or weighted share, right alignment.
const COLUMNS: [(&str, Option<f32>, f32, bool); 5] = [
    ("Name", None, 2.2, false),
    ("Kind", None, 1.5, false),
    ("Size", None, 1.0, false),
    ("On device", None, 1.2, false),
    ("Time", Some(84.0), 0.0, true),
];
const NUMBER: f32 = 56.0;

pub(super) fn view(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let compact = ui.view_size().0 < compact::BREAKPOINT;
    let directory = player.files.and_then(|files| files.directory.as_ref());
    // Swift's source list pads 16 points; its folder page 32.
    let inset = if compact || directory.is_none() { 16.0 } else { 32.0 };
    let root = ui.node(
        "files-page",
        Node {
            flags: if directory.is_none() { SCROLL | CLIP } else { 0 },
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges { l: inset, r: inset, t: if directory.is_none() { if compact { 6.0 } else { 10.0 } } else { 0.0 }, b: 12.0 },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let Some(files) = player.files else {
            text(ui, "loading", "Loading folders…", 18, p.secondary, (Size::Grow, 40.0), false);
            return;
        };
        let content = (width - inset * 2.0).max(1.0);
        match &files.directory {
            Some(directory) if files.account.is_some() => folder_page(ui, files, directory, content, compact, p, intent),
            _ => source_page(ui, files, content, compact, p, intent),
        }
    });
}

/// Worker progress and the last message, shared by both pages.
fn status(ui: &mut Ui, files: &crate::files::Snapshot, width: f32, compact: bool, p: Palette, intent: &mut Option<Intent>) {
    if files.busy {
        let progress = container(ui, "files-progress", Axis::Row, Size::Grow, Size::Px(if compact { 44.0 } else { 40.0 }), p.panel);
        ui.within(&progress, &mut |ui| {
            let label = text(ui, "progress-label", &files.progress, 13, p.ink, (Size::Grow, 32.0), true);
            ui.set_role(&label, Role::Progress, 0);
            if action(ui, "cancel", "Cancel", 80.0, true, p) {
                *intent = Some(Intent::Files(Request::Cancel));
            }
        });
    }
    if !files.message.is_empty() {
        settings::paragraph(ui, "files-message", &files.message, width, 13.0, p.secondary);
    }
}

fn source_page(ui: &mut Ui, files: &crate::files::Snapshot, width: f32, compact: bool, p: Palette, intent: &mut Option<Intent>) {
    let enabled = files.account.is_some() && !files.busy;
    let heading = ui.node(
        "files-heading",
        Node { axis: Axis::Row, w: Size::Grow, h: Size::Fit, margin: Edges { b: 16.0, ..Edges::ZERO }, ..Node::DEFAULT },
    );
    ui.within(&heading, &mut |ui| {
        let titles = ui.node("files-titles", Node { w: Size::Grow, h: Size::Fit, gap: 4.0, ..Node::DEFAULT });
        ui.within(&titles, &mut |ui| {
            text(ui, "title", "Folders", 32, p.ink, (Size::Grow, 44.0), false);
            text(ui, "files-detail", &files.source_count, 13, p.secondary, (Size::Grow, 18.2), false);
        });
        if compact {
            // Swift's "+" opens an Add music sheet; its two choices are the picker intents.
            let add = ui.node("add-music", Node { flags: if enabled { CLICKABLE | FOCUSABLE } else { 0 }, role: Role::Button, w: Size::Px(44.0), h: Size::Px(44.0), ..Node::DEFAULT });
            ui.a11y_name(&add, "Add music");
            let color = p.ink;
            ui.draw(&add, move |paint, rect| {
                paint.fill(rect.x + 10.0, rect.y + 21.25, 24.0, 1.5, color);
                paint.fill(rect.x + 21.25, rect.y + 10.0, 1.5, 24.0, color);
            });
            if enabled && ui.signal_state(&add).clicked {
                *intent = Some(Intent::ChooseFolder);
            }
        } else {
            if action(ui, "add-folder", "Add folder", 110.0, enabled, p) {
                *intent = Some(Intent::ChooseFolder);
            }
            if action(ui, "add-audio", "Add audio files", 130.0, enabled, p) {
                *intent = Some(Intent::ChooseAudio);
            }
            if action(ui, "refresh", "Refresh folders", 130.0, files.account.is_some(), p) {
                *intent = Some(Intent::Files(Request::Roots));
            }
        }
    });
    status(ui, files, width, compact, p, intent);
    let Some(user) = files.account else {
        if files.message.is_empty() {
            text(ui, "files-loading", "Loading folders…", 14, p.secondary, (Size::Grow, 40.0), false);
        }
        return;
    };
    if files.sources.is_empty() {
        text(ui, "empty-heading", "Point it at a folder.", 28, p.ink, (Size::Grow, 48.0), false);
        settings::paragraph(ui, "empty-description", "Your files stay where they are. Nothing is uploaded.", width, 14.0, p.secondary);
        for (index, step) in ADD_STEPS.iter().enumerate() {
            let row = ui.node_keyed(int_key(Ui::key_of(0, "files-step"), index as i64), Node { axis: Axis::Row, align: Align::Center, gap: 16.0, w: Size::Grow, h: Size::Px(44.0), ..Node::DEFAULT });
            ui.within(&row, &mut |ui| {
                text(ui, "number", &["1", "2", "3"][index], 13, p.secondary, (Size::Px(12.0), 18.2), true);
                text(ui, "step", step, 15, p.ink, (Size::Grow, 24.0), false);
            });
            ui.draw_over(&row, move |paint, rect| paint.fill_fractional(rect.x, rect.y + rect.h - 1.0, rect.w, 1.0, p.line));
        }
    }
    for folder in files.sources.iter() {
        source_card(ui, user, folder, width, p, intent);
    }
    let offline = ui.node("offline-space", Node { w: Size::Grow, h: Size::Fit, margin: Edges { t: 16.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&offline, &mut |ui| {
        rule(ui, "offline-rule", false, p);
        spacer(ui, "offline-gap", 16.0);
        let label = container(ui, "offline-label", Axis::Row, Size::Grow, Size::Px(19.6), p.paper);
        ui.within(&label, &mut |ui| {
            text(ui, "label", "Offline space", 14, p.ink, (Size::Grow, 19.6), false);
            text(ui, "usage", &files.offline_usage, 12, p.ink, (Size::Fit, 16.8), true);
        });
        let meter = ui.node("offline-meter", Node { axis: Axis::Row, w: Size::Grow, h: Size::Px(4.0), margin: Edges { t: 10.0, ..Edges::ZERO }, look: Look { fill: p.line, ..Look::NONE }, ..Node::DEFAULT });
        ui.within(&meter, &mut |ui| {
            container(ui, "fill", Axis::Column, Size::Pct(files.offline_fraction), Size::Grow, p.ink);
        });
        spacer(ui, "offline-note-gap", 8.0);
        settings::paragraph(ui, "offline-note", "Cloud songs download when played and the next two download ahead. Pinned folders stay.", width, 13.0, p.secondary);
    });
}

fn source_card(ui: &mut Ui, user: i64, folder: &Folder, width: f32, p: Palette, intent: &mut Option<Intent>) {
    let target = Target { user, source: folder.id, root: folder.root };
    let error = folder.error.is_some();
    let card = ui.node_keyed(
        int_key(Ui::key_of(0, "files-source"), folder.id),
        Node {
            flags: CLICKABLE | FOCUSABLE,
            role: if error { Role::Group } else { Role::Button },
            w: Size::Grow,
            h: Size::Fit,
            margin: Edges { b: 8.0, ..Edges::ZERO },
            padding: Edges::all(17.0),
            look: Look { fill: p.paper, border: if error { p.ink } else { p.line }, border_w: 1.0, ..Look::NONE },
            ..Node::DEFAULT
        },
    );
    ui.a11y_name(&card, &folder.name);
    ui.within(&card, &mut |ui| {
        let top = container(ui, "source-top", Axis::Row, Size::Grow, Size::Px(44.0), p.paper);
        ui.within(&top, &mut |ui| {
            let names = ui.node("source-labels", Node { w: Size::Grow, h: Size::Grow, gap: 2.0, ..Node::DEFAULT });
            ui.within(&names, &mut |ui| {
                text(ui, "folder-name", &folder.name, 17, p.ink, (Size::Grow, 23.8), false);
                text(ui, "folder-path", &folder.path, 13, p.secondary, (Size::Grow, 18.2), false);
            });
            text(ui, "folder-count", &folder.count, 13, p.secondary, (Size::Fit, 18.2), false);
        });
        if let Some(message) = &folder.error {
            spacer(ui, "error-gap", 12.0);
            let row = ui.node("source-error", Node { axis: Axis::Row, align: Align::Center, gap: 8.0, w: Size::Grow, h: Size::Px(20.0), ..Node::DEFAULT });
            ui.within(&row, &mut |ui| {
                super::songs::row::glyph(ui, (lw_core::FileState::Failed, Some(lw_core::FileAvailability::Unknown), false), true, p);
                text(ui, "message", message, 13, p.ink, (Size::Grow, 18.9), false);
            });
            spacer(ui, "buttons-gap", 12.0);
            let buttons = ui.node("source-buttons", Node { axis: Axis::Row, gap: 2.0, w: Size::Grow, h: Size::Px(44.0), ..Node::DEFAULT });
            ui.within(&buttons, &mut |ui| {
                let half = ((width - 34.0 - 2.0) / 2.0).floor().max(1.0);
                let reconnect = sized_button(ui, "reconnect", "Reconnect folder", half, 44.0, true, Palette { panel: p.ink, ink: p.paper, ..p });
                if ui.signal_state(&reconnect).clicked {
                    *intent = Some(Intent::ReconnectSource(target));
                }
                let options = sized_button(ui, "source-options", "Options", half, 44.0, true, p);
                if ui.signal_state(&options).clicked {
                    let at = ui.rect(&options);
                    *intent = Some(Intent::SourceMenu(target, (at.x, at.y + at.h)));
                }
            });
        } else {
            let bar = ui.node("source-bar", Node { axis: Axis::Row, w: Size::Grow, h: Size::Px(4.0), margin: Edges { t: 14.0, b: 10.0, ..Edges::ZERO }, look: Look { fill: p.line, ..Look::NONE }, ..Node::DEFAULT });
            ui.within(&bar, &mut |ui| {
                container(ui, "fill", Axis::Column, Size::Pct(folder.local), Size::Grow, p.ink);
            });
            let footer = container(ui, "source-footer", Axis::Row, Size::Grow, Size::Px(18.2), p.paper);
            ui.within(&footer, &mut |ui| {
                text(ui, "stats", &folder.stats, 13, p.secondary, (Size::Grow, 18.2), false);
                text(ui, "scanned", &folder.scanned, 13, p.secondary, (Size::Fit, 18.2), false);
            });
        }
    });
    let signal = ui.signal_state(&card);
    if signal.pressed && signal.button == Some(1) {
        *intent = Some(Intent::SourceMenu(target, signal.pointer));
    } else if !error && intent.is_none() && signal.clicked {
        *intent = Some(Intent::Files(Request::Browse(Location { user, source: folder.id, parent: None })));
    }
}

fn folder_page(ui: &mut Ui, files: &crate::files::Snapshot, directory: &crate::files::Directory, width: f32, compact: bool, p: Palette, intent: &mut Option<Intent>) {
    use super::find::{Action, button as tool};
    if compact {
        return compact_folder(ui, files, directory, width, p, intent);
    }
    let location = directory.location;
    let target = Target { user: location.user, source: location.source, root: directory.source_root };
    let top = ui.node("folder-top", Node { axis: Axis::Row, wrap: compact, align: Align::Center, w: Size::Grow, h: Size::Fit, padding: Edges { t: 16.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&top, &mut |ui| {
        if link(ui, "Back", "‹ Back", 64.0, p) {
            *intent = Some(up(directory));
        }
        ui.box_("top-space", BoxOpts::default().w(Size::Grow).h(Size::Px(1.0)));
        if action(ui, "import", "Import folder", 108.0, !files.busy, p) {
            *intent = Some(Intent::Files(Request::Import(location)));
        }
        options(ui, target, p, intent);
    });
    if let Some(message) = &directory.error {
        let row = ui.node("folder-error", Node { axis: Axis::Row, align: Align::Center, gap: 12.0, w: Size::Grow, h: Size::Px(56.0), margin: Edges { t: 20.0, ..Edges::ZERO }, padding: Edges { l: 16.0, r: 16.0, ..Edges::ZERO }, look: Look { border: p.ink, border_w: 1.5, ..Look::NONE }, ..Node::DEFAULT });
        ui.within(&row, &mut |ui| {
            super::songs::row::glyph(ui, (lw_core::FileState::Failed, Some(lw_core::FileAvailability::Unknown), false), true, p);
            text(ui, "message", message, 14, p.ink, (Size::Grow, 20.0), false);
            if action(ui, "reconnect", "Reconnect folder", 140.0, !files.busy, Palette { panel: p.ink, ink: p.paper, ..p }) {
                *intent = Some(Intent::ReconnectSource(target));
            }
        });
    }
    let header = ui.node("folder-header", Node { w: Size::Grow, h: Size::Fit, padding: Edges { t: 28.0, b: 18.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&header, &mut |ui| {
        if !directory.breadcrumb.is_empty() {
            text(ui, "breadcrumb", &directory.breadcrumb, 14, p.secondary, (Size::Grow, 19.6), false);
            spacer(ui, "breadcrumb-gap", 8.0);
        }
        text(ui, "title", &directory.title, 34, p.ink, (Size::Grow, 36.0), false);
        spacer(ui, "summary-gap", 6.0);
        text(ui, "summary", &directory.summary, 14, p.secondary, (Size::Grow, 19.6), false);
    });
    let tools = ui.node("folder-tools", Node { axis: Axis::Row, wrap: compact, align: Align::Center, gap: 8.0, w: Size::Grow, h: Size::Fit, padding: Edges { b: 15.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&tools, &mut |ui| {
        let ready = !directory.songs.is_empty() && !files.busy;
        if tool(ui, Action::Folder, ready, p) {
            *intent = Some(Intent::PlayFolder);
        }
        if tool(ui, Action::Shuffle, ready, p) {
            *intent = Some(Intent::ShuffleFolder);
        }
        if tool(ui, Action::Offline(directory.kept), ready, p) {
            *intent = Some(Intent::KeepFolder);
        }
        ui.box_("tools-space", BoxOpts::default().w(Size::Grow).h(Size::Px(1.0)));
        if !compact {
            legend(ui, p);
        }
    });
    ui.draw_over(&tools, move |paint, rect| paint.fill_fractional(rect.x - 32.0, rect.y + rect.h - 1.0, rect.w + 64.0, 1.0, p.line));
    status(ui, files, width, compact, p, intent);
    if directory.entries.is_empty() {
        text(ui, "empty-directory", "Nothing here.", 14, p.secondary, (Size::Grow, 48.0), false);
        return;
    }
    let head = ui.node("folder-columns", Node { axis: Axis::Row, w: Size::Grow, h: Size::Px(38.0), ..Node::DEFAULT });
    ui.within(&head, &mut |ui| {
        cell(ui, "number", Some(NUMBER), 0.0, false, &mut |ui| {
            text(ui, "label", "#", 12, p.secondary, (Size::Fit, 16.8), false);
        });
        for (label, fixed, share, end) in COLUMNS {
            if compact && matches!(label, "Kind" | "Size") {
                continue;
            }
            cell(ui, label, fixed, share, end, &mut |ui| {
                text(ui, "label", label, 12, p.secondary, (Size::Fit, 16.8), false);
            });
        }
    });
    ui.draw_over(&head, move |paint, rect| paint.fill_fractional(rect.x, rect.y + rect.h - 1.0, rect.w, 1.0, p.line));
    // The parent participates in the identity, giving each destination its own scroll state.
    let key = int_key(int_key(Ui::key_of(0, "files-directory"), location.source), location.parent.unwrap_or(0));
    let list = ui.node_keyed(key, Node { w: Size::Grow, h: Size::Grow, ..Node::DEFAULT });
    // Desktop lists every file greyed (Swift); the compact list honours Show non-audio files.
    let shown = directory.entries.len() - if compact && !directory.show_non_audio { directory.non_audio } else { 0 };
    ui.within(&list, &mut |ui| {
        ui.grid_list("directory-entries", shown, width, 38.0, &mut |ui, index| {
            let entry = &directory.entries[index];
            let other = !entry.directory && !entry.audio;
            let color = if other { p.tertiary } else { p.secondary };
            let ink = if other { p.tertiary } else { p.ink };
            let row = ui.node_keyed(
                int_key(Ui::key_of(0, "files-entry"), entry.id),
                Node { axis: Axis::Row, w: Size::Grow, h: Size::Px(38.0), flags: if entry.directory { CLICKABLE | FOCUSABLE } else { 0 }, role: if entry.directory { Role::Button } else { Role::Row }, ..Node::DEFAULT },
            );
            ui.a11y_name(&row, &entry.name);
            let signal = ui.signal_state(&row);
            ui.within(&row, &mut |ui| {
                cell(ui, "number", Some(NUMBER), 0.0, false, &mut |ui| {
                    if entry.audio {
                        text(ui, "digits", &entry.number, 13, color, (Size::Fit, 18.2), false);
                    } else {
                        mark(ui, entry.directory, color);
                    }
                });
                cell(ui, "Name", None, COLUMNS[0].2, false, &mut |ui| {
                    text(ui, "name", &entry.name, 14, ink, (Size::Grow, 19.6), false);
                });
                if !compact {
                    cell(ui, "Kind", None, COLUMNS[1].2, false, &mut |ui| {
                        text(ui, "kind", &entry.kind, 14, color, (Size::Grow, 19.6), false);
                    });
                    cell(ui, "Size", None, COLUMNS[2].2, false, &mut |ui| {
                        text(ui, "size", &entry.size, 12, color, (Size::Grow, 16.8), true);
                    });
                }
                cell(ui, "On device", None, COLUMNS[3].2, false, &mut |ui| match entry.glyph {
                    Some(glyph) => super::songs::row::glyph(ui, (glyph.state, glyph.availability, glyph.pinned), true, p),
                    None if entry.audio => {
                        // Not imported yet: Swift imports on play; here the row offers Import.
                        if action(ui, "import-file", "Import", 72.0, !files.busy, p) {
                            *intent = Some(Intent::Files(Request::ImportPath { user: location.user, source: location.source, path: entry.id }));
                        }
                    }
                    None => {}
                });
                cell(ui, "Time", COLUMNS[4].1, 0.0, true, &mut |ui| {
                    text(ui, "time", &entry.time, 12, color, (Size::Fit, 16.8), true);
                });
            });
            ui.draw_over(&row, move |paint, rect| paint.fill_fractional(rect.x, rect.y + rect.h - 1.0, rect.w, 1.0, p.line));
            if signal.clicked && entry.directory && intent.is_none() {
                *intent = Some(Intent::Files(Request::Browse(Location { parent: Some(entry.id), ..location })));
            }
        });
    });
}

/// Swift's mobileFolder: back and item count, title with options, breadcrumb, Play folder / Shuffle / Keep offline
/// keys and 56 pt file rows. Gaps: no Select mode, availability filter tabs or per-row menus; audio rows do not play.
fn compact_folder(ui: &mut Ui, files: &crate::files::Snapshot, directory: &crate::files::Directory, width: f32, p: Palette, intent: &mut Option<Intent>) {
    use super::compact::list::key;
    use super::kit::{Icon, paint_icon};
    let location = directory.location;
    let target = Target { user: location.user, source: location.source, root: directory.source_root };
    let top = ui.node("folder-top", Node { axis: Axis::Row, align: Align::Center, w: Size::Grow, h: Size::Px(44.0), ..Node::DEFAULT });
    ui.within(&top, &mut |ui| {
        let back = ui.node("Back", Node { flags: CLICKABLE | FOCUSABLE, role: Role::Button, axis: Axis::Row, align: Align::Center, gap: 4.0, w: Size::Fit, h: Size::Px(44.0), ..Node::DEFAULT });
        ui.a11y_name(&back, "Back");
        ui.within(&back, &mut |ui| {
            let glyph = ui.node("icon", Node { w: Size::Px(20.0), h: Size::Px(20.0), ..Node::DEFAULT });
            let color = p.secondary;
            ui.draw(&glyph, move |paint, rect| paint_icon(paint, Icon::Back, rect.x, rect.y, color));
            text(ui, "label", if directory.up.is_some() { "Back" } else { "Folders" }, 15, p.secondary, (Size::Fit, 21.0), false);
        });
        if ui.signal_state(&back).clicked {
            *intent = Some(up(directory));
        }
        ui.box_("top-space", BoxOpts::default().w(Size::Grow).h(Size::Px(1.0)));
        text(ui, "count", &directory.detail, 12, p.secondary, (Size::Fit, 16.8), true);
    });
    let titles = ui.node("folder-titles", Node { axis: Axis::Row, align: Align::Center, w: Size::Grow, h: Size::Px(48.0), ..Node::DEFAULT });
    ui.within(&titles, &mut |ui| {
        let name = ui.node("name", Node { w: Size::Grow, h: Size::Fit, ..Node::DEFAULT });
        ui.within(&name, &mut |ui| { super::detail::line(ui, "title", &directory.title, (32.0, 33.6, 500, -1.12), p.ink, 0); });
        let more = ui.node("source-options", Node { flags: CLICKABLE | FOCUSABLE, role: Role::Button, w: Size::Px(44.0), h: Size::Px(44.0), ..Node::DEFAULT });
        ui.a11y_name(&more, "Folder options");
        let color = p.ink;
        ui.draw(&more, move |paint, rect| paint_icon(paint, Icon::More, rect.x + 12.0, rect.y + 12.0, color));
        if ui.signal_state(&more).clicked {
            let at = ui.rect(&more);
            *intent = Some(Intent::SourceMenu(target, (at.x, at.y + at.h)));
        }
    });
    if !directory.breadcrumb.is_empty() {
        text(ui, "breadcrumb", &directory.breadcrumb, 13, p.secondary, (Size::Grow, 24.0), false);
    }
    if let Some(message) = &directory.error {
        let row = ui.node("folder-error", Node { axis: Axis::Row, align: Align::Center, gap: 8.0, w: Size::Grow, h: Size::Px(56.0), margin: Edges { t: 8.0, ..Edges::ZERO }, padding: Edges { l: 12.0, r: 12.0, ..Edges::ZERO }, look: Look { border: p.ink, border_w: 1.5, ..Look::NONE }, ..Node::DEFAULT });
        ui.within(&row, &mut |ui| {
            super::songs::row::glyph(ui, (lw_core::FileState::Failed, Some(lw_core::FileAvailability::Unknown), false), true, p);
            text(ui, "message", message, 13, p.ink, (Size::Grow, 18.9), false);
            if action(ui, "reconnect", "Reconnect", 88.0, !files.busy, Palette { panel: p.ink, ink: p.paper, ..p }) {
                *intent = Some(Intent::ReconnectSource(target));
            }
        });
    }
    let ready = !directory.songs.is_empty() && !files.busy;
    let keys = ui.node("folder-keys", Node { axis: Axis::Row, gap: 8.0, w: Size::Grow, h: Size::Px(52.0), margin: Edges { t: 12.0, b: 8.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&keys, &mut |ui| {
        if key(ui, "play-folder", "Play folder", Icon::Play, true, p) && ready {
            *intent = Some(Intent::PlayFolder);
        }
        for (id, name, offline) in [("shuffle", "Shuffle folder", false), ("keep-offline", if directory.kept { "Kept offline" } else { "Keep folder offline" }, true)] {
            let kept = offline && directory.kept;
            let node = ui.node(id, Node { flags: CLICKABLE | FOCUSABLE, role: Role::Button, w: Size::Px(52.0), h: Size::Px(52.0), look: Look { fill: if kept { p.ink } else { p.panel }, ..Look::NONE }, ..Node::DEFAULT });
            ui.a11y_name(&node, name);
            let color = if kept { p.paper } else { p.ink };
            ui.draw(&node, move |paint, rect| {
                let rect = pxui::geom::Rect { x: rect.x + 16.0, y: rect.y + 16.0, w: 20.0, h: 20.0 };
                if offline { crate::icons::offline(paint, rect, kept, color) } else { crate::icons::shuffle(paint, rect, color) }
            });
            if ui.signal_state(&node).clicked && ready {
                *intent = Some(if offline { Intent::KeepFolder } else { Intent::ShuffleFolder });
            }
        }
    });
    status(ui, files, width, true, p, intent);
    if directory.entries.is_empty() {
        text(ui, "empty-directory", "No files in this folder.", 14, p.secondary, (Size::Grow, 48.0), false);
        return;
    }
    let key = int_key(int_key(Ui::key_of(0, "files-directory"), location.source), location.parent.unwrap_or(0));
    let list = ui.node_keyed(key, Node { w: Size::Grow, h: Size::Grow, ..Node::DEFAULT });
    let shown = directory.entries.len() - if directory.show_non_audio { 0 } else { directory.non_audio };
    ui.within(&list, &mut |ui| {
        ui.grid_list("directory-entries", shown, width, 56.0, &mut |ui, index| {
            let entry = &directory.entries[index];
            let other = !entry.directory && !entry.audio;
            let row = ui.node_keyed(
                int_key(Ui::key_of(0, "files-entry"), entry.id),
                Node { axis: Axis::Row, align: Align::Center, gap: 12.0, w: Size::Grow, h: Size::Px(56.0), flags: if entry.directory { CLICKABLE | FOCUSABLE } else { 0 }, role: if entry.directory { Role::Button } else { Role::Row }, ..Node::DEFAULT },
            );
            ui.a11y_name(&row, &entry.name);
            let signal = ui.signal_state(&row);
            ui.within(&row, &mut |ui| {
                let mark_cell = ui.node("mark-cell", Node { w: Size::Px(16.0), h: Size::Px(16.0), ..Node::DEFAULT });
                ui.within(&mark_cell, &mut |ui| match entry.glyph {
                    Some(glyph) if entry.audio => super::songs::row::glyph(ui, (glyph.state, glyph.availability, glyph.pinned), true, p),
                    _ => mark(ui, entry.directory, p.secondary),
                });
                let labels = ui.node("labels", Node { w: Size::Grow, h: Size::Fit, ..Node::DEFAULT });
                ui.within(&labels, &mut |ui| {
                    text(ui, "name", &entry.name, 15, if other { p.tertiary } else { p.ink }, (Size::Grow, 21.0), false);
                    text(ui, "detail", &entry.detail, 12, p.secondary, (Size::Grow, 16.8), true);
                });
            });
            ui.draw_over(&row, move |paint, rect| paint.fill_fractional(rect.x + 28.0, rect.y + rect.h - 1.0, rect.w - 28.0, 1.0, p.line));
            if signal.clicked && entry.directory && intent.is_none() {
                *intent = Some(Intent::Files(Request::Browse(Location { parent: Some(entry.id), ..location })));
            }
        });
    });
}

/// The right-hand key of the folder toolbar.
fn legend(ui: &mut Ui, p: Palette) {
    use lw_core::{FileAvailability::{InCloud, OnDevice}, FileState::Copied};
    let legend = ui.node("legend", Node { axis: Axis::Row, align: Align::Center, gap: 6.0, w: Size::Fit, h: Size::Px(30.0), padding: Edges { l: 4.0, ..Edges::ZERO }, ..Node::DEFAULT });
    ui.within(&legend, &mut |ui| {
        super::songs::row::glyph(ui, (Copied, Some(OnDevice), true), true, p);
        text(ui, "kept", "kept offline", 12, p.secondary, (Size::Fit, 16.8), false);
        super::songs::row::glyph(ui, (Copied, Some(OnDevice), false), true, p);
        text(ui, "device", "on this Mac", 12, p.secondary, (Size::Fit, 16.8), false);
        super::songs::row::glyph(ui, (Copied, Some(InCloud), false), true, p);
        text(ui, "cloud", "in cloud", 12, p.secondary, (Size::Fit, 16.8), false);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::{Directory, Entry, Folder, Snapshot};
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
        ui.frame(
            &FrameIn {
                width: size.0,
                height: size.1,
                time: *time,
                events: events.as_ptr(),
                event_count: events.len(),
                ..Default::default()
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
                Event {
                    kind: input::wire::POINTER_MOVED,
                    x: x as f64,
                    y: y as f64,
                    ..Default::default()
                },
                Event {
                    kind: input::wire::POINTER_PRESSED,
                    x: x as f64,
                    y: y as f64,
                    ..Default::default()
                },
            ],
            &mut intent,
        );
        frame(
            ui,
            files,
            catalog,
            state,
            time,
            &[Event {
                kind: input::wire::POINTER_RELEASED,
                x: x as f64,
                y: y as f64,
                ..Default::default()
            }],
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
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
            click(&mut ui, &files, &catalog, &state, &mut time, "Add folder"),
            Some(Intent::ChooseFolder)
        );
        assert_eq!(
            click(
                &mut ui,
                &files,
                &catalog,
                &state,
                &mut time,
                "Add audio files"
            ),
            Some(Intent::ChooseAudio)
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
                    Event {
                        kind: input::wire::POINTER_MOVED,
                        x: 650.0,
                        y: 400.0,
                        ..Default::default()
                    },
                    Event {
                        kind: input::wire::SCROLLED,
                        x: 0.0,
                        y: if step < 10 { -1.0 } else { 1.0 },
                        ..Default::default()
                    },
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
            ui.frame(
                &FrameIn {
                    width: 640.0,
                    height: 480.0,
                    time,
                    ..Default::default()
                },
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
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
    }
}
