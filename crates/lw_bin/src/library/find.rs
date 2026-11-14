use super::{LibraryWorker, hash};
use lw_apple_sys::{
    audio, bookmarks::Access, collation::Key, history::History, preferences::file_path,
};
use lw_core::{FileAvailability, LibraryRepository, Song, Source, query::SongQuery};
use lw_ui::catalog::{Catalog, FindSummary};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;
use std::sync::Arc;

pub(super) struct Find {
    query: SongQuery,
    needles: HashMap<String, Key>,
    history: Option<History>,
    empty: bool,
    raw: String,
}
#[derive(Default)]
pub(super) struct Candidates {
    rows: Vec<usize>,
    files: HashMap<i64, Song>,
    sources: HashMap<i64, Option<Source>>,
    blocked: HashSet<i64>,
    folders: usize,
}
pub(super) struct Found {
    pub rows: Vec<usize>,
    pub summary: FindSummary,
}
impl Find {
    pub fn new(library: &LibraryWorker, raw: &str) -> io::Result<Self> {
        let query = SongQuery::new(raw);
        let mut needles = HashMap::new();
        for word in query.text_words() {
            if !needles.contains_key(word) {
                needles.insert(word.to_owned(), Key::new(word)?);
            }
        }
        let history = query.needs_history().then(|| library.playback_history());
        Ok(Self {
            query,
            needles,
            history,
            empty: raw.trim().is_empty(),
            raw: raw.to_owned(),
        })
    }
    /// Keep metadata/FTS membership and source identities in the same SQLite read snapshot.
    pub fn prepare(
        &self,
        repository: &LibraryRepository,
        catalog: &Catalog,
    ) -> lw_apple_sys::sqlite::Result<Candidates> {
        let mut out = Candidates::default();
        if self.empty {
            return Ok(out);
        }
        if let Some(user) = catalog
            .source_identity
            .map(|identity| repository.user(identity))
            .transpose()?
            .flatten()
        {
            if !self.query.has_filters() {
                out.folders =
                    repository.matching_user_directory_count(user.id, self.query.text())?;
            }
            for source in repository.sources(user.id)? {
                let Some(id) = source.id else {
                    continue;
                };
                if source.sync_error.is_some() {
                    out.blocked.insert(id);
                    out.sources.insert(id, Some(source.clone()));
                }
            }
        }
        let matching: Option<HashSet<_>> = if self.query.text().is_empty() {
            None
        } else {
            Some(
                repository
                    .matching_song_ids(self.query.text())?
                    .into_iter()
                    .collect(),
            )
        };
        let mut files = Vec::new();
        for index in catalog.all.as_ref().unwrap().rows.iter().copied() {
            let row = catalog.row(index);
            if matching.as_ref().is_some_and(|ids| !ids.contains(&row.id)) {
                continue;
            }
            let plays = self
                .history
                .as_ref()
                .map_or(0, |history| history.get(row.id).plays);
            if !self
                .query
                .matches_metadata(row.release_year, row.favorite, plays, &mut |word| {
                    [row.title, row.artist, row.album, row.album_artist]
                        .iter()
                        .any(|value| self.needles[word].contained_in(value))
                })
            {
                continue;
            }
            if self.query.needs_format() && row.has_format && !self.query.matches_format(row.format)
            {
                continue;
            }
            out.rows.push(index);
            if self.query.needs_availability()
                || (self.query.needs_format() && !row.has_format)
                || row
                    .source_id
                    .is_some_and(|source| out.blocked.contains(&source))
            {
                files.push(row.id);
            }
        }
        // Hydrate file locations only for survivors that need probing or legacy format inference.
        for song in repository.get_songs(&files)? {
            if self.query.needs_availability()
                && let Some(id) = song.source_id
                && let std::collections::hash_map::Entry::Vacant(entry) = out.sources.entry(id)
            {
                entry.insert(repository.source(id)?);
            }
            out.files.insert(song.id.unwrap(), song);
        }
        Ok(out)
    }
    /// Filesystem resource queries happen after the read transaction; no audio is opened/downloaded.
    pub fn finish(
        &self,
        library: &LibraryWorker,
        catalog: &Catalog,
        candidates: Candidates,
    ) -> Found {
        let Candidates {
            mut rows,
            files,
            sources,
            blocked,
            folders,
        } = candidates;
        let mut grants = HashMap::new();
        let mut observed = HashMap::new();
        rows.retain(|index| {
            let row = catalog.row(*index);
            let Some(song) = files.get(&row.id) else {
                return !self.query.needs_availability()
                    && (!self.query.needs_format() || row.has_format);
            };
            if self.query.needs_format()
                && !row.has_format
                && !self.query.matches_format(&format(song))
            {
                return false;
            }
            !self.query.needs_availability()
                || self.query.matches_availability(
                    *observed
                        .entry(row.id)
                        .or_insert_with(|| availability(library, song, &sources, &mut grants)),
                )
        });
        let identities = rows
            .iter()
            .map(|index| {
                let row = catalog.row(*index);
                (
                    row.album,
                    if row.album_artist.is_empty() {
                        row.artist
                    } else {
                        row.album_artist
                    },
                )
            })
            .collect::<HashSet<_>>();
        let albums = if self.query.text().is_empty() {
            0
        } else {
            catalog
                .albums
                .iter()
                .filter(|album| {
                    let lw_core::context::PlaybackContext::Album { name, artist } =
                        album.context.as_ref()
                    else {
                        return false;
                    };
                    if !identities.contains(&(name.as_str(), artist.as_str())) {
                        return false;
                    }
                    let label = format!("{name} {artist}");
                    self.query
                        .text()
                        .split_whitespace()
                        .all(|word| self.needles[word].contained_in(&label))
                })
                .count()
        };
        let eligible: Arc<[i64]> = rows
            .iter()
            .filter_map(|index| {
                let row = catalog.row(*index);
                let allowed =
                    !row.source_id
                        .is_some_and(|source| blocked.contains(&source))
                        || files.get(&row.id).is_some_and(|song| {
                            *observed.entry(row.id).or_insert_with(|| {
                                availability(library, song, &sources, &mut grants)
                            }) == FileAvailability::OnDevice
                        });
                allowed.then_some(row.id)
            })
            .collect();
        let pinned = !eligible.is_empty()
            && eligible
                .iter()
                .all(|id| catalog.row(catalog.row_index(*id).unwrap()).pinned);
        let summary = FindSummary::new(&self.raw, rows.len(), albums, folders, eligible, pinned);
        Found { rows, summary }
    }
}

fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_owned()
}
fn format(song: &Song) -> String {
    if let Some(format) = &song.file_extension {
        return format.clone();
    }
    if let Some(path) = &song.relative_path {
        return extension(path);
    }
    if let Some(path) = &song.local_file_path {
        return extension(path);
    }
    song.bookmark
        .as_deref()
        .and_then(|data| Access::resolve(data).ok())
        .map_or_else(String::new, |grant| {
            extension(grant.path().to_str().unwrap_or(""))
        })
}
fn source_grant(library: &LibraryWorker, source: &Source) -> Option<Access> {
    match source.bookmark.as_deref() {
        Some(bookmark) => Access::resolve(bookmark).ok(),
        None => library
            .preferences
            .bookmark(hash(&file_path(&source.dir_path).ok()?).ok()?)
            .ok()?
            .as_deref()
            .and_then(|bookmark| Access::resolve(bookmark).ok()),
    }
}
pub(super) fn availability(
    library: &LibraryWorker,
    song: &Song,
    sources: &HashMap<i64, Option<Source>>,
    grants: &mut HashMap<i64, Option<Access>>,
) -> FileAvailability {
    let inspect = |path: &Path, known| {
        audio::probe(path, known).map_or(FileAvailability::Unknown, |info| {
            match info.availability {
                audio::Availability::OnDevice => FileAvailability::OnDevice,
                audio::Availability::InCloud => FileAvailability::InCloud,
                audio::Availability::Unknown => FileAvailability::Unknown,
            }
        })
    };
    if let Some(stored) = &song.local_file_path {
        let path = if Path::new(stored).is_absolute() {
            Ok(stored.into())
        } else {
            lw_core::file_access::source_file(&library.paths.documents, stored)
        };
        if path.is_ok_and(|path| inspect(&path, true) == FileAvailability::OnDevice) {
            return FileAvailability::OnDevice;
        }
    }
    if let (Some(id), Some(relative)) = (song.source_id, song.relative_path.as_deref()) {
        let grant = grants.entry(id).or_insert_with(|| {
            sources
                .get(&id)
                .and_then(Option::as_ref)
                .and_then(|source| source_grant(library, source))
        });
        if let Some(grant) = grant
            && let Ok(file) = lw_core::file_access::source_file(grant.path(), relative)
        {
            return inspect(&file, false);
        }
    }
    if let Some(grant) = song
        .bookmark
        .as_deref()
        .and_then(|data| Access::resolve(data).ok())
    {
        return inspect(grant.path(), false);
    }
    FileAvailability::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    use lw_core::FileState;
    use lw_ui::{
        browse::{Query, Request, Scope, Sort},
        catalog::Route,
    };

    #[test]
    fn find_filters_use_shared_metadata_history_formats_and_real_resource_availability() {
        let base = std::env::temp_dir().join(format!("lw-find-filters-{}", std::process::id()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let documents = base.join("Documents");
        let root = base.join("Originals");
        std::fs::create_dir_all(documents.join("Music")).unwrap();
        std::fs::create_dir_all(root.join("Disc")).unwrap();
        for file in [
            documents.join("Music/cached.flac"),
            root.join("Disc/track.MP3"),
            root.join("legacy.AIFF"),
            base.join("outside.flac"),
        ] {
            std::fs::write(file, [7, 8, 9]).unwrap();
        }
        let mut worker = LibraryWorker::open(
            &base.join("library.sqlite"),
            Preferences::open(Some(&format!(
                "com.olegpustovit.localwave.find-filters-{}",
                std::process::id()
            )))
            .unwrap(),
            AppPaths {
                documents,
                caches: base.join("Caches"),
                temporary: base.join("tmp"),
                test_database: None,
            },
        )
        .unwrap();
        let user = worker.repository.get_or_create_user(1).unwrap();
        let source = worker
            .repository
            .create_source(&Source {
                dir_path: root.to_str().unwrap().into(),
                path_id: 1,
                user_id: user.id,
                bookmark: Some(lw_apple_sys::bookmarks::create(&root).unwrap()),
                ..Default::default()
            })
            .unwrap()
            .id
            .unwrap();
        let inaccessible = worker
            .repository
            .create_source(&Source {
                dir_path: base.join("Disconnected").to_str().unwrap().into(),
                path_id: 2,
                user_id: user.id,
                bookmark: Some(vec![0]),
                ..Default::default()
            })
            .unwrap()
            .id
            .unwrap();
        let legacy = lw_apple_sys::bookmarks::create(&root.join("legacy.AIFF")).unwrap();
        let mut songs = [
            Song {
                title: "Café Live".into(),
                file_extension: Some("FLAC".into()),
                local_file_path: Some("Music/cached.flac".into()),
                source_availability: Some(FileAvailability::InCloud),
                ..Default::default()
            },
            Song {
                title: "Other".into(),
                album_artist: "Café Collective".into(),
                source_id: Some(source),
                relative_path: Some("Disc/track.MP3".into()),
                local_file_path: Some("Music/cached.flac".into()),
                ..Default::default()
            },
            Song {
                title: "Bookmarked".into(),
                bookmark: Some(legacy.clone()),
                ..Default::default()
            },
            Song {
                title: "Local".into(),
                local_file_path: Some("Music/cached.flac".into()),
                ..Default::default()
            },
            Song {
                title: "Explicit empty".into(),
                file_extension: Some("".into()),
                relative_path: Some("track.flac".into()),
                local_file_path: Some("Music/cached.flac".into()),
                ..Default::default()
            },
            Song {
                title: "Relative empty".into(),
                relative_path: Some("".into()),
                local_file_path: Some("Music/cached.flac".into()),
                ..Default::default()
            },
            Song {
                title: "Escaping".into(),
                source_id: Some(source),
                relative_path: Some("../outside.flac".into()),
                file_extension: Some("flac".into()),
                ..Default::default()
            },
            Song {
                title: "Missing".into(),
                source_id: Some(source),
                relative_path: Some("missing.flac".into()),
                source_availability: Some(FileAvailability::OnDevice),
                ..Default::default()
            },
            Song {
                title: "Fallback".into(),
                source_id: Some(inaccessible),
                relative_path: Some("legacy.flac".into()),
                bookmark: Some(legacy),
                ..Default::default()
            },
            Song {
                title: "Untrusted cache".into(),
                file_state: FileState::Copied,
                local_file_path: Some("../outside.flac".into()),
                ..Default::default()
            },
            Song {
                title: "Year literal year:no".into(),
                ..Default::default()
            },
        ];
        for (index, song) in songs.iter_mut().enumerate() {
            song.song_key = index as i64 + 1;
            song.created_at = index as f64 + 1.0;
            song.release_year = (index < 6).then_some(2019);
            song.is_favorite = i64::from(index % 2 == 0);
            let invalid = song
                .relative_path
                .as_ref()
                .filter(|path| !lw_core::models::valid_relative_path(path))
                .cloned();
            if invalid.is_some() {
                song.relative_path = None;
            }
            *song = worker.repository.upsert_song(song, 1.0).unwrap();
            if let Some(path) = invalid {
                // Simulate pre-validation legacy rows without weakening the production write boundary.
                lw_apple_sys::sqlite::Connection::open(
                    base.join("library.sqlite").to_str().unwrap(),
                )
                .unwrap()
                .execute(
                    "UPDATE songs SET relativePath=? WHERE id=?",
                    &[
                        lw_apple_sys::sqlite::Value::Text(&path),
                        lw_apple_sys::sqlite::Value::Integer(song.id.unwrap()),
                    ],
                )
                .unwrap();
            }
        }
        let id = |index: usize| songs[index].id.unwrap();
        worker.preferences.record_playback(id(0), 1.0).unwrap();
        let catalog = worker.catalog().unwrap();
        let find = |raw: &str| {
            let prepared = worker
                .catalog_view(
                    Some(&catalog),
                    Some(Request {
                        scope: Scope::Find,
                        query: Query::new(raw).unwrap(),
                        sort: Sort::Original,
                        ascending: true,
                    }),
                )
                .unwrap();
            assert!(Arc::ptr_eq(&catalog.library, &prepared.library));
            prepared.collection(Route::Find).unwrap().ids.to_vec()
        };
        assert_eq!(
            find("cafe"),
            [id(1), id(0)],
            "album-artist text is included, with localized accents"
        );
        assert_eq!(find("cafe live favorites FLAC YEAR:+2019"), [id(0)]);
        assert!(find("cafe live unplayed").is_empty());
        assert_eq!(find("cafe UNPLAYED"), [id(1)]);
        assert_eq!(find("aiff"), [id(2)]);
        assert_eq!(
            find("mp3"),
            [id(1)],
            "relative extension precedes local cache extension"
        );
        assert_eq!(find("flac offline"), [id(8), id(3), id(0)]);
        assert_eq!(
            find("offline"),
            [id(8), id(5), id(4), id(3), id(2), id(1), id(0)],
            "saved availability and Copied state are not file proof"
        );
        assert!(find("cloud").is_empty());
        assert!(find("cloud offline").is_empty());
        assert!(find("flac mp3").is_empty());
        assert_eq!(
            find("year:no"),
            [id(10)],
            "invalid year words still use localized metadata matching"
        );
        assert!(find("  \t").is_empty());
        let query = Find::new(&worker, "cafe fav year:2019").unwrap();
        let candidates = worker
            .repository
            .read_snapshot(|repository| query.prepare(repository, &catalog))
            .unwrap();
        assert!(
            candidates.files.is_empty() && candidates.sources.is_empty(),
            "metadata-only Find must not rehydrate files or open grants"
        );
        assert_eq!(candidates.rows.len(), 1);
        assert_eq!(
            worker.repository.source(source).unwrap().unwrap().dir_path,
            root.to_str().unwrap()
        );
    }
    #[test]
    fn find_summary_counts_matching_albums_and_owned_directories_and_excludes_blocked_offline_actions()
     {
        let base = std::env::temp_dir().join(format!("lw-find-summary-{}", std::process::id()));
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
                "com.olegpustovit.localwave.find-summary-{}",
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
        let source = worker
            .repository
            .create_source(&Source {
                dir_path: "/Music".into(),
                path_id: 1,
                user_id: user.id,
                sync_error: Some("Reconnect".into()),
                ..Default::default()
            })
            .unwrap()
            .id
            .unwrap();
        let foreign = worker
            .repository
            .create_source(&Source {
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
                    source_id: source,
                    path_id: 11,
                    name: "Café shows".into(),
                    relative_path: "Café shows".into(),
                    is_directory: true,
                    ..Default::default()
                },
                lw_core::SourcePath {
                    source_id: source,
                    path_id: 12,
                    name: "Café.flac".into(),
                    relative_path: "Café.flac".into(),
                    ..Default::default()
                },
                lw_core::SourcePath {
                    source_id: foreign,
                    path_id: 21,
                    name: "Café other".into(),
                    relative_path: "Café other".into(),
                    is_directory: true,
                    ..Default::default()
                },
            ])
            .unwrap();
        std::fs::create_dir_all(base.join("Documents/Music")).unwrap();
        std::fs::write(base.join("Documents/Music/cached.flac"), [7, 8, 9]).unwrap();
        let mut ids = Vec::new();
        for (index, (album, source_id, availability)) in [
            ("Café album", None, None),
            ("Other", None, None),
            ("Café album", Some(source), None),
            ("Café album", Some(source), Some(FileAvailability::OnDevice)),
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
                            title: "Café track".into(),
                            artist: "Artist".into(),
                            album: album.into(),
                            source_id,
                            source_availability: availability,
                            local_file_path: (index == 3).then(|| "Music/cached.flac".into()),
                            file_extension: Some("flac".into()),
                            ..Default::default()
                        },
                        1.0,
                    )
                    .unwrap()
                    .id
                    .unwrap(),
            );
        }
        for (raw, expected) in [
            ("cafe", "4 songs · 1 albums · 1 folders"),
            ("cafe flac", "4 songs · 1 albums"),
            ("flac", "4 songs"),
            ("", "0 songs"),
        ] {
            let catalog = worker
                .catalog_view(
                    None,
                    Some(Request {
                        scope: Scope::Find,
                        query: Query::new(raw).unwrap(),
                        sort: Sort::Original,
                        ascending: true,
                    }),
                )
                .unwrap();
            let summary = catalog.view.as_ref().unwrap().find.as_ref().unwrap();
            assert_eq!(summary.title, format!("“{raw}”"));
            assert_eq!(summary.summary, expected);
            if !raw.is_empty() {
                assert_eq!(summary.eligible.as_ref(), &[ids[0], ids[1], ids[3]]);
            } else {
                assert!(summary.eligible.is_empty());
            }
            assert!(!summary.pinned);
            if raw == "cafe" {
                let mut state = lw_ui::ShellState::default();
                state.apply(lw_ui::Intent::Navigate(Route::Find));
                state.apply(lw_ui::Intent::Search(Query::new(raw).unwrap()));
                let mut ui =
                    pxui::Ui::create(pxui::Platform::Apple, Some(15), &[lw_ui::FONT, lw_ui::MONO]);
                ui.set_a11y(true);
                ui.frame(
                    &pxui::FrameIn {
                        width: 1440.0,
                        height: 872.0,
                        reduced_motion: 1,
                        ..Default::default()
                    },
                    &mut |ui| {
                        lw_ui::shell(
                            ui,
                            &lw_ui::PlayerView::default(),
                            &catalog,
                            &state,
                            &mut None,
                        )
                    },
                )
                .unwrap();
                assert!(
                    ui.a11y_nodes()
                        .iter()
                        .any(|node| ui.a11y_name_of(node) == expected),
                    "Find displays the worker's matching album/folder summary"
                );
            }
        }
        worker.preferences.set_pins(&[ids[0], ids[1], ids[3]], true);
        let catalog = worker
            .catalog_view(
                None,
                Some(Request {
                    scope: Scope::Find,
                    query: Query::new("cafe").unwrap(),
                    sort: Sort::Original,
                    ascending: true,
                }),
            )
            .unwrap();
        assert!(catalog.view.as_ref().unwrap().find.as_ref().unwrap().pinned);
        assert!(
            worker.toggle_pins(&[ids[2]]).is_err(),
            "disconnected uncached songs cannot be pinned"
        );
        assert!(
            !worker.toggle_pins(&ids[..3]).unwrap(),
            "blocked songs do not change the bulk toggle target"
        );
        let pins = worker.preferences.import_snapshot();
        assert!(
            !pins.pinned(ids[0])
                && !pins.pinned(ids[1])
                && !pins.pinned(ids[2])
                && pins.pinned(ids[3])
        );
        assert!(worker.toggle_pins(&[ids[0], 9000]).is_err());
        assert!(
            !worker.preferences.import_snapshot().pinned(ids[0]),
            "invalid selections cannot partly change preferences"
        );
        assert_eq!(
            worker
                .repository
                .get_song(ids[0])
                .unwrap()
                .unwrap()
                .file_state,
            FileState::BookmarkOnly
        );
        worker.preferences.set_pins(&ids, false);
    }
}
