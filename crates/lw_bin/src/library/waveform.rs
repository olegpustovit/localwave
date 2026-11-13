use super::{LibraryWorker, check, db, text};
use lw_apple_sys::{audio, bookmarks::Access, waveform};
use lw_core::{FileState, Song};
use std::io;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

fn readable(info: audio::FileInfo) -> bool {
    info.availability == audio::Availability::OnDevice
        || (info.availability == audio::Availability::Unknown && !info.provider)
}

impl LibraryWorker {
    /// All viewers use this serialized worker/cache; never decode from a draw callback.
    pub fn waveform_peaks(
        &mut self,
        song_id: i64,
        now: f64,
        cancelled: &AtomicBool,
    ) -> io::Result<Option<lw_core::waveform::Peaks>> {
        check(cancelled)?;
        if !now.is_finite() {
            return Err(io::Error::other("Waveform request date must be finite"));
        }
        let Some(song) = self.repository.get_song(song_id).map_err(db)? else {
            return Ok(None);
        };
        if self.waveforms.is_none() {
            let directory = self.paths.caches.join("LocalWave");
            std::fs::create_dir_all(&directory)?;
            self.waveforms = Some(
                lw_core::waveform::Repository::open(text(&directory.join("waveforms.sqlite"))?)
                    .map_err(db)?,
            );
        }
        let cached = self
            .waveforms
            .as_ref()
            .expect("Waveform database is open")
            .get(song.song_key)
            .map_err(db)?;
        let mut grant = None;
        let file = match self.readable_song_file(&song, now, &mut grant) {
            Ok(file) => file,
            Err(_) if cached.is_some() => {
                check(cancelled)?;
                return Ok(cached.map(|cached| cached.peaks));
            }
            Err(error) => return Err(error),
        };
        check(cancelled)?;
        let Some((file, known_local)) = file else {
            return Ok(cached.map(|cached| cached.peaks));
        };
        let version = waveform::version(&file, &self.paths.documents.join("Music"), known_local)?;
        let signature = lw_core::import::hex_digest(&version.digest);
        check(cancelled)?;
        if let Some(cached) = cached
            && cached.signature == signature
        {
            return Ok(Some(cached.peaks));
        }
        let Some(peaks) = waveform::decode(&version, cancelled)? else {
            return Ok(None);
        };
        check(cancelled)?;
        self.waveforms
            .as_ref()
            .expect("Waveform database is open")
            .put(song.song_key, &signature, &peaks)
            .map_err(db)?;
        Ok(Some(peaks))
    }
    pub(super) fn readable_song_file(
        &mut self,
        song: &Song,
        now: f64,
        grant: &mut Option<Access>,
    ) -> io::Result<Option<(PathBuf, bool)>> {
        let music = super::cache::music_root(&self.paths.documents).ok();
        if song.file_state == FileState::Copied
            && let Some(stored) = &song.local_file_path
        {
            let file = if std::path::Path::new(stored).is_absolute() {
                PathBuf::from(stored)
            } else {
                lw_core::file_access::source_file(&self.paths.documents, stored)?
            };
            let known = file
                .parent()
                .and_then(|parent| parent.canonicalize().ok())
                .zip(music)
                .is_some_and(|(parent, music)| parent == music);
            if file.is_file() && readable(audio::probe(&file, known)?) {
                return Ok(Some((file, known)));
            }
        }
        let file = if let (Some(source), Some(relative)) =
            (song.source_id, song.relative_path.as_deref())
        {
            self.source_file(source, relative, now)?
        } else if let Some(bookmark) = &song.bookmark {
            *grant = Some(Access::resolve(bookmark)?);
            grant
                .as_ref()
                .expect("Bookmark grant is live")
                .path()
                .to_owned()
        } else {
            return Ok(None);
        };
        if !file.is_file() {
            return Ok(None);
        }
        Ok(readable(audio::probe(&file, false)?).then_some((file, false)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    use lw_apple_sys::sqlite::{Connection, Value};
    use std::path::Path;
    use std::sync::atomic::Ordering;

    fn signal(path: &Path, silent: bool) {
        let mut bytes = Vec::from(*b"RIFF\xA4\x3E\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00\x40\x1F\x00\x00\x80\x3E\x00\x00\x02\x00\x10\x00data\x80\x3E\x00\x00");
        bytes.extend((0..8000).flat_map(|frame| {
            let value: i16 = if silent || frame < 4000 { 0 } else { 16384 };
            value.to_le_bytes()
        }));
        std::fs::write(path, bytes).unwrap();
    }
    fn paths(base: &Path) -> AppPaths {
        AppPaths {
            documents: base.join("Documents"),
            caches: base.join("Caches"),
            temporary: base.join("tmp"),
            test_database: None,
        }
    }
    #[test]
    fn worker_waveforms_invalidate_corruption_and_audio_changes_but_survive_lru_touches_eviction_and_relaunch()
     {
        let base = std::env::temp_dir().join(format!(
            "lw-waveform-worker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(base.join("Documents/Music")).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let file = base.join("signal.wav");
        signal(&file, false);
        let suite = format!(
            "com.olegpustovit.localwave.waveform-test-{}",
            base.file_name().unwrap().to_str().unwrap()
        );
        let database = base.join("library.sqlite");
        let mut worker = LibraryWorker::open(
            &database,
            Preferences::open(Some(&suite)).unwrap(),
            paths(&base),
        )
        .unwrap();
        let song = worker
            .repository
            .upsert_song(
                &Song {
                    song_key: 42,
                    local_file_path: Some(text(&file).unwrap().into()),
                    file_state: FileState::Copied,
                    ..Song::default()
                },
                0.0,
            )
            .unwrap();
        let id = song.id.unwrap();
        let cancel = AtomicBool::new(false);
        let version = waveform::version(&file, &base.join("Documents/Music"), false).unwrap();
        mem_sys::local_reset();
        let decoded = waveform::decode(&version, &cancel).unwrap().unwrap();
        assert_eq!(mem_sys::local_alloc_count(), 0);
        assert!(decoded[256..].iter().all(|peak| *peak == 1.0));
        let peaks = worker.waveform_peaks(id, 1.0, &cancel).unwrap().unwrap();
        assert!(peaks[..256].iter().all(|peak| *peak == 0.0));
        assert!(peaks[256..].iter().all(|peak| *peak == 1.0));
        let waveform_path = base.join("Caches/LocalWave/waveforms.sqlite");
        let store = lw_core::waveform::Repository::open(text(&waveform_path).unwrap()).unwrap();
        let sentinel = [0.25; 512];
        let original = store.get(42).unwrap().unwrap().signature;
        store.put(42, &original, &sentinel).unwrap();
        for _ in 0..8 {
            assert_eq!(
                worker.waveform_peaks(id, 1.0, &cancel).unwrap().unwrap(),
                sentinel
            );
        }
        signal(&file, true);
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(2);
        std::fs::File::open(&file)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert_eq!(
            worker.waveform_peaks(id, 2.0, &cancel).unwrap().unwrap(),
            [0.0; 512]
        );
        assert_ne!(store.get(42).unwrap().unwrap().signature, original);
        let connection = Connection::open(text(&waveform_path).unwrap()).unwrap();
        connection
            .execute(
                "UPDATE waveforms SET peaks=?",
                &[Value::Blob(b"invalid JSON")],
            )
            .unwrap();
        assert_eq!(
            worker.waveform_peaks(id, 2.0, &cancel).unwrap().unwrap(),
            [0.0; 512]
        );
        let digest = audio::read(&file, false, &cancel).unwrap().sha256;
        let relative = format!("Music/{}.wav", lw_core::import::hex_digest(&digest));
        let managed = worker.paths.documents.join(&relative);
        std::fs::copy(&file, &managed).unwrap();
        assert!(
            worker
                .repository
                .update_cached_file(
                    id,
                    Some(&relative),
                    FileState::Copied,
                    FileState::Copied,
                    song.local_file_path.as_deref()
                )
                .unwrap()
        );
        assert_eq!(
            worker.waveform_peaks(id, 3.0, &cancel).unwrap().unwrap(),
            [0.0; 512]
        );
        let signature = store.get(42).unwrap().unwrap().signature;
        store.put(42, &signature, &sentinel).unwrap();
        std::fs::File::open(&managed)
            .unwrap()
            .set_modified(later + std::time::Duration::from_secs(5))
            .unwrap();
        assert_eq!(
            worker.waveform_peaks(id, 3.0, &cancel).unwrap().unwrap(),
            sentinel
        );
        assert_eq!(store.get(42).unwrap().unwrap().signature, signature);
        std::fs::remove_file(&managed).unwrap();
        assert!(
            worker
                .repository
                .update_cached_file(
                    id,
                    None,
                    FileState::BookmarkOnly,
                    FileState::Copied,
                    Some(&relative)
                )
                .unwrap()
        );
        assert_eq!(
            worker.waveform_peaks(id, 3.0, &cancel).unwrap().unwrap(),
            sentinel
        );
        drop(worker);
        let mut worker = LibraryWorker::open(
            &database,
            Preferences::open(Some(&suite)).unwrap(),
            paths(&base),
        )
        .unwrap();
        assert_eq!(
            worker.waveform_peaks(id, 4.0, &cancel).unwrap().unwrap(),
            sentinel
        );
        cancel.store(true, Ordering::Relaxed);
        assert_eq!(
            worker.waveform_peaks(id, 4.0, &cancel).unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        cancel.store(false, Ordering::Relaxed);
        assert!(worker.waveform_peaks(id, f64::NAN, &cancel).is_err());
        assert!(worker.waveform_peaks(-1, 4.0, &cancel).unwrap().is_none());
        let mut song = worker.repository.get_song(id).unwrap().unwrap();
        song.bookmark = Some(lw_apple_sys::bookmarks::create(&file).unwrap());
        worker.repository.upsert_song(&song, 4.0).unwrap();
        assert_eq!(
            worker.waveform_peaks(id, 4.0, &cancel).unwrap().unwrap(),
            [0.0; 512]
        );
        let user = worker.repository.get_or_create_user(1).unwrap();
        let source = worker
            .repository
            .create_source(&lw_core::Source {
                dir_path: text(&base).unwrap().into(),
                path_id: super::super::hash(&base).unwrap(),
                user_id: user.id,
                bookmark: Some(lw_apple_sys::bookmarks::create(&base).unwrap()),
                ..lw_core::Source::default()
            })
            .unwrap()
            .id
            .unwrap();
        song.source_id = Some(source);
        song.relative_path = Some("signal.wav".into());
        song.bookmark = None;
        worker.repository.upsert_song(&song, 5.0).unwrap();
        assert_eq!(
            worker.waveform_peaks(id, 5.0, &cancel).unwrap().unwrap(),
            [0.0; 512]
        );
        let provider = base.join("Library/CloudStorage/Provider");
        std::fs::create_dir_all(&provider).unwrap();
        std::fs::copy(&file, provider.join("signal.wav")).unwrap();
        let source = worker
            .repository
            .create_source(&lw_core::Source {
                dir_path: text(&provider).unwrap().into(),
                path_id: super::super::hash(&provider).unwrap(),
                user_id: user.id,
                bookmark: Some(lw_apple_sys::bookmarks::create(&provider).unwrap()),
                ..lw_core::Source::default()
            })
            .unwrap()
            .id
            .unwrap();
        song.source_id = Some(source);
        worker.repository.upsert_song(&song, 6.0).unwrap();
        assert_eq!(
            worker.waveform_peaks(id, 6.0, &cancel).unwrap().unwrap(),
            [0.0; 512]
        );
        connection.execute("DELETE FROM waveforms", &[]).unwrap();
        assert!(worker.waveform_peaks(id, 6.0, &cancel).unwrap().is_none());
    }
}
