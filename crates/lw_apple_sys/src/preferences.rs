//! Native preference and sandbox locations; construct and use on the library worker.
use std::ffi::{CStr, CString, c_char, c_void};
use std::io;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::ptr;
use std::rc::Rc;

#[repr(C)]
#[derive(Default)]
struct Bytes {
    data: *mut u8,
    len: usize,
}
unsafe extern "C" {
    fn lw_preferences_open(suite: *const c_char) -> *mut c_void;
    fn lw_preferences_set_no_network(raw: *mut c_void, disabled: i32);
    fn lw_preferences_no_network(raw: *mut c_void) -> i32;
    fn lw_cache_lock();
    fn lw_cache_unlock();
    fn lw_cache_generation() -> u64;
    fn lw_preferences_cache_snapshot(raw: *mut c_void) -> *mut c_void;
    fn lw_preferences_cache_owner_protected(raw: *mut c_void, song: i64) -> i32;
    fn lw_preferences_cache_path_protected(raw: *mut c_void, path: *const c_char) -> i32;
    fn lw_preferences_cache_limit(raw: *mut c_void) -> u64;
    fn lw_preferences_set_cache_limit(raw: *mut c_void, gigabytes: i64);
    fn lw_preferences_set_cache_protection(
        raw: *mut c_void,
        song: i64,
        active: *const c_char,
        queued: *const *const c_char,
        count: usize,
    );
    fn lw_preferences_bookmark(raw: *mut c_void, hash: i64, bytes: *mut Bytes) -> i32;
    fn lw_preferences_set_bookmark(raw: *mut c_void, hash: i64, data: *const u8, len: usize);
    fn lw_preferences_release(raw: *mut c_void);
    fn lw_preferences_import_snapshot(raw: *mut c_void) -> *mut c_void;
    fn lw_preferences_pinned(raw: *mut c_void, song: i64) -> i32;
    fn lw_preferences_cache_matches(
        raw: *mut c_void,
        song: i64,
        path: *const c_char,
        modified: f64,
        size: i64,
    ) -> i32;
    fn lw_preferences_set_pinned(raw: *mut c_void, song: i64, pinned: i32);
    fn lw_preferences_set_cache_version(
        raw: *mut c_void,
        song: i64,
        path: *const c_char,
        modified: f64,
        size: i64,
    );
    fn lw_app_path(slot: u32) -> *mut c_char;
    fn lw_file_path(value: *const c_char) -> *mut c_char;
    fn lw_free_string(value: *mut c_char);
}

fn string(value: &str) -> io::Result<CString> {
    CString::new(value).map_err(|_| io::Error::other("Preference or path contains NUL"))
}
fn path(raw: *mut c_char) -> io::Result<PathBuf> {
    if raw.is_null() {
        return Err(io::Error::other("Expected an absolute local file location"));
    }
    let result = unsafe { CStr::from_ptr(raw) }
        .to_str()
        .map(PathBuf::from)
        .map_err(io::Error::other);
    unsafe { lw_free_string(raw) };
    result
}

pub fn file_path(value: &str) -> io::Result<PathBuf> {
    path(unsafe { lw_file_path(string(value)?.as_ptr()) })
}

