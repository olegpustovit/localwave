use super::{
    LibraryWorker,
    find::{Candidates, Find},
};
use lw_apple_sys::collation::Key;
use lw_core::{LibraryRepository, context::PlaybackContext, query::SongQuery};
use lw_ui::{
    browse::Query,
    catalog::{Catalog, Route},
    commands::{Action, Command, Icon, Item, Results},
    files::Location,
};
use std::collections::HashSet;
use std::{io, path::Path, sync::Arc};

pub(super) struct Commands {
    query: Query,
    find: Find,
    needle: Key,
    drive: Key,
}

pub(super) struct Prepared {
    songs: Candidates,
    items: Vec<Item>,
    rescan: Option<Location>,
}
impl Commands {
    pub fn new(worker: &LibraryWorker, query: Query) -> io::Result<Self> {
        Ok(Self {
            query,
            find: Find::new(worker, query.as_str().trim())?,
            needle: Key::new(query.as_str().trim())?,
            drive: Key::new("Google Drive")?,
        })
    }
    /// Prepare metadata and indexed suggestions in the same snapshot as the underlying page.
    pub fn prepare(
        &self,
        repository: &LibraryRepository,
        catalog: &Catalog,
    ) -> lw_apple_sys::sqlite::Result<Prepared> {
        let user = catalog
            .source_identity
            .map(|id| repository.user(id))
            .transpose()?
            .flatten();
        let mut items = Vec::new();
        let mut rescan = None;
        if let Some(user) = &user {
            for source in repository.sources(user.id)? {
                if self.drive.contained_in_case_insensitive(&source.dir_path) {
                    rescan = source.id.map(|id| Location {
                        user: user.id,
                        source: id,
                        parent: None,
                    });
                    break;
                }
            }
            let text = SongQuery::new(self.query.as_str());
            for (source, path, name, directory) in
                repository.search_user_directories(user.id, text.text(), 3)?
            {
                items.push(Item::new(
                    "Folders",
                    &name,
                    Path::new(&directory)
                        .file_name()
                        .and_then(|v| v.to_str())
                        .unwrap_or(&directory),
                    Icon::Folder,
                    None,
                    Action::Folder(Location {
                        user: user.id,
                        source,
                        parent: Some(path),
                    }),
                ));
            }
        }
        Ok(Prepared {
            songs: self.find.prepare_ranked(repository, catalog)?,
            items,
            rescan,
        })
    }

