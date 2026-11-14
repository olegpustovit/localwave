//! Folder enumeration/import has its own connection and worker, leaving playback preparation free.
use crate::library::LibraryWorker;
use lw_apple_sys::{bookmarks::FileSelection, collation::Key, preferences::file_path};
use lw_ui::files::{Directory, Entry, Folder, Location, Request, Snapshot, Target};
use std::{
    io,
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

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
    latest: Option<Arc<Snapshot>>,
    retired: Option<Arc<Snapshot>>,
    closed: bool,
}
struct Shared {
    inbox: Mutex<Inbox>,
    changed: Condvar,
    cancelled: AtomicBool,
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
        let shared = Arc::new(Shared {
            inbox: Mutex::new(Inbox {
                browse: None,
                job: None,
                busy: false,
                latest: None,
                retired: None,
                closed: false,
            }),
            changed: Condvar::new(),
            cancelled: AtomicBool::new(false),
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
    loop {
        let (job, browse, retired) = {
            let mut inbox = shared.inbox.lock().unwrap();
            while !inbox.closed
                && inbox.browse.is_none()
                && inbox.job.is_none()
                && inbox.retired.is_none()
            {
                inbox = shared.changed.wait(inbox).unwrap();
            }
            if inbox.closed {
                return;
            }
            (inbox.job.take(), inbox.browse.take(), inbox.retired.take())
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
            if let Err(error) = refresh_sources(&mut library, &mut snapshot) {
                clear_account(&mut snapshot, error);
            }
            if let Some(directory) = snapshot.directory.as_ref() {
                let location = directory.location;
                if let Err(error) = browse_directory(&mut library, &mut snapshot, location) {
                    snapshot.message = error.to_string();
                }
            }
            shared.inbox.lock().unwrap().busy = false;
            publish(&shared, &mut snapshot);
        }
        if let Some(request) = browse {
            snapshot.message.clear();
            let result = match refresh_sources(&mut library, &mut snapshot) {
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
            publish(&shared, &mut snapshot);
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
fn refresh_sources(library: &mut LibraryWorker, snapshot: &mut Snapshot) -> io::Result<()> {
    let account = library.current_sources()?;
    if snapshot.account != Some(account.user.id) {
        snapshot.directory = None;
        snapshot.selection = None;
    }
    snapshot.account = Some(account.user.id);
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
            Ok(Folder {
                id: source
                    .id
                    .ok_or_else(|| io::Error::other("Source is not saved"))?,
                root: source.path_id,
                name,
                path: path.to_string_lossy().into_owned(),
                current: source.is_current,
                detail: source.sync_error.unwrap_or_else(|| {
                    source.total_paths.map_or_else(
                        || "Not indexed yet".into(),
                        |count| format!("{count} audio files"),
                    )
                }),
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    snapshot.source_count = format!("{} sources", sources.len());
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
    let mut entries = Vec::new();
    let show_non_audio = library.load_settings().show_non_audio;
    for path in library
        .repository
        .source_paths(location.source, Some(parent))
        .map_err(io::Error::other)?
    {
        if !path.is_directory && !path.is_audio() && !show_non_audio {
            continue;
        }
        let key = Key::new(&path.name)?;
        entries.push((
            Entry {
                id: path.path_id,
                directory: path.is_directory,
                audio: path.is_audio(),
                name: path.name,
                detail: if path.is_directory {
                    "Folder".into()
                } else {
                    path.file_size
                        .map_or_else(|| "File".into(), |bytes| format!("{bytes} bytes"))
                },
            },
            key,
        ));
    }
    entries.sort_unstable_by(|(a, ak), (b, bk)| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| ak.compare(bk))
            .then(a.id.cmp(&b.id))
    });
    let entries = entries
        .into_iter()
        .map(|(entry, _)| entry)
        .collect::<Vec<_>>();
    let root = file_path(&source.dir_path)?;
    let directory = Directory {
        source_root: source.path_id,
        location: Location {
            parent: Some(parent),
            ..location
        },
        up: path.as_ref().and_then(|path| path.parent_path_id),
        title: path.as_ref().map_or_else(
            || {
                root.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Music folder")
                    .to_owned()
            },
            |path| path.name.clone(),
        ),
        path: path.as_ref().map_or_else(
            || root.to_string_lossy().into_owned(),
            |path| path.relative_path.clone(),
        ),
        detail: format!("{} items", entries.len()),
        entries,
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
            refresh_sources(library, snapshot)?;
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
            refresh_sources(library, snapshot)?;
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
    refresh_sources(library, snapshot)?;
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
        let mut worker = FilesWorker::open(
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
            1,
            "non-audio files are hidden by default"
        );
        assert_eq!(directory.entries[0].name, "Album");
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
        assert_eq!(directory.entries[0].name, "01.flac");
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
        assert_eq!(directory.entries.len(), 2);
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
            lw_apple_sys::sqlite::Connection::open(database.to_str().unwrap())
                .unwrap()
                .execute(
                    "UPDATE songs SET sourceId=NULL,relativePath=NULL WHERE id=?",
                    &[lw_apple_sys::sqlite::Value::Integer(id)],
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
            2,
            "{}",
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