pub struct Preferences {
    raw: *mut c_void,
    _thread: PhantomData<Rc<()>>,
}
impl Preferences {
    pub(crate) fn from_raw(raw: *mut c_void) -> io::Result<Self> {
        if raw.is_null() {
            return Err(io::Error::other("Could not open native preferences"));
        }
        Ok(Self {
            raw,
            _thread: PhantomData,
        })
    }
    pub fn cache_guard(&self) -> CacheGuard<'_> {
        unsafe { lw_cache_lock() };
        CacheGuard {
            _owner: PhantomData,
        }
    }
    /// Cache protection writers use these setters so generation changes invalidate retained snapshots.
    pub fn cache_protection(&self) -> CacheProtection {
        let _guard = self.cache_guard();
        CacheProtection {
            raw: unsafe { lw_preferences_cache_snapshot(self.raw) },
            generation: unsafe { lw_cache_generation() },
            _thread: PhantomData,
        }
    }
    pub fn set_cache_limit(&self, gigabytes: i64) -> io::Result<()> {
        if ![2, 4, 8, 16, 32].contains(&gigabytes) {
            return Err(io::Error::other("Unsupported offline cache limit"));
        }
        let _guard = self.cache_guard();
        unsafe { lw_preferences_set_cache_limit(self.raw, gigabytes) };
        Ok(())
    }
    pub fn set_cache_protection(
        &self,
        active_song: Option<i64>,
        active_path: Option<&std::path::Path>,
        queued_paths: &[PathBuf],
    ) -> io::Result<()> {
        let canonical = |path: &std::path::Path| {
            let path = path.canonicalize()?;
            string(
                path.to_str()
                    .ok_or_else(|| io::Error::other("Cache path is not UTF-8"))?,
            )
        };
        let active = active_path.map(canonical).transpose()?;
        let queued = queued_paths
            .iter()
            .map(|path| canonical(path))
            .collect::<io::Result<Vec<_>>>()?;
        let pointers: Vec<_> = queued.iter().map(|path| path.as_ptr()).collect();
        let _guard = self.cache_guard();
        unsafe {
            lw_preferences_set_cache_protection(
                self.raw,
                active_song.unwrap_or(-1),
                active.as_ref().map_or(ptr::null(), |path| path.as_ptr()),
                pointers.as_ptr(),
                pointers.len(),
            )
        };
        Ok(())
    }
    pub(crate) fn raw(&self) -> *mut c_void {
        self.raw
    }
    pub fn set_no_network(&self, disabled: bool) {
        unsafe { lw_preferences_set_no_network(self.raw, disabled as i32) };
    }
    pub fn no_network(&self) -> bool {
        unsafe { lw_preferences_no_network(self.raw) != 0 }
    }
    pub fn import_snapshot(&self) -> ImportPreferences {
        let _guard = self.cache_guard();
        ImportPreferences {
            raw: unsafe { lw_preferences_import_snapshot(self.raw) },
            _thread: PhantomData,
        }
    }
    pub fn set_pinned(&self, song_id: i64, pinned: bool) {
        let _guard = self.cache_guard();
        unsafe { lw_preferences_set_pinned(self.raw, song_id, pinned as i32) };
    }
    pub fn set_cache_version(
        &self,
        song_id: i64,
        cache: Option<&std::path::Path>,
        modified: Option<f64>,
        size: Option<i64>,
    ) -> io::Result<()> {
        if modified.is_some_and(|date| !date.is_finite()) || size.is_some_and(|size| size < 0) {
            return Err(io::Error::other("Invalid cached source version"));
        }
        let cache = cache
            .map(|path| {
                let path = path.canonicalize()?;
                string(
                    path.to_str()
                        .ok_or_else(|| io::Error::other("Cache path is not UTF-8"))?,
                )
            })
            .transpose()?;
        let _guard = self.cache_guard();
        unsafe {
            lw_preferences_set_cache_version(
                self.raw,
                song_id,
                cache.as_ref().map_or(ptr::null(), |path| path.as_ptr()),
                modified.unwrap_or(f64::NAN),
                size.unwrap_or(-1),
            )
        };
        Ok(())
    }
    /// None selects the shipping app's standard defaults; tests use an isolated suite.
    pub fn open(suite: Option<&str>) -> io::Result<Self> {
        if suite.is_some_and(str::is_empty) {
            return Err(io::Error::other("Preference suite must not be empty"));
        }
        let suite = suite.map(string).transpose()?;
        let raw = unsafe {
            lw_preferences_open(suite.as_ref().map_or(ptr::null(), |value| value.as_ptr()))
        };
        if raw.is_null() {
            return Err(io::Error::other("Could not open native preferences"));
        }
        Ok(Self {
            raw,
            _thread: PhantomData,
        })
    }
    pub fn bookmark(&self, path_hash: i64) -> io::Result<Option<Vec<u8>>> {
        let mut bytes = Bytes::default();
        match unsafe { lw_preferences_bookmark(self.raw, path_hash, &mut bytes) } {
            0 => Ok(None),
            1 => {
                let data = unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) }.to_vec();
                unsafe { lw_free_string(bytes.data.cast()) };
                Ok(Some(data))
            }
            _ => Err(io::Error::other("Native bookmark exceeds its memory limit")),
        }
    }
    pub fn set_bookmark(&self, path_hash: i64, bookmark: Option<&[u8]>) -> io::Result<()> {
        if bookmark.is_some_and(|data| data.len() > 64 * 1024 * 1024) {
            return Err(io::Error::other("Bookmark exceeds its memory limit"));
        }
        let (data, len) = bookmark.map_or((ptr::null(), 0), |data| (data.as_ptr(), data.len()));
        unsafe { lw_preferences_set_bookmark(self.raw, path_hash, data, len) };
        Ok(())
    }
}
impl Drop for Preferences {
    fn drop(&mut self) {
        unsafe { lw_preferences_release(self.raw) };
    }
}

