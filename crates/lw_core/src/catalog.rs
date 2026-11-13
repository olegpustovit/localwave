use crate::repository::{SONG_COLUMNS, prefix_query, read_song, required};
use crate::{LibraryRepository, Playlist, Song, Source, SourcePath, User};
use Value::{Blob, Integer, Null, Real, Text};
use lw_apple_sys::sqlite::{Connection, Error, Result, Row, Value};
use std::collections::{HashMap, HashSet};

// SQLite.swift Date columns are UTC text with millisecond precision; song dates are numeric.
const SOURCE_COLUMNS: &str = "id,dirPath,pathId,userId,type,bookmark,totalPaths,syncError,isCurrent,CAST(strftime('%s',createdAt) AS REAL)+CAST(substr(createdAt,20) AS REAL),CAST(strftime('%s',lastSyncedAt) AS REAL)+CAST(substr(lastSyncedAt,20) AS REAL),CAST(strftime('%s',updatedAt) AS REAL)+CAST(substr(updatedAt,20) AS REAL)";
const PATH_COLUMNS: &str = "id,sourceId,pathId,parentPathId,name,relativePath,isDirectory,fileSize,fileHashSHA256,runId,CAST(strftime('%s',createdAt) AS REAL)+CAST(substr(createdAt,20) AS REAL),CAST(strftime('%s',updatedAt) AS REAL)+CAST(substr(updatedAt,20) AS REAL)";
const LIST_COLUMNS: &str = "id,name,CAST(strftime('%s',createdAt) AS REAL)+CAST(substr(createdAt,20) AS REAL),CAST(strftime('%s',updatedAt) AS REAL)+CAST(substr(updatedAt,20) AS REAL)";

pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(include_str!("catalog.sql"))?;
    for (table, column, definition) in [
        ("sources", "bookmark", "BLOB"),
        ("source_paths", "fileSize", "INTEGER"),
    ] {
        let mut exists = false;
        db.query(&format!("PRAGMA table_info({table})"), &[], &mut |row| {
            exists |= row.text(1)? == Some(column);
            Ok(())
        })?;
        if !exists {
            db.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN {column} {definition}"
            ))?;
        }
    }
    db.execute_batch("CREATE INDEX IF NOT EXISTS lw_sources_user ON sources(userId); CREATE INDEX IF NOT EXISTS lw_paths_source_relative ON source_paths(sourceId,relativePath); CREATE INDEX IF NOT EXISTS lw_paths_source_id ON source_paths(sourceId,pathId); CREATE INDEX IF NOT EXISTS lw_paths_parent ON source_paths(sourceId,parentPathId); CREATE INDEX IF NOT EXISTS lw_playlist_order ON playlist_songs(playlistId,position); CREATE INDEX IF NOT EXISTS lw_playlist_song ON playlist_songs(songId)")
}

pub(crate) fn require_source(db: &Connection, id: i64) -> Result<()> {
    require_row(
        db,
        "SELECT 1 FROM sources WHERE id=?",
        id,
        "This music source was removed",
    )
}

fn require_row(db: &Connection, query: &str, id: i64, message: &str) -> Result<()> {
    let mut exists = false;
    db.query(query, &[Integer(id)], &mut |_| {
        exists = true;
        Ok(())
    })?;
    if exists {
        Ok(())
    } else {
        Err(Error::invalid(message))
    }
}

impl LibraryRepository {
    pub fn playlist(&self, id: i64) -> Result<Option<Playlist>> {
        playlist(&self.db, id)
    }
    pub fn get_or_create_user(&mut self, icloud_id: i64) -> Result<User> {
        if let Some(user) = self.user(icloud_id)? {
            return Ok(user);
        }
        self.db.transaction(|db| {
            db.execute(
                "INSERT INTO users (icloudId) VALUES (?) ON CONFLICT(icloudId) DO NOTHING",
                &[Integer(icloud_id)],
            )?;
            let mut id = None;
            db.query(
                "SELECT id FROM users WHERE icloudId=?",
                &[Integer(icloud_id)],
                &mut |row| {
                    id = row.integer(0)?;
                    Ok(())
                },
            )?;
            Ok(User {
                id: required(id)?,
                icloud_id,
            })
        })
    }

