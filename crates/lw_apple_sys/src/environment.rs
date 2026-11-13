//! Main-thread AppKit preferences, cached by notifications and read without querying the OS per frame.
use std::ffi::c_void;
use std::io;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// 0 unknown, 1 light, 2 dark, matching the frame input and persisted appearance codes.
    pub appearance: u32,
    pub reduced_motion: u32,
}

pub struct Environment {
    raw: NonNull<c_void>,
    _main: PhantomData<Rc<()>>,
}
impl Environment {
    /// Create after the window and drop before it, so notifications can safely wake its event loop.
    pub fn open(wake: extern "C" fn()) -> io::Result<Self> {
        let raw = unsafe { lw_environment_create(wake) };
        NonNull::new(raw)
            .map(|raw| Self {
                raw,
                _main: PhantomData,
            })
            .ok_or_else(|| {
                io::Error::other(
                    "System preferences need a live AppKit application on its main thread",
                )
            })
    }
    pub fn snapshot(&self) -> Snapshot {
        let mut value = Snapshot::default();
        unsafe { lw_environment_read(self.raw.as_ptr(), &mut value) };
        value
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        unsafe { lw_environment_release(self.raw.as_ptr()) };
    }
}
unsafe extern "C" {
    fn lw_environment_create(wake: extern "C" fn()) -> *mut c_void;
    fn lw_environment_read(environment: *mut c_void, state: *mut Snapshot);
    fn lw_environment_release(environment: *mut c_void);
}

#[cfg(feature = "interop-test")]
pub fn verify_notifications() -> bool {
    unsafe extern "C" {
        fn lw_environment_test_notifications() -> i32;
    }
    unsafe { lw_environment_test_notifications() != 0 }
}

#[cfg(test)]
mod tests {
    #[test]
    fn system_preferences_reject_worker_thread_construction() {
        extern "C" fn wake() {}
        std::thread::spawn(|| assert!(super::Environment::open(wake).is_err()))
            .join()
            .unwrap();
    }
}
