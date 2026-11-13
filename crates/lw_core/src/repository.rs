use crate::{FileAvailability, FileState, Song};
use Value::{Blob, Integer, Null, Real, Text};
use lw_apple_sys::sqlite::{Connection, Error, Result, Row, Value};
use std::collections::HashMap;

pub(crate) const SONG_COLUMNS: &str = "s.id,s.songKey,s.artist,s.title,s.album,s.albumArtist,s.releaseYear,s.discNumber,s.trackNumber,s.coverArtPath,s.bookmark,s.pathHash,s.createdAt,s.updatedAt,s.localFilePath,s.fileState,s.isFavorite,s.sourceId,s.relativePath,s.duration,s.fileSize,s.fileExtension,s.sampleRate,s.bitDepth,s.sourceAvailability";
const ALBUM_QUERY: &str = "SELECT album,artist,coverArtPath,MIN(LENGTH(artist)) FROM songs WHERE album<>'' GROUP BY album ORDER BY album";

pub struct LibraryRepository {
    pub(crate) db: Connection,
    pub(crate) song_fts: SongSearchRows,
}

#[derive(Default)]
pub(crate) struct SongSearchRows {
    version: Option<i64>,
    rows: HashMap<i64, Vec<i64>>,
}

impl SongSearchRows {
    pub(crate) fn invalidate(&mut self) {
        self.version = None;
        self.rows.clear();
    }

    fn refresh(&mut self, db: &Connection) -> Result<()> {
        let mut version = None;
        db.query("PRAGMA data_version", &[], &mut |row| {
            version = Some(required(row.integer(0)?)?);
            Ok(())
        })?;
        if version == self.version {
            return Ok(());
        }
        self.invalidate();
        // songId is UNINDEXED in Swift's FTS schema; scan once, then address rows directly.
        db.query("SELECT songId,rowid FROM songs_fts", &[], &mut |row| {
            self.rows
                .entry(required(row.integer(0)?)?)
                .or_default()
                .push(required(row.integer(1)?)?);
            Ok(())
        })?;
        self.version = version;
        Ok(())
    }
}
impl LibraryRepository {
    pub fn data_version(&self) -> Result<i64> {
        let mut version = None;
        self.db.query("PRAGMA data_version", &[], &mut |row| {
            version = row.integer(0)?;
            Ok(())
        })?;
        required(version)
    }

