use super::*;
use crate::files::{Location, Request, Target};

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
                if action(
                    ui,
                    "remove-confirm",
                    "Remove",
                    100.0,
                    enabled,
                    Palette { ink: p.danger, ..p },
                ) {
                    *intent = Some(Intent::Files(Request::Remove(target)));
                }
            });
        });
    });
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseRemoveSource);
    }
}

fn button(ui: &mut Ui, key: &str, label: &str, width: f32, enabled: bool, p: Palette) -> Handle {
    let button = ui.node(
        key,
        Node {
            flags: if enabled { CLICKABLE | FOCUSABLE } else { 0 },
            role: Role::Button,
            w: Size::Px(width),
            h: Size::Px(if ui.view_size().0 < compact::BREAKPOINT {
                44.0
            } else {
                36.0
            }),
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
fn action(ui: &mut Ui, key: &str, label: &str, width: f32, enabled: bool, p: Palette) -> bool {
    let button = button(ui, key, label, width, enabled, p);
    ui.signal_state(&button).clicked && enabled
}

pub(super) fn view(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let compact = ui.view_size().0 < compact::BREAKPOINT;
    let inset = if compact { 16.0 } else { 28.0 };
    let root = ui.node(
        "files-page",
        Node {
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges {
                l: inset,
                r: inset,
                t: 28.0,
                b: 12.0,
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let Some(files) = player.files else {
            text(ui, "loading", "Loading folders…", 18, p.secondary, (Size::Grow, 40.0), false);
            return;
        };
        let heading = container(ui, "files-heading", Axis::Row, Size::Grow, Size::Px(44.0), p.paper);
        ui.within(&heading, &mut |ui| {
            text(ui, "title", files.directory.as_ref().map_or("Folders", |directory| directory.title.as_str()), 32, p.ink, (Size::Grow, 44.0), false);
            if !compact && let Some(directory) = &files.directory { options(ui, Target { user: directory.location.user, source: directory.location.source, root: directory.source_root }, p, intent); }
            if action(ui, "add-folder", "Add folder", 110.0, files.account.is_some() && !files.busy, p) { *intent = Some(Intent::ChooseFolder); }
        });
        text(ui, "files-detail", files.directory.as_ref().map_or(files.source_count.as_str(), |directory| directory.detail.as_str()), 13, p.secondary, (Size::Grow, 26.0), true);
        let tools = ui.node("files-tools", Node { axis: Axis::Row, wrap: compact,
            w: Size::Grow, h: if compact { Size::Fit } else { Size::Px(40.0) },
            ..Node::DEFAULT });
        ui.within(&tools, &mut |ui| {
            if action(ui, "add-audio", "Add audio files", 118.0, files.account.is_some() && !files.busy, p) { *intent = Some(Intent::ChooseAudio); }
            if let Some(directory) = &files.directory {
                let location = directory.location;
                if action(ui, "back", "Back", 64.0, true, p) {
                    *intent = Some(Intent::Files(directory.up.map_or(Request::Roots, |parent| Request::Browse(Location { parent: Some(parent), ..location }))));
                }
                if action(ui, "rescan", "Rescan source", 110.0, !files.busy, p) { *intent = Some(Intent::Files(Request::Rescan(location))); }
                if action(ui, "import", "Import folder", 108.0, !files.busy, p) { *intent = Some(Intent::Files(Request::Import(location))); }
                if compact { options(ui, Target { user: location.user, source: location.source, root: directory.source_root }, p, intent); }
            } else if action(ui, "refresh", "Refresh folders", 130.0, true, p) { *intent = Some(Intent::Files(Request::Roots)); }
        });
        if files.busy {
            let progress = container(ui, "files-progress", Axis::Row, Size::Grow, Size::Px(if compact { 44.0 } else { 40.0 }), p.panel);
            ui.within(&progress, &mut |ui| {
                let label = text(ui, "progress-label", &files.progress, 13, p.ink, (Size::Grow, 32.0), true);
                ui.set_role(&label, Role::Progress, 0);
                if action(ui, "cancel", "Cancel", 80.0, true, p) { *intent = Some(Intent::Files(Request::Cancel)); }
            });
        }
        if !files.message.is_empty() {
            settings::paragraph(ui, "files-message", &files.message, width - inset * 2.0, 13.0, p.secondary);
        }
        rule(ui, "files-divider", false, p);
        if files.account.is_none() {
            if files.message.is_empty() { text(ui, "files-loading", "Loading folders…", 14, p.secondary, (Size::Grow, 40.0), false); }
            return;
        }
        if let Some(directory) = &files.directory {
            let location = directory.location;
            text(ui, "directory-path", &directory.path, 12, p.secondary, (Size::Grow, 26.0), true);
            // The parent participates in the identity, giving each destination its own scroll state.
            if directory.entries.is_empty() {
                text(ui, "empty-directory", "No indexed audio here. Rescan this source to update it.", 14, p.secondary, (Size::Grow, 48.0), false);
                return;
            }
            let key = int_key(int_key(Ui::key_of(0, "files-directory"), location.source), location.parent.unwrap_or(0));
            let list = ui.node_keyed(key, Node { w: Size::Grow, h: Size::Grow, ..Node::DEFAULT });
            ui.within(&list, &mut |ui| {
                ui.grid_list("directory-entries", directory.entries.len(), (width - inset * 2.0).max(1.0), 44.0, &mut |ui, index| {
                    let entry = &directory.entries[index];
                    let row = ui.node_keyed(int_key(Ui::key_of(0, "files-entry"), entry.id), Node { axis: Axis::Row, w: Size::Grow, h: Size::Px(44.0), align: Align::Center, flags: if entry.directory { CLICKABLE | FOCUSABLE } else { 0 }, role: if entry.directory { Role::Button } else { Role::Row }, ..Node::DEFAULT });
                    ui.a11y_name(&row, &entry.name);
                    let signal = ui.signal_state(&row);
                    ui.within(&row, &mut |ui| {
                        text(ui, "name", &entry.name, 14, p.ink, (Size::Grow, 32.0), false);
                        text(ui, "detail", &entry.detail, 12, p.secondary, (Size::Px(140.0), 32.0), true);
                        if entry.audio && action(ui, "import-file", "Import", 72.0, !files.busy, p) {
                            *intent = Some(Intent::Files(Request::ImportPath { user: location.user, source: location.source, path: entry.id }));
                        }
                    });
                    if signal.clicked && entry.directory { *intent = Some(Intent::Files(Request::Browse(Location { parent: Some(entry.id), ..location }))); }
                });
            });
        } else {
            if files.sources.is_empty() && files.account.is_some() {
                text(ui, "empty-heading", "Point it at a folder.", 28, p.ink, (Size::Grow, 48.0), false);
                settings::paragraph(ui, "empty-description", "Your files stay where they are. Nothing is uploaded. Choose an original music folder to index and import its audio files.", width - inset * 2.0, 14.0, p.secondary);
                return;
            }
            ui.grid_list("source-folders", files.sources.len(), (width - inset * 2.0).max(1.0), 88.0, &mut |ui, index| {
                let folder = &files.sources[index];
                let row = ui.node_keyed(int_key(Ui::key_of(0, "files-source"), folder.id), Node { axis: Axis::Row, w: Size::Grow, h: Size::Px(88.0), flags: CLICKABLE | FOCUSABLE, role: Role::Button, padding: Edges::all(8.0), look: Look { fill: if folder.current { p.selected } else { p.paper }, ..Look::NONE }, ..Node::DEFAULT });
                ui.a11y_name(&row, &folder.name);
                ui.within(&row, &mut |ui| {
                    let labels = ui.node("source-labels", Node { w: Size::Grow, h: Size::Grow, ..Node::DEFAULT });
                    ui.within(&labels, &mut |ui| {
                    text(ui, "folder-name", &folder.name, 18, p.ink, (Size::Grow, 28.0), false);
                    text(ui, "folder-path", &folder.path, 12, p.secondary, (Size::Grow, 22.0), true);
                    text(ui, "folder-summary", &folder.detail, 12, p.secondary, (Size::Grow, 22.0), true);
                    });
                    if let Some(user) = files.account { options(ui, Target { user, source: folder.id, root: folder.root }, p, intent); }
                });
                let signal = ui.signal_state(&row);
                if signal.pressed && signal.button == Some(1) && let Some(user) = files.account { *intent = Some(Intent::SourceMenu(Target { user, source: folder.id, root: folder.root }, signal.pointer)); }
                if intent.is_none() && ui.signal_state(&row).clicked && let Some(user) = files.account { *intent = Some(Intent::Files(Request::Browse(Location { user, source: folder.id, parent: None }))); }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::{Directory, Entry, Folder, Snapshot};
    use pxui::{Event, FrameIn, Platform};
    use std::sync::Arc;

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
                detail: "10000 indexed items".into(),
                current: true,
            }]),
            ..Default::default()
        };
        let catalog = Catalog::default();
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Files));
        let mut time = 0.0;
        fn frame(
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
                    width: 1440.0,
                    height: 872.0,
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
        let menu = click(
            &mut ui,
            &files,
            &catalog,
            &state,
            &mut time,
            "Folder options",
        )
        .unwrap();
        assert!(
            matches!(menu, Intent::SourceMenu(value, (x, y)) if value == target && x > 0.0 && y > 0.0)
        );
        state.apply(menu);
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
        let menu = click(
            &mut ui,
            &files,
            &catalog,
            &state,
            &mut time,
            "Folder options",
        )
        .unwrap();
        state.apply(menu);
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
            detail: String::new(),
            current: true,
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
            path: "/Music/Original".into(),
            detail: "10000 items".into(),
            entries: (0..10_000)
                .map(|id| Entry {
                    id,
                    name: format!("Track {id:05}.flac"),
                    detail: "2000 bytes".into(),
                    directory: false,
                    audio: true,
                })
                .collect(),
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
        assert_eq!(
            click(
                &mut ui,
                &files,
                &catalog,
                &state,
                &mut time,
                "Rescan source"
            ),
            Some(Intent::Files(Request::Rescan(Location {
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
            click(&mut ui, &files, &catalog, &state, &mut time, "Add folder"),
            None
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
            None
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
                        &catalog,
                        &state,
                        &mut None,
                    );
                },
            )
            .unwrap();
        }
        let mut right = 0.0;
        for name in ["Add audio files", "Back", "Rescan source", "Import folder"] {
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
}
