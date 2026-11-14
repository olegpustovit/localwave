//! Worker-owned immutable strings for Apple's natural, localized desktop column ordering.
use std::cmp::Ordering;
use std::ffi::c_void;
use std::io;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

unsafe extern "C" {
    fn lw_collation_create(bytes: *const u8, len: usize) -> *mut c_void;
    fn lw_collation_compare(left: *mut c_void, right: *mut c_void) -> i32;
    fn lw_collation_release(key: *mut c_void);
    fn lw_collation_contains(needle: *mut c_void, bytes: *const u8, len: usize) -> i32;
    fn lw_collation_contains_case_insensitive(
        needle: *mut c_void,
        bytes: *const u8,
        len: usize,
    ) -> i32;
}

pub struct Key {
    raw: NonNull<c_void>,
    _thread: PhantomData<Rc<()>>,
}
impl Key {
    pub fn new(text: &str) -> io::Result<Self> {
        crate::require_worker_thread()?;
        let raw = NonNull::new(unsafe { lw_collation_create(text.as_ptr(), text.len()) })
            .ok_or_else(|| io::Error::other("Could not prepare a sort key"))?;
        Ok(Self {
            raw,
            _thread: PhantomData,
        })
    }
    pub fn compare(&self, other: &Self) -> Ordering {
        unsafe { lw_collation_compare(self.raw.as_ptr(), other.raw.as_ptr()) }.cmp(&0)
    }
    pub fn contained_in(&self, value: &str) -> bool {
        unsafe { lw_collation_contains(self.raw.as_ptr(), value.as_ptr(), value.len()) != 0 }
    }
    /// Foundation's localized case-insensitive search preserves diacritic distinctions.
    pub fn contained_in_case_insensitive(&self, value: &str) -> bool {
        unsafe {
            lw_collation_contains_case_insensitive(self.raw.as_ptr(), value.as_ptr(), value.len())
                != 0
        }
    }
}
impl Drop for Key {
    fn drop(&mut self) {
        unsafe { lw_collation_release(self.raw.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn natural_sort_keys_keep_numeric_order_and_reuse_warm_rust_storage() {
        let two = Key::new("Track 2").unwrap();
        let ten = Key::new("Track 10").unwrap();
        let empty = Key::new("").unwrap();
        assert_eq!(two.compare(&ten), Ordering::Less);
        assert_eq!(empty.compare(&two), Ordering::Less);
        let cafe = Key::new("cafe").unwrap();
        assert!(cafe.contained_in("CAFÉ del Mar"));
        assert!(cafe.contained_in("Cafe\u{301}"));
        assert!(!cafe.contained_in("Tea"));
        assert!(!cafe.contained_in_case_insensitive("CAFÉ"));
        let accented = Key::new("café").unwrap();
        assert!(accented.contained_in_case_insensitive("CAFÉ"));
        assert!(accented.contained_in_case_insensitive("Cafe\u{301}"));
        mem_sys::local_reset();
        for _ in 0..100 {
            assert_eq!(two.compare(&ten), Ordering::Less);
            assert_eq!(ten.compare(&two), Ordering::Greater);
            assert!(cafe.contained_in("CAFÉ del Mar"));
            assert!(!cafe.contained_in_case_insensitive("CAFÉ"));
            assert!(accented.contained_in_case_insensitive("Cafe\u{301}"));
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
}