    /// Find determines membership; its view keeps the library's original order.
    pub fn matching_song_ids(&self, query: &str) -> Result<Vec<i64>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let query = prefix_query(query);
        let mut ids = Vec::new();
        self.db.query("SELECT DISTINCT s.id FROM songs s JOIN songs_fts fts ON s.id=fts.songId WHERE songs_fts MATCH ?",&[Text(&query)],&mut |row| { ids.push(required(row.integer(0)?)?);Ok(()) })?;
        Ok(ids)
    }

    /// Borrow a consistent catalog across song, playlist and membership queries.
    pub fn read_snapshot<T>(&self, read: impl FnOnce(&Self) -> Result<T>) -> Result<T> {
        self.db.read_transaction(|| read(self))
    }

    pub fn open(path: &str) -> Result<Self> {
        Self::new(Connection::open(path)?)
    }
    pub fn new(mut db: Connection) -> Result<Self> {
        db.transaction(|db| {
            db.execute_batch(include_str!("songs.sql"))?;
            let mut columns = Vec::new();
            db.query("PRAGMA table_info(songs)", &[], &mut |row| {
                columns.push(required(row.text(1)?)?.to_owned());
                Ok(())
            })?;
            for (name, definition) in [
                ("localFilePath", "TEXT"),
                ("fileState", "INTEGER NOT NULL DEFAULT 0"),
                ("isFavorite", "INTEGER NOT NULL DEFAULT 0"),
                ("sourceId", "INTEGER"),
                ("relativePath", "TEXT"),
                ("duration", "REAL"),
                ("fileSize", "INTEGER"),
                ("fileExtension", "TEXT"),
                ("sampleRate", "REAL"),
                ("bitDepth", "INTEGER"),
                ("sourceAvailability", "TEXT"),
            ] {
                if !columns.iter().any(|column| column == name) {
                    db.execute_batch(&format!("ALTER TABLE songs ADD COLUMN {name} {definition}"))?;
                }
            }
            crate::catalog::migrate(db)?;
            db.execute_batch("CREATE INDEX IF NOT EXISTS lw_songs_key ON songs(songKey); CREATE INDEX IF NOT EXISTS lw_songs_source_path ON songs(sourceId,relativePath); CREATE INDEX IF NOT EXISTS lw_songs_path_hash ON songs(pathHash); CREATE INDEX IF NOT EXISTS lw_songs_created ON songs(createdAt DESC); CREATE INDEX IF NOT EXISTS lw_songs_artist ON songs(artist); CREATE INDEX IF NOT EXISTS lw_songs_album ON songs(album,artist)")?;
            Ok(())
        })?;
        Ok(Self {
            db,
            song_fts: SongSearchRows::default(),
        })
    }

    pub fn search_songs(&self, query: &str, limit: usize, offset: usize) -> Result<Vec<Song>> {
        let limit =
            i64::try_from(limit).map_err(|_| Error::invalid("Search limit is too large"))?;
        let offset =
            i64::try_from(offset).map_err(|_| Error::invalid("Search offset is too large"))?;
        let mut songs = Vec::new();
        if query.trim().is_empty() {
            self.db.query(
                &format!(
                    "SELECT {SONG_COLUMNS} FROM songs s ORDER BY s.createdAt DESC LIMIT ? OFFSET ?"
                ),
                &[Integer(limit), Integer(offset)],
                &mut |row| {
                    songs.push(read_song(row)?);
                    Ok(())
                },
            )?;
        } else {
            let query = prefix_query(query);
            self.db.query(&format!("SELECT {SONG_COLUMNS} FROM songs s JOIN songs_fts fts ON s.id=fts.songId WHERE songs_fts MATCH ? ORDER BY bm25(songs_fts) LIMIT ? OFFSET ?"),
                &[Text(&query), Integer(limit), Integer(offset)], &mut |row| { songs.push(read_song(row)?); Ok(()) })?;
        }
        Ok(songs)
    }

    pub fn get_song(&self, id: i64) -> Result<Option<Song>> {
        get_song(&self.db, id)
    }

    pub fn get_songs(&self, ids: &[i64]) -> Result<Vec<Song>> {
        let mut found = std::collections::HashMap::with_capacity(ids.len());
        for id in ids {
            found.entry(*id).or_insert((None, 0usize)).1 += 1;
        }
        // Stay below older system SQLite parameter limits without one round trip per song.
        for chunk in ids.chunks(250) {
            let placeholders = vec!["?"; chunk.len()].join(",");
            let parameters: Vec<_> = chunk.iter().copied().map(Integer).collect();
            self.db.query(
                &format!("SELECT {SONG_COLUMNS} FROM songs s WHERE s.id IN ({placeholders})"),
                &parameters,
                &mut |row| {
                    if let Some((song, _)) = found.get_mut(&required(row.integer(0)?)?)
                        && song.is_none()
                    {
                        *song = Some(read_song(row)?);
                    }
                    Ok(())
                },
            )?;
        }
        let mut songs = Vec::with_capacity(ids.len());
        for id in ids {
            let (song, remaining) = found.get_mut(id).expect("Requested ID was counted");
            *remaining -= 1;
            let song = if *remaining == 0 {
                song.take()
            } else {
                song.clone()
            };
            if let Some(song) = song {
                songs.push(song);
            }
        }
        Ok(songs)
    }

    pub fn upsert_song(&mut self, song: &Song, now: f64) -> Result<Song> {
        let result = self
            .db
            .transaction(|db| store_song(db, song, now, &mut self.song_fts));
        if result.is_err() {
            self.song_fts.invalidate();
        }
        result
    }

    pub fn set_favorite(&self, id: i64, favorite: bool) -> Result<()> {
        self.db.execute(
            "UPDATE songs SET isFavorite=? WHERE id=?",
            &[Integer(favorite as i64), Integer(id)],
        )?;
        Ok(())
    }

    pub fn song_count(&self, query: &str) -> Result<usize> {
        let mut count = 0;
        let mut visit = |row: &Row<'_>| {
            count = required(row.integer(0)?)? as usize;
            Ok(())
        };
        if query.trim().is_empty() {
            self.db
                .query("SELECT COUNT(*) FROM songs", &[], &mut visit)?;
        } else {
            self.db.query("SELECT COUNT(*) FROM songs s JOIN songs_fts f ON s.id=f.songId WHERE songs_fts MATCH ?", &[Text(&prefix_query(query))], &mut visit)?;
        }
        Ok(count)
    }

    pub fn artists(&self) -> Result<Vec<String>> {
        let mut artists = Vec::new();
        self.db.query(
            "SELECT DISTINCT artist FROM songs WHERE artist<>'' ORDER BY artist",
            &[],
            &mut |row| {
                artists.push(required(row.text(0)?)?.into());
                Ok(())
            },
        )?;
        Ok(artists)
    }

    pub fn albums(&self) -> Result<Vec<crate::Album>> {
        let mut albums = Vec::new();
        // SQLite's single MIN selects artist/artwork from the same shortest-artist row.
        self.db.query(ALBUM_QUERY, &[], &mut |row| {
            albums.push(crate::Album {
                name: required(row.text(0)?)?.into(),
                artist: required(row.text(1)?)?.into(),
                cover_art_path: row.text(2)?.map(str::to_owned),
            });
            Ok(())
        })?;
        Ok(albums)
    }

    pub fn songs_by_artist(&self, artist: &str) -> Result<Vec<Song>> {
        query_songs(&self.db, "WHERE s.artist=? ORDER BY s.id", &[Text(artist)])
    }

    pub fn songs_by_album(&self, album: &str, artist: Option<&str>) -> Result<Vec<Song>> {
        query_songs(
            &self.db,
            "WHERE s.album=? AND (? IS NULL OR s.artist=?) ORDER BY s.id",
            &[
                Text(album),
                artist.map_or(Null, Text),
                artist.map_or(Null, Text),
            ],
        )
    }

    pub fn song_at_source_path(&self, source_id: i64, path: &str) -> Result<Option<Song>> {
        Ok(query_songs(
            &self.db,
            "WHERE s.sourceId=? AND s.relativePath=? LIMIT 1",
            &[Integer(source_id), Text(path)],
        )?
        .pop())
    }

    pub fn song_with_path_hash(&self, path_hash: i64) -> Result<Option<Song>> {
        Ok(query_songs(
            &self.db,
            "WHERE s.pathHash=? LIMIT 1",
            &[Integer(path_hash)],
        )?
        .pop())
    }

    pub fn update_song_bookmark(&self, id: i64, bookmark: &[u8]) -> Result<()> {
        self.db.execute(
            "UPDATE songs SET bookmark=? WHERE id=?",
            &[Blob(bookmark), Integer(id)],
        )?;
        Ok(())
    }

    pub fn renew_song_bookmark(&self, id: i64, expected: &[u8], renewed: &[u8]) -> Result<bool> {
        Ok(self.db.execute(
            "UPDATE songs SET bookmark=? WHERE id=? AND bookmark=?",
            &[Blob(renewed), Integer(id), Blob(expected)],
        )? > 0)
    }

    pub fn update_song_availability(&self, id: i64, availability: FileAvailability) -> Result<()> {
        self.db.execute(
            "UPDATE songs SET sourceAvailability=? WHERE id=?",
            &[Text(availability.as_str()), Integer(id)],
        )?;
        Ok(())
    }

    pub fn update_song_source_location(
        &mut self,
        id: i64,
        source_id: i64,
        path: &str,
    ) -> Result<bool> {
        if !crate::models::valid_relative_path(path) {
            return Err(Error::invalid("Invalid source-relative file path"));
        }
        self.db.transaction(|db| {
            crate::catalog::require_source(db, source_id)?;
            Ok(db.execute("UPDATE songs SET sourceId=?,relativePath=? WHERE id=? AND (sourceId IS NULL OR (sourceId=? AND relativePath IS NULL))", &[Integer(source_id), Text(path), Integer(id), Integer(source_id)])? > 0)
        })
    }

    pub fn songs_needing_copy(&self) -> Result<Vec<Song>> {
        query_songs(&self.db, "WHERE s.fileState IN (1,3) ORDER BY s.id", &[])
    }

    pub fn cache_owners(&self) -> Result<Vec<crate::cache::Owner>> {
        let mut owners = Vec::new();
        self.db.query(
            "SELECT id,localFilePath,fileState FROM songs WHERE localFilePath IS NOT NULL",
            &[],
            &mut |row| {
                owners.push(crate::cache::Owner {
                    song_id: required(row.integer(0)?)?,
                    stored_path: required(row.text(1)?)?.to_owned(),
                    state: FileState::from_raw(required(row.integer(2)?)?),
                });
                Ok(())
            },
        )?;
        Ok(owners)
    }

    pub fn mark_song_for_copy(&self, id: i64) -> Result<()> {
        self.db
            .execute("UPDATE songs SET fileState=1 WHERE id=?", &[Integer(id)])?;
        Ok(())
    }

    pub fn delete_song(&mut self, id: i64) -> Result<()> {
        self.song_fts.invalidate();
        self.db.transaction(|db| {
            db.execute("DELETE FROM playlist_songs WHERE songId=?", &[Integer(id)])?;
            db.execute("DELETE FROM songs_fts WHERE songId=?", &[Integer(id)])?;
            db.execute("DELETE FROM songs WHERE id=?", &[Integer(id)])?;
            Ok(())
        })
    }

    pub fn delete_album(&mut self, album: &str, artist: Option<&str>) -> Result<()> {
        self.song_fts.invalidate();
        let parameters = [
            Text(album),
            artist.map_or(Null, Text),
            artist.map_or(Null, Text),
        ];
        self.db.transaction(|db| {
            db.execute("DELETE FROM playlist_songs WHERE songId IN (SELECT id FROM songs WHERE album=? AND (? IS NULL OR artist=?))", &parameters)?;
            db.execute("DELETE FROM songs_fts WHERE songId IN (SELECT id FROM songs WHERE album=? AND (? IS NULL OR artist=?))", &parameters)?;
            db.execute("DELETE FROM songs WHERE album=? AND (? IS NULL OR artist=?)", &parameters)?;
            Ok(())
        })
    }

    pub fn update_cached_file(
        &self,
        id: i64,
        path: Option<&str>,
        state: FileState,
        expected_state: FileState,
        expected_path: Option<&str>,
    ) -> Result<bool> {
        Ok(self.db.execute("UPDATE songs SET localFilePath=?,fileState=? WHERE id=? AND fileState=? AND localFilePath IS ?",
            &[path.map_or(Null, Text), Integer(state as i64), Integer(id), Integer(expected_state as i64), expected_path.map_or(Null, Text)])? > 0)
    }
}