pub struct CacheGuard<'a> {
    _owner: PhantomData<&'a Preferences>,
}
impl Drop for CacheGuard<'_> {
    fn drop(&mut self) {
        unsafe { lw_cache_unlock() };
    }
}
pub struct CacheProtection {
    raw: *mut c_void,
    generation: u64,
    _thread: PhantomData<Rc<()>>,
}
impl CacheProtection {
    /// Hold cache_guard while checking this and deleting a file.
    pub fn is_current(&self) -> bool {
        self.generation == unsafe { lw_cache_generation() }
    }
    pub fn owner_protected(&self, song: i64) -> bool {
        unsafe { lw_preferences_cache_owner_protected(self.raw, song) != 0 }
    }
    pub fn path_protected(&self, path: &std::path::Path) -> io::Result<bool> {
        let path = string(
            path.to_str()
                .ok_or_else(|| io::Error::other("Cache path is not UTF-8"))?,
        )?;
        Ok(unsafe { lw_preferences_cache_path_protected(self.raw, path.as_ptr()) != 0 })
    }
    pub fn limit_bytes(&self) -> u64 {
        unsafe { lw_preferences_cache_limit(self.raw) }
    }
}
impl Drop for CacheProtection {
    fn drop(&mut self) {
        unsafe { lw_preferences_release(self.raw) };
    }
}

/// One consistent preference view for an import; dictionary and pin lookups are constant-time.
pub struct ImportPreferences {
    raw: *mut c_void,
    _thread: PhantomData<Rc<()>>,
}
impl ImportPreferences {
    pub fn pinned(&self, song_id: i64) -> bool {
        unsafe { lw_preferences_pinned(self.raw, song_id) != 0 }
    }
    pub fn cache_matches(
        &self,
        song_id: i64,
        cache: &std::path::Path,
        modified: Option<f64>,
        size: Option<i64>,
    ) -> io::Result<Option<bool>> {
        let cache = match cache.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let cache = string(
            cache
                .to_str()
                .ok_or_else(|| io::Error::other("Cache path is not UTF-8"))?,
        )?;
        Ok(
            match unsafe {
                lw_preferences_cache_matches(
                    self.raw,
                    song_id,
                    cache.as_ptr(),
                    modified.unwrap_or(f64::NAN),
                    size.unwrap_or(-1),
                )
            } {
                -1 => None,
                value => Some(value != 0),
            },
        )
    }
}
impl Drop for ImportPreferences {
    fn drop(&mut self) {
        unsafe { lw_preferences_release(self.raw) };
    }
}

