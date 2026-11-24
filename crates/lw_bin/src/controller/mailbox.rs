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
#[derive(Default)]
pub(super) struct Feedback {
    pub(super) ack: Option<Ack>,
    pub(super) stopped: bool,
    pub(super) retired_catalog: Option<Arc<lw_ui::catalog::Catalog>>,
}
impl Feedback {
    pub(super) fn pending(&self) -> bool {
        self.ack.is_some() || self.stopped || self.retired_catalog.is_some()
    }
    pub(super) fn flush(&mut self, inbox: &mut Inbox) -> bool {
        let changed = self.pending();
        if let Some(ack) = self.ack.take() {
            debug_assert!(inbox.ack.is_none());
            inbox.ack = Some(ack);
            inbox.dirty = true;
        }
        if std::mem::take(&mut self.stopped) {
            inbox.stopped = true;
        }
        if let Some(retired) = self.retired_catalog.take() {
            debug_assert!(inbox.retired_catalog.is_none());
            inbox.retired_catalog = Some(retired);
        }
        changed
    }
    pub(super) fn adopt(
        &mut self,
        current: &mut Arc<lw_ui::catalog::Catalog>,
        next: Option<Arc<lw_ui::catalog::Catalog>>,
        changed: &mut dyn FnMut(&lw_ui::catalog::Catalog, &lw_ui::catalog::Catalog),
    ) -> bool {
        if let Some(next) = next {
            changed(current, &next);
            debug_assert!(self.retired_catalog.is_none());
            self.retired_catalog = Some(std::mem::replace(current, next));
            true
        } else {
            false
        }
    }
}
pub(super) struct Poll {
    pub(super) stop_requested: bool,
    pub(super) stopped: bool,
    pub(super) output: Option<Output>,
    pub(super) catalog: Option<Arc<lw_ui::catalog::Catalog>>,
    pub(super) status: Option<Status>,
    pub(super) worker_changed: bool,
}
pub(super) struct Envelope {
    pub(super) audio_notice: Option<lw_apple_sys::session::Notification>,
    pub(super) revision: u64,
    pub(super) preparation: u64,
    pub(super) command: Command,
}
pub(super) struct Inbox {
    pub(super) catalog: Option<Arc<lw_ui::catalog::Catalog>>,
    pub(super) presented_catalog: Arc<lw_ui::catalog::Catalog>,
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
    fn take_catalog(
        &mut self,
        current: &Arc<lw_ui::catalog::Catalog>,
    ) -> Option<Arc<lw_ui::catalog::Catalog>> {
        if self.closed || self.retired_catalog.is_some() {
            return None;
        }
        let next = self.catalog.take();
        if let Some(next) = &next {
            debug_assert!(Arc::ptr_eq(&self.presented_catalog, current));
            // Reserve the new base before repair: a worker can publish during the callback.
            self.presented_catalog = next.clone();
        }
        next
    }
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
    /// Worker only: large remaps and superseded deliveries are prepared/dropped outside the lock.
    pub(super) fn publish_catalog(&self, catalog: &lw_ui::catalog::Catalog) {
        loop {
            let (base, remap) = {
                let inbox = self.inbox.lock().unwrap();
                let base = inbox.presented_catalog.clone();
                let remap = inbox
                    .catalog
                    .as_ref()
                    .and_then(|pending| pending.remap.as_ref())
                    .filter(|remap| remap.matches(&base, catalog))
                    .cloned();
                (base, remap)
            };
            let mut next = catalog.clone();
            if let Some(remap) = remap {
                next.remap = Some(remap);
            } else {
                next.prepare_remap(&base);
            }
            let next = Arc::new(next);
            let mut inbox = self.inbox.lock().unwrap();
            if !base.same_page(&inbox.presented_catalog) {
                drop(inbox);
                continue;
            }
            let previous = inbox.catalog.replace(next);
            #[cfg(test)]
            self.changed.notify_all();
            drop(inbox);
            drop(previous);
            (self.wake)();
            return;
        }
    }
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
    #[allow(clippy::too_many_arguments)] // One bounded mailbox exchange; native work stays outside it.
    pub(super) fn exchange(
        &self,
        feedback: &mut Feedback,
        batch: &[Option<Event>],
        snapshot: Snapshot,
        fault: Option<io::Error>,
        apply_output: bool,
        current: &Arc<lw_ui::catalog::Catalog>,
        revision: u64,
    ) -> io::Result<Poll> {
        let mut inbox = self.inbox.lock().unwrap();
        if inbox.native.token != snapshot.token || !batch.is_empty() || fault.is_some() {
            self.waveform_cancelled.store(true, Ordering::SeqCst);
        }
        let worker_changed = inbox.update_native(batch, snapshot, fault)?;
        let worker_changed = feedback.flush(&mut inbox) || worker_changed;
        let stop_requested = inbox.stop_requested;
        let output = if apply_output || stop_requested {
            inbox.output.take()
        } else {
            None
        };
        let catalog = if apply_output {
            inbox.take_catalog(current)
        } else {
            None
        };
        Ok(Poll {
            stop_requested,
            stopped: inbox.stopped,
            output,
            catalog,
            status: (inbox.status.revision != revision).then(|| inbox.status.clone()),
            worker_changed,
        })
    }
}