    pub fn user(&self, icloud_id: i64) -> Result<Option<User>> {
        let mut user = None;
        self.db.query(
            "SELECT id FROM users WHERE icloudId=?",
            &[Integer(icloud_id)],
            &mut |row| {
                user = Some(User {
                    id: required(row.integer(0)?)?,
                    icloud_id,
                });
                Ok(())
            },
        )?;
        Ok(user)
    }

    pub fn playlists(&self) -> Result<Vec<Playlist>> {
        let mut lists = Vec::new();
        self.db.query(
            &format!("SELECT {LIST_COLUMNS} FROM playlists ORDER BY id"),
            &[],
            &mut |row| {
                lists.push(read_playlist(row)?);
                Ok(())
            },
        )?;
        Ok(lists)
    }

    pub fn create_playlist(&self, name: &str, now: f64) -> Result<Playlist> {
        create_playlist(&self.db, name, now)
    }

    pub fn create_playlist_with_songs(
        &mut self,
        name: &str,
        song_ids: &[i64],
        now: f64,
    ) -> Result<Playlist> {
        self.db.transaction(|db| {
            let list = create_playlist(db, name, now)?;
            add_playlist_songs(db, list.id, song_ids)?;
            Ok(list)
        })
    }

    pub fn rename_playlist(&self, id: i64, name: &str, now: f64) -> Result<Playlist> {
        self.db.execute("UPDATE playlists SET name=?,updatedAt=strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch') WHERE id=?", &[Text(name), Real(now), Integer(id)])?;
        required(playlist(&self.db, id)?)
    }

    pub fn delete_playlist(&mut self, id: i64) -> Result<()> {
        self.db.transaction(|db| {
            db.execute(
                "DELETE FROM playlist_songs WHERE playlistId=?",
                &[Integer(id)],
            )?;
            db.execute("DELETE FROM playlists WHERE id=?", &[Integer(id)])?;
            Ok(())
        })
    }

    pub fn playlist_songs(&self, id: i64) -> Result<Vec<Song>> {
        let mut songs = Vec::new();
        self.db.query(&format!("SELECT {SONG_COLUMNS} FROM playlist_songs p JOIN songs s ON s.id=p.songId WHERE p.playlistId=? ORDER BY p.position,p.id"), &[Integer(id)], &mut |row| { songs.push(read_song(row)?); Ok(()) })?;
        Ok(songs)
    }

    pub fn add_playlist_songs(&mut self, list_id: i64, song_ids: &[i64]) -> Result<()> {
        self.db.transaction(|db| {
            require_row(
                db,
                "SELECT 1 FROM playlists WHERE id=?",
                list_id,
                "This playlist was removed",
            )?;
            add_playlist_songs(db, list_id, song_ids)
        })
    }

    pub fn remove_playlist_songs(&mut self, list_id: i64, song_ids: &[i64]) -> Result<()> {
        self.db.transaction(|db| {
            for id in song_ids {
                db.execute(
                    "DELETE FROM playlist_songs WHERE playlistId=? AND songId=?",
                    &[Integer(list_id), Integer(*id)],
                )?;
            }
            Ok(())
        })
    }

    pub fn reorder_playlist(&mut self, list_id: i64, order: &[i64]) -> Result<()> {
        let mut seen = HashSet::with_capacity(order.len());
        if order.iter().any(|id| !seen.insert(*id)) {
            return Err(Error::invalid("Playlist order contains duplicate songs"));
        }
        self.db.transaction(|db| {
            require_row(
                db,
                "SELECT 1 FROM playlists WHERE id=?",
                list_id,
                "This playlist was removed",
            )?;
            // A partial order removes omitted membership, matching Swift's list-edit operation.
            db.execute(
                "UPDATE playlist_songs SET position=-1 WHERE playlistId=?",
                &[Integer(list_id)],
            )?;
            for (position, song_id) in order.iter().enumerate() {
                let changed = db.execute(
                    "UPDATE playlist_songs SET position=? WHERE playlistId=? AND songId=?",
                    &[
                        Integer(position as i64),
                        Integer(list_id),
                        Integer(*song_id),
                    ],
                )?;
                if changed != 1 {
                    return Err(Error::invalid(
                        "Playlist membership changed; refresh and try again",
                    ));
                }
            }
            db.execute(
                "DELETE FROM playlist_songs WHERE playlistId=? AND position=-1",
                &[Integer(list_id)],
            )?;
            Ok(())
        })
    }

