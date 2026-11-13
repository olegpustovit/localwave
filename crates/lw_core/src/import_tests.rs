use super::*;

struct Files {
    availability: FileAvailability,
    provider: bool,
    modified: f64,
    size: i64,
    cached: Option<(String, Option<bool>)>,
    digest: u8,
    reads: usize,
    pinned: bool,
    cancel_read: bool,
}
impl Default for Files {
    fn default() -> Self {
        Self {
            availability: FileAvailability::OnDevice,
            provider: false,
            modified: 1.0,
            size: 1000,
            cached: None,
            digest: 1,
            reads: 0,
            pinned: false,
            cancel_read: false,
        }
    }
}
impl ImportIO for Files {
    fn inspect(&mut self, _: &Source, _: &SourcePath, _: Option<&Song>) -> io::Result<FileInfo> {
        Ok(FileInfo {
            path_hash: 10,
            availability: self.availability,
            provider: self.provider,
            modified: Some(self.modified),
            size: Some(self.size),
            cached: self.cached.as_ref().map(|(path, current)| CachedFile {
                path: path.clone(),
                source_matches: *current,
            }),
            bookmark_valid: false,
        })
    }
    fn read(
        &mut self,
        _: &Source,
        _: &SourcePath,
        cached: Option<&str>,
        cancelled: &AtomicBool,
    ) -> io::Result<Audio> {
        self.reads += 1;
        if self.provider || self.availability == FileAvailability::InCloud {
            assert!(cached.is_some(), "Opened unconfirmed provider audio");
        }
        if self.cancel_read {
            cancelled.store(true, Ordering::Relaxed);
        }
        Ok(Audio {
            artist: Some("Artist".into()),
            title: Some("Imported".into()),
            album: Some("Album".into()),
            album_artist: Some("Ensemble".into()),
            year: Some(2026),
            track: Some(3),
            disc: Some(2),
            duration: Some(10.0),
            sample_rate: Some(48000.0),
            bit_depth: Some(24),
            sha256: [self.digest; 32],
            ..Audio::default()
        })
    }
    fn pinned(&self, _: i64) -> bool {
        self.pinned
    }
}

fn fixture() -> (LibraryRepository, SourcePath) {
    let mut repo = LibraryRepository::open(":memory:").unwrap();
    let source = repo
        .create_source(&Source {
            user_id: 1,
            dir_path: "/source".into(),
            ..Source::default()
        })
        .unwrap();
    let folder = SourcePath {
        source_id: source.id.unwrap(),
        path_id: 1,
        name: "Album".into(),
        relative_path: "Album".into(),
        is_directory: true,
        ..SourcePath::default()
    };
    let path = SourcePath {
        source_id: source.id.unwrap(),
        path_id: 2,
        parent_path_id: Some(1),
        name: "01 – Track.FLAC".into(),
        relative_path: "Album/01 – Track.FLAC".into(),
        ..SourcePath::default()
    };
    let note = SourcePath {
        source_id: source.id.unwrap(),
        path_id: 3,
        parent_path_id: Some(1),
        name: "notes.txt".into(),
        relative_path: "Album/notes.txt".into(),
        ..SourcePath::default()
    };
    repo.upsert_source_paths(&[folder.clone(), path, note])
        .unwrap();
    (repo, folder)
}

#[test]
fn recursive_import_progress_unchanged_bytes_and_changed_cache_preserve_identity() {
    let (mut repo, folder) = fixture();
    let mut files = Files::default();
    let cancelled = AtomicBool::new(false);
    let mut progress = Vec::new();
    let result = repo
        .import_paths(
            std::slice::from_ref(&folder),
            &mut files,
            &cancelled,
            100.0,
            &mut |_, done, total| progress.push((done, total)),
        )
        .unwrap();
    assert_eq!(
        result,
        Summary {
            imported: 1,
            ..Summary::default()
        }
    );
    assert_eq!(progress, [(0, 1), (1, 1)]);
    let song = repo.search_songs("Imported", 10, 0).unwrap().pop().unwrap();
    let id = song.id.unwrap();
    assert_eq!(song.album_artist, "Ensemble");
    assert_eq!(song.file_size, Some(1000));
    assert_eq!(song.file_extension.as_deref(), Some("flac"));
    assert!(song.bookmark.is_none());
    repo.set_favorite(id, true).unwrap();
    let list = repo.create_playlist("List", 100.0).unwrap();
    repo.add_playlist_songs(list.id, &[id]).unwrap();
    repo.update_cached_file(
        id,
        Some("Music/cached.flac"),
        FileState::Copied,
        FileState::BookmarkOnly,
        None,
    )
    .unwrap();
    assert_eq!(
        repo.import_paths(
            std::slice::from_ref(&folder),
            &mut files,
            &cancelled,
            101.0,
            &mut |_, _, _| {}
        )
        .unwrap()
        .skipped,
        1
    );
    assert_eq!(files.reads, 1);
    files.modified = 200.0;
    repo.import_paths(
        std::slice::from_ref(&folder),
        &mut files,
        &cancelled,
        201.0,
        &mut |_, _, _| {},
    )
    .unwrap();
    assert_eq!(
        repo.get_song(id)
            .unwrap()
            .unwrap()
            .local_file_path
            .as_deref(),
        Some("Music/cached.flac")
    );
    files.modified = 300.0;
    files.digest = 2;
    repo.import_paths(&[folder], &mut files, &cancelled, 301.0, &mut |_, _, _| {})
        .unwrap();
    let changed = repo.get_song(id).unwrap().unwrap();
    assert_eq!(changed.file_state, FileState::BookmarkOnly);
    assert!(changed.local_file_path.is_none());
    assert_eq!(changed.is_favorite, 1);
    assert_eq!(repo.playlist_songs(list.id).unwrap()[0].id, Some(id));
}

