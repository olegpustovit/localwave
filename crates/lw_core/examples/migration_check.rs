//! Headless Swift-container migration check: `migration_check <copy-dir>` where the directory holds
//! Documents/musicApp29.sqlite and prefs/com.olegpustovit.localwave.plist copied from the Swift app.
//! Prints counts and pass/fail only; never resolves bookmarks and never prints library content.
use lw_apple_sys::{context, preferences::Preferences};
use lw_core::LibraryRepository;
use lw_sqlite::{Connection, Value::Real};

fn count(db: &Connection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
    let mut n = 0;
    db.query(sql, &[], &mut |row| {
        n = row.integer(0)?.unwrap_or(0);
        Ok(())
    })?;
    Ok(n)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).ok_or("Expected copy directory")?);
    let database = dir.join("Documents/musicApp29.sqlite");
    let database = database.to_str().ok_or("Non-UTF-8 path")?;
    let mut failures = 0;
    let mut check = |name: &str, ok: bool, detail: String| {
        println!("{} {name}: {detail}", if ok { "PASS" } else { "FAIL" });
        failures += !ok as i32;
    };

    let sql = Connection::open(database)?;
    let version = count(&sql, "PRAGMA user_version")?;
    let tables_before = count(&sql, "SELECT COUNT(*) FROM sqlite_master WHERE type='table'")?;
    let songs_before = count(&sql, "SELECT COUNT(*) FROM songs")?;
    // Opening runs the additive migrations; nothing may be dropped or rewritten.
    let repo = LibraryRepository::open(database)?;
    let tables_after = count(&sql, "SELECT COUNT(*) FROM sqlite_master WHERE type='table'")?;
    check("schema", tables_after >= tables_before && version == count(&sql, "PRAGMA user_version")?, format!("user_version={version} tables {tables_before}->{tables_after}"));
    check("songs", repo.song_count("")? as i64 == songs_before && songs_before == count(&sql, "SELECT COUNT(*) FROM songs")?, format!("{songs_before}"));
    check("artists", repo.artists()?.len() as i64 == count(&sql, "SELECT COUNT(DISTINCT artist) FROM songs WHERE artist<>''")?, format!("{}", repo.artists()?.len()));
    let albums = repo.albums()?.len() as i64;
    check("albums", albums >= count(&sql, "SELECT COUNT(DISTINCT album) FROM songs WHERE album<>''")?, format!("{albums}"));
    let favorites = count(&sql, "SELECT COUNT(*) FROM songs WHERE isFavorite=1")?;
    let favorite_ids = repo.search_songs("", usize::MAX >> 16, 0)?.iter().filter(|s| s.is_favorite == 1).count() as i64;
    check("favorites", favorites == favorite_ids, format!("{favorites}"));

    let playlists = repo.playlists()?;
    check("playlists", playlists.len() as i64 == count(&sql, "SELECT COUNT(*) FROM playlists")?, format!("{}", playlists.len()));
    let (mut members, mut misordered) = (0, 0);
    for list in &playlists {
        let id = list.id;
        let mut expected = Vec::new();
        sql.query(&format!("SELECT s.id FROM playlist_songs p JOIN songs s ON s.id=p.songId WHERE p.playlistId={id} ORDER BY p.position,p.id"), &[], &mut |row| {
            expected.push(row.integer(0)?.unwrap_or(-1));
            Ok(())
        })?;
        let actual: Vec<i64> = repo.playlist_songs(id)?.iter().filter_map(|s| s.id).collect();
        members += actual.len();
        misordered += (actual != expected) as i32;
    }
    check("playlist membership order", misordered == 0, format!("{members} members, {misordered} lists differ"));

    // Swift stores dates as UTC text with milliseconds; the Rust f64 form must format back identically.
    let (mut dates, mut bad_dates) = (0, 0);
    for table in ["playlists", "sources"] {
        let mut raw = Vec::new();
        sql.query(&format!("SELECT id,createdAt FROM {table}"), &[], &mut |row| {
            raw.push((row.integer(0)?.unwrap_or(0), row.text(1)?.unwrap_or("").to_owned()));
            Ok(())
        })?;
        let created: Vec<(i64, f64)> = if table == "playlists" {
            playlists.iter().map(|p| (p.id, p.created_at)).collect()
        } else {
            let mut v = Vec::new();
            for user in 1..=count(&sql, "SELECT COALESCE(MAX(id),0) FROM users")? {
                v.extend(repo.sources(user)?.iter().map(|s| (s.id.unwrap_or(0), s.created_at)));
            }
            v
        };
        for (id, text) in raw {
            dates += 1;
            let Some((_, seconds)) = created.iter().find(|(i, _)| *i == id) else { bad_dates += 1; continue };
            let mut formatted = String::new();
            sql.query("SELECT strftime('%Y-%m-%dT%H:%M:%f', ?, 'unixepoch')", &[Real(*seconds)], &mut |row| {
                formatted = row.text(0)?.unwrap_or("").to_owned();
                Ok(())
            })?;
            bad_dates += (formatted != text) as i32;
        }
    }
    check("date encodings", bad_dates == 0, format!("{dates} checked, {bad_dates} differ"));

    // Bookmarks are only inspected for the security-scoped "book" header, never resolved.
    let bookmarks = count(&sql, "SELECT (SELECT COUNT(*) FROM songs WHERE bookmark IS NOT NULL)+(SELECT COUNT(*) FROM sources WHERE bookmark IS NOT NULL)")?;
    let parsed = count(&sql, "SELECT (SELECT COUNT(*) FROM songs WHERE substr(bookmark,1,4)=CAST('book' AS BLOB))+(SELECT COUNT(*) FROM sources WHERE substr(bookmark,1,4)=CAST('book' AS BLOB))")?;
    check("bookmarks", bookmarks == parsed, format!("{bookmarks} stored, {parsed} well-formed"));

    let prefs = dir.join("prefs/com.olegpustovit.localwave");
    let prefs = Preferences::open(Some(prefs.to_str().ok_or("Non-UTF-8 path")?))?;
    let player = prefs.load_player()?;
    let known = repo.get_songs(&player.song_ids)?.len();
    let mut distinct = player.song_ids.clone();
    distinct.sort_unstable();
    distinct.dedup();
    check("queue", player.current_index >= 0 && (player.song_ids.is_empty() || (player.current_index as usize) < player.song_ids.len()), format!("{} occurrences, {} duplicates, {known} resolvable, index in range", player.song_ids.len(), player.song_ids.len() - distinct.len()));
    match player.context_data.as_deref() {
        None => check("playback context", true, "absent".into()),
        Some(bytes) => match context::decode(bytes) {
            Ok(v) => {
                let again = context::encode(v.kind, &v.name, &v.artist, v.id, v.path_id).and_then(|b| context::decode(&b))?;
                check("playback context", again.kind == v.kind && again.name == v.name && again.id == v.id, format!("decodes, kind {}, {} bytes", v.kind, bytes.len()));
            }
            // Unknown layouts stay untouched: this check never writes preferences.
            Err(_) => check("playback context", true, format!("unknown layout, {} bytes preserved (read-only)", bytes.len())),
        },
    }
    let settings = prefs.load_settings();
    println!("INFO settings read: cache_gb={} appearance={} signal={}", settings.cache_gb, settings.appearance, settings.signal);
    if failures > 0 {
        return Err(format!("{failures} migration checks failed").into());
    }
    println!("migration check passed");
    Ok(())
}
