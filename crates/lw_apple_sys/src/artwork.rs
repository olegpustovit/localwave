//! Content-addressed, atomic cover-art publication on the import worker.
use crate::bookmarks::failure;
use std::ffi::{CStr, CString, c_char};
use std::{io, path::Path, ptr};

unsafe extern "C" {
    fn lw_artwork_store(
        documents: *const c_char,
        bytes: *const u8,
        len: usize,
        relative: *mut *mut c_char,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_free_string(value: *mut c_char);
}
pub fn store(documents: &Path, bytes: &[u8]) -> io::Result<Option<String>> {
    let documents = CString::new(
        documents
            .to_str()
            .ok_or_else(|| io::Error::other("Artwork path is not UTF-8"))?,
    )
    .map_err(io::Error::other)?;
    let (mut relative, mut error) = (ptr::null_mut(), ptr::null_mut());
    match unsafe {
        lw_artwork_store(
            documents.as_ptr(),
            bytes.as_ptr(),
            bytes.len(),
            &mut relative,
            &mut error,
        )
    } {
        0 => Ok(None),
        1 => {
            let path = unsafe { CStr::from_ptr(relative) }
                .to_str()
                .map(str::to_owned)
                .map_err(io::Error::other);
            unsafe { lw_free_string(relative) };
            path.map(Some)
        }
        _ => Err(failure(error)),
    }
}
