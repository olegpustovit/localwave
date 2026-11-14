//! Main-thread window chrome; read geometry after native size changes, never during a UI build.
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
