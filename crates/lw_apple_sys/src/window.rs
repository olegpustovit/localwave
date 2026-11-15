//! Open external help links on the main thread.
use std::io;

pub enum Link {
    PrivacyPolicy,
    Support,
}
pub fn open_link(link: Link) -> io::Result<()> {
    unsafe extern "C" {
        fn lw_window_open_link(kind: i32) -> i32;
    }
    if unsafe {
        lw_window_open_link(match link {
            Link::PrivacyPolicy => 0,
            Link::Support => 1,
        })
    } == 0
    {
        Err(io::Error::other("Could not open this link"))
    } else {
        Ok(())
    }
}

/// Select a file in Finder. Callable from any thread; the caller must hold the source's grant
/// and pass the path it resolved. macOS only: iOS has no Finder, so it reports an error.
pub fn reveal(path: &std::path::Path) -> io::Result<()> {
    unsafe extern "C" {
        fn lw_window_reveal_file(path: *const std::ffi::c_char) -> i32;
    }
    let text = path
        .to_str()
        .and_then(|text| std::ffi::CString::new(text).ok())
        .ok_or_else(|| io::Error::other("File path is not valid"))?;
    if unsafe { lw_window_reveal_file(text.as_ptr()) } == 0 {
        Err(io::Error::other("Could not show this file"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod link_tests {
    #[test]
    fn external_help_links_reject_worker_threads_without_launching_a_browser() {
        std::thread::spawn(|| {
            assert!(super::open_link(super::Link::PrivacyPolicy).is_err());
            assert!(super::open_link(super::Link::Support).is_err());
        })
        .join()
        .unwrap();
    }
}
