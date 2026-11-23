//! Change observation for one source folder: FSEvents on macOS, an NSFilePresenter on iOS. The
//! observer takes only the registered path; it never holds or rebuilds a security-scoped grant, and
//! rescans resolve the bookmark themselves. Callbacks run on a private serial queue and only call
//! the supplied closure (set a flag, wake a worker). Dropping never blocks: the native queue stops
//! delivery and then frees the closure, so a callback already in flight may still run after drop
//! returns (consumers ignore marks for sources they no longer know).
use std::ffi::{CString, c_char, c_void};
use std::{io, path::Path};

unsafe extern "C" {
    fn lw_observer_create(
        path: *const c_char,
        notify: extern "C" fn(*mut c_void),
        done: extern "C" fn(*mut c_void),
        context: *mut c_void,
    ) -> *mut c_void;
    fn lw_observer_release(raw: *mut c_void);
}

type Notify = Box<dyn Fn() + Send + Sync>;

pub struct FolderObserver {
    raw: *mut c_void,
}
// The native stream delivers on its own serial queue and the closure is Send + Sync.
unsafe impl Send for FolderObserver {}

extern "C" fn trampoline(context: *mut c_void) {
    unsafe { (*context.cast::<Notify>())() }
}
extern "C" fn done(context: *mut c_void) {
    drop(unsafe { Box::from_raw(context.cast::<Notify>()) });
}

impl FolderObserver {
    pub fn new(path: &Path, notify: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let path = CString::new(
            path.to_str()
                .ok_or_else(|| io::Error::other("Source path is not UTF-8"))?,
        )
        .map_err(|_| io::Error::other("Source path contains NUL"))?;
        let notify = Box::into_raw(Box::new(Box::new(notify) as Notify));
        let raw = unsafe { lw_observer_create(path.as_ptr(), trampoline, done, notify.cast()) };
        if raw.is_null() {
            done(notify.cast());
            return Err(io::Error::other("Folder observation is unavailable"));
        }
        Ok(Self { raw })
    }
}

impl Drop for FolderObserver {
    fn drop(&mut self) {
        unsafe { lw_observer_release(self.raw) };
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::{Duration, Instant};

    #[test]
    fn drop_returns_immediately_even_while_fseventsd_registration_is_pending() {
        let observer = FolderObserver::new(&std::env::temp_dir(), || {}).unwrap();
        let started = Instant::now();
        drop(observer);
        // Freeing the closure waits for fseventsd on the native queue (seconds on a degraded
        // host); the on-demand delivery test checks it. Callers only rely on drop not blocking.
        assert!(
            started.elapsed() < Duration::from_millis(50),
            "drop blocked"
        );
    }

    // Reason: real FSEvents delivery depends on fseventsd. On the M1 host (2026-11-15) fseventsd ran
    // at ~100% CPU with 5 GB resident and a standalone C probe got events in 2 of 6 runs, so this
    // is on demand: cargo t -p lw_apple_sys observer -- --ignored.
    #[test]
    #[ignore = "fseventsd delivery is degraded on the reference host"]
    fn plain_writes_notify_until_drop_and_drop_frees_the_closure() {
        let dir = std::env::temp_dir().join(format!("lw-observer-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dir = dir.canonicalize().unwrap();
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        let observer = FolderObserver::new(&dir, move || {
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();
        // FSEvents coalesces with a 0.2 s latency; allow host scheduling on top.
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut index = 0u8;
        while hits.load(Ordering::SeqCst) == 0 {
            // A stream only reports changes after it starts; repeat until the first arrives.
            std::fs::write(dir.join("song.flac"), [index]).unwrap();
            index = index.wrapping_add(1);
            assert!(Instant::now() < deadline, "no folder change notification");
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(observer);
        while Arc::strong_count(&hits) > 1 {
            assert!(Instant::now() < deadline, "closure not freed after drop");
            std::thread::yield_now();
        }
        let after = hits.load(Ordering::SeqCst);
        std::fs::write(dir.join("later.flac"), [1]).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(
            hits.load(Ordering::SeqCst),
            after,
            "no delivery after the closure is freed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
