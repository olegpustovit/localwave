//! Development-only golden mode: `--reference-screen <name>` seeds the shared Swift
//! `reference.json` into an isolated container (never user data), routes to the screen and prints
//! `LOCALWAVE_REFERENCE_READY` once drawing has been idle for two seconds. tools/reference-goldens.sh
//! then captures the real 1440x872 window; pxui_host has no pixel readback API.
use crate::app::App;
use lw_bin::library::LibraryWorker;
use std::path::Path;
use lw_ui::{Intent, catalog::Route};

/// Screens with a Rust equivalent. Skipped: add (picker sheet), folder (nested Files navigation),
/// track (anchored menu), onboarding (first-run flow is not seedable here).
pub const SCREENS: &[&str] = &[
    "songs", "artists", "album", "artist", "list", "favorites", "find", "find-results", "lists",
    "folders", "settings", "commands", "shortcuts", "now", "queue",
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

#[derive(Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn str(&self, key: &str) -> &str {
        match self.get(key) {
            Some(Json::Str(s)) => s,
            _ => "",
        }
    }
    pub fn num(&self, key: &str) -> f64 {
        match self.get(key) {
            Some(Json::Num(n)) => *n,
            _ => 0.0,
        }
    }
    pub fn flag(&self, key: &str) -> bool {
        matches!(self.get(key), Some(Json::Bool(true)))
    }
    pub fn items(&self, key: &str) -> &[Json] {
        match self.get(key) {
            Some(Json::Arr(items)) => items,
            _ => &[],
        }
    }
}

pub fn parse(text: &str) -> Result<Json, String> {
    let mut p = (text.as_bytes(), 0usize);
    let value = value(&mut p)?;
    ws(&mut p);
    if p.1 != p.0.len() {
        return Err("Trailing JSON data".into());
    }
    Ok(value)
}

type P<'a> = (&'a [u8], usize);
fn ws(p: &mut P) {
    while p.0.get(p.1).is_some_and(u8::is_ascii_whitespace) {
        p.1 += 1;
    }
}
fn eat(p: &mut P, byte: u8) -> Result<(), String> {
    ws(p);
    if p.0.get(p.1) == Some(&byte) {
        p.1 += 1;
        Ok(())
    } else {
        Err(format!("Expected '{}' at byte {}", byte as char, p.1))
    }
}
fn value(p: &mut P) -> Result<Json, String> {
    ws(p);
    match p.0.get(p.1).copied() {
        Some(b'{') => {
            p.1 += 1;
            let mut items = Vec::new();
            ws(p);
            if p.0.get(p.1) == Some(&b'}') {
                p.1 += 1;
                return Ok(Json::Obj(items));
            }
            loop {
                ws(p);
                let key = string(p)?;
                eat(p, b':')?;
                items.push((key, value(p)?));
                ws(p);
                match p.0.get(p.1) {
                    Some(b',') => p.1 += 1,
                    Some(b'}') => {
                        p.1 += 1;
                        return Ok(Json::Obj(items));
                    }
                    _ => return Err(format!("Bad object at byte {}", p.1)),
                }
            }
        }
        Some(b'[') => {
            p.1 += 1;
            let mut items = Vec::new();
            ws(p);
            if p.0.get(p.1) == Some(&b']') {
                p.1 += 1;
                return Ok(Json::Arr(items));
            }
            loop {
                items.push(value(p)?);
                ws(p);
                match p.0.get(p.1) {
                    Some(b',') => p.1 += 1,
                    Some(b']') => {
                        p.1 += 1;
                        return Ok(Json::Arr(items));
                    }
                    _ => return Err(format!("Bad array at byte {}", p.1)),
                }
            }
        }
        Some(b'"') => Ok(Json::Str(string(p)?)),
        Some(b't') if p.0[p.1..].starts_with(b"true") => {
            p.1 += 4;
            Ok(Json::Bool(true))
        }
        Some(b'f') if p.0[p.1..].starts_with(b"false") => {
            p.1 += 5;
            Ok(Json::Bool(false))
        }
        Some(b'n') if p.0[p.1..].starts_with(b"null") => {
            p.1 += 4;
            Ok(Json::Null)
        }
        Some(_) => {
            let start = p.1;
            while p.0.get(p.1).is_some_and(|b| b"+-.eE0123456789".contains(b)) {
                p.1 += 1;
            }
            std::str::from_utf8(&p.0[start..p.1])
                .ok()
                .and_then(|s| s.parse().ok())
                .map(Json::Num)
                .ok_or_else(|| format!("Bad value at byte {start}"))
        }
        None => Err("Unexpected end of JSON".into()),
    }
}
fn string(p: &mut P) -> Result<String, String> {
    if p.0.get(p.1) != Some(&b'"') {
        return Err(format!("Expected string at byte {}", p.1));
    }
    p.1 += 1;
    let mut out: Vec<u8> = Vec::new();
    let mut high: Option<u32> = None;
    loop {
        let byte = *p.0.get(p.1).ok_or("Unterminated string")?;
        p.1 += 1;
        match byte {
            b'"' => return String::from_utf8(out).map_err(|e| e.to_string()),
            b'\\' => {
                let escape = *p.0.get(p.1).ok_or("Unterminated escape")?;
                p.1 += 1;
                let c = match escape {
                    b'n' => '\n',
                    b't' => '\t',
                    b'r' => '\r',
                    b'b' => '\u{8}',
                    b'f' => '\u{c}',
                    b'u' => {
                        let hex = p.0.get(p.1..p.1 + 4).ok_or("Short \\u escape")?;
                        let unit = u32::from_str_radix(std::str::from_utf8(hex).map_err(|e| e.to_string())?, 16)
                            .map_err(|e| e.to_string())?;
                        p.1 += 4;
                        let code = match (high.take(), unit) {
                            (Some(h), 0xDC00..=0xDFFF) => 0x10000 + ((h - 0xD800) << 10) + (unit - 0xDC00),
                            (_, 0xD800..=0xDBFF) => {
                                high = Some(unit);
                                continue;
                            }
                            _ => unit,
                        };
                        char::from_u32(code).ok_or("Bad \\u escape")?
                    }
                    other => other as char,
                };
                out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
            }
            _ => out.push(byte),
        }
    }
}

