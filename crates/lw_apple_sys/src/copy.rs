//! Coordinated, bounded-memory offline copies. Source grants remain owned by the caller.
use crate::bookmarks::failure;
use crate::preferences::Preferences;
use std::ffi::{CStr, CString, c_char, c_void};
use std::io;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

#[repr(C)]
#[derive(Default)]
struct RawResult {
    path: *mut c_char,
    modified: f64,
    size: i64,
}
impl Drop for RawResult {
    fn drop(&mut self) {
        unsafe { lw_free_string(self.path) };
    }
}
unsafe extern "C" {
    fn lw_file_copy(
        source: *const c_char,
        destination: *const c_char,
        preserve_name: i32,
        preferences: *mut c_void,
        cancelled: extern "C" fn(*mut c_void) -> i32,
        user: *mut c_void,
        progress: extern "C" fn(*mut c_void, u64) -> i32,
        progress_user: *mut c_void,
        result: *mut RawResult,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_free_string(text: *mut c_char);
}
#[derive(Debug)]
pub struct CopiedFile {
    pub path: PathBuf,
    pub modified: Option<f64>,
    pub size: Option<i64>,
}

pub fn copy(
    source: &Path,
    destination: &Path,
    preferences: &Preferences,
    cancelled: &AtomicBool,
) -> io::Result<CopiedFile> {
    copy_with(source, destination, preferences, cancelled, &mut |_| Ok(()))
}
pub fn copy_with(
    source: &Path,
    destination: &Path,
    preferences: &Preferences,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(u64) -> io::Result<()>,
) -> io::Result<CopiedFile> {
    copy_impl(source, destination, false, preferences, cancelled, progress)
}

/// Preserve the source filename, reusing identical content or prefixing collisions with a UUID.
pub fn copy_named(
    source: &Path,
    destination: &Path,
    preferences: &Preferences,
    cancelled: &AtomicBool,
) -> io::Result<CopiedFile> {
    copy_impl(
        source,
        destination,
        true,
        preferences,
        cancelled,
        &mut |_| Ok(()),
    )
}

fn copy_impl(
    source: &Path,
    destination: &Path,
    preserve_name: bool,
    preferences: &Preferences,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(u64) -> io::Result<()>,
) -> io::Result<CopiedFile> {
    struct Progress<'a> {
        operation: &'a mut dyn FnMut(u64) -> io::Result<()>,
        outcome: Option<std::thread::Result<io::Result<()>>>,
    }
    extern "C" fn notify(raw: *mut c_void, bytes: u64) -> i32 {
        let progress = unsafe { &mut *raw.cast::<Progress<'_>>() };
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (progress.operation)(bytes)));
        let success = matches!(&outcome, Ok(Ok(())));
        progress.outcome = Some(outcome);
        success as i32
    }
    extern "C" fn is_cancelled(raw: *mut c_void) -> i32 {
        unsafe { &*raw.cast::<AtomicBool>() }.load(Ordering::Relaxed) as i32
    }
    let path = |path: &Path| {
        CString::new(
            path.to_str()
                .ok_or_else(|| io::Error::other("Copy path is not UTF-8"))?,
        )
        .map_err(io::Error::other)
    };
    let source = path(source)?;
    let destination = path(destination)?;
    let mut progress = Progress {
        operation: progress,
        outcome: None,
    };
    let mut result = RawResult::default();
    let mut error = ptr::null_mut();
    let status = unsafe {
        lw_file_copy(
            source.as_ptr(),
            destination.as_ptr(),
            preserve_name as i32,
            preferences.raw(),
            is_cancelled,
            ptr::from_ref(cancelled).cast_mut().cast(),
            notify,
            ptr::from_mut(&mut progress).cast(),
            &mut result,
            &mut error,
        )
    };
    let native_error = (status != 1).then(|| failure(error));
    match progress.outcome {
        Some(Err(panic)) => std::panic::resume_unwind(panic),
        Some(Ok(Err(error))) => return Err(error),
        _ => {}
    }
    if let Some(error) = native_error {
        return Err(match status {
            -1 => io::Error::new(io::ErrorKind::Interrupted, error),
            -2 => io::Error::new(io::ErrorKind::NotConnected, error),
            _ => error,
        });
    }
    if result.path.is_null() {
        return Err(io::Error::other("Copy produced no file location"));
    }
    Ok(CopiedFile {
        path: unsafe { CStr::from_ptr(result.path) }
            .to_str()
            .map(PathBuf::from)
            .map_err(io::Error::other)?,
        modified: result.modified.is_finite().then_some(result.modified),
        size: (result.size >= 0).then_some(result.size),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn coordinated_streams_promote_once_and_clean_up_cancelled_changed_failed_and_offline_copies() {
        let root = std::env::temp_dir().join(format!(
            "lw-copy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        fs::create_dir_all(&root).unwrap();
        let preferences =
            Preferences::open(Some(&format!("localwave-copy-test-{}", std::process::id())))
                .unwrap();
        let source = root.join("large.FLAC");
        let content: Vec<_> = (0..768 * 1024 + 37).map(|i| (i % 251) as u8).collect();
        fs::write(&source, &content).unwrap();
        let destination = root.join("Music");
        let cancelled = AtomicBool::new(false);
        let mut last = 0;
        let first = copy_with(
            &source,
            &destination,
            &preferences,
            &cancelled,
            &mut |bytes| {
                assert!(bytes > last);
                last = bytes;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(last, content.len() as u64);
        assert_eq!(first.size, Some(content.len() as i64));
        assert_eq!(first.path.extension().unwrap(), "flac");
        assert_eq!(
            first.path.file_name().unwrap(),
            "1402d6132b06cf2f2e0e41a31414673396b153fbc895322bbde62f942837d28e.flac"
        );
        assert_eq!(fs::read(&first.path).unwrap(), content);
        let mut threads = Vec::new();
        for _ in 0..8 {
            let source = source.clone();
            let destination = destination.clone();
            threads.push(std::thread::spawn(move || {
                let preferences =
                    Preferences::open(Some(&format!("localwave-copy-test-{}", std::process::id())))
                        .unwrap();
                copy(&source, &destination, &preferences, &AtomicBool::new(false))
                    .unwrap()
                    .path
            }));
        }
        for thread in threads {
            assert_eq!(thread.join().unwrap(), first.path);
        }
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 1);
        fs::write(&first.path, b"corrupt cache").unwrap();
        assert!(
            copy(&source, &destination, &preferences, &cancelled)
                .unwrap_err()
                .to_string()
                .contains("does not match")
        );
        assert_eq!(fs::read(&source).unwrap(), content);
        fs::remove_file(&first.path).unwrap();
        std::os::unix::fs::symlink(&source, &first.path).unwrap();
        assert!(copy(&source, &destination, &preferences, &cancelled).is_err());
        fs::remove_file(&first.path).unwrap();
        let cancel_dir = root.join("Cancelled");
        let error = copy_with(&source, &cancel_dir, &preferences, &cancelled, &mut |_| {
            cancelled.store(true, Ordering::Relaxed);
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(fs::read_dir(&cancel_dir).unwrap().count(), 0);
        let untouched = root.join("NeverCreated");
        assert_eq!(
            copy(&source, &untouched, &preferences, &cancelled)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
        assert!(!untouched.exists());
        cancelled.store(false, Ordering::Relaxed);
        let error = copy_with(&source, &cancel_dir, &preferences, &cancelled, &mut |_| {
            Err(io::ErrorKind::InvalidData.into())
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read_dir(&cancel_dir).unwrap().count(), 0);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| copy_with(
                &source,
                &cancel_dir,
                &preferences,
                &cancelled,
                &mut |_| panic!("copy callback")
            )))
            .is_err()
        );
        assert_eq!(fs::read_dir(&cancel_dir).unwrap().count(), 0);
        let mut modified = false;
        let error = copy_with(&source, &cancel_dir, &preferences, &cancelled, &mut |_| {
            if !modified {
                use std::io::Write;
                fs::OpenOptions::new()
                    .append(true)
                    .open(&source)?
                    .write_all(&[0])?;
                modified = true;
            }
            Ok(())
        })
        .unwrap_err();
        assert!(error.to_string().contains("changed during offline copy"));
        assert_eq!(fs::read_dir(&cancel_dir).unwrap().count(), 0);
        assert!(
            copy(
                &root.join("missing.flac"),
                &untouched,
                &preferences,
                &cancelled
            )
            .is_err()
        );
        assert!(!untouched.exists());
        let provider = root.join("Library/CloudStorage/Provider/track.flac");
        fs::create_dir_all(provider.parent().unwrap()).unwrap();
        fs::write(&provider, &content).unwrap();
        preferences.set_no_network(true);
        assert_eq!(
            copy(&provider, &untouched, &preferences, &cancelled)
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotConnected
        );
        assert!(!untouched.exists());
        preferences.set_no_network(false);
        let error = copy_with(
            &provider,
            &cancel_dir,
            &preferences,
            &cancelled,
            &mut |_| {
                preferences.set_no_network(true);
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotConnected);
        assert_eq!(fs::read_dir(&cancel_dir).unwrap().count(), 0);
        assert!(copy(&source, &destination, &preferences, &cancelled).is_ok());
        preferences.set_no_network(false);
        assert_eq!(
            fs::read(
                copy(&provider, &destination, &preferences, &cancelled)
                    .unwrap()
                    .path
            )
            .unwrap(),
            content
        );
        let added = root.join("Added Files");
        let named = copy_named(&source, &added, &preferences, &cancelled).unwrap();
        assert_eq!(named.path.file_name(), source.file_name());
        assert_eq!(fs::read(&named.path).unwrap(), fs::read(&source).unwrap());
        assert_eq!(
            copy_named(&source, &added, &preferences, &cancelled)
                .unwrap()
                .path,
            named.path
        );
        assert_eq!(fs::read_dir(&added).unwrap().count(), 1);
        fs::write(&named.path, b"another track").unwrap();
        let collision = copy_named(&source, &added, &preferences, &cancelled).unwrap();
        assert_ne!(collision.path, named.path);
        assert!(
            collision
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with("-large.FLAC")
        );
        assert_eq!(fs::read(&named.path).unwrap(), b"another track");
        assert_eq!(
            fs::read(&collision.path).unwrap(),
            fs::read(&source).unwrap()
        );
        fs::remove_file(&named.path).unwrap();
        std::os::unix::fs::symlink(&source, &named.path).unwrap();
        let no_follow = copy_named(&source, &added, &preferences, &cancelled).unwrap();
        assert_ne!(no_follow.path, named.path);
        assert!(!fs::symlink_metadata(&no_follow.path).unwrap().is_symlink());
        assert!(fs::symlink_metadata(&named.path).unwrap().is_symlink());
        let error = copy_impl(
            &source,
            &cancel_dir,
            true,
            &preferences,
            &cancelled,
            &mut |_| {
                cancelled.store(true, Ordering::Relaxed);
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(fs::read_dir(&cancel_dir).unwrap().count(), 0);
        cancelled.store(false, Ordering::Relaxed);
        preferences.set_no_network(true);
        assert_eq!(
            copy_named(&provider, &untouched, &preferences, &cancelled)
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotConnected
        );
        assert!(!untouched.exists());
        preferences.set_no_network(false);
    }
}
