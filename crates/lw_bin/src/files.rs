//! Use a lazy separate LibraryWorker/SQLite connection so scans/imports cannot occupy playback
//! preparation. Keep one coalesced browse request, one bounded mutation, explicit cancellation and
//! worker-formatted progress. Immutable snapshots, retained selections and old rows retire on the
//! worker. Retry controller delivery until accepted and clear selection on account change. Hidden
//! progress must not force library frames; completed imports refresh the controller catalog.
//!
//! Rescan from a nested directory indexes/imports the whole source. Cancelled scans keep the
//! previous complete index; completed per-file imports remain. Completion/cancellation invalidates
//! the catalog. Capture account/source/root for options, confirmation and picker purposes.
//! Reconnect/removal requires an idle mutation slot; removal needs explicit confirmation. Reject
//! stale roots both in UI and worker.
//!
//! Validate every selected path/folder exclusion before copying anything. Hold each resolved grant
//! only through coordinated bounded copy, cancellation and version checks. Preserve original
//! filenames, identical-content reuse and exclusive UUID-prefixed collision promotion; never
//! overwrite a different track or leave staging files. Named copies skip unused hashing.
//! Register/index/import Added Files through source services and attach private copies as Copied
//! under the cache guard to avoid duplication into managed Music.
//!
//! Mixed imports copy selected audio, import selected folders, then Added Files. Playback order is
//! selected audio order followed by localized natural relative-path order per folder, deduplicated
//! by song key. Preserve shared-song-key semantics, Favorites and membership. indexed_source_songs
//! uses one minimal projection per source, without full-song hydration or per-track queue lookups.
//! Empty-folder success navigates Files without clearing playback.
//!
//! Folder enumeration/import has its own connection and worker, leaving playback preparation free.
//!
//! Source observation: the worker thread owns one `FolderObserver` per known source (created when a
//! snapshot lists it, dropped before release when the source disappears, the account clears or the
//! worker closes). Observer callbacks only mark the source dirty in the inbox and wake the worker.
//! The worker coalesces marks with a trailing `QUIET` debounce and then runs the ordinary rescan
//! job once, never while a user job is pending. Observers watch the registered path only; rescans
//! still resolve the stored bookmark grant. Sources are observed from the first Files use (the
//! worker is lazy).
mod format;
use crate::library::LibraryWorker;
use lw_apple_sys::{
    bookmarks::FileSelection, collation::Key, preferences::file_path, observer::FolderObserver,
};
use lw_core::FileAvailability;
use lw_ui::files::{Directory, Entry, Folder, Glyph, Location, Request, Snapshot, Target};
use std::{
    collections::HashMap,
    io,
    path::Path,
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const QUIET: Duration = Duration::from_secs(2);

/// Trailing debounce per source; callers pass the clock so tests need no sleeps.
#[derive(Default)]
struct Debounce(Vec<(i64, Instant)>);
impl Debounce {
    fn mark(&mut self, source: i64, now: Instant, quiet: Duration) {
        let due = now + quiet;
        match self.0.iter_mut().find(|(id, _)| *id == source) {
            Some(entry) => entry.1 = due,
            None => self.0.push((source, due)),
        }
    }
    fn next(&self, now: Instant) -> Option<Duration> {
        let due = self.0.iter().map(|entry| entry.1).min()?;
        Some(due.saturating_duration_since(now))
    }
    fn take_due(&mut self, now: Instant) -> Option<i64> {
        let at = self.0.iter().position(|entry| entry.1 <= now)?;
        Some(self.0.swap_remove(at).0)
    }
    fn forget(&mut self, source: i64) {
        self.0.retain(|entry| entry.0 != source);
    }
}

enum Job {
    Add {
        user: i64,
        path: FileSelection,
    },
    Selection {
        user: Option<i64>,
        paths: Vec<PathBuf>,
        dropped: bool,
        grants: Option<Vec<FileSelection>>,
    },
    Reconnect {
        target: Target,
        path: FileSelection,
    },
    Remove(Target),
    Rescan(Location),
    Import(Location),
    ImportPath {
        user: i64,
        source: i64,
        path: i64,
    },
}
struct Inbox {
    browse: Option<Request>,
    job: Option<Job>,
    busy: bool,
    dirty: Vec<i64>,
    latest: Option<Arc<Snapshot>>,
    retired: Option<Arc<Snapshot>>,
    closed: bool,
}
struct Shared {
    inbox: Mutex<Inbox>,
    changed: Condvar,
    cancelled: AtomicBool,
    /// None disables observation (tests with exact revision counts and heavy coordinated I/O).
    quiet: Option<Duration>,
    wake: extern "C" fn(),
}
pub struct FilesWorker {
    shared: Arc<Shared>,
    snapshot: Arc<Snapshot>,
    worker: Option<JoinHandle<()>>,
}
impl FilesWorker {
    pub fn open(
        factory: impl FnOnce() -> io::Result<LibraryWorker> + Send + 'static,
        wake: extern "C" fn(),
    ) -> io::Result<Self> {
        Self::open_quiet(factory, wake, Some(QUIET))
    }
    fn open_quiet(
        factory: impl FnOnce() -> io::Result<LibraryWorker> + Send + 'static,
        wake: extern "C" fn(),
        quiet: Option<Duration>,
    ) -> io::Result<Self> {
        let shared = Arc::new(Shared {
            inbox: Mutex::new(Inbox {
                browse: None,
                job: None,
                busy: false,
                dirty: Vec::new(),
                latest: None,
                retired: None,
                closed: false,
            }),
            changed: Condvar::new(),
            cancelled: AtomicBool::new(false),
            quiet,
            wake,
        });
        let owner = shared.clone();
        let worker = std::thread::Builder::new()
            .name("localwave-files".into())
            .spawn(move || run(factory, owner))?;
        Ok(Self {
            shared,
            snapshot: Arc::new(Snapshot::default()),
            worker: Some(worker),
        })
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub fn request(&self, request: Request) -> io::Result<()> {
        match request {
            Request::Cancel => {
                self.shared.cancelled.store(true, Ordering::SeqCst);
                Ok(())
            }
            Request::Roots | Request::Browse(_) => {
                let mut inbox = self.shared.inbox.lock().unwrap();
                if inbox.closed {
                    return Err(io::ErrorKind::BrokenPipe.into());
                }
                inbox.browse = Some(request);
                self.shared.changed.notify_one();
                Ok(())
            }
            Request::Rescan(location) => self.job(Job::Rescan(location)),
            Request::Import(location) => self.job(Job::Import(location)),
            Request::ImportPath { user, source, path } => {
                self.job(Job::ImportPath { user, source, path })
            }
            Request::Remove(target) => self.job(Job::Remove(target)),
        }
    }
    pub fn add_folder(&self, user: i64, path: PathBuf) -> io::Result<()> {
        if !path.is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.add_folder_selected(user, path.into())
    }
    pub fn reconnect_folder(&self, target: Target, path: PathBuf) -> io::Result<()> {
        if !path.is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.reconnect_folder_selected(target, path.into())
    }
    pub fn add_folder_selected(&self, user: i64, path: FileSelection) -> io::Result<()> {
        if !path.path().is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.job(Job::Add { user, path })
    }
    pub fn reconnect_folder_selected(&self, target: Target, path: FileSelection) -> io::Result<()> {
        if !path.path().is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.job(Job::Reconnect { target, path })
    }
    pub fn add_audio_granted(
        &self,
        user: i64,
        paths: Vec<PathBuf>,
        grants: Vec<FileSelection>,
    ) -> io::Result<()> {
        if paths.is_empty()
            || paths.len() > 4096
            || paths.len() != grants.len()
            || paths
                .iter()
                .zip(&grants)
                .any(|(path, file)| !path.is_absolute() || path != file.path())
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.job(Job::Selection {
            user: Some(user),
            paths,
            dropped: false,
            grants: Some(grants),
        })
    }
    pub fn add_audio(&self, user: i64, paths: Vec<PathBuf>) -> io::Result<()> {
        self.selection(Some(user), paths, false)
    }
    pub fn drop_files(&self, user: Option<i64>, paths: Vec<PathBuf>) -> io::Result<()> {
        self.selection(user, paths, true)
    }
    fn selection(&self, user: Option<i64>, paths: Vec<PathBuf>, dropped: bool) -> io::Result<()> {
        if paths.is_empty()
            || paths.len() > 1_000_000
            || paths.iter().any(|path| !path.is_absolute())
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.job(Job::Selection {
            user,
            paths,
            dropped,
            grants: None,
        })
    }
    fn job(&self, job: Job) -> io::Result<()> {
        let mut inbox = self.shared.inbox.lock().unwrap();
        if inbox.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if inbox.busy {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.shared.cancelled.store(false, Ordering::SeqCst);
        inbox.busy = true;
        inbox.job = Some(job);
        self.shared.changed.notify_one();
        Ok(())
    }
    /// Transfer old snapshots back to the worker; large row arrays never retire on a warm UI frame.
    pub fn poll(&mut self) -> bool {
        let mut inbox = self.shared.inbox.lock().unwrap();
        if inbox.retired.is_some() {
            return false;
        }
        let Some(next) = inbox.latest.take() else {
            return false;
        };
        inbox.retired = Some(std::mem::replace(&mut self.snapshot, next));
        self.shared.changed.notify_one();
        true
    }
}
impl Drop for FilesWorker {
    fn drop(&mut self) {
        {
            let mut inbox = self.shared.inbox.lock().unwrap();
            inbox.closed = true;
            self.shared.cancelled.store(true, Ordering::SeqCst);
            self.shared.changed.notify_one();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn publish(shared: &Shared, snapshot: &mut Snapshot) {
    snapshot.revision = snapshot.revision.wrapping_add(1);
    let next = Arc::new(snapshot.clone());
    let (previous, retired) = {
        let mut inbox = shared.inbox.lock().unwrap();
        (inbox.latest.replace(next), inbox.retired.take())
    };
    drop((previous, retired));
    (shared.wake)();
}
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
fn run(factory: impl FnOnce() -> io::Result<LibraryWorker>, shared: Arc<Shared>) {
    let mut snapshot = Snapshot::default();
    // Opening Files is the first reason to open a second database connection.
    {
        let mut inbox = shared.inbox.lock().unwrap();
        while !inbox.closed && inbox.browse.is_none() && inbox.job.is_none() {
            inbox = shared.changed.wait(inbox).unwrap();
        }
        if inbox.closed {
            return;
        }
    }
    let mut library = match factory() {
        Ok(library) => library,
        Err(error) => {
            snapshot.message = error.to_string();
            shared.inbox.lock().unwrap().closed = true;
            publish(&shared, &mut snapshot);
            return;
        }
    };
    let mut debounce = Debounce::default();
    let mut observers: HashMap<i64, (String, FolderObserver)> = HashMap::new();
    loop {
        let (job, browse, retired) = {
            let mut inbox = shared.inbox.lock().unwrap();
            loop {
                let now = Instant::now();
                for source in inbox.dirty.drain(..) {
                    debounce.mark(source, now, shared.quiet.unwrap_or_default());
                }
                if inbox.closed {
                    return;
                }
                if inbox.browse.is_some() || inbox.job.is_some() || inbox.retired.is_some() {
                    break;
                }
                match debounce.next(now) {
                    Some(wait) if wait.is_zero() => break,
                    Some(wait) => inbox = shared.changed.wait_timeout(inbox, wait).unwrap().0,
                    None => inbox = shared.changed.wait(inbox).unwrap(),
                }
            }
            let mut job = inbox.job.take();
            if job.is_none()
                && let Some(source) = debounce.take_due(Instant::now())
                && let (Some(user), Some(folder)) = (
                    snapshot.account,
                    snapshot.sources.iter().find(|folder| folder.id == source),
                )
            {
                shared.cancelled.store(false, Ordering::SeqCst);
                inbox.busy = true;
                job = Some(Job::Rescan(Location {
                    user,
                    source,
                    parent: Some(folder.root),
                }));
            }
            (job, inbox.browse.take(), inbox.retired.take())
        };
        drop(retired);
        if let Some(job) = job {
            snapshot.busy = true;
            snapshot.progress = "Starting…".into();
            snapshot.message.clear();
            publish(&shared, &mut snapshot);
            let result = execute(&mut library, job, &shared, &mut snapshot);
            snapshot.busy = false;
            snapshot.progress.clear();
            // A cancelled import can already have committed completed files.
            snapshot.catalog_revision = snapshot.catalog_revision.wrapping_add(1);
            snapshot.message = match result {
                Ok(message) => message,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                    "Cancelled. Completed imports are kept.".into()
                }
                Err(error) => error.to_string(),
            };
            let roots = snapshot.directory.is_none();
            if let Err(error) = refresh_sources(&mut library, &mut snapshot, roots) {
                clear_account(&mut snapshot, error);
            }
            if let Some(directory) = snapshot.directory.as_ref() {
                let location = directory.location;
                if let Err(error) = browse_directory(&mut library, &mut snapshot, location) {
                    snapshot.message = error.to_string();
                }
            }
            shared.inbox.lock().unwrap().busy = false;
            sync_observers(&mut observers, &snapshot, &shared, &mut debounce);
            publish(&shared, &mut snapshot);
        }
        if let Some(request) = browse {
            snapshot.message.clear();
            let result = match refresh_sources(&mut library, &mut snapshot, matches!(request, Request::Roots)) {
                Err(error) => {
                    clear_account(&mut snapshot, error);
                    None
                }
                Ok(()) => Some(match request {
                    Request::Roots => {
                        snapshot.directory = None;
                        Ok(())
                    }
                    Request::Browse(location) => {
                        browse_directory(&mut library, &mut snapshot, location)
                    }
                    _ => unreachable!(),
                }),
            };
            if let Some(Err(error)) = result {
                if error.kind() == io::ErrorKind::PermissionDenied || snapshot.account.is_none() {
                    clear_account(&mut snapshot, error);
                } else {
                    snapshot.message = error.to_string();
                }
            }
            sync_observers(&mut observers, &snapshot, &shared, &mut debounce);
            publish(&shared, &mut snapshot);
        }
    }
}
/// Best effort: a source whose folder cannot be presented is simply not observed.
fn sync_observers(
    observers: &mut HashMap<i64, (String, FolderObserver)>,
    snapshot: &Snapshot,
    shared: &Arc<Shared>,
    debounce: &mut Debounce,
) {
    if shared.quiet.is_none() {
        return;
    }
    observers.retain(|id, (path, _)| {
        let keep = snapshot
            .sources
            .iter()
            .any(|folder| folder.id == *id && folder.path == *path);
        if !keep {
            debounce.forget(*id);
        }
        keep
    });
    for folder in snapshot.sources.iter() {
        if observers.contains_key(&folder.id) {
            continue;
        }
        let (owner, source) = (shared.clone(), folder.id);
        let observer = FolderObserver::new(Path::new(&folder.path), move || {
            let mut inbox = owner.inbox.lock().unwrap();
            if !inbox.dirty.contains(&source) {
                inbox.dirty.push(source);
            }
            owner.changed.notify_one();
        });
        if let Ok(observer) = observer {
            observers.insert(folder.id, (folder.path.clone(), observer));
        }
    }
}
fn clear_account(snapshot: &mut Snapshot, error: io::Error) {
    snapshot.account = None;
    snapshot.sources = Arc::from([]);
    snapshot.directory = None;
    snapshot.selection = None;
    snapshot.source_count.clear();
    snapshot.message = error.to_string();
}
/// `stats` probes every saved song's availability (Swift's source cards), so only the source list
/// asks for it; folder pages and jobs keep the cheap labels.
fn refresh_sources(
    library: &mut LibraryWorker,
    snapshot: &mut Snapshot,
    stats: bool,
) -> io::Result<()> {
    let account = library.current_sources()?;
    if snapshot.account != Some(account.user.id) {
        snapshot.directory = None;
        snapshot.selection = None;
    }
    snapshot.account = Some(account.user.id);
    let now = now();
    if stats && let Ok((bytes, limit)) = library.cache_usage() {
        const GIB: f64 = 1_073_741_824.0;
        let gigabytes = if limit == 0 { 8 } else { limit >> 30 };
        snapshot.offline_usage = format!("{:.1} of {gigabytes} GB", bytes as f64 / GIB);
        snapshot.offline_fraction = (bytes as f64 / (gigabytes as f64 * GIB)).min(1.0) as f32;
    }
    let sources = account
        .sources
        .into_iter()
        .map(|source| {
            let path = file_path(&source.dir_path)?;
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Music folder")
                .to_owned();
            let id = source
                .id
                .ok_or_else(|| io::Error::other("Source is not saved"))?;
            // Swift shows stats only for sources that have saved songs.
            let (mut songs, mut local, mut cloud, mut pinned) = (0, 0, 0, 0);
            if stats {
                let saved = library.repository.songs_in_source(id).map_err(io::Error::other)?;
                for (availability, kept) in library.song_states(&saved) {
                    songs += 1;
                    local += usize::from(availability == FileAvailability::OnDevice);
                    cloud += usize::from(availability == FileAvailability::InCloud);
                    pinned += usize::from(kept);
                }
            }
            Ok(Folder {
                id,
                root: source.path_id,
                name,
                path: path.to_string_lossy().into_owned(),
                current: source.is_current,
                count: if songs > 0 {
                    format!("{songs} songs")
                } else {
                    format!("{} indexed items", source.total_paths.unwrap_or(0))
                },
                stats: if songs > 0 {
                    format::stats(pinned, local, cloud)
                } else {
                    String::new()
                },
                scanned: format::age(now, source.last_synced_at),
                error: source.sync_error,
                local: if songs > 0 { local as f32 / songs as f32 } else { 0.0 },
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    snapshot.source_count = format!("{} sources, watched for changes", sources.len());
    snapshot.sources = Arc::from(sources);
    Ok(())
}
fn browse_directory(
    library: &mut LibraryWorker,
    snapshot: &mut Snapshot,
    location: Location,
) -> io::Result<()> {
    let source = library.owned_source(location.user, location.source)?;
    let parent = location.parent.unwrap_or(source.path_id);
    let path = if parent == source.path_id {
        None
    } else {
        Some(
            library
                .repository
                .source_path_by_id(location.source, parent)
                .map_err(io::Error::other)?
                .filter(|path| path.is_directory)
                .ok_or_else(|| io::Error::other("This folder was removed. Refresh its source."))?,
        )
    };
    let root = file_path(&source.dir_path)?;
    let root_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Music folder")
        .to_owned();
    let prefix = path.as_ref().map(|path| path.relative_path.as_str());
    // Swift lists every saved song below the folder, in natural relative-path order.
    let mut inside = Vec::new();
    for song in library
        .repository
        .songs_in_source(location.source)
        .map_err(io::Error::other)?
    {
        let Some(relative) = song.relative_path.as_deref() else {
            continue;
        };
        let rest = match prefix {
            None => Some(relative),
            Some(prefix) => relative
                .strip_prefix(prefix)
                .and_then(|rest| rest.strip_prefix('/')),
        };
        if rest.is_some() {
            inside.push((Key::new(relative)?, song));
        }
    }
    inside.sort_unstable_by(|(ak, a), (bk, b)| ak.compare(bk).then(a.id.cmp(&b.id)));
    let inside = inside.into_iter().map(|(_, song)| song).collect::<Vec<_>>();
    let states = library.song_states(&inside);
    let by_path = inside
        .iter()
        .enumerate()
        .filter_map(|(index, song)| Some((song.relative_path.as_deref()?, index)))
        .collect::<HashMap<_, _>>();
    // Per child folder: saved songs and bytes below it (the first path component after the prefix).
    let mut totals: HashMap<&str, (usize, i64)> = HashMap::new();
    for song in &inside {
        let relative = song.relative_path.as_deref().unwrap_or_default();
        let rest = prefix.map_or(relative, |prefix| &relative[prefix.len() + 1..]);
        if let Some((child, _)) = rest.split_once('/') {
            let total = totals.entry(child).or_default();
            total.0 += 1;
            total.1 += song.file_size.unwrap_or(0);
        }
    }
    let mut entries = Vec::new();
    for path in library
        .repository
        .source_paths(location.source, Some(parent))
        .map_err(io::Error::other)?
    {
        let audio = path.is_audio();
        let key = Key::new(&path.name)?;
        let index = by_path.get(path.relative_path.as_str()).copied();
        let song = index.map(|index| &inside[index]);
        let (count, total) = totals.get(path.name.as_str()).copied().unwrap_or_default();
        entries.push((
            Entry {
                id: path.path_id,
                directory: path.is_directory,
                audio,
                number: String::new(),
                kind: if path.is_directory {
                    format!("Folder · {count} songs")
                } else if audio {
                    let extension = song
                        .and_then(|song| song.file_extension.as_deref())
                        .or_else(|| Path::new(&path.name).extension().and_then(|e| e.to_str()))
                        .unwrap_or_default();
                    format!("{extension} audio")
                } else {
                    "Not audio".into()
                },
                size: if path.is_directory {
                    format::bytes(total)
                } else {
                    song.and_then(|song| song.file_size)
                        .or(path.file_size)
                        .map_or_else(|| "—".into(), format::megabytes)
                },
                time: song
                    .and_then(|song| song.duration)
                    .map_or_else(String::new, format::duration),
                detail: {
                    let size = song.and_then(|song| song.file_size).or(path.file_size);
                    if path.is_directory {
                        "Folder".into()
                    } else if !audio {
                        size.map_or_else(|| "not audio".into(), |size| format!("not audio · {}", format::megabytes(size)))
                    } else {
                        let extension = song
                            .and_then(|song| song.file_extension.as_deref())
                            .or_else(|| Path::new(&path.name).extension().and_then(|e| e.to_str()))
                            .unwrap_or_default();
                        let mut parts = vec![extension.to_uppercase()];
                        parts.extend(size.map(format::megabytes));
                        parts.extend(song.and_then(|song| song.duration).map(format::duration));
                        parts.join(" · ")
                    }
                },
                song: song.and_then(|song| song.id),
                glyph: index.map(|index| Glyph {
                    state: inside[index].file_state,
                    availability: Some(states[index].0),
                    pinned: states[index].1,
                }),
                name: path.name,
            },
            key,
        ));
    }
    // Swift's order: folders, then audio, then other files.
    let rank = |entry: &Entry| if entry.directory { 0 } else if entry.audio { 1 } else { 2 };
    entries.sort_unstable_by(|(a, ak), (b, bk)| {
        rank(a)
            .cmp(&rank(b))
            .then_with(|| ak.compare(bk))
            .then(a.id.cmp(&b.id))
    });
    let mut entries = entries
        .into_iter()
        .map(|(entry, _)| entry)
        .collect::<Vec<_>>();
    for (number, entry) in entries.iter_mut().filter(|entry| entry.audio).enumerate() {
        entry.number = (number + 1).to_string();
    }
    // Swift's tabs: folders always, audio by availability; counts cover audio only.
    let tab = |wanted: FileAvailability| {
        entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry.directory
                    || (entry.audio && entry.glyph.is_some_and(|glyph| glyph.availability == Some(wanted)))
            })
            .map(|(index, _)| index as u32)
            .collect::<Vec<_>>()
    };
    let filters = [tab(FileAvailability::OnDevice), tab(FileAvailability::InCloud)];
    let audio = entries.iter().filter(|entry| entry.audio).count();
    let filter_counts = [
        audio.to_string(),
        filters[0].iter().filter(|&&i| entries[i as usize].audio).count().to_string(),
        filters[1].iter().filter(|&&i| entries[i as usize].audio).count().to_string(),
    ];
    let title = path
        .as_ref()
        .map_or_else(|| root_name.clone(), |path| path.name.clone());
    let offline = states
        .iter()
        .filter(|(availability, _)| *availability == FileAvailability::OnDevice)
        .count();
    let directory = Directory {
        source_root: source.path_id,
        location: Location {
            parent: Some(parent),
            ..location
        },
        up: path.as_ref().and_then(|path| path.parent_path_id),
        context: Arc::new(lw_core::context::PlaybackContext::Folder {
            source_id: location.source,
            path_id: Some(parent),
            name: title.clone(),
        }),
        title,
        breadcrumb: prefix.map_or_else(String::new, |prefix| {
            format!("{root_name} / {}", prefix.replace('/', " / "))
        }),
        detail: format!("{} items", entries.len()),
        summary: format!(
            "{} songs · {} · {offline} offline",
            inside.len(),
            format::bytes(inside.iter().filter_map(|song| song.file_size).sum())
        ),
        error: source.sync_error.clone(),
        kept: !states.is_empty() && states.iter().all(|(_, pinned)| *pinned),
        songs: inside.iter().filter_map(|song| song.id).collect(),
        show_non_audio: library.load_settings().show_non_audio,
        non_audio: entries.iter().filter(|entry| !entry.directory && !entry.audio).count(),
        entries,
        filters,
        filter_counts,
    };
    library.select_source(location.user, location.source)?;
    snapshot.directory = Some(Arc::new(directory));
    Ok(())
}
fn execute(
    library: &mut LibraryWorker,
    job: Job,
    shared: &Shared,
    snapshot: &mut Snapshot,
) -> io::Result<String> {
    if shared.cancelled.load(Ordering::SeqCst) {
        return Err(io::ErrorKind::Interrupted.into());
    }
    let (mut location, rescan, single) = match job {
        Job::Remove(target) => {
            library.remove_source(target, &shared.cancelled)?;
            snapshot.directory = None;
            return Ok("Folder removed from the library. Files were not deleted.".into());
        }
        Job::Selection {
            user,
            paths,
            dropped,
            grants,
        } => {
            let user = match user {
                Some(user) => user,
                None => library.current_sources()?.user.id,
            };
            let mut last = Instant::now();
            let import = if dropped {
                LibraryWorker::add_dropped
            } else {
                LibraryWorker::add_audio
            };
            let mut progress = |stage: &str, completed: usize, total: usize| {
                if completed <= 1 || last.elapsed().as_millis() >= 100 {
                    snapshot.progress = if total == 0 {
                        format!("{stage}: {completed} items")
                    } else {
                        format!("{stage}: {completed} of {total} files")
                    };
                    publish(shared, snapshot);
                    last = Instant::now();
                }
            };
            let selection = if let Some(grants) = grants {
                library.add_audio_granted(
                    user,
                    (&paths, &grants),
                    now(),
                    &shared.cancelled,
                    &mut progress,
                )?
            } else {
                import(
                    library,
                    user,
                    &paths,
                    now(),
                    &shared.cancelled,
                    &mut progress,
                )?
            };
            let location = Location {
                user,
                source: selection
                    .source
                    .id
                    .ok_or_else(|| io::Error::other("Source is not saved"))?,
                parent: Some(selection.source.path_id),
            };
            refresh_sources(library, snapshot, false)?;
            if dropped {
                snapshot.directory = None;
            } else {
                browse_directory(library, snapshot, location)?;
            }
            let count = selection.songs.len();
            snapshot.selection = Some(lw_ui::files::Selection {
                revision: snapshot.catalog_revision.wrapping_add(1),
                songs: selection.songs,
                context: selection.context,
                dropped,
            });
            return Ok(format!("{count} selected tracks added."));
        }
        Job::Reconnect { target, path } => {
            let source =
                library.reconnect_selected_folder(target, &path, now(), &shared.cancelled)?;
            let location = Location {
                user: target.user,
                source: target.source,
                parent: Some(source.path_id),
            };
            refresh_sources(library, snapshot, false)?;
            browse_directory(library, snapshot, location)?;
            (location, true, None)
        }
        Job::Add { user, path } => {
            let source = library.register_selected_folder(user, &path, now(), &shared.cancelled)?;
            let location = Location {
                user,
                source: source
                    .id
                    .ok_or_else(|| io::Error::other("Source is not saved"))?,
                parent: Some(source.path_id),
            };
            (location, true, None)
        }
        Job::Rescan(location) => (location, true, None),
        Job::Import(location) => (location, false, None),
        Job::ImportPath { user, source, path } => (
            Location {
                user,
                source,
                parent: None,
            },
            false,
            Some(path),
        ),
    };
    let mut source = library.owned_source(location.user, location.source)?;
    let mut last = Instant::now();
    if rescan {
        let mut count = 0;
        source = library.scan_source(location.source, now(), &shared.cancelled, &mut |_| {
            count += 1;
            if count == 1 || last.elapsed().as_millis() >= 100 {
                snapshot.progress = format!("Indexing: {count} items");
                publish(shared, snapshot);
                last = Instant::now();
            }
        })?;
        location.parent = Some(source.path_id);
    }
    let selected = if let Some(path) = single {
        vec![
            library
                .repository
                .source_path_by_id(location.source, path)
                .map_err(io::Error::other)?
                .ok_or_else(|| io::Error::other("This file was removed. Refresh its source."))?,
        ]
    } else {
        let parent = location.parent.unwrap_or(source.path_id);
        if parent != source.path_id {
            let path = library
                .repository
                .source_path_by_id(location.source, parent)
                .map_err(io::Error::other)?
                .filter(|path| path.is_directory)
                .ok_or_else(|| io::Error::other("This folder was removed. Refresh its source."))?;
            vec![path]
        } else {
            library
                .repository
                .source_paths(location.source, Some(parent))
                .map_err(io::Error::other)?
        }
    };
    let summary = library.import_paths(
        &selected,
        now(),
        &shared.cancelled,
        &mut |_, completed, total| {
            if completed == 0 || last.elapsed().as_millis() >= 100 {
                snapshot.progress = format!("Importing: {completed} of {total} files");
                publish(shared, snapshot);
                last = Instant::now();
            }
        },
    )?;
    refresh_sources(library, snapshot, false)?;
    browse_directory(library, snapshot, location)?;
    Ok(format!(
        "{} imported, {} unchanged, {} awaiting download.",
        summary.imported, summary.skipped, summary.deferred
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    use std::{sync::mpsc, time::Duration};
    extern "C" fn wake() {}
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn until(worker: &mut FilesWorker, done: impl Fn(&Snapshot) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            worker.poll();
            if done(worker.snapshot()) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "folder worker did not finish: {} / {}",
                worker.snapshot().progress,
                worker.snapshot().message
            );
            std::thread::yield_now();
        }
    }

    #[test]
    fn debounce_coalesces_marks_and_reports_deadlines_from_the_injected_clock() {
        let start = Instant::now();
        let quiet = Duration::from_secs(2);
        let mut debounce = Debounce::default();
        assert_eq!(debounce.next(start), None);
        debounce.mark(1, start, quiet);
        debounce.mark(1, start + Duration::from_secs(1), quiet);
        debounce.mark(2, start + Duration::from_secs(1), quiet);
        assert_eq!(debounce.next(start), Some(Duration::from_secs(3)));
        assert_eq!(debounce.take_due(start + Duration::from_secs(2)), None);
        assert_eq!(debounce.take_due(start + Duration::from_secs(3)), Some(1));
        debounce.forget(2);
        assert_eq!(debounce.take_due(start + Duration::from_secs(9)), None);
        assert_eq!(debounce.next(start), None);
    }

    #[test]
    fn dirty_sources_rescan_once_after_the_debounce() {
        let base = std::env::temp_dir().join(format!("lw-files-dirty-{}", std::process::id()));
        let _cleanup = Cleanup(base.clone());
        let music = base.join("Music");
        std::fs::create_dir_all(&music).unwrap();
        let tagged = include_bytes!("../../../Tests/LocalWaveDataTests/Fixtures/tagged.flac");
        std::fs::write(music.join("01.flac"), tagged).unwrap();
        let worker_base = base.clone();
        let mut worker = FilesWorker::open_quiet(
            move || {
                LibraryWorker::open(
                    &worker_base.join("library.sqlite"),
                    Preferences::open(Some(&format!(
                        "localwave.files-dirty.{}",
                        std::process::id()
                    )))?,
                    AppPaths {
                        documents: worker_base.join("Documents"),
                        caches: worker_base.join("Caches"),
                        temporary: worker_base.join("tmp"),
                        test_database: None,
                    },
                )
            },
            wake,
            Some(Duration::from_millis(30)),
        )
        .unwrap();
        worker.request(Request::Roots).unwrap();
        until(&mut worker, |snapshot| snapshot.account.is_some());
        let user = worker.snapshot().account.unwrap();
        let (selection, _) = sdl3_sys::test_util::file_selection(&music).unwrap();
        worker.add_folder_selected(user, selection).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 1 && !snapshot.busy
        });
        let (source, root) = {
            let folder = &worker.snapshot().sources[0];
            (folder.id, folder.root)
        };
        std::fs::write(music.join("02.flac"), tagged).unwrap();
        // Same inbox path as an observer callback; repeated marks coalesce into one rescan.
        for _ in 0..3 {
            let mut inbox = worker.shared.inbox.lock().unwrap();
            inbox.dirty.push(source);
            worker.shared.changed.notify_one();
        }
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision >= 2 && !snapshot.busy
        });
        assert_eq!(
            worker.snapshot().directory.as_ref().unwrap().entries.len(),
            2
        );
        let target = Target { user, source, root };
        worker.request(Request::Remove(target)).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.sources.is_empty() && !snapshot.busy
        });
    }

    #[test]
    fn a_first_drop_opens_the_lazy_worker_and_empty_folders_do_not_request_playback() {
        let base = std::env::temp_dir().join(format!("lw-first-drop-{}", std::process::id()));
        let _cleanup = Cleanup(base.clone());
        let folder = base.join("Empty music");
        std::fs::create_dir_all(&folder).unwrap();
        let worker_base = base.clone();
        let mut worker = FilesWorker::open(
            move || {
                LibraryWorker::open(
                    &worker_base.join("library.sqlite"),
                    Preferences::open(Some(&format!(
                        "localwave.first-drop.{}",
                        std::process::id()
                    )))?,
                    AppPaths {
                        documents: worker_base.join("Documents"),
                        caches: worker_base.join("Caches"),
                        temporary: worker_base.join("tmp"),
                        test_database: None,
                    },
                )
            },
            wake,
        )
        .unwrap();
        assert!(worker.snapshot().account.is_none());
        worker.drop_files(None, vec![folder]).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 1 && !snapshot.busy
        });
        assert!(worker.snapshot().account.is_some());
        assert_eq!(worker.snapshot().sources.len(), 1);
        assert!(worker.snapshot().directory.is_none());
        let selection = worker.snapshot().selection.as_ref().unwrap();
        assert!(selection.dropped);
        assert!(selection.songs.is_empty());
        assert_eq!(selection.context.title(), "Dropped music");
        assert!(
            worker
                .snapshot()
                .message
                .contains("0 selected tracks added")
        );
    }

    #[test]
    fn files_mailbox_coalesces_browsing_bounds_mutation_and_returns_old_snapshots_to_worker() {
        let (started, seen) = mpsc::sync_channel(1);
        let (release, blocked) = mpsc::sync_channel(1);
        struct Release(mpsc::SyncSender<()>);
        impl Drop for Release {
            fn drop(&mut self) {
                let _ = self.0.try_send(());
            }
        }
        let mut worker = FilesWorker::open(
            move || {
                started.send(()).unwrap();
                blocked.recv().unwrap();
                Err(io::ErrorKind::Interrupted.into())
            },
            wake,
        )
        .unwrap();
        let release = Release(release);
        worker.request(Request::Roots).unwrap();
        seen.recv_timeout(Duration::from_secs(1)).unwrap();
        let location = Location {
            user: 1,
            source: 2,
            parent: None,
        };
        for parent in 0..100 {
            worker
                .request(Request::Browse(Location {
                    parent: Some(parent),
                    ..location
                }))
                .unwrap();
        }
        assert_eq!(
            worker.shared.inbox.lock().unwrap().browse,
            Some(Request::Browse(Location {
                parent: Some(99),
                ..location
            }))
        );
        worker.add_folder(1, PathBuf::from("/Music")).unwrap();
        assert_eq!(
            worker
                .request(Request::Rescan(location))
                .unwrap_err()
                .kind(),
            io::ErrorKind::WouldBlock
        );
        worker.request(Request::Cancel).unwrap();
        assert!(worker.shared.cancelled.load(Ordering::SeqCst));
        // Keep a weak reference so the exact last owner is observable before native setup resumes.
        let mut snapshot = Snapshot {
            revision: 3,
            ..Default::default()
        };
        publish(&worker.shared, &mut snapshot);
        worker.poll();
        let old = Arc::downgrade(&worker.snapshot);
        publish(&worker.shared, &mut snapshot);
        mem_sys::local_reset();
        assert!(worker.poll());
        assert!(
            old.upgrade().is_some(),
            "poll must retain, not free, its previous snapshot"
        );
        for _ in 0..100 {
            assert!(!worker.poll());
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        publish(&worker.shared, &mut snapshot);
        assert!(
            old.upgrade().is_none(),
            "publication drains retired storage on the worker path"
        );
        release.0.send(()).unwrap();
        until(&mut worker, |snapshot| !snapshot.message.is_empty());
    }

    #[test]
    fn files_worker_adds_and_imports_recursive_folders_browses_rescans_and_rejects_stale_accounts()
    {
        let base = std::env::temp_dir().join(format!("lw-files-worker-{}", std::process::id()));
        let _cleanup = Cleanup(base.clone());
        let music = base.join("Original music 名前");
        std::fs::create_dir_all(music.join("Album")).unwrap();
        std::fs::write(
            music.join("Album/01.flac"),
            include_bytes!("../../../Tests/LocalWaveDataTests/Fixtures/tagged.flac"),
        )
        .unwrap();
        std::fs::write(music.join("notes.txt"), "not audio").unwrap();
        let worker_base = base.clone();
        let mut worker = FilesWorker::open_quiet(
            move || {
                LibraryWorker::open(
                    &worker_base.join("library.sqlite"),
                    Preferences::open(Some(&format!(
                        "localwave.files-worker.{}",
                        std::process::id()
                    )))?,
                    AppPaths {
                        documents: worker_base.join("Documents"),
                        caches: worker_base.join("Caches"),
                        temporary: worker_base.join("tmp"),
                        test_database: None,
                    },
                )
            },
            wake,
            // Exact revision counts and heavy coordinated I/O: observing the source adds latency.
            None,
        )
        .unwrap();
        worker.request(Request::Roots).unwrap();
        until(&mut worker, |snapshot| snapshot.account.is_some());
        let user = worker.snapshot().account.unwrap();
        let (selection, identity) = sdl3_sys::test_util::file_selection(&music).unwrap();
        worker.add_folder_selected(user, selection).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 1 && !snapshot.busy
        });
        assert_eq!(worker.snapshot().sources.len(), 1);
        let (seen, calls, starts, stops) = sdl3_sys::test_util::file_selection_status();
        assert_eq!((seen, calls), (identity, 1));
        assert_eq!(starts, stops);
        assert!(
            worker.snapshot().message.starts_with("1 imported"),
            "{}",
            worker.snapshot().message
        );
        let directory = worker.snapshot().directory.as_ref().unwrap();
        assert_eq!(
            directory.entries.len(),
            2,
            "the desktop list shows non-audio files after folders and audio"
        );
        assert_eq!(directory.entries[0].name, "Album");
        assert_eq!(directory.entries[0].kind, "Folder · 1 songs");
        assert!(directory.entries[0].size.ends_with(" MB"));
        let notes = &directory.entries[1];
        assert_eq!(
            (notes.name.as_str(), notes.kind.as_str(), notes.glyph, notes.audio),
            ("notes.txt", "Not audio", None, false)
        );
        assert_eq!((directory.non_audio, directory.songs.len()), (1, 1));
        assert!(!directory.show_non_audio, "the setting only affects compact lists");
        assert!(directory.summary.starts_with("1 songs · "), "{}", directory.summary);
        assert!(directory.summary.ends_with(" offline"));
        assert_eq!(directory.title, "Original music 名前");
        assert_eq!(directory.breadcrumb, "", "no breadcrumb at the source root");
        let root = directory.location;
        let nested = Location {
            parent: Some(directory.entries[0].id),
            ..root
        };
        let before = worker.snapshot().revision;
        worker.request(Request::Browse(nested)).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.revision > before
                && snapshot
                    .directory
                    .as_ref()
                    .is_some_and(|directory| directory.location == nested)
        });
        let directory = worker.snapshot().directory.as_ref().unwrap();
        assert_eq!(directory.up, root.parent);
        let entry = &directory.entries[0];
        assert_eq!(entry.name, "01.flac");
        assert_eq!((entry.number.as_str(), entry.kind.as_str()), ("1", "flac audio"));
        assert!(entry.size.ends_with(" MB") && entry.time.contains(':'), "{} {}", entry.size, entry.time);
        assert!(entry.glyph.is_some_and(|glyph| !glyph.pinned));
        // The compact tabs: the saved song, and which tab lists it (a local file is on device).
        assert_eq!(entry.song, directory.songs.first().copied());
        assert_eq!(directory.filter_counts, ["1", "1", "0"]);
        assert_eq!((&directory.filters[0][..], &directory.filters[1][..]), (&[0][..], &[][..]));
        assert_eq!(directory.breadcrumb, "Original music 名前 / Album");
        assert_eq!(directory.title, "Album");
        assert_eq!(directory.songs.len(), 1);
        assert!(matches!(
            directory.context.as_ref(),
            lw_core::context::PlaybackContext::Folder { name, path_id: Some(_), .. } if name == "Album"
        ));
        std::fs::write(
            music.join("02.flac"),
            include_bytes!("../../../Tests/LocalWaveDataTests/Fixtures/tagged.flac"),
        )
        .unwrap();
        worker.request(Request::Rescan(nested)).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 2 && !snapshot.busy
        });
        let directory = worker.snapshot().directory.as_ref().unwrap();
        assert_eq!(
            directory.location, root,
            "rescan updates the entire source, even from a nested page"
        );
        assert_eq!(directory.entries.len(), 3, "Album, 02.flac, then notes.txt");
        assert_eq!(directory.entries[1].name, "02.flac");
        let stale = Location {
            user: user.checked_add(1).unwrap(),
            ..root
        };
        let before = worker.snapshot().revision;
        worker.request(Request::Browse(stale)).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.revision > before && !snapshot.message.is_empty()
        });
        assert!(worker.snapshot().directory.is_none());
        assert!(worker.snapshot().account.is_none());
        worker.request(Request::Roots).unwrap();
        until(&mut worker, |snapshot| snapshot.account == Some(user));
        assert_eq!(
            worker.snapshot().sources.len(),
            1,
            "stale requests must not remove the registered source"
        );
        let card = &worker.snapshot().sources[0];
        assert_eq!(card.name, "Original music 名前");
        assert!(card.count.ends_with(" songs") && !card.count.starts_with('0'), "{}", card.count);
        assert!(card.stats.starts_with("0 kept · "), "{}", card.stats);
        assert!(card.scanned.ends_with(" ago"), "{}", card.scanned);
        assert!(card.error.is_none() && card.local >= 0.0 && card.local <= 1.0);
        assert!(worker.snapshot().offline_usage.ends_with(" GB"), "{}", worker.snapshot().offline_usage);
        assert!(worker.snapshot().source_count.ends_with(" sources, watched for changes"));
        worker
            .add_folder(user, base.join("Documents/CoverArt"))
            .unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 3 && !snapshot.busy
        });
        assert_eq!(worker.snapshot().sources.len(), 1);
        assert!(!worker.snapshot().message.is_empty());
        let target = Target {
            user,
            source: worker.snapshot().sources[0].id,
            root: worker.snapshot().sources[0].root,
        };
        let database = base.join("library.sqlite");
        let (song_id, list_id, other_source, other_song) = {
            let mut repository =
                lw_core::LibraryRepository::open(database.to_str().unwrap()).unwrap();
            let song = repository.search_songs("", 10, 0).unwrap().remove(0);
            let id = song.id.unwrap();
            repository.set_favorite(id, true).unwrap();
            let list = repository.create_playlist("Keep my song IDs", 1.0).unwrap();
            repository.add_playlist_songs(list.id, &[id]).unwrap();
            let other = repository.get_or_create_user(i64::MIN).unwrap();
            let source = repository
                .create_source(&lw_core::Source {
                    user_id: other.id,
                    dir_path: "/Other account".into(),
                    path_id: 17,
                    ..Default::default()
                })
                .unwrap();
            let other_song = repository
                .upsert_song(
                    &lw_core::Song {
                        song_key: 98765,
                        title: "Other account's track".into(),
                        source_id: source.id,
                        ..Default::default()
                    },
                    1.0,
                )
                .unwrap();
            // Exercise a real legacy row whose identity must migrate before reconnecting its root.
            lw_sqlite::Connection::open(database.to_str().unwrap())
                .unwrap()
                .execute(
                    "UPDATE songs SET sourceId=NULL,relativePath=NULL WHERE id=?",
                    &[lw_sqlite::Value::Integer(id)],
                )
                .unwrap();
            (id, list.id, source, other_song)
        };
        let moved = base.join("Reconnected music 名前");
        std::fs::rename(&music, &moved).unwrap();
        let (selection, identity) = sdl3_sys::test_util::file_selection(&moved).unwrap();
        worker.reconnect_folder_selected(target, selection).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 4 && !snapshot.busy
        });
        assert_eq!(worker.snapshot().sources.len(), 1);
        assert_eq!(worker.snapshot().sources[0].id, target.source);
        let (seen, calls, starts, stops) = sdl3_sys::test_util::file_selection_status();
        assert_eq!((seen, calls), (identity, 2));
        assert_eq!(starts, stops);
        assert_ne!(worker.snapshot().sources[0].root, target.root);
        assert_eq!(
            worker.snapshot().directory.as_ref().unwrap().entries.len(),
            3,
            "Album, 02.flac and notes.txt: {}",
            worker.snapshot().message
        );
        {
            let repository = lw_core::LibraryRepository::open(database.to_str().unwrap()).unwrap();
            let song = repository.get_song(song_id).unwrap().unwrap();
            assert_eq!(song.source_id, Some(target.source));
            assert!(song.relative_path.is_some());
            assert_eq!(song.is_favorite, 1);
            assert_eq!(
                repository.playlist_songs(list_id).unwrap()[0].id,
                Some(song_id)
            );
        }
        worker.request(Request::Remove(target)).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 5 && !snapshot.busy
        });
        assert_eq!(
            worker.snapshot().sources.len(),
            1,
            "the old root captured by a dialog must be rejected"
        );
        worker
            .request(Request::Remove(Target {
                user,
                source: other_source.id.unwrap(),
                root: other_source.path_id,
            }))
            .unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 6 && !snapshot.busy
        });
        assert_eq!(worker.snapshot().sources.len(), 1);
        let current = Target {
            root: worker.snapshot().sources[0].root,
            ..target
        };
        worker.request(Request::Remove(current)).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 7 && !snapshot.busy
        });
        assert!(worker.snapshot().sources.is_empty());
        assert!(worker.snapshot().directory.is_none());
        assert!(moved.join("Album/01.flac").is_file());
        assert!(moved.join("02.flac").is_file());
        let repository = lw_core::LibraryRepository::open(database.to_str().unwrap()).unwrap();
        assert!(repository.get_song(song_id).unwrap().is_none());
        assert!(repository.playlist_songs(list_id).unwrap().is_empty());
        assert_eq!(
            repository.source(other_source.id.unwrap()).unwrap(),
            Some(other_source)
        );
        assert_eq!(
            repository.get_song(other_song.id.unwrap()).unwrap(),
            Some(other_song)
        );
        let paths = vec![moved.join("02.flac"), moved.join("Album/01.flac")];
        let grants: Vec<_> = paths
            .iter()
            .map(|path| sdl3_sys::test_util::file_selection(path).unwrap())
            .collect();
        let identity = grants[1].1;
        assert!(
            worker
                .add_audio_granted(user, paths.clone(), vec![grants[0].0.clone()])
                .is_err()
        );
        worker
            .add_audio_granted(
                user,
                paths,
                grants.into_iter().map(|(file, _)| file).collect(),
            )
            .unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 8 && !snapshot.busy
        });
        let (seen, calls, starts, stops) = sdl3_sys::test_util::file_selection_status();
        assert_eq!((seen, calls), (identity, 4));
        assert_eq!(starts, stops);
        assert!(
            worker.snapshot().message.contains("selected tracks added"),
            "{}",
            worker.snapshot().message
        );
        assert_eq!(
            worker.snapshot().directory.as_ref().unwrap().title,
            "Added Files"
        );
        let selection = worker.snapshot().selection.as_ref().unwrap();
        assert_eq!(selection.revision, 8);
        assert_eq!(
            selection.songs.len(),
            1,
            "matching song keys are deduplicated in selection order"
        );
        let imported = repository.get_song(selection.songs[0]).unwrap().unwrap();
        let relative = imported.relative_path.as_deref().unwrap();
        assert!(["02.flac", "01.flac"].contains(&relative));
        assert_eq!(
            imported.local_file_path.as_deref(),
            Some(format!("Added Files/{relative}").as_str())
        );
        assert_eq!(imported.file_state, lw_core::FileState::Copied);
        assert!(moved.join("02.flac").is_file());
        worker.drop_files(None, vec![moved.clone()]).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 9 && !snapshot.busy
        });
        assert!(worker.snapshot().directory.is_none());
        let dropped = worker.snapshot().selection.as_ref().unwrap();
        assert!(dropped.dropped);
        assert!(!dropped.songs.is_empty());
        assert_eq!(dropped.context.title(), "Dropped music");
        let before = worker.snapshot().selection.as_ref().unwrap().revision;
        worker.drop_files(Some(user + 1), vec![moved]).unwrap();
        until(&mut worker, |snapshot| {
            snapshot.catalog_revision == 10 && !snapshot.busy
        });
        assert_eq!(
            worker.snapshot().selection.as_ref().unwrap().revision,
            before
        );
        assert!(!worker.snapshot().message.is_empty());
    }
}
