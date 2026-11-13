//! Explicit worker-side storage benchmark; frame allocation budgets are checked separately.
use lw_core::{LibraryRepository, Source, SourcePath};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let mut repo = LibraryRepository::open(":memory:")?;
    let source = repo.create_source(&Source {
        dir_path: "/benchmark".into(),
        user_id: 1,
        ..Source::default()
    })?;
    let source_id = source.id.ok_or("Source is unsaved")?;
    let mut paths: Vec<_> = (1..=10000)
        .map(|id| SourcePath {
            source_id,
            path_id: id,
            parent_path_id: Some(0),
            name: format!("Track{id}.flac"),
            relative_path: format!("Album{}/Track{id}.flac", id / 10),
            file_size: Some(1000),
            run_id: 1,
            created_at: 100.125,
            ..SourcePath::default()
        })
        .collect();
    let initial = Instant::now();
    repo.upsert_source_paths(&paths)?;
    let initial_ms = initial.elapsed().as_secs_f64() * 1000.0;
    let first_id = repo
        .source_path(source_id, &paths[0].relative_path)?
        .ok_or("Missing initial path")?
        .id;
    for path in &mut paths {
        path.run_id = 2;
        path.path_id += 10000;
        path.updated_at = Some(200.125);
    }
    let rescan = Instant::now();
    repo.upsert_source_paths(&paths)?;
    repo.finish_source_scan(source_id, 2)?;
    let rescan_ms = rescan.elapsed().as_secs_f64() * 1000.0;
    let hydrated = repo.source_paths(source_id, None)?;
    assert_eq!(hydrated.len(), 10000);
    assert_eq!(hydrated.first().unwrap().id, first_id);
    assert_eq!(hydrated.first().unwrap().file_size, Some(1000));
    assert_eq!(
        repo.search_source_paths(source_id, "Track", 10001)?.len(),
        10000
    );
    let matches = repo.search_source_paths(source_id, "Track9999", 10)?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].0, 19999);
    println!(
        "10,000 paths: initial={initial_ms:.2}ms rescan={rescan_ms:.2}ms total={:.2}ms; identity, metadata, pruning and FTS count verified",
        started.elapsed().as_secs_f64() * 1000.0
    );
    Ok(())
}