    pub fn sources(&self, user_id: i64) -> Result<Vec<Source>> {
        let mut sources = Vec::new();
        self.db.query(
            &format!("SELECT {SOURCE_COLUMNS} FROM sources WHERE userId=? ORDER BY id"),
            &[Integer(user_id)],
            &mut |row| {
                sources.push(read_source(row)?);
                Ok(())
            },
        )?;
        Ok(sources)
    }

    pub fn source(&self, id: i64) -> Result<Option<Source>> {
        source(&self.db, id)
    }

    pub fn create_source(&self, source: &Source) -> Result<Source> {
        create_source(&self.db, source)
    }

    /// A validated folder grant and its current selection commit together, including re-selection.
    pub fn register_source_folder(
        &mut self,
        user_id: i64,
        directory: &str,
        path_id: i64,
        bookmark: &[u8],
        now: f64,
    ) -> Result<Source> {
        if !now.is_finite() || directory.is_empty() || bookmark.is_empty() {
            return Err(Error::invalid("Invalid source registration"));
        }
        self.db.transaction(|db| {
            require_row(db, "SELECT 1 FROM users WHERE id=?", user_id, "This account was removed")?;
            let mut existing = None;
            db.query("SELECT id FROM sources WHERE userId=? AND dirPath=? ORDER BY id LIMIT 1", &[Integer(user_id), Text(directory)], &mut |row| {
                existing = row.integer(0)?;
                Ok(())
            })?;
            let id = if let Some(id) = existing {
                db.execute("UPDATE sources SET bookmark=?,syncError=NULL,updatedAt=strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch') WHERE id=?", &[Blob(bookmark), Real(now), Integer(id)])?;
                id
            } else {
                required(create_source(db, &Source {
                    user_id,
                    dir_path: directory.into(),
                    path_id,
                    source_type: Some("iCloud".into()),
                    bookmark: Some(bookmark.to_vec()),
                    created_at: now,
                    ..Source::default()
                })?.id)?
            };
            select_source(db, user_id, id)
        })
    }

    pub fn update_source(&self, source: &Source) -> Result<Source> {
        let id = required(source.id)?;
        self.db.execute("UPDATE sources SET dirPath=?,pathId=?,type=?,totalPaths=?,syncError=?,isCurrent=?,lastSyncedAt=strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch'),updatedAt=strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch') WHERE id=?", &[
            Text(&source.dir_path), Integer(source.path_id), source.source_type.as_deref().map_or(Null, Text), source.total_paths.map_or(Null, Integer),
            source.sync_error.as_deref().map_or(Null, Text), Integer(source.is_current as i64), source.last_synced_at.map_or(Null, Real), source.updated_at.map_or(Null, Real), Integer(id)
        ])?;
        required(self.source(id)?)
    }

    pub fn set_current_source(&mut self, user_id: i64, source_id: i64) -> Result<Source> {
        self.db
            .transaction(|db| select_source(db, user_id, source_id))
    }

    pub fn update_source_bookmark(
        &self,
        id: i64,
        bookmark: &[u8],
        expected: Option<&[u8]>,
    ) -> Result<bool> {
        Ok(self.db.execute(
            "UPDATE sources SET bookmark=? WHERE id=? AND bookmark IS ?",
            &[Blob(bookmark), Integer(id), expected.map_or(Null, Blob)],
        )? > 0)
    }

    pub fn reconnect_source(
        &self,
        id: i64,
        root: &str,
        path_id: i64,
        bookmark: &[u8],
        now: f64,
    ) -> Result<Option<Source>> {
        self.db.execute("UPDATE sources SET dirPath=?,pathId=?,bookmark=?,syncError=NULL,updatedAt=strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch') WHERE id=?", &[Text(root), Integer(path_id), Blob(bookmark), Real(now), Integer(id)])?;
        self.source(id)
    }

    pub fn update_resolved_source_root(
        &self,
        id: i64,
        root: &str,
        path_id: i64,
        bookmark: &[u8],
        expected: Option<&[u8]>,
        now: f64,
    ) -> Result<bool> {
        Ok(self.db.execute("UPDATE sources SET dirPath=?,pathId=?,bookmark=?,updatedAt=strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch') WHERE id=? AND bookmark IS ?", &[Text(root), Integer(path_id), Blob(bookmark), Real(now), Integer(id), expected.map_or(Null, Blob)])? > 0)
    }

