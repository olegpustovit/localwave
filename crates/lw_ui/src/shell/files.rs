use super::*;
use crate::files::{Location, Request};

fn action(ui: &mut Ui, key: &str, label: &str, width: f32, enabled: bool, p: Palette) -> bool {
    let button = ui.node(
        key,
        Node {
            flags: if enabled { CLICKABLE | FOCUSABLE } else { 0 },
            role: Role::Button,
            w: Size::Px(width),
            h: Size::Px(36.0),
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
    ui.signal_state(&button).clicked && enabled
}

pub(super) fn view(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let root = ui.node(
        "files-page",
        Node {
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges {
                l: 28.0,
                r: 28.0,
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
            if action(ui, "add-folder", "Add folder", 110.0, files.account.is_some() && !files.busy, p) { *intent = Some(Intent::ChooseFolder); }
        });
        text(ui, "files-detail", files.directory.as_ref().map_or(files.source_count.as_str(), |directory| directory.detail.as_str()), 13, p.secondary, (Size::Grow, 26.0), true);
        let tools = container(ui, "files-tools", Axis::Row, Size::Grow, Size::Px(40.0), p.paper);
        ui.within(&tools, &mut |ui| {
            if let Some(directory) = &files.directory {
                let location = directory.location;
                if action(ui, "back", "Back", 76.0, true, p) {
                    *intent = Some(Intent::Files(directory.up.map_or(Request::Roots, |parent| Request::Browse(Location { parent: Some(parent), ..location }))));
                }
                if action(ui, "rescan", "Rescan source", 124.0, !files.busy, p) { *intent = Some(Intent::Files(Request::Rescan(location))); }
                if action(ui, "import", "Import folder", 116.0, !files.busy, p) { *intent = Some(Intent::Files(Request::Import(location))); }
            } else if action(ui, "refresh", "Refresh folders", 130.0, true, p) { *intent = Some(Intent::Files(Request::Roots)); }
        });
        if files.busy {
            let progress = container(ui, "files-progress", Axis::Row, Size::Grow, Size::Px(40.0), p.panel);
            ui.within(&progress, &mut |ui| {
                let label = text(ui, "progress-label", &files.progress, 13, p.ink, (Size::Grow, 32.0), true);
                ui.set_role(&label, Role::Progress, 0);
                if action(ui, "cancel", "Cancel", 80.0, true, p) { *intent = Some(Intent::Files(Request::Cancel)); }
            });
        }
        if !files.message.is_empty() {
            settings::paragraph(ui, "files-message", &files.message, width - 56.0, 13.0, p.secondary);
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
                ui.grid_list("directory-entries", directory.entries.len(), (width - 56.0).max(1.0), 44.0, &mut |ui, index| {
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
                settings::paragraph(ui, "empty-description", "Your files stay where they are. Nothing is uploaded. Choose an original music folder to index and import its audio files.", width - 56.0, 14.0, p.secondary);
                return;
            }
            ui.grid_list("source-folders", files.sources.len(), (width - 56.0).max(1.0), 88.0, &mut |ui, index| {
                let folder = &files.sources[index];
                let row = ui.node_keyed(int_key(Ui::key_of(0, "files-source"), folder.id), Node { w: Size::Grow, h: Size::Px(88.0), flags: CLICKABLE | FOCUSABLE, role: Role::Button, padding: Edges::all(8.0), look: Look { fill: if folder.current { p.selected } else { p.paper }, ..Look::NONE }, ..Node::DEFAULT });
                ui.a11y_name(&row, &folder.name);
                ui.within(&row, &mut |ui| {
                    text(ui, "folder-name", &folder.name, 18, p.ink, (Size::Grow, 28.0), false);
                    text(ui, "folder-path", &folder.path, 12, p.secondary, (Size::Grow, 22.0), true);
                    text(ui, "folder-summary", &folder.detail, 12, p.secondary, (Size::Grow, 22.0), true);
                });
                if ui.signal_state(&row).clicked && let Some(user) = files.account { *intent = Some(Intent::Files(Request::Browse(Location { user, source: folder.id, parent: None }))); }
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
            click(&mut ui, &files, &catalog, &state, &mut time, "Back"),
            Some(Intent::Files(Request::Roots))
        );
        state.apply(Intent::Navigate(Route::Songs));
        assert!(!state.files_visible());
        for _ in 0..3 {
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
    }
}
