use super::{LibraryWorker, check, db};
use lw_apple_sys::{audio, player::Prepared};
use lw_core::Song;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

pub struct PreparedPlayback {
    pub item: Prepared,
    pub song: Song,
    pub path: PathBuf,
}

pub(super) struct Lease {
    song_id: i64,
    path: PathBuf,
    counted: bool,
}

impl LibraryWorker {
    pub fn may_prepare_ahead(&mut self, song_id: i64, now: f64) -> io::Result<bool> {
        if !now.is_finite() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        if self.load_settings().download_ahead {
            return Ok(true);
        }
        let Some(song) = self.repository.get_song(song_id).map_err(db)? else {
            return Ok(false);
        };
        let mut grant = None;
        // Unconfirmed or inaccessible future files defer until selected; they must not trigger a download.
        Ok(matches!(
            self.readable_song_file(&song, now, &mut grant),
            Ok(Some(_))
        ))
    }
    /// Called for an actual native start, before acknowledging retirement of that physical token.
    pub fn record_playback_start(&mut self, token: u64, now: f64) -> io::Result<bool> {
        if !now.is_finite() || now < 0.0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid playback date",
            ));
        }
        let Some(lease) = self.playback_files.get_mut(&token) else {
            return Ok(false);
        };
        if lease.counted {
            return Ok(false);
        }
        self.preferences.record_playback(lease.song_id, now)?;
        lease.counted = true;
        // Recency must never prevent playback or count the same start twice after an IO error.
        if let Ok(root) = super::cache::music_root(&self.paths.documents) {
            lw_apple_sys::history::touch_cache_file(&lease.path, &root, now);
        }
        Ok(true)
    }
    pub fn playback_history(&self) -> lw_apple_sys::history::History {
        self.preferences.history()
    }
    /// Release only after the main thread has removed the corresponding native item.
    pub fn release_playback(&mut self, token: u64) -> io::Result<bool> {
        if self.active_playback == Some(token) {
            return Err(io::Error::other(
                "Stop or adopt another item before releasing the active playback file",
            ));
        }
        let Some(lease) = self.playback_files.remove(&token) else {
            return Ok(false);
        };
        if let Err(error) = self.sync_playback_protection() {
            self.playback_files.insert(token, lease);
            return Err(error);
        }
        Ok(true)
    }
    pub fn set_active_playback(&mut self, token: Option<u64>) -> io::Result<()> {
        if token.is_some_and(|token| !self.playback_files.contains_key(&token)) {
            return Err(io::Error::other("Unknown active playback token"));
        }
        let previous = self.active_playback;
        self.active_playback = token;
        if let Err(error) = self.sync_playback_protection() {
            self.active_playback = previous;
            return Err(error);
        }
        Ok(())
    }
    /// The native player must be stopped before clearing the final cache leases.
    pub fn release_all_playback(&mut self) -> io::Result<()> {
        self.preferences.set_cache_protection(None, None, &[])?;
        self.active_playback = None;
        self.playback_files.clear();
        Ok(())
    }
    fn materialize_for_playback(
        &mut self,
        song: &Song,
        now: f64,
        cancelled: &AtomicBool,
    ) -> io::Result<PathBuf> {
        let (copied, saved) = self.materialize_song(song, now, cancelled, &mut |_, _| {})?;
        if !saved {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Song cache changed during playback preparation",
            ));
        }
        Ok(copied.path)
    }
    fn sync_playback_protection(&self) -> io::Result<()> {
        let active = self
            .active_playback
            .and_then(|token| self.playback_files.get(&token));
        let paths: Vec<_> = self
            .playback_files
            .values()
            .map(|lease| lease.path.clone())
            .collect();
        self.preferences.set_cache_protection(
            active.map(|lease| lease.song_id),
            active.map(|lease| lease.path.as_path()),
            &paths,
        )
    }
    pub fn prepare_playback(
        &mut self,
        song_id: i64,
        token: u64,
        occurrence: u64,
        now: f64,
        cancelled: &AtomicBool,
    ) -> io::Result<PreparedPlayback> {
        check(cancelled)?;
        if token == 0
            || occurrence == 0
            || !now.is_finite()
            || self.playback_files.contains_key(&token)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid or duplicate playback request",
            ));
        }
        // Current + two upcoming files, plus one replacement batch awaiting native retirement.
        if self.playback_files.len() == 6 {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let song = self
            .repository
            .get_song(song_id)
            .map_err(db)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "This song was removed"))?;
        let docs = self.paths.documents.canonicalize().ok();
        let candidate = song
            .local_file_path
            .as_ref()
            .map(|stored| {
                if std::path::Path::new(stored).is_absolute() {
                    Ok(PathBuf::from(stored))
                } else {
                    lw_core::file_access::source_file(&self.paths.documents, stored)
                }
            })
            .transpose();
        let candidate = match candidate {
            Ok(candidate) => candidate,
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        let local = candidate
            .and_then(|path| path.canonicalize().ok())
            .filter(|path| path.is_file());
        let path = if let Some(local) = local {
            let attempt = (|| -> io::Result<PathBuf> {
                let private = docs.as_ref().is_some_and(|docs| local.starts_with(docs));
                let info = audio::probe(&local, private)?;
                if self.preferences.no_network()
                    && info.availability != audio::Availability::OnDevice
                {
                    return Err(io::ErrorKind::NotConnected.into());
                }
                if info.availability == audio::Availability::InCloud
                    || (info.provider && info.availability == audio::Availability::Unknown)
                {
                    return Err(io::Error::new(
                        io::ErrorKind::NotConnected,
                        "Playback file is not confirmed local",
                    ));
                }
                if private {
                    return Ok(local);
                }
                let copied = lw_apple_sys::copy::copy(
                    &local,
                    &super::cache::music_root(&self.paths.documents)?,
                    &self.preferences,
                    cancelled,
                )?;
                check(cancelled)?;
                if !self.save_materialized(&song, &copied)? {
                    return Err(io::Error::new(
                        io::ErrorKind::Interrupted,
                        "Song cache changed during playback preparation",
                    ));
                }
                Ok(copied.path)
            })();
            match attempt {
                Err(error)
                    if error.kind() != io::ErrorKind::Interrupted
                        && (song.bookmark.is_some()
                            || (song.source_id.is_some() && song.relative_path.is_some())) =>
                {
                    self.materialize_for_playback(&song, now, cancelled)?
                }
                result => result?,
            }
        } else {
            self.materialize_for_playback(&song, now, cancelled)?
        };
        let path = path.canonicalize()?;
        check(cancelled)?;
        let song = self
            .repository
            .get_song(song_id)
            .map_err(db)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "This song was removed"))?;
        self.playback_files.insert(
            token,
            Lease {
                song_id,
                path: path.clone(),
                counted: false,
            },
        );
        if let Err(error) = self.sync_playback_protection() {
            self.playback_files.remove(&token);
            return Err(error);
        }
        match Prepared::open(&path, token, occurrence, true, cancelled) {
            Ok(item) => Ok(PreparedPlayback { item, song, path }),
            Err(error) => {
                self.playback_files.remove(&token);
                self.sync_playback_protection()?;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    use lw_core::{FileState, Source};
    use std::path::Path;
    use std::sync::atomic::Ordering;
    #[test]
    fn playback_materializes_bookmarks_and_leases_duplicate_files_until_native_retirement() {
        let base = std::env::temp_dir().join(format!(
            "lw-playback-files-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let original = base.join("Originals");
        std::fs::create_dir_all(&original).unwrap();
        std::fs::create_dir_all(base.join("Documents/Added Files")).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a");
        let input = original.join("track.m4a");
        std::fs::copy(&fixture, &input).unwrap();
        let suite = format!(
            "com.olegpustovit.localwave.playback-{}",
            base.file_name().unwrap().to_str().unwrap()
        );
        let preferences = Preferences::open(Some(&suite)).unwrap();
        let second = Preferences::open(Some(&suite)).unwrap();
        let mut worker = LibraryWorker::open(
            &base.join("library.sqlite"),
            preferences,
            AppPaths {
                documents: base.join("Documents"),
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
                dir_path: super::super::text(&original).unwrap().into(),
                path_id: super::super::hash(&original).unwrap(),
                user_id: user.id,
                bookmark: Some(lw_apple_sys::bookmarks::create(&original).unwrap()),
                ..Source::default()
            })
            .unwrap()
            .id
            .unwrap();
        let song = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 1,
                    source_id: Some(source),
                    relative_path: Some("track.m4a".into()),
                    title: "Preserved title".into(),
                    ..Song::default()
                },
                1.0,
            )
            .unwrap();
        let id = song.id.unwrap();
        worker.repository.set_favorite(id, true).unwrap();
        let list = worker.repository.create_playlist("Keep", 1.0).unwrap();
        worker
            .repository
            .add_playlist_songs(list.id, &[id])
            .unwrap();
        let cancel = AtomicBool::new(false);
        let first = worker.prepare_playback(id, 100, 1, 2.0, &cancel).unwrap();
        assert!(
            first
                .path
                .starts_with(worker.paths.documents.canonicalize().unwrap().join("Music"))
        );
        assert_eq!(first.song.file_state, FileState::Copied);
        assert_eq!(first.song.is_favorite, 1);
        assert_eq!(first.song.title, "Preserved title");
        assert_eq!(
            worker.repository.playlist_songs(list.id).unwrap()[0].id,
            Some(id)
        );
        assert!(
            second
                .cache_protection()
                .path_protected(&first.path)
                .unwrap()
        );
        assert_eq!(worker.enforce_cache_bytes(0).unwrap().files, 0);
        assert_eq!(worker.playback_history().get(id).plays, 0);
        let history = worker.playback_history();
        mem_sys::local_reset();
        for _ in 0..10000 {
            std::hint::black_box(history.get(id));
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        let original_modified = input.metadata().unwrap().modified().unwrap();
        assert!(worker.record_playback_start(100, 100.125).unwrap());
        assert!(!worker.record_playback_start(100, 101.0).unwrap());
        assert_eq!(
            worker.playback_history().get(id),
            lw_apple_sys::history::Record {
                plays: 1,
                last_played: Some(100.125)
            }
        );
        assert_eq!(
            first
                .path
                .metadata()
                .unwrap()
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            100.125
        );
        assert_eq!(
            input.metadata().unwrap().modified().unwrap(),
            original_modified
        );
        worker.set_active_playback(Some(100)).unwrap();
        let duplicate = worker.prepare_playback(id, 101, 2, 2.0, &cancel).unwrap();
        assert_eq!(duplicate.path, first.path);
        assert_eq!(worker.playback_history().get(id).plays, 1);
        assert!(worker.record_playback_start(101, 102.5).unwrap());
        assert_eq!(worker.playback_history().get(id).plays, 2);
        assert!(worker.record_playback_start(101, f64::NAN).is_err());
        assert!(worker.release_playback(100).is_err());
        worker.set_active_playback(Some(101)).unwrap();
        assert!(worker.release_playback(100).unwrap());
        assert!(!worker.record_playback_start(100, 103.0).unwrap());
        assert_eq!(worker.enforce_cache_bytes(0).unwrap().files, 0);
        worker.set_active_playback(None).unwrap();
        worker.release_playback(101).unwrap();
        assert_eq!(worker.enforce_cache_bytes(0).unwrap().files, 1);
        assert!(!first.path.exists());
        drop((first, duplicate));
        let legacy = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 2,
                    bookmark: Some(lw_apple_sys::bookmarks::create(&input).unwrap()),
                    ..Song::default()
                },
                3.0,
            )
            .unwrap();
        let prepared = worker
            .prepare_playback(legacy.id.unwrap(), 102, 3, 3.0, &cancel)
            .unwrap();
        assert_eq!(prepared.song.file_state, FileState::Copied);
        drop(prepared);
        worker.release_playback(102).unwrap();
        let direct = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 3,
                    local_file_path: Some(super::super::text(&input).unwrap().into()),
                    file_state: FileState::Copied,
                    ..Song::default()
                },
                3.0,
            )
            .unwrap();
        let prepared = worker
            .prepare_playback(direct.id.unwrap(), 103, 4, 3.0, &cancel)
            .unwrap();
        assert_ne!(prepared.path, input);
        assert!(input.exists());
        drop(prepared);
        worker.release_playback(103).unwrap();
        let added = worker.paths.documents.join("Added Files/track.m4a");
        std::fs::copy(&input, &added).unwrap();
        let private = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 4,
                    local_file_path: Some("Added Files/track.m4a".into()),
                    file_state: FileState::Copied,
                    ..Song::default()
                },
                3.0,
            )
            .unwrap();
        let prepared = worker
            .prepare_playback(private.id.unwrap(), 104, 5, 3.0, &cancel)
            .unwrap();
        assert_eq!(prepared.path, added.canonicalize().unwrap());
        let added_modified = added.metadata().unwrap().modified().unwrap();
        assert!(worker.record_playback_start(104, 105.0).unwrap());
        assert_eq!(
            added.metadata().unwrap().modified().unwrap(),
            added_modified
        );
        drop(prepared);
        worker.release_playback(104).unwrap();
        let broken = worker.paths.documents.join("Added Files/broken.m4a");
        std::fs::write(&broken, b"not audio").unwrap();
        let bad = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 5,
                    local_file_path: Some("Added Files/broken.m4a".into()),
                    file_state: FileState::Copied,
                    ..Song::default()
                },
                3.0,
            )
            .unwrap();
        assert!(
            worker
                .prepare_playback(bad.id.unwrap(), 105, 6, 3.0, &cancel)
                .is_err()
        );
        assert!(worker.playback_files.is_empty());
        cancel.store(true, Ordering::Relaxed);
        assert_eq!(
            worker
                .prepare_playback(id, 106, 7, 3.0, &cancel)
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::Interrupted
        );
        cancel.store(false, Ordering::Relaxed);
        let provider = base.join("Library/CloudStorage/Provider");
        std::fs::create_dir_all(&provider).unwrap();
        std::fs::copy(&input, provider.join("track.m4a")).unwrap();
        let cloud = worker
            .repository
            .create_source(&Source {
                dir_path: super::super::text(&provider).unwrap().into(),
                path_id: super::super::hash(&provider).unwrap(),
                user_id: user.id,
                bookmark: Some(lw_apple_sys::bookmarks::create(&provider).unwrap()),
                ..Source::default()
            })
            .unwrap()
            .id
            .unwrap();
        let cloud = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 6,
                    source_id: Some(cloud),
                    relative_path: Some("track.m4a".into()),
                    ..Song::default()
                },
                3.0,
            )
            .unwrap();
        worker.preferences.set_no_network(true);
        let mut settings = worker.load_settings();
        settings.download_ahead = false;
        worker.save_settings(&settings).unwrap();
        assert!(!worker.may_prepare_ahead(cloud.id.unwrap(), 3.0).unwrap());
        assert!(worker.may_prepare_ahead(private.id.unwrap(), 3.0).unwrap());
        assert!(worker.may_prepare_ahead(id, 3.0).unwrap());
        settings.download_ahead = true;
        worker.save_settings(&settings).unwrap();
        assert!(
            worker
                .prepare_playback(cloud.id.unwrap(), 107, 8, 3.0, &cancel)
                .is_err()
        );
        assert!(worker.playback_files.is_empty());
        let fallback = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 7,
                    source_id: Some(source),
                    relative_path: Some("track.m4a".into()),
                    local_file_path: Some(
                        provider.join("track.m4a").to_string_lossy().into_owned(),
                    ),
                    file_state: FileState::Copied,
                    ..Song::default()
                },
                3.0,
            )
            .unwrap();
        let prepared = worker
            .prepare_playback(fallback.id.unwrap(), 108, 9, 3.0, &cancel)
            .unwrap();
        assert!(
            prepared
                .path
                .starts_with(worker.paths.documents.canonicalize().unwrap())
        );
        drop(prepared);
        worker.release_playback(108).unwrap();
        for token in 110..116 {
            drop(
                worker
                    .prepare_playback(private.id.unwrap(), token, token, 3.0, &cancel)
                    .unwrap(),
            );
        }
        assert_eq!(
            worker
                .prepare_playback(private.id.unwrap(), 116, 116, 3.0, &cancel)
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::WouldBlock
        );
        worker.release_all_playback().unwrap();
        worker.preferences.set_no_network(false);
        let moved_child = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 8,
                    source_id: Some(source),
                    relative_path: Some("missing-relative.m4a".into()),
                    bookmark: Some(lw_apple_sys::bookmarks::create(&input).unwrap()),
                    ..Song::default()
                },
                3.0,
            )
            .unwrap();
        let prepared = worker
            .prepare_playback(moved_child.id.unwrap(), 117, 117, 3.0, &cancel)
            .unwrap();
        assert!(prepared.path.is_file());
        assert_eq!(second.history().get(id).plays, 2);
        drop(prepared);
        worker.release_playback(117).unwrap();
    }
}
