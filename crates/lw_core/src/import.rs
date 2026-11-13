//! Import policy and progress. File/provider/framework operations belong to the worker's IO boundary.
use crate::{
    FileAvailability, FileState, LibraryRepository, Song, Source, SourcePath, hash_string, song_key,
};
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct CachedFile {
    pub path: String,
    pub source_matches: Option<bool>,
}

#[derive(Debug)]
pub struct FileInfo {
    pub path_hash: i64,
    pub availability: FileAvailability,
    pub provider: bool,
    pub modified: Option<f64>,
    pub size: Option<i64>,
    pub cached: Option<CachedFile>,
    pub bookmark_valid: bool,
}

#[derive(Debug, Default)]
pub struct Audio {
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<i64>,
    pub track: Option<i64>,
    pub disc: Option<i64>,
    pub duration: Option<f64>,
    pub sample_rate: Option<f64>,
    pub bit_depth: Option<i64>,
    pub file_size: Option<i64>,
    pub cover_art_path: Option<String>,
    pub sha256: [u8; 32],
}

impl Audio {
    pub fn apply_flac_tags(&mut self, tags: &BTreeMap<String, String>) {
        let number = |key: &str| {
            tags.get(key)
                .and_then(|value| value.split('/').next()?.parse().ok())
        };
        self.artist = tags.get("ARTIST").cloned();
        self.title = tags.get("TITLE").cloned();
        self.album = tags.get("ALBUM").cloned();
        self.album_artist = tags
            .get("ALBUMARTIST")
            .or_else(|| tags.get("ALBUM ARTIST"))
            .cloned();
        self.year = tags
            .get("DATE")
            .and_then(|value| value.chars().take(4).collect::<String>().parse().ok())
            .or_else(|| number("YEAR"));
        self.track = number("TRACKNUMBER");
        self.disc = number("DISCNUMBER");
    }
}

pub trait ImportIO {
    /// Resource attributes and verified local-cache proof only; must not download or open audio.
    fn inspect(
        &mut self,
        source: &Source,
        path: &SourcePath,
        existing: Option<&Song>,
    ) -> io::Result<FileInfo>;
    /// Hash, measure and read tags/artwork while a coordinated read and source grant are live.
    fn read(
        &mut self,
        source: &Source,
        path: &SourcePath,
        cached: Option<&str>,
        cancelled: &AtomicBool,
    ) -> io::Result<Audio>;
    fn pinned(&self, song_id: i64) -> bool;
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub imported: usize,
    pub skipped: usize,
    pub deferred: usize,
}

fn db_error(error: lw_apple_sys::sqlite::Error) -> io::Error {
    io::Error::other(error)
}

