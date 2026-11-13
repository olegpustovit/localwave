use super::*;

#[test]
fn resolved_source_access_migrates_legacy_locations_atomically_and_rejects_conflicts() {
    let mut repo = LibraryRepository::open(":memory:").unwrap();
    let original = source(&repo, 1, "/original");
    let id = original.id.unwrap();
    let legacy = song(&mut repo, 10, None);
    let other = source(&repo, 1, "/other");
    let owned = song(&mut repo, 11, other.id);
    let list = repo.create_playlist("Keep", 100.0).unwrap();
    repo.add_playlist_songs(list.id, &[legacy.id.unwrap()])
        .unwrap();
    repo.set_favorite(legacy.id.unwrap(), true).unwrap();
    let resolved = Source {
        dir_path: "/moved".into(),
        path_id: 22,
        bookmark: Some(vec![9]),
        ..original.clone()
    };
    let locations = vec![(legacy.path_hash, "track.flac".to_owned())];
    repo.db.execute_batch("CREATE TRIGGER fail_root BEFORE UPDATE ON sources BEGIN SELECT RAISE(FAIL,'injected root failure'); END").unwrap();
    assert!(
        repo.save_source_access(&original, &resolved, &locations, 200.0)
            .is_err()
    );
    assert_eq!(
        repo.get_song(legacy.id.unwrap())
            .unwrap()
            .unwrap()
            .source_id,
        None
    );
    assert_eq!(repo.source(id).unwrap(), Some(original.clone()));
    repo.db.execute_batch("DROP TRIGGER fail_root").unwrap();
    let saved = repo
        .save_source_access(&original, &resolved, &locations, 200.0)
        .unwrap();
    assert_eq!(saved.dir_path, "/moved");
    let migrated = repo.get_song(legacy.id.unwrap()).unwrap().unwrap();
    assert_eq!(migrated.source_id, Some(id));
    assert_eq!(migrated.relative_path.as_deref(), Some("track.flac"));
    assert_eq!(migrated.is_favorite, 1);
    assert_eq!(repo.playlist_songs(list.id).unwrap()[0].id, legacy.id);
    assert_eq!(
        repo.get_song(owned.id.unwrap()).unwrap().unwrap().source_id,
        other.id
    );
    assert!(
        repo.save_source_access(&original, &resolved, &[], 201.0)
            .is_err()
    );
    let conflict = vec![(33, "one.flac".into()), (33, "two.flac".into())];
    assert!(
        repo.save_source_access(&saved, &saved, &conflict, 201.0)
            .is_err()
    );
}

