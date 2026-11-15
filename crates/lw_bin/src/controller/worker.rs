use super::*;
use crate::library::{LibraryWorker, database_error};
use lw_apple_sys::player::{Kind, Snapshot};
use lw_core::context::PlaybackContext;
use lw_core::playback::{Fault, Phase};
use lw_core::player::{Session, Ticket};
use lw_core::preferences::{PlayerState, Settings};
use lw_core::queue::Entry;
use lw_ui::browse::Request;
use lw_ui::catalog::Catalog;
use std::io;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct Worker {
    pub(super) catalog: Catalog,
    pub(super) queue: Option<Arc<lw_ui::queue::Snapshot>>,
    pub(super) queue_dirty: bool,
    pub(super) browse: Option<Request>,
    pub(super) browse_dirty: bool,
    pub(super) catalog_dirty: bool,
    pub(super) catalog_error: Option<Arc<str>>,
    pub(super) settings: Settings,
    pub(super) cache_usage: Arc<str>,
    pub(super) settings_message: Arc<str>,
    pub(super) storage_pending: bool,
    pub(super) copies_pending: bool,
    pub(super) context: Arc<PlaybackContext>,
    pub(super) library: LibraryWorker,
    pub(super) shared: Arc<Shared>,
    pub(super) session: Session,
    pub(super) saved: PlayerState,
    pub(super) revision: u64,
    pub(super) preparation: u64,
    pub(super) native: Snapshot,
    pub(super) pending: Option<Pending>,
    pub(super) leases: [Option<Ticket>; 6],
    pub(super) intent: bool,
    pub(super) reset: bool,
    pub(super) seek: Option<f64>,
    pub(super) yield_controls: bool,
    pub(super) volume_last: bool,
    pub(super) advance: Option<(u64, u64)>,
    pub(super) undo: Option<Vec<Entry>>,
    pub(super) metadata_id: Option<i64>,
    pub(super) waveform_key: Option<(i64, u64)>,
    pub(super) waveform: Option<lw_core::waveform::Peaks>,
    pub(super) title: Arc<str>,
    pub(super) artist: Arc<str>,
    pub(super) album: Arc<str>,
    pub(super) message: Arc<str>,
    pub(super) empty: Arc<str>,
    pub(super) status: Status,
    pub(super) active: Option<u64>,
    pub(super) empty_from: Option<u64>,
    pub(super) deferred: [Option<Ticket>; 3],
}
pub(super) fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
impl Worker {
    pub(super) fn run(factory: impl FnOnce() -> io::Result<LibraryWorker>, shared: Arc<Shared>) {
        let setup = || -> io::Result<Self> {
            let mut library = factory()?;
            let settings = library.load_settings();
            let catalog = library.catalog()?;
            shared.inbox.lock().unwrap().catalog = Some(Arc::new(catalog.clone()));
            library.release_all_playback()?;
            let mut saved = library.load_player_state()?;
            let context = Arc::new(library.playback_context(saved.context_data.as_deref()));
            let restored = library
                .repository
                .restore_queue(&saved)
                .map_err(io::Error::other)?;
            let ids: Vec<_> = restored.songs.iter().filter_map(|song| song.id).collect();
            let mut session = Session::default();
            session.configure(&ids, restored.current_index.unwrap_or(0));
            let index = restored.current_index.unwrap_or(0) as i64;
            if saved.song_ids != ids || saved.current_index != index {
                saved.song_ids = ids;
                saved.current_index = index;
                library.save_player_state(&saved)?;
            }
            let copies_pending = catalog.all.as_ref().is_some_and(|all| {
                all.rows.iter().any(|index| {
                    let row = catalog.row(*index);
                    row.state == lw_core::FileState::CopyPending
                        || (row.state == lw_core::FileState::Failed && row.pinned)
                })
            });
            let empty: Arc<str> = Arc::from("");
            let native = shared.inbox.lock().unwrap().native;
            Ok(Self {
                catalog,
                queue: None,
                queue_dirty: true,
                browse: None,
                browse_dirty: false,
                catalog_dirty: false,
                catalog_error: None,
                settings,
                cache_usage: empty.clone(),
                settings_message: empty.clone(),
                storage_pending: false,
                copies_pending,
                context,
                library,
                shared: shared.clone(),
                session,
                saved,
                revision: 0,
                preparation: 0,
                native,
                pending: None,
                leases: [None; 6],
                intent: false,
                reset: false,
                seek: None,
                yield_controls: false,
                volume_last: false,
                advance: None,
                undo: None,
                metadata_id: None,
                waveform_key: None,
                waveform: None,
                title: empty.clone(),
                artist: empty.clone(),
                album: empty.clone(),
                message: empty.clone(),
                empty,
                status: Status::default(),
                active: None,
                empty_from: None,
                deferred: [None; 3],
            })
        };
        let mut worker = match setup() {
            Ok(worker) => worker,
            Err(error) => {
                let mut inbox = shared.inbox.lock().unwrap();
                inbox.status = Status {
                    initialized: true,
                    revision: 1,
                    stable_revision: 1,
                    phase: Phase::Failed,
                    fault: Some(Fault::Session),
                    message: Arc::from(error.to_string()),
                    ..Status::default()
                };
                inbox.closed = true;
                drop(inbox);
                (shared.wake)();
                return;
            }
        };
        let result = worker.work();
        // Controller::drop stops the native engine before closing the inbox and joining this worker.
        if let Err(error) = result {
            worker.session_error(error);
            worker.publish();
            let mut inbox = shared.inbox.lock().unwrap();
            inbox.stop_requested = true;
            (shared.wake)();
            while !inbox.closed && !inbox.stopped {
                inbox = shared.changed.wait(inbox).unwrap();
            }
        }
        let _ = worker.library.release_all_playback();
        let retired = {
            let mut inbox = shared.inbox.lock().unwrap();
            inbox.closed = true;
            inbox.retired_catalog.take()
        };
        drop(retired);
        (shared.wake)();
    }
    pub(super) fn work(&mut self) -> io::Result<()> {
        self.publish();
        loop {
            let mut controls: [Option<Envelope>; 32] = std::array::from_fn(|_| None);
            let mut events = [None; 128];
            let (ack, fault, dirty, snapshot, priority, browse, retired_catalog, catalog_waiting) = {
                let mut inbox = self.shared.inbox.lock().unwrap();
                if inbox.closed {
                    return Ok(());
                }
                for control in &mut controls {
                    *control = inbox.commands.pop_front();
                }
                for event in &mut events {
                    *event = inbox.events.pop_front();
                }
                inbox.catalog_pending |= self.catalog_dirty
                    || self.browse_dirty
                    || inbox.browse.is_some()
                    || controls
                        .iter()
                        .flatten()
                        .any(|command| command.command.changes_catalog());
                let dirty = std::mem::take(&mut inbox.dirty);
                (
                    inbox.ack.take(),
                    inbox.fault.take(),
                    dirty,
                    inbox.native,
                    inbox.priority.take(),
                    inbox.browse.take(),
                    inbox.retired_catalog.take(),
                    inbox.catalog.is_some(),
                )
            };
            let retired = retired_catalog.is_some();
            drop(retired_catalog);
            if retired && catalog_waiting {
                (self.shared.wake)();
            }
            if let Some(ack) = ack {
                self.acknowledge(ack)?;
            }
            // Starts must be recorded before a newer snapshot retires their physical file leases.
            for event in events
                .iter()
                .flatten()
                .filter(|event| event.kind == Kind::Started)
            {
                if self.library.record_playback_start(event.token(), now())? {
                    self.browse_dirty = true;
                }
            }
            if dirty {
                self.observe(snapshot);
            }
            for event in events.into_iter().flatten() {
                self.event(event);
            }
            if let Some(error) = fault {
                self.session_error(error);
            }
            if let Some(generation) = self.empty_from.take()
                && self.native.token == 0
                && generation == self.session.transport().generation()
            {
                self.session.native_empty();
            }
            for envelope in controls.into_iter().flatten() {
                self.command(envelope)?;
            }
            if let Some(priority) = priority {
                self.command(priority)?;
            }
            if let Some((revision, request)) = browse {
                self.revision = self.revision.max(revision);
                self.browse = Some(request);
                self.browse_dirty = true;
            }
            let index = self.session.queue().current_index().unwrap_or(0) as i64;
            if self.saved.current_index != index {
                self.library.save_player_index(index)?;
                self.saved.current_index = index;
            }
            self.retire()?;
            if self.library.take_playback_file_changes() {
                self.dirty_catalog();
            }
            self.publish();
            if self.pending.is_none()
                && self.preparation == self.shared.preparation.load(Ordering::SeqCst)
                && self.drive()?
            {
                continue;
            }
            if self.pending.is_none() {
                self.load_storage();
                let reload = std::mem::take(&mut self.catalog_dirty);
                if std::mem::take(&mut self.browse_dirty) || reload {
                    self.shared.inbox.lock().unwrap().catalog_pending = true;
                    match self
                        .library
                        .catalog_view(if reload { None } else { Some(&self.catalog) }, self.browse)
                    {
                        Ok(mut catalog) => {
                            if !Arc::ptr_eq(&self.catalog.library, &catalog.library) {
                                self.metadata_id = None;
                            }
                            if self
                                .catalog_error
                                .take()
                                .is_some_and(|error| Arc::ptr_eq(&error, &self.message))
                            {
                                self.message = self.empty.clone();
                            }
                            catalog.queue = self.queue.clone();
                            self.catalog = catalog;
                            self.publish_catalog();
                        }
                        Err(error) => {
                            eprintln!("LocalWave catalog refresh: {error}");
                            self.message = Arc::from("Could not refresh the library. Try again.");
                            self.catalog_error = Some(self.message.clone());
                        }
                    }
                }
                self.shared.inbox.lock().unwrap().catalog_pending =
                    self.catalog_dirty || self.browse_dirty;
                self.publish();
                self.load_waveform();
                self.publish();
                if self.load_copies() {
                    continue;
                }
            }
            let mut inbox = self.shared.inbox.lock().unwrap();
            while !inbox.closed
                && inbox.commands.is_empty()
                && inbox.priority.is_none()
                && inbox.browse.is_none()
                && inbox.events.is_empty()
                && inbox.ack.is_none()
                && inbox.fault.is_none()
                && !inbox.dirty
                && inbox.retired_catalog.is_none()
            {
                inbox = self.shared.changed.wait(inbox).unwrap();
            }
        }
    }
    pub(super) fn send(&mut self, operation: Operation, pending: Pending) {
        self.yield_controls = matches!(pending, Pending::Volume | Pending::Seek);
        if matches!(pending, Pending::Volume | Pending::Seek) {
            self.volume_last = matches!(pending, Pending::Volume);
        }
        self.pending = Some(pending);
        let mut inbox = self.shared.inbox.lock().unwrap();
        debug_assert!(inbox.output.is_none());
        inbox.output = Some(Output {
            preparation: self.preparation,
            operation,
        });
        drop(inbox);
        (self.shared.wake)();
    }
    pub(super) fn retire(&mut self) -> io::Result<()> {
        let active = (self.native.token != 0).then_some(self.native.token);
        if active != self.active
            && (active.is_none()
                || self
                    .leases
                    .iter()
                    .flatten()
                    .any(|ticket| Some(ticket.token) == active))
        {
            self.library.set_active_playback(active)?;
            self.active = active;
        }
        let pending = match self.pending {
            Some(Pending::Install { ticket, .. }) => Some(ticket.token),
            _ => None,
        };
        for slot in &mut self.leases {
            if let Some(ticket) = *slot
                && !self.native.queued.contains(&ticket.token)
                && Some(ticket.token) != pending
            {
                self.library.release_playback(ticket.token)?;
                *slot = None;
            }
        }
        Ok(())
    }
    pub(super) fn controls(&mut self) -> bool {
        let volume = self.saved.volume.unwrap_or(1.0);
        if self.native.token != 0
            && (self.volume_last || self.native.volume == volume)
            && let Some(seconds) = self.seek.take()
        {
            self.send(Operation::Seek(seconds), Pending::Seek);
            return true;
        }
        if self.native.volume != volume {
            self.send(Operation::Volume(volume), Pending::Volume);
            return true;
        }
        false
    }
    // Keep native-operation priority and lease preparation in one ordered pass.
    pub(super) fn drive(&mut self) -> io::Result<bool> {
        for deferred in &mut self.deferred {
            if deferred.is_some_and(|ticket| {
                !self.session.expected(ticket) || self.session.current_ticket() == Some(ticket)
            }) {
                *deferred = None;
            }
        }
        let transport = self.session.transport();
        if self.reset && transport.wants_playback() {
            self.send(Operation::Reset, Pending::Reset);
            return Ok(true);
        }
        if self.native.token == 0 && self.intent && !self.session.transport().wants_playback() {
            self.send(Operation::Intent(false), Pending::Intent(false));
            return Ok(true);
        }
        if let Some((generation, token)) = self.advance.take() {
            if self.native.generation == generation && self.native.queued[1] == token {
                let ticket = self
                    .session
                    .planned()
                    .into_iter()
                    .flatten()
                    .find(|ticket| ticket.token == token && ticket.generation == generation)
                    .ok_or_else(|| io::Error::other("Unknown pending native advancement"))?;
                self.send(
                    Operation::Advance {
                        generation,
                        current: self.native.token,
                        next: token,
                    },
                    Pending::Advance(ticket),
                );
                return Ok(true);
            }
            if self.native.token != token
                && let Some(entry) = self
                    .session
                    .planned()
                    .into_iter()
                    .flatten()
                    .find(|ticket| ticket.token == token)
                    .map(|ticket| ticket.entry)
            {
                self.session.invalidate_upcoming();
                let action = self.session.select(entry.id, true);
                self.action(action);
            }
        }
        let active = !matches!(
            self.session.transport().phase(),
            Phase::Stopped | Phase::Failed | Phase::Ended
        );
        let current = self.session.current_ticket().filter(|_| active);
        if self.native.token != 0 && current.is_none_or(|ticket| ticket.token != self.native.token)
        {
            self.send(Operation::Stop, Pending::Stop);
            return Ok(true);
        }
        if self.native.token != 0 {
            let expected: [Option<Ticket>; 3] = self.session.planned();
            let mut keep = 0;
            for ticket in expected
                .into_iter()
                .skip(1)
                .flatten()
                .filter(|ticket| self.session.expected(*ticket))
            {
                if self.native.queued[keep + 1] != ticket.token {
                    break;
                }
                keep += 1;
                if keep == 2 {
                    break;
                }
            }
            if self.native.queued[keep + 1..]
                .iter()
                .any(|token| *token != 0)
            {
                self.send(
                    Operation::Trim {
                        generation: self.native.generation,
                        current: self.native.token,
                        keep,
                    },
                    Pending::Trim,
                );
                return Ok(true);
            }
            let intent = self.session.transport().wants_playback();
            if intent != self.intent {
                #[cfg(target_os = "ios")]
                if intent {
                    match self.shared.audio_session.activate() {
                        Ok(true) => {}
                        Ok(false) => return Ok(false),
                        Err(error) => {
                            self.session_error(error);
                            return Ok(true);
                        }
                    }
                }
                self.send(Operation::Intent(intent), Pending::Intent(intent));
                return Ok(true);
            }
        }
        // Give a ready preparation a turn after a control, keeping both streams progressing.
        if !self.yield_controls && self.controls() {
            return Ok(true);
        }
        let mut request = None;
        for ticket in self.session.requests().into_iter().flatten() {
            if request.is_none()
                && self.session.installable(ticket)
                && !self.deferred.contains(&Some(ticket))
            {
                request = Some(ticket);
            } else {
                self.session.retry_request(ticket);
            }
        }
        let Some(ticket) = request else {
            return Ok(self.controls());
        };
        self.yield_controls = false;
        if self.session.current_ticket() != Some(ticket)
            && !self
                .library
                .may_prepare_ahead(ticket.entry.song_id, now())?
        {
            let slot = self
                .deferred
                .iter_mut()
                .find(|slot| slot.is_none())
                .ok_or_else(|| io::Error::other("Deferred playback exceeded its ticket bound"))?;
            *slot = Some(ticket);
            self.session.retry_request(ticket);
            return Ok(true);
        }
        self.shared.cancelled.store(false, Ordering::SeqCst);
        if self.shared.preparation.load(Ordering::SeqCst) != self.preparation {
            self.session.retry_request(ticket);
            return Ok(true);
        }
        match self.library.prepare_playback(
            ticket.entry.song_id,
            ticket.token,
            ticket.entry.id,
            now(),
            &self.shared.cancelled,
        ) {
            Ok(prepared) => {
                let slot = self
                    .leases
                    .iter_mut()
                    .find(|slot| slot.is_none())
                    .ok_or_else(|| {
                        io::Error::other("Playback lease bookkeeping exceeded its bound")
                    })?;
                *slot = Some(ticket);
                if self.shared.preparation.load(Ordering::SeqCst) != self.preparation {
                    drop(prepared);
                    self.library.release_playback(ticket.token)?;
                    *slot = None;
                    self.session.retry_request(ticket);
                } else {
                    let replace = self.session.current_ticket() == Some(ticket);
                    self.send(
                        Operation::Install {
                            ticket,
                            item: prepared.item,
                            replace,
                        },
                        Pending::Install { ticket, replace },
                    );
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                ) =>
            {
                self.session.retry_request(ticket);
                return Ok(self.controls());
            }
            Err(error) if database_error(&error) => self.session_error(error),
            Err(error) => {
                let action = self.session.file_failed(ticket);
                self.action(action);
                self.message = Arc::from(error.to_string());
            }
        }
        Ok(true)
    }
    pub(super) fn session_error(&mut self, error: io::Error) {
        self.session
            .session_failed(self.session.transport().generation());
        self.message = Arc::from(error.to_string());
        self.reset = true;
        self.advance = None;
        self.seek = None;
    }
}
