//! Safe access to native platform services.
pub mod sqlite;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod collation;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod timing;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod bookmarks;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod audio;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod scan;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod preferences;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod cloud;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod history;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod artwork;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod copy;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod profile;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod context;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod waveform;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod session;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod player;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod media;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn require_worker_thread() -> std::io::Result<()> {
    unsafe extern "C" {
        fn lw_is_main_thread() -> i32;
    }
    if unsafe { lw_is_main_thread() } != 0 {
        Err(std::io::Error::other(
            "Library I/O must run on a worker thread",
        ))
    } else {
        Ok(())
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod window;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn thread_cpu_time_ns() -> std::io::Result<u64> {
    unsafe extern "C" {
        fn lw_thread_cpu_ns() -> u64;
    }
    let value = unsafe { lw_thread_cpu_ns() };
    if value == u64::MAX {
        Err(std::io::Error::other("Thread CPU clock is unavailable"))
    } else {
        Ok(value)
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn ui_thread_priority() -> std::io::Result<()> {
    unsafe extern "C" {
        fn lw_ui_thread_priority() -> i32;
    }
    match unsafe { lw_ui_thread_priority() } {
        0 => Ok(()),
        code => Err(std::io::Error::from_raw_os_error(code)),
    }
}

#[cfg(all(test, target_os = "macos"))]
mod thread_tests {
    #[test]
    fn ui_priority_rejects_worker_threads() {
        std::thread::spawn(|| {
            assert_eq!(
                super::ui_thread_priority().unwrap_err().kind(),
                std::io::ErrorKind::InvalidInput
            )
        })
        .join()
        .unwrap();
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn normalized_file_url(path: &str) -> std::io::Result<String> {
    use std::ffi::{CStr, CString, c_char};
    unsafe extern "C" {
        fn lw_normalized_file_url(path: *const c_char) -> *mut c_char;
        fn lw_free_string(text: *mut c_char);
    }
    let path = CString::new(path).map_err(|_| std::io::Error::other("File path contains NUL"))?;
    let raw = unsafe { lw_normalized_file_url(path.as_ptr()) };
    if raw.is_null() {
        return Err(std::io::Error::other("Expected an absolute file path"));
    }
    let result = unsafe { CStr::from_ptr(raw) }
        .to_str()
        .map(str::to_owned)
        .map_err(std::io::Error::other);
    unsafe {
        lw_free_string(raw);
    }
    result
}
