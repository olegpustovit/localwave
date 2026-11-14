use super::LibraryWorker;
use lw_apple_sys::collation::Key;
use lw_core::{LibraryRepository, context::PlaybackContext};
use lw_ui::browse::{Request, Scope, Sort};
use lw_ui::catalog::Catalog;
use std::cmp::Ordering;
use std::io;

fn load(
    repository: &LibraryRepository,
    count: usize,
    identity: i64,
) -> lw_apple_sys::sqlite::Result<Catalog> {
    let songs = repository.search_songs("", count, 0)?;
    let mut catalog = Catalog::new(songs);
    let source_count = repository
        .user(identity)?
        .map(|user| repository.source_count(user.id))
        .transpose()?
        .unwrap_or(0);
    std::sync::Arc::make_mut(&mut catalog.library).set_source_count(source_count);
    for list in repository.playlists()? {
        let rows = repository
            .playlist_songs(list.id)?
            .into_iter()
            .filter_map(|song| catalog.row_index(song.id?))
            .collect();
        catalog.add_collection(
            &list.name,
            rows,
            PlaybackContext::Playlist {
                id: list.id,
                name: list.name.clone(),
            },
        );
    }
    Ok(catalog)
}

fn sort(catalog: &Catalog, rows: &mut [usize], column: Sort, ascending: bool) -> io::Result<()> {
    if column == Sort::Original {
        return Ok(());
    }
    if column == Sort::Duration {
        rows.sort_unstable_by(|a, b| {
            let (a, b) = (catalog.row(*a), catalog.row(*b));
            match (a.seconds, b.seconds) {
                (Some(a_time), Some(b_time)) => {
                    let comparison = a_time.total_cmp(&b_time);
                    (if ascending {
                        comparison
                    } else {
                        comparison.reverse()
                    })
                    .then(a.id.cmp(&b.id))
                }
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => a.id.cmp(&b.id),
            }
        });
    } else {
        let mut keys = Vec::with_capacity(rows.len());
        for index in rows.iter().copied() {
            let row = catalog.row(index);
            let value = match column {
                Sort::Title => row.title,
                Sort::Artist => row.artist,
                Sort::Album => row.album,
                Sort::Format => row.format,
                _ => unreachable!(),
            };
            keys.push((index, Key::new(value)?));
        }
        keys.sort_unstable_by(|(a, a_key), (b, b_key)| {
            let comparison = a_key.compare(b_key);
            (if ascending {
                comparison
            } else {
                comparison.reverse()
            })
            .then(catalog.row(*a).id.cmp(&catalog.row(*b).id))
        });
        for (out, (index, _)) in rows.iter_mut().zip(keys) {
            *out = index;
        }
    }
    Ok(())
}

impl LibraryWorker {
    pub fn catalog(&self) -> io::Result<Catalog> {
        self.catalog_view(None, None)
    }