impl LibraryRepository {
    pub fn import_paths(
        &mut self,
        selected: &[SourcePath],
        files: &mut dyn ImportIO,
        cancelled: &AtomicBool,
        now: f64,
        progress: &mut dyn FnMut(&SourcePath, usize, usize),
    ) -> io::Result<Summary> {
        if !now.is_finite() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Import date must be finite",
            ));
        }
        let check_cancelled = || {
            if cancelled.load(Ordering::Relaxed) {
                Err(io::Error::from(io::ErrorKind::Interrupted))
            } else {
                Ok(())
            }
        };
        check_cancelled()?;
        let paths = self.import_files(selected).map_err(db_error)?;
        let mut sources = HashMap::new();
        for path in &paths {
            if let std::collections::hash_map::Entry::Vacant(entry) = sources.entry(path.source_id)
            {
                entry.insert(
                    self.source(path.source_id)
                        .map_err(db_error)?
                        .ok_or_else(|| io::Error::other("This music source was removed"))?,
                );
            }
        }
        let mut result = Summary::default();
        for (completed, path) in paths.iter().enumerate() {
            check_cancelled()?;
            progress(path, completed, paths.len());
            check_cancelled()?;
            let source = &sources[&path.source_id];
            let mut existing = self
                .song_at_source_path(path.source_id, &path.relative_path)
                .map_err(db_error)?;
            let mut info = files.inspect(source, path, existing.as_ref())?;
            if existing.is_none() {
                existing = self.song_with_path_hash(info.path_hash).map_err(db_error)?;
                if existing.is_some() {
                    info = files.inspect(source, path, existing.as_ref())?;
                }
            }
            check_cancelled()?;
            if let Some(song) = &existing
                && song.source_availability != Some(info.availability)
            {
                self.update_song_availability(
                    song.id
                        .ok_or_else(|| io::Error::other("Unsaved imported song"))?,
                    info.availability,
                )
                .map_err(db_error)?;
            }
            let deferred = info.availability == FileAvailability::InCloud
                || (info.availability == FileAvailability::Unknown && info.provider);
            let mut cached = None;
            if deferred && let (Some(song), Some(cache)) = (&existing, &info.cached) {
                let changed = cache.source_matches == Some(false)
                    || (cache.source_matches.is_none()
                        && song.duration.is_some()
                        && (info.modified.is_some_and(|modified| {
                            modified > song.updated_at.unwrap_or(song.created_at)
                        }) || info
                            .size
                            .zip(song.file_size)
                            .is_some_and(|(current, previous)| current != previous)));
                if changed {
                    let id = song
                        .id
                        .ok_or_else(|| io::Error::other("Unsaved imported song"))?;
                    if self
                        .update_cached_file(
                            id,
                            None,
                            FileState::BookmarkOnly,
                            song.file_state,
                            song.local_file_path.as_deref(),
                        )
                        .map_err(db_error)?
                        && files.pinned(id)
                    {
                        self.mark_song_for_copy(id).map_err(db_error)?;
                    }
                } else {
                    cached = Some(cache.path.as_str());
                }
            }
            if deferred && cached.is_none() {
                if let Some(song) = &existing {
                    if song.source_id.is_none() {
                        self.update_song_source_location(
                            song.id
                                .ok_or_else(|| io::Error::other("Unsaved imported song"))?,
                            path.source_id,
                            &path.relative_path,
                        )
                        .map_err(db_error)?;
                    }
                } else {
                    self.upsert_song(
                        &Song {
                            song_key: hash_string(&format!(
                                "source:{}/{}",
                                path.source_id, path.relative_path
                            )),
                            artist: "Unknown Artist".into(),
                            title: fallback_title(&path.name).into(),
                            album: "Unknown Album".into(),
                            album_artist: "Unknown Artist".into(),
                            path_hash: info.path_hash,
                            source_id: Some(path.source_id),
                            relative_path: Some(path.relative_path.clone()),
                            created_at: now,
                            file_size: info.size,
                            file_extension: extension(&path.name),
                            source_availability: Some(info.availability),
                            ..Song::default()
                        },
                        now,
                    )
                    .map_err(db_error)?;
                }
                result.deferred += 1;
            } else {
                let unchanged = path.file_hash_sha256.as_deref().is_some_and(|hash| {
                    existing.as_ref().is_some_and(|song| {
                        let same_source = song.source_id == Some(path.source_id)
                            && song.relative_path.as_deref() == Some(path.relative_path.as_str());
                        let current = if let Some(cache) = cached {
                            digest_matches(cache, hash)
                        } else {
                            info.modified.is_some_and(|modified| {
                                modified <= song.updated_at.unwrap_or(song.created_at)
                            }) && info
                                .size
                                .zip(song.file_size)
                                .is_none_or(|(current, previous)| current == previous)
                        };
                        (same_source || info.bookmark_valid)
                            && song.duration.is_some()
                            && song.file_size.is_some()
                            && song.file_extension.is_some()
                            && current
                    })
                });
                if unchanged {
                    if let Some(song) = &existing
                        && song.source_id.is_none()
                    {
                        self.update_song_source_location(
                            song.id.unwrap(),
                            path.source_id,
                            &path.relative_path,
                        )
                        .map_err(db_error)?;
                    }
                    result.skipped += 1;
                } else {
                    let audio = files.read(source, path, cached, cancelled)?;
                    check_cancelled()?;
                    let latest = self
                        .song_at_source_path(path.source_id, &path.relative_path)
                        .map_err(db_error)?
                        .or_else(|| existing.clone());
                    let unchanged_cache = path.file_hash_sha256.as_deref() == Some(&audio.sha256);
                    let cached_path = latest.as_ref().and_then(|song| {
                        let verified = song
                            .local_file_path
                            .as_deref()
                            .is_some_and(|path| digest_matches(path, &audio.sha256));
                        let replaced = song.local_file_path.as_deref()
                            != existing
                                .as_ref()
                                .and_then(|old| old.local_file_path.as_deref());
                        (unchanged_cache || verified || replaced)
                            .then(|| song.local_file_path.clone())
                            .flatten()
                    });
                    let artist = audio.artist.unwrap_or_else(|| "Unknown Artist".into());
                    let title = audio
                        .title
                        .unwrap_or_else(|| fallback_title(&path.name).into());
                    let album = audio.album.unwrap_or_else(|| "Unknown Album".into());
                    let key = song_key(&artist, &title, &album);
                    let song = Song {
                        song_key: key,
                        album_artist: audio.album_artist.unwrap_or_else(|| artist.clone()),
                        artist,
                        title,
                        album,
                        release_year: audio.year,
                        track_number: audio.track,
                        disc_number: audio.disc,
                        cover_art_path: audio.cover_art_path,
                        path_hash: info.path_hash,
                        created_at: now,
                        file_state: if cached_path.is_some() {
                            latest
                                .as_ref()
                                .map_or(FileState::BookmarkOnly, |song| song.file_state)
                        } else {
                            FileState::BookmarkOnly
                        },
                        local_file_path: cached_path,
                        source_id: Some(path.source_id),
                        relative_path: Some(path.relative_path.clone()),
                        duration: audio.duration,
                        file_size: info.size.or(audio.file_size),
                        file_extension: extension(&path.name),
                        sample_rate: audio.sample_rate,
                        bit_depth: audio.bit_depth,
                        source_availability: Some(info.availability),
                        ..Song::default()
                    };
                    let saved = self
                        .upsert_imported_song(path, &song, &audio.sha256, now)
                        .map_err(db_error)?;
                    if let Some(id) = saved.id
                        && files.pinned(id)
                    {
                        self.mark_song_for_copy(id).map_err(db_error)?;
                    }
                    result.imported += 1;
                }
            }
            progress(path, completed + 1, paths.len());
        }
        Ok(result)
    }
}

