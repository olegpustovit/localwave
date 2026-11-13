//! Invoked by the Swift compatibility check against a temporary fixture database.
use lw_apple_sys::{
    normalized_file_url,
    sqlite::{Connection, Error},
};
use lw_core::{
    FileAvailability, FileState, LibraryRepository, Song, SourcePath, hash_string, song_key,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Expected fixture database path")?;
    let db = Connection::open(&path)?;
    let mut vectors = 0;
    db.query("SELECT text,hash,songKey,path,pathId FROM rust_compat_keys", &[], &mut |row| {
        let text = row.text(0)?.ok_or_else(|| Error::invalid("Missing fixture text"))?;
        let path = row.text(3)?.ok_or_else(|| Error::invalid("Missing fixture path"))?;
        let normalized = normalized_file_url(path).map_err(|_| Error::invalid("Could not normalize fixture path"))?;
        if row.integer(1)? != Some(hash_string(text)) || row.integer(2)? != Some(song_key(text, text, text)) || row.integer(4)? != Some(hash_string(&normalized)) {
            return Err(Error::invalid(&format!("Identity mismatch for {text:?}: hash {:?}/{}, songKey {:?}/{}, pathId {:?}/{} ({normalized})",
                row.integer(1)?, hash_string(text), row.integer(2)?, song_key(text, text, text), row.integer(4)?, hash_string(&normalized))));
        }
        vectors += 1;
        Ok(())
    })?;
    if vectors < 10 {
        return Err("Identity fixture is incomplete".into());
    }
    let mut titles = 0;
    db.query(
        "SELECT fileName,title FROM rust_compat_titles",
        &[],
        &mut |row| {
            let file = row
                .text(0)?
                .ok_or_else(|| Error::invalid("Missing title fixture name"))?;
            if row.text(1)? != Some(lw_core::import::fallback_title(file)) {
                return Err(Error::invalid(&format!(
                    "Fallback title differs from Swift for {file:?}"
                )));
            }
            titles += 1;
            Ok(())
        },
    )?;
    if titles < 14 {
        return Err("Title fixture is incomplete".into());
    }
    let mut repo = LibraryRepository::new(db)?;
    let swift = repo
        .search_songs("Swift", 20, 0)?
        .pop()
        .ok_or("Missing Swift song")?;
    if swift.title != "Swift fixture"
        || swift.bookmark.as_deref() != Some(&[0, 255, 3][..])
        || swift.is_favorite != 1
        || swift.duration != Some(90.5)
        || swift.sample_rate != Some(48000.0)
        || swift.source_availability != Some(FileAvailability::OnDevice)
    {
        return Err("Rust did not hydrate the complete Swift song".into());
    }
    let wave_path = std::path::Path::new(&path)
        .parent()
        .ok_or("Missing waveform fixture directory")?
        .join("waveforms.sqlite");
    let waveforms = lw_core::waveform::Repository::open(
        wave_path
            .to_str()
            .ok_or("Waveform fixture path is not UTF-8")?,
    )?;
    let mut cached = waveforms
        .get(swift.song_key)?
        .ok_or("Missing Swift waveform")?;
    if cached.signature != "swift-waveform"
        || cached.peaks[1] != f32::from_bits(1)
        || cached.peaks[2] != 1e-30
    {
        return Err("Rust did not decode Swift float peaks".into());
    }
    for (i, peak) in cached.peaks.iter().enumerate().skip(3) {
        if *peak != i as f32 / 511.0 {
            return Err("Waveform float value changed".into());
        }
    }
    cached.peaks.reverse();
    waveforms.put(swift.song_key, "rust-waveform", &cached.peaks)?;
    let mut changed = swift.clone();
    changed.song_key = song_key(&swift.artist, "Edited by Rust", &swift.album);
    changed.title = "Edited by Rust".into();
    changed.duration = None;
    let updated = repo.upsert_song(&changed, 200.0)?;
    if updated.id != swift.id || updated.is_favorite != 1 || updated.duration != swift.duration {
        return Err("Rust reimport lost Swift identity or user data".into());
    }
    let rust = repo.upsert_song(
        &Song {
            song_key: song_key("Rust", "Rust fixture", "Exchange"),
            artist: "Rust".into(),
            title: "Rust fixture".into(),
            album: "Exchange".into(),
            album_artist: "Various".into(),
            release_year: Some(2026),
            disc_number: Some(2),
            track_number: Some(3),
            bookmark: Some(vec![9, 0, 254]),
            path_hash: hash_string("file:///fixture/rust.flac"),
            created_at: 100.0,
            local_file_path: Some("Music/rust.flac".into()),
            file_state: FileState::Copied,
            is_favorite: 1,
            relative_path: Some("rust.flac".into()),
            duration: Some(42.25),
            file_size: Some(1024),
            file_extension: Some("flac".into()),
            sample_rate: Some(96000.0),
            bit_depth: Some(24),
            source_availability: Some(FileAvailability::OnDevice),
            ..Song::default()
        },
        201.0,
    )?;
    let source_id = swift.source_id.ok_or("Swift source is missing")?;
    let source = repo
        .source(source_id)?
        .ok_or("Swift source did not hydrate")?;
    if source.created_at != 100.125
        || source.bookmark.as_deref() != Some(&[1, 2, 3][..])
        || source.sync_error.as_deref() != Some("Needs reconnect")
    {
        return Err("Rust did not preserve Swift source data and Date encoding".into());
    }
    if repo.get_or_create_user(12345)?.id != source.user_id {
        return Err("Rust did not reuse the Swift user".into());
    }
    let path = repo
        .source_path(source_id, "swift.flac")?
        .ok_or("Swift path is missing")?;
    if path.file_size != Some(1000)
        || path.file_hash_sha256.as_deref() != Some(&[0, 255][..])
        || path.created_at != 100.125
    {
        return Err("Rust did not hydrate the Swift path".into());
    }
    let root = &source.dir_path;
    let root_id = source.path_id;
    repo.reconnect_source(source_id, root, root_id, &[9, 8, 7], 201.625)?;
    let list = repo.create_playlist("Temporary Rust list", 200.375)?;
    repo.rename_playlist(list.id, "Rust list", 201.625)?;
    let swift_id = swift.id.ok_or("Swift song is unsaved")?;
    let rust_id = rust.id.ok_or("Rust song is unsaved")?;
    repo.add_playlist_songs(list.id, &[rust_id, swift_id, rust_id])?;
    repo.reorder_playlist(list.id, &[swift_id, rust_id])?;
    repo.upsert_source_paths(&[SourcePath {
        source_id,
        path_id: hash_string(&normalized_file_url(&format!("{}/rust.flac", root))?),
        parent_path_id: Some(root_id),
        name: "rust.flac".into(),
        relative_path: "rust.flac".into(),
        file_size: Some(1024),
        run_id: 2,
        created_at: 200.375,
        ..SourcePath::default()
    }])?;
    println!(
        "Swift/Rust exchange: {vectors} identity and {titles} title vectors; song, user, source, path, playlist, UTC Date and 512 float waveform exchange"
    );
    Ok(())
}
