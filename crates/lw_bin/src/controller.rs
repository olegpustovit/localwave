//! Bounded main-thread/native and worker/model bridge. No database or file work runs in `poll`.
use crate::library::{LibraryWorker, database_error};
use lw_apple_sys::player::{Event, Kind, Player, Prepared, Snapshot};
use lw_apple_sys::session::Event as AudioEvent;
use lw_core::context::PlaybackContext;
use lw_core::playback::{Fault, Phase, Reading};
use lw_core::player::{Action, Session, Ticket};
use lw_core::preferences::{Change as SettingChange, PlayerState, Settings};
use lw_core::queue::{Entry, RepeatMode};
use lw_ui::browse::Request;
use lw_ui::catalog::Catalog;
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Inline name packets keep delivery through the bounded mailbox allocation-free.
#[allow(clippy::large_enum_variant)]
pub enum Command {
    Configure {
        songs: Arc<[i64]>,
        start: usize,
        autoplay: bool,
        context: Option<Arc<PlaybackContext>>,
    },
    ShuffleAll {
        songs: Arc<[i64]>,
        seed: u64,
    },
    Select {
        occurrence: u64,
        autoplay: bool,
    },
    Play,
    AudioSession(AudioEvent),
    Toggle,
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
    MoveUpcoming {
        occurrence: u64,
        before: Option<u64>,
    },
    MoveEntry {
        occurrence: u64,
        before: Option<u64>,
    },
    Remove(u64),
    Append(i64),
    ClearUpcoming,
    UndoClear,
    SaveQueue(lw_ui::browse::Query),
    DismissNotice,
    RefreshCatalog,
    Setting(SettingChange),
    ReadStorage,
    TogglePins(Arc<[i64]>),
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
            Self::ShuffleAll { songs, .. } => songs.is_empty() || songs.len() > 1_000_000,
            Self::Volume(volume) => !volume.is_finite() || !(0.0..=1.0).contains(volume),
            Self::Seek(seconds) => !seconds.is_finite(),
            Self::Reorder { indices, .. } => indices.len() > 1_000_000,
            Self::TogglePins(songs) => songs.is_empty() || songs.len() > 1_000_000,
            Self::SaveQueue(name) => name.as_str().trim().is_empty(),
            Self::Setting(change) => Settings::default().applying(*change).is_err(),
            _ => false,
        };
        if invalid {
            Err(io::ErrorKind::InvalidInput.into())
        } else {
            Ok(())
        }
    }
    fn fences_preparation(&self) -> bool {
        !matches!(
            self,
            Self::Volume(_)
                | Self::Seek(_)
                | Self::DismissNotice
                | Self::RefreshCatalog
                | Self::SaveQueue(_)
                | Self::ReadStorage
                | Self::TogglePins(_)
                | Self::Setting(
                    SettingChange::Appearance(_)
                        | SettingChange::Signal(_)
                        | SettingChange::ToggleReduceMotion
                        | SettingChange::ToggleShowNonAudio
                        | SettingChange::CacheGB(_)
                )
        )
    }
    fn interrupts(&self) -> bool {
        matches!(
            self,
            Self::Configure { .. }
                | Self::ShuffleAll { .. }
                | Self::Select { .. }
                | Self::Pause
                | Self::AudioSession(
                    AudioEvent::Began
                        | AudioEvent::OutputUnavailable
                        | AudioEvent::Overflow
                        | AudioEvent::Reset
                )
                | Self::Toggle
                | Self::Stop
                | Self::Next
                | Self::Previous
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub settings: Settings,
    pub cache_usage: Arc<str>,
    pub settings_message: Arc<str>,
    pub storage_busy: bool,
    pub waveform: Option<lw_core::waveform::Peaks>,
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
            cache_usage: Arc::from(""),
            settings_message: Arc::from(""),
            storage_busy: false,
            waveform: None,
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
    audio_notice: Option<lw_apple_sys::session::Notification>,
    revision: u64,
    preparation: u64,
    command: Command,
}
struct Inbox {
    catalog: Option<Arc<lw_ui::catalog::Catalog>>,
    retired_catalog: Option<Arc<lw_ui::catalog::Catalog>>,
    commands: VecDeque<Envelope>,
    priority: Option<Envelope>,
    browse: Option<(u64, Request)>,
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
impl Inbox {
    fn update_native(
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
struct Shared {
    #[cfg(target_os = "ios")]
    audio_session: lw_apple_sys::session::Session,
    inbox: Mutex<Inbox>,
    changed: Condvar,
    cancelled: AtomicBool,
    waveform_cancelled: AtomicBool,
    preparation: AtomicU64,
    wake: extern "C" fn(),
}

impl Shared {
    fn begin_background(&self, preparation: u64, token: u64) -> bool {
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
    fn update_catalog(
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
        let shared = Arc::new(Shared {
            #[cfg(target_os = "ios")]
            audio_session: lw_apple_sys::session::Session::open(wake)?,
            inbox: Mutex::new(Inbox {
                catalog: None,
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
            catalog: Arc::new(lw_ui::catalog::Catalog::default()),
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

struct Worker {
    catalog: Catalog,
    queue: Option<Arc<lw_ui::queue::Snapshot>>,
    queue_dirty: bool,
    browse: Option<Request>,
    browse_dirty: bool,
    catalog_dirty: bool,
    catalog_error: Option<Arc<str>>,
    settings: Settings,
    cache_usage: Arc<str>,
    settings_message: Arc<str>,
    storage_pending: bool,
    copies_pending: bool,
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
    yield_controls: bool,
    volume_last: bool,
    advance: Option<(u64, u64)>,
    undo: Option<Vec<Entry>>,
    metadata_id: Option<i64>,
    waveform_key: Option<(i64, u64)>,
    waveform: Option<lw_core::waveform::Peaks>,
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
            self.catalog_dirty |= self.library.take_playback_file_changes();
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
    fn load_storage(&mut self) {
        if !std::mem::take(&mut self.storage_pending) {
            return;
        }
        match self.library.enforce_cache_limit() {
            Ok(eviction) => {
                self.catalog_dirty |= eviction.files != 0;
                let bytes = eviction.remaining_bytes;
                let usage = if bytes >= 1_000_000_000 {
                    format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
                } else if bytes >= 1_000_000 {
                    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
                } else if bytes >= 1_000 {
                    format!("{:.1} KB", bytes as f64 / 1_000.0)
                } else {
                    format!("{bytes} bytes")
                };
                let summary = format!(
                    "{usage} cached. Kept songs may exceed the limit; Added Files are never removed."
                );
                if self.cache_usage.as_ref() != summary {
                    self.cache_usage = Arc::from(summary);
                }
                self.settings_message = self.empty.clone();
            }
            Err(error) => {
                eprintln!("LocalWave offline space: {error}");
                self.settings_message =
                    Arc::from("Could not read or trim offline space. Try again.");
            }
        }
    }
    fn load_copies(&mut self) -> bool {
        if !self.copies_pending
            || !self
                .shared
                .begin_background(self.preparation, self.native.token)
        {
            return false;
        }
        match self.library.process_copy_queue(
            now(),
            &self.shared.waveform_cancelled,
            &mut |_, _| {},
        ) {
            Ok(summary) => {
                self.copies_pending = false;
                if !summary.failed.is_empty() {
                    self.message = Arc::from(
                        "Some songs could not be kept offline. Reconnect the source and try again.",
                    );
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                eprintln!("LocalWave offline copies: {error}");
                self.copies_pending = false;
                self.message = Arc::from("Could not finish keeping songs offline. Try again.");
            }
        }
        self.catalog_dirty = true;
        true
    }
    fn load_waveform(&mut self) {
        let Some(song) = self.session.queue().current().map(|entry| entry.song_id) else {
            return;
        };
        let key = (song, self.native.token);
        if self.waveform_key == Some(key) || !self.shared.begin_background(self.preparation, key.1)
        {
            return;
        }
        let result = self
            .library
            .waveform_peaks(song, now(), &self.shared.waveform_cancelled);
        if self.shared.waveform_cancelled.load(Ordering::SeqCst) {
            return;
        }
        self.waveform_key = Some(key);
        self.waveform = match result {
            Ok(peaks) => peaks,
            Err(error) => {
                eprintln!("LocalWave waveform: {error}");
                None
            }
        };
    }
    fn send(&mut self, operation: Operation, pending: Pending) {
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
                if self.leases.iter().flatten().any(|ticket| {
                    ticket.token == event.token()
                        && ticket.generation == event.generation()
                        && ticket.entry.id == event.occurrence()
                }) && let Err(error) = self.library.record_playback_failure(event.token())
                {
                    eprintln!("LocalWave native file failure state: {error}");
                }
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
    fn command(&mut self, mut envelope: Envelope) -> io::Result<()> {
        if let Command::ShuffleAll { songs, seed } = &envelope.command {
            let mut songs = songs.clone();
            lw_core::queue::shuffle_songs(Arc::make_mut(&mut songs), *seed);
            envelope.command = Command::Configure {
                songs,
                start: 0,
                autoplay: true,
                context: None,
            };
        }
        if matches!(envelope.command, Command::Toggle) {
            envelope.command = if self.session.transport().wants_playback() {
                Command::Pause
            } else {
                Command::Play
            };
        }
        self.revision = self.revision.max(envelope.revision);
        self.preparation = self.preparation.max(envelope.preparation);
        let session_pause =
            matches!(&envelope.command, Command::AudioSession(event) if event.pauses());
        if (session_pause
            || matches!(
                envelope.command,
                Command::Next | Command::Previous | Command::Pause | Command::Stop
            ))
            && let Some(ticket) = self.pending_selection()
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
            Command::ShuffleAll { .. } => unreachable!("translated to Configure on the worker"),
            Command::Select {
                occurrence,
                autoplay,
            } => self.session.select(occurrence, autoplay),
            Command::Play => {
                self.message = self.empty.clone();
                self.session.play()
            }
            Command::Toggle => {
                unreachable!("Toggle resolves before committing a pending selection")
            }
            Command::AudioSession(event) => match event {
                AudioEvent::Began => {
                    self.session.interruption_began();
                    Action::None
                }
                AudioEvent::Ended { resume } => self.session.interruption_ended(resume),
                AudioEvent::OutputUnavailable => {
                    self.session.output_unavailable();
                    Action::None
                }
                AudioEvent::Reset => {
                    self.session_error(io::Error::other(
                        "The audio service restarted. Play to try again.",
                    ));
                    Action::None
                }
                AudioEvent::Overflow => {
                    self.session_error(io::Error::other(
                        "Audio-session notifications overflowed; playback was paused",
                    ));
                    Action::None
                }
            },
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
            Command::MoveUpcoming { occurrence, before } => {
                structural = self.session.move_upcoming(occurrence, before);
                Action::None
            }
            Command::MoveEntry { occurrence, before } => {
                structural = self.session.move_entry(occurrence, before);
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
            Command::SaveQueue(name) => {
                if self.saved.song_ids.is_empty() {
                    self.message =
                        Arc::from("The queue is empty. Add tracks before saving a playlist.");
                } else {
                    match self.library.repository.create_playlist_with_songs(
                        name.as_str().trim(),
                        &self.saved.song_ids,
                        now(),
                    ) {
                        Ok(_) => {
                            self.catalog_dirty = true;
                            self.metadata_id = None;
                            self.message = Arc::from("Playlist saved.");
                        }
                        Err(error) => {
                            eprintln!("LocalWave save queue: {error}");
                            self.message = Arc::from("Could not save the playlist. Try again.");
                        }
                    }
                }
                Action::None
            }
            Command::DismissNotice => {
                self.session.dismiss_notice();
                self.message = self.empty.clone();
                Action::None
            }
            Command::RefreshCatalog => {
                self.catalog_dirty = true;
                self.copies_pending = true;
                Action::None
            }
            Command::TogglePins(songs) => {
                match self.library.toggle_pins(&songs) {
                    Ok(pinned) => {
                        self.copies_pending |= pinned;
                        self.catalog_dirty = true;
                        self.message = self.empty.clone();
                    }
                    Err(error) => {
                        eprintln!("LocalWave keep offline: {error}");
                        self.message = Arc::from(
                            "Could not change kept songs. Refresh or reconnect the source and try again.",
                        );
                    }
                }
                Action::None
            }
            Command::ReadStorage => {
                self.storage_pending = true;
                Action::None
            }
            Command::Setting(change) => {
                let next = self.settings.applying(change)?;
                if next != self.settings {
                    match self.library.save_settings(&next) {
                        Ok(()) => {
                            self.storage_pending |= next.cache_gb != self.settings.cache_gb;
                            if next.download_ahead != self.settings.download_ahead {
                                self.deferred = [None; 3];
                            }
                            self.settings = next;
                            self.settings_message = self.empty.clone();
                        }
                        Err(error) => {
                            eprintln!("LocalWave settings: {error}");
                            self.settings_message =
                                Arc::from("Could not save this setting. Try again.");
                        }
                    }
                }
                Action::None
            }
        };
        self.action(action);
        if structural && !keep_undo {
            self.undo = None;
        }
        if structural {
            self.queue_dirty = true;
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
        if let Some(notice) = envelope.audio_notice {
            notice.mark_processed();
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
    fn controls(&mut self) -> bool {
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
    fn session_error(&mut self, error: io::Error) {
        self.session
            .session_failed(self.session.transport().generation());
        self.message = Arc::from(error.to_string());
        self.reset = true;
        self.advance = None;
        self.seek = None;
    }
    fn publish_catalog(&self) {
        let previous = self
            .shared
            .inbox
            .lock()
            .unwrap()
            .catalog
            .replace(Arc::new(self.catalog.clone()));
        drop(previous);
        (self.shared.wake)();
    }
    fn publish(&mut self) {
        let cursor = self.session.queue().current_index();
        let can_undo = self.undo.is_some();
        if self.queue_dirty
            || self.queue.as_ref().is_none_or(|queue| {
                queue.current != cursor
                    || queue.can_undo != can_undo
                    || !Arc::ptr_eq(&queue.context, &self.context)
            })
        {
            let entries = if self.queue_dirty {
                Arc::from(self.session.queue().entries())
            } else {
                self.queue.as_ref().unwrap().entries.clone()
            };
            let revision = self
                .queue
                .as_ref()
                .map_or(1, |queue| queue.revision.wrapping_add(1));
            let queue = Arc::new(lw_ui::queue::Snapshot::new(
                revision,
                entries,
                cursor,
                self.context.clone(),
                can_undo,
            ));
            self.queue = Some(queue.clone());
            self.catalog.queue = Some(queue);
            self.queue_dirty = false;
            self.publish_catalog();
        }
        let current = self.session.queue().current();
        let id = current.map(|entry| entry.song_id);
        if id != self.metadata_id {
            self.waveform_key = None;
            self.waveform = None;
            self.metadata_id = id;
            let fetched = match id
                .map(|id| self.library.repository.get_song(id))
                .transpose()
            {
                Ok(song) => song.flatten(),
                Err(error) => {
                    self.message = Arc::from(error.to_string());
                    None
                }
            };
            // A removed library row can still own the current physical cached item.
            let song = fetched.as_ref().or_else(|| {
                current
                    .filter(|entry| entry.id == self.native.occurrence && self.native.token != 0)
                    .and_then(|_| self.library.playback_song(self.native.token))
                    .filter(|song| song.id == id)
            });
            if let Some(song) = song {
                self.title = Arc::from(song.title.as_str());
                self.artist = Arc::from(song.artist.as_str());
                self.album = Arc::from(song.album.as_str());
            } else {
                self.title = self.empty.clone();
                self.artist = self.empty.clone();
                self.album = self.empty.clone();
            }
        }
        let transport = self.session.transport();
        let status = Status {
            settings: self.settings,
            cache_usage: self.cache_usage.clone(),
            settings_message: self.settings_message.clone(),
            storage_busy: self.storage_pending,
            waveform: self.waveform,
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

#[cfg(test)]
mod tests {
    use super::*;
    use lw_ui::catalog::Catalog;
    extern "C" fn wake() {}
    #[test]
    fn audio_session_events_fence_preparation_and_only_pause_for_interruptions_or_faults() {
        use lw_apple_sys::session::Event;
        for event in [
            Event::Began,
            Event::OutputUnavailable,
            Event::Overflow,
            Event::Reset,
            Event::Ended { resume: false },
            Event::Ended { resume: true },
        ] {
            let command = Command::AudioSession(event);
            assert!(command.validate().is_ok());
            assert!(command.fences_preparation());
            assert_eq!(command.interrupts(), event.pauses());
        }
    }
    #[test]
    fn fresh_shuffle_commands_validate_and_interrupt_the_previous_selection() {
        let command = Command::ShuffleAll {
            songs: Arc::from([1, 1, 2]),
            seed: 42,
        };
        assert!(command.validate().is_ok());
        assert!(command.fences_preparation());
        assert!(command.interrupts());
        assert!(
            Command::ShuffleAll {
                songs: Arc::from([]),
                seed: 0
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn settings_requests_validate_limits_and_only_file_policy_changes_fence_preparation() {
        for change in [
            SettingChange::Appearance(lw_core::preferences::Appearance::Light),
            SettingChange::Signal(lw_core::preferences::Signal::Green),
            SettingChange::ToggleReduceMotion,
            SettingChange::ToggleShowNonAudio,
            SettingChange::CacheGB(2),
        ] {
            let command = Command::Setting(change);
            assert!(command.validate().is_ok());
            assert!(!command.fences_preparation());
            assert!(!command.interrupts());
        }
        for change in [
            SettingChange::ToggleDownloadAhead,
            SettingChange::ToggleNoNetwork,
        ] {
            let command = Command::Setting(change);
            assert!(command.fences_preparation());
            assert!(!command.interrupts());
        }
        assert!(
            Command::Setting(SettingChange::CacheGB(3))
                .validate()
                .is_err()
        );
        assert!(!Command::ReadStorage.fences_preparation());
        assert!(!Command::ReadStorage.interrupts());
    }
    #[test]
    fn saving_queue_names_is_bounded_and_does_not_fence_or_interrupt_preparation() {
        for name in ["", "  \t\n"] {
            assert!(
                Command::SaveQueue(lw_ui::browse::Query::new(name).unwrap())
                    .validate()
                    .is_err()
            );
        }
        let command = Command::SaveQueue(lw_ui::browse::Query::new("  Playlist 🎵  ").unwrap());
        assert!(command.validate().is_ok());
        assert!(!command.fences_preparation());
        assert!(!command.interrupts());
    }
    fn shared() -> Arc<Shared> {
        Arc::new(Shared {
            #[cfg(target_os = "ios")]
            audio_session: lw_apple_sys::session::Session::open(wake).unwrap(),
            inbox: Mutex::new(Inbox {
                catalog: None,
                retired_catalog: None,
                commands: VecDeque::new(),
                priority: None,
                browse: None,
                events: VecDeque::with_capacity(128),
                native: Snapshot::default(),
                dirty: false,
                ack: None,
                output: None,
                fault: None,
                status: Status::default(),
                closed: false,
                stop_requested: false,
                stopped: false,
            }),
            changed: Condvar::new(),
            cancelled: AtomicBool::new(false),
            waveform_cancelled: AtomicBool::new(false),
            preparation: AtomicU64::new(0),
            wake,
        })
    }
    #[test]
    fn continuous_volume_and_seek_delivery_alternates_with_current_and_upcoming_preparation() {
        let base = std::env::temp_dir().join(format!("lw-control-fairness-{}", std::process::id()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let shared = shared();
        shared.preparation.store(1, Ordering::SeqCst);
        shared.inbox.lock().unwrap().native.volume = 1.0;
        let owner = shared.clone();
        let worker_base = base.clone();
        let thread = std::thread::spawn(move || {
            Worker::run(
                move || {
                    let documents = worker_base.join("Documents");
                    std::fs::create_dir_all(documents.join("Added Files"))?;
                    std::fs::write(
                        documents.join("Added Files/track.m4a"),
                        include_bytes!("../../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a"),
                    )?;
                    let mut library = LibraryWorker::open(
                        &worker_base.join("library.sqlite"),
                        lw_apple_sys::preferences::Preferences::open(Some(&format!(
                            "localwave.control-fairness.{}",
                            std::process::id()
                        )))?,
                        lw_apple_sys::preferences::AppPaths {
                            documents,
                            caches: worker_base.join("Caches"),
                            temporary: worker_base.join("tmp"),
                            test_database: None,
                        },
                    )?;
                    for key in 1..=3 {
                        library
                            .repository
                            .upsert_song(
                                &lw_core::Song {
                                    song_key: key,
                                    path_hash: key,
                                    local_file_path: Some("Added Files/track.m4a".into()),
                                    file_state: lw_core::FileState::Copied,
                                    ..Default::default()
                                },
                                1.0,
                            )
                            .map_err(io::Error::other)?;
                    }
                    Ok(library)
                },
                owner,
            )
        });
        struct Close(Arc<Shared>, Option<JoinHandle<()>>);
        impl Drop for Close {
            fn drop(&mut self) {
                self.0.inbox.lock().unwrap().closed = true;
                self.0.changed.notify_one();
                self.1.take().unwrap().join().unwrap();
            }
        }
        let _close = Close(shared.clone(), Some(thread));
        shared.inbox.lock().unwrap().commands.push_back(Envelope {
            audio_notice: None,
            revision: 1,
            preparation: 1,
            command: Command::Configure {
                songs: Arc::from([1, 2, 3]),
                start: 0,
                autoplay: true,
                context: None,
            },
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        let (mut installs, mut seeks, mut volumes) = (0, 0, 0);
        for step in 0..16 {
            let output = loop {
                if let Some(output) = shared.inbox.lock().unwrap().output.take() {
                    break output;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "worker did not produce its next operation"
                );
                std::thread::yield_now();
            };
            let mut native = shared.inbox.lock().unwrap().native;
            match output.operation {
                Operation::Volume(value) => {
                    native.volume = value;
                    volumes += 1;
                }
                Operation::Seek(seconds) => {
                    native.position = seconds;
                    seeks += 1;
                }
                Operation::Install {
                    ticket,
                    item,
                    replace,
                } => {
                    drop(item);
                    if replace {
                        native.queued = [0; 3];
                        native.token = ticket.token;
                        native.occurrence = ticket.entry.id;
                    }
                    *native.queued.iter_mut().find(|token| **token == 0).unwrap() = ticket.token;
                    native.generation = ticket.generation;
                    native.status = 1;
                    installs += 1;
                }
                Operation::Intent(playing) => {
                    native.time_control = if playing { 2 } else { 0 };
                }
                Operation::Reset | Operation::Stop => {
                    native.token = 0;
                    native.queued = [0; 3];
                }
                _ => panic!("unexpected operation in steady preparation"),
            }
            let mut inbox = shared.inbox.lock().unwrap();
            inbox.native = native;
            inbox.ack = Some(Ack {
                result: Ok(true),
                snapshot: native,
            });
            for (index, command) in [
                Command::Volume(if native.volume == 0.25 { 0.75 } else { 0.25 }),
                Command::Seek(0.0),
                Command::RefreshCatalog,
            ]
            .into_iter()
            .enumerate()
            {
                inbox.commands.push_back(Envelope {
                    audio_notice: None,
                    revision: 2 + step * 3 + index as u64,
                    preparation: 1,
                    command,
                });
            }
            drop(inbox);
            shared.changed.notify_one();
            if installs == 3 && seeks >= 2 && volumes >= 2 {
                break;
            }
        }
        assert_eq!(
            installs, 3,
            "continuous controls starved current/upcoming preparation"
        );
        assert!(
            seeks >= 2 && volumes >= 2,
            "control delivery must also keep progressing"
        );
    }
    #[test]
    fn waveform_start_yields_to_controls_and_never_clears_playback_cancellation() {
        let shared = shared();
        shared.cancelled.store(true, Ordering::SeqCst);
        shared.waveform_cancelled.store(true, Ordering::SeqCst);
        assert!(shared.begin_background(0, 0));
        assert!(!shared.waveform_cancelled.load(Ordering::SeqCst));
        assert!(shared.cancelled.load(Ordering::SeqCst));
        shared.waveform_cancelled.store(true, Ordering::SeqCst);
        shared.inbox.lock().unwrap().commands.push_back(Envelope {
            audio_notice: None,
            revision: 1,
            preparation: 0,
            command: Command::Seek(5.0),
        });
        assert!(!shared.begin_background(0, 0));
        assert!(shared.waveform_cancelled.load(Ordering::SeqCst));
        shared.inbox.lock().unwrap().commands.clear();
        assert!(!shared.begin_background(1, 0));
        assert!(!shared.begin_background(0, 1));
        shared.inbox.lock().unwrap().browse = Some((
            2,
            Request {
                commands: None,
                scope: lw_ui::browse::Scope::Find,
                query: lw_ui::browse::Query::default(),
                sort: lw_ui::browse::Sort::Original,
                ascending: true,
            },
        ));
        assert!(!shared.begin_background(0, 0));
        shared.inbox.lock().unwrap().browse = None;
        let status = Status {
            waveform: Some([0.5; 512]),
            ..Status::default()
        };
        mem_sys::local_reset();
        for _ in 0..1000 {
            assert!(shared.begin_background(0, 0));
            assert_eq!(
                std::hint::black_box(status.clone()).waveform,
                status.waveform
            );
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }

    #[test]
    fn unchanged_native_polls_need_no_worker_notification_or_warm_allocation() {
        let shared = shared();
        let mut inbox = shared.inbox.lock().unwrap();
        mem_sys::local_reset();
        for _ in 0..1000 {
            assert!(!inbox.update_native(&[], Snapshot::default(), None).unwrap());
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        assert!(!inbox.dirty);
        let next = Snapshot {
            volume: 0.5,
            ..Default::default()
        };
        assert!(inbox.update_native(&[], next, None).unwrap());
        assert!(inbox.dirty);
        inbox.dirty = false;
        assert!(!inbox.update_native(&[], next, None).unwrap());
        assert!(
            inbox.update_native(&[None], next, None).unwrap(),
            "a delivered batch must wake the worker"
        );
        assert!(
            inbox
                .update_native(&[], next, Some(io::ErrorKind::Other.into()))
                .unwrap()
        );
        assert!(inbox.fault.is_some());
        inbox.fault = None;
        assert!(
            inbox
                .update_native(
                    &[None; 129],
                    Snapshot::default(),
                    Some(io::ErrorKind::Other.into())
                )
                .is_err()
        );
        assert_eq!(
            inbox.native, next,
            "overflow must refuse the whole delivery"
        );
        assert!(inbox.fault.is_none());
    }
    #[test]
    fn catalog_handoff_is_bounded_and_leaves_old_storage_for_the_worker() {
        let shared = shared();
        let entries: Arc<[lw_core::queue::Entry]> = (0..10_000)
            .map(|id| lw_core::queue::Entry { id, song_id: 7 })
            .collect();
        let old_entries = Arc::downgrade(&entries);
        let catalog = Catalog {
            queue: Some(Arc::new(lw_ui::queue::Snapshot::new(
                1,
                entries,
                Some(0),
                Arc::new(PlaybackContext::Library),
                false,
            ))),
            ..Catalog::default()
        };
        let mut current = Arc::new(catalog);
        let old = Arc::downgrade(&current);
        shared.inbox.lock().unwrap().catalog = Some(Arc::new(Catalog::new(Vec::new())));
        // Warm the condition variable before measuring exchanges.
        shared.changed.notify_one();
        mem_sys::local_reset();
        assert!(shared.update_catalog(&mut current, &mut |previous, next| {
            assert!(previous.all.is_none());
            assert!(next.all.is_some());
            assert!(
                shared.inbox.try_lock().is_ok(),
                "view-state repair must not hold the mailbox lock"
            );
        }));
        assert_eq!(mem_sys::local_alloc_count(), 0);
        assert_eq!(
            old.strong_count(),
            1,
            "only the worker's retirement slot keeps the old catalog alive"
        );
        shared.inbox.lock().unwrap().catalog = Some(Arc::new(Catalog::default()));
        assert!(
            !shared.update_catalog(&mut current, &mut |_, _| panic!("retirement backpressure"))
        );
        assert!(shared.inbox.lock().unwrap().catalog.is_some());
        assert_eq!(
            old_entries.strong_count(),
            1,
            "the queue array stays in the bounded retirement slot"
        );
        let worker = shared.clone();
        std::thread::spawn(move || {
            let retired = worker.inbox.lock().unwrap().retired_catalog.take();
            drop(retired);
            assert!(
                old.upgrade().is_none(),
                "the worker releases the final strong owner"
            );
            assert!(
                old_entries.upgrade().is_none(),
                "the worker releases large queue arrays too"
            );
        })
        .join()
        .unwrap();
        assert!(shared.update_catalog(&mut current, &mut |previous, next| {
            assert!(previous.all.is_some());
            assert!(next.all.is_none());
        }));
        {
            let mut inbox = shared.inbox.lock().unwrap();
            inbox.retired_catalog.take();
            inbox.catalog = Some(Arc::new(Catalog::new(Vec::new())));
            inbox.closed = true;
        }
        assert!(!shared.update_catalog(&mut current, &mut |_, _| panic!("closed mailbox")));
    }
    #[test]
    fn offline_commands_copy_on_the_library_worker_preserve_playback_and_release_pins() {
        use lw_core::{FileState, Song};
        let base = std::env::temp_dir().join(format!("lw-offline-command-{}", std::process::id()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let shared = shared();
        shared.inbox.lock().unwrap().native.volume = 1.0;
        let worker_base = base.clone();
        let owner = shared.clone();
        let thread = std::thread::spawn(move || {
            Worker::run(
                move || {
                    std::fs::create_dir_all(&worker_base)?;
                    let original = worker_base.join("original.flac");
                    std::fs::write(&original, [7; 64])?;
                    let mut library = LibraryWorker::open(
                        &worker_base.join("library.sqlite"),
                        lw_apple_sys::preferences::Preferences::open(Some(&format!(
                            "localwave.offline-command.{}",
                            std::process::id()
                        )))?,
                        lw_apple_sys::preferences::AppPaths {
                            documents: worker_base.join("Documents"),
                            caches: worker_base.join("Caches"),
                            temporary: worker_base.join("tmp"),
                            test_database: None,
                        },
                    )?;
                    for id in 1..=3 {
                        library
                            .repository
                            .upsert_song(
                                &Song {
                                    song_key: id,
                                    title: format!("Track {id}"),
                                    file_state: if id == 3 {
                                        FileState::CopyPending
                                    } else {
                                        FileState::BookmarkOnly
                                    },
                                    bookmark: Some(lw_apple_sys::bookmarks::create(&original)?),
                                    ..Default::default()
                                },
                                1.0,
                            )
                            .map_err(io::Error::other)?;
                    }
                    Ok(library)
                },
                owner,
            )
        });
        struct Close(Arc<Shared>, Option<JoinHandle<()>>);
        impl Drop for Close {
            fn drop(&mut self) {
                self.0.waveform_cancelled.store(true, Ordering::SeqCst);
                self.0.cancelled.store(true, Ordering::SeqCst);
                self.0.inbox.lock().unwrap().closed = true;
                self.0.changed.notify_one();
                self.1.take().unwrap().join().unwrap();
            }
        }
        let _close = Close(shared.clone(), Some(thread));
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        loop {
            let resumed = shared
                .inbox
                .lock()
                .unwrap()
                .catalog
                .as_ref()
                .is_some_and(|catalog| {
                    catalog
                        .row_index(3)
                        .is_some_and(|index| catalog.row(index).state == FileState::Copied)
                        && catalog.row_index(1).is_some_and(|index| {
                            catalog.row(index).state == FileState::BookmarkOnly
                        })
                });
            if resumed {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "persisted copy requests resume on startup"
            );
            std::thread::yield_now();
        }
        for (revision, ids, pinned) in [
            (1, Arc::<[i64]>::from([1, 2, 1]), true),
            (2, Arc::from([1, 2]), false),
            (3, Arc::from([1, 9000]), false),
        ] {
            let command = Command::TogglePins(ids);
            assert!(command.validate().is_ok());
            assert!(!command.interrupts() && !command.fences_preparation());
            shared.inbox.lock().unwrap().commands.push_back(Envelope {
                audio_notice: None,
                revision,
                preparation: 0,
                command,
            });
            shared.waveform_cancelled.store(true, Ordering::SeqCst);
            shared.changed.notify_one();
            let deadline = std::time::Instant::now() + Duration::from_secs(1);
            loop {
                let done = {
                    let inbox = shared.inbox.lock().unwrap();
                    assert!(
                        inbox.output.is_none(),
                        "offline jobs cannot start or pause native playback"
                    );
                    if revision == 3 {
                        inbox.status.processed_command >= revision
                            && !inbox.status.message.is_empty()
                    } else {
                        inbox.catalog.as_ref().is_some_and(|catalog| {
                            catalog.all.as_ref().is_some_and(|all| {
                                all.rows.len() == 3
                                    && all.rows.iter().all(|index| {
                                        let row = catalog.row(*index);
                                        row.state == FileState::Copied
                                            && row.pinned == (pinned && row.id != 3)
                                    })
                            })
                        })
                    }
                };
                if done {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "offline command {revision} did not finish"
                );
                std::thread::yield_now();
            }
        }
        let inbox = shared.inbox.lock().unwrap();
        assert_eq!(inbox.native.token, 0);
        assert_eq!(inbox.status.queue_len, 0);
        let catalog = inbox.catalog.as_ref().unwrap();
        assert!(
            catalog
                .all
                .as_ref()
                .unwrap()
                .rows
                .iter()
                .all(|index| !catalog.row(*index).pinned)
        );
        assert_eq!(std::fs::read(base.join("original.flac")).unwrap(), [7; 64]);
        let files = std::fs::read_dir(base.join("Documents/Music"))
            .unwrap()
            .count();
        assert_eq!(files, 1, "identical content shares a managed copy");
    }
}
