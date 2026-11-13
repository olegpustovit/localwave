use super::{Root, check, hash, text};
use lw_apple_sys::{artwork, audio, bookmarks::Access, preferences::ImportPreferences};
use lw_core::import::{Audio, CachedFile, FileInfo, ImportIO};
use lw_core::{FileAvailability, Song, Source, SourcePath};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

pub(super) struct Files<'a> {
    pub roots: &'a HashMap<i64, Root>,
    pub documents: &'a Path,
    pub preferences: ImportPreferences,
}
impl Files<'_> {
    fn access(&self, source_id: i64) -> io::Result<&Access> {
        self.roots
            .get(&source_id)
            .map(|root| &root.access)
            .ok_or_else(|| io::Error::other("Source folder access is missing"))
    }
    fn cached(&self, song: &Song) -> Option<PathBuf> {
        let stored = Path::new(song.local_file_path.as_ref()?);
        let file = if stored.is_absolute() {
            stored.canonicalize().ok()?
        } else {
            lw_core::file_access::source_file(self.documents, stored.to_str()?).ok()?
        };
        (audio::probe(&file, true).ok()?.availability == audio::Availability::OnDevice)
            .then_some(file)
    }
}
impl ImportIO for Files<'_> {
    fn inspect(
        &mut self,
        _: &Source,
        path: &SourcePath,
        existing: Option<&Song>,
    ) -> io::Result<FileInfo> {
        let access = self.access(path.source_id)?;
        let file = lw_core::file_access::source_file(access.path(), &path.relative_path)?;
        let info = audio::probe(&file, false)?;
        let cached = existing
            .and_then(|song| self.cached(song).map(|cache| (song, cache)))
            .map(|(song, cache)| {
                Ok::<_, io::Error>(CachedFile {
                    source_matches: match song.id {
                        Some(id) => {
                            self.preferences
                                .cache_matches(id, &cache, info.modified, info.size)?
                        }
                        None => None,
                    },
                    path: text(&cache)?.to_owned(),
                })
            })
            .transpose()?;
        let bookmark_valid = existing
            .filter(|song| song.source_id.is_none())
            .and_then(|song| song.bookmark.as_deref())
            .is_some_and(|bookmark| {
                Access::resolve(bookmark).is_ok_and(|grant| grant.renewed_bookmark().is_none())
            });
        Ok(FileInfo {
            path_hash: hash(&access.path().join(&path.relative_path))?,
            availability: match info.availability {
                audio::Availability::Unknown => FileAvailability::Unknown,
                audio::Availability::OnDevice => FileAvailability::OnDevice,
                audio::Availability::InCloud => FileAvailability::InCloud,
            },
            provider: info.provider,
            modified: info.modified,
            size: info.size,
            cached,
            bookmark_valid,
        })
    }

    fn read(
        &mut self,
        _: &Source,
        path: &SourcePath,
        cached: Option<&str>,
        cancelled: &AtomicBool,
    ) -> io::Result<Audio> {
        let file = match cached {
            Some(path) => PathBuf::from(path),
            None => lw_core::file_access::source_file(
                self.access(path.source_id)?.path(),
                &path.relative_path,
            )?,
        };
        let mut flac = None;
        let mut size = None;
        let metadata = audio::read_with(&file, cached.is_some(), cancelled, &mut |readable| {
            size =
                Some(i64::try_from(std::fs::metadata(readable)?.len()).map_err(io::Error::other)?);
            if Path::new(&path.name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("flac"))
            {
                flac = lw_core::flac::Metadata::read(&mut File::open(readable)?)?;
            }
            Ok(())
        })?;
        if let Some(warning) = metadata.warning {
            eprintln!("Audio metadata: {warning}");
        }
        check(cancelled)?;
        let mut audio = Audio {
            artist: metadata.artist,
            title: metadata.title,
            album: metadata.album,
            album_artist: metadata.album_artist,
            year: metadata.year,
            track: metadata.track,
            disc: metadata.disc,
            duration: metadata.duration,
            sample_rate: metadata.sample_rate,
            bit_depth: metadata.bit_depth,
            file_size: size,
            sha256: metadata.sha256,
            ..Audio::default()
        };
        let cover = if let Some(flac) = flac {
            audio.apply_flac_tags(&flac.comments);
            flac.artwork.or(metadata.artwork)
        } else {
            metadata.artwork
        };
        if let Some(cover) = cover {
            audio.cover_art_path = artwork::store(self.documents, &cover)?;
        }
        Ok(audio)
    }
    fn pinned(&self, song_id: i64) -> bool {
        self.preferences.pinned(song_id)
    }
}
