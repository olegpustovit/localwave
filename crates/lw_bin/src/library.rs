use lw_apple_sys::bookmarks::Access;
use lw_apple_sys::preferences::{AppPaths, Preferences, file_path};
use lw_apple_sys::{normalized_file_url, scan::Scan};
use lw_core::{LibraryRepository, Source, SourcePath, hash_string};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

mod artwork;
pub use artwork::{ArtworkReady, ArtworkWorker};
mod cache;
mod catalog;
mod files;
mod find;
mod playback;
mod preferences;
mod waveform;
pub use cache::CopySummary;
pub use playback::PreparedPlayback;

struct Root {
    bookmark: Vec<u8>,
    access: Access,
}

/// Construct on the worker: SQLite, defaults and security-scoped grants stay on that thread.
pub struct LibraryWorker {
    pub repository: LibraryRepository,
    preferences: Preferences,
    paths: AppPaths,
    excluded: Vec<PathBuf>,
    roots: HashMap<i64, Root>,
    waveforms: Option<lw_core::waveform::Repository>,
    playback_files: HashMap<u64, playback::Lease>,
    active_playback: Option<u64>,
}

fn db(error: lw_apple_sys::sqlite::Error) -> io::Error {
    io::Error::other(error)
}
fn text(path: &Path) -> io::Result<&str> {
    path.to_str()
        .ok_or_else(|| io::Error::other("File path is not UTF-8"))
}
fn hash(path: &Path) -> io::Result<i64> {
    Ok(hash_string(&normalized_file_url(text(path)?)?))
}
fn check(cancelled: &AtomicBool) -> io::Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        Err(io::ErrorKind::Interrupted.into())
    } else {
        Ok(())
    }
}

impl LibraryWorker {
    pub fn current() -> io::Result<Self> {
        let paths = AppPaths::load()?;
        std::fs::create_dir_all(&paths.documents)?;
        let database = if cfg!(debug_assertions) {
            paths.test_database.clone()
        } else {
            None
        }
        .unwrap_or_else(|| paths.documents.join("musicApp29.sqlite"));
        Self::open(&database, Preferences::current()?, paths)
    }
    pub fn import_paths(
        &mut self,
        selected: &[SourcePath],
        now: f64,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(&SourcePath, usize, usize),
    ) -> io::Result<lw_core::import::Summary> {
        check(cancelled)?;
        let sources: std::collections::HashSet<_> =
            selected.iter().map(|path| path.source_id).collect();
        for source in sources {
            check(cancelled)?;
            self.resolve_source(source, now)?;
        }
        let mut files = files::Files {
            roots: &self.roots,
            documents: &self.paths.documents,
            preferences: self.preferences.import_snapshot(),
        };
        self.repository
            .import_paths(selected, &mut files, cancelled, now, progress)
    }

