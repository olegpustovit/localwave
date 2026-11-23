//! Development-only golden mode: `--reference-screen <name>` seeds the shared Swift
//! `reference.json` into an isolated container (never user data), routes to the screen and prints
//! `LOCALWAVE_REFERENCE_READY` once drawing has been idle for two seconds. tools/reference-goldens.sh
//! then captures the real 1440x872 window; pxui_host has no pixel readback API.
use crate::app::App;
use lw_bin::library::LibraryWorker;
use std::path::Path;
use lw_ui::{Intent, catalog::Route};
use lw_bin::controller::Command;
use lw_bin::json::{Json, parse};

/// Screens with a Rust equivalent. Skipped: add (Swift's sheet; Rust uses the native picker) and
/// track (the song menu popup never paints in a captured window, see docs/rust-port.md). Folder is Files
/// browsing, onboarding an empty library.
pub const SCREENS: &[&str] = &[
    "songs", "artists", "album", "artist", "list", "favorites", "find", "find-results", "lists",
    "folders", "settings", "commands", "shortcuts", "now", "queue", "folder", "onboarding",
];

pub fn from_args(args: &[String]) -> Result<Option<String>, String> {
    let Some(at) = args.iter().position(|arg| arg == "--reference-screen") else {
        return Ok(None);
    };
    match args.get(at + 1) {
        Some(name) if SCREENS.contains(&name.as_str()) => Ok(Some(name.clone())),
        Some(name) => Err(format!("Unknown reference screen {name}; expected one of {SCREENS:?}")),
        None => Err("--reference-screen requires a screen name".into()),
    }
}

use lw_core::hash_string as hash;

fn manifest() -> Result<Json, String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../localwave/ReferenceFixture/reference.json");
    parse(&std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?)
}

/// Swift's fixture source labels: (root, scanned) per source, in source-id order.
pub fn source_display() -> std::io::Result<Vec<(String, String)>> {
    let data = manifest().map_err(std::io::Error::other)?;
    Ok(data.items("sources").iter().map(|s| (s.str("root").to_owned(), s.str("scanned").to_owned())).collect())
}