#[test]
fn complete_scan_publication_preserves_metadata_and_rejects_stale_or_failed_snapshots() {
    let mut repo = LibraryRepository::open(":memory:").unwrap();
    let original = source(&repo, 1, "/scan");
    let id = original.id.unwrap();
    let path = SourcePath {
        source_id: id,
        path_id: 2,
        parent_path_id: Some(original.path_id),
        name: "track.flac".into(),
        relative_path: "track.flac".into(),
        file_size: Some(10),
        file_hash_sha256: Some(vec![1; 32]),
        ..SourcePath::default()
    };
    let note = SourcePath {
        path_id: 3,
        name: "notes.txt".into(),
        relative_path: "notes.txt".into(),
        file_hash_sha256: None,
        ..path.clone()
    };
    let first = repo
        .replace_source_scan(&original, vec![path.clone(), note], 200.125)
        .unwrap();
    assert_eq!(first.total_paths, Some(1));
    assert_eq!(first.last_synced_at, Some(200.125));
    let saved = repo.source_path(id, &path.relative_path).unwrap().unwrap();
    let mut update = path.clone();
    update.path_id = 20;
    update.file_hash_sha256 = None;
    update.file_size = None;
    let second = repo
        .replace_source_scan(&first, vec![update.clone()], 200.125)
        .unwrap();
    assert!(second.updated_at > first.updated_at);
    let rescanned = repo.source_path(id, &path.relative_path).unwrap().unwrap();
    assert_eq!(rescanned.id, saved.id);
    assert_eq!(rescanned.created_at, saved.created_at);
    assert_eq!(rescanned.file_hash_sha256, saved.file_hash_sha256);
    assert_eq!(rescanned.file_size, saved.file_size);
    assert_eq!(repo.source_paths(id, None).unwrap().len(), 1);
    assert_eq!(repo.search_source_paths(id, "track", 10).unwrap().len(), 1);
    assert!(repo.replace_source_scan(&first, vec![], 201.0).is_err());
    repo.db.execute_batch("CREATE TRIGGER fail_scan BEFORE UPDATE ON sources BEGIN SELECT RAISE(FAIL,'injected source failure'); END").unwrap();
    assert!(repo.replace_source_scan(&second, vec![], 202.0).is_err());
    assert_eq!(repo.source_paths(id, None).unwrap(), [rescanned]);
    assert_eq!(repo.search_source_paths(id, "track", 10).unwrap().len(), 1);
    repo.db.execute_batch("DROP TRIGGER fail_scan").unwrap();
    assert!(
        repo.replace_source_scan(&second, vec![update.clone(), update], 203.0)
            .is_err()
    );
    let empty = repo.replace_source_scan(&second, vec![], 204.0).unwrap();
    assert_eq!(empty.total_paths, Some(0));
    assert!(repo.source_paths(id, None).unwrap().is_empty());
    assert!(
        repo.search_source_paths(id, "track", 10)
            .unwrap()
            .is_empty()
    );
    repo.delete_source(id).unwrap();
    assert!(repo.replace_source_scan(&empty, vec![path], 205.0).is_err());
}

#[test]
fn recursive_import_gathers_deep_overlapping_selections_without_cycles_or_cross_source_rows() {
    let mut repo = LibraryRepository::open(":memory:").unwrap();
    let first = source(&repo, 1, "/first").id.unwrap();
    let second = source(&repo, 1, "/second").id.unwrap();
    let mut paths: Vec<_> = (1..=1300)
        .map(|id| SourcePath {
            source_id: first,
            path_id: id,
            parent_path_id: Some(if id == 1 { 1300 } else { id - 1 }),
            name: format!("Folder{id}"),
            relative_path: format!("Folder{id}"),
            is_directory: true,
            ..SourcePath::default()
        })
        .collect();
    paths.extend([
        SourcePath {
            source_id: first,
            path_id: 2000,
            parent_path_id: Some(1300),
            name: "song.FLAC".into(),
            relative_path: "song.FLAC".into(),
            ..SourcePath::default()
        },
        SourcePath {
            source_id: first,
            path_id: 2001,
            parent_path_id: Some(1300),
            name: "notes.txt".into(),
            relative_path: "notes.txt".into(),
            ..SourcePath::default()
        },
        SourcePath {
            source_id: second,
            path_id: 2000,
            parent_path_id: Some(1300),
            name: "other.flac".into(),
            relative_path: "other.flac".into(),
            ..SourcePath::default()
        },
    ]);
    repo.upsert_source_paths(&paths).unwrap();
    let mut selected = vec![paths[0].clone(); 201];
    selected.push(paths[1300].clone());
    let files = repo.import_files(&selected).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].source_id, first);
    assert_eq!(files[0].name, "song.FLAC");
    assert!(
        repo.import_files(&[paths[1301].clone()])
            .unwrap()
            .is_empty()
    );
    assert!(repo.import_files(&[]).unwrap().is_empty());
}