    pub fn open(database: &Path, preferences: Preferences, paths: AppPaths) -> io::Result<Self> {
        lw_apple_sys::require_worker_thread()?;
        if !database.is_absolute() {
            return Err(io::Error::other("Library database path must be absolute"));
        }
        let mut excluded = paths.generated();
        let database = file_path(text(database)?)?;
        excluded.push(PathBuf::from(format!("{}-wal", database.display())));
        excluded.push(PathBuf::from(format!("{}-shm", database.display())));
        excluded.push(database.clone());
        Ok(Self {
            repository: LibraryRepository::open(text(&database)?).map_err(db)?,
            preferences,
            paths,
            excluded,
            roots: HashMap::new(),
            waveforms: None,
            playback_files: HashMap::new(),
            active_playback: None,
        })
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn resolve_source(&mut self, source_id: i64, now: f64) -> io::Result<Source> {
        if !now.is_finite() {
            return Err(io::Error::other("Source access date must be finite"));
        }
        let Some(source) = self.repository.source(source_id).map_err(db)? else {
            self.roots.remove(&source_id);
            return Err(io::Error::other("This music source was removed"));
        };
        let registered = file_path(&source.dir_path)?;
        let bookmark = match &source.bookmark {
            Some(data) => data.clone(),
            None => self
                .preferences
                .bookmark(hash(&registered)?)?
                .ok_or_else(|| {
                    io::Error::other("Folder access is missing. Reconnect its source folder.")
                })?,
        };
        if self
            .roots
            .get(&source_id)
            .is_some_and(|root| root.bookmark == bookmark && root.access.path().is_dir())
        {
            return Ok(source);
        }
        self.roots.remove(&source_id);
        let access = Access::resolve(&bookmark)?;
        // Validate the root and exclusions before persisting a renewed or relocated bookmark.
        drop(Scan::open(&access, &registered, &self.excluded)?);
        let original = registered
            .canonicalize()
            .unwrap_or_else(|_| registered.clone());
        let moved = original != access.path().canonicalize()?;
        let mut resolved = source.clone();
        resolved.bookmark = Some(access.renewed_bookmark().unwrap_or(&bookmark).to_vec());
        let mut locations = Vec::new();
        if moved {
            resolved.dir_path = text(access.path())?.to_owned();
            resolved.path_id = hash(access.path())?;
            for path in self
                .repository
                .source_paths(source_id, None)
                .map_err(db)?
                .into_iter()
                .filter(|path| !path.is_directory)
            {
                if !lw_core::models::valid_relative_path(&path.relative_path) {
                    return Err(io::Error::other("Invalid legacy source-relative path"));
                }
                for identity in [
                    path.path_id,
                    hash(&registered.join(&path.relative_path))?,
                    hash(&access.path().join(&path.relative_path))?,
                ] {
                    locations.push((identity, path.relative_path.clone()));
                }
            }
        }
        let saved = if moved || resolved.bookmark != source.bookmark {
            self.repository
                .save_source_access(&source, &resolved, &locations, now)
                .map_err(db)?
        } else {
            source
        };
        self.roots.insert(
            source_id,
            Root {
                bookmark: saved
                    .bookmark
                    .clone()
                    .ok_or_else(|| io::Error::other("Source bookmark was not saved"))?,
                access,
            },
        );
        Ok(saved)
    }

    pub fn source_file(&mut self, source_id: i64, relative: &str, now: f64) -> io::Result<PathBuf> {
        self.resolve_source(source_id, now)?;
        lw_core::file_access::source_file(self.roots[&source_id].access.path(), relative)
    }

    /// Cancellation, failed enumeration and stale source versions never publish a partial index.
    pub fn scan_source(
        &mut self,
        source_id: i64,
        now: f64,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(&SourcePath),
    ) -> io::Result<Source> {
        check(cancelled)?;
        let source = self.resolve_source(source_id, now)?;
        let registered = file_path(&source.dir_path)?;
        let root = &self.roots[&source_id].access;
        let mut scan = Scan::open(root, &registered, &self.excluded)?;
        let mut paths = Vec::new();
        while let Some(entry) = scan.next(cancelled)? {
            let path = SourcePath {
                source_id,
                path_id: hash_string(&entry.normalized_url),
                parent_path_id: Some(hash_string(&entry.normalized_parent)),
                name: entry.name,
                relative_path: entry.relative_path,
                is_directory: entry.is_directory,
                file_size: entry.file_size,
                ..SourcePath::default()
            };
            progress(&path);
            check(cancelled)?;
            paths.push(path);
        }
        check(cancelled)?;
        self.repository
            .replace_source_scan(&source, paths, now)
            .map_err(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_core::Song;

    fn flac_with_picture(png: &[u8]) -> Vec<u8> {
        let mut flac =
            include_bytes!("../../../Tests/LocalWaveDataTests/Fixtures/tagged.flac").to_vec();
        let mut position = 4;
        loop {
            let end = position
                + 4
                + u32::from_be_bytes([
                    0,
                    flac[position + 1],
                    flac[position + 2],
                    flac[position + 3],
                ]) as usize;
            if flac[position] & 0x80 != 0 {
                flac[position] &= 0x7f;
                let mut picture = Vec::new();
                picture.extend_from_slice(&3u32.to_be_bytes());
                picture.extend_from_slice(&9u32.to_be_bytes());
                picture.extend_from_slice(b"image/png");
                for value in [0, 16, 16, 32, 0, png.len() as u32] {
                    picture.extend_from_slice(&value.to_be_bytes());
                }
                picture.extend_from_slice(png);
                let mut block = vec![0x86];
                block.extend_from_slice(&(picture.len() as u32).to_be_bytes()[1..]);
                block.extend(picture);
                flac.splice(end..end, block);
                return flac;
            }
            position = end;
        }
    }

    #[test]
    fn real_worker_scans_cancels_and_migrates_a_moved_legacy_source_without_losing_song_ids() {
        let base = std::env::temp_dir().join(format!(
            "lw-worker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let original = base.join("Original Music");
        std::fs::create_dir_all(original.join("Album")).unwrap();
        let cover = include_bytes!(
            "../../../localwave/Assets.xcassets/MacAppIcon.appiconset/icon_16x16.png"
        );
        let flac = flac_with_picture(cover);
        std::fs::write(original.join("Album/01 track.flac"), &flac).unwrap();
        std::fs::write(original.join(".hidden.flac"), [1]).unwrap();
        let paths = AppPaths {
            documents: base.join("Documents"),
            caches: base.join("Caches"),
            temporary: base.join("tmp"),
            test_database: None,
        };
        std::fs::create_dir_all(paths.documents.join("CoverArt")).unwrap();
        let suite = format!(
            "com.olegpustovit.localwave.worker-test-{}",
            std::process::id()
        );
        let preferences = Preferences::open(Some(&suite)).unwrap();
        let key = hash(&original).unwrap();
        preferences
            .set_bookmark(
                key,
                Some(&lw_apple_sys::bookmarks::create(&original).unwrap()),
            )
            .unwrap();
        let mut worker =
            LibraryWorker::open(&base.join("library.sqlite"), preferences, paths).unwrap();
        let user = worker.repository.get_or_create_user(1).unwrap();
        let source = worker
            .repository
            .create_source(&Source {
                dir_path: normalized_file_url(text(&original).unwrap()).unwrap(),
                path_id: key,
                user_id: user.id,
                created_at: 100.0,
                ..Source::default()
            })
            .unwrap();
        let id = source.id.unwrap();
        let cancelled = AtomicBool::new(false);
        let first = worker
            .scan_source(id, 200.0, &cancelled, &mut |_| {})
            .unwrap();
        assert_eq!(first.total_paths, Some(1));
        assert!(first.bookmark.is_some());
        let indexed = worker.repository.source_paths(id, None).unwrap();
        assert_eq!(indexed.len(), 2);
        let path = indexed.iter().find(|path| !path.is_directory).unwrap();
        let legacy = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 99,
                    path_hash: path.path_id,
                    title: "Keep".into(),
                    is_favorite: 1,
                    ..Song::default()
                },
                200.0,
            )
            .unwrap();
        let list = worker.repository.create_playlist("Keep", 200.0).unwrap();
        worker
            .repository
            .add_playlist_songs(list.id, &[legacy.id.unwrap()])
            .unwrap();
        std::fs::write(
            original.join("new.m4a"),
            include_bytes!("../../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a"),
        )
        .unwrap();
        assert_eq!(
            worker
                .scan_source(id, 201.0, &cancelled, &mut |_| cancelled
                    .store(true, Ordering::Relaxed))
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(worker.repository.source_paths(id, None).unwrap(), indexed);
        assert_eq!(worker.repository.source(id).unwrap(), Some(first));
        cancelled.store(false, Ordering::Relaxed);
        let moved = base.join("Moved Music");
        std::fs::rename(&original, &moved).unwrap();
        let second = worker
            .scan_source(id, 202.0, &cancelled, &mut |_| {})
            .unwrap();
        assert_eq!(second.total_paths, Some(2));
        assert_eq!(
            file_path(&second.dir_path).unwrap().canonicalize().unwrap(),
            moved.canonicalize().unwrap()
        );
        let rescanned = worker
            .repository
            .source_path(id, &path.relative_path)
            .unwrap()
            .unwrap();
        assert_eq!(rescanned.id, path.id);
        assert_ne!(rescanned.path_id, path.path_id);
        let saved = worker
            .repository
            .get_song(legacy.id.unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(saved.source_id, Some(id));
        assert_eq!(saved.relative_path, Some(path.relative_path.clone()));
        assert_eq!(saved.is_favorite, 1);
        assert_eq!(
            worker.repository.playlist_songs(list.id).unwrap()[0].id,
            legacy.id
        );
        assert_eq!(
            std::fs::read(worker.source_file(id, &path.relative_path, 203.0).unwrap()).unwrap(),
            flac
        );
        assert!(worker.source_file(id, "../library.sqlite", 203.0).is_err());
        worker.preferences.set_pinned(legacy.id.unwrap(), true);
        let selection = worker.repository.source_paths(id, None).unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            + 1.0;
        assert_eq!(
            worker
                .import_paths(&selection, now, &cancelled, &mut |_, _, _| {})
                .unwrap()
                .imported,
            2
        );
        let imported = worker
            .repository
            .get_song(legacy.id.unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(imported.title, "FLAC Fixture");
        assert_eq!(imported.album_artist, "Test Ensemble");
        assert_eq!(
            (
                imported.release_year,
                imported.track_number,
                imported.disc_number
            ),
            (Some(2026), Some(3), Some(2))
        );
        assert_eq!(imported.is_favorite, 1);
        assert_eq!(imported.file_state, lw_core::FileState::CopyPending);
        assert!((imported.duration.unwrap() - 0.15).abs() < 0.02);
        let cover_path = imported.cover_art_path.unwrap();
        assert_eq!(
            std::fs::read(worker.paths.documents.join(&cover_path)).unwrap(),
            cover
        );
        assert_eq!(
            lw_apple_sys::artwork::store(&worker.paths.documents, cover).unwrap(),
            Some(cover_path)
        );
        assert_eq!(
            lw_apple_sys::artwork::store(&worker.paths.documents, b"not image data").unwrap(),
            None
        );
        assert_eq!(
            std::fs::read_dir(worker.paths.documents.join("CoverArt"))
                .unwrap()
                .count(),
            2
        );
        assert_eq!(
            worker.repository.playlist_songs(list.id).unwrap()[0].id,
            legacy.id
        );
        let m4a = worker
            .repository
            .search_songs("M4A Fixture", 10, 0)
            .unwrap();
        assert_eq!(m4a.len(), 1);
        assert_eq!(m4a[0].album_artist, "AAC Ensemble");
        assert_eq!(
            worker
                .import_paths(&selection, now + 1.0, &cancelled, &mut |_, _, _| {})
                .unwrap()
                .skipped,
            2
        );
        worker.preferences.set_pinned(legacy.id.unwrap(), false);
        let provider = base.join("Library/CloudStorage/Provider");
        std::fs::create_dir_all(&provider).unwrap();
        let provider_file = provider.join("cloud.m4a");
        std::fs::copy(moved.join("new.m4a"), &provider_file).unwrap();
        let cloud = worker
            .repository
            .create_source(&Source {
                dir_path: text(&provider).unwrap().into(),
                path_id: hash(&provider).unwrap(),
                bookmark: Some(lw_apple_sys::bookmarks::create(&provider).unwrap()),
                user_id: user.id,
                created_at: now,
                ..Source::default()
            })
            .unwrap()
            .id
            .unwrap();
        worker
            .scan_source(cloud, now, &cancelled, &mut |_| {})
            .unwrap();
        let cloud_paths = worker.repository.source_paths(cloud, None).unwrap();
        assert_eq!(
            worker
                .import_paths(&cloud_paths, now, &cancelled, &mut |_, _, _| {})
                .unwrap()
                .deferred,
            1
        );
        let placeholder = worker
            .repository
            .song_at_source_path(cloud, "cloud.m4a")
            .unwrap()
            .unwrap();
        let cloud_id = placeholder.id.unwrap();
        assert_eq!(placeholder.duration, None);
        assert_eq!(
            placeholder.source_availability,
            Some(lw_core::FileAvailability::Unknown)
        );
        worker.repository.set_favorite(cloud_id, true).unwrap();
        worker
            .repository
            .add_playlist_songs(list.id, &[cloud_id])
            .unwrap();
        let digest = worker
            .repository
            .source_path(id, "new.m4a")
            .unwrap()
            .unwrap()
            .file_hash_sha256
            .unwrap();
        let relative_cache = format!("Music/{}.m4a", lw_core::import::hex_digest(&digest));
        let cache = worker.paths.documents.join(&relative_cache);
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        std::fs::copy(moved.join("new.m4a"), &cache).unwrap();
        let version = lw_apple_sys::audio::probe(&provider_file, false).unwrap();
        assert!(version.provider);
        worker
            .preferences
            .set_cache_version(cloud_id, Some(&cache), version.modified, version.size)
            .unwrap();
        assert!(
            worker
                .repository
                .update_cached_file(
                    cloud_id,
                    Some(&relative_cache),
                    lw_core::FileState::Copied,
                    placeholder.file_state,
                    None
                )
                .unwrap()
        );
        assert_eq!(
            worker
                .import_paths(&cloud_paths, now + 1.0, &cancelled, &mut |_, _, _| {})
                .unwrap()
                .imported,
            1
        );
        let enriched = worker.repository.get_song(cloud_id).unwrap().unwrap();
        assert_eq!(enriched.title, "M4A Fixture");
        assert_eq!(enriched.is_favorite, 1);
        assert_eq!(
            enriched.local_file_path.as_deref(),
            Some(relative_cache.as_str())
        );
        assert!(
            worker
                .repository
                .playlist_songs(list.id)
                .unwrap()
                .iter()
                .any(|song| song.id == Some(cloud_id))
        );
        worker.preferences.set_pinned(cloud_id, true);
        worker
            .preferences
            .set_cache_version(
                cloud_id,
                Some(&cache),
                version.modified.map(|date| date + 1.0),
                version.size,
            )
            .unwrap();
        assert_eq!(
            worker
                .import_paths(&cloud_paths, now + 2.0, &cancelled, &mut |_, _, _| {})
                .unwrap()
                .deferred,
            1
        );
        let invalidated = worker.repository.get_song(cloud_id).unwrap().unwrap();
        assert_eq!(invalidated.local_file_path, None);
        assert_eq!(invalidated.file_state, lw_core::FileState::CopyPending);
        assert!(cache.exists());
        worker.preferences.set_pinned(cloud_id, false);
        worker
            .preferences
            .set_cache_version(cloud_id, None, None, None)
            .unwrap();
        use lw_core::context::PlaybackContext;
        use lw_core::preferences::{Appearance, PlayerState, Settings, Signal};
        let contexts = [
            PlaybackContext::Library,
            PlaybackContext::Album {
                name: "Album".into(),
                artist: "Björk".into(),
            },
            PlaybackContext::Artist("Artist".into()),
            PlaybackContext::Playlist {
                id: i64::MAX,
                name: "Playlist".into(),
            },
            PlaybackContext::Folder {
                source_id: i64::MIN,
                path_id: Some(0),
                name: "Folder".into(),
            },
            PlaybackContext::Folder {
                source_id: 1,
                path_id: None,
                name: "Root".into(),
            },
            PlaybackContext::Automatic("Automatic".into()),
            PlaybackContext::Selection("Selection\0名前".into()),
        ];
        let titles = [
            "Library",
            "Album",
            "Artist",
            "Playlist",
            "Folder",
            "Root",
            "Automatic",
            "Selection\0名前",
        ];
        for (context, title) in contexts.iter().zip(titles) {
            let encoded = worker.playback_context_data(context).unwrap();
            assert_eq!(worker.playback_context(Some(&encoded)), *context);
            assert_eq!(context.title(), title);
        }
        assert_eq!(contexts[0].text_len(), 0);
        assert_eq!(contexts[1].text_len(), 11);
        assert_eq!(worker.playback_context(None), PlaybackContext::Library);
        assert_eq!(
            worker.playback_context(Some(&[0, 255])),
            PlaybackContext::Library
        );
        let settings = Settings {
            appearance: Appearance::Dark,
            signal: Signal::Green,
            cache_gb: 16,
            ..Settings::default()
        };
        worker.save_settings(&settings).unwrap();
        assert_eq!(worker.load_settings(), settings);
        assert!(
            worker
                .save_settings(&Settings {
                    cache_gb: 3,
                    ..settings
                })
                .is_err()
        );
        assert_eq!(worker.load_settings(), settings);
        let player = PlayerState {
            song_ids: vec![legacy.id.unwrap(), legacy.id.unwrap(), m4a[0].id.unwrap()],
            current_index: 1,
            volume: Some(0.0),
            // Unknown future cases display Library while their original bytes remain intact.
            context_data: Some(br#"{"future":{"name":"Keep raw"}}"#.to_vec()),
        };
        worker.save_player_state(&player).unwrap();
        let restored = worker.load_player_state().unwrap();
        assert_eq!(restored, player);
        assert_eq!(
            worker.playback_context(restored.context_data.as_deref()),
            PlaybackContext::Library
        );
        assert_eq!(
            worker
                .repository
                .restore_queue(&restored)
                .unwrap()
                .current_index,
            Some(1)
        );
        assert!(
            worker
                .save_player_state(&PlayerState {
                    current_index: 99,
                    ..player.clone()
                })
                .is_err()
        );
        assert_eq!(worker.load_player_state().unwrap(), player);
        worker.save_player_state(&PlayerState::default()).unwrap();
        worker.save_settings(&Settings::default()).unwrap();
        worker.repository.delete_source(cloud).unwrap();
        assert!(worker.resolve_source(cloud, now + 3.0).is_err());
        worker.repository.delete_source(id).unwrap();
        assert!(worker.resolve_source(id, 204.0).is_err());
        assert!(worker.roots.is_empty());
        worker.preferences.set_bookmark(key, None).unwrap();
    }
}
