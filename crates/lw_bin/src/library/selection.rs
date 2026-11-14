//! Native audio selections become durable private files before indexing or playback.
use super::{LibraryWorker, check, db, text};
use lw_apple_sys::{
    audio,
    bookmarks::{self, Access},
};
use lw_core::{FileState, Source, context::PlaybackContext};
use std::{
    collections::HashSet,
    io,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

pub struct MusicSelection {
    pub source: Source,
    pub songs: Arc<[i64]>,
    pub context: Arc<PlaybackContext>,
}

impl LibraryWorker {
    pub fn add_audio(
        &mut self,
        user: i64,
        selected: &[PathBuf],
        now: f64,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(&str, usize, usize),
    ) -> io::Result<MusicSelection> {
        self.add_music(user, selected, false, now, cancelled, progress)
    }

    pub fn add_dropped(
        &mut self,
        user: i64,
        selected: &[PathBuf],
        now: f64,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(&str, usize, usize),
    ) -> io::Result<MusicSelection> {
        self.add_music(user, selected, true, now, cancelled, progress)
    }

    fn add_music(
        &mut self,
        user: i64,
        selected: &[PathBuf],
        dropped: bool,
        now: f64,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(&str, usize, usize),
    ) -> io::Result<MusicSelection> {
        check(cancelled)?;
        self.current_source_user(user)?;
        if selected.is_empty() || selected.len() > 1_000_000 || !now.is_finite() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // Validate the entire selection before creating any private files.
        let (mut folders, mut audio) = (Vec::new(), Vec::new());
        let mut bytes = 0usize;
        for path in selected {
            check(cancelled)?;
            bytes = bytes.saturating_add(text(path)?.len());
            if !path.is_absolute() || bytes > 64 * 1024 * 1024 {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            let info = std::fs::metadata(path)?;
            if dropped && info.is_dir() {
                drop(self.folder_access(path, now, cancelled)?);
                folders.push(path.as_path());
            } else if info.is_file() && lw_core::models::supports_audio(text(path)?) {
                audio.push(path);
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Choose supported audio files or original music folders.",
                ));
            }
        }
        let mut copied = Vec::with_capacity(audio.len());
        let directory = if audio.is_empty() {
            None
        } else {
            std::fs::create_dir_all(&self.paths.documents)?;
            let directory = self.paths.documents.canonicalize()?.join("Added Files");
            match directory.symlink_metadata() {
                Ok(info) if !info.is_dir() || info.file_type().is_symlink() => {
                    return Err(io::Error::other("Added Files must be a real directory"));
                }
                Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
                _ => {}
            }
            for (index, path) in audio.iter().enumerate() {
                check(cancelled)?;
                progress("Copying", index, audio.len());
                let bookmark = bookmarks::create(path)?;
                if bookmark.len() > 64 * 1024 * 1024 {
                    return Err(io::Error::other("Bookmark exceeds its memory limit"));
                }
                let access = Access::resolve(&bookmark)?;
                let file = lw_apple_sys::copy::copy_named(
                    access.path(),
                    &directory,
                    &self.preferences,
                    cancelled,
                )?;
                copied.push(
                    text(
                        file.path
                            .canonicalize()?
                            .strip_prefix(&directory)
                            .map_err(io::Error::other)?,
                    )?
                    .to_owned(),
                );
            }
            Some(directory)
        };
        let now = now.max(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_secs_f64(),
        );
        let mut sources = Vec::with_capacity(folders.len() + usize::from(directory.is_some()));
        for folder in folders.iter().copied().chain(directory.as_deref()) {
            let source = self.register_folder(user, folder, now, cancelled)?;
            let id = source
                .id
                .ok_or_else(|| io::Error::other("Source is not saved"))?;
            let mut indexed = 0;
            let source = self.scan_source(id, now, cancelled, &mut |_| {
                indexed += 1;
                progress("Indexing", indexed, 0);
            })?;
            let paths = self
                .repository
                .source_paths(id, Some(source.path_id))
                .map_err(db)?;
            self.import_paths(&paths, now, cancelled, &mut |_, completed, total| {
                progress("Importing", completed, total);
            })?;
            self.owned_source(user, id)?;
            check(cancelled)?;
            sources.push(source);
        }
        let mut songs = Vec::with_capacity(copied.len());
        let mut keys = HashSet::with_capacity(copied.len());
        if let Some(directory) = &directory {
            let id = sources
                .last()
                .and_then(|source| source.id)
                .ok_or_else(|| io::Error::other("Source is not saved"))?;
            for relative in copied {
                check(cancelled)?;
                let Some(song) = self
                    .repository
                    .song_at_source_path(id, &relative)
                    .map_err(db)?
                else {
                    // Identical song keys share a row; its final imported source path wins.
                    continue;
                };
                let song_id = song
                    .id
                    .ok_or_else(|| io::Error::other("Song is not saved"))?;
                let private = directory.join(&relative);
                let info = audio::probe(&private, true)?;
                let stored = PathBuf::from("Added Files").join(&relative);
                let _guard = self.preferences.cache_guard();
                if !self
                    .repository
                    .update_cached_file(
                        song_id,
                        Some(text(&stored)?),
                        FileState::Copied,
                        song.file_state,
                        song.local_file_path.as_deref(),
                    )
                    .map_err(db)?
                {
                    return Err(io::Error::new(
                        io::ErrorKind::Interrupted,
                        "The added track changed. Try again.",
                    ));
                }
                self.preferences.set_cache_version(
                    song_id,
                    Some(&private),
                    info.modified,
                    info.size,
                )?;
                if keys.insert(song.song_key) {
                    songs.push(song_id);
                }
            }
        }
        for source in sources.iter().take(folders.len()) {
            check(cancelled)?;
            let id = source
                .id
                .ok_or_else(|| io::Error::other("Source is not saved"))?;
            self.owned_source(user, id)?;
            let mut entries = Vec::new();
            for (id, key, path) in self.repository.indexed_source_songs(id).map_err(db)? {
                if lw_core::models::supports_audio(&path) {
                    entries.push((id, key, lw_apple_sys::collation::Key::new(&path)?));
                }
            }
            entries.sort_unstable_by(|a, b| a.2.compare(&b.2).then(a.0.cmp(&b.0)));
            for (id, key, _) in entries {
                check(cancelled)?;
                if keys.insert(key) {
                    if songs.len() == 1_000_000 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "The dropped selection exceeds the queue limit.",
                        ));
                    }
                    songs.push(id);
                }
            }
        }
        self.current_source_user(user)?;
        check(cancelled)?;
        if songs.is_empty() && !dropped {
            return Err(io::Error::other(
                "The selected audio files could not be imported.",
            ));
        }
        Ok(MusicSelection {
            source: sources
                .pop()
                .ok_or_else(|| io::Error::other("No music source was imported"))?,
            songs: songs.into(),
            context: Arc::new(PlaybackContext::Selection(
                if dropped {
                    "Dropped music"
                } else {
                    "Selected files"
                }
                .into(),
            )),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    use std::{fs, sync::atomic::Ordering};

    #[test]
    fn selected_audio_preserves_order_reuses_files_and_plays_private_content_without_cache_copy() {
        let base = std::env::temp_dir().join(format!("lw-audio-selection-{}", std::process::id()));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        fs::create_dir_all(base.join("Original")).unwrap();
        let m4a = base.join("Original/日本語.m4a");
        let flac = base.join("Original/Album.FLAC");
        fs::write(
            &m4a,
            include_bytes!("../../../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a"),
        )
        .unwrap();
        fs::write(
            &flac,
            include_bytes!("../../../../Tests/LocalWaveDataTests/Fixtures/tagged.flac"),
        )
        .unwrap();
        let preferences = Preferences::open(Some(&format!(
            "localwave-audio-selection.{}",
            std::process::id()
        )))
        .unwrap();
        let mut library = LibraryWorker::open(
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
        let user = library.current_sources().unwrap().user.id;
        let cancelled = AtomicBool::new(false);
        let mut phases = Vec::new();
        let selected = [m4a.clone(), flac.clone(), m4a.clone()];
        let result = library
            .add_audio(user, &selected, 100.0, &cancelled, &mut |phase, _, _| {
                phases.push(phase.to_owned());
            })
            .unwrap();
        assert_eq!(result.songs.len(), 2);
        assert_eq!(
            *result.context,
            PlaybackContext::Selection("Selected files".into())
        );
        assert!(phases.iter().any(|phase| phase == "Copying"));
        assert!(phases.iter().any(|phase| phase == "Indexing"));
        assert!(phases.iter().any(|phase| phase == "Importing"));
        let first = library
            .repository
            .get_song(result.songs[0])
            .unwrap()
            .unwrap();
        let second = library
            .repository
            .get_song(result.songs[1])
            .unwrap()
            .unwrap();
        assert_eq!(first.relative_path.as_deref(), Some("日本語.m4a"));
        assert_eq!(second.relative_path.as_deref(), Some("Album.FLAC"));
        assert_eq!(
            first.local_file_path.as_deref(),
            Some("Added Files/日本語.m4a")
        );
        assert_eq!(first.file_state, FileState::Copied);
        assert_eq!(first.source_id, result.source.id);
        library
            .repository
            .set_favorite(result.songs[0], true)
            .unwrap();
        let playlist = library.repository.create_playlist("Keep", 101.0).unwrap();
        library
            .repository
            .add_playlist_songs(playlist.id, &result.songs)
            .unwrap();
        let repeated = library
            .add_audio(user, &selected, 102.0, &cancelled, &mut |_, _, _| {})
            .unwrap();
        assert_eq!(repeated.songs, result.songs);
        assert_eq!(repeated.source.id, result.source.id);
        assert_eq!(
            fs::read_dir(base.join("Documents/Added Files"))
                .unwrap()
                .count(),
            2
        );
        assert_eq!(
            library
                .repository
                .get_song(result.songs[0])
                .unwrap()
                .unwrap()
                .is_favorite,
            1
        );
        assert_eq!(
            library
                .repository
                .playlist_songs(playlist.id)
                .unwrap()
                .len(),
            2
        );
        fs::rename(base.join("Original"), base.join("Moved original")).unwrap();
        let prepared = library
            .prepare_playback(result.songs[0], 901, 1, 103.0, &cancelled)
            .unwrap();
        assert_eq!(
            prepared.path,
            base.join("Documents/Added Files/日本語.m4a")
                .canonicalize()
                .unwrap()
        );
        assert!(!base.join("Documents/Music").exists());
        drop(prepared);
        library.release_playback(901).unwrap();
        let invalid = base.join("Moved original/notes.txt");
        fs::write(&invalid, b"not audio").unwrap();
        let fresh = base.join("Moved original/Fresh.flac");
        fs::copy(base.join("Moved original/Album.FLAC"), &fresh).unwrap();
        assert!(
            library
                .add_audio(
                    user,
                    &[fresh.clone(), invalid],
                    104.0,
                    &cancelled,
                    &mut |_, _, _| {}
                )
                .is_err()
        );
        assert!(!base.join("Documents/Added Files/Fresh.flac").exists());
        assert_eq!(
            library
                .add_audio(
                    user + 1,
                    std::slice::from_ref(&fresh),
                    104.0,
                    &cancelled,
                    &mut |_, _, _| {}
                )
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        cancelled.store(true, Ordering::Relaxed);
        assert_eq!(
            library
                .add_audio(user, &[fresh], 104.0, &cancelled, &mut |_, _, _| {})
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(
            fs::read_dir(base.join("Documents/Added Files"))
                .unwrap()
                .count(),
            2
        );
        cancelled.store(false, Ordering::Relaxed);
        let folder_a = base.join("Folder A");
        let folder_b = base.join("Folder B");
        fs::create_dir_all(folder_a.join("Album 2")).unwrap();
        fs::create_dir_all(folder_a.join("Album 10")).unwrap();
        fs::create_dir_all(&folder_b).unwrap();
        fn track(path: &std::path::Path, title: &[u8; 12]) {
            let mut bytes =
                include_bytes!("../../../../Tests/LocalWaveDataTests/Fixtures/tagged.flac")
                    .to_vec();
            let at = bytes
                .windows(12)
                .position(|text| text == b"FLAC Fixture")
                .unwrap();
            bytes[at..at + 12].copy_from_slice(title);
            fs::write(path, bytes).unwrap();
        }
        track(&folder_a.join("Album 2/2.flac"), b"Track two___");
        track(&folder_a.join("Album 10/10.flac"), b"Track ten___");
        track(&folder_b.join("3.flac"), b"Track end___");
        let dropped = library
            .add_dropped(
                user,
                &[
                    folder_a.clone(),
                    base.join("Moved original/日本語.m4a"),
                    folder_b.clone(),
                    base.join("Moved original/Album.FLAC"),
                ],
                200.0,
                &cancelled,
                &mut |_, _, _| {},
            )
            .unwrap();
        let titles = library
            .repository
            .get_songs(&dropped.songs)
            .unwrap()
            .into_iter()
            .map(|song| song.title)
            .collect::<Vec<_>>();
        assert_eq!(
            &titles[2..],
            &["Track two___", "Track ten___", "Track end___"]
        );
        assert_eq!(dropped.songs.len(), 5);
        assert_eq!(
            *dropped.context,
            PlaybackContext::Selection("Dropped music".into())
        );
        assert_eq!(dropped.songs[0], result.songs[0]);
        assert_eq!(dropped.songs[1], result.songs[1]);
        let empty = base.join("Empty folder");
        fs::create_dir(&empty).unwrap();
        assert!(
            library
                .add_dropped(user, &[empty], 201.0, &cancelled, &mut |_, _, _| {})
                .unwrap()
                .songs
                .is_empty()
        );
        let source_id = library
            .current_sources()
            .unwrap()
            .sources
            .iter()
            .find(|source| source.dir_path.ends_with("Folder A"))
            .unwrap()
            .id
            .unwrap();
        let before = library.repository.source_paths(source_id, None).unwrap();
        track(&folder_a.join("fresh.flac"), b"Track new___");
        let error = library
            .add_dropped(
                user,
                std::slice::from_ref(&folder_a),
                202.0,
                &cancelled,
                &mut |stage, _, _| {
                    if stage == "Indexing" {
                        cancelled.store(true, Ordering::Relaxed);
                    }
                },
            )
            .err()
            .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(
            library.repository.source_paths(source_id, None).unwrap(),
            before
        );
        cancelled.store(false, Ordering::Relaxed);
        let unregistered = base.join("Not registered");
        fs::create_dir(&unregistered).unwrap();
        let fresh = base.join("Fresh.flac");
        track(&fresh, b"Track new___");
        let invalid = base.join("unsupported.txt");
        fs::write(&invalid, b"not music").unwrap();
        assert!(
            library
                .add_dropped(
                    user,
                    &[unregistered.clone(), fresh.clone(), invalid],
                    203.0,
                    &cancelled,
                    &mut |_, _, _| {}
                )
                .is_err()
        );
        assert!(
            !library
                .current_sources()
                .unwrap()
                .sources
                .iter()
                .any(|source| source.dir_path == unregistered.to_str().unwrap())
        );
        assert!(!base.join("Documents/Added Files/Fresh.flac").exists());
        let generated = base.join("Documents/CoverArt");
        fs::create_dir_all(&generated).unwrap();
        assert!(
            library
                .add_dropped(
                    user,
                    &[fresh, generated],
                    204.0,
                    &cancelled,
                    &mut |_, _, _| {}
                )
                .is_err()
        );
    }
}