#[test]
fn cloud_placeholder_enriches_in_place_and_unknown_provider_reads_only_verified_cache() {
    let (mut repo, folder) = fixture();
    let cancelled = AtomicBool::new(false);
    let mut files = Files {
        availability: FileAvailability::InCloud,
        ..Files::default()
    };
    assert_eq!(
        repo.import_paths(
            std::slice::from_ref(&folder),
            &mut files,
            &cancelled,
            100.0,
            &mut |_, _, _| {}
        )
        .unwrap()
        .deferred,
        1
    );
    assert_eq!(files.reads, 0);
    let placeholder = repo.search_songs("", 10, 0).unwrap().pop().unwrap();
    let id = placeholder.id.unwrap();
    assert_eq!(placeholder.title, "Track");
    assert!(placeholder.duration.is_none());
    repo.set_favorite(id, true).unwrap();
    files.availability = FileAvailability::Unknown;
    files.provider = true;
    files.cached = Some((format!("Music/{}.flac", hex_digest(&[1; 32])), Some(true)));
    repo.update_cached_file(
        id,
        Some(&files.cached.as_ref().unwrap().0),
        FileState::Copied,
        FileState::BookmarkOnly,
        None,
    )
    .unwrap();
    assert_eq!(
        repo.import_paths(
            std::slice::from_ref(&folder),
            &mut files,
            &cancelled,
            101.0,
            &mut |_, _, _| {}
        )
        .unwrap()
        .imported,
        1
    );
    let enriched = repo.get_song(id).unwrap().unwrap();
    assert_eq!(enriched.title, "Imported");
    assert_eq!(enriched.is_favorite, 1);
    assert_eq!(
        enriched.source_availability,
        Some(FileAvailability::Unknown)
    );
    assert_eq!(enriched.file_state, FileState::Copied);
    files.cached.as_mut().unwrap().1 = Some(false);
    files.pinned = true;
    assert_eq!(
        repo.import_paths(&[folder], &mut files, &cancelled, 102.0, &mut |_, _, _| {})
            .unwrap()
            .deferred,
        1
    );
    assert_eq!(files.reads, 1);
    let invalidated = repo.get_song(id).unwrap().unwrap();
    assert_eq!(invalidated.file_state, FileState::CopyPending);
    assert!(invalidated.local_file_path.is_none());
    assert_eq!(invalidated.title, "Imported");
    assert_eq!(
        repo.source_paths(placeholder.source_id.unwrap(), None)
            .unwrap()
            .iter()
            .find(|path| path.is_audio())
            .unwrap()
            .file_hash_sha256,
        Some(vec![1; 32])
    );
}