/// Seeds the isolated library from the Swift reference manifest: songs, playlists, sources, queue,
/// artwork, non-audio files, play counts/last played, pins and waveforms (LWReferenceFixture.seed).
pub fn library(base: &Path, screen: &str, table_styles: lw_ui::catalog::TableStyles) -> std::io::Result<LibraryWorker> {
    use lw_core::{FileAvailability, FileState, Song, Source, SourcePath, preferences::*};
    let other = |e: String| std::io::Error::other(e);
    let data = manifest().map_err(other)?;
    let documents = base.join("Documents");
    let covers = documents.join("CoverArt");
    std::fs::create_dir_all(&covers)?;
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../localwave/ReferenceFixture");
    for album in data.items("albums") {
        let name = format!("reference-{}.png", album.str("id"));
        std::fs::copy(fixtures.join(&name), covers.join(&name))?;
    }
    let suite = format!("com.olegpustovit.localwave.{}", base.file_name().unwrap().to_string_lossy());
    let mut library = LibraryWorker::open(
        &base.join("library.sqlite"),
        lw_apple_sys::preferences::Preferences::open(Some(&suite))?,
        lw_apple_sys::preferences::AppPaths {
            documents,
            caches: base.join("Caches"),
            temporary: base.join("tmp"),
            test_database: None,
        },
        table_styles,
    )?;
    library.trust_stored_availability = true;
    // Swift's fixture keeps the System appearance; the app pins the system to light in reference mode
    library.save_settings(&Settings { appearance: Appearance::System, ..Default::default() })?;
    if screen == "onboarding" {
        return Ok(library); // first run: no sources, no songs
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let account = library.current_sources()?;
    let mut sources = std::collections::HashMap::new();
    for (index, root) in data.items("sources").iter().enumerate() {
        let id = root.str("id");
        let age = match id {
            "icloud" => 120.0,
            "gdrive" => 840.0,
            "dropbox" => 259_200.0,
            _ => 0.0,
        };
        // Real empty directory + bookmark, as the Swift fixture does; the audio files do not exist.
        let directory = base.join("Sources").join(root.str("name"));
        std::fs::create_dir_all(&directory)?;
        let bookmark = lw_apple_sys::bookmarks::create(&directory)?;
        let source = library
            .repository
            .create_source(&Source {
                user_id: account.user.id,
                dir_path: directory.to_string_lossy().into_owned(),
                bookmark: Some(bookmark),
                path_id: hash(id),
                source_type: (id == "icloud").then(|| "iCloud".to_string()),
                total_paths: Some(data.items("tracks").iter().filter(|t| t.str("dir").starts_with(&format!("{id}/"))).count() as i64),
                sync_error: root.flag("reconnect").then(|| "Access expired. The folder was moved, renamed, or the app was reinstalled.".to_string()),
                is_current: index == 0,
                created_at: now,
                last_synced_at: Some(now - age),
                ..Default::default()
            })
            .map_err(|e| other(e.to_string()))?;
        sources.insert(id.to_string(), source.id.unwrap_or_default());
    }
    let root_of = |dir: &str| dir.split('/').next().unwrap_or("").to_string();
    let mut paths = Vec::new();
    for folder in data.items("folders") {
        let path = folder.str("path");
        // The root is the source itself; an empty relative path is invalid.
        if let Some(&source_id) = sources.get(&root_of(path)).filter(|_| path.contains('/')) {
            paths.push(SourcePath {
                source_id,
                path_id: hash(path),
                parent_path_id: match folder.get("parent") { Some(Json::Str(p)) => Some(hash(p)), _ => None },
                name: folder.str("name").into(),
                relative_path: path.split_once('/').map_or("", |(_, rest)| rest).into(),
                is_directory: true,
                created_at: now,
                ..Default::default()
            });
        }
    }
    for file in data.items("nonAudio") {
        let dir = file.str("dir");
        let Some(&source_id) = sources.get(&root_of(dir)) else { continue };
        let relative_dir = dir.split_once('/').map_or("", |(_, rest)| rest);
        let parent = base.join("Sources").join(source_name(&data, &root_of(dir))).join(relative_dir);
        std::fs::create_dir_all(&parent)?;
        let name = file.str("name");
        let target = parent.join(name);
        // Swift copies the album cover for cover.jpg and writes a rip log otherwise.
        let album = data.items("tracks").iter().find(|t| t.str("dir") == dir).map(|t| t.str("al"));
        match album.filter(|_| name.ends_with(".jpg")) {
            Some(album) => drop(std::fs::copy(fixtures.join(format!("reference-{album}.png")), &target)?),
            None => std::fs::write(&target, "LocalWave reference rip log.\n")?,
        }
        paths.push(SourcePath {
            source_id,
            path_id: hash(&format!("{dir}/{name}")),
            parent_path_id: Some(hash(dir)),
            name: name.into(),
            relative_path: if relative_dir.is_empty() { name.into() } else { format!("{relative_dir}/{name}") },
            file_size: Some(std::fs::metadata(&target)?.len() as i64),
            created_at: now,
            ..Default::default()
        });
    }
    let mut songs = std::collections::HashMap::new();
    let mut audio = std::collections::HashMap::new();
    let current = data.get("player").map_or(0, |p| p.num("i") as usize);
    let documents_music = base.join("Documents").join("Music");
    let by_id = |id: &Json| data.items("tracks").iter().find(|t| matches!(id, Json::Str(s) if s == t.str("id")));
    // Swift's library order, not manifest order.
    for track in data.items("order").iter().filter_map(by_id) {
        let album = data.items("albums").iter().find(|a| a.str("id") == track.str("al")).ok_or_else(|| other("Unknown album".into()))?;
        let dir = track.str("dir");
        let relative = format!("{}/{}", dir.split_once('/').map_or("", |(_, rest)| rest), track.str("file"));
        let relative = relative.trim_start_matches('/').to_string();
        let local = track.str("state") == "local";
        let rate = album.str("rate");
        let source_id = sources.get(&root_of(dir)).copied();
        let slot = data.get("player").and_then(|p| p.items("queue").iter().position(|q| matches!(q, Json::Str(id) if id == track.str("id"))));
        // Swift displays a paused snapshot with an empty Music cache. Give Settings playable
        // fixture files outside that cache so restoring the paused queue leaves the same total.
        let fixture_audio = (screen == "settings" && slot.is_some())
            .then(|| format!("Reference Audio/{:016x}.wav", hash(track.str("id"))));
        let song = library
            .repository
            .upsert_song(
                &Song {
                    song_key: hash(track.str("id")),
                    artist: track.str("ar").into(),
                    title: track.str("t").into(),
                    album: album.str("t").into(),
                    album_artist: album.str("ar").into(),
                    release_year: Some(album.num("y") as i64),
                    disc_number: Some(1),
                    track_number: Some(track.num("n") as i64),
                    cover_art_path: Some(format!("CoverArt/reference-{}.png", album.str("id"))),
                    path_hash: hash(&format!("{dir}/{}", track.str("file"))),
                    created_at: track.num("added") / 1000.0,
                    file_state: if local { FileState::Copied } else { FileState::BookmarkOnly },
                    is_favorite: i64::from(track.flag("fav")),
                    source_id,
                    relative_path: Some(relative.clone()),
                    local_file_path: fixture_audio.clone(),
                    duration: Some(track.num("dur")),
                    file_size: Some((track.num("size") * 1_048_576.0) as i64),
                    file_extension: Some(track.str("ext").into()),
                    sample_rate: Some(if rate.contains("96 kHz") { 96000.0 } else if rate.contains("48 kHz") { 48000.0 } else { 44100.0 }),
                    bit_depth: if rate.contains("24 bit") { Some(24) } else if album.str("f") == "flac" { Some(16) } else { None },
                    source_availability: Some(if local { FileAvailability::OnDevice } else { FileAvailability::InCloud }),
                    ..Default::default()
                },
                now,
            )
            .map_err(|e| other(e.to_string()))?;
        if let Some(source_id) = source_id {
            paths.push(SourcePath {
                source_id,
                path_id: song.path_hash,
                parent_path_id: Some(hash(dir)),
                name: track.str("file").into(),
                relative_path: relative,
                file_size: song.file_size,
                created_at: now,
                ..Default::default()
            });
        }
        songs.insert(track.str("id").to_string(), song.id.unwrap_or_default());
        // Queue songs get real playable files: the current one at full length (the native player
        // reports its duration), the rest one second so next-item preparation succeeds.
        if let Some(slot) = slot {
            let file = fixture_audio.as_ref().map_or_else(
                || base.join("Sources").join(source_name(&data, &root_of(dir))).join(song.relative_path.as_deref().unwrap_or_default()),
                |path| base.join("Documents").join(path),
            );
            std::fs::create_dir_all(file.parent().ok_or_else(|| other("No parent".into()))?)?;
            std::fs::write(&file, silent_wav(if slot == current { track.num("dur") as u32 } else { 1 }))?;
            if slot == current && fixture_audio.is_some() {
                audio.insert(track.str("id").to_string(), file.canonicalize()?);
            } else if slot == current {
                // Playing copies a Copied song into Music/<sha256>.<ext>, and the waveform signature is
                // derived from that name: pre-place the identical copy to compute the same signature.
                let sha = lw_core::import::hex_digest(&lw_apple_sys::audio::read(&file, false, &std::sync::atomic::AtomicBool::new(false))?.sha256);
                std::fs::create_dir_all(&documents_music)?;
                let copy = documents_music.join(format!("{sha}.{}", track.str("ext")));
                std::fs::copy(&file, &copy)?;
                audio.insert(track.str("id").to_string(), copy);
            }
        }
    }
    seed_history(&suite, &data, &songs, &audio, &documents_music, &base.join("Caches"))?;
    library.repository.upsert_source_paths(&paths).map_err(|e| other(e.to_string()))?;
    for list in data.items("lists").iter().filter(|l| l.num("auto") != 1.0) {
        let ids: Vec<i64> = list.items("tracks").iter().filter_map(|t| if let Json::Str(t) = t { songs.get(t).copied() } else { None }).collect();
        library.repository.create_playlist_with_songs(list.str("name"), &ids, now).map_err(|e| other(e.to_string()))?;
    }
    let player = data.get("player").ok_or_else(|| other("Missing player".into()))?;
    library.save_player_state(&PlayerState {
        song_ids: player.items("queue").iter().filter_map(|t| if let Json::Str(t) = t { songs.get(t).copied() } else { None }).collect(),
        current_index: player.num("i") as i64,
        volume: Some(player.num("vol") as f32),
        context_data: match player.str("ctx").strip_prefix("al:") {
            Some(id) => {
                let album = data.items("albums").iter().find(|a| a.str("id") == id).ok_or_else(|| other("Unknown context album".into()))?;
                Some(library.playback_context_data(&lw_core::context::PlaybackContext::Album { name: album.str("t").into(), artist: album.str("ar").into() })?)
            }
            None => None,
        },
    })?;
    Ok(library)
}

/// Play counts (`lwPlayCounts`), last played (`lwLastPlayed`), pins (`lwPinnedSongs`) and persisted
/// waveforms, keyed like Swift: by song row id, and by song key for waveforms.
fn seed_history(
    suite: &str,
    data: &Json,
    songs: &std::collections::HashMap<String, i64>,
    audio: &std::collections::HashMap<String, std::path::PathBuf>,
    music: &Path,
    caches: &Path,
) -> std::io::Result<()> {
    let other = |e: String| std::io::Error::other(e);
    let preferences = lw_apple_sys::preferences::Preferences::open(Some(suite))?;
    let directory = caches.join("LocalWave");
    std::fs::create_dir_all(&directory)?;
    let waveforms = lw_core::waveform::Repository::open(directory.join("waveforms.sqlite").to_str().ok_or_else(|| other("Fixture path is not UTF-8".into()))?)
        .map_err(|e| other(e.to_string()))?;
    for track in data.items("tracks") {
        let id = *songs.get(track.str("id")).ok_or_else(|| other("Unknown track".into()))?;
        // record_playback increments, so replaying `plays` times at the added date gives both values.
        for _ in 0..track.num("plays") as u32 {
            preferences.record_playback(id, track.num("added") / 1000.0)?;
        }
        if track.flag("pin") {
            preferences.set_pinned(id, true);
        }
        // Swift draws the manifest's phone peaks in its phone player (`LWReferenceFixture.peaks`: 200 bars take the desktop set)
        let set = if std::env::var_os("LOCALWAVE_REFERENCE_MOBILE").is_some() { "mobile" } else { "desktop" };
        let Some(Json::Arr(peaks)) = data.get("waveforms").and_then(|w| w.get(track.str("id"))).and_then(|w| w.get(set)) else { continue };
        // A real file makes the worker re-derive the signature; match it so the seeded peaks stay.
        let signature = match audio.get(track.str("id")) {
            Some(file) => lw_core::import::hex_digest(&lw_apple_sys::waveform::version(file, music, false)?.digest),
            None => "reference".into(),
        };
        waveforms
            .put(hash(track.str("id")), &signature, &resample(peaks))
            .map_err(|e| other(e.to_string()))?;
    }
    Ok(())
}

/// Fill each render bucket with the matching peak from the 96- or 200-bar reference.
fn resample(peaks: &[Json]) -> lw_core::waveform::Peaks {
    let mut samples = [0.0; lw_core::waveform::PEAK_COUNT];
    let count = samples.len();
    for (bar, peak) in peaks.iter().enumerate() {
        let Json::Num(peak) = peak else { continue };
        samples[bar * count / peaks.len()..(bar + 1) * count / peaks.len()]
            .fill(peak.clamp(0.0, 1.0) as f32);
    }
    samples
}

/// Silent 8 kHz mono 8-bit WAV of the exact duration; AVFoundation sniffs content, not the extension.
fn silent_wav(seconds: u32) -> Vec<u8> {
    let data = seconds * 8000;
    let mut bytes = Vec::with_capacity(44 + data as usize);
    for part in [&b"RIFF"[..], &(36 + data).to_le_bytes(), b"WAVEfmt ", &16u32.to_le_bytes(), &1u16.to_le_bytes(), &1u16.to_le_bytes(),
        &8000u32.to_le_bytes(), &8000u32.to_le_bytes(), &1u16.to_le_bytes(), &8u16.to_le_bytes(), b"data", &data.to_le_bytes()] {
        bytes.extend_from_slice(part);
    }
    bytes.resize(44 + data as usize, 0x80);
    bytes
}

/// Encodes the content of a 1× RGBA drawable as a PNG, cropping the first `inset` titlebar rows.
pub fn capture_png((width, height, rgba): (u32, u32, &[u8]), inset: u32) -> Result<Vec<u8>, String> {
    let invalid = || format!("Capture {width}x{height} has invalid RGBA data or titlebar inset {inset}");
    let stride = (width as usize).checked_mul(4).filter(|&bytes| bytes > 0).ok_or_else(invalid)?;
    if stride.checked_mul(height as usize) != Some(rgba.len()) || inset >= height {
        return Err(invalid());
    }
    let h = height - inset;
    let content = &rgba[inset as usize * stride..];
    let mut raw = Vec::with_capacity(content.len().checked_add(h as usize).ok_or_else(invalid)?);
    for row in content.chunks_exact(stride) {
        raw.push(0); // no row filter
        raw.extend_from_slice(row);
    }
    // zlib stream of stored (uncompressed) deflate blocks: the fixture needs no compressor
    let mut zlib = vec![0x78, 0x01];
    let last = raw.len().div_ceil(65535) - 1;
    for (index, block) in raw.chunks(65535).enumerate() {
        let len = block.len() as u16;
        zlib.push(u8::from(index == last));
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut header = [0u8; 13];
    header[..4].copy_from_slice(&width.to_be_bytes());
    header[4..8].copy_from_slice(&h.to_be_bytes());
    header[8..].copy_from_slice(&[8, 6, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, body) in [(b"IHDR", &header[..]), (b"IDAT", &zlib[..]), (b"IEND", &[][..])] {
        png.extend_from_slice(&(body.len() as u32).to_be_bytes());
        let start = png.len();
        png.extend_from_slice(kind);
        png.extend_from_slice(body);
        png.extend_from_slice(&crc32(&png[start..]).to_be_bytes());
    }
    Ok(png)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

fn source_name<'a>(data: &'a Json, id: &str) -> &'a str {
    data.items("sources").iter().find(|s| s.str("id") == id).map_or("", |s| s.str("name"))
}

/// Paused position of the Swift snapshot, in seconds.
pub fn position() -> Result<f64, String> {
    Ok(manifest()?.get("player").map_or(0.0, |p| p.num("t")))
}

/// Screens that open over the Albums page; their overlay is applied after the page has drawn.
pub fn is_overlay(screen: &str) -> bool {
    matches!(screen, "settings" | "commands" | "shortcuts" | "queue" | "folder")
}

/// Route state for a screen once the catalog is prepared. Overlay screens are routed in two calls:
/// `late == false` shows the page, `late == true` (after the page drew) opens the overlay.
pub fn route(app: &mut App, screen: &str, late: bool) -> Result<(), String> {
    let App { shell_state, player, files, .. } = app;
    let library = &player.catalog().library;
    let find = |list: &[lw_ui::catalog::Collection], title: &str| {
        list.iter().position(|c| c.title == title).ok_or_else(|| format!("Reference collection missing: {title}"))
    };
    // Swift's sheet screens sit over its default Albums section.
    let albums = Intent::Navigate(Route::Albums);
    let intents = match screen {
        "artists" => vec![Intent::Navigate(Route::Artists)],
        "album" => vec![Intent::Navigate(Route::Album(find(&library.albums, "Field Recordings 04")?))],
        "artist" => vec![Intent::Navigate(Route::Artist(find(&library.artists, "Juno Abara")?))],
        "list" => vec![Intent::Navigate(Route::Collection(find(&library.collections, "Late drive")?))],
        "favorites" => vec![Intent::Navigate(Route::Collection(find(&library.collections, "Favorites")?))],
        "find" => vec![Intent::Navigate(Route::Find)],
        "find-results" => vec![Intent::Navigate(Route::Find), Intent::SearchResults(lw_ui::browse::Query::new("cloud").ok_or("Bad query")?)],
        "lists" => vec![Intent::Navigate(Route::Lists)],
        "folders" => vec![Intent::Navigate(Route::Files)],
        "settings" => vec![albums, Intent::ShowSettings],
        "commands" => vec![albums, Intent::ShowCommands],
        "shortcuts" => vec![albums, Intent::ShowShortcuts],
        "now" => vec![albums, Intent::TogglePlayer],
        "onboarding" => vec![],
        "folder" => vec![Intent::Navigate(Route::Files)],
        // Swift opens Now first (`showNow`): on a phone the queue sheet covers the full-screen player
        "queue" if std::env::var_os("LOCALWAVE_REFERENCE_MOBILE").is_some() => vec![albums, Intent::TogglePlayer, Intent::ShowQueue],
        "queue" => vec![albums, Intent::ShowQueue],
        _ => vec![Intent::Navigate(Route::Songs)],
    };
    let split = if is_overlay(screen) { intents.len().saturating_sub(1) } else { intents.len() };
    for intent in if late { &intents[split..] } else { &intents[..split] } {
        shell_state.apply(intent.clone());
    }
    if !late {
        // Swift's snapshot is paused at `t`; prepare the restored current item without autoplay (the frame loop seeks once its duration is known).
        if let Some(current) = player.status().current {
            player.command(Command::Select { occurrence: current.id, autoplay: false }).map_err(|e| e.to_string())?;
        }
    }
    if late && screen == "folder" {
        let snapshot = files.snapshot();
        let (user, source) = (snapshot.account.ok_or("No account")?, snapshot.sources.first().ok_or("No source")?.id);
        let location = lw_ui::files::Location { user, source, parent: Some(hash("icloud/Halden Marsh/Field Recordings 04")) };
        files.request(lw_ui::files::Request::Browse(location)).map_err(|e| e.to_string())?;
    }
    // actions.rs requests these when the intent is dispatched; the route applies intents directly.
    if !late && matches!(screen, "folders" | "folder") {
        files.request(lw_ui::files::Request::Roots).map_err(|e| e.to_string())?;
    }
    // actions.rs reads offline space when Settings opens
    if late && screen == "settings" {
        player.command(Command::ReadStorage).map_err(|e| e.to_string())?;
    }
    if (late || !is_overlay(screen)) && matches!(screen, "find-results" | "commands") {
        let request = shell_state.request(player.catalog()).ok_or("Browse request unavailable")?;
        player.browse(request).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resamples_desktop_peaks_to_the_cache_width() {
        let peaks: Vec<Json> = (0..200).map(|i| Json::Num(i as f64 / 199.0)).collect();
        let out = resample(&peaks);
        assert_eq!((out[0], out[511]), (0.0, 1.0));
        assert!(out.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn reference_waveform_preserves_every_displayed_peak() {
        for count in [96, 200] {
            let peaks: Vec<_> = (0..count).map(|i| Json::Num(if i % 2 == 0 { 0.75 } else { 0.25 })).collect();
            let samples = resample(&peaks);
            for bar in 0..count {
                let bucket = &samples[bar * samples.len() / count..(bar + 1) * samples.len() / count];
                let actual = bucket.iter().copied().fold(0.0_f32, f32::max);
                assert_eq!(actual, if bar % 2 == 0 { 0.75 } else { 0.25 }, "bar {bar} of {count}");
            }
        }
    }

    #[test]
    fn reads_the_shared_manifest_and_validates_screens() {
        let data = manifest().unwrap();
        assert_eq!(data.items("tracks").len(), 92);
        let args = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(from_args(&args(&["x"])), Ok(None));
        assert_eq!(from_args(&args(&["x", "--reference-screen", "queue"])), Ok(Some("queue".into())));
        assert!(from_args(&args(&["x", "--reference-screen", "nope"])).is_err());
    }

    #[test]
    fn settings_fixture_restores_playable_audio_and_waveform_without_filling_the_music_cache() {
        let base = std::env::temp_dir().join(format!("lw-reference-settings-{}", std::process::id()));
        let mut worker = library(&base, "settings", lw_bin::library::headless_table_styles()).unwrap();
        let player = worker.load_player_state().unwrap();
        let song = player.song_ids[player.current_index as usize];
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        let audio = worker.prepare_playback(song, 1, 1, 1.0, &cancelled).unwrap();
        assert!(audio.path.starts_with(base.canonicalize().unwrap().join("Documents/Reference Audio")));
        assert_eq!(worker.cache_usage().unwrap().0, 0);
        let peaks = worker.waveform_peaks(song, 1.0, &cancelled).unwrap().unwrap();
        assert!(peaks.iter().any(|peak| *peak > 0.1), "restore the seeded waveform");
        drop(audio);
        drop(worker);
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn capture_png_crops_the_inset_and_preserves_native_pixels() {
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
        let rgba = [
            0, 0, 0, 255, 0, 0, 0, 255,
            10, 20, 30, 255, 40, 50, 60, 200,
            70, 80, 90, 150, 100, 110, 120, 100,
        ];
        let png = capture_png((2, 3, &rgba), 1).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[16..24], &[0, 0, 0, 2, 0, 0, 0, 2]);
        // IDAT starts with a zlib header and one final stored block containing two RGBA scanlines.
        let idat = 8 + 25 + 8;
        assert_eq!(&png[idat..idat + 7], &[0x78, 0x01, 1, 18, 0, 0xED, 0xFF]);
        assert_eq!(&png[idat + 7..idat + 25], &[
            0, 10, 20, 30, 255, 40, 50, 60, 200,
            0, 70, 80, 90, 150, 100, 110, 120, 100,
        ]);
        assert!(capture_png((2, 3, &rgba), 3).is_err());
        assert!(capture_png((2, 3, &rgba[..23]), 1).is_err());
        assert!(capture_png((0, 3, &[]), 1).is_err());
        assert!(capture_png((u32::MAX, u32::MAX, &[]), 1).is_err());
    }
}