    pub fn delete_source(&mut self, id: i64) -> Result<()> {
        self.song_fts.invalidate();
        self.db.transaction(|db| delete_source_rows(db, id))
    }

    /// Migrate legacy identities and remove only the source version inspected by the worker.
    pub fn delete_source_with_locations(
        &mut self,
        expected: &Source,
        locations: &[(i64, String)],
    ) -> Result<()> {
        let id = required(expected.id)?;
        let hashes = legacy_locations(locations)?;
        self.song_fts.invalidate();
        self.db.transaction(|db| {
            require_source_version(db, expected)?;
            migrate_locations(db, id, &hashes)?;
            delete_source_rows(db, id)
        })
    }

    /// Commit a resolved root/bookmark and its legacy song locations together.
    pub fn save_source_access(
        &mut self,
        expected: &Source,
        resolved: &Source,
        locations: &[(i64, String)],
        now: f64,
    ) -> Result<Source> {
        let id = required(expected.id)?;
        let bookmark = required(resolved.bookmark.as_deref())?;
        if resolved.id != expected.id
            || resolved.user_id != expected.user_id
            || !now.is_finite()
            || !resolved.dir_path.starts_with('/') && !resolved.dir_path.starts_with("file:")
        {
            return Err(Error::invalid("Invalid resolved music source"));
        }
        let hashes = legacy_locations(locations)?;
        self.db.transaction(|db| {
            let current = require_source_version(db, expected)?;
            migrate_locations(db, id, &hashes)?;
            let moved = current.dir_path != resolved.dir_path || current.path_id != resolved.path_id;
            let stamp = if moved || current.updated_at != resolved.updated_at { Some(now.max(current.updated_at.unwrap_or(current.created_at) + 0.001)) } else { current.updated_at };
            db.execute("UPDATE sources SET dirPath=?,pathId=?,bookmark=?,syncError=?,updatedAt=strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch') WHERE id=?", &[Text(&resolved.dir_path), Integer(resolved.path_id), Blob(bookmark), resolved.sync_error.as_deref().map_or(Null, Text), stamp.map_or(Null, Real), Integer(id)])?;
            required(source(db, id)?)
        })
    }

    pub fn source_paths(&self, source_id: i64, parent: Option<i64>) -> Result<Vec<SourcePath>> {
        let mut paths = Vec::new();
        let query = if parent.is_some() {
            format!(
                "SELECT {PATH_COLUMNS} FROM source_paths WHERE sourceId=? AND parentPathId=? ORDER BY id"
            )
        } else {
            format!("SELECT {PATH_COLUMNS} FROM source_paths WHERE sourceId=? ORDER BY id")
        };
        let mut parameters = vec![Integer(source_id)];
        if let Some(parent) = parent {
            parameters.push(Integer(parent));
        }
        self.db.query(&query, &parameters, &mut |row| {
            paths.push(read_path(row)?);
            Ok(())
        })?;
        Ok(paths)
    }

    pub fn source_path(&self, source_id: i64, relative_path: &str) -> Result<Option<SourcePath>> {
        path(&self.db, source_id, relative_path)
    }

    pub fn source_path_by_id(&self, source_id: i64, path_id: i64) -> Result<Option<SourcePath>> {
        let mut found = None;
        self.db.query(
            &format!(
                "SELECT {PATH_COLUMNS} FROM source_paths WHERE sourceId=? AND pathId=? LIMIT 1"
            ),
            &[Integer(source_id), Integer(path_id)],
            &mut |row| {
                found = Some(read_path(row)?);
                Ok(())
            },
        )?;
        Ok(found)
    }