    pub fn finish(&self, worker: &LibraryWorker, catalog: &Catalog, prepared: Prepared) -> Results {
        let Prepared {
            songs,
            items: folders,
            rescan,
        } = prepared;
        let found = self.find.finish(worker, catalog, songs);
        let mut items = Vec::new();
        let empty = self.query.as_str().trim().is_empty();
        if !empty {
            for (index, row) in found.rows.iter().copied().take(6).enumerate() {
                let song = catalog.row(row);
                items.push(Item::new(
                    "Songs",
                    song.title,
                    song.artist,
                    Icon::Chevron,
                    Some(row),
                    Action::Play(index),
                ));
            }
            // Native palette albums use album + track artist, independently of song-query filters.
            let mut seen = HashSet::new();
            let mut album_text = String::new();
            for row in catalog.all.iter().flat_map(|all| &all.rows).copied() {
                let song = catalog.row(row);
                album_text.clear();
                album_text.push_str(song.album);
                album_text.push(' ');
                album_text.push_str(song.artist);
                if self.needle.contained_in_case_insensitive(&album_text)
                    && seen.insert((song.album, song.artist))
                {
                    if let Some(index) = catalog
                        .albums
                        .iter()
                        .position(|album| album.rows.contains(&row))
                    {
                        items.push(Item::new(
                            "Albums",
                            song.album,
                            song.artist,
                            Icon::Chevron,
                            Some(row),
                            Action::Navigate(Route::Album(index)),
                        ));
                    }
                    if seen.len() == 3 {
                        break;
                    }
                }
            }
            let mut artists = catalog
                .artists
                .iter()
                .enumerate()
                .filter(|(_, artist)| self.needle.contained_in_case_insensitive(&artist.title))
                .collect::<Vec<_>>();
            artists.sort_unstable_by(|(_, a), (_, b)| a.title.cmp(&b.title));
            for (index, artist) in artists.into_iter().take(3) {
                items.push(Item::new(
                    "Artists",
                    &artist.title,
                    "",
                    Icon::Person,
                    None,
                    Action::Navigate(Route::Artist(index)),
                ));
            }
            items.extend(folders);
            if found.rows.len() > 6 {
                items.push(Item::new(
                    "Songs",
                    &format!("Show all {} songs", found.rows.len()),
                    "",
                    Icon::Rows,
                    None,
                    Action::Find,
                ));
            }
        }
        for command in [
            Some(Command::PlayPause),
            Some(Command::Shuffle),
            Some(Command::Repeat),
            Some(Command::AddMusic),
            rescan.map(Command::Rescan),
            Some(Command::Network),
            Some(Command::Theme),
            Some(Command::Panel),
            Some(Command::Shortcuts),
            Some(Command::Settings),
        ]
        .into_iter()
        .flatten()
        {
            if (empty && command != Command::Settings)
                || (!empty && self.needle.contained_in_case_insensitive(command.label()))
            {
                items.push(Item::new(
                    "Commands",
                    command.label(),
                    command.hint(),
                    Icon::Chevron,
                    None,
                    Action::Command(command),
                ));
            }
        }
        if empty {
            for (name, label) in [
                ("Favorites", "Favorites"),
                ("Recently Added", "Recently added"),
                ("Most Played", "Most played"),
            ] {
                if let Some(index) = catalog.collections.iter().position(|group| matches!(&*group.context, PlaybackContext::Automatic(value) if value == name)) {
                    items.push(Item::new("Lists", label, "", Icon::Queue, None, Action::Navigate(Route::Collection(index))));
                }
            }
        }
        Results {
            query: self.query,
            items,
            songs: found.rows.iter().map(|row| catalog.row(*row).id).collect(),
            context: Arc::new(PlaybackContext::Selection(
                self.query.as_str().trim().into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    use lw_ui::browse::{Request, Scope, Sort};
    #[test]
    fn command_results_keep_ranked_queues_owned_folders_commands_and_native_lists_without_replacing_find()
     {
        let base = std::env::temp_dir().join(format!("lw-commands-{}", std::process::id()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        std::fs::create_dir_all(&base).unwrap();
        let mut worker = LibraryWorker::open(
            &base.join("library.sqlite"),
            Preferences::open(Some(&format!(
                "com.olegpustovit.localwave.commands-{}",
                std::process::id()
            )))
            .unwrap(),
            AppPaths {
                documents: base.join("Documents"),
                caches: base.join("Caches"),
                temporary: base.join("tmp"),
                test_database: None,
            },
        )
        .unwrap();
        let identity = lw_apple_sys::cloud::current_identity()
            .unwrap()
            .as_deref()
            .map_or(-1, lw_core::hash_string);
        let user = worker.repository.get_or_create_user(identity).unwrap();
        let other = worker
            .repository
            .get_or_create_user(identity.wrapping_add(1))
            .unwrap();
        let own = worker
            .repository
            .create_source(&lw_core::Source {
                dir_path: "/Music/Google Drive".into(),
                path_id: 1,
                user_id: user.id,
                ..Default::default()
            })
            .unwrap()
            .id
            .unwrap();
        let foreign = worker
            .repository
            .create_source(&lw_core::Source {
                dir_path: "/Other".into(),
                path_id: 2,
                user_id: other.id,
                ..Default::default()
            })
            .unwrap()
            .id
            .unwrap();
        worker
            .repository
            .upsert_source_paths(&[
                lw_core::SourcePath {
                    source_id: own,
                    path_id: 10,
                    name: "Blue folder".into(),
                    relative_path: "Blue folder".into(),
                    is_directory: true,
                    run_id: 1,
                    ..Default::default()
                },
                lw_core::SourcePath {
                    source_id: own,
                    path_id: 11,
                    name: "Blue.flac".into(),
                    relative_path: "Blue.flac".into(),
                    run_id: 1,
                    ..Default::default()
                },
                lw_core::SourcePath {
                    source_id: foreign,
                    path_id: 12,
                    name: "Blue private".into(),
                    relative_path: "Blue private".into(),
                    is_directory: true,
                    run_id: 1,
                    ..Default::default()
                },
            ])
            .unwrap();
        let mut ids = Vec::new();
        for index in 0..40 {
            ids.push(
                worker
                    .repository
                    .upsert_song(
                        &lw_core::Song {
                            song_key: index + 1,
                            title: if index == 0 {
                                "Blue blue blue".into()
                            } else if index < 9 {
                                format!("Blue track {index}")
                            } else {
                                format!("Other track {index}")
                            },
                            artist: if index < 9 {
                                "Café".into()
                            } else {
                                "Zulu".into()
                            },
                            album: if index < 9 {
                                "Blue album".into()
                            } else {
                                "Other album".into()
                            },
                            release_year: Some(if index < 7 { 2019 } else { 2020 }),
                            created_at: index as f64 + 1.0,
                            ..Default::default()
                        },
                        index as f64 + 1.0,
                    )
                    .unwrap()
                    .id
                    .unwrap(),
            );
        }
        for (index, id) in ids.iter().enumerate() {
            worker
                .preferences
                .record_playback(*id, index as f64 + 100.0)
                .unwrap();
        }
        worker.preferences.record_playback(ids[2], 200.0).unwrap();
        let request = Request {
            commands: None,
            scope: Scope::Find,
            query: Query::new("Blue").unwrap(),
            sort: Sort::Original,
            ascending: true,
        };
        let find = worker.catalog_view(None, Some(request)).unwrap();
        let page = find.view.clone().unwrap();
        let expected = worker
            .repository
            .search_songs("Blue", 40, 0)
            .unwrap()
            .iter()
            .map(|song| song.id.unwrap())
            .collect::<Vec<_>>();
        let open = worker
            .catalog_view(
                Some(&find),
                Some(Request {
                    commands: Some(Query::new("Blue").unwrap()),
                    ..request
                }),
            )
            .unwrap();
        assert!(Arc::ptr_eq(&find.library, &open.library));
        assert!(
            Arc::ptr_eq(&page, open.view.as_ref().unwrap()),
            "palette search must retain the exact background Find projection"
        );
        let results = open.commands.as_ref().unwrap();
        assert_eq!(&*results.songs, expected);
        assert_ne!(
            &*results.songs, &*page.collection.ids,
            "palette rank differs from Find's original order"
        );
        assert_eq!(
            &*results.context,
            &PlaybackContext::Selection("Blue".into())
        );
        assert_eq!(
            results
                .items
                .iter()
                .filter(|item| item.group == "Songs" && matches!(item.action, Action::Play(_)))
                .count(),
            6
        );
        assert!(
            results
                .items
                .iter()
                .any(|item| item.label == "Show all 9 songs" && item.action == Action::Find)
        );
        let folders = results
            .items
            .iter()
            .filter(|item| item.group == "Folders")
            .collect::<Vec<_>>();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].label, "Blue folder");
        assert_eq!(folders[0].hint, "Google Drive");
        assert_eq!(
            folders[0].action,
            Action::Folder(Location {
                user: user.id,
                source: own,
                parent: Some(10)
            })
        );
        let empty = worker
            .catalog_view(
                Some(&open),
                Some(Request {
                    commands: Some(Query::default()),
                    ..request
                }),
            )
            .unwrap();
        let results = empty.commands.as_ref().unwrap();
        assert!(
            results
                .items
                .iter()
                .all(|item| item.action != Action::Command(Command::Settings))
        );
        assert!(results.items.iter().any(|item| matches!(item.action, Action::Command(Command::Rescan(location)) if location.user == user.id && location.source == own)));
        let lists = results
            .items
            .iter()
            .filter(|item| item.group == "Lists")
            .collect::<Vec<_>>();
        assert_eq!(
            lists
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            ["Favorites", "Recently added", "Most played"]
        );
        let Action::Navigate(route) = lists[2].action else {
            panic!("list must navigate");
        };
        let most = empty.collection(route).unwrap();
        assert_eq!(most.ids.len(), 24);
        assert_eq!(most.ids[0], ids[2]);
        assert_eq!(
            &most.ids[1..],
            &ids[17..].iter().rev().copied().collect::<Vec<_>>()
        );
        for (query, command) in [("settings", Command::Settings), ("REPEAT", Command::Repeat)] {
            let next = worker
                .catalog_view(
                    Some(&empty),
                    Some(Request {
                        commands: Some(Query::new(query).unwrap()),
                        ..request
                    }),
                )
                .unwrap();
            let results = next.commands.as_ref().unwrap();
            assert_eq!(results.items.len(), 1);
            assert_eq!(results.items[0].action, Action::Command(command));
            assert!(Arc::ptr_eq(&page, next.view.as_ref().unwrap()));
        }
        let accented_command = worker
            .catalog_view(
                Some(&empty),
                Some(Request {
                    commands: Some(Query::new("répèat").unwrap()),
                    ..request
                }),
            )
            .unwrap();
        assert!(
            accented_command.commands.as_ref().unwrap().items.is_empty(),
            "native palette command matching preserves accents"
        );
        let cafe = worker
            .catalog_view(
                Some(&empty),
                Some(Request {
                    commands: Some(Query::new("cafe").unwrap()),
                    ..request
                }),
            )
            .unwrap();
        let results = cafe.commands.as_ref().unwrap();
        assert_eq!(
            results.songs.len(),
            9,
            "song filtering follows localizedStandardContains"
        );
        assert!(
            results.items.iter().all(|item| item.group != "Artists"),
            "palette artist filtering follows localizedCaseInsensitiveContains"
        );
        let filtered = worker
            .catalog_view(
                Some(&empty),
                Some(Request {
                    commands: Some(Query::new("year:2019").unwrap()),
                    ..request
                }),
            )
            .unwrap();
        let results = filtered.commands.as_ref().unwrap();
        assert_eq!(results.songs.len(), 7);
        assert!(
            results
                .items
                .iter()
                .all(|item| item.group != "Albums" && item.group != "Artists")
        );
        let unplayed = Request {
            commands: Some(Query::new("unplayed").unwrap()),
            ..request
        };
        let first = worker
            .catalog_view(Some(&filtered), Some(unplayed))
            .unwrap();
        assert!(first.commands.as_ref().unwrap().songs.is_empty());
        let closed = worker.catalog_view(Some(&first), Some(request)).unwrap();
        assert!(closed.commands.is_none());
        assert!(Arc::ptr_eq(&page, closed.view.as_ref().unwrap()));
        let fresh = worker.catalog_view(Some(&closed), Some(request)).unwrap();
        assert!(
            !Arc::ptr_eq(&page, fresh.view.as_ref().unwrap()),
            "explicit refresh must not reuse stale preferences/history"
        );
    }
}