#[test]
fn imported_song_and_digest_roll_back_together_on_stale_paths_and_write_failures() {
    let mut repo = LibraryRepository::open(":memory:").unwrap();
    let source_id = source(&repo, 1, "/source").id.unwrap();
    let path = SourcePath {
        source_id,
        path_id: 10,
        name: "track.flac".into(),
        relative_path: "track.flac".into(),
        ..SourcePath::default()
    };
    repo.upsert_source_paths(std::slice::from_ref(&path))
        .unwrap();
    let mut song = Song {
        song_key: 1,
        artist: "Artist".into(),
        title: "Imported".into(),
        album: "Album".into(),
        source_id: Some(source_id),
        relative_path: Some(path.relative_path.clone()),
        ..Song::default()
    };
    repo.db.execute_batch("CREATE TRIGGER fail_import BEFORE UPDATE ON source_paths BEGIN SELECT RAISE(FAIL,'injected digest error'); END").unwrap();
    assert!(
        repo.upsert_imported_song(&path, &song, &[1; 32], 100.0)
            .is_err()
    );
    assert_eq!(repo.song_count("").unwrap(), 0);
    assert!(repo.search_songs("Imported", 10, 0).unwrap().is_empty());
    assert!(
        repo.source_path(source_id, &path.relative_path)
            .unwrap()
            .unwrap()
            .file_hash_sha256
            .is_none()
    );
    repo.db.execute_batch("DROP TRIGGER fail_import").unwrap();
    let saved = repo
        .upsert_imported_song(&path, &song, &[1; 32], 100.0)
        .unwrap();
    repo.set_favorite(saved.id.unwrap(), true).unwrap();
    let list = repo.create_playlist("Preserve", 100.0).unwrap();
    repo.add_playlist_songs(list.id, &[saved.id.unwrap()])
        .unwrap();
    song.title = "Updated".into();
    song.song_key = 2;
    let updated = repo
        .upsert_imported_song(&path, &song, &[2; 32], 101.0)
        .unwrap();
    assert_eq!(updated.id, saved.id);
    assert_eq!(updated.is_favorite, 1);
    assert_eq!(repo.playlist_songs(list.id).unwrap()[0].title, "Updated");
    assert_eq!(
        repo.source_path(source_id, &path.relative_path)
            .unwrap()
            .unwrap()
            .file_hash_sha256,
        Some(vec![2; 32])
    );
    repo.clear_source_paths(source_id).unwrap();
    song.title = "Stale write".into();
    assert!(
        repo.upsert_imported_song(&path, &song, &[3; 32], 102.0)
            .is_err()
    );
    assert_eq!(
        repo.get_song(saved.id.unwrap()).unwrap().unwrap().title,
        "Updated"
    );
    assert!(repo.search_songs("Stale", 10, 0).unwrap().is_empty());
}

fn source(repo: &LibraryRepository, owner: i64, name: &str) -> Source {
    repo.create_source(&Source {
        dir_path: name.into(),
        path_id: crate::hash_string(name),
        user_id: owner,
        source_type: Some("iCloud".into()),
        bookmark: Some(vec![1, 2, 3]),
        is_current: true,
        created_at: 100.125,
        ..Source::default()
    })
    .unwrap()
}

fn song(repo: &mut LibraryRepository, key: i64, source_id: Option<i64>) -> Song {
    repo.upsert_song(
        &Song {
            song_key: key,
            artist: "Artist".into(),
            title: format!("Track {key}"),
            album: "Album".into(),
            album_artist: "Artist".into(),
            source_id,
            relative_path: source_id.map(|_| format!("{key}.flac")),
            ..Song::default()
        },
        100.0,
    )
    .unwrap()
}