    /// Gather selected folders in one recursive indexed query per batch, with no Rust recursion.
    pub fn import_files(&self, selected: &[SourcePath]) -> Result<Vec<SourcePath>> {
        let mut files = Vec::new();
        let mut seen = HashSet::new();
        for batch in selected.chunks(200) {
            let values = vec!["(?,?)"; batch.len()].join(",");
            let parameters: Vec<_> = batch
                .iter()
                .flat_map(|path| [Integer(path.source_id), Integer(path.path_id)])
                .collect();
            self.db.query(&format!("WITH RECURSIVE selected(sourceId,pathId) AS (VALUES {values}), tree(id,sourceId,pathId,isDirectory) AS (SELECT p.id,p.sourceId,p.pathId,p.isDirectory FROM source_paths p JOIN selected s ON s.sourceId=p.sourceId AND s.pathId=p.pathId UNION SELECT p.id,p.sourceId,p.pathId,p.isDirectory FROM source_paths p JOIN tree t ON p.sourceId=t.sourceId AND p.parentPathId=t.pathId WHERE t.isDirectory=1) SELECT {PATH_COLUMNS} FROM source_paths WHERE id IN (SELECT id FROM tree WHERE isDirectory=0) ORDER BY id"), &parameters, &mut |row| {
                let path = read_path(row)?;
                if path.is_audio() && seen.insert(required(path.id)?) { files.push(path); }
                Ok(())
            })?;
        }
        files.sort_unstable_by_key(|path| path.id);
        Ok(files)
    }

    /// Commit song metadata/search and its content digest together, including stale-path protection.
    pub fn upsert_imported_song(
        &mut self,
        path: &SourcePath,
        song: &Song,
        digest: &[u8; 32],
        now: f64,
    ) -> Result<Song> {
        if !path.is_audio()
            || song.source_id != Some(path.source_id)
            || song.relative_path.as_deref() != Some(&path.relative_path)
        {
            return Err(Error::invalid(
                "Imported song does not belong to its source path",
            ));
        }
        let result = self.db.transaction(|db| {
            let saved = crate::repository::store_song(db, song, now, &mut self.song_fts)?;
            if db.execute("UPDATE source_paths SET fileHashSHA256=? WHERE sourceId=? AND pathId=? AND relativePath=? AND isDirectory=0", &[Blob(digest), Integer(path.source_id), Integer(path.path_id), Text(&path.relative_path)])? != 1 {
                return Err(Error::invalid("This source path changed during import; scan again"));
            }
            Ok(saved)
        });
        if result.is_err() {
            self.song_fts.invalidate();
        }
        result
    }

    pub fn update_source_path_hash(
        &self,
        source_id: i64,
        path_id: i64,
        hash: Option<&[u8]>,
    ) -> Result<()> {
        self.db.execute(
            "UPDATE source_paths SET fileHashSHA256=? WHERE sourceId=? AND pathId=?",
            &[
                hash.map_or(Null, Blob),
                Integer(source_id),
                Integer(path_id),
            ],
        )?;
        Ok(())
    }

    pub fn clear_source_paths(&mut self, source_id: i64) -> Result<()> {
        self.db.transaction(|db| {
            db.execute(
                "DELETE FROM source_paths_fts WHERE sourceId=?",
                &[Integer(source_id)],
            )?;
            db.execute(
                "DELETE FROM source_paths WHERE sourceId=?",
                &[Integer(source_id)],
            )?;
            Ok(())
        })
    }

