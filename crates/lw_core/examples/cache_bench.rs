//! Worker-only cache metadata and pure eviction policy; no filesystem or provider latency.
use lw_apple_sys::sqlite::Connection;
use lw_core::{LibraryRepository, cache};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Connection::open(":memory:")?;
    db.execute_batch(include_str!("../src/songs.sql"))?;
    db.execute_batch("WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM seq WHERE n<10000) INSERT INTO songs(songKey,artist,title,album,albumArtist,pathHash,createdAt,coverArtPath,localFilePath,fileState) SELECT n,'Artist','Track','Album','Ensemble',n,n,'CoverArt/cover.png','Music/'||n||'.flac',2 FROM seq")?;
    let repo = LibraryRepository::new(db)?;
    let ids: Vec<_> = (1..=10000).collect();
    let _ = repo.cache_owners()?;
    mem_sys::local_reset();
    let started = Instant::now();
    let owners = repo.cache_owners()?;
    let owner_ms = started.elapsed().as_secs_f64() * 1000.0;
    let owner_allocs = mem_sys::local_alloc_count();
    assert_eq!(owners.len(), ids.len());
    assert!(owner_allocs < 11000);
    mem_sys::local_reset();
    let started = Instant::now();
    let songs = repo.get_songs(&ids)?;
    let song_ms = started.elapsed().as_secs_f64() * 1000.0;
    let song_allocs = mem_sys::local_alloc_count();
    assert_eq!(songs.len(), owners.len());
    struct Remove;
    impl cache::EvictionIO for Remove {
        fn remove_if_unprotected(&mut self, _: &cache::File) -> std::io::Result<bool> {
            Ok(true)
        }
    }
    let mut files: Vec<_> = owners
        .into_iter()
        .enumerate()
        .map(|(i, owner)| cache::File {
            path: owner.stored_path.into(),
            bytes: 1024,
            last_use: ((i * 137) % 10000) as f64,
        })
        .collect();
    mem_sys::local_reset();
    let started = Instant::now();
    let result = cache::evict(&mut files, 0, &mut Remove)?;
    let policy_ms = started.elapsed().as_secs_f64() * 1000.0;
    let policy_allocs = mem_sys::local_alloc_count();
    assert_eq!(result.files, 10000);
    assert_eq!(result.remaining_bytes, 0);
    assert_eq!(policy_allocs, 0);
    println!(
        "10,000 cached songs: owners={owner_ms:.2}ms/{owner_allocs} Rust allocations; full_songs={song_ms:.2}ms/{song_allocs} Rust allocations; eviction_policy={policy_ms:.2}ms/{policy_allocs} Rust allocations; no disk/native allocation claim"
    );
    Ok(())
}
