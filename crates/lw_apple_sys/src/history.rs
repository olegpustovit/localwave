//! Worker-only Swift-compatible playback history; snapshots avoid repeated defaults copies per song.
use crate::preferences::Preferences;
use std::ffi::{CString, c_char, c_void};
use std::io;
use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;

unsafe extern "C" {
    fn lw_preferences_history(raw: *mut c_void) -> *mut c_void;
    fn lw_history_get(raw: *mut c_void, song: i64, plays: *mut i64, last_played: *mut f64);
    fn lw_preferences_record_playback(raw: *mut c_void, song: i64, now: f64);
    fn lw_preferences_release(raw: *mut c_void);
    fn lw_touch_cache_file(path: *const c_char, root: *const c_char, now: f64) -> i32;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Record {
    pub plays: i64,
    pub last_played: Option<f64>,
}
pub struct History {
    raw: *mut c_void,
    _thread: PhantomData<Rc<()>>,
}
impl History {
    pub fn get(&self, song: i64) -> Record {
        let (mut plays, mut last_played) = (0, f64::NAN);
        unsafe { lw_history_get(self.raw, song, &mut plays, &mut last_played) };
        Record {
            plays,
            last_played: last_played.is_finite().then_some(last_played),
        }
    }
}
impl Drop for History {
    fn drop(&mut self) {
        unsafe { lw_preferences_release(self.raw) };
    }
}
impl Preferences {
    pub fn history(&self) -> History {
        History {
            raw: unsafe { lw_preferences_history(self.raw()) },
            _thread: PhantomData,
        }
    }
    /// The caller deduplicates actual starts by physical playback token, including repeat-one loads.
    pub fn record_playback(&self, song: i64, now: f64) -> io::Result<()> {
        if !now.is_finite() || now < 0.0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid playback date",
            ));
        }
        unsafe { lw_preferences_record_playback(self.raw(), song, now) };
        Ok(())
    }
}

/// Best-effort recency update restricted to immediate regular-file children of the managed cache.
pub fn touch_cache_file(path: &Path, root: &Path, now: f64) -> bool {
    if !path.is_absolute() || !root.is_absolute() {
        return false;
    }
    let strings = || {
        Some((
            CString::new(path.to_str()?).ok()?,
            CString::new(root.to_str()?).ok()?,
        ))
    };
    let Some((path, root)) = strings() else {
        return false;
    };
    unsafe { lw_touch_cache_file(path.as_ptr(), root.as_ptr(), now) != 0 }
}

#[cfg(feature = "interop-test")]
#[unsafe(no_mangle)]
/// # Safety
/// `suite` must be a valid NUL-terminated UTF-8 string for an isolated fixture suite.
pub unsafe extern "C" fn lw_history_interop_fixture(suite: *const c_char) -> i32 {
    let run = || -> io::Result<()> {
        let suite = unsafe { std::ffi::CStr::from_ptr(suite) }
            .to_str()
            .map_err(io::Error::other)?;
        let preferences = Preferences::open(Some(suite))?;
        let snapshot = preferences.history();
        if snapshot.get(i64::MAX)
            != (Record {
                plays: 7,
                last_played: Some(100.125),
            })
            || snapshot.get(22).plays != 3
            || snapshot.get(88).plays != i64::MAX
            || snapshot.get(77).plays != 0
        {
            return Err(io::Error::other("Swift history did not load"));
        }
        preferences.record_playback(i64::MAX, 200.375)?;
        preferences.record_playback(42, 201.625)?;
        preferences.record_playback(88, 202.75)?;
        preferences.record_playback(77, 203.875)?;
        Ok(())
    };
    if suite.is_null() {
        return -1;
    }
    if run().is_ok() { 0 } else { -1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshots_preserve_other_songs_and_reject_invalid_starts() {
        let suite = format!(
            "com.olegpustovit.localwave.history-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        );
        let preferences = Preferences::open(Some(&suite)).unwrap();
        let other = Preferences::open(Some(&suite)).unwrap();
        let before = preferences.history();
        assert_eq!(before.get(22), Record::default());
        preferences.record_playback(i64::MAX, 100.125).unwrap();
        other.record_playback(22, 101.25).unwrap();
        preferences.record_playback(22, 102.5).unwrap();
        assert_eq!(before.get(22), Record::default());
        let snapshot = other.history();
        assert_eq!(
            snapshot.get(22),
            Record {
                plays: 2,
                last_played: Some(102.5)
            }
        );
        assert_eq!(
            snapshot.get(i64::MAX),
            Record {
                plays: 1,
                last_played: Some(100.125)
            }
        );
        for bad in [f64::NAN, f64::INFINITY, -1.0] {
            assert!(preferences.record_playback(22, bad).is_err());
        }
        assert_eq!(preferences.history().get(22), snapshot.get(22));
    }

    #[test]
    fn recency_updates_only_real_cache_files_without_following_symlinks() {
        let base = std::env::temp_dir().join(format!(
            "lw-history-touch-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let root = base.join("Music");
        std::fs::create_dir_all(&root).unwrap();
        struct Cleanup(PathBuf);
        use std::path::PathBuf;
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let root = root.canonicalize().unwrap();
        let original = base.join("original.wav");
        std::fs::write(&original, b"original").unwrap();
        let before = original.metadata().unwrap().modified().unwrap();
        let cached = root.join("cached.wav");
        std::fs::write(&cached, b"cache").unwrap();
        let accessed = cached.metadata().unwrap().accessed().unwrap();
        assert!(touch_cache_file(&cached, &root, 100.125));
        assert_eq!(cached.metadata().unwrap().accessed().unwrap(), accessed);
        assert_eq!(
            cached
                .metadata()
                .unwrap()
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            100.125
        );
        assert!(!touch_cache_file(&original, &root, 101.0));
        let linked = root.join("linked.wav");
        std::os::unix::fs::symlink(&original, &linked).unwrap();
        assert!(!touch_cache_file(&linked, &root, 102.0));
        let linked_root = base.join("Alias");
        std::os::unix::fs::symlink(&root, &linked_root).unwrap();
        assert!(!touch_cache_file(
            &linked_root.join("cached.wav"),
            &linked_root,
            103.0
        ));
        std::fs::create_dir(root.join("directory")).unwrap();
        assert!(!touch_cache_file(&root.join("directory"), &root, 104.0));
        assert!(!touch_cache_file(&root.join("missing"), &root, 105.0));
        assert!(!touch_cache_file(&cached, &root, f64::INFINITY));
        assert_eq!(original.metadata().unwrap().modified().unwrap(), before);
    }
}
