//! Main-thread window chrome; read geometry after native size changes, never during a UI build.
use native_layer_token_sys::portable::Token;
use std::{ffi::c_void, io, marker::PhantomData, rc::Rc};

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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub top_inset: f64,
    pub content_width: f64,
    pub content_height: f64,
    pub resizable: u32,
    pub zoom_enabled: u32,
    pub full_size_content: u32,
    pub full_screen_primary: u32,
}

pub struct Chrome {
    layer: Token,
    _main: PhantomData<Rc<()>>,
}
impl Chrome {
    pub fn open(layer: Token) -> io::Result<Self> {
        let raw = layer
            .borrow_raw(layer.owner_id(), layer.generation())
            .ok_or_else(unavailable)?;
        if unsafe { lw_window_configure(raw.as_ptr()) } == 0 {
            return Err(unavailable());
        }
        Ok(Self {
            layer,
            _main: PhantomData,
        })
    }
    pub fn snapshot(&self) -> io::Result<Snapshot> {
        let raw = self
            .layer
            .borrow_raw(self.layer.owner_id(), self.layer.generation())
            .ok_or_else(unavailable)?;
        let mut state = Snapshot::default();
        if unsafe { lw_window_read(raw.as_ptr(), &mut state) } == 0
            || !state.top_inset.is_finite()
            || state.top_inset < 0.0
        {
            return Err(unavailable());
        }
        Ok(state)
    }
}
fn unavailable() -> io::Error {
    io::Error::other("Window chrome needs a live main-thread AppKit view")
}
unsafe extern "C" {
    fn lw_window_configure(layer: *mut c_void) -> i32;
    fn lw_window_read(layer: *mut c_void, state: *mut Snapshot) -> i32;
}
