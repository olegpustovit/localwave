//! Offline cache policy. Filesystem, preference and playback checks stay in the worker's IO.
use crate::FileState;
use std::io;
use std::path::PathBuf;

pub struct File {
    pub path: PathBuf,
    pub bytes: u64,
    pub last_use: f64,
}
pub struct Owner {
    pub song_id: i64,
    pub stored_path: String,
    pub state: FileState,
}
pub trait EvictionIO {
    /// Recheck pin/playback protection and remove only a managed copy, including its stale records.
    fn remove_if_unprotected(&mut self, file: &File) -> io::Result<bool>;
}
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Eviction {
    pub files: usize,
    pub remaining_bytes: u64,
}
pub fn total_bytes(files: &[File]) -> io::Result<u64> {
    let mut total = 0u64;
    for file in files.iter() {
        if !file.last_use.is_finite() {
            return Err(io::Error::other("Invalid cache modification date"));
        }
        total = total
            .checked_add(file.bytes)
            .ok_or_else(|| io::Error::other("Offline cache size overflow"))?;
    }
    Ok(total)
}
pub fn evict(files: &mut [File], limit: u64, io: &mut dyn EvictionIO) -> io::Result<Eviction> {
    let mut result = Eviction {
        remaining_bytes: total_bytes(files)?,
        ..Eviction::default()
    };
    if result.remaining_bytes <= limit {
        return Ok(result);
    }
    files.sort_unstable_by(|a, b| {
        a.last_use
            .total_cmp(&b.last_use)
            .then_with(|| a.path.cmp(&b.path))
    });
    for file in files {
        if result.remaining_bytes <= limit {
            break;
        }
        if io.remove_if_unprotected(file)? {
            result.files += 1;
            result.remaining_bytes -= file.bytes;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lru_rechecks_protection_and_keeps_protected_bytes_over_the_limit() {
        struct IO(Vec<String>);
        impl EvictionIO for IO {
            fn remove_if_unprotected(&mut self, file: &File) -> io::Result<bool> {
                let name = file.path.to_str().unwrap();
                self.0.push(name.to_owned());
                Ok(!["pinned", "active", "queued"].contains(&name))
            }
        }
        let mut files: Vec<_> = ["queued", "active", "pinned", "old", "recent"]
            .into_iter()
            .enumerate()
            .map(|(i, name)| File {
                path: name.into(),
                bytes: 4,
                last_use: i as f64,
            })
            .collect();
        let mut io = IO(vec![]);
        assert_eq!(
            evict(&mut files, 16, &mut io).unwrap(),
            Eviction {
                files: 1,
                remaining_bytes: 16
            }
        );
        assert_eq!(io.0, ["queued", "active", "pinned", "old"]);
        files.retain(|file| file.path != std::path::Path::new("old"));
        assert_eq!(
            evict(&mut files, 0, &mut io).unwrap(),
            Eviction {
                files: 1,
                remaining_bytes: 12
            }
        );
        let mut overflow = [
            File {
                path: "huge".into(),
                bytes: u64::MAX,
                last_use: 0.0,
            },
            File {
                path: "one".into(),
                bytes: 1,
                last_use: 0.0,
            },
        ];
        let calls = io.0.len();
        assert!(evict(&mut overflow, 0, &mut io).is_err());
        assert_eq!(calls, io.0.len());
    }
}
