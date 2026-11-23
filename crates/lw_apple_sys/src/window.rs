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

/// Whether this platform has the audio route key: AVKit's route picker on iOS and macOS (Swift's shipping Mac app is its iOS
/// build, which shows the key).
pub const ROUTE_PICKER: bool = cfg!(any(target_os = "ios", target_os = "macos"));

/// Present the system audio route picker from the key at `anchor` (window points, top-left origin). Main thread; an error when no
/// window or picker button is there.
pub fn route_picker(anchor: (f32, f32)) -> io::Result<()> {
    unsafe extern "C" {
        fn lw_window_route_picker(x: f64, y: f64) -> i32;
    }
    if unsafe { lw_window_route_picker(f64::from(anchor.0), f64::from(anchor.1)) } == 0 {
        Err(io::Error::other("Could not show audio outputs"))
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

/// Show the system share picker for prepared files, anchored at a window point (top-left origin).
/// Callable from any thread; the files must already be copies the process may read.
pub fn share(paths: &[std::path::PathBuf], anchor: (f32, f32)) -> io::Result<()> {
    unsafe extern "C" {
        fn lw_window_share_files(
            paths: *const *const std::ffi::c_char,
            count: usize,
            x: f64,
            y: f64,
        ) -> i32;
    }
    let texts = paths
        .iter()
        .map(|path| {
            path.to_str()
                .and_then(|text| std::ffi::CString::new(text).ok())
                .ok_or_else(|| io::Error::other("File path is not valid"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    let pointers: Vec<_> = texts.iter().map(|text| text.as_ptr()).collect();
    let (x, y) = (f64::from(anchor.0), f64::from(anchor.1));
    if unsafe { lw_window_share_files(pointers.as_ptr(), pointers.len(), x, y) } == 0 {
        Err(io::Error::other("Could not share these files"))
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
