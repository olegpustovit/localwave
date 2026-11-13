//! Main-thread Now Playing publication and bounded, thread-safe remote events.
use crate::artwork::Thumbnail;
use std::ffi::{c_int, c_void};
use std::io;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Command {
    /// Play, Pause, Toggle, Stop, Next, Previous, Seek = 1...7.
    pub kind: c_int,
    pub seconds: f64,
}
#[repr(C)]
struct Info {
    song: i64,
    generation: u64,
    title: *const u8,
    title_len: usize,
    artist: *const u8,
    artist_len: usize,
    album: *const u8,
    album_len: usize,
    position: f64,
    duration: f64,
    playback_state: c_int,
    has_queue: c_int,
}
pub struct Metadata<'a> {
    pub song: i64,
    pub generation: u64,
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub position: f64,
    pub duration: f64,
    /// 1 playing, 2 paused/buffering, 3 stopped/ended.
    pub playback_state: i32,
    pub has_queue: bool,
}
pub struct Media {
    raw: NonNull<c_void>,
    _main: PhantomData<Rc<()>>,
}
impl Media {
    /// Isolated smoke/performance hosts pass false and never touch the system media session.
    pub fn open(publish: bool, wake: extern "C" fn()) -> io::Result<Self> {
        NonNull::new(unsafe { lw_media_create(publish.into(), wake) })
            .map(|raw| Self {
                raw,
                _main: PhantomData,
            })
            .ok_or_else(|| {
                io::Error::other("Media controls require the main thread and a live application")
            })
    }
    pub fn peek(&self) -> Option<Command> {
        let mut command = Command::default();
        (unsafe { lw_media_peek(self.raw.as_ptr(), &mut command) } != 0).then_some(command)
    }
    /// Acknowledge only after the application accepted the command.
    pub fn acknowledge(&mut self) {
        unsafe { lw_media_ack(self.raw.as_ptr()) };
    }
    pub fn update(&mut self, value: &Metadata<'_>) -> io::Result<()> {
        let info = Info {
            song: value.song,
            generation: value.generation,
            title: value.title.as_ptr(),
            title_len: value.title.len(),
            artist: value.artist.as_ptr(),
            artist_len: value.artist.len(),
            album: value.album.as_ptr(),
            album_len: value.album.len(),
            position: value.position,
            duration: value.duration,
            playback_state: value.playback_state,
            has_queue: value.has_queue.into(),
        };
        if unsafe { lw_media_update(self.raw.as_ptr(), &info) } == 0 {
            Err(io::ErrorKind::InvalidInput.into())
        } else {
            Ok(())
        }
    }
    pub fn artwork(&mut self, image: Option<&Thumbnail>) -> io::Result<()> {
        let (pixels, len, width, height) = image.map_or((std::ptr::null(), 0, 0, 0), |image| {
            (
                image.pixels.as_ptr(),
                image.pixels.len(),
                image.width,
                image.height,
            )
        });
        if unsafe { lw_media_artwork(self.raw.as_ptr(), pixels, len, width, height) } == 0 {
            Err(io::Error::other("Invalid Now Playing artwork"))
        } else {
            Ok(())
        }
    }
    #[cfg(feature = "interop-test")]
    pub fn inject(&mut self, kind: i32, seconds: f64) -> i32 {
        unsafe extern "C" {
            fn lw_media_test_enqueue(raw: *mut c_void, kind: c_int, seconds: f64) -> c_int;
        }
        unsafe { lw_media_test_enqueue(self.raw.as_ptr(), kind, seconds) }
    }
    #[cfg(feature = "interop-test")]
    pub fn state(&self) -> State {
        unsafe extern "C" {
            fn lw_media_test_state(raw: *mut c_void, state: *mut State);
        }
        let mut state = State::default();
        unsafe { lw_media_test_state(self.raw.as_ptr(), &mut state) };
        state
    }
}
impl Drop for Media {
    fn drop(&mut self) {
        unsafe { lw_media_release(self.raw.as_ptr()) };
    }
}
#[cfg(feature = "interop-test")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub song: i64,
    pub generation: u64,
    pub position: f64,
    pub duration: f64,
    pub playback_state: i32,
    pub artwork_width: u32,
    pub artwork_height: u32,
    pub updates: u64,
}
unsafe extern "C" {
    fn lw_media_create(publish: c_int, wake: extern "C" fn()) -> *mut c_void;
    fn lw_media_peek(raw: *mut c_void, command: *mut Command) -> c_int;
    fn lw_media_ack(raw: *mut c_void);
    fn lw_media_update(raw: *mut c_void, info: *const Info) -> c_int;
    fn lw_media_artwork(
        raw: *mut c_void,
        pixels: *const u8,
        len: usize,
        width: u32,
        height: u32,
    ) -> c_int;
    fn lw_media_release(raw: *mut c_void);
}

#[cfg(feature = "interop-test")]
pub fn verify_callbacks() -> i32 {
    unsafe extern "C" {
        fn lw_media_test_callbacks() -> c_int;
    }
    unsafe { lw_media_test_callbacks() }
}

#[cfg(test)]
mod tests {
    #[test]
    fn media_controls_reject_worker_construction() {
        extern "C" fn wake() {}
        std::thread::spawn(|| assert!(super::Media::open(false, wake).is_err()))
            .join()
            .unwrap();
    }
}
