use super::*;
use lw_apple_sys::player::{Event, Prepared, Snapshot};
use lw_core::player::Ticket;
use lw_ui::browse::Request;
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
pub(super) enum Operation {
    Install {
        ticket: Ticket,
        item: Prepared,
        replace: bool,
    },
    Trim {
        generation: u64,
        current: u64,
        keep: usize,
    },
    Advance {
        generation: u64,
        current: u64,
        next: u64,
    },
    Intent(bool),
    Volume(f32),
    Seek(f64),
    Stop,
    Reset,
}
#[derive(Clone, Copy)]
pub(super) enum Pending {
    Install { ticket: Ticket, replace: bool },
    Trim,
    Advance(Ticket),
    Intent(bool),
    Volume,
    Seek,
    Stop,
    Reset,
}
pub(super) struct Output {
    pub(super) preparation: u64,
    pub(super) operation: Operation,
}
pub(super) struct Ack {
    pub(super) result: io::Result<bool>,
    pub(super) snapshot: Snapshot,
}
pub(super) struct Envelope {
    pub(super) audio_notice: Option<lw_apple_sys::session::Notification>,
    pub(super) revision: u64,
    pub(super) preparation: u64,
    pub(super) command: Command,
}
pub(super) struct Inbox {
    pub(super) catalog: Option<Arc<lw_ui::catalog::Catalog>>,
    pub(super) catalog_pending: bool,
    pub(super) retired_catalog: Option<Arc<lw_ui::catalog::Catalog>>,
    pub(super) commands: VecDeque<Envelope>,
    pub(super) priority: Option<Envelope>,
    pub(super) browse: Option<(u64, Request)>,
    pub(super) events: VecDeque<Event>,
    pub(super) native: Snapshot,
    pub(super) dirty: bool,
    pub(super) ack: Option<Ack>,
    pub(super) output: Option<Output>,
    pub(super) fault: Option<io::Error>,
    pub(super) status: Status,
    pub(super) closed: bool,
    pub(super) stop_requested: bool,
    pub(super) stopped: bool,
}
impl Inbox {
    pub(super) fn has_pending_catalog(&self) -> bool {
        self.catalog_pending
            || self.catalog.is_some()
            || self.browse.is_some()
            || self
                .commands
                .iter()
                .any(|command| command.command.changes_catalog())
    }

    pub(super) fn update_native(
        &mut self,
        batch: &[Option<Event>],
        snapshot: Snapshot,
        fault: Option<io::Error>,
    ) -> io::Result<bool> {
        if self.events.len() + batch.len() > 128 {
            return Err(io::Error::other(
                "Playback event delivery exceeded its bounded queue",
            ));
        }
        let changed = !batch.is_empty() || self.native != snapshot || fault.is_some();
        self.events.extend(batch.iter().flatten().copied());
        if self.native != snapshot {
            self.native = snapshot;
            self.dirty = true;
        }
        if fault.is_some() {
            self.fault = fault;
        }
        Ok(changed)
    }
}
pub(super) struct Shared {
    #[cfg(target_os = "ios")]
    pub(super) audio_session: lw_apple_sys::session::Session,
    pub(super) inbox: Mutex<Inbox>,
    pub(super) changed: Condvar,
    pub(super) cancelled: AtomicBool,
    pub(super) waveform_cancelled: AtomicBool,
    pub(super) preparation: AtomicU64,
    pub(super) wake: extern "C" fn(),
}

impl Shared {
    pub(super) fn begin_background(&self, preparation: u64, token: u64) -> bool {
        let inbox = self.inbox.lock().unwrap();
        if inbox.closed
            || inbox.stop_requested
            || !inbox.commands.is_empty()
            || inbox.priority.is_some()
            || inbox.browse.is_some()
            || !inbox.events.is_empty()
            || inbox.ack.is_some()
            || inbox.fault.is_some()
            || inbox.output.is_some()
            || inbox.native.token != token
            || self.preparation.load(Ordering::SeqCst) != preparation
        {
            return false;
        }
        self.waveform_cancelled.store(false, Ordering::SeqCst);
        true
    }
    pub(super) fn update_catalog(
        &self,
        current: &mut Arc<lw_ui::catalog::Catalog>,
        changed: &mut dyn FnMut(&lw_ui::catalog::Catalog, &lw_ui::catalog::Catalog),
    ) -> bool {
        let catalog = {
            let mut inbox = self.inbox.lock().unwrap();
            if !inbox.closed && inbox.retired_catalog.is_none() {
                inbox.catalog.take()
            } else {
                None
            }
        };
        if let Some(catalog) = catalog {
            changed(current, &catalog);
            let old = std::mem::replace(current, catalog);
            self.inbox.lock().unwrap().retired_catalog = Some(old);
            self.changed.notify_one();
            true
        } else {
            false
        }
    }
}
