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

pub struct AudioSelection {
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
    ) -> io::Result<AudioSelection> {
        check(cancelled)?;
        self.current_source_user(user)?;
        if selected.is_empty() || selected.len() > 1_000_000 || !now.is_finite() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // Validate the entire selection before creating any private files.
        for path in selected {
            check(cancelled)?;
            if !path.is_absolute()
                || !lw_core::models::supports_audio(text(path)?)
                || !path.is_file()
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Choose supported audio files.",
                ));
            }
        }
        std::fs::create_dir_all(&self.paths.documents)?;
        let directory = self.paths.documents.canonicalize()?.join("Added Files");
        match directory.symlink_metadata() {
            Ok(info) if !info.is_dir() || info.file_type().is_symlink() => {
                return Err(io::Error::other("Added Files must be a real directory"));
            }
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        let mut copied = Vec::with_capacity(selected.len());
        for (index, path) in selected.iter().enumerate() {
            check(cancelled)?;
            progress("Copying", index, selected.len());
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
        let now = now.max(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_secs_f64(),
        );
        let source = self.register_folder(user, &directory, now, cancelled)?;
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
        let mut songs = Vec::with_capacity(copied.len());
        let mut keys = HashSet::with_capacity(copied.len());
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
        if songs.is_empty() {
            return Err(io::Error::other(
                "The selected audio files could not be imported.",
            ));
        }
        Ok(AudioSelection {
            source,
            songs: songs.into(),
            context: Arc::new(PlaybackContext::Selection("Selected files".into())),
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
    }
}
