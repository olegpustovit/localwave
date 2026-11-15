//! The initial thread owns the native player; this bridge owns one operation/acknowledgement, 32
//! ordinary commands, a reserved Pause/Stop slot and 128 critical events. Queue/model mutations,
//! history snapshots and display strings belong on the worker. Keep current plus two upcoming
//! native items and preserve buffered items on append. Worker errors wait for native stop
//! acknowledgement before clearing leases.
//!
//! Preserve logical queue selection while native advance awaits acknowledgement. Coalesce
//! volume/seek without cancelling preparation; queue changes fence operations/preparation. Browse
//! has its own coalesced fixed-capacity slot, independent of playback capacity, and never fences
//! preparation. Drive acknowledgements/preparation before sorting; revisions and preparation
//! identities never decrease when a reserved control follows a newer ordinary control. Alternate
//! continuous volume/seek with ready current/upcoming preparation and alternate both control kinds.
//! Failed/deferred/cancelled preparation must still allow controls to progress; reset/stop/queue
//! fencing retains priority.
//!
//! Catalog refresh coalesces without fencing preparation. Keep at most one incoming and one retired
//! catalog. Repair route/selection before adoption, outside the inbox mutex; never consume catalogs
//! during command delivery. Superseded/retired catalogs drop on the worker outside the mutex.
//! Preserve retirement backpressure and wake a waiting replacement after draining. Unchanged native
//! polls must not wake the worker; notify for delivered events, changed snapshots, faults,
//! acknowledgements, commands, stop and retirement.
//! Prepare remaps against the displayed catalog, not an undelivered worker result. Reserve the new
//! displayed base under the take lock before UI repair, so concurrent publications use that base.
//! Retain only a shared owner of the current catalog; remaps weakly identify their source/target
//! pages and never keep retired label arenas alive. Bulk remap work and drops stay on the worker.
//!
//! Status carries 512 peaks inline. Clear stale peaks on selection/catalog identity changes and
//! revalidate after materialization token changes. Background waveform/copy cancellation resets
//! under the inbox lock, after playback work. Explicit refresh invalidates current metadata before
//! replacement peaks.
//! Every changed status advances revision for delivery/media; stable_revision excludes position
//! ticks so retained library/metadata views can stay cached. All other status facts invalidate it.
//!
//! Audio-session delivery is bounded to 32 notices per poll. Blocking category/activation stays on
//! the worker; callbacks cache facts and wake. Preserve configure/activate epochs, reset
//! invalidation, owner/sequence acknowledgement and fail-closed overflow. Pause immediately on the
//! main thread under command backpressure. Manual pause/stop/route loss cancels interruption
//! resume; repeated begins retain intent. Only processing, not delivery acknowledgement, releases
//! activation fences; an old notice cannot release a newer pause. Explicit Play may retry without
//! an end notice.
//!
//! Bounded main-thread/native and worker/model bridge. No database or file work runs in `poll`.
use crate::library::LibraryWorker;
use lw_apple_sys::player::{Event, Kind, Player, Snapshot};
use lw_apple_sys::session::Event as AudioEvent;
use lw_core::context::PlaybackContext;
use lw_core::playback::{Phase, Reading};
use lw_core::player::{Action, Ticket};
use lw_ui::browse::Request;
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct Controller {
    catalog: Arc<lw_ui::catalog::Catalog>,
    native: Player,
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
    revision: u64,
    preparation: u64,
    status: Status,
    faulted: bool,
    media: Option<crate::media::Media>,
}
impl Controller {
    pub fn open(
        factory: impl FnOnce() -> io::Result<LibraryWorker> + Send + 'static,
        wake: extern "C" fn(),
    ) -> io::Result<Self> {
        let mut native = Player::open()?;
        native.set_wake(Some(wake));
        let status = Status::default();
        let catalog = Arc::new(lw_ui::catalog::Catalog::default());
        let shared = Arc::new(Shared {
            #[cfg(target_os = "ios")]
            audio_session: lw_apple_sys::session::Session::open(wake)?,
            inbox: Mutex::new(Inbox {
                catalog: None,
                presented_catalog: catalog.clone(),
                catalog_pending: true,
                retired_catalog: None,
                commands: VecDeque::with_capacity(32),
                priority: None,
                browse: None,
                events: VecDeque::with_capacity(128),
                native: native.snapshot(),
                dirty: false,
                ack: None,
                output: None,
                fault: None,
                status: status.clone(),
                closed: false,
                stop_requested: false,
                stopped: false,
            }),
            changed: Condvar::new(),
            cancelled: AtomicBool::new(false),
            waveform_cancelled: AtomicBool::new(false),
            preparation: AtomicU64::new(0),
            wake,
        });
        let worker_shared = shared.clone();
        let worker = std::thread::Builder::new()
            .name("localwave-library".into())
            .spawn(move || Worker::run(factory, worker_shared))?;
        Ok(Self {
            catalog,
            native,
            shared,
            worker: Some(worker),
            revision: 0,
            preparation: 0,
            status,
            faulted: false,
            media: None,
        })
    }
    pub fn enable_media_controls(&mut self, publish: bool) -> io::Result<()> {
        if self.media.is_none() {
            self.media = Some(crate::media::Media::open(publish, self.shared.wake)?);
        }
        Ok(())
    }
    pub fn media_controls(&mut self) -> Option<&mut crate::media::Media> {
        self.media.as_mut()
    }
    pub fn sync_media_artwork<'a>(
        &mut self,
        epoch: u64,
        ready: impl Iterator<Item = &'a crate::library::ArtworkReady>,
    ) -> io::Result<()> {
        if let Some(media) = &mut self.media {
            media.artwork(&self.catalog, epoch, ready)?;
        }
        Ok(())
    }
    pub fn status(&self) -> &Status {
        &self.status
    }
    pub fn catalog(&self) -> &lw_ui::catalog::Catalog {
        &self.catalog
    }
    /// Queued, building or undelivered catalog data. Used at qualification boundaries, not frames.
    pub fn catalog_pending(&self) -> bool {
        self.shared.inbox.lock().unwrap().has_pending_catalog()
    }
    pub fn native_snapshot(&self) -> Snapshot {
        self.native.snapshot()
    }
    pub fn command(&mut self, command: Command) -> io::Result<u64> {
        self.command_with_notice(command, None)
    }
    fn command_with_notice(
        &mut self,
        command: Command,
        audio_notice: Option<lw_apple_sys::session::Notification>,
    ) -> io::Result<u64> {
        command.validate()?;
        self.poll_inner(false, &mut |_, _| {})?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Playback command identity exhausted"))?;
        let interrupts = command.interrupts();
        let fences_preparation = command.fences_preparation();
        let preparation = if fences_preparation {
            revision
        } else {
            self.preparation
        };
        {
            let mut inbox = self.shared.inbox.lock().unwrap();
            if inbox.closed || inbox.stop_requested {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            let priority = matches!(command, Command::Pause | Command::Stop);
            inbox.commands.retain(|pending| {
                !matches!(
                    (&command, &pending.command),
                    (Command::Volume(_), Command::Volume(_))
                        | (Command::Seek(_), Command::Seek(_))
                        | (Command::RefreshCatalog, Command::RefreshCatalog)
                        | (Command::ReadStorage, Command::ReadStorage)
                )
            });
            if inbox.commands.len() == 32 && !priority {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            let mut envelope = Envelope {
                audio_notice,
                revision,
                preparation,
                command,
            };
            if inbox.commands.len() == 32 {
                if matches!(
                    inbox.priority.as_ref().map(|pending| &pending.command),
                    Some(Command::Stop)
                ) && matches!(envelope.command, Command::Pause)
                {
                    envelope.command = Command::Stop;
                }
                inbox.priority = Some(envelope);
            } else {
                inbox.commands.push_back(envelope);
            }
            self.shared.waveform_cancelled.store(true, Ordering::SeqCst);
            self.revision = revision;
            if fences_preparation {
                self.preparation = preparation;
                self.shared.preparation.store(preparation, Ordering::SeqCst);
                self.shared.cancelled.store(true, Ordering::SeqCst);
            }
        }
        if interrupts {
            self.native.pause();
        }
        self.shared.changed.notify_one();
        Ok(revision)
    }
    /// Keep only the latest browse packet, independently of the bounded playback queue.
    pub fn browse(&mut self, request: Request) -> io::Result<u64> {
        self.poll_inner(false, &mut |_, _| {})?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Playback command identity exhausted"))?;
        {
            let mut inbox = self.shared.inbox.lock().unwrap();
            if inbox.closed || inbox.stop_requested {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            inbox.browse = Some((revision, request));
            self.shared.waveform_cancelled.store(true, Ordering::SeqCst);
            self.revision = revision;
        }
        self.shared.changed.notify_one();
        Ok(revision)
    }
    /// Main-thread event delivery and acknowledgements only; retained buffers bound every message.
    pub fn poll(&mut self) -> io::Result<bool> {
        self.poll_with_catalog(&mut |_, _| {})
    }
    /// Repair view state while both snapshots are borrowed; old catalog storage retires on the worker.
    pub fn poll_with_catalog(
        &mut self,
        changed: &mut dyn FnMut(&lw_ui::catalog::Catalog, &lw_ui::catalog::Catalog),
    ) -> io::Result<bool> {
        let revision = self.status.revision;
        #[cfg(target_os = "ios")]
        let blocked = self.deliver_audio_session()?;
        #[cfg(not(target_os = "ios"))]
        let blocked = false;
        let updated = self.poll_inner(!blocked, changed)?;
        if let Some(mut media) = self.media.take() {
            let result = media.deliver(self).and_then(|_| media.sync(&self.status));
            self.media = Some(media);
            result?;
        }
        Ok(updated || revision != self.status.revision)
    }
    #[cfg(target_os = "ios")]
    fn deliver_audio_session(&mut self) -> io::Result<bool> {
        let session = self.shared.audio_session.clone();
        for _ in 0..32 {
            let Some(notification) = session.peek()? else {
                return Ok(false);
            };
            if notification.event.pauses() {
                self.native.pause();
                self.shared.cancelled.store(true, Ordering::SeqCst);
                self.shared.waveform_cancelled.store(true, Ordering::SeqCst);
            }
            match self.command_with_notice(
                Command::AudioSession(notification.event),
                Some(notification.clone()),
            ) {
                Ok(_) => session.acknowledge(notification),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(true),
                Err(error) => return Err(error),
            }
        }
        Ok(session.peek()?.is_some())
    }
    fn poll_inner(
        &mut self,
        apply_output: bool,
        changed: &mut dyn FnMut(&lw_ui::catalog::Catalog, &lw_ui::catalog::Catalog),
    ) -> io::Result<bool> {
        self.native.run_loop(Duration::ZERO);
        let stop_requested = self.shared.inbox.lock().unwrap().stop_requested;
        if stop_requested {
            self.native.stop();
            let mut inbox = self.shared.inbox.lock().unwrap();
            inbox.output.take();
            inbox.stopped = true;
            self.shared.changed.notify_one();
        }
        let mut batch = [None; 64];
        let mut count = 0;
        let mut fault = None;
        while count < batch.len() {
            match self.native.event() {
                Ok(Some(event)) => {
                    if event.kind == Kind::EngineFailed {
                        self.faulted = true;
                        self.native.pause();
                    }
                    if matches!(
                        event.kind,
                        Kind::Started | Kind::Finished | Kind::Failed | Kind::EngineFailed
                    ) {
                        batch[count] = Some(event);
                        count += 1;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    fault = Some(error);
                    self.native.stop();
                    self.faulted = true;
                    break;
                }
            }
        }
        let snapshot = self.native.snapshot();
        let (output, mut worker_changed) = {
            let mut inbox = self.shared.inbox.lock().unwrap();
            if inbox.native.token != snapshot.token || count != 0 || fault.is_some() {
                self.shared.waveform_cancelled.store(true, Ordering::SeqCst);
            }
            let worker_changed = match inbox.update_native(&batch[..count], snapshot, fault) {
                Ok(changed) => changed,
                Err(error) => {
                    self.native.pause();
                    return Err(error);
                }
            };
            let output = if apply_output {
                inbox.output.take()
            } else {
                None
            };
            (output, worker_changed)
        };
        if let Some(output) = output {
            let result = if output.preparation != self.preparation {
                Ok(false)
            } else if self.faulted
                && !matches!(output.operation, Operation::Reset | Operation::Stop)
            {
                Err(io::Error::other(
                    "The audio engine must be reset before retrying",
                ))
            } else {
                self.apply(output.operation)
            };
            let snapshot = self.native.snapshot();
            let mut inbox = self.shared.inbox.lock().unwrap();
            debug_assert!(inbox.ack.is_none());
            inbox.native = snapshot;
            inbox.dirty = true;
            inbox.ack = Some(Ack { result, snapshot });
            worker_changed = true;
        }
        if worker_changed {
            self.shared.changed.notify_one();
        }
        let catalog_changed =
            apply_output && self.shared.update_catalog(&mut self.catalog, changed);
        let inbox = self.shared.inbox.lock().unwrap();
        if inbox.status.revision == self.status.revision {
            return Ok(catalog_changed);
        }
        self.status = inbox.status.clone();
        Ok(true)
    }
    fn apply(&mut self, operation: Operation) -> io::Result<bool> {
        match operation {
            Operation::Install {
                ticket,
                item,
                replace,
            } => {
                if replace {
                    self.native.replace(ticket.generation, item)?;
                } else {
                    return self.native.append(
                        ticket.generation,
                        self.native.snapshot().token,
                        &item,
                    );
                }
            }
            Operation::Trim {
                generation,
                current,
                keep,
            } => return self.native.trim(generation, current, keep),
            Operation::Advance {
                generation,
                current,
                next,
            } => {
                let actual = self.native.snapshot();
                if actual.generation != generation
                    || actual.token != current
                    || actual.queued[1] != next
                {
                    return Ok(false);
                }
                self.native.advance();
            }
            Operation::Intent(playing) => {
                if playing {
                    self.native.play();
                } else {
                    self.native.pause();
                }
            }
            Operation::Volume(volume) => self.native.set_volume(volume)?,
            Operation::Seek(seconds) => self.native.seek(seconds)?,
            Operation::Stop => self.native.stop(),
            Operation::Reset => {
                self.native.stop();
                let mut player = Player::open()?;
                player.set_wake(Some(self.shared.wake));
                self.native = player;
                self.faulted = false;
            }
        }
        Ok(true)
    }
    /// Playing progress is sampled; paused and empty players wake the host through native events.
    pub fn poll_interval(&self) -> Option<Duration> {
        self.status
            .wants_playback
            .then_some(Duration::from_millis(50))
    }
    pub fn run_loop(&mut self, wait: Duration) {
        self.native.run_loop(wait);
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        self.media.take();
        self.native.stop();
        self.shared.cancelled.store(true, Ordering::SeqCst);
        self.shared.waveform_cancelled.store(true, Ordering::SeqCst);
        {
            let mut inbox = self.shared.inbox.lock().unwrap();
            inbox.closed = true;
            inbox.output.take();
        }
        self.shared.changed.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

mod types;
pub use types::{Command, Status};
mod mailbox;
use mailbox::*;
mod worker;
use worker::*;

mod background;
mod commands;
mod state;

#[cfg(test)]
mod tests;
