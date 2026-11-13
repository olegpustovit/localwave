use super::{LibraryWorker, hash};
use lw_apple_sys::{
    audio, bookmarks::Access, collation::Key, history::History, preferences::file_path,
};
use lw_core::{FileAvailability, LibraryRepository, Song, Source, query::SongQuery};
use lw_ui::catalog::Catalog;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;

pub(super) struct Find {
    query: SongQuery,
    needles: HashMap<String, Key>,
    history: Option<History>,
    empty: bool,
}
#[derive(Default)]
pub(super) struct Candidates {
    rows: Vec<usize>,
    files: HashMap<i64, Song>,
    sources: HashMap<i64, Option<Source>>,
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
            if self.query.needs_availability() || (self.query.needs_format() && !row.has_format) {
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
    ) -> Vec<usize> {
        let Candidates {
            mut rows,
            files,
            sources,
        } = candidates;
        let mut grants = HashMap::new();
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
                || self.query.matches_availability(availability(
                    library,
                    song,
                    &sources,
                    &mut grants,
                ))
        });
        rows
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
fn availability(
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
    use std::sync::Arc;

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
}
