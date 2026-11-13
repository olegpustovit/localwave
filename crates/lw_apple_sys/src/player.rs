//! Worker-prepared items move once to a main-thread AVQueuePlayer; polling does not allocate.
use crate::bookmarks::failure;
use std::cell::Cell;
use std::ffi::{CString, c_char, c_void};
use std::io;
use std::marker::PhantomData;
use std::path::Path;
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Debug)]
pub struct Prepared {
    raw: NonNull<c_void>,
    token: u64,
    occurrence: u64,
    _unique: PhantomData<Cell<()>>,
}
// The item has no observers or player until its sole owner moves it to the main thread.
unsafe impl Send for Prepared {}
impl Prepared {
    pub fn open(
        path: &Path,
        token: u64,
        occurrence: u64,
        known_local_copy: bool,
        cancelled: &AtomicBool,
    ) -> io::Result<Self> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if !path.is_absolute() || token == 0 || occurrence == 0 {
            return Err(io::Error::other(
                "Invalid prepared playback identity or path",
            ));
        }
        let path = CString::new(
            path.to_str()
                .ok_or_else(|| io::Error::other("Playback path is not UTF-8"))?,
        )
        .map_err(io::Error::other)?;
        extern "C" fn is_cancelled(raw: *mut c_void) -> i32 {
            unsafe { &*raw.cast::<AtomicBool>() }.load(Ordering::Relaxed) as i32
        }
        let mut error = ptr::null_mut();
        let raw = unsafe {
            lw_player_prepare(
                path.as_ptr(),
                token,
                occurrence,
                known_local_copy as i32,
                is_cancelled,
                ptr::from_ref(cancelled).cast_mut().cast(),
                &mut error,
            )
        };
        let Some(raw) = NonNull::new(raw) else {
            let error = failure(error);
            return Err(if cancelled.load(Ordering::Relaxed) {
                io::ErrorKind::Interrupted.into()
            } else {
                error
            });
        };
        let item = Self {
            raw,
            token,
            occurrence,
            _unique: PhantomData,
        };
        if cancelled.load(Ordering::Relaxed) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        Ok(item)
    }
    pub fn token(&self) -> u64 {
        self.token
    }
    pub fn occurrence(&self) -> u64 {
        self.occurrence
    }
}
impl Drop for Prepared {
    fn drop(&mut self) {
        unsafe { lw_player_prepared_release(self.raw.as_ptr()) };
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub generation: u64,
    pub token: u64,
    pub occurrence: u64,
    pub queued: [u64; 3],
    pub position: f64,
    pub duration: f64,
    pub rate: f32,
    pub volume: f32,
    pub status: i32,
    pub time_control: i32,
}
impl Snapshot {
    pub fn playing(&self) -> bool {
        self.status == 1 && self.time_control == 2
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Current,
    Finished,
    Failed,
    Ready,
    State,
    EngineFailed,
    Started,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct RawEvent {
    kind: i32,
    status: i32,
    generation: u64,
    token: u64,
    occurrence: u64,
    seconds: f64,
    message: [u8; 1024],
}
#[derive(Clone, Copy)]
pub struct Event {
    pub kind: Kind,
    raw: RawEvent,
}
impl Event {
    pub fn generation(&self) -> u64 {
        self.raw.generation
    }
    pub fn token(&self) -> u64 {
        self.raw.token
    }
    pub fn occurrence(&self) -> u64 {
        self.raw.occurrence
    }
    pub fn duration(&self) -> f64 {
        self.raw.seconds
    }
    pub fn message(&self) -> &str {
        let length = self
            .raw
            .message
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.raw.message.len());
        std::str::from_utf8(&self.raw.message[..length])
            .unwrap_or("The audio file could not be decoded")
    }
}
#[derive(Debug)]
pub struct Player {
    raw: NonNull<c_void>,
    _thread: PhantomData<Rc<()>>,
}
impl Player {
    pub fn set_wake(&mut self, wake: Option<extern "C" fn()>) {
        unsafe { lw_player_set_wake(self.raw.as_ptr(), wake) };
    }
    /// Retain the buffered prefix after the current item; false rejects a stale current identity.
    pub fn trim(&mut self, generation: u64, current: u64, keep: usize) -> io::Result<bool> {
        let mut error = ptr::null_mut();
        match unsafe { lw_player_trim(self.raw.as_ptr(), generation, current, keep, &mut error) } {
            1 => Ok(true),
            2 => Ok(false),
            _ => Err(failure(error)),
        }
    }
    pub fn open() -> io::Result<Self> {
        let mut error = ptr::null_mut();
        NonNull::new(unsafe { lw_player_create(&mut error) })
            .map(|raw| Self {
                raw,
                _thread: PhantomData,
            })
            .ok_or_else(|| failure(error))
    }
    pub fn replace(&mut self, generation: u64, current: Prepared) -> io::Result<()> {
        let mut error = ptr::null_mut();
        if unsafe {
            lw_player_replace(
                self.raw.as_ptr(),
                generation,
                current.raw.as_ptr(),
                &mut error,
            )
        } == 0
        {
            return Err(failure(error));
        }
        Ok(())
    }
    /// False means playback advanced while these upcoming items were being prepared.
    pub fn upcoming(
        &mut self,
        generation: u64,
        current: u64,
        items: [Option<Prepared>; 2],
    ) -> io::Result<bool> {
        if items[0].is_none() && items[1].is_some() {
            return Err(io::Error::other(
                "Upcoming playback items must form a prefix",
            ));
        }
        let mut pointers = [ptr::null_mut(); 2];
        let mut count = 0;
        for item in items.iter().flatten() {
            pointers[count] = item.raw.as_ptr();
            count += 1;
        }
        let mut error = ptr::null_mut();
        match unsafe {
            lw_player_upcoming(
                self.raw.as_ptr(),
                generation,
                current,
                pointers.as_ptr(),
                count,
                &mut error,
            )
        } {
            1 => Ok(true),
            2 => Ok(false),
            _ => Err(failure(error)),
        }
    }
    pub fn play(&mut self) {
        unsafe { lw_player_play(self.raw.as_ptr()) };
    }
    pub fn append(&mut self, generation: u64, current: u64, item: &Prepared) -> io::Result<bool> {
        let mut error = ptr::null_mut();
        match unsafe {
            lw_player_append(
                self.raw.as_ptr(),
                generation,
                current,
                item.raw.as_ptr(),
                &mut error,
            )
        } {
            1 => Ok(true),
            2 => Ok(false),
            _ => Err(failure(error)),
        }
    }
    pub fn pause(&mut self) {
        unsafe { lw_player_pause(self.raw.as_ptr()) };
    }
    pub fn stop(&mut self) {
        unsafe { lw_player_stop(self.raw.as_ptr()) };
    }
    pub fn advance(&mut self) {
        unsafe { lw_player_advance(self.raw.as_ptr()) };
    }
    pub fn seek(&mut self, seconds: f64) -> io::Result<()> {
        if !seconds.is_finite() {
            return Err(io::Error::other("Playback position must be finite"));
        }
        unsafe { lw_player_seek(self.raw.as_ptr(), seconds) };
        Ok(())
    }
    pub fn set_volume(&mut self, volume: f32) -> io::Result<()> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(io::Error::other(
                "Playback volume must be between zero and one",
            ));
        }
        unsafe { lw_player_volume(self.raw.as_ptr(), volume) };
        Ok(())
    }
    pub fn snapshot(&self) -> Snapshot {
        let mut snapshot = Snapshot::default();
        unsafe { lw_player_snapshot(self.raw.as_ptr(), &mut snapshot) };
        snapshot
    }
    pub fn event(&mut self) -> io::Result<Option<Event>> {
        let mut raw = RawEvent {
            kind: 0,
            status: 0,
            generation: 0,
            token: 0,
            occurrence: 0,
            seconds: 0.0,
            message: [0; 1024],
        };
        match unsafe { lw_player_event(self.raw.as_ptr(), &mut raw) } {
            0 => Ok(None),
            1 => {
                let kind = match raw.kind {
                    1 => Kind::Current,
                    2 => Kind::Finished,
                    3 => Kind::Failed,
                    4 => Kind::Ready,
                    5 => Kind::State,
                    6 => Kind::EngineFailed,
                    7 => Kind::Started,
                    _ => return Err(io::Error::other("Unknown native playback event")),
                };
                Ok(Some(Event { kind, raw }))
            }
            _ => Err(io::Error::other(
                "Native playback event queue overflowed; playback was paused",
            )),
        }
    }
    /// Headless hosts must service the main queue; the GUI uses its normal event loop.
    pub fn run_loop(&mut self, wait: Duration) {
        unsafe { lw_player_run_loop(wait.as_secs_f64().min(0.05)) };
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        unsafe { lw_player_release(self.raw.as_ptr()) };
    }
}
unsafe extern "C" {
    fn lw_player_set_wake(player: *mut c_void, wake: Option<extern "C" fn()>);
    fn lw_player_trim(
        player: *mut c_void,
        generation: u64,
        current: u64,
        keep: usize,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_player_prepare(
        path: *const c_char,
        token: u64,
        occurrence: u64,
        known: i32,
        cancelled: extern "C" fn(*mut c_void) -> i32,
        user: *mut c_void,
        error: *mut *mut c_char,
    ) -> *mut c_void;
    fn lw_player_prepared_release(prepared: *mut c_void);
    fn lw_player_create(error: *mut *mut c_char) -> *mut c_void;
    fn lw_player_replace(
        player: *mut c_void,
        generation: u64,
        prepared: *mut c_void,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_player_upcoming(
        player: *mut c_void,
        generation: u64,
        current: u64,
        prepared: *const *mut c_void,
        count: usize,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_player_play(player: *mut c_void);
    fn lw_player_append(
        player: *mut c_void,
        generation: u64,
        current: u64,
        prepared: *mut c_void,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_player_pause(player: *mut c_void);
    fn lw_player_stop(player: *mut c_void);
    fn lw_player_advance(player: *mut c_void);
    fn lw_player_seek(player: *mut c_void, seconds: f64);
    fn lw_player_volume(player: *mut c_void, volume: f32);
    fn lw_player_snapshot(player: *mut c_void, snapshot: *mut Snapshot);
    fn lw_player_event(player: *mut c_void, event: *mut RawEvent) -> i32;
    fn lw_player_release(player: *mut c_void);
    fn lw_player_run_loop(seconds: f64);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_items_prepare_on_workers_and_reject_invalid_cancelled_corrupt_and_cloud_inputs() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a");
        let cancel = AtomicBool::new(false);
        let item = Prepared::open(&fixture, 1, 2, false, &cancel).unwrap();
        assert_eq!((item.token(), item.occurrence()), (1, 2));
        assert!(
            Player::open()
                .unwrap_err()
                .to_string()
                .contains("main thread")
        );
        assert!(Prepared::open(&fixture, 0, 2, false, &cancel).is_err());
        assert!(Prepared::open(Path::new("relative.m4a"), 1, 2, false, &cancel).is_err());
        cancel.store(true, Ordering::Relaxed);
        assert_eq!(
            Prepared::open(&fixture, 1, 2, false, &cancel)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
        cancel.store(false, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "lw-player-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("Library/FileProviderStorage")).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let provider = root.join("Library/FileProviderStorage/file.m4a");
        std::fs::copy(&fixture, &provider).unwrap();
        assert!(
            Prepared::open(&provider, 1, 2, false, &cancel)
                .unwrap_err()
                .to_string()
                .contains("not confirmed local")
        );
        let bad = root.join("bad.m4a");
        std::fs::write(&bad, b"not audio").unwrap();
        assert!(Prepared::open(&bad, 1, 2, false, &cancel).is_err());
    }
}