#[test]
fn cancellation_after_read_leaves_song_digest_and_fts_uncommitted() {
    let (mut repo, folder) = fixture();
    let cancelled = AtomicBool::new(false);
    let mut files = Files {
        cancel_read: true,
        ..Files::default()
    };
    assert_eq!(
        repo.import_paths(
            std::slice::from_ref(&folder),
            &mut files,
            &cancelled,
            f64::NAN,
            &mut |_, _, _| {}
        )
        .unwrap_err()
        .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(files.reads, 0);
    assert_eq!(
        repo.import_paths(
            std::slice::from_ref(&folder),
            &mut files,
            &cancelled,
            100.0,
            &mut |_, _, _| {}
        )
        .unwrap_err()
        .kind(),
        io::ErrorKind::Interrupted
    );
    cancelled.store(false, Ordering::Relaxed);
    assert_eq!(
        repo.import_paths(&[folder], &mut files, &cancelled, 100.0, &mut |_, _, _| {
            cancelled.store(true, Ordering::Relaxed)
        })
        .unwrap_err()
        .kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(files.reads, 1);
    assert_eq!(repo.song_count("").unwrap(), 0);
    assert!(repo.search_songs("Imported", 10, 0).unwrap().is_empty());
    assert!(
        repo.source_paths(1, None)
            .unwrap()
            .iter()
            .all(|path| path.file_hash_sha256.is_none())
    );
}

#[test]
fn title_cleanup_and_flac_tag_numbers_match_swift_without_regex_allocations() {
    for (name, expected) in [
        ("01 – COUSINS.m4a", "COUSINS"),
        ("07.artist-name.flac", "artist-name"),
        ("1984.m4a", "1984"),
        ("CJKタイトル.wav", "CJKタイトル"),
        ("١٢ – عربي.flac", "عربي"),
        ("Ⅻ – Roman.flac", "Ⅻ – Roman"),
        ("² – Superscript.flac", "² – Superscript"),
        ("01 - .flac", "01 - "),
        ("1234-Title.mp3", "1234-Title"),
    ] {
        assert_eq!(fallback_title(name), expected);
    }
    let mut audio = Audio::default();
    audio.apply_flac_tags(&BTreeMap::from([
        ("ARTIST".into(), "Artist".into()),
        ("TITLE".into(), "Title".into()),
        ("ALBUM ARTIST".into(), "Ensemble".into()),
        ("DATE".into(), "2026-11-12".into()),
        ("TRACKNUMBER".into(), "3/8".into()),
        ("DISCNUMBER".into(), "2/2".into()),
    ]));
    assert_eq!(audio.album_artist.as_deref(), Some("Ensemble"));
    assert_eq!(
        (audio.year, audio.track, audio.disc),
        (Some(2026), Some(3), Some(2))
    );
    mem_sys::local_reset();
    assert_eq!(fallback_title("01 – Track.flac"), "Track");
    assert!(digest_matches("Music/0101.flac", &[1, 1]));
    assert!(!digest_matches("Music/0102.flac", &[1, 1]));
    assert_eq!(mem_sys::local_alloc_count(), 0);
}

#[cfg(target_os = "macos")]
#[test]
fn native_recursive_flac_and_m4a_import_follow_renamed_bookmark_and_reject_mid_read_changes() {
    use lw_apple_sys::{audio, bookmarks, normalized_file_url};
    use std::fs::{self, File};
    use std::path::PathBuf;
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    struct NativeFiles {
        access: bookmarks::Access,
        reads: usize,
        mutate: bool,
    }
    impl ImportIO for NativeFiles {
        fn inspect(
            &mut self,
            _: &Source,
            path: &SourcePath,
            _: Option<&Song>,
        ) -> io::Result<FileInfo> {
            let file = crate::file_access::source_file(self.access.path(), &path.relative_path)?;
            let info = audio::probe(&file, false)?;
            Ok(FileInfo {
                path_hash: hash_string(&normalized_file_url(file.to_str().unwrap())?),
                availability: match info.availability {
                    audio::Availability::Unknown => FileAvailability::Unknown,
                    audio::Availability::OnDevice => FileAvailability::OnDevice,
                    audio::Availability::InCloud => FileAvailability::InCloud,
                },
                provider: info.provider,
                size: info.size,
                modified: info.modified,
                cached: None,
                bookmark_valid: false,
            })
        }
        fn read(
            &mut self,
            _: &Source,
            path: &SourcePath,
            cached: Option<&str>,
            cancelled: &AtomicBool,
        ) -> io::Result<Audio> {
            assert!(cached.is_none());
            self.reads += 1;
            let file = crate::file_access::source_file(self.access.path(), &path.relative_path)?;
            let mut flac = None;
            let mut size = None;
            let metadata = audio::read_with(&file, false, cancelled, &mut |readable| {
                size = Some(fs::metadata(readable)?.len() as i64);
                if readable
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("flac"))
                {
                    flac = crate::flac::Metadata::read(&mut File::open(readable)?)?;
                }
                if self.mutate {
                    use std::io::Write;
                    fs::OpenOptions::new()
                        .append(true)
                        .open(readable)?
                        .write_all(&[0])?;
                }
                Ok(())
            })?;
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
                sha256: metadata.sha256,
                file_size: size,
                cover_art_path: None,
            };
            if let Some(flac) = flac {
                audio.apply_flac_tags(&flac.comments);
            }
            Ok(audio)
        }
        fn pinned(&self, _: i64) -> bool {
            false
        }
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let root = std::env::temp_dir().join(format!(
        "localwave-import-{}-{}",
        std::process::id(),
        (now * 1e9) as u128
    ));
    let _cleanup = Cleanup(root.clone());
    let original = root.join("Original");
    fs::create_dir_all(original.join("Album")).unwrap();
    let fixtures =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Tests/LocalWaveDataTests/Fixtures");
    fs::copy(
        fixtures.join("tagged.flac"),
        original.join("Album/track.FLAC"),
    )
    .unwrap();
    fs::copy(
        fixtures.join("tagged-m4a.m4a"),
        original.join("Album/track.m4a"),
    )
    .unwrap();
    let bookmark = bookmarks::create(&original).unwrap();
    let mut repo = LibraryRepository::open(":memory:").unwrap();
    let source = repo
        .create_source(&Source {
            user_id: 1,
            dir_path: original.to_str().unwrap().into(),
            bookmark: Some(bookmark.clone()),
            created_at: now,
            ..Source::default()
        })
        .unwrap();
    let folder = SourcePath {
        source_id: source.id.unwrap(),
        path_id: 1,
        name: "Album".into(),
        relative_path: "Album".into(),
        is_directory: true,
        ..SourcePath::default()
    };
    let flac = SourcePath {
        source_id: source.id.unwrap(),
        path_id: 2,
        parent_path_id: Some(1),
        name: "track.FLAC".into(),
        relative_path: "Album/track.FLAC".into(),
        ..SourcePath::default()
    };
    let m4a = SourcePath {
        source_id: source.id.unwrap(),
        path_id: 3,
        parent_path_id: Some(1),
        name: "track.m4a".into(),
        relative_path: "Album/track.m4a".into(),
        ..SourcePath::default()
    };
    repo.upsert_source_paths(&[folder.clone(), flac.clone(), m4a])
        .unwrap();
    let renamed = root.join("Renamed");
    fs::rename(&original, &renamed).unwrap();
    let mut files = NativeFiles {
        access: bookmarks::Access::resolve(&bookmark).unwrap(),
        reads: 0,
        mutate: false,
    };
    let cancelled = AtomicBool::new(false);
    assert_eq!(
        repo.import_paths(
            std::slice::from_ref(&folder),
            &mut files,
            &cancelled,
            now + 1.0,
            &mut |_, _, _| {}
        )
        .unwrap()
        .imported,
        2
    );
    let songs = repo.search_songs("", 10, 0).unwrap();
    assert_eq!(songs.len(), 2);
    let flac_song = songs
        .iter()
        .find(|song| song.title == "FLAC Fixture")
        .unwrap();
    assert_eq!(flac_song.album_artist, "Test Ensemble");
    assert_eq!(
        (
            flac_song.release_year,
            flac_song.track_number,
            flac_song.disc_number
        ),
        (Some(2026), Some(3), Some(2))
    );
    assert!((flac_song.duration.unwrap() - 0.15).abs() < 0.02);
    let m4a_song = songs
        .iter()
        .find(|song| song.title == "M4A Fixture")
        .unwrap();
    assert_eq!(m4a_song.album_artist, "AAC Ensemble");
    assert_eq!(
        (
            m4a_song.release_year,
            m4a_song.track_number,
            m4a_song.disc_number
        ),
        (Some(2025), Some(3), Some(2))
    );
    assert_eq!(
        repo.import_paths(
            std::slice::from_ref(&folder),
            &mut files,
            &cancelled,
            now + 2.0,
            &mut |_, _, _| {}
        )
        .unwrap()
        .skipped,
        2
    );
    assert_eq!(files.reads, 2);
    let id = flac_song.id.unwrap();
    let saved_digest = repo
        .source_path(flac.source_id, &flac.relative_path)
        .unwrap()
        .unwrap()
        .file_hash_sha256
        .unwrap();
    assert_eq!(
        hex_digest(&saved_digest),
        "56eca9596a28522e4821c785176b095a64d4c6e88547672f68f6f8b959507528"
    );
    repo.update_source_path_hash(flac.source_id, flac.path_id, None)
        .unwrap();
    files.mutate = true;
    assert!(
        repo.import_paths(
            &[folder],
            &mut files,
            &cancelled,
            now + 3.0,
            &mut |_, _, _| {}
        )
        .unwrap_err()
        .to_string()
        .contains("changed during import")
    );
    assert_eq!(repo.get_song(id).unwrap().unwrap().title, "FLAC Fixture");
    assert!(
        repo.source_path(flac.source_id, &flac.relative_path)
            .unwrap()
            .unwrap()
            .file_hash_sha256
            .is_none()
    );
}