#[test]
fn playlist_membership_order_dates_duplicate_adds_and_rollback() {
    let mut repo = LibraryRepository::open(":memory:").unwrap();
    let songs: Vec<_> = (1..=5)
        .map(|key| song(&mut repo, key, None).id.unwrap())
        .collect();
    let list = repo.create_playlist("List", 100.125).unwrap();
    assert_eq!(list.created_at, 100.125);
    repo.add_playlist_songs(list.id, &songs[..3]).unwrap();
    repo.reorder_playlist(list.id, &[songs[2], songs[0], songs[1]])
        .unwrap();
    repo.add_playlist_songs(list.id, &[songs[0], songs[3], songs[2], songs[3], songs[4]])
        .unwrap();
    let expected = [songs[2], songs[0], songs[1], songs[3], songs[4]];
    assert_eq!(
        repo.playlist_songs(list.id)
            .unwrap()
            .iter()
            .map(|song| song.id.unwrap())
            .collect::<Vec<_>>(),
        expected
    );
    let mut positions = Vec::new();
    repo.db
        .query(
            "SELECT position FROM playlist_songs WHERE playlistId=? ORDER BY position",
            &[Integer(list.id)],
            &mut |row| {
                positions.push(row.integer(0)?.unwrap());
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(positions, [0, 1, 2, 3, 4]);
    assert!(repo.reorder_playlist(list.id, &[songs[0], 9999]).is_err());
    assert!(
        repo.reorder_playlist(list.id, &[songs[0], songs[0]])
            .is_err()
    );
    assert_eq!(repo.playlist_songs(list.id).unwrap().len(), 5);
    let second = repo.create_playlist("Other", -0.125).unwrap();
    assert_eq!(second.created_at, -0.125);
    assert!(
        repo.add_playlist_songs(second.id, &[songs[0], 9999])
            .is_err()
    );
    assert!(repo.playlist_songs(second.id).unwrap().is_empty());
    let renamed = repo.rename_playlist(list.id, "Renamed", 200.375).unwrap();
    assert_eq!(renamed.created_at, 100.125);
    assert_eq!(renamed.updated_at, Some(200.375));
    repo.db.execute_batch("PRAGMA query_only=ON").unwrap();
    assert_eq!(
        repo.add_playlist_songs(list.id, &[songs[0]])
            .unwrap_err()
            .code,
        8
    );
    repo.db.execute_batch("PRAGMA query_only=OFF").unwrap();
    repo.delete_playlist(list.id).unwrap();
    assert!(repo.playlist_songs(list.id).unwrap().is_empty());
    assert!(repo.add_playlist_songs(list.id, &[songs[0]]).is_err());
    assert_eq!(repo.playlists().unwrap(), [second]);
    assert_eq!(repo.get_songs(&songs).unwrap().len(), 5);
}

#[test]
fn users_source_ownership_bookmark_compare_and_swap_and_reconnect() {
    let mut repo = LibraryRepository::open(":memory:").unwrap();
    let user = repo.get_or_create_user(12345).unwrap();
    assert_eq!(repo.get_or_create_user(12345).unwrap(), user);
    repo.db.execute_batch("PRAGMA query_only=ON").unwrap();
    assert_eq!(repo.get_or_create_user(12345).unwrap(), user);
    assert!(repo.get_or_create_user(99999).is_err());
    repo.db.execute_batch("PRAGMA query_only=OFF").unwrap();
    let other = repo.get_or_create_user(67890).unwrap();
    let first = source(&repo, user.id, "/first");
    let second = source(&repo, user.id, "/second");
    let foreign = source(&repo, other.id, "/other");
    let id = first.id.unwrap();
    repo.set_current_source(user.id, id).unwrap();
    assert!(repo.source(id).unwrap().unwrap().is_current);
    assert!(!repo.source(second.id.unwrap()).unwrap().unwrap().is_current);
    assert!(
        repo.set_current_source(user.id, foreign.id.unwrap())
            .is_err()
    );
    assert!(repo.source(id).unwrap().unwrap().is_current);
    assert!(
        repo.source(foreign.id.unwrap())
            .unwrap()
            .unwrap()
            .is_current
    );
    assert!(
        repo.update_source_bookmark(id, &[4, 5], Some(&[1, 2, 3]))
            .unwrap()
    );
    assert!(
        !repo
            .update_source_bookmark(id, &[9], Some(&[1, 2, 3]))
            .unwrap()
    );
    assert!(
        !repo
            .update_resolved_source_root(id, "/stale", 99, &[6], Some(&[1, 2, 3]), 200.0)
            .unwrap()
    );
    assert!(
        repo.update_resolved_source_root(id, "/moved", 99, &[6], Some(&[4, 5]), 200.25)
            .unwrap()
    );
    let moved = repo.source(id).unwrap().unwrap();
    assert_eq!(moved.dir_path, "/moved");
    assert_eq!(moved.created_at, 100.125);
    assert_eq!(moved.updated_at, Some(200.25));
    let mut updated = moved.clone();
    updated.bookmark = Some(vec![0]);
    updated.sync_error = Some("Needs reconnect".into());
    assert_eq!(
        repo.update_source(&updated).unwrap().bookmark,
        moved.bookmark
    );
    let connected = repo
        .reconnect_source(id, "/reconnected", 100, &[9], 201.5)
        .unwrap()
        .unwrap();
    assert_eq!(connected.sync_error, None);
    assert_eq!(connected.bookmark, Some(vec![9]));
    assert_eq!(repo.sources(user.id).unwrap().len(), 2);
    repo.delete_source(id).unwrap();
    assert!(!repo.update_source_bookmark(id, &[0], Some(&[9])).unwrap());
    assert!(
        repo.reconnect_source(id, "/ghost", 101, &[0], 202.0)
            .unwrap()
            .is_none()
    );
}

#[test]
fn source_path_identity_search_metadata_pruning_and_removal_are_atomic() {
    let mut repo = LibraryRepository::open(":memory:").unwrap();
    let first = source(&repo, 1, "/first");
    let second = source(&repo, 1, "/nested");
    let id = first.id.unwrap();
    let retained_id = second.id.unwrap();
    let path = SourcePath {
        source_id: id,
        path_id: 10,
        parent_path_id: Some(first.path_id),
        name: "Track.flac".into(),
        relative_path: "Album/Track.flac".into(),
        file_size: Some(1000),
        file_hash_sha256: Some(vec![0, 255]),
        run_id: 1,
        created_at: 100.125,
        ..SourcePath::default()
    };
    let mut stale = path.clone();
    stale.path_id = 11;
    stale.relative_path = "Old.flac".into();
    repo.upsert_source_paths(&[path.clone(), stale]).unwrap();
    let saved = repo.source_path(id, &path.relative_path).unwrap().unwrap();
    assert!(saved.is_audio());
    let mut moved = path.clone();
    moved.path_id = 99;
    moved.file_size = None;
    moved.file_hash_sha256 = None;
    moved.run_id = 2;
    moved.updated_at = Some(200.125);
    repo.upsert_source_paths(&[moved.clone()]).unwrap();
    repo.upsert_source_paths(&[moved.clone(), moved.clone()])
        .unwrap();
    let latest = repo.source_path(id, &path.relative_path).unwrap().unwrap();
    assert_eq!(latest.id, saved.id);
    assert_eq!(latest.created_at, saved.created_at);
    assert_eq!(latest.file_size, Some(1000));
    assert_eq!(latest.file_hash_sha256, path.file_hash_sha256);
    let matches = repo.search_source_paths(id, "Track", 10).unwrap();
    assert_eq!(
        matches.iter().map(|entry| entry.0).collect::<Vec<_>>(),
        [99, 11]
    );
    assert!(
        repo.search_source_paths(retained_id, "Track", 10)
            .unwrap()
            .is_empty()
    );
    assert_eq!(repo.finish_source_scan(id, 2).unwrap(), 1);
    assert_eq!(repo.search_source_paths(id, "Track", 10).unwrap().len(), 1);
    assert_eq!(
        repo.source_paths(id, Some(first.path_id)).unwrap(),
        [latest]
    );
    let target = song(&mut repo, 1, Some(id));
    let other = song(&mut repo, 2, Some(retained_id));
    let list = repo.create_playlist("Mixed", 100.0).unwrap();
    repo.add_playlist_songs(list.id, &[target.id.unwrap(), other.id.unwrap()])
        .unwrap();
    repo.delete_source(id).unwrap();
    assert!(repo.get_song(target.id.unwrap()).unwrap().is_none());
    assert_eq!(repo.playlist_songs(list.id).unwrap(), [other]);
    assert!(repo.source_paths(id, None).unwrap().is_empty());
    assert!(
        repo.search_source_paths(id, "Track", 10)
            .unwrap()
            .is_empty()
    );
    assert!(repo.upsert_source_paths(&[moved]).is_err());
    assert!(repo.upsert_song(&target, 300.0).is_err());
    assert!(repo.finish_source_scan(id, 3).is_err());
    assert!(
        repo.upsert_source_paths(&[SourcePath {
            source_id: retained_id,
            relative_path: "../escape.flac".into(),
            ..SourcePath::default()
        }])
        .is_err()
    );
    repo.upsert_source_paths(&[]).unwrap();
}

#[test]
fn source_batches_roll_back_all_metadata_and_search_rows_on_failure() {
    let mut repo = LibraryRepository::open(":memory:").unwrap();
    let saved = source(&repo, 1, "/source");
    let id = saved.id.unwrap();
    let first = SourcePath {
        source_id: id,
        path_id: 1,
        relative_path: "first.flac".into(),
        name: "first.flac".into(),
        run_id: 1,
        ..SourcePath::default()
    };
    let second = SourcePath {
        source_id: id,
        path_id: 2,
        relative_path: "second.flac".into(),
        name: "second.flac".into(),
        run_id: 1,
        ..SourcePath::default()
    };
    repo.db.execute_batch("CREATE TRIGGER reject_second BEFORE INSERT ON source_paths WHEN NEW.pathId=2 BEGIN SELECT RAISE(ABORT,'Injected failure'); END").unwrap();
    assert!(repo.upsert_source_paths(&[first.clone(), second]).is_err());
    assert!(repo.source_paths(id, None).unwrap().is_empty());
    assert!(
        repo.search_source_paths(id, "first", 10)
            .unwrap()
            .is_empty()
    );
    repo.db.execute_batch("DROP TRIGGER reject_second").unwrap();
    repo.upsert_source_paths(&[first]).unwrap();
    repo.db.execute_batch("CREATE TRIGGER reject_source_delete BEFORE DELETE ON sources BEGIN SELECT RAISE(ABORT,'Injected failure'); END").unwrap();
    assert!(repo.delete_source(id).is_err());
    assert!(repo.source(id).unwrap().is_some());
    assert_eq!(repo.source_paths(id, None).unwrap().len(), 1);
    assert_eq!(repo.search_source_paths(id, "first", 10).unwrap().len(), 1);
}

#[test]
fn legacy_source_and_path_columns_upgrade_without_losing_bookmarks_or_sizes() {
    let db = Connection::open(":memory:").unwrap();
    db.execute_batch("CREATE TABLE sources (id INTEGER PRIMARY KEY AUTOINCREMENT, dirPath TEXT NOT NULL,pathId INTEGER NOT NULL,userId INTEGER NOT NULL,type TEXT,totalPaths INTEGER,syncError TEXT,isCurrent INTEGER NOT NULL,createdAt TEXT NOT NULL,lastSyncedAt TEXT,updatedAt TEXT); INSERT INTO sources (dirPath,pathId,userId,isCurrent,createdAt) VALUES ('/legacy',1,1,1,'1970-01-01T00:01:40.125'); CREATE TABLE source_paths (id INTEGER PRIMARY KEY AUTOINCREMENT,sourceId INTEGER NOT NULL,pathId INTEGER NOT NULL,parentPathId INTEGER,name TEXT NOT NULL,relativePath TEXT NOT NULL,isDirectory INTEGER NOT NULL,fileHashSHA256 BLOB,runId INTEGER NOT NULL,createdAt TEXT NOT NULL,updatedAt TEXT); INSERT INTO source_paths (sourceId,pathId,name,relativePath,isDirectory,runId,createdAt) VALUES (1,2,'track.flac','track.flac',0,1,'1970-01-01T00:01:40.125')").unwrap();
    let repo = LibraryRepository::new(db).unwrap();
    let repo = LibraryRepository::new(repo.db).unwrap();
    let source = repo.source(1).unwrap().unwrap();
    assert_eq!(source.bookmark, None);
    assert_eq!(source.created_at, 100.125);
    let path = repo.source_path(1, "track.flac").unwrap().unwrap();
    assert_eq!(path.file_size, None);
    assert_eq!(path.created_at, 100.125);
    let mut indexed = false;
    repo.db.query("EXPLAIN QUERY PLAN SELECT id FROM songs WHERE sourceId=1 AND relativePath='track.flac'", &[], &mut |row| { indexed |= row.text(3)?.is_some_and(|detail| detail.contains("INDEX lw_songs_source_path")); Ok(()) }).unwrap();
    assert!(indexed);
}
