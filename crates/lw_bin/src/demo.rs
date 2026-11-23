//! The bundled demo library (Swift's `LWDemoLibrary`): 92 recordings copied from the app's resources into Application Support's
//! "LocalWave/Demo Library" and indexed as an ordinary folder source by the FilesWorker. Runs on that worker only (file I/O).
use crate::json::{Json, parse};
use std::io;
use std::path::{Path, PathBuf};

/// The catalog's track count; a catalog of another size is incomplete.
pub const TRACKS: usize = 92;
const CATALOG: &str = "localwave-demo-catalog.json";
const MARKER: &str = ".localwave-demo-catalog-v1.json";

/// The bundled recordings' directory when the app carries them (`DemoLibrary/` in its resources).
pub fn bundled(resources: &Path) -> Option<PathBuf> {
    let directory = resources.join("DemoLibrary");
    directory.join(CATALOG).is_file().then_some(directory)
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos())
}

fn error(message: &str) -> io::Error {
    io::Error::other(message)
}

/// `relative` under `root`, refusing absolute paths and `.`/`..` or empty components (a catalog entry never leaves the folder).
fn file_under(root: &Path, relative: &str) -> io::Result<PathBuf> {
    if relative.is_empty()
        || relative.starts_with('/')
        || relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(error("The demo catalog contains an invalid file path."));
    }
    Ok(root.join(relative))
}

/// Copies the bundled recordings from `bundle` into `support`/LocalWave/Demo Library and returns that folder. An existing folder is
/// reused only when it is this app's completed copy (its marker equals the catalog and every file is there); it is never replaced.
/// A copy is staged beside it and appears by one rename, so a failure leaves no partial folder.
pub fn prepare(bundle: &Path, support: &Path) -> io::Result<PathBuf> {
    let catalog_bytes = std::fs::read(bundle.join(CATALOG))
        .map_err(|_| error("The bundled demo library is unavailable."))?;
    let catalog = std::str::from_utf8(&catalog_bytes)
        .ok()
        .and_then(|text| parse(text).ok())
        .ok_or_else(|| error("The bundled demo catalog is incomplete."))?;
    let tracks = catalog.items("tracks");
    if catalog.num("version") != 1.0 || tracks.len() != TRACKS {
        return Err(error("The bundled demo catalog is incomplete."));
    }
    let root = support.join("LocalWave");
    std::fs::create_dir_all(&root)?;
    let destination = root.join("Demo Library");
    if destination.exists() {
        if std::fs::read(destination.join(MARKER)).ok().as_deref() != Some(&catalog_bytes[..]) {
            return Err(error(
                "The Demo Library folder already exists and cannot be replaced.",
            ));
        }
        for track in tracks {
            if !file_under(&destination, track.str("relativePath"))?.is_file() {
                return Err(error(
                    "A demo file was removed. Open Folders to manage the existing Demo Library source.",
                ));
            }
        }
        return Ok(destination);
    }
    let staging = root.join(format!(".demo-staging-{}-{}", std::process::id(), nanos()));
    std::fs::create_dir_all(&staging)?;
    let result = copy_tracks(bundle, &staging, tracks)
        .and_then(|()| std::fs::write(staging.join(MARKER), &catalog_bytes))
        .and_then(|()| std::fs::rename(&staging, &destination));
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    result.map(|()| destination)
}

fn copy_tracks(bundle: &Path, staging: &Path, tracks: &[Json]) -> io::Result<()> {
    for track in tracks {
        let resource = track.str("resource");
        if resource.is_empty() || resource.contains('/') {
            return Err(error("A bundled demo recording is missing."));
        }
        let from = bundle.join(resource);
        if !from.is_file() {
            return Err(error("A bundled demo recording is missing."));
        }
        let to = file_under(staging, track.str("relativePath"))?;
        std::fs::create_dir_all(
            to.parent()
                .ok_or_else(|| error("The demo catalog contains an invalid file path."))?,
        )?;
        std::fs::copy(&from, &to)?;
        if std::fs::metadata(&to)?.len() as f64 != track.num("bytes") {
            return Err(error("A bundled demo recording is incomplete."));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(root: &Path, count: usize) -> PathBuf {
        let bundle = root.join("DemoLibrary");
        std::fs::create_dir_all(&bundle).unwrap();
        let mut tracks = Vec::new();
        for i in 0..count {
            std::fs::write(bundle.join(format!("t{i}.m4a")), [i as u8; 3]).unwrap();
            tracks.push(format!(r#"{{"resource":"t{i}.m4a","relativePath":"Artist/Album/{i:02} Song.m4a","bytes":3}}"#));
        }
        std::fs::write(
            bundle.join(CATALOG),
            format!(r#"{{"version":1,"tracks":[{}]}}"#, tracks.join(",")),
        )
        .unwrap();
        bundle
    }

    #[test]
    fn demo_copies_once_reuses_its_own_folder_and_refuses_strangers() {
        let root = std::env::temp_dir().join(format!("lw-demo-{}-{}", std::process::id(), nanos()));
        let (source, support) = (bundle(&root, TRACKS), root.join("Support"));
        assert_eq!(bundled(&root).as_deref(), Some(source.as_path()));
        let folder = prepare(&source, &support).unwrap();
        assert!(folder.join("Artist/Album/91 Song.m4a").is_file() && folder.join(MARKER).is_file());
        // its own completed copy is reused
        assert_eq!(prepare(&source, &support).unwrap(), folder);
        // a removed file, or a folder that is not ours, is reported and never replaced
        std::fs::remove_file(folder.join("Artist/Album/00 Song.m4a")).unwrap();
        assert!(
            prepare(&source, &support)
                .unwrap_err()
                .to_string()
                .starts_with("A demo file was removed")
        );
        std::fs::remove_file(folder.join(MARKER)).unwrap();
        assert!(
            prepare(&source, &support)
                .unwrap_err()
                .to_string()
                .contains("cannot be replaced")
        );
        // an incomplete catalog and an escaping path are refused before any copy
        let small = bundle(&root.join("small"), 3);
        assert!(prepare(&small, &root.join("Support2")).is_err());
        assert!(file_under(&folder, "../x").is_err() && file_under(&folder, "/x").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