use lw_core::hash_string as hash;

fn manifest() -> Result<Json, String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../localwave/ReferenceFixture/reference.json");
    parse(&std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?)
}

/// Seeds the isolated library from the Swift reference manifest: songs, playlists, sources,
/// queue and artwork. Play counts, pins and waveforms are not seeded.
pub fn library(base: &Path) -> std::io::Result<LibraryWorker> {
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
    )?;
    library.save_settings(&Settings { appearance: Appearance::Light, ..Default::default() })?;
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
    let mut songs = std::collections::HashMap::new();
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
    }
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
        context_data: None,
    })?;
    Ok(library)
}

/// Route state for a screen once the catalog is prepared.
pub fn route(app: &mut App, screen: &str) -> Result<(), String> {
    let App { shell_state, player, .. } = app;
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
        "find-results" => vec![Intent::Navigate(Route::Find), Intent::Search(lw_ui::browse::Query::new("cloud").ok_or("Bad query")?)],
        "lists" => vec![Intent::Navigate(Route::Lists)],
        "folders" => vec![Intent::Navigate(Route::Files)],
        "settings" => vec![albums, Intent::ShowSettings],
        "commands" => vec![albums, Intent::ShowCommands],
        "shortcuts" => vec![albums, Intent::ShowShortcuts],
        "now" => vec![albums],
        "queue" => vec![albums, Intent::ShowQueue],
        _ => vec![Intent::Navigate(Route::Songs)],
    };
    for intent in intents {
        shell_state.apply(intent);
    }
    if screen == "find-results" {
        let request = shell_state.request(player.catalog()).ok_or("Find request unavailable")?;
        player.browse(request).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_escapes_and_nesting() {
        let json = parse(r#"{"a":[1,-2.5,true,null],"s":"x·😀\n","o":{}}"#).unwrap();
        assert_eq!(json.items("a").len(), 4);
        assert_eq!(json.str("s"), "x\u{b7}\u{1f600}\n");
        assert!(parse("{\"a\":1} x").is_err());
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
}
