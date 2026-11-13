use super::LibraryWorker;
use lw_apple_sys::artwork::{self, Thumbnail};
use lw_ui::artwork::{Key, REQUEST_CAPACITY, Request};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::{io, path::Path};

impl LibraryWorker {
    pub fn artwork_thumbnail(&self, stored_path: &str, max_pixels: u32) -> io::Result<Thumbnail> {
        load(&self.paths.documents, stored_path, max_pixels)
    }
}

fn load(documents: &Path, stored_path: &str, max_pixels: u32) -> io::Result<Thumbnail> {
    let path = Path::new(stored_path);
    let file = if path.is_absolute() {
        path.to_owned()
    } else {
        lw_core::file_access::source_file(documents, stored_path)?
    };
    artwork::thumbnail(&file, max_pixels)
}

pub struct ArtworkReady {
    pub revision: u64,
    pub epoch: u64,
    pub key: Key,
    pub thumbnail: Option<Thumbnail>,
}
type Batch = [Option<Arc<ArtworkReady>>; REQUEST_CAPACITY];
const READY_BYTE_LIMIT: usize = 8 * 1024 * 1024;

struct Mailbox {
    request: Option<Request>,
    results: VecDeque<Arc<ArtworkReady>>,
    retired: Batch,
    final_results: Batch,
    closed: bool,
}
struct Shared {
    mailbox: Mutex<Mailbox>,
    changed: Condvar,
    wake: extern "C" fn(),
}
pub struct ArtworkWorker {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
    results: Batch,
    sent: Request,
}
impl ArtworkWorker {
    /// Start before elevating the UI thread's QoS. Decoding never competes with playback's worker.
    pub fn open(documents: Option<PathBuf>, wake: extern "C" fn()) -> io::Result<Self> {
        let shared = Arc::new(Shared {
            mailbox: Mutex::new(Mailbox {
                request: None,
                results: VecDeque::with_capacity(REQUEST_CAPACITY),
                retired: [const { None }; REQUEST_CAPACITY],
                final_results: [const { None }; REQUEST_CAPACITY],
                closed: false,
            }),
            changed: Condvar::new(),
            wake,
        });
        let owner = shared.clone();
        let worker = std::thread::Builder::new()
            .name("localwave-artwork".into())
            .spawn(move || {
                let documents = documents.map(Ok).unwrap_or_else(|| {
                    lw_apple_sys::preferences::AppPaths::load().map(|paths| paths.documents)
                });
                let mut pending = Request::default();
                let (mut next, mut revision) = (0, 0u64);
                loop {
                    let (key, retired, retry) =
                        {
                            let mut inbox = owner.mailbox.lock().unwrap();
                            loop {
                                if inbox.closed {
                                    let remaining = (
                                        std::mem::take(&mut inbox.results),
                                        std::mem::replace(
                                            &mut inbox.retired,
                                            [const { None }; REQUEST_CAPACITY],
                                        ),
                                        std::mem::replace(
                                            &mut inbox.final_results,
                                            [const { None }; REQUEST_CAPACITY],
                                        ),
                                    );
                                    drop(inbox);
                                    drop(remaining);
                                    return;
                                }
                                if let Some(request) = inbox.request.take() {
                                    pending = request;
                                    next = 0;
                                }
                                if inbox.retired.iter().any(Option::is_some) {
                                    let retired = std::mem::replace(
                                        &mut inbox.retired,
                                        [const { None }; REQUEST_CAPACITY],
                                    );
                                    break (None, retired, !inbox.results.is_empty());
                                }
                                if let Some(key) = pending.keys().get(next).copied() {
                                    if inbox.results.iter().any(|ready| {
                                        ready.epoch == pending.epoch() && ready.key == key
                                    }) {
                                        next += 1;
                                        continue;
                                    }
                                    let bytes: usize = inbox
                                        .results
                                        .iter()
                                        .filter_map(|ready| ready.thumbnail.as_ref())
                                        .map(|image| image.pixels.len())
                                        .sum();
                                    let required =
                                        key.max_pixels as usize * key.max_pixels as usize * 4;
                                    if inbox.results.len() < REQUEST_CAPACITY
                                        && bytes + required <= READY_BYTE_LIMIT
                                    {
                                        next += 1;
                                        break (
                                            Some((key, pending.epoch())),
                                            [const { None }; REQUEST_CAPACITY],
                                            false,
                                        );
                                    }
                                }
                                inbox = owner.changed.wait(inbox).unwrap();
                            }
                        };
                    drop(retired);
                    if retry {
                        (owner.wake)();
                    }
                    let Some((key, epoch)) = key else {
                        continue;
                    };
                    let decoded = match &documents {
                        Ok(documents) => load(documents, key.path.as_str(), key.max_pixels),
                        Err(error) => Err(io::Error::other(error.to_string())),
                    };
                    let thumbnail = match decoded {
                        Ok(image) => Some(image),
                        Err(error) => {
                            eprintln!("LocalWave artwork {}: {error}", key.path.as_str());
                            None
                        }
                    };
                    revision = revision.checked_add(1).expect("Artwork identity exhausted");
                    let ready = Arc::new(ArtworkReady {
                        revision,
                        epoch,
                        key,
                        thumbnail,
                    });
                    let mut inbox = owner.mailbox.lock().unwrap();
                    if inbox.closed
                        || inbox.request.as_ref().is_some_and(|request| {
                            request.epoch() != epoch
                                || !request.keys().iter().any(|item| {
                                    item.id == key.id && item.max_pixels >= key.max_pixels
                                })
                        })
                    {
                        drop(inbox);
                        drop(ready);
                        continue;
                    }
                    inbox.results.push_back(ready);
                    drop(inbox);
                    (owner.wake)();
                }
            })?;
        Ok(Self {
            shared,
            worker: Some(worker),
            results: [const { None }; REQUEST_CAPACITY],
            sent: Request::default(),
        })
    }
    pub fn request(&mut self, request: Request) -> io::Result<()> {
        self.send(request)
    }
    pub fn cancel(&mut self) -> io::Result<()> {
        let mut empty = self.sent;
        empty.clear();
        self.send(empty)
    }
    fn send(&mut self, request: Request) -> io::Result<()> {
        if self.sent == request {
            return Ok(());
        }
        let mut inbox = self.shared.mailbox.lock().unwrap();
        if inbox.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        inbox.request = Some(request);
        self.sent = request;
        drop(inbox);
        self.shared.changed.notify_one();
        Ok(())
    }
    pub fn poll(&mut self) -> bool {
        let mut inbox = self.shared.mailbox.lock().unwrap();
        if inbox.retired.iter().any(Option::is_some) || inbox.results.is_empty() {
            return false;
        }
        inbox.retired = std::mem::replace(&mut self.results, [const { None }; REQUEST_CAPACITY]);
        for result in &mut self.results {
            *result = inbox.results.pop_front();
        }
        drop(inbox);
        self.shared.changed.notify_one();
        true
    }
    pub fn ready(&self) -> Option<&ArtworkReady> {
        self.ready_images().last()
    }
    pub fn ready_images(&self) -> impl Iterator<Item = &ArtworkReady> {
        self.results.iter().filter_map(Option::as_deref)
    }
}
impl Drop for ArtworkWorker {
    fn drop(&mut self) {
        {
            let mut inbox = self.shared.mailbox.lock().unwrap();
            inbox.closed = true;
            inbox.final_results =
                std::mem::replace(&mut self.results, [const { None }; REQUEST_CAPACITY]);
        }
        self.shared.changed.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::{AppPaths, Preferences};

    #[test]
    fn image_worker_coalesces_requests_retires_results_and_keeps_unchanged_polls_quiet() {
        // Real ImageIO and its background handoff require a worker; all waits have a short deadline.
        extern "C" fn wake() {}
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../Tests/LocalWaveDataTests/Fixtures/artwork-rgba.png");
        let mut worker =
            ArtworkWorker::open(Some(fixture.parent().unwrap().to_owned()), wake).unwrap();
        let mut request = Request::default();
        request.insert(1, fixture.to_str().unwrap(), 512);
        worker.request(request).unwrap();
        // Cached views keep their outstanding packet until the result is adopted.
        worker.request(request).unwrap();
        let until = |worker: &mut ArtworkWorker, path: &str| {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
            loop {
                worker.poll();
                if worker
                    .ready()
                    .is_some_and(|ready| ready.key.path.as_str() == path)
                {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "artwork handoff timed out"
                );
                std::thread::yield_now();
            }
        };
        until(&mut worker, fixture.to_str().unwrap());
        assert!(worker.ready().unwrap().thumbnail.is_some());
        let old = Arc::downgrade(worker.results.iter().flatten().last().unwrap());
        let mut missing = Request::default();
        missing.insert(2, "not-present.png", 512);
        worker.request(missing).unwrap();
        until(&mut worker, "not-present.png");
        assert!(worker.ready().unwrap().thumbnail.is_none());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while old.strong_count() != 0 {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        mem_sys::local_reset();
        for _ in 0..1000 {
            worker.request(missing).unwrap();
            assert!(!worker.poll());
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        let mut batch = Request::default();
        for id in 10..18 {
            assert!(batch.insert(id, fixture.to_str().unwrap(), 64));
        }
        worker.request(batch).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        let mut received = 0u8;
        while received != u8::MAX {
            if worker.poll() {
                for ready in worker.ready_images() {
                    assert!(ready.thumbnail.is_some());
                    received |= 1 << (ready.key.id - 10);
                }
            }
            assert!(std::time::Instant::now() < deadline, "batch stalled");
            std::thread::yield_now();
        }
        let previous_revision = worker.ready().unwrap().revision;
        worker.cancel().unwrap();
        worker.request(batch).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        loop {
            worker.poll();
            if worker.ready().unwrap().revision > previous_revision {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "cancelled request could not restart"
            );
            std::thread::yield_now();
        }
        let last = Arc::downgrade(worker.results.iter().flatten().last().unwrap());
        drop(worker);
        assert_eq!(last.strong_count(), 0);
    }

    #[test]
    fn full_thumbnail_packet_finishes_before_the_ui_polls_and_retires_off_thread() {
        extern "C" fn wake() {}
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../Tests/LocalWaveDataTests/Fixtures/artwork-rgba.png");
        let mut worker =
            ArtworkWorker::open(Some(fixture.parent().unwrap().to_owned()), wake).unwrap();
        let mut request = Request::default();
        for id in 1..=REQUEST_CAPACITY {
            assert!(request.insert(id, fixture.to_str().unwrap(), 160));
        }
        worker.request(request).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        loop {
            let inbox = worker.shared.mailbox.lock().unwrap();
            if inbox.results.len() == REQUEST_CAPACITY {
                assert!(inbox.results.iter().all(|ready| ready.thumbnail.is_some()));
                let bytes: usize = inbox
                    .results
                    .iter()
                    .map(|ready| ready.thumbnail.as_ref().unwrap().pixels.len())
                    .sum();
                assert!(bytes <= READY_BYTE_LIMIT);
                break;
            }
            drop(inbox);
            assert!(
                std::time::Instant::now() < deadline,
                "decoder still waits for a frame between covers"
            );
            std::thread::yield_now();
        }
        mem_sys::local_reset();
        assert!(worker.poll());
        assert_eq!(worker.ready_images().count(), REQUEST_CAPACITY);
        assert_eq!(
            mem_sys::local_alloc_count(),
            0,
            "batch delivery must not allocate on the UI thread"
        );
        let old: Vec<_> = worker
            .results
            .iter()
            .flatten()
            .map(Arc::downgrade)
            .collect();
        let mut missing = Request::default();
        assert!(missing.insert(100, "not-present.png", 160));
        worker.request(missing).unwrap();
        loop {
            worker.poll();
            if worker.ready().is_some_and(|ready| ready.key.id == 100)
                && old.iter().all(|ready| ready.strong_count() == 0)
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "batch retirement or replacement stalled"
            );
            std::thread::yield_now();
        }
        assert!(worker.ready().unwrap().thumbnail.is_none());
        worker.cancel().unwrap();
        for _ in 0..1000 {
            assert!(!worker.poll());
        }
    }

    #[test]
    fn catalog_cover_paths_resolve_managed_and_legacy_files_without_relative_escapes() {
        let name = format!(
            "lw-artwork-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let base = std::env::temp_dir().join(&name);
        let documents = base.join("Documents");
        std::fs::create_dir_all(documents.join("CoverArt")).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../Tests/LocalWaveDataTests/Fixtures/artwork-rgba.png");
        std::fs::copy(&fixture, documents.join("CoverArt/cover.png")).unwrap();
        std::fs::copy(&fixture, base.join("legacy.png")).unwrap();
        std::os::unix::fs::symlink(
            base.join("legacy.png"),
            documents.join("CoverArt/escape.png"),
        )
        .unwrap();
        let mut worker = LibraryWorker::open(
            &base.join("library.sqlite"),
            Preferences::open(Some(&format!("com.olegpustovit.localwave.{name}"))).unwrap(),
            AppPaths {
                documents: documents.clone(),
                caches: base.join("Caches"),
                temporary: base.join("tmp"),
                test_database: None,
            },
        )
        .unwrap();
        let song = worker
            .repository
            .upsert_song(
                &lw_core::Song {
                    song_key: 7,
                    title: "Covered song".into(),
                    album: "Album".into(),
                    cover_art_path: Some("CoverArt/cover.png".into()),
                    ..Default::default()
                },
                1.0,
            )
            .unwrap();
        let catalog = worker.catalog().unwrap();
        let row = catalog.row(catalog.row_index(song.id.unwrap()).unwrap());
        assert_eq!(row.cover_art_path, Some("CoverArt/cover.png"));
        let pixels = worker
            .artwork_thumbnail(row.cover_art_path.unwrap(), 32)
            .unwrap();
        assert_eq!((pixels.width, pixels.height), (8, 4));
        assert_eq!(&pixels.pixels[..4], [255, 0, 0, 255]);
        assert!(
            worker
                .artwork_thumbnail(base.join("legacy.png").to_str().unwrap(), 32)
                .is_ok()
        );
        for bad in [
            "",
            "../legacy.png",
            "CoverArt/escape.png",
            "CoverArt/missing.png",
        ] {
            assert!(worker.artwork_thumbnail(bad, 32).is_err(), "{bad}");
        }
    }
}
