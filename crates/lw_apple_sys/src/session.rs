//! Cached audio-session state and bounded notifications. Blocking activation stays on a worker.
use crate::bookmarks::failure;
use std::{
    ffi::{c_char, c_void},
    io,
    ptr::{self, NonNull},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Began,
    Ended { resume: bool },
    OutputUnavailable,
    Overflow,
    Reset,
}
impl Event {
    pub fn pauses(self) -> bool {
        !matches!(self, Self::Ended { .. })
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RawEvent {
    sequence: u64,
    kind: u32,
    resume: u32,
}
#[derive(Clone)]
pub struct Notification {
    owner: Arc<Handle>,
    sequence: u64,
    pub event: Event,
}
impl Notification {
    /// Call after applying this notice on the serialized model worker, including pause intent.
    pub fn mark_processed(self) {
        unsafe { lw_session_processed(self.owner.0.as_ptr(), self.sequence) };
    }
}
struct Handle(NonNull<c_void>);
// Native locks serialize state; OS callbacks retain no Rust references.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { lw_session_release(self.0.as_ptr()) };
    }
}
#[derive(Clone)]
pub struct Session(Arc<Handle>);
impl Session {
    /// Wake only signals the owner; it may run on a producer while activation is in progress.
    pub fn open(wake: extern "C" fn()) -> io::Result<Self> {
        NonNull::new(unsafe { lw_session_create(wake) })
            .map(|raw| Self(Arc::new(Handle(raw))))
            .ok_or_else(|| io::Error::other("Could not observe the audio session"))
    }
    /// False means an unprocessed pause/reset or notification overflow fenced activation.
    pub fn activate(&self) -> io::Result<bool> {
        crate::require_worker_thread()?;
        let mut error = ptr::null_mut();
        match unsafe { lw_session_activate(self.0.0.as_ptr(), &mut error) } {
            1 => Ok(true),
            0 => Ok(false),
            _ => Err(failure(error)),
        }
    }
    pub fn peek(&self) -> io::Result<Option<Notification>> {
        let mut raw = RawEvent::default();
        if unsafe { lw_session_peek(self.0.0.as_ptr(), &mut raw) } == 0 {
            return Ok(None);
        }
        if raw.sequence == 0 || raw.resume > 1 {
            return Err(io::Error::other("Invalid audio-session notification"));
        }
        let event = match raw.kind {
            1 => Event::Began,
            2 => Event::Ended {
                resume: raw.resume != 0,
            },
            3 => Event::OutputUnavailable,
            4 => Event::Overflow,
            5 => Event::Reset,
            _ => return Err(io::Error::other("Invalid audio-session notification")),
        };
        Ok(Some(Notification {
            owner: self.0.clone(),
            sequence: raw.sequence,
            event,
        }))
    }
    pub fn acknowledge(&self, notification: Notification) {
        if !Arc::ptr_eq(&self.0, &notification.owner) {
            return;
        }
        unsafe { lw_session_ack(self.0.0.as_ptr(), notification.sequence) };
    }
    #[cfg(feature = "interop-test")]
    pub fn inject(&self, event: Event) {
        let (kind, resume) = match event {
            Event::Began => (1, 0),
            Event::Ended { resume } => (2, i32::from(resume)),
            Event::OutputUnavailable => (3, 0),
            Event::Overflow => (4, 0),
            Event::Reset => (5, 0),
        };
        unsafe { lw_session_test_push(self.0.0.as_ptr(), kind, resume) };
    }
}
unsafe extern "C" {
    fn lw_session_create(wake: extern "C" fn()) -> *mut c_void;
    fn lw_session_activate(session: *mut c_void, error: *mut *mut c_char) -> i32;
    fn lw_session_peek(session: *mut c_void, event: *mut RawEvent) -> i32;
    fn lw_session_ack(session: *mut c_void, sequence: u64);
    fn lw_session_release(session: *mut c_void);
    fn lw_session_processed(session: *mut c_void, sequence: u64);
    #[cfg(feature = "interop-test")]
    fn lw_session_test_push(session: *mut c_void, kind: u32, resume: i32);
}
#[cfg(feature = "interop-test")]
pub fn verify_activation() -> i32 {
    unsafe extern "C" {
        fn lw_session_test_activation() -> i32;
    }
    unsafe { lw_session_test_activation() }
}

#[cfg(feature = "interop-test")]
pub fn verify_notifications() -> i32 {
    unsafe extern "C" {
        fn lw_session_test_notifications() -> i32;
    }
    unsafe { lw_session_test_notifications() }
}

#[cfg(all(test, feature = "interop-test"))]
mod tests {
    use super::*;
    #[test]
    fn session_queue_preserves_backpressure_overflow_tokens_and_warm_storage() {
        assert_eq!(std::mem::size_of::<RawEvent>(), 16);
        assert_eq!(std::mem::offset_of!(RawEvent, kind), 8);
        assert_eq!(std::mem::offset_of!(RawEvent, resume), 12);
        extern "C" fn wake() {}
        let session = Session::open(wake).unwrap();
        let other = Session::open(wake).unwrap();
        session.inject(Event::Began);
        other.inject(Event::OutputUnavailable);
        other.acknowledge(session.peek().unwrap().unwrap());
        assert_eq!(
            other.peek().unwrap().unwrap().event,
            Event::OutputUnavailable
        );
        other.acknowledge(other.peek().unwrap().unwrap());
        session.acknowledge(session.peek().unwrap().unwrap());
        drop(other);
        for round in 0..100 {
            mem_sys::local_reset();
            for event in [
                Event::Began,
                Event::Ended { resume: true },
                Event::OutputUnavailable,
                Event::Reset,
            ] {
                session.inject(event);
                let first = session.peek().unwrap().unwrap();
                assert_eq!(first.event, event);
                assert_eq!(session.peek().unwrap().unwrap().sequence, first.sequence);
                session.acknowledge(first.clone());
                first.mark_processed();
                assert!(session.peek().unwrap().is_none());
            }
            session.inject(Event::Began);
            let old = session.peek().unwrap().unwrap();
            for _ in 0..32 {
                session.inject(Event::Ended { resume: true });
            }
            session.acknowledge(old);
            let overflow = session.peek().unwrap().unwrap();
            assert_eq!(overflow.event, Event::Overflow);
            session.acknowledge(overflow.clone());
            overflow.mark_processed();
            assert!(session.peek().unwrap().is_none());
            if round > 0 {
                assert_eq!(mem_sys::local_alloc_count(), 0);
            }
        }
        let producers: Vec<_> = (0..4)
            .map(|_| {
                let session = session.clone();
                std::thread::spawn(move || {
                    for _ in 0..32 {
                        session.inject(Event::Began);
                        session.inject(Event::Ended { resume: true });
                    }
                })
            })
            .collect();
        for producer in producers {
            producer.join().unwrap();
        }
        let overflow = session.peek().unwrap().unwrap();
        assert_eq!(overflow.event, Event::Overflow);
        session.acknowledge(overflow);
        assert!(session.peek().unwrap().is_none());
        std::thread::spawn(|| assert_eq!(verify_activation(), 0))
            .join()
            .unwrap();
    }
}
