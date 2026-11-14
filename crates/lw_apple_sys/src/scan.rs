//! Incremental resource-only folder enumeration; its root grant outlives the enumerator.
use crate::bookmarks::{Access, failure};
use std::ffi::{CStr, CString, c_char, c_void};
use std::io;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct Entry {
    pub name: String,
    pub relative_path: String,
    pub normalized_url: String,
    pub normalized_parent: String,
    pub is_directory: bool,
    pub file_size: Option<i64>,
}

#[repr(C)]
#[derive(Default)]
struct RawEntry {
    name: *mut c_char,
    relative: *mut c_char,
    url: *mut c_char,
    parent: *mut c_char,
    directory: i32,
    size: i64,
}
impl Drop for RawEntry {
    fn drop(&mut self) {
        unsafe {
            lw_scan_entry_clear(self);
        }
    }
}

unsafe extern "C" {
    fn lw_bookmark_url(access: *mut c_void) -> *mut c_void;
    fn lw_scan_open(
        root: *mut c_void,
        registered: *const c_char,
        excluded: *const *const c_char,
        count: usize,
        error: *mut *mut c_char,
    ) -> *mut c_void;
    fn lw_scan_next(scan: *mut c_void, entry: *mut RawEntry, error: *mut *mut c_char) -> i32;
    fn lw_scan_entry_clear(entry: *mut RawEntry);
    fn lw_scan_release(scan: *mut c_void);
}

pub struct Scan<'a> {
    raw: *mut c_void,
    _root: PhantomData<&'a Access>,
}
fn path(path: &Path) -> io::Result<CString> {
    CString::new(
        path.to_str()
            .ok_or_else(|| io::Error::other("Scan path is not UTF-8"))?,
    )
    .map_err(|_| io::Error::other("Scan path contains NUL"))
}
impl<'a> Scan<'a> {
    pub fn open(
        root: &'a Access,
        registered_root: &Path,
        excluded: &[PathBuf],
    ) -> io::Result<Self> {
        let registered = path(registered_root)?;
        let excluded = excluded
            .iter()
            .map(|item| path(item))
            .collect::<io::Result<Vec<_>>>()?;
        let pointers: Vec<_> = excluded.iter().map(|item| item.as_ptr()).collect();
        let mut error = ptr::null_mut();
        let raw = unsafe {
            lw_scan_open(
                lw_bookmark_url(root.raw()),
                registered.as_ptr(),
                pointers.as_ptr(),
                pointers.len(),
                &mut error,
            )
        };
        if raw.is_null() {
            return Err(failure(error));
        }
        Ok(Self {
            raw,
            _root: PhantomData,
        })
    }
    pub fn next(&mut self, cancelled: &AtomicBool) -> io::Result<Option<Entry>> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let mut entry = RawEntry::default();
        let mut error = ptr::null_mut();
        match unsafe { lw_scan_next(self.raw, &mut entry, &mut error) } {
            0 => Ok(None),
            1 => {
                let text = |raw: *const c_char| {
                    if raw.is_null() {
                        return Err(io::Error::other("Native scan omitted a path"));
                    }
                    unsafe { CStr::from_ptr(raw) }
                        .to_str()
                        .map(str::to_owned)
                        .map_err(io::Error::other)
                };
                Ok(Some(Entry {
                    name: text(entry.name)?,
                    relative_path: text(entry.relative)?,
                    normalized_url: text(entry.url)?,
                    normalized_parent: text(entry.parent)?,
                    is_directory: entry.directory != 0,
                    file_size: (entry.size >= 0).then_some(entry.size),
                }))
            }
            _ => Err(failure(error)),
        }
    }
}
impl Drop for Scan<'_> {
    fn drop(&mut self) {
        unsafe {
            lw_scan_release(self.raw);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resource_scan_preserves_identity_and_filters_generated_hidden_package_and_escaping_items() {
        let root = std::env::temp_dir().join(format!(
            "lw-scan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        for folder in [
            "Collection/Music",
            "App/Documents/Music",
            "App/Documents/Music Original",
            "App/tmp/LocalWave Share",
            "Example.app/Contents",
        ] {
            std::fs::create_dir_all(root.join(folder)).unwrap();
        }
        for file in [
            "Collection/Music/original.flac",
            "App/Documents/Music/generated.flac",
            "App/Documents/Music Original/retained.flac",
            "App/tmp/LocalWave Share/shared.flac",
            "Example.app/Contents/package.flac",
            ".hidden.flac",
            "database.sqlite",
        ] {
            std::fs::write(root.join(file), [1, 2, 3]).unwrap();
        }
        std::fs::write(root.join("notes.txt"), b"cover notes").unwrap();
        std::os::unix::fs::symlink(
            root.join("Collection/Music/original.flac"),
            root.join("alias.flac"),
        )
        .unwrap();
        std::os::unix::fs::symlink(root.join("missing.flac"), root.join("dangling.flac")).unwrap();
        std::os::unix::fs::symlink("/etc/hosts", root.join("outside.flac")).unwrap();
        let data = crate::bookmarks::create(&root).unwrap();
        let access = Access::resolve(&data).unwrap();
        let excluded = [
            root.join("App/Documents/Music"),
            root.join("App/tmp/LocalWave Share"),
            root.join("database.sqlite"),
        ];
        let registered = Path::new("/registered/Music");
        let mut scan = Scan::open(&access, registered, &excluded).unwrap();
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            scan.next(&cancelled).unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        cancelled.store(false, Ordering::Relaxed);
        let mut entries = Vec::new();
        while let Some(entry) = scan.next(&cancelled).unwrap() {
            entries.push(entry);
        }
        let files: std::collections::BTreeSet<_> = entries
            .iter()
            .filter(|entry| !entry.is_directory)
            .map(|entry| entry.relative_path.as_str())
            .collect();
        assert_eq!(
            files,
            std::collections::BTreeSet::from([
                "App/Documents/Music Original/retained.flac",
                "Collection/Music/original.flac",
                "alias.flac",
                "notes.txt"
            ])
        );
        let note = entries
            .iter()
            .find(|entry| entry.name == "notes.txt")
            .unwrap();
        assert_eq!(note.file_size, Some(11));
        assert_eq!(note.normalized_url, "file:///registered/Music/notes.txt");
        assert_eq!(note.normalized_parent, "file:///registered/Music");
        assert!(
            entries
                .iter()
                .any(|entry| entry.relative_path == "Example.app" && entry.is_directory)
        );
        let generated = Access::resolve(&crate::bookmarks::create(&excluded[0]).unwrap()).unwrap();
        assert!(Scan::open(&generated, registered, &excluded).is_err());
        assert!(Scan::open(&access, Path::new("relative"), &[]).is_err());
        for registered in [root.clone(), root.canonicalize().unwrap()] {
            let mut scan = Scan::open(&access, &registered, &excluded).unwrap();
            while let Some(entry) = scan.next(&cancelled).unwrap() {
                let identity = registered.join(&entry.relative_path);
                assert_eq!(
                    entry.normalized_url,
                    crate::normalized_file_url(identity.to_str().unwrap()).unwrap()
                );
                assert_eq!(
                    entry.normalized_parent,
                    crate::normalized_file_url(identity.parent().unwrap().to_str().unwrap())
                        .unwrap()
                );
            }
        }
    }
}