    pub fn upsert_source_paths(&mut self, paths: &[SourcePath]) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }
        self.db.transaction(|db| store_paths(db, paths))
    }

    pub fn finish_source_scan(&mut self, source_id: i64, run_id: i64) -> Result<usize> {
        self.db.transaction(|db| {
            require_source(db, source_id)?;
            db.execute(
                "DELETE FROM source_paths_fts WHERE sourceId=? AND runId<>?",
                &[Integer(source_id), Integer(run_id)],
            )?;
            db.execute(
                "DELETE FROM source_paths WHERE sourceId=? AND runId<>?",
                &[Integer(source_id), Integer(run_id)],
            )
        })
    }

    /// Publish only a complete snapshot; preserve the previous index on failed or stale scans.
    pub fn replace_source_scan(
        &mut self,
        expected: &Source,
        mut paths: Vec<SourcePath>,
        now: f64,
    ) -> Result<Source> {
        let id = required(expected.id)?;
        let mut seen = HashSet::with_capacity(paths.len());
        if !now.is_finite()
            || paths
                .iter()
                .any(|path| path.source_id != id || !seen.insert(path.relative_path.as_str()))
        {
            return Err(Error::invalid("Invalid or duplicate source scan records"));
        }
        self.db.transaction(|db| {
            let current = required(source(db, id)?)?;
            if current.dir_path != expected.dir_path || current.path_id != expected.path_id || current.bookmark != expected.bookmark || current.updated_at != expected.updated_at || current.last_synced_at != expected.last_synced_at {
                return Err(Error::invalid("This music source changed during its scan; try again"));
            }
            let mut run = 0i64;
            db.query("SELECT COALESCE(MAX(runId),0) FROM source_paths WHERE sourceId=?", &[Integer(id)], &mut |row| { run = required(row.integer(0)?)?; Ok(()) })?;
            let run = run.checked_add(1).ok_or_else(|| Error::invalid("Source scan identity exhausted"))?;
            let stamp = now.max(current.updated_at.unwrap_or(current.created_at) + 0.001);
            for path in &mut paths {
                path.run_id = run;
                path.created_at = stamp;
                path.updated_at = Some(stamp);
            }
            store_paths(db, &paths)?;
            db.execute("DELETE FROM source_paths_fts WHERE sourceId=? AND runId<>?", &[Integer(id), Integer(run)])?;
            db.execute("DELETE FROM source_paths WHERE sourceId=? AND runId<>?", &[Integer(id), Integer(run)])?;
            let count = paths.iter().filter(|path| path.is_audio()).count();
            db.execute("UPDATE sources SET totalPaths=?,syncError=NULL,lastSyncedAt=strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch'),updatedAt=strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch') WHERE id=?", &[Integer(count as i64), Real(stamp), Real(stamp), Integer(id)])?;
            required(source(db, id)?)
        })
    }

    pub fn search_source_paths(
        &self,
        source_id: i64,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(i64, f64)>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let query = prefix_query(query);
        let limit =
            i64::try_from(limit).map_err(|_| Error::invalid("Search limit is too large"))?;
        let mut found = Vec::new();
        self.db.query("SELECT pathId,bm25(source_paths_fts) FROM source_paths_fts WHERE source_paths_fts MATCH ? AND sourceId=? ORDER BY bm25(source_paths_fts) LIMIT ?", &[Text(&query), Integer(source_id), Integer(limit)], &mut |row| { found.push((required(row.integer(0)?)?, required(row.real(1)?)?)); Ok(()) })?;
        Ok(found)
    }
}

fn store_paths(db: &Connection, paths: &[SourcePath]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    if paths
        .iter()
        .any(|path| !crate::models::valid_relative_path(&path.relative_path))
    {
        return Err(Error::invalid("Invalid source-relative file path"));
    }
    let sources: HashSet<_> = paths.iter().map(|path| path.source_id).collect();
    for id in &sources {
        require_source(db, *id)?;
    }
    // UNINDEXED FTS identifiers otherwise make a full rescan quadratic.
    let mut fts_rows: HashMap<(i64, i64), Vec<i64>> = HashMap::new();
    db.query(
        "SELECT sourceId,pathId,rowid FROM source_paths_fts",
        &[],
        &mut |row| {
            let source = required(row.integer(0)?)?;
            if sources.contains(&source) {
                fts_rows
                    .entry((source, required(row.integer(1)?)?))
                    .or_default()
                    .push(required(row.integer(2)?)?);
            }
            Ok(())
        },
    )?;
    for next in paths {
        let existing = path(db, next.source_id, &next.relative_path)?;
        let old_id = existing.as_ref().map_or(next.path_id, |old| old.path_id);
        for key in [(next.source_id, old_id), (next.source_id, next.path_id)] {
            if let Some(rows) = fts_rows.remove(&key) {
                for row_id in rows {
                    db.execute(
                        "DELETE FROM source_paths_fts WHERE rowid=?",
                        &[Integer(row_id)],
                    )?;
                }
            }
        }
        if let Some(existing) = existing {
            db.execute("UPDATE source_paths SET pathId=?,parentPathId=?,name=?,isDirectory=?,fileSize=COALESCE(?,fileSize),fileHashSHA256=COALESCE(?,fileHashSHA256),runId=?,updatedAt=strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch') WHERE id=?", &[
                        Integer(next.path_id), next.parent_path_id.map_or(Null, Integer), Text(&next.name), Integer(next.is_directory as i64), next.file_size.map_or(Null, Integer),
                        next.file_hash_sha256.as_deref().map_or(Null, Blob), Integer(next.run_id), next.updated_at.map_or(Null, Real), Integer(required(existing.id)?)
                    ])?;
        } else {
            db.execute("INSERT INTO source_paths (sourceId,pathId,parentPathId,name,relativePath,isDirectory,fileSize,fileHashSHA256,runId,createdAt,updatedAt) VALUES (?,?,?,?,?,?,?,?,?,strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch'),strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch'))", &[
                        Integer(next.source_id), Integer(next.path_id), next.parent_path_id.map_or(Null, Integer), Text(&next.name), Text(&next.relative_path), Integer(next.is_directory as i64),
                        next.file_size.map_or(Null, Integer), next.file_hash_sha256.as_deref().map_or(Null, Blob), Integer(next.run_id), Real(next.created_at), next.updated_at.map_or(Null, Real)
                    ])?;
        }
        db.execute("INSERT INTO source_paths_fts (pathId,sourceId,runId,fullPath,fileName) VALUES (?,?,?,?,?)", &[Integer(next.path_id), Integer(next.source_id), Integer(next.run_id), Text(&next.relative_path), Text(&next.name)])?;
        fts_rows.insert((next.source_id, next.path_id), vec![db.last_insert_id()]);
    }
    Ok(())
}

