use std::io;
use std::path::{Component, Path, PathBuf};

/// Resolve a relative song location within an already-granted source folder.
pub fn source_file(root: &Path, relative: &str) -> io::Result<PathBuf> {
    if !crate::models::valid_relative_path(relative) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid source-relative audio path",
        ));
    }
    let root = root.canonicalize()?;
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            "Music source is not a folder",
        ));
    }
    let mut file = root.clone();
    for component in Path::new(relative).components() {
        match component {
            Component::CurDir => continue,
            Component::Normal(component) => file.push(component),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Audio path is outside its source folder",
                ));
            }
        }
        match file.symlink_metadata() {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                file = file.canonicalize()?;
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        if !file.starts_with(&root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Audio path is outside its source folder",
            ));
        }
    }
    if file == root {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Audio path names its source folder",
        ));
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn paths_preserve_moved_roots_and_reject_absolute_parent_and_symlink_escapes() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base =
            std::env::temp_dir().join(format!("localwave-paths-{}-{unique}", std::process::id()));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        std::fs::create_dir(&base).unwrap();
        let _cleanup = Cleanup(base.clone());
        let root = base.join("Root");
        std::fs::create_dir_all(root.join("Album")).unwrap();
        std::fs::write(root.join("Album/track.flac"), [1, 2, 3]).unwrap();
        std::fs::write(base.join("outside.flac"), [4, 5, 6]).unwrap();
        std::os::unix::fs::symlink(base.join("outside.flac"), root.join("escape.flac")).unwrap();
        std::os::unix::fs::symlink(base.join("missing.flac"), root.join("dangling.flac")).unwrap();
        std::os::unix::fs::symlink(root.join("Album"), root.join("inside")).unwrap();
        for path in [
            "",
            ".",
            "../outside.flac",
            "Album/../../outside.flac",
            "/outside.flac",
            "escape.flac",
            "dangling.flac",
            "invalid\0path",
        ] {
            assert!(source_file(&root, path).is_err(), "Accepted {path:?}");
        }
        assert_eq!(
            std::fs::read(source_file(&root, "inside/track.flac").unwrap()).unwrap(),
            [1, 2, 3]
        );
        assert_eq!(
            source_file(&root, "Album/missing.flac").unwrap(),
            root.canonicalize().unwrap().join("Album/missing.flac")
        );
        let moved = base.join("Moved");
        std::fs::rename(&root, &moved).unwrap();
        assert_eq!(
            std::fs::read(source_file(&moved, "Album/track.flac").unwrap()).unwrap(),
            [1, 2, 3]
        );
        assert!(source_file(&root, "Album/track.flac").is_err());
    }
}
