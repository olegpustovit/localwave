use super::{LibraryWorker, check, db, text};
use lw_apple_sys::bookmarks::Access;
use lw_apple_sys::preferences::{CacheProtection, Preferences};
use lw_core::cache::{self, EvictionIO, File, Owner};
use lw_core::{FileState, LibraryRepository, Song};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::UNIX_EPOCH;

#[derive(Debug, Default)]
pub struct CopySummary {
    pub copied: usize,
    pub skipped: usize,
    pub failed: Vec<(i64, io::Error)>,
}

pub(super) fn music_root(documents: &Path) -> io::Result<PathBuf> {
    if !documents.is_absolute() {
        return Err(io::Error::other("Cache documents path must be absolute"));
    }
    let documents = match documents.canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == io::ErrorKind::NotFound => documents.to_owned(),
        Err(error) => return Err(error),
    };
    let root = documents.join("Music");
    match root.symlink_metadata() {
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            return Err(io::Error::other(
                "Managed Music cache must be a real directory",
            ));
        }
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    Ok(root)
}
fn files(root: &Path) -> io::Result<Vec<File>> {
    let directory = match std::fs::read_dir(root) {
        Ok(directory) => directory,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(error),
    };
    let mut files = Vec::new();
    for entry in directory {
        let entry = entry?;
        if !entry.file_type()?.is_file()
            || entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "partial")
        {
            continue;
        }
        let metadata = entry.metadata()?;
        let last_use = metadata
            .modified()
            .map(|time| match time.duration_since(UNIX_EPOCH) {
                Ok(elapsed) => elapsed.as_secs_f64(),
                Err(error) => -error.duration().as_secs_f64(),
            })
            .unwrap_or(f64::MIN);
        files.push(File {
            path: entry.path(),
            bytes: metadata.len(),
            last_use,
        });
    }
    Ok(files)
}

struct Evict<'a> {
    repository: &'a LibraryRepository,
    preferences: &'a Preferences,
    protection: CacheProtection,
    owners: HashMap<PathBuf, Vec<Owner>>,
    root: PathBuf,
}
impl EvictionIO for Evict<'_> {
    fn remove_if_unprotected(&mut self, file: &File) -> io::Result<bool> {
        {
            let _guard = self.preferences.cache_guard();
            if !self.protection.is_current() {
                self.protection = self.preferences.cache_protection();
            }
            let owners = self.owners.get(&file.path);
            if self.protection.path_protected(&file.path)?
                || owners.is_some_and(|owners| {
                    owners
                        .iter()
                        .any(|owner| self.protection.owner_protected(owner.song_id))
                })
            {
                return Ok(false);
            }
            if file.path.parent() != Some(self.root.as_path()) {
                return Err(io::Error::other(
                    "Cache eviction escaped the managed Music directory",
                ));
            }
            let root = self.root.symlink_metadata()?;
            if !root.is_dir()
                || root.file_type().is_symlink()
                || self.root.canonicalize()? != self.root
            {
                return Err(io::Error::other(
                    "Managed Music cache moved during eviction",
                ));
            }
            match file.path.symlink_metadata() {
                Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
                    return Ok(false);
                }
                Ok(_) => match std::fs::remove_file(&file.path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                },
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        if let Some(owners) = self.owners.get(&file.path) {
            for owner in owners {
                if self
                    .repository
                    .update_cached_file(
                        owner.song_id,
                        None,
                        FileState::BookmarkOnly,
                        owner.state,
                        Some(&owner.stored_path),
                    )
                    .map_err(db)?
                {
                    self.preferences
                        .set_cache_version(owner.song_id, None, None, None)?;
                }
            }
        }
        Ok(true)
    }
}

