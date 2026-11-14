//! Foundation owns bookmark encoding and the lifetime of each security-scoped grant.
pub use file_access_sys::FileSelection;
use std::ffi::{CStr, CString, c_char, c_void};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{io, ptr};

#[repr(C)]
#[derive(Default)]
struct Bytes {
    data: *mut u8,
    len: usize,
}

unsafe extern "C" {
    fn lw_bookmark_create(
        path: *const c_char,
        bookmark: *mut Bytes,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_bookmark_resolve(
        bookmark: *const u8,
        len: usize,
        renewed: *mut Bytes,
        error: *mut *mut c_char,
    ) -> *mut c_void;
    fn lw_bookmark_path(access: *mut c_void) -> *mut c_char;
    fn lw_bookmark_release(access: *mut c_void);
    fn lw_free_string(text: *mut c_char);
}

impl Bytes {
    fn take(self) -> Vec<u8> {
        if self.data.is_null() {
            return Vec::new();
        }
        let bytes = unsafe { std::slice::from_raw_parts(self.data, self.len) }.to_vec();
        unsafe {
            lw_free_string(self.data.cast());
        }
        bytes
    }
}

pub(crate) fn failure(raw: *mut c_char) -> io::Error {
    if raw.is_null() {
        return io::Error::other("Native file access failed");
    }
    let message = unsafe { CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned();
    unsafe {
        lw_free_string(raw);
    }
    io::Error::other(message)
}

pub fn create(path: &Path) -> io::Result<Vec<u8>> {
    let path = path
        .to_str()
        .ok_or_else(|| io::Error::other("File path is not UTF-8"))?;
    let path = CString::new(path).map_err(|_| io::Error::other("File path contains NUL"))?;
    let mut bytes = Bytes::default();
    let mut error = ptr::null_mut();
    if unsafe { lw_bookmark_create(path.as_ptr(), &mut bytes, &mut error) } == 0 {
        return Err(failure(error));
    }
    Ok(bytes.take())
}

pub struct Access {
    raw: *mut c_void,
    path: PathBuf,
    renewed: Option<Vec<u8>>,
    _thread: PhantomData<Rc<()>>,
}

impl Access {
    pub(crate) fn raw(&self) -> *mut c_void {
        self.raw
    }
    pub fn resolve(bookmark: &[u8]) -> io::Result<Self> {
        let mut renewed = Bytes::default();
        let mut error = ptr::null_mut();
        let raw = unsafe {
            lw_bookmark_resolve(bookmark.as_ptr(), bookmark.len(), &mut renewed, &mut error)
        };
        if raw.is_null() {
            return Err(failure(error));
        }
        let mut access = Self {
            raw,
            path: PathBuf::new(),
            renewed: (!renewed.data.is_null()).then(|| renewed.take()),
            _thread: PhantomData,
        };
        let path = unsafe { lw_bookmark_path(raw) };
        if path.is_null() {
            return Err(io::Error::other("Could not read resolved bookmark path"));
        }
        let decoded = unsafe { CStr::from_ptr(path) }
            .to_str()
            .map(PathBuf::from)
            .map_err(io::Error::other);
        unsafe {
            lw_free_string(path);
        }
        access.path = decoded?;
        Ok(access)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn renewed_bookmark(&self) -> Option<&[u8]> {
        self.renewed.as_deref()
    }
}

impl Drop for Access {
    fn drop(&mut self) {
        unsafe {
            lw_bookmark_release(self.raw);
        }
    }
}

/// Same-process bridge used by Swift's native-bookmark qualification.
/// # Safety
/// All non-null buffers must be valid for their declared lengths and must not overlap.
#[cfg(feature = "interop-test")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lw_bookmark_interop_resolve_fixture(
    data: *const u8,
    len: usize,
    out: *mut u8,
    capacity: usize,
) -> isize {
    if data.is_null()
        || out.is_null()
        || len > isize::MAX as usize
        || capacity > isize::MAX as usize
    {
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    let Ok(access) = Access::resolve(bytes) else {
        return -1;
    };
    if std::fs::read(access.path().join("swift.flac"))
        .ok()
        .as_deref()
        != Some(&[1, 2, 3])
    {
        return -1;
    }
    let path = access.path().as_os_str().as_encoded_bytes();
    if path.len() > capacity {
        return -2;
    }
    unsafe {
        ptr::copy_nonoverlapping(path.as_ptr(), out, path.len());
    }
    path.len() as isize
}

/// Same-process bridge used by Swift's native-bookmark qualification.
/// # Safety
/// `path` must be a NUL-terminated string; `out` must be valid for `capacity` bytes and not overlap it.
#[cfg(feature = "interop-test")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lw_bookmark_interop_create(
    path: *const c_char,
    out: *mut u8,
    capacity: usize,
) -> isize {
    if path.is_null() || out.is_null() || capacity > isize::MAX as usize {
        return -1;
    }
    let Ok(path) = unsafe { CStr::from_ptr(path) }.to_str() else {
        return -1;
    };
    let Ok(data) = create(Path::new(path)) else {
        return -1;
    };
    if data.len() > capacity {
        return -2;
    }
    unsafe {
        ptr::copy_nonoverlapping(data.as_ptr(), out, data.len());
    }
    data.len() as isize
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_bookmarks_and_nonabsolute_paths_are_rejected() {
        assert!(Access::resolve(&[]).is_err());
        assert!(Access::resolve(&[1, 2, 3]).is_err());
        assert!(create(Path::new("relative/path")).is_err());
        assert!(create(Path::new("/invalid\0path")).is_err());
    }
}