    /// Reuse immutable labels; database changes and Find membership are read from one version.
    pub fn catalog_view(
        &self,
        current: Option<&Catalog>,
        request: Option<Request>,
    ) -> io::Result<Catalog> {
        let command_query = request.and_then(|request| request.commands);
        let command_query_changed = current
            .and_then(|catalog| catalog.commands.as_ref().map(|commands| commands.query))
            != command_query;
        let commands = request
            .and_then(|request| request.commands)
            .map(|query| super::commands::Commands::new(self, query))
            .transpose()?;
        let request = request.map(|mut request| {
            request.commands = None;
            request
        });
        let find = request
            .filter(|request| request.scope == Scope::Find)
            .map(|request| super::find::Find::new(self, request.query.as_str()))
            .transpose()?;
        let identity = lw_apple_sys::cloud::current_identity()?
            .as_deref()
            .map_or(-1, lw_core::hash_string);
        let history = self.playback_history();
        let (mut catalog, mut rows, candidates, command_candidates) = self
            .repository
            .read_snapshot(|repository| {
                // Establish the SQLite read snapshot before stamping or reusing library data.
                let count = repository.song_count("")?;
                let version = repository.data_version()?;
                let mut catalog = match current.filter(|catalog| {
                    catalog.source_version == Some(version)
                        && catalog.source_identity == Some(identity)
                }) {
                    Some(catalog) => catalog.clone(),
                    None => load(repository, count, identity)?,
                };
                catalog.source_version = Some(version);
                catalog.source_identity = Some(identity);
                catalog.prepare_most_played(&mut |id| history.get(id).plays);
                let keep_view = command_query_changed
                    && request.is_some_and(|request| {
                        catalog
                            .view
                            .as_ref()
                            .is_some_and(|view| view.request == request)
                    });
                if !keep_view {
                    catalog.view = None;
                }
                catalog.commands = None;
                let mut candidates = None;
                let rows = match request.map(|request| request.scope) {
                    _ if keep_view => None,
                    Some(Scope::Find) => {
                        candidates = Some(find.as_ref().unwrap().prepare(repository, &catalog)?);
                        None
                    }
                    Some(Scope::Collection(key)) => catalog
                        .base_collection(key)
                        .filter(|_| request.unwrap().sort != Sort::Original)
                        .map(|group| group.rows.clone()),
                    Some(Scope::Overview) | None => None,
                };
                let command_candidates = commands
                    .as_ref()
                    .map(|commands| commands.prepare(repository, &catalog))
                    .transpose()?;
                Ok((catalog, rows, candidates, command_candidates))
            })
            .map_err(io::Error::other)?;
        if catalog.artist_table.is_none() {
            let pins = self.preferences.import_snapshot();
            catalog.prepare_pins(&mut |id| pins.pinned(id));
            catalog.prepare_tables().map_err(io::Error::other)?;
        }
        let summary = if let (Some(find), Some(candidates)) = (find, candidates) {
            let found = find.finish(self, &catalog, candidates);
            rows = Some(found.rows);
            Some(found.summary)
        } else {
            None
        };
        if let (Some(request), Some(rows)) = (request, rows.as_mut())
            && (request.scope == Scope::Find || request.sort != Sort::Original)
        {
            sort(&catalog, rows, request.sort, request.ascending)?;
            catalog.project(request, std::mem::take(rows));
            if let Some(view) = catalog.view.as_mut() {
                std::sync::Arc::make_mut(view).find = summary;
            }
        }
        if let (Some(commands), Some(prepared)) = (commands, command_candidates) {
            catalog.commands = Some(std::sync::Arc::new(
                commands.finish(self, &catalog, prepared),
            ));
        }
        Ok(catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    use lw_core::Song;
    use lw_ui::browse::Query;
    use lw_ui::catalog::Route;
    use std::sync::Arc;
    #[test]
    fn worker_views_share_labels_sort_naturally_keep_unknown_times_last_and_refresh_external_edits()
    {
        let base = std::env::temp_dir().join(format!("lw-browse-{}", std::process::id()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        std::fs::create_dir_all(&base).unwrap();
        let database = base.join("library.sqlite");
        let mut worker = LibraryWorker::open(
            &database,
            Preferences::open(Some(&format!(
                "com.olegpustovit.localwave.browse-{}",
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
        let mut ids = Vec::new();
        for (index, (title, duration)) in [
            ("Track 10", Some(100.0)),
            ("Track 2", Some(120.0)),
            ("Same", None),
            ("Same", Some(30.0)),
            ("Unknown", None),
        ]
        .into_iter()
        .enumerate()
        {
            ids.push(
                worker
                    .repository
                    .upsert_song(
                        &Song {
                            song_key: index as i64 + 1,
                            title: title.into(),
                            artist: "Artist".into(),
                            album: "Album".into(),
                            duration,
                            created_at: index as f64 + 1.0,
                            ..Song::default()
                        },
                        index as f64 + 1.0,
                    )
                    .unwrap()
                    .id
                    .unwrap(),
            );
        }
        let external = lw_apple_sys::sqlite::Connection::open(database.to_str().unwrap()).unwrap();
        external
            .execute(
                "INSERT INTO songs_fts(songId,title) VALUES (?,'Track 10')",
                &[lw_apple_sys::sqlite::Value::Integer(ids[0])],
            )
            .unwrap();
        let catalog = worker.catalog().unwrap();
        let mut stale_identity = catalog.clone();
        stale_identity.source_identity = Some(i64::MAX);
        let refreshed_identity = worker.catalog_view(Some(&stale_identity), None).unwrap();
        assert!(
            !Arc::ptr_eq(&catalog.library, &refreshed_identity.library),
            "source ownership participates in catalog reuse"
        );
        assert_eq!(refreshed_identity.source_identity, catalog.source_identity);
        let scope = Scope::Collection(catalog.all.as_ref().unwrap().key);
        let original = catalog.collection(Route::Songs).unwrap().ids.clone();
        for (sort, ascending, expected) in [
            (
                Sort::Title,
                true,
                vec![ids[2], ids[3], ids[1], ids[0], ids[4]],
            ),
            (
                Sort::Title,
                false,
                vec![ids[4], ids[0], ids[1], ids[2], ids[3]],
            ),
            (
                Sort::Duration,
                true,
                vec![ids[3], ids[0], ids[1], ids[2], ids[4]],
            ),
            (
                Sort::Duration,
                false,
                vec![ids[1], ids[0], ids[3], ids[2], ids[4]],
            ),
        ] {
            let request = Request {
                commands: None,
                scope,
                query: Query::default(),
                sort,
                ascending,
            };
            let prepared = worker.catalog_view(Some(&catalog), Some(request)).unwrap();
            assert!(Arc::ptr_eq(&catalog.library, &prepared.library));
            assert_eq!(&*prepared.collection(Route::Songs).unwrap().ids, expected);
            assert_eq!(catalog.collection(Route::Songs).unwrap().ids, original);
        }
        let find = Request {
            commands: None,
            scope: Scope::Find,
            query: Query::new("track").unwrap(),
            sort: Sort::Original,
            ascending: true,
        };
        let prepared = worker.catalog_view(Some(&catalog), Some(find)).unwrap();
        assert_eq!(
            &*prepared.collection(Route::Find).unwrap().ids,
            [ids[1], ids[0]],
            "Find retains library order and deduplicates FTS matches"
        );
        assert!(Arc::ptr_eq(&catalog.library, &prepared.library));
        let mut writer = LibraryRepository::open(database.to_str().unwrap()).unwrap();
        let mut changed = writer.get_song(ids[1]).unwrap().unwrap();
        changed.title = "Moved title".into();
        writer.upsert_song(&changed, 10.0).unwrap();
        let current = worker
            .catalog_view(
                Some(&prepared),
                Some(Request {
                    query: Query::new("moved").unwrap(),
                    ..find
                }),
            )
            .unwrap();
        assert!(!Arc::ptr_eq(&prepared.library, &current.library));
        assert_eq!(&*current.collection(Route::Find).unwrap().ids, [ids[1]]);
        assert_eq!(
            current
                .row(current.collection(Route::Find).unwrap().rows[0])
                .title,
            "Moved title"
        );
        mem_sys::local_reset();
        for _ in 0..100 {
            let view = current.clone();
            assert!(Arc::ptr_eq(&current.library, &view.library));
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
}
