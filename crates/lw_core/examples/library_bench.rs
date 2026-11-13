//! Worker-side library workload; warmed UI frames have their own zero-allocation gate.
use lw_apple_sys::sqlite::Connection;
use lw_core::LibraryRepository;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Connection::open(":memory:")?;
    db.execute_batch(include_str!("../src/songs.sql"))?;
    db.execute_batch("WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM seq WHERE n<10000) INSERT INTO songs (songKey,artist,title,album,albumArtist,pathHash,createdAt,coverArtPath) SELECT n,'Artist '||(n%10),'Track '||n,'Album '||(n%100),'Ensemble',n,n,'cover-'||n||'.jpg' FROM seq; INSERT INTO songs_fts(songId,artist,title,album,albumArtist) SELECT id,artist,title,album,albumArtist FROM songs")?;
    // Give both queries the same album index as a migrated user library.
    db.execute_batch("CREATE INDEX lw_songs_album ON songs(album,artist)")?;
    let baseline = Instant::now();
    let mut old_count = 0;
    db.query("SELECT album,artist,coverArtPath FROM songs s1 WHERE album<>'' AND LENGTH(artist)=(SELECT MIN(LENGTH(artist)) FROM songs s2 WHERE s2.album=s1.album) GROUP BY album ORDER BY album", &[], &mut |_| { old_count += 1; Ok(()) })?;
    let baseline_ms = baseline.elapsed().as_secs_f64() * 1000.0;
    let mut repo = LibraryRepository::new(db)?;
    let started = Instant::now();
    let albums = repo.albums()?;
    let albums_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(albums.len(), 100);
    assert_eq!(albums.len(), old_count);

    let ids: Vec<_> = (1..=10000).rev().collect();
    mem_sys::local_reset();
    let started = Instant::now();
    let mut songs = repo.get_songs(&ids)?;
    let load_ms = started.elapsed().as_secs_f64() * 1000.0;
    let load_allocations = mem_sys::local_alloc_count();
    assert_eq!(songs.len(), ids.len());
    assert!(
        songs
            .iter()
            .zip(&ids)
            .all(|(song, id)| song.id == Some(*id))
    );
    // Five nonempty owned strings per fixture song, plus query and result storage.
    assert!(load_allocations < ids.len() as u64 * 6);
    let started = Instant::now();
    let matches = repo.search_songs("Track 9999", 20, 0)?;
    let search_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, Some(9999));

    let playlist = repo.create_playlist("10k songs", 100.0)?;
    let started = Instant::now();
    repo.add_playlist_songs(playlist.id, &ids)?;
    let playlist_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(repo.playlist_songs(playlist.id)?.len(), ids.len());
    let started = Instant::now();
    for song in &mut songs {
        song.title.push_str(" updated");
        repo.upsert_song(song, 20_000.0)?;
    }
    let tags_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(repo.song_count("updated")?, ids.len());
    assert_eq!(repo.playlist_songs(playlist.id)?.len(), ids.len());
    println!(
        "10,000 songs: albums={albums_ms:.2}ms (previous={baseline_ms:.2}ms); bulk_read={load_ms:.2}ms rust_allocations={load_allocations}; FTS={search_ms:.2}ms; playlist_add={playlist_ms:.2}ms; tag_updates={tags_ms:.2}ms; count, order and FTS verified"
    );
    Ok(())
}