pub(crate) fn store_song(
    db: &Connection,
    song: &Song,
    now: f64,
    fts: &mut SongSearchRows,
) -> Result<Song> {
    if song
        .relative_path
        .as_deref()
        .is_some_and(|path| !crate::models::valid_relative_path(path))
    {
        return Err(Error::invalid("Invalid source-relative file path"));
    }
    if let Some(source_id) = song.source_id {
        crate::catalog::require_source(db, source_id)?;
    }
    fts.refresh(db)?;
    let mut id = None;
    if let (Some(source), Some(path)) = (song.source_id, song.relative_path.as_deref()) {
        db.query(
            "SELECT id FROM songs WHERE sourceId=? AND relativePath=? LIMIT 1",
            &[Integer(source), Text(path)],
            &mut |row| {
                id = row.integer(0)?;
                Ok(())
            },
        )?;
    }
    if id.is_none() {
        db.query(
            "SELECT id FROM songs WHERE songKey=? LIMIT 1",
            &[Integer(song.song_key)],
            &mut |row| {
                id = row.integer(0)?;
                Ok(())
            },
        )?;
    }
    let values = song_values(song);
    let id = if let Some(id) = id {
        let mut parameters = [Null; 26];
        parameters[..24].copy_from_slice(&values);
        parameters[24..].copy_from_slice(&[Real(now), Integer(id)]);
        db.execute("UPDATE songs SET songKey=?1,artist=?2,title=?3,album=?4,albumArtist=?5,releaseYear=?6,discNumber=?7,trackNumber=?8,coverArtPath=?9,bookmark=?10,pathHash=?11,localFilePath=?14,fileState=?15,sourceId=COALESCE(?17,sourceId),relativePath=COALESCE(?18,relativePath),duration=COALESCE(?19,duration),fileSize=COALESCE(?20,fileSize),fileExtension=COALESCE(?21,fileExtension),sampleRate=COALESCE(?22,sampleRate),bitDepth=COALESCE(?23,bitDepth),sourceAvailability=COALESCE(?24,sourceAvailability),updatedAt=?25 WHERE id=?26", &parameters)?;
        id
    } else {
        db.execute("INSERT INTO songs (songKey,artist,title,album,albumArtist,releaseYear,discNumber,trackNumber,coverArtPath,bookmark,pathHash,createdAt,updatedAt,localFilePath,fileState,isFavorite,sourceId,relativePath,duration,fileSize,fileExtension,sampleRate,bitDepth,sourceAvailability) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)", &values)?;
        db.last_insert_id()
    };
    let rows = fts.rows.entry(id).or_default();
    if rows.is_empty() {
        db.execute(
            "INSERT INTO songs_fts (songId,artist,title,album,albumArtist) VALUES (?,?,?,?,?)",
            &[
                Integer(id),
                Text(&song.artist),
                Text(&song.title),
                Text(&song.album),
                Text(&song.album_artist),
            ],
        )?;
        rows.push(db.last_insert_id());
    } else {
        for &row_id in rows.iter() {
            db.execute(
                "UPDATE songs_fts SET artist=?,title=?,album=?,albumArtist=? WHERE rowid=? AND songId=?",
                &[
                    Text(&song.artist),
                    Text(&song.title),
                    Text(&song.album),
                    Text(&song.album_artist),
                    Integer(row_id),
                    Integer(id),
                ],
            )?;
        }
    }
    required(get_song(db, id)?)
}

