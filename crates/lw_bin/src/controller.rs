//! Bounded main-thread/native and worker/model bridge. No database or file work runs in `poll`.
use crate::library::LibraryWorker;
use lw_apple_sys::player::{Event, Kind, Player, Prepared, Snapshot};
use lw_core::context::PlaybackContext;
use lw_core::playback::{Fault, Phase, Reading};
use lw_core::player::{Action, Session, Ticket};
use lw_core::preferences::{PlayerState, Settings};
use lw_core::queue::{Entry, RepeatMode};
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub enum Command {
    Configure {
        songs: Arc<[i64]>,
        start: usize,
        autoplay: bool,
        context: Option<Arc<PlaybackContext>>,
    },
    Select {
        occurrence: u64,
        autoplay: bool,
    },
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Volume(f32),
    Seek(f64),
    Repeat(RepeatMode),
    Shuffle {
        enabled: bool,
        seed: u64,
    },
    Reorder {
        indices: Arc<[usize]>,
        destination: usize,
    },
    Remove(u64),
    Append(i64),
    ClearUpcoming,
    UndoClear,
    DismissNotice,
}
impl Command {
    fn validate(&self) -> io::Result<()> {
        let invalid = match self {
            Self::Configure {
                songs,
                start,
                context,
                ..
            } => {
                songs.len() > 1_000_000
                    || (!songs.is_empty() && *start >= songs.len())
                    || (songs.is_empty() && *start != 0)
                    || context
                        .as_ref()
                        .is_some_and(|context| context.text_len() > (64 * 1024 * 1024 - 1024) / 6)
            }
            Self::Volume(volume) => !volume.is_finite() || !(0.0..=1.0).contains(volume),
            Self::Seek(seconds) => !seconds.is_finite(),
            Self::Reorder { indices, .. } => indices.len() > 1_000_000,
            _ => false,
        };
        if invalid {
            Err(io::ErrorKind::InvalidInput.into())
        } else {
            Ok(())
        }
    }
    fn fences_preparation(&self) -> bool {
        !matches!(self, Self::Volume(_) | Self::Seek(_) | Self::DismissNotice)
    }
    fn interrupts(&self) -> bool {
        matches!(
            self,
            Self::Configure { .. }
                | Self::Select { .. }
                | Self::Pause
                | Self::Stop
                | Self::Next
                | Self::Previous
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub settings: Settings,
    pub revision: u64,
    pub initialized: bool,
    pub processed_command: u64,
    pub generation: u64,
    pub current: Option<Entry>,
    pub queue_len: usize,
    pub current_index: Option<usize>,
    pub phase: Phase,
    pub wants_playback: bool,
    pub fault: Option<Fault>,
    pub repeat: RepeatMode,
    pub shuffled: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f32,
    pub title: Arc<str>,
    pub artist: Arc<str>,
    pub album: Arc<str>,
    pub context: Arc<PlaybackContext>,
    pub message: Arc<str>,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            revision: 0,
            context: Arc::new(PlaybackContext::Library),
            initialized: false,
            processed_command: 0,
            generation: 0,
            current: None,
            queue_len: 0,
            current_index: None,
            phase: Phase::Stopped,
            wants_playback: false,
            fault: None,
            repeat: RepeatMode::None,
            shuffled: false,
            position: 0.0,
            duration: 0.0,
            volume: 1.0,
            title: Arc::from(""),
            artist: Arc::from(""),
            album: Arc::from(""),
            message: Arc::from(""),
        }
    }
}
enum Operation {
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
enum Pending {
    Install { ticket: Ticket, replace: bool },
    Trim,
    Advance(Ticket),
    Intent(bool),
    Volume,
    Seek,
    Stop,
    Reset,
}
struct Output {
    preparation: u64,
    operation: Operation,
}
struct Ack {
    result: io::Result<bool>,
    snapshot: Snapshot,
}
struct Envelope {
    revision: u64,
    preparation: u64,
    command: Command,
}
struct Inbox {
    catalog: Option<Arc<lw_ui::catalog::Catalog>>,
    commands: VecDeque<Envelope>,
    priority: Option<Envelope>,
    events: VecDeque<Event>,
    native: Snapshot,
    dirty: bool,
    ack: Option<Ack>,
    output: Option<Output>,
    fault: Option<io::Error>,
    status: Status,
    closed: bool,
    stop_requested: bool,
    stopped: bool,
}
struct Shared {
    inbox: Mutex<Inbox>,
    changed: Condvar,
    cancelled: AtomicBool,
    revision: AtomicU64,
    preparation: AtomicU64,
    wake: extern "C" fn(),
}

pub struct Controller {
    catalog: Arc<lw_ui::catalog::Catalog>,
    native: Player,
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
    revision: u64,
    preparation: u64,
    status: Status,
    faulted: bool,
}
impl Controller {
    pub fn open(
        factory: impl FnOnce() -> io::Result<LibraryWorker> + Send + 'static,
        wake: extern "C" fn(),
    ) -> io::Result<Self> {
        let mut native = Player::open()?;
        native.set_wake(Some(wake));
        let status = Status::default();
        let shared = Arc::new(Shared {
            inbox: Mutex::new(Inbox {
                catalog: None,
                commands: VecDeque::with_capacity(32),
                priority: None,
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
            revision: AtomicU64::new(0),
            preparation: AtomicU64::new(0),
            wake,
        });
        let worker_shared = shared.clone();
        let worker = std::thread::Builder::new()
            .name("localwave-library".into())
            .spawn(move || Worker::run(factory, worker_shared))?;
        Ok(Self {
            catalog: Arc::new(lw_ui::catalog::Catalog::default()),
            native,
            shared,
            worker: Some(worker),
            revision: 0,
            preparation: 0,
            status,
            faulted: false,
        })
    }
    pub fn status(&self) -> &Status {
        &self.status
    }
    pub fn catalog(&self) -> &lw_ui::catalog::Catalog {
        &self.catalog
    }
    pub fn native_snapshot(&self) -> Snapshot {
        self.native.snapshot()
    }
    pub fn command(&mut self, command: Command) -> io::Result<u64> {
        command.validate()?;
        self.poll_inner(false)?;
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
                    (Command::Volume(_), Command::Volume(_)) | (Command::Seek(_), Command::Seek(_))
                )
            });
            if inbox.commands.len() == 32 && !priority {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            let mut envelope = Envelope {
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
            self.revision = revision;
            self.shared.revision.store(revision, Ordering::SeqCst);
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
    /// Main-thread event delivery and acknowledgements only; retained buffers bound every message.
    pub fn poll(&mut self) -> io::Result<bool> {
        self.poll_inner(true)
    }
    fn poll_inner(&mut self, apply_output: bool) -> io::Result<bool> {
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
        let output = {
            let mut inbox = self.shared.inbox.lock().unwrap();
            if inbox.events.len() + count > 128 {
                self.native.pause();
                return Err(io::Error::other(
                    "Playback event delivery exceeded its bounded queue",
                ));
            }
            inbox
                .events
                .extend(batch[..count].iter().flatten().copied());
            if inbox.native != snapshot {
                inbox.native = snapshot;
                inbox.dirty = true;
            }
            if fault.is_some() {
                inbox.fault = fault;
            }
            if apply_output {
                inbox.output.take()
            } else {
                None
            }
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
        }
        self.shared.changed.notify_one();
        let mut inbox = self.shared.inbox.lock().unwrap();
        let catalog_changed = if let Some(catalog) = inbox.catalog.take() {
            self.catalog = catalog;
            true
        } else {
            false
        };
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
        self.native.stop();
        self.shared.cancelled.store(true, Ordering::SeqCst);
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

struct Worker {
    settings: Settings,
    context: Arc<PlaybackContext>,
    library: LibraryWorker,
    shared: Arc<Shared>,
    session: Session,
    saved: PlayerState,
    revision: u64,
    preparation: u64,
    native: Snapshot,
    pending: Option<Pending>,
    leases: [Option<Ticket>; 6],
    intent: bool,
    reset: bool,
    seek: Option<f64>,
    advance: Option<(u64, u64)>,
    undo: Option<Vec<Entry>>,
    metadata_id: Option<i64>,
    title: Arc<str>,
    artist: Arc<str>,
    album: Arc<str>,
    message: Arc<str>,
    empty: Arc<str>,
    status: Status,
    active: Option<u64>,
    empty_from: Option<u64>,
    deferred: [Option<Ticket>; 3],
}
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
impl Worker {
    fn run(factory: impl FnOnce() -> io::Result<LibraryWorker>, shared: Arc<Shared>) {
        let setup = || -> io::Result<Self> {
            let mut library = factory()?;
            let settings = library.load_settings();
            // shortcut: one initial catalog snapshot; reload on import/edit when those commands are connected.
            let catalog = Arc::new(library.catalog()?);
            shared.inbox.lock().unwrap().catalog = Some(catalog);
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
            let empty: Arc<str> = Arc::from("");
            let native = shared.inbox.lock().unwrap().native;
            Ok(Self {
                settings,
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
                advance: None,
                undo: None,
                metadata_id: None,
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
        let mut inbox = shared.inbox.lock().unwrap();
        inbox.closed = true;
        drop(inbox);
        (shared.wake)();
    }
    fn work(&mut self) -> io::Result<()> {
        self.publish();
        loop {
            let mut controls: [Option<Envelope>; 32] = std::array::from_fn(|_| None);
            let mut events = [None; 128];
            let (ack, fault, dirty, snapshot, priority) = {
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
                let dirty = std::mem::take(&mut inbox.dirty);
                (
                    inbox.ack.take(),
                    inbox.fault.take(),
                    dirty,
                    inbox.native,
                    inbox.priority.take(),
                )
            };
            if let Some(ack) = ack {
                self.acknowledge(ack)?;
            }
            // Starts must be recorded before a newer snapshot retires their physical file leases.
            for event in events
                .iter()
                .flatten()
                .filter(|event| event.kind == Kind::Started)
            {
                self.library.record_playback_start(event.token(), now())?;
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
            let index = self.session.queue().current_index().unwrap_or(0) as i64;
            if self.saved.current_index != index {
                self.library.save_player_index(index)?;
                self.saved.current_index = index;
            }
            self.retire()?;
            self.publish();
            if self.pending.is_none()
                && self.revision == self.shared.revision.load(Ordering::SeqCst)
                && self.drive()?
            {
                continue;
            }
            let mut inbox = self.shared.inbox.lock().unwrap();
            while !inbox.closed
                && inbox.commands.is_empty()
                && inbox.priority.is_none()
                && inbox.events.is_empty()
                && inbox.ack.is_none()
                && inbox.fault.is_none()
                && !inbox.dirty
            {
                inbox = self.shared.changed.wait(inbox).unwrap();
            }
        }
    }
    fn send(&mut self, operation: Operation, pending: Pending) {
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
    fn acknowledge(&mut self, ack: Ack) -> io::Result<()> {
        let rejected = matches!(&ack.result, Ok(false));
        let Some(pending) = self.pending.take() else {
            return Err(io::Error::other(
                "Unexpected native playback acknowledgement",
            ));
        };
        match ack.result {
            Ok(true) => match pending {
                Pending::Install { ticket, replace } => {
                    self.session.installed(ticket);
                    if replace {
                        self.intent = false;
                    }
                }
                Pending::Intent(intent) => self.intent = intent,
                Pending::Stop | Pending::Reset => {
                    self.intent = false;
                    if matches!(pending, Pending::Reset) {
                        self.reset = false;
                    }
                }
                Pending::Volume | Pending::Trim | Pending::Advance(_) | Pending::Seek => {}
            },
            Ok(false) => {
                if let Pending::Install { ticket, .. } = pending {
                    self.session.retry_request(ticket);
                }
            }
            Err(error) => self.session_error(error),
        }
        self.observe(ack.snapshot);
        if rejected && let Pending::Advance(ticket) = pending {
            let autoplay = self.session.transport().wants_playback();
            if self.commit_selection(ticket) && autoplay {
                let action = self.session.play();
                self.action(action);
            }
        }
        Ok(())
    }
    fn observe(&mut self, snapshot: Snapshot) {
        let previous = self.native;
        self.native = snapshot;
        if snapshot.token != 0 {
            if let Some(ticket) = self
                .leases
                .iter()
                .flatten()
                .find(|ticket| {
                    ticket.token == snapshot.token && ticket.generation == snapshot.generation
                })
                .copied()
            {
                self.session.reconcile_native(ticket);
            }
            self.session.update(Reading {
                generation: snapshot.generation,
                token: snapshot.token,
                position: snapshot.position,
                duration: snapshot.duration,
                ready: snapshot.status == 1,
                playing: snapshot.playing(),
                waiting: snapshot.time_control == 1,
            });
        } else if previous.token != 0
            && previous.generation == self.session.transport().generation()
        {
            self.empty_from = Some(previous.generation);
        }
    }
    fn event(&mut self, event: Event) {
        match event.kind {
            Kind::EngineFailed => self.session_error(io::Error::other(event.message())),
            Kind::Failed => {
                let ticket = self.session.planned().into_iter().flatten().find(|ticket| {
                    ticket.token == event.token() && ticket.generation == event.generation()
                });
                if let Some(ticket) = ticket {
                    let action = self.session.file_failed(ticket);
                    self.action(action);
                } else if let Some(entry) = self
                    .session
                    .queue()
                    .entries()
                    .iter()
                    .find(|entry| entry.id == event.occurrence())
                    .copied()
                {
                    self.session.retired_failure(event.generation(), entry);
                }
                if self.session.notice().is_some() || self.session.error().is_some() {
                    self.message = Arc::from(event.message());
                }
            }
            Kind::Finished => {
                self.session
                    .finished(event.generation(), event.token(), event.duration())
            }
            _ => {}
        }
    }
    fn action(&mut self, action: Action) {
        if let Action::Advance { generation, token } = action {
            self.advance = Some((generation, token));
        } else if matches!(action, Action::Load(_) | Action::Stop | Action::Ended) {
            self.advance = None;
        }
    }
    fn pending_selection(&self) -> Option<Ticket> {
        self.advance
            .and_then(|(generation, token)| {
                self.session
                    .planned()
                    .into_iter()
                    .flatten()
                    .find(|ticket| ticket.generation == generation && ticket.token == token)
            })
            .or(match self.pending {
                Some(Pending::Advance(ticket)) => Some(ticket),
                _ => None,
            })
    }
    fn commit_selection(&mut self, ticket: Ticket) -> bool {
        if ticket.generation != self.session.transport().generation()
            || matches!(
                self.session.transport().phase(),
                Phase::Stopped | Phase::Ended | Phase::Failed
            )
            || self.native.token == ticket.token
            || !self.native.queued.contains(&ticket.token)
            || !self.session.queue().entries().contains(&ticket.entry)
        {
            return false;
        }
        self.advance = None;
        self.session.select(ticket.entry.id, false);
        true
    }
    fn command(&mut self, envelope: Envelope) -> io::Result<()> {
        self.revision = envelope.revision;
        self.preparation = envelope.preparation;
        if matches!(
            envelope.command,
            Command::Next | Command::Previous | Command::Pause | Command::Stop
        ) && let Some(ticket) = self.pending_selection()
        {
            self.commit_selection(ticket);
        }
        if matches!(envelope.command, Command::Append(_))
            && self.session.queue().entries().len() == 1_000_000
        {
            self.message = Arc::from("The queue is full");
            return Ok(());
        }
        let keep_undo = matches!(
            envelope.command,
            Command::ClearUpcoming | Command::UndoClear
        );
        if envelope.command.interrupts() {
            self.intent = false;
        }
        let mut structural = false;
        let action = match envelope.command {
            Command::Configure {
                songs,
                start,
                autoplay,
                context,
            } => {
                let context_data = context
                    .as_ref()
                    .map(|value| self.library.playback_context_data(value))
                    .transpose()?;
                self.session.configure(&songs, start);
                self.context = context.unwrap_or_else(|| Arc::new(PlaybackContext::Library));
                self.saved.context_data = context_data;
                self.undo = None;
                self.advance = None;
                self.seek = None;
                self.message = self.empty.clone();
                structural = true;
                self.session
                    .queue()
                    .current()
                    .map_or(Action::None, |entry| {
                        self.session.select(entry.id, autoplay)
                    })
            }
            Command::Select {
                occurrence,
                autoplay,
            } => self.session.select(occurrence, autoplay),
            Command::Play => {
                self.message = self.empty.clone();
                self.session.play()
            }
            Command::Pause => {
                self.session.pause();
                Action::None
            }
            Command::Stop => {
                self.session.stop();
                self.advance = None;
                self.seek = None;
                Action::Stop
            }
            Command::Next => self.session.next_song(),
            Command::Previous => self.session.previous(),
            Command::Volume(volume) => {
                if self.saved.volume != Some(volume) {
                    self.library.save_player_volume(volume)?;
                    self.saved.volume = Some(volume);
                }
                Action::None
            }
            Command::Seek(seconds) => {
                self.seek = Some(seconds.max(0.0));
                Action::None
            }
            Command::Repeat(mode) => {
                self.session.set_repeat(mode);
                Action::None
            }
            Command::Shuffle { enabled, seed } => {
                self.session.set_shuffle(enabled, seed);
                structural = true;
                Action::None
            }
            Command::Reorder {
                indices,
                destination,
            } => {
                structural = self.session.reorder(&indices, destination);
                Action::None
            }
            Command::Remove(occurrence) => {
                structural = self.session.remove(occurrence);
                Action::None
            }
            Command::Append(song) => {
                self.session.append(song);
                structural = true;
                Action::None
            }
            Command::ClearUpcoming => {
                if self
                    .session
                    .queue()
                    .current_index()
                    .is_some_and(|index| index + 1 < self.session.queue().entries().len())
                {
                    self.undo = Some(self.session.queue().entries().to_vec());
                    self.session.clear_upcoming();
                    structural = true;
                }
                Action::None
            }
            Command::UndoClear => {
                if let Some(entries) = self.undo.take() {
                    structural = self.session.restore(&entries);
                }
                Action::None
            }
            Command::DismissNotice => {
                self.session.dismiss_notice();
                self.message = self.empty.clone();
                Action::None
            }
        };
        self.action(action);
        if structural && !keep_undo {
            self.undo = None;
        }
        if structural {
            self.saved.song_ids.clear();
            self.saved.song_ids.extend(
                self.session
                    .queue()
                    .entries()
                    .iter()
                    .map(|entry| entry.song_id),
            );
            self.saved.current_index = self.session.queue().current_index().unwrap_or(0) as i64;
            self.library.save_player_state(&self.saved)?;
        }
        Ok(())
    }
    fn retire(&mut self) -> io::Result<()> {
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
    fn drive(&mut self) -> io::Result<bool> {
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
        let volume = self.saved.volume.unwrap_or(1.0);
        if self.native.volume != volume {
            self.send(Operation::Volume(volume), Pending::Volume);
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
                self.send(Operation::Intent(intent), Pending::Intent(intent));
                return Ok(true);
            }
            if let Some(seconds) = self.seek.take() {
                self.send(Operation::Seek(seconds), Pending::Seek);
                return Ok(true);
            }
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
            return Ok(false);
        };
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
                return Ok(false);
            }
            Err(error) => {
                let action = self.session.file_failed(ticket);
                self.action(action);
                self.message = Arc::from(error.to_string());
            }
        }
        Ok(true)
    }
    fn session_error(&mut self, error: io::Error) {
        self.session
            .session_failed(self.session.transport().generation());
        self.message = Arc::from(error.to_string());
        self.reset = true;
        self.advance = None;
        self.seek = None;
    }
    fn publish(&mut self) {
        let current = self.session.queue().current();
        let id = current.map(|entry| entry.song_id);
        if id != self.metadata_id {
            // shortcut: refresh on song identity; invalidate this cache when catalog edit commands are connected.
            self.metadata_id = id;
            match id
                .map(|id| self.library.repository.get_song(id))
                .transpose()
            {
                Ok(Some(Some(song))) => {
                    self.title = Arc::from(song.title);
                    self.artist = Arc::from(song.artist);
                    self.album = Arc::from(song.album);
                }
                result => {
                    self.title = self.empty.clone();
                    self.artist = self.empty.clone();
                    self.album = self.empty.clone();
                    if let Err(error) = result {
                        self.message = Arc::from(error.to_string());
                    }
                }
            };
        }
        let transport = self.session.transport();
        let status = Status {
            settings: self.settings,
            context: self.context.clone(),
            revision: self.status.revision,
            initialized: true,
            processed_command: self.revision,
            generation: transport.generation(),
            current,
            queue_len: self.session.queue().entries().len(),
            current_index: self.session.queue().current_index(),
            phase: transport.phase(),
            wants_playback: transport.wants_playback(),
            fault: transport.fault(),
            repeat: self.session.repeat(),
            shuffled: self.session.queue().shuffled(),
            position: transport.position(),
            duration: transport.duration(),
            volume: self.saved.volume.unwrap_or(1.0),
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            message: self.message.clone(),
        };
        if status != self.status {
            self.status = status;
            self.status.revision += 1;
            let mut inbox = self.shared.inbox.lock().unwrap();
            inbox.status = self.status.clone();
            drop(inbox);
            (self.shared.wake)();
        }
    }
}