fn extension(name: &str) -> Option<String> {
    Path::new(name)
        .extension()?
        .to_str()
        .map(str::to_ascii_lowercase)
}
fn digest_stem(path: &str) -> Option<&str> {
    Path::new(path).file_stem()?.to_str()
}
fn digest_matches(path: &str, digest: &[u8]) -> bool {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    digest_stem(path).is_some_and(|stem| {
        stem.len() == digest.len() * 2
            && stem
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .zip(digest)
                .all(|(pair, byte)| *pair == [HEX[(byte >> 4) as usize], HEX[(byte & 15) as usize]])
    })
}
pub fn hex_digest(digest: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(text, "{byte:02x}").expect("Writing into String");
    }
    text
}

/// Borrow the filename stem, removing Swift's optional 1–3 decimal-digit track prefix.
pub fn fallback_title(name: &str) -> &str {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(name);
    let start = stem.trim_start();
    let digits = start
        .chars()
        .take_while(|c| crate::decimal::is_decimal(*c))
        .count();
    if !(1..=3).contains(&digits) {
        return stem;
    }
    let after = start
        .trim_start_matches(crate::decimal::is_decimal)
        .trim_start();
    if let Some(separator) = after.chars().next()
        && matches!(separator, '-' | '–' | '—' | '.' | '_' | ')')
    {
        let title = after[separator.len_utf8()..].trim_start();
        if !title.is_empty() {
            return title;
        }
    }
    stem
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