fn create_playlist(db: &Connection, name: &str, now: f64) -> Result<Playlist> {
    db.execute("INSERT INTO playlists (name,createdAt) VALUES (?,strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch'))", &[Text(name), Real(now)])?;
    required(playlist(db, db.last_insert_id())?)
}

fn add_playlist_songs(db: &Connection, list_id: i64, song_ids: &[i64]) -> Result<()> {
    for id in song_ids {
        require_row(
            db,
            "SELECT 1 FROM songs WHERE id=?",
            *id,
            "This song was removed",
        )?;
        db.execute("INSERT INTO playlist_songs (playlistId,songId,position) SELECT ?,?,COALESCE(MAX(position),-1)+1 FROM playlist_songs WHERE playlistId=? ON CONFLICT(playlistId,songId) DO NOTHING", &[Integer(list_id), Integer(*id), Integer(list_id)])?;
    }
    Ok(())
}

fn playlist(db: &Connection, id: i64) -> Result<Option<Playlist>> {
    let mut list = None;
    db.query(
        &format!("SELECT {LIST_COLUMNS} FROM playlists WHERE id=?"),
        &[Integer(id)],
        &mut |row| {
            list = Some(read_playlist(row)?);
            Ok(())
        },
    )?;
    Ok(list)
}

fn read_playlist(row: &Row<'_>) -> Result<Playlist> {
    Ok(Playlist {
        id: required(row.integer(0)?)?,
        name: required(row.text(1)?)?.into(),
        created_at: required(row.real(2)?)?,
        updated_at: row.real(3)?,
    })
}

fn create_source(db: &Connection, source: &Source) -> Result<Source> {
    db.execute("INSERT INTO sources (dirPath,pathId,userId,type,bookmark,totalPaths,syncError,isCurrent,createdAt,lastSyncedAt,updatedAt) VALUES (?,?,?,?,?,?,?,?,strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch'),strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch'),strftime('%Y-%m-%dT%H:%M:%f',?,'unixepoch'))", &[
        Text(&source.dir_path), Integer(source.path_id), Integer(source.user_id), source.source_type.as_deref().map_or(Null, Text),
        source.bookmark.as_deref().map_or(Null, Blob), source.total_paths.map_or(Null, Integer), source.sync_error.as_deref().map_or(Null, Text),
        Integer(source.is_current as i64), Real(source.created_at), source.last_synced_at.map_or(Null, Real), source.updated_at.map_or(Null, Real)
    ])?;
    required(self::source(db, db.last_insert_id())?)
}

fn select_source(db: &Connection, user_id: i64, id: i64) -> Result<Source> {
    let chosen = required(source(db, id)?)?;
    if chosen.user_id != user_id {
        return Err(Error::invalid("This source belongs to another user"));
    }
    db.execute(
        "UPDATE sources SET isCurrent=0 WHERE userId=?",
        &[Integer(user_id)],
    )?;
    db.execute("UPDATE sources SET isCurrent=1 WHERE id=?", &[Integer(id)])?;
    required(source(db, id)?)
}