pub struct AppPaths {
    pub documents: PathBuf,
    pub caches: PathBuf,
    pub temporary: PathBuf,
    pub test_database: Option<PathBuf>,
}
impl AppPaths {
    pub fn load() -> io::Result<Self> {
        let documents = path(unsafe { lw_app_path(0) })?;
        let caches = path(unsafe { lw_app_path(1) })?;
        let temporary = path(unsafe { lw_app_path(2) })?;
        let test = unsafe { lw_app_path(3) };
        Ok(Self {
            documents,
            caches,
            temporary,
            test_database: if test.is_null() {
                None
            } else {
                Some(path(test)?)
            },
        })
    }
    pub fn generated(&self) -> Vec<PathBuf> {
        let mut paths = vec![
            self.documents.join("CoverArt"),
            self.temporary.join("LocalWave Share"),
        ];
        if cfg!(target_os = "ios") {
            paths.push(self.documents.join("Music"));
        }
        for database in [
            Some(self.documents.join("musicApp29.sqlite")),
            Some(self.caches.join("waveforms.sqlite")),
            Some(self.caches.join("LocalWave/waveforms.sqlite")),
            self.test_database.clone(),
        ]
        .into_iter()
        .flatten()
        {
            paths.push(PathBuf::from(format!("{}-wal", database.display())));
            paths.push(PathBuf::from(format!("{}-shm", database.display())));
            paths.push(database);
        }
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_bookmarks_persist_across_handles_and_file_locations_are_strict() {
        let suite = format!(
            "com.olegpustovit.localwave.rust-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        );
        let preferences = Preferences::open(Some(&suite)).unwrap();
        preferences.set_bookmark(12, Some(&[0, 1, 0, 255])).unwrap();
        let second = Preferences::open(Some(&suite)).unwrap();
        assert_eq!(second.bookmark(12).unwrap(), Some(vec![0, 1, 0, 255]));
        second.set_bookmark(12, Some(&[])).unwrap();
        assert_eq!(preferences.bookmark(12).unwrap(), Some(vec![]));
        second.set_bookmark(12, None).unwrap();
        assert_eq!(preferences.bookmark(12).unwrap(), None);
        let cache = std::env::current_exe().unwrap().canonicalize().unwrap();
        assert_eq!(
            preferences.cache_protection().limit_bytes(),
            8 * 1_073_741_824
        );
        preferences.set_cache_limit(2).unwrap();
        assert_eq!(
            preferences.cache_protection().limit_bytes(),
            2 * 1_073_741_824
        );
        assert!(preferences.set_cache_limit(3).is_err());
        {
            let _guard = preferences.cache_guard();
            preferences
                .set_cache_protection(Some(43), Some(&cache), &[])
                .unwrap();
            let protection = preferences.cache_protection();
            assert!(protection.owner_protected(43));
            assert!(!protection.owner_protected(44));
            assert!(protection.path_protected(&cache).unwrap());
            preferences.set_cache_protection(None, None, &[]).unwrap();
            assert!(!protection.is_current());
            assert!(
                !preferences
                    .cache_protection()
                    .path_protected(&cache)
                    .unwrap()
            );
        }
        preferences.set_cache_limit(8).unwrap();
        preferences.set_pinned(42, true);
        preferences
            .set_cache_version(42, Some(&cache), Some(10.25), Some(12))
            .unwrap();
        let snapshot = preferences.import_snapshot();
        assert!(snapshot.pinned(42));
        assert_eq!(
            snapshot
                .cache_matches(42, &cache, Some(10.25), Some(12))
                .unwrap(),
            Some(true)
        );
        assert_eq!(
            snapshot
                .cache_matches(42, &cache, Some(10.5), Some(12))
                .unwrap(),
            Some(false)
        );
        assert_eq!(
            snapshot.cache_matches(42, &cache, None, Some(12)).unwrap(),
            Some(false)
        );
        assert_eq!(
            snapshot
                .cache_matches(42, std::path::Path::new("/other"), Some(10.25), Some(12))
                .unwrap(),
            None
        );
        preferences.set_pinned(42, false);
        preferences.set_cache_version(42, None, None, None).unwrap();
        assert!(snapshot.pinned(42));
        let fresh = preferences.import_snapshot();
        assert!(!fresh.pinned(42));
        assert_eq!(
            fresh
                .cache_matches(42, &cache, Some(10.25), Some(12))
                .unwrap(),
            None
        );
        assert_eq!(
            file_path(" file:///Music/Album%20One/../song.flac ").unwrap(),
            PathBuf::from("/Music/song.flac")
        );
        assert_eq!(
            file_path("/Music/Album One").unwrap(),
            PathBuf::from("/Music/Album One")
        );
        for invalid in [
            "",
            "relative/path",
            "https://example.com/song.mp3",
            "file://remote/Music",
            "file:///Music?x",
            "file:///Music#x",
            "file:///Music/%00",
            "/bad\0path",
        ] {
            assert!(file_path(invalid).is_err(), "{invalid:?}");
        }
        let paths = AppPaths::load().unwrap();
        assert!(
            paths.documents.is_absolute()
                && paths.caches.is_absolute()
                && paths.temporary.is_absolute()
        );
        let generated = paths.generated();
        assert!(generated.contains(&paths.documents.join("musicApp29.sqlite-wal")));
        assert!(generated.contains(&paths.documents.join("CoverArt")));
        assert!(generated.contains(&paths.caches.join("LocalWave/waveforms.sqlite")));
        assert!(generated.contains(&paths.caches.join("LocalWave/waveforms.sqlite-wal")));
        assert!(generated.contains(&paths.caches.join("LocalWave/waveforms.sqlite-shm")));
        assert!(!generated.contains(&paths.documents.join("Music")));
    }
}