impl LibraryWorker {
    /// Swift's bulk pin toggle: persist one preference update and queue uncopied songs together.
    pub fn toggle_pins(&mut self, ids: &[i64]) -> io::Result<bool> {
        if ids.is_empty() || ids.len() > 1_000_000 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let ids = ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let songs = self.repository.get_songs(&ids).map_err(db)?;
        if songs.len() != ids.len() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Some selected songs were removed",
            ));
        }
        let mut sources = HashMap::new();
        for song in &songs {
            if let Some(id) = song.source_id
                && let std::collections::hash_map::Entry::Vacant(entry) = sources.entry(id)
            {
                entry.insert(self.repository.source(id).map_err(db)?);
            }
        }
        let mut grants = HashMap::new();
        let ids = songs
            .iter()
            .filter(|song| {
                !song
                    .source_id
                    .and_then(|id| sources.get(&id))
                    .and_then(|source| source.as_ref())
                    .is_some_and(|source| source.sync_error.is_some())
                    || super::find::availability(self, song, &sources, &mut grants)
                        == lw_core::FileAvailability::OnDevice
            })
            .map(|song| song.id.unwrap())
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Err(io::Error::other(
                "Reconnect this music source before keeping its songs offline",
            ));
        }
        let _guard = self.preferences.cache_guard();
        let before = self.preferences.import_snapshot();
        let pinned = !ids.iter().all(|id| before.pinned(*id));
        if pinned {
            self.repository.mark_songs_for_copy(&ids).map_err(db)?;
        }
        self.preferences.set_pins(&ids, pinned);
        Ok(pinned)
    }

    pub fn cached_byte_count(&self) -> io::Result<u64> {
        cache::total_bytes(&files(&music_root(&self.paths.documents)?)?)
    }
    pub fn enforce_cache_limit(&self) -> io::Result<cache::Eviction> {
        self.enforce_cache_bytes(self.preferences.cache_protection().limit_bytes())
    }
    pub fn enforce_cache_bytes(&self, max_bytes: u64) -> io::Result<cache::Eviction> {
        let root = music_root(&self.paths.documents)?;
        let mut files = files(&root)?;
        let total = cache::total_bytes(&files)?;
        if total <= max_bytes {
            return Ok(cache::Eviction {
                files: 0,
                remaining_bytes: total,
            });
        }
        let mut owners: HashMap<PathBuf, Vec<Owner>> = HashMap::new();
        for owner in self.repository.cache_owners().map_err(db)? {
            let stored = Path::new(&owner.stored_path);
            let path = if stored.is_absolute() {
                stored.to_owned()
            } else {
                self.paths.documents.join(stored)
            };
            let path = match path.canonicalize() {
                Ok(path) => path,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            if path.parent() == Some(root.as_path()) {
                owners.entry(path).or_default().push(owner);
            }
        }
        let mut eviction = Evict {
            repository: &self.repository,
            preferences: &self.preferences,
            protection: self.preferences.cache_protection(),
            owners,
            root,
        };
        cache::evict(&mut files, max_bytes, &mut eviction)
    }

    /// Materialization and eviction are serialized on this worker; never invoke either on a frame.
    pub fn process_copy_queue(
        &mut self,
        now: f64,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(i64, u64),
    ) -> io::Result<CopySummary> {
        check(cancelled)?;
        if !now.is_finite() {
            return Err(io::Error::other("Copy date must be finite"));
        }
        let songs = self.repository.songs_needing_copy().map_err(db)?;
        let pinned = self.preferences.import_snapshot();
        let mut summary = CopySummary::default();
        for song in songs {
            check(cancelled)?;
            let id = song
                .id
                .ok_or_else(|| io::Error::other("Unsaved copy request"))?;
            if song.file_state == FileState::Failed && !pinned.pinned(id) {
                summary.skipped += 1;
                continue;
            }
            match self.materialize_song(&song, now, cancelled, progress) {
                Ok((_, true)) => summary.copied += 1,
                Ok((_, false)) => summary.skipped += 1,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => return Err(error),
                Err(error) => {
                    self.repository
                        .update_cached_file(
                            id,
                            song.local_file_path.as_deref(),
                            FileState::Failed,
                            song.file_state,
                            song.local_file_path.as_deref(),
                        )
                        .map_err(db)?;
                    summary.failed.push((id, error));
                }
            }
        }
        self.enforce_cache_limit()?;
        Ok(summary)
    }

    pub(super) fn materialize_song(
        &mut self,
        song: &Song,
        now: f64,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(i64, u64),
    ) -> io::Result<(lw_apple_sys::copy::CopiedFile, bool)> {
        let id = song
            .id
            .ok_or_else(|| io::Error::other("Unsaved copy request"))?;
        let mut legacy = None;
        let source = match (song.source_id, &song.relative_path) {
            (Some(source), Some(relative)) => self.source_file(source, relative, now),
            _ => Err(io::Error::other("No source folder is available")),
        };
        let directory = music_root(&self.paths.documents)?;
        let mut copy = |source: &Path| {
            lw_apple_sys::copy::copy_with(
                source,
                &directory,
                &self.preferences,
                cancelled,
                &mut |bytes| {
                    progress(id, bytes);
                    Ok(())
                },
            )
        };
        let copied = source
            .and_then(|source| copy(&source))
            .or_else(|source_error| {
                if source_error.kind() == io::ErrorKind::Interrupted {
                    return Err(source_error);
                }
                let Some(bookmark) = song.bookmark.as_deref() else {
                    return Err(source_error);
                };
                let access = Access::resolve(bookmark)?;
                let copied = copy(access.path())?;
                legacy = Some(access);
                Ok(copied)
            })?;
        check(cancelled)?;
        let saved = self.save_materialized(song, &copied)?;
        if saved
            && let (Some(expected), Some(renewed)) = (
                song.bookmark.as_deref(),
                legacy.as_ref().and_then(Access::renewed_bookmark),
            )
        {
            self.repository
                .renew_song_bookmark(id, expected, renewed)
                .map_err(db)?;
        }
        Ok((copied, saved))
    }

    pub(super) fn save_materialized(
        &self,
        song: &Song,
        copied: &lw_apple_sys::copy::CopiedFile,
    ) -> io::Result<bool> {
        let id = song
            .id
            .ok_or_else(|| io::Error::other("Unsaved copy request"))?;
        let filename = copied
            .path
            .file_name()
            .ok_or_else(|| io::Error::other("Copied file has no name"))?;
        let relative = Path::new("Music").join(filename);
        let _guard = self.preferences.cache_guard();
        let saved = self
            .repository
            .update_cached_file(
                id,
                Some(text(&relative)?),
                FileState::Copied,
                song.file_state,
                song.local_file_path.as_deref(),
            )
            .map_err(db)?;
        if saved {
            self.preferences.set_cache_version(
                id,
                Some(&copied.path),
                copied.modified,
                copied.size,
            )?;
        }
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::AppPaths;
    use lw_core::Source;
    use std::fs;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, SystemTime};

    #[test]
    fn worker_copies_only_requests_retries_pins_and_evicts_only_unprotected_managed_files() {
        let root = std::env::temp_dir().join(format!(
            "lw-cache-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let documents = root.join("Documents");
        let music = documents.join("Music");
        let added = documents.join("Added Files");
        fs::create_dir_all(&music).unwrap();
        fs::create_dir_all(&added).unwrap();
        let original = added.join("original.flac");
        fs::write(&original, [7; 100]).unwrap();
        let prefs = Preferences::open(Some(&format!(
            "localwave-cache-test-{}",
            std::process::id()
        )))
        .unwrap();
        let mut worker = LibraryWorker::open(
            &root.join("library.sqlite"),
            prefs,
            AppPaths {
                documents: documents.clone(),
                caches: root.join("Caches"),
                temporary: root.join("tmp"),
                test_database: None,
            },
        )
        .unwrap();
        let mut songs = Vec::new();
        for (index, name) in ["old", "recent", "pinned", "active", "queued"]
            .into_iter()
            .enumerate()
        {
            let path = music.join(format!("{name}.flac"));
            fs::write(&path, [index as u8; 4]).unwrap();
            fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_times(
                    fs::FileTimes::new()
                        .set_modified(UNIX_EPOCH + Duration::from_secs(index as u64)),
                )
                .unwrap();
            songs.push(
                worker
                    .repository
                    .upsert_song(
                        &Song {
                            song_key: index as i64 + 1,
                            title: name.into(),
                            bookmark: Some(vec![42]),
                            local_file_path: Some(format!("Music/{name}.flac")),
                            file_state: FileState::Copied,
                            ..Song::default()
                        },
                        100.0,
                    )
                    .unwrap(),
            );
        }
        let alias = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 99,
                    title: "Other owner".into(),
                    local_file_path: songs[2].local_file_path.clone(),
                    file_state: FileState::Copied,
                    ..Song::default()
                },
                100.0,
            )
            .unwrap();
        let unowned = music.join("not-yet-in-database.flac");
        fs::write(&unowned, [8, 8, 8]).unwrap();
        fs::write(music.join("unfinished.partial"), [0; 100]).unwrap();
        fs::create_dir_all(music.join("Folder")).unwrap();
        fs::write(music.join("Folder/original.flac"), [6; 100]).unwrap();
        std::os::unix::fs::symlink(&original, music.join("original-link.flac")).unwrap();
        let protection = worker.preferences.cache_protection();
        worker.preferences.set_pinned(songs[2].id.unwrap(), true);
        assert!(!protection.is_current());
        let managed = music.canonicalize().unwrap();
        let protected_path = managed.join("pinned.flac");
        let mut eviction = Evict {
            repository: &worker.repository,
            preferences: &worker.preferences,
            protection,
            root: managed,
            owners: HashMap::from([(
                protected_path.clone(),
                vec![Owner {
                    song_id: songs[2].id.unwrap(),
                    stored_path: "Music/pinned.flac".into(),
                    state: FileState::Copied,
                }],
            )]),
        };
        assert!(
            !eviction
                .remove_if_unprotected(&File {
                    path: protected_path,
                    bytes: 4,
                    last_use: 2.0
                })
                .unwrap()
        );
        drop(eviction);
        worker
            .preferences
            .set_cache_protection(songs[3].id, Some(&unowned), &[music.join("queued.flac")])
            .unwrap();
        assert_eq!(worker.cached_byte_count().unwrap(), 23);
        let evicted = worker.enforce_cache_bytes(19).unwrap();
        assert_eq!((evicted.files, evicted.remaining_bytes), (1, 19));
        assert!(!music.join("old.flac").exists());
        let old = worker
            .repository
            .get_song(songs[0].id.unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(old.local_file_path, None);
        assert_eq!(old.file_state, FileState::BookmarkOnly);
        assert_eq!(old.bookmark, Some(vec![42]));
        let evicted = worker.enforce_cache_bytes(0).unwrap();
        assert_eq!((evicted.files, evicted.remaining_bytes), (1, 15));
        for name in [
            "pinned.flac",
            "active.flac",
            "queued.flac",
            "not-yet-in-database.flac",
        ] {
            assert!(music.join(name).exists());
        }
        assert_eq!(
            worker
                .repository
                .get_song(alias.id.unwrap())
                .unwrap()
                .unwrap()
                .local_file_path,
            songs[2].local_file_path
        );
        assert_eq!(fs::read(&original).unwrap(), [7; 100]);
        assert!(music.join("unfinished.partial").exists());
        assert!(music.join("Folder/original.flac").exists());
        let source = worker
            .repository
            .create_source(&Source {
                dir_path: text(&added).unwrap().into(),
                bookmark: Some(lw_apple_sys::bookmarks::create(&added).unwrap()),
                created_at: 100.0,
                ..Source::default()
            })
            .unwrap()
            .id
            .unwrap();
        let pending = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 1000,
                    title: "Pending".into(),
                    source_id: Some(source),
                    relative_path: Some("original.flac".into()),
                    ..Song::default()
                },
                100.0,
            )
            .unwrap();
        let cancelled = AtomicBool::new(false);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        assert_eq!(
            worker
                .process_copy_queue(now, &cancelled, &mut |_, _| {})
                .unwrap()
                .copied,
            0
        );
        worker
            .repository
            .mark_song_for_copy(pending.id.unwrap())
            .unwrap();
        let mut reported = 0;
        assert_eq!(
            worker
                .process_copy_queue(now, &cancelled, &mut |id, bytes| {
                    assert_eq!(id, pending.id.unwrap());
                    reported = bytes;
                })
                .unwrap()
                .copied,
            1
        );
        assert_eq!(reported, 100);
        let copied = worker
            .repository
            .get_song(pending.id.unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(copied.file_state, FileState::Copied);
        let copied_path = documents.join(copied.local_file_path.unwrap());
        assert_eq!(fs::read(&copied_path).unwrap(), [7; 100]);
        let version = lw_apple_sys::audio::probe(&original, false).unwrap();
        assert_eq!(
            worker
                .preferences
                .import_snapshot()
                .cache_matches(
                    pending.id.unwrap(),
                    &copied_path,
                    version.modified,
                    version.size
                )
                .unwrap(),
            Some(true)
        );
        fs::copy(&original, added.join("duplicate.flac")).unwrap();
        let duplicate = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 1001,
                    title: "Duplicate".into(),
                    source_id: Some(source),
                    relative_path: Some("duplicate.flac".into()),
                    file_state: FileState::CopyPending,
                    ..Song::default()
                },
                100.0,
            )
            .unwrap();
        let before = worker.cached_byte_count().unwrap();
        assert_eq!(
            worker
                .process_copy_queue(now, &cancelled, &mut |_, _| {})
                .unwrap()
                .copied,
            1
        );
        assert_eq!(worker.cached_byte_count().unwrap(), before);
        let bookmark = lw_apple_sys::bookmarks::create(&original).unwrap();
        let legacy = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 1004,
                    title: "Legacy file bookmark".into(),
                    bookmark: Some(bookmark.clone()),
                    file_state: FileState::CopyPending,
                    ..Song::default()
                },
                100.0,
            )
            .unwrap();
        assert_eq!(
            worker
                .process_copy_queue(now, &cancelled, &mut |_, _| {})
                .unwrap()
                .copied,
            1
        );
        assert_eq!(worker.cached_byte_count().unwrap(), before);
        assert_eq!(
            worker
                .repository
                .get_song(legacy.id.unwrap())
                .unwrap()
                .unwrap()
                .bookmark,
            Some(bookmark)
        );
        let bytes = vec![11; 768 * 1024 + 37];
        fs::write(added.join("cancel.flac"), &bytes).unwrap();
        let cancel_song = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 1002,
                    title: "Cancel".into(),
                    source_id: Some(source),
                    relative_path: Some("cancel.flac".into()),
                    file_state: FileState::CopyPending,
                    ..Song::default()
                },
                100.0,
            )
            .unwrap();
        assert_eq!(
            worker
                .process_copy_queue(now, &cancelled, &mut |_, _| cancelled
                    .store(true, Ordering::Relaxed))
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(
            worker
                .repository
                .get_song(cancel_song.id.unwrap())
                .unwrap()
                .unwrap()
                .file_state,
            FileState::CopyPending
        );
        assert_eq!(
            fs::read_dir(&music)
                .unwrap()
                .filter(|entry| entry
                    .as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "partial"))
                .count(),
            1
        );
        cancelled.store(false, Ordering::Relaxed);
        assert_eq!(
            worker
                .process_copy_queue(now, &cancelled, &mut |_, _| {})
                .unwrap()
                .copied,
            1
        );
        let missing = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 1003,
                    title: "Missing".into(),
                    source_id: Some(source),
                    relative_path: Some("missing.flac".into()),
                    file_state: FileState::CopyPending,
                    ..Song::default()
                },
                100.0,
            )
            .unwrap();
        assert_eq!(
            worker
                .process_copy_queue(now, &cancelled, &mut |_, _| {})
                .unwrap()
                .failed
                .len(),
            1
        );
        assert_eq!(
            worker
                .repository
                .get_song(missing.id.unwrap())
                .unwrap()
                .unwrap()
                .file_state,
            FileState::Failed
        );
        assert_eq!(
            worker
                .process_copy_queue(now, &cancelled, &mut |_, _| {})
                .unwrap()
                .skipped,
            1
        );
        worker.preferences.set_pinned(missing.id.unwrap(), true);
        assert_eq!(
            worker
                .process_copy_queue(now, &cancelled, &mut |_, _| {})
                .unwrap()
                .failed
                .len(),
            1
        );
        fs::copy(&original, added.join("missing.flac")).unwrap();
        assert_eq!(
            worker
                .process_copy_queue(now, &cancelled, &mut |_, _| {})
                .unwrap()
                .copied,
            1
        );
        worker.preferences.set_pinned(missing.id.unwrap(), false);
        worker.preferences.set_pinned(songs[2].id.unwrap(), false);
        worker
            .preferences
            .set_cache_protection(None, None, &[])
            .unwrap();
        assert_eq!(worker.enforce_cache_bytes(0).unwrap().remaining_bytes, 0);
        assert_eq!(worker.cached_byte_count().unwrap(), 0);
        assert_eq!(
            worker
                .repository
                .get_song(duplicate.id.unwrap())
                .unwrap()
                .unwrap()
                .file_state,
            FileState::BookmarkOnly
        );
        assert_eq!(fs::read(&original).unwrap(), [7; 100]);
        assert_eq!(fs::read(added.join("cancel.flac")).unwrap(), bytes);
        let moved = documents.join("Old Music");
        fs::rename(&music, &moved).unwrap();
        std::os::unix::fs::symlink(&added, &music).unwrap();
        assert!(worker.enforce_cache_bytes(0).is_err());
        assert_eq!(fs::read(&original).unwrap(), [7; 100]);
    }
}
