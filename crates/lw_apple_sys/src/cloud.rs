//! Swift-compatible secure identity encoding; no account read runs on the UI thread.
use crate::bookmarks::failure;
use std::{
    ffi::{CStr, c_char},
    io, ptr,
};

unsafe extern "C" {
    fn lw_cloud_identity(encoded: *mut *mut c_char, error: *mut *mut c_char) -> i32;
    fn lw_free_string(value: *mut c_char);
}

fn take(encoded: *mut c_char, error: *mut c_char, success: i32) -> io::Result<Option<String>> {
    if success == 0 {
        return Err(failure(error));
    }
    if encoded.is_null() {
        return Ok(None);
    }
    let value = unsafe { CStr::from_ptr(encoded) }
        .to_str()
        .map(str::to_owned)
        .map_err(io::Error::other);
    unsafe {
        lw_free_string(encoded);
    }
    value.map(Some)
}

/// Encode exactly as DefaultICloudProvider: secure keyed archive, then base64 without line breaks.
/// Keep this opaque value on the worker; callers hash it with the shared library identity helper.
pub fn current_identity() -> io::Result<Option<String>> {
    crate::require_worker_thread()?;
    let (mut encoded, mut error) = (ptr::null_mut(), ptr::null_mut());
    let success = unsafe { lw_cloud_identity(&mut encoded, &mut error) };
    take(encoded, error, success)
}

#[cfg(all(test, feature = "interop-test"))]
mod tests {
    use super::*;
    unsafe extern "C" {
        fn lw_cloud_identity_fixture(
            kind: i32,
            encoded: *mut *mut c_char,
            error: *mut *mut c_char,
        ) -> i32;
    }
    #[test]
    fn secure_identity_archive_matches_swift_and_distinguishes_absence_from_failure() {
        let fixture = include_str!("../fixtures/cloud-identity.txt");
        for kind in 0..3 {
            let (mut encoded, mut error) = (ptr::null_mut(), ptr::null_mut());
            let success = unsafe { lw_cloud_identity_fixture(kind, &mut encoded, &mut error) };
            let result = take(encoded, error, success);
            match kind {
                0 => assert_eq!(result.unwrap(), None),
                1 => assert_eq!(result.unwrap().as_deref(), fixture.lines().next()),
                _ => assert!(
                    result.is_err(),
                    "an archive error must not become a signed-out account"
                ),
            }
        }
    }
}