fn delete_source_rows(db: &Connection, id: i64) -> Result<()> {
    db.execute(
        "DELETE FROM playlist_songs WHERE songId IN (SELECT id FROM songs WHERE sourceId=?)",
        &[Integer(id)],
    )?;
    db.execute(
        "DELETE FROM songs_fts WHERE songId IN (SELECT id FROM songs WHERE sourceId=?)",
        &[Integer(id)],
    )?;
    db.execute("DELETE FROM songs WHERE sourceId=?", &[Integer(id)])?;
    db.execute(
        "DELETE FROM source_paths_fts WHERE sourceId=?",
        &[Integer(id)],
    )?;
    db.execute("DELETE FROM source_paths WHERE sourceId=?", &[Integer(id)])?;
    db.execute("DELETE FROM sources WHERE id=?", &[Integer(id)])?;
    Ok(())
}

fn require_source_version(db: &Connection, expected: &Source) -> Result<Source> {
    let current = required(source(db, required(expected.id)?)?)?;
    if current.user_id != expected.user_id
        || current.dir_path != expected.dir_path
        || current.path_id != expected.path_id
        || current.bookmark != expected.bookmark
        || current.updated_at != expected.updated_at
        || current.sync_error != expected.sync_error
    {
        return Err(Error::invalid(
            "This music source changed. Reload folders and try again.",
        ));
    }
    Ok(current)
}

fn legacy_locations(locations: &[(i64, String)]) -> Result<HashMap<i64, &str>> {
    let mut hashes = HashMap::with_capacity(locations.len());
    for (hash, path) in locations {
        if !crate::models::valid_relative_path(path)
            || hashes
                .insert(*hash, path.as_str())
                .is_some_and(|previous| previous != path)
        {
            return Err(Error::invalid("Ambiguous or invalid legacy song location"));
        }
    }
    Ok(hashes)
}

fn migrate_locations(db: &Connection, id: i64, hashes: &HashMap<i64, &str>) -> Result<()> {
    for (hash, path) in hashes {
        db.execute(
            "UPDATE songs SET sourceId=?,relativePath=? WHERE sourceId IS NULL AND pathHash=?",
            &[Integer(id), Text(path), Integer(*hash)],
        )?;
    }
    Ok(())
}

fn source(db: &Connection, id: i64) -> Result<Option<Source>> {
    let mut found = None;
    db.query(
        &format!("SELECT {SOURCE_COLUMNS} FROM sources WHERE id=?"),
        &[Integer(id)],
        &mut |row| {
            found = Some(read_source(row)?);
            Ok(())
        },
    )?;
    Ok(found)
}

fn read_source(row: &Row<'_>) -> Result<Source> {
    Ok(Source {
        id: row.integer(0)?,
        dir_path: required(row.text(1)?)?.into(),
        path_id: required(row.integer(2)?)?,
        user_id: required(row.integer(3)?)?,
        source_type: row.text(4)?.map(str::to_owned),
        bookmark: row.blob(5)?.map(<[u8]>::to_vec),
        total_paths: row.integer(6)?,
        sync_error: row.text(7)?.map(str::to_owned),
        is_current: required(row.integer(8)?)? != 0,
        created_at: required(row.real(9)?)?,
        last_synced_at: row.real(10)?,
        updated_at: row.real(11)?,
    })
}

fn path(db: &Connection, source_id: i64, relative: &str) -> Result<Option<SourcePath>> {
    let mut found = None;
    db.query(
        &format!(
            "SELECT {PATH_COLUMNS} FROM source_paths WHERE sourceId=? AND relativePath=? LIMIT 1"
        ),
        &[Integer(source_id), Text(relative)],
        &mut |row| {
            found = Some(read_path(row)?);
            Ok(())
        },
    )?;
    Ok(found)
}

fn read_path(row: &Row<'_>) -> Result<SourcePath> {
    Ok(SourcePath {
        id: row.integer(0)?,
        source_id: required(row.integer(1)?)?,
        path_id: required(row.integer(2)?)?,
        parent_path_id: row.integer(3)?,
        name: required(row.text(4)?)?.into(),
        relative_path: required(row.text(5)?)?.into(),
        is_directory: required(row.integer(6)?)? != 0,
        file_size: row.integer(7)?,
        file_hash_sha256: row.blob(8)?.map(<[u8]>::to_vec),
        run_id: required(row.integer(9)?)?,
        created_at: required(row.real(10)?)?,
        updated_at: row.real(11)?,
    })
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