fn query_songs(db: &Connection, clause: &str, parameters: &[Value<'_>]) -> Result<Vec<Song>> {
    let mut songs = Vec::new();
    db.query(
        &format!("SELECT {SONG_COLUMNS} FROM songs s {clause}"),
        parameters,
        &mut |row| {
            songs.push(read_song(row)?);
            Ok(())
        },
    )?;
    Ok(songs)
}

pub(crate) fn required<T>(value: Option<T>) -> Result<T> {
    value.ok_or_else(|| Error::invalid("Required database value is missing"))
}
fn get_song(db: &Connection, id: i64) -> Result<Option<Song>> {
    let mut song = None;
    db.query(
        &format!("SELECT {SONG_COLUMNS} FROM songs s WHERE s.id=?"),
        &[Integer(id)],
        &mut |row| {
            song = Some(read_song(row)?);
            Ok(())
        },
    )?;
    Ok(song)
}

fn song_values(song: &Song) -> [Value<'_>; 24] {
    [
        Integer(song.song_key),
        Text(&song.artist),
        Text(&song.title),
        Text(&song.album),
        Text(&song.album_artist),
        song.release_year.map_or(Null, Integer),
        song.disc_number.map_or(Null, Integer),
        song.track_number.map_or(Null, Integer),
        song.cover_art_path.as_deref().map_or(Null, Text),
        song.bookmark.as_deref().map_or(Null, Blob),
        Integer(song.path_hash),
        Real(song.created_at),
        song.updated_at.map_or(Null, Real),
        song.local_file_path.as_deref().map_or(Null, Text),
        Integer(song.file_state as i64),
        Integer(song.is_favorite),
        song.source_id.map_or(Null, Integer),
        song.relative_path.as_deref().map_or(Null, Text),
        song.duration.map_or(Null, Real),
        song.file_size.map_or(Null, Integer),
        song.file_extension.as_deref().map_or(Null, Text),
        song.sample_rate.map_or(Null, Real),
        song.bit_depth.map_or(Null, Integer),
        song.source_availability
            .map_or(Null, |value| Text(value.as_str())),
    ]
}

pub(crate) fn read_song(row: &Row<'_>) -> Result<Song> {
    Ok(Song {
        id: row.integer(0)?,
        song_key: required(row.integer(1)?)?,
        artist: required(row.text(2)?)?.into(),
        title: required(row.text(3)?)?.into(),
        album: required(row.text(4)?)?.into(),
        album_artist: required(row.text(5)?)?.into(),
        release_year: row.integer(6)?,
        disc_number: row.integer(7)?,
        track_number: row.integer(8)?,
        cover_art_path: row.text(9)?.map(str::to_owned),
        bookmark: row.blob(10)?.map(<[u8]>::to_vec),
        path_hash: required(row.integer(11)?)?,
        created_at: required(row.real(12)?)?,
        updated_at: row.real(13)?,
        local_file_path: row.text(14)?.map(str::to_owned),
        file_state: FileState::from_raw(required(row.integer(15)?)?),
        is_favorite: required(row.integer(16)?)?,
        source_id: row.integer(17)?,
        relative_path: row.text(18)?.map(str::to_owned),
        duration: row.real(19)?,
        file_size: row.integer(20)?,
        file_extension: row.text(21)?.map(str::to_owned),
        sample_rate: row.real(22)?,
        bit_depth: row.integer(23)?,
        source_availability: row.text(24)?.and_then(FileAvailability::from_raw),
    })
}

pub(crate) fn prefix_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|token| format!("\"{}\"*", token.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn song() -> Song {
        Song {
            song_key: 123,
            artist: "Artist".into(),
            title: "Title".into(),
            album: "Album".into(),
            album_artist: "Album Artist".into(),
            release_year: Some(2026),
            disc_number: Some(2),
            track_number: Some(3),
            bookmark: Some(vec![0, 255, 3]),
            path_hash: 123,
            source_id: Some(7),
            relative_path: Some("Album/track.flac".into()),
            duration: Some(90.5),
            file_size: Some(1000),
            file_extension: Some("flac".into()),
            sample_rate: Some(48000.0),
            bit_depth: Some(24),
            source_availability: Some(FileAvailability::OnDevice),
            ..Song::default()
        }
    }

    #[test]
    fn find_membership_deduplicates_legacy_fts_rows_and_ignores_orphans() {
        let mut repo = LibraryRepository::open(":memory:").unwrap();
        let song = repo
            .upsert_song(
                &Song {
                    song_key: 1,
                    title: "Blue sky".into(),
                    ..Song::default()
                },
                1.0,
            )
            .unwrap();
        let id = song.id.unwrap();
        repo.db.execute("INSERT INTO songs_fts(rowid,songId,title) VALUES (700,?,'Blue sky'),(701,900,'Blue sky')",&[Integer(id)]).unwrap();
        assert_eq!(repo.matching_song_ids("Blu").unwrap(), [id]);
        assert!(repo.matching_song_ids("  ").unwrap().is_empty());
        assert!(repo.matching_song_ids("missing").unwrap().is_empty());
        assert_eq!(
            repo.search_songs("Blu", 10, 0).unwrap().len(),
            2,
            "ranked repository search retains its existing wire behavior"
        );
    }

    #[test]
    fn catalog_read_snapshot_keeps_one_version_and_rolls_back_errors() {
        let path =
            std::env::temp_dir().join(format!("lw-read-snapshot-{}.sqlite", std::process::id()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                for suffix in ["", "-wal", "-shm"] {
                    let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
                }
            }
        }
        let _cleanup = Cleanup(path.clone());
        let repo = LibraryRepository::open(path.to_str().unwrap()).unwrap();
        // WAL permits a concurrent commit while the reader retains its earlier snapshot.
        repo.db.execute_batch("PRAGMA journal_mode=WAL").unwrap();
        let mut writer = LibraryRepository::open(path.to_str().unwrap()).unwrap();
        repo.read_snapshot(|snapshot| {
            assert_eq!(snapshot.song_count("")?, 0);
            let version = snapshot.data_version()?;
            writer.upsert_song(
                &Song {
                    song_key: 1,
                    title: "Concurrent insert".into(),
                    ..Song::default()
                },
                1.0,
            )?;
            assert_eq!(snapshot.song_count("")?, 0);
            assert_eq!(
                snapshot.data_version()?,
                version,
                "the version stamp belongs to the held read snapshot"
            );
            assert!(snapshot.search_songs("", 10, 0)?.is_empty());
            Ok(())
        })
        .unwrap();
        assert_eq!(repo.song_count("").unwrap(), 1);
        let failure: Result<()> = repo.read_snapshot(|snapshot| {
            assert_eq!(snapshot.song_count("")?, 1);
            assert!(snapshot.read_snapshot(|_| Ok(())).is_err());
            Err(Error::invalid("deliberate snapshot failure"))
        });
        assert!(failure.is_err());
        assert_eq!(
            repo.read_snapshot(|snapshot| snapshot.song_count(""))
                .unwrap(),
            1
        );
    }

    #[test]
    fn bulk_read_crosses_sql_parameter_batches_and_preserves_requested_occurrences() {
        let repo = LibraryRepository::open(":memory:").unwrap();
        repo.db.execute_batch("WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM seq WHERE n<1001) INSERT INTO songs (songKey,artist,title,album,albumArtist,pathHash,createdAt) SELECT n,'Artist','Track '||n,'Album','Artist',n,n FROM seq").unwrap();
        let mut requested: Vec<_> = (1..=1001).rev().collect();
        requested.extend([5000, 250, 1, 1001]);
        let found = repo.get_songs(&requested).unwrap();
        let expected: Vec<_> = requested.into_iter().filter(|id| *id != 5000).collect();
        assert_eq!(
            found
                .iter()
                .map(|song| song.id.unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(repo.get_songs(&[]).unwrap().is_empty());
        let unique: Vec<_> = (1..=1001).collect();
        mem_sys::local_reset();
        let songs = repo.get_songs(&unique).unwrap();
        let allocations = mem_sys::local_alloc_count();
        assert_eq!(songs.len(), unique.len());
        // Four nonempty strings per row, plus bounded query/map/output buffers; no second Song copy.
        assert!(
            allocations < 5 * unique.len() as u64,
            "{allocations} allocations for {} songs",
            unique.len()
        );
    }

    #[test]
    fn album_grouping_uses_one_scan_and_keeps_artist_and_artwork_together() {
        let repo = LibraryRepository::open(":memory:").unwrap();
        repo.db.execute_batch("INSERT INTO songs (songKey,artist,title,album,albumArtist,pathHash,createdAt,coverArtPath) VALUES (1,'A long artist','One','Shared','',1,0,'long.jpg'),(2,'短い','Two','Shared','',2,0,'short.jpg'),(3,'Solo','Three','Other','',3,0,'other.jpg'),(4,'Ignored','Four','','',4,0,'ignored.jpg')").unwrap();
        let albums = repo.albums().unwrap();
        assert_eq!(albums.len(), 2);
        assert_eq!(albums[0].name, "Other");
        assert_eq!(albums[1].name, "Shared");
        assert_eq!(albums[1].artist, "短い");
        assert_eq!(albums[1].cover_art_path.as_deref(), Some("short.jpg"));
        repo.db
            .query(
                &format!("EXPLAIN QUERY PLAN {ALBUM_QUERY}"),
                &[],
                &mut |row| {
                    assert!(!required(row.text(3)?)?.contains("CORRELATED"));
                    Ok(())
                },
            )
            .unwrap();
    }

    #[test]
    fn metadata_identity_favorites_fts_and_cache_compare_and_swap() {
        let mut repo = LibraryRepository::open(":memory:").unwrap();
        let source = repo
            .create_source(&crate::Source {
                dir_path: "/fixture".into(),
                user_id: 1,
                ..crate::Source::default()
            })
            .unwrap();
        let mut original = song();
        original.source_id = source.id;
        let saved = repo.upsert_song(&original, 1.0).unwrap();
        let id = saved.id.unwrap();
        repo.set_favorite(id, true).unwrap();
        let mut changed = original.clone();
        changed.song_key = 999;
        changed.title = "Changed tags".into();
        changed.duration = None;
        let updated = repo.upsert_song(&changed, 2.0).unwrap();
        assert_eq!(updated.id, saved.id);
        assert_eq!(updated.is_favorite, 1);
        assert_eq!(updated.duration, Some(90.5));
        assert_eq!(updated.bookmark, original.bookmark);
        assert_eq!(
            repo.search_songs("Chang Alb", 20, 0).unwrap().as_slice(),
            std::slice::from_ref(&updated)
        );
        assert!(repo.search_songs("Title", 20, 0).unwrap().is_empty());
        for query in ["\"", "OR", "*", "a:b", "a-b", "(tags)"] {
            repo.search_songs(query, 20, 0).unwrap();
        }
        assert_eq!(
            repo.get_songs(&[id, 123456, id]).unwrap(),
            [updated.clone(), updated]
        );
        assert!(
            repo.update_cached_file(
                id,
                Some("Music/cached.flac"),
                FileState::Copied,
                FileState::BookmarkOnly,
                None
            )
            .unwrap()
        );
        assert!(
            !repo
                .update_cached_file(id, None, FileState::Failed, FileState::BookmarkOnly, None)
                .unwrap()
        );
        assert_eq!(repo.get_song(id).unwrap().unwrap().title, "Changed tags");
        assert!(
            !repo
                .update_cached_file(
                    123456,
                    None,
                    FileState::Failed,
                    FileState::BookmarkOnly,
                    None
                )
                .unwrap()
        );
    }

    #[test]
    fn cached_fts_rows_follow_external_changes_legacy_rowids_and_missing_entries() {
        let root = std::env::temp_dir().join(format!(
            "lw-fts-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let path = root.join("library.sqlite");
        let mut repo = LibraryRepository::open(path.to_str().unwrap()).unwrap();
        let mut first = song();
        first.source_id = None;
        let first = repo.upsert_song(&first, 1.0).unwrap();
        let mut second = first.clone();
        second.song_key = 456;
        second.title = "Untouched".into();
        let second = repo.upsert_song(&second, 1.0).unwrap();
        let mut external = Connection::open(path.to_str().unwrap()).unwrap();
        external.transaction(|db| {
            db.execute_batch("DELETE FROM songs_fts")?;
            for (row_id, song) in [(1, &second), (-9, &first), (700, &first)] {
                db.execute("INSERT INTO songs_fts(rowid,songId,artist,title,album,albumArtist) VALUES (?,?,?,?,?,?)", &[
                    Integer(row_id), Integer(song.id.unwrap()), Text(&song.artist), Text(&song.title), Text(&song.album), Text(&song.album_artist),
                ])?;
            }
            Ok(())
        }).unwrap();
        let mut changed = first.clone();
        changed.title = "Refreshed".into();
        repo.upsert_song(&changed, 2.0).unwrap();
        let matches = repo.search_songs("Refreshed", 10, 0).unwrap();
        assert_eq!(matches.len(), 2); // Preserve both legacy FTS rows rather than losing one.
        assert!(matches.iter().all(|song| song.id == first.id));
        assert_eq!(repo.search_songs("Untouched", 10, 0).unwrap(), [second]);
        assert!(repo.search_songs("Title", 10, 0).unwrap().is_empty());
        external
            .execute(
                "DELETE FROM songs_fts WHERE songId=?",
                &[Integer(first.id.unwrap())],
            )
            .unwrap();
        changed.title = "Repaired".into();
        repo.upsert_song(&changed, 3.0).unwrap();
        assert_eq!(repo.search_songs("Repaired", 10, 0).unwrap().len(), 1);
        repo.delete_album("Album", None).unwrap();
        assert!(repo.song_fts.rows.is_empty());
        assert_eq!(repo.song_count("").unwrap(), 0);
    }

    #[test]
    fn upgrades_legacy_database_twice_without_losing_rows() {
        let db = Connection::open(":memory:").unwrap();
        db.execute_batch("CREATE TABLE songs (id INTEGER PRIMARY KEY AUTOINCREMENT,songKey INTEGER NOT NULL,artist TEXT NOT NULL,title TEXT NOT NULL,album TEXT NOT NULL,albumArtist TEXT NOT NULL,releaseYear INTEGER,discNumber INTEGER,trackNumber INTEGER,coverArtPath TEXT,bookmark BLOB,pathHash INTEGER NOT NULL,createdAt REAL NOT NULL,updatedAt REAL); INSERT INTO songs (songKey,artist,title,album,albumArtist,pathHash,createdAt) VALUES (99,'Existing','Keep me','Old library','Existing',99,0)").unwrap();
        let repo = LibraryRepository::new(db).unwrap();
        let repo = LibraryRepository::new(repo.db).unwrap();
        let old = repo.get_song(1).unwrap().unwrap();
        assert_eq!(old.title, "Keep me");
        assert_eq!(old.file_state, FileState::BookmarkOnly);
        assert_eq!(old.is_favorite, 0);
    }

    #[test]
    fn song_filters_pins_bookmarks_source_claims_and_deletion_preserve_other_rows() {
        let mut repo = LibraryRepository::open(":memory:").unwrap();
        let source = repo
            .create_source(&crate::Source {
                dir_path: "/fixture".into(),
                user_id: 1,
                ..crate::Source::default()
            })
            .unwrap();
        let source_id = source.id.unwrap();
        let mut first = song();
        first.source_id = None;
        let first = repo.upsert_song(&first, 1.0).unwrap();
        let first_id = first.id.unwrap();
        let mut second = first.clone();
        second.id = None;
        second.song_key = 456;
        second.artist = "Other Artist".into();
        second.title = "Another title".into();
        second.path_hash = 456;
        let second = repo.upsert_song(&second, 2.0).unwrap();
        let second_id = second.id.unwrap();
        assert_eq!(repo.song_count("").unwrap(), 2);
        assert_eq!(repo.song_count("Another").unwrap(), 1);
        assert_eq!(repo.artists().unwrap(), ["Artist", "Other Artist"]);
        assert_eq!(repo.albums().unwrap().len(), 1);
        assert_eq!(repo.songs_by_artist("Artist").unwrap().len(), 1);
        assert_eq!(repo.songs_by_album("Album", None).unwrap().len(), 2);
        assert_eq!(
            repo.songs_by_album("Album", Some("Other Artist"))
                .unwrap()
                .as_slice(),
            std::slice::from_ref(&second)
        );
        repo.mark_song_for_copy(first_id).unwrap();
        assert_eq!(repo.songs_needing_copy().unwrap().len(), 1);
        repo.update_song_bookmark(first_id, &[9]).unwrap();
        assert!(!repo.renew_song_bookmark(first_id, &[8], &[10]).unwrap());
        assert!(repo.renew_song_bookmark(first_id, &[9], &[10]).unwrap());
        assert!(!repo.renew_song_bookmark(first_id, &[9], &[11]).unwrap());
        repo.update_song_availability(first_id, FileAvailability::InCloud)
            .unwrap();
        assert!(
            repo.update_song_source_location(first_id, source_id, "Album/track.flac")
                .unwrap()
        );
        assert!(
            !repo
                .update_song_source_location(first_id, source_id, "Other.flac")
                .unwrap()
        );
        assert!(
            repo.update_song_source_location(second_id, source_id, "../escape.flac")
                .is_err()
        );
        let updated = repo
            .song_at_source_path(source_id, "Album/track.flac")
            .unwrap()
            .unwrap();
        assert_eq!(updated.bookmark, Some(vec![10]));
        assert_eq!(updated.source_availability, Some(FileAvailability::InCloud));
        assert_eq!(repo.song_with_path_hash(456).unwrap(), Some(second));
        let list = repo.create_playlist("Both", 1.0).unwrap();
        repo.add_playlist_songs(list.id, &[first_id, second_id])
            .unwrap();
        repo.delete_album("Album", Some("Artist")).unwrap();
        assert!(repo.get_song(first_id).unwrap().is_none());
        assert_eq!(repo.playlist_songs(list.id).unwrap().len(), 1);
        assert_eq!(repo.song_count("").unwrap(), 1);
        assert_eq!(repo.song_count("Title").unwrap(), 1);
        repo.delete_song(second_id).unwrap();
        assert!(repo.playlist_songs(list.id).unwrap().is_empty());
        assert_eq!(repo.song_count("Title").unwrap(), 0);
        assert!(
            !repo
                .update_cached_file(
                    second_id,
                    Some("Music/stale"),
                    FileState::Copied,
                    FileState::BookmarkOnly,
                    None
                )
                .unwrap()
        );
    }
}
