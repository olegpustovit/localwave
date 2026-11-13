//! Headless native queue host. The shipping application controller is integrated separately.
use lw_apple_sys::player::{Kind, Player, Prepared};
use lw_core::playback::{Reading, Transport};
use lw_core::queue::Queue;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::thread::JoinHandle;
use std::time::Duration;

#[path = "playback/controller_smoke.rs"]
mod controller_smoke;
#[path = "playback/library_smoke.rs"]
mod library_smoke;

struct Worker {
    requests: Option<SyncSender<usize>>,
    ready: Option<Receiver<(usize, io::Result<Prepared>)>>,
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    fn open(files: Arc<Vec<PathBuf>>) -> io::Result<Self> {
        let (send, receive) = mpsc::sync_channel::<usize>(3);
        let (ready, results) = mpsc::sync_channel(3);
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let thread = std::thread::Builder::new()
            .name("localwave-playback-files".into())
            .spawn(move || {
                while let Ok(index) = receive.recv() {
                    if signal.load(Ordering::Relaxed) {
                        break;
                    }
                    let token = index as u64 + 1;
                    let item = Prepared::open(&files[index], token, token, false, &signal);
                    if ready.send((index, item)).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            requests: Some(send),
            ready: Some(results),
            cancel,
            thread: Some(thread),
        })
    }
    fn request(&self, index: usize) -> io::Result<()> {
        self.requests
            .as_ref()
            .expect("Worker is live")
            .send(index)
            .map_err(io::Error::other)
    }
    fn receive(&self) -> io::Result<(usize, Prepared)> {
        let (index, item) = self
            .ready
            .as_ref()
            .expect("Worker is live")
            .recv()
            .map_err(io::Error::other)?;
        Ok((index, item?))
    }
    fn try_receive(&self) -> io::Result<Option<(usize, Prepared)>> {
        match self.ready.as_ref().expect("Worker is live").try_recv() {
            Ok((index, item)) => Ok(Some((index, item?))),
            Err(TryRecvError::Empty) => Ok(None),
            Err(error) => Err(io::Error::other(error)),
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.requests.take();
        self.ready.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn run() -> io::Result<()> {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--controller-smoke-test")
    {
        return controller_smoke::run(&PathBuf::from(
            std::env::args_os()
                .nth(2)
                .ok_or_else(|| io::Error::other("Supply an audio fixture"))?,
        ));
    }
    let mut arguments = std::env::args_os().skip(1);
    if arguments
        .next()
        .is_some_and(|argument| argument == "--library-smoke-test")
    {
        return library_smoke::run(&PathBuf::from(arguments.next().ok_or_else(|| {
            io::Error::other("Supply a local audio fixture for the library smoke check")
        })?));
    }
    let mut files = Vec::new();
    let (mut smoke, mut mute) = (false, false);
    for argument in std::env::args_os().skip(1) {
        if argument == "--smoke-test" {
            smoke = true;
        } else if argument == "--mute" {
            mute = true;
        } else if argument == "--help" {
            println!(
                "Usage: localwave-playback [--mute] [--smoke-test] AUDIO...\nPlays local files with two upcoming native items. A bad input stops with an error."
            );
            return Ok(());
        } else {
            let file = PathBuf::from(argument);
            files.push(if file.is_absolute() {
                file
            } else {
                std::env::current_dir()?.join(file)
            });
        }
    }
    if files.is_empty() || files.len() > 1_000_000 {
        return Err(io::Error::other(
            "Supply between one and one million audio files",
        ));
    }
    let files = Arc::new(files);
    let worker = Worker::open(files.clone())?;
    let mut player = Player::open()?;
    player.set_volume(if mute || smoke { 0.0 } else { 1.0 })?;
    let mut transport = Transport::default();
    let generation = transport.begin(!smoke);
    let mut queue = Queue::default();
    let ids: Vec<_> = files
        .iter()
        .map(|path| lw_core::hash_string(&path.to_string_lossy()))
        .collect();
    queue.configure(&ids, 0);
    let initial = files.len().min(3);
    for index in 0..initial {
        worker.request(index)?;
    }
    for expected in 0..initial {
        let (index, item) = worker.receive()?;
        if index != expected {
            return Err(io::Error::other("Audio preparation order changed"));
        }
        if index == 0 {
            let token = item.token();
            player.replace(generation, item)?;
            transport.adopt(generation, token);
        } else if !player.append(generation, 1, &item)? {
            return Err(io::Error::other(
                "Initial native prequeue changed unexpectedly",
            ));
        }
    }
    println!("initial_queue={:?}", player.snapshot().queued);
    if smoke {
        while player.event()?.is_some() {}
        mem_sys::local_reset();
        for _ in 0..10000 {
            std::hint::black_box(player.snapshot());
            std::hint::black_box(player.event()?);
        }
        let allocations = mem_sys::local_alloc_count();
        if allocations != 0 {
            return Err(io::Error::other(format!(
                "Native poll allocated {allocations} Rust objects"
            )));
        }
        if initial > 1 {
            player.advance();
            let current = player.snapshot();
            if current.token != 2 || current.queued[0] != 2 {
                return Err(io::Error::other(
                    "Native advancement lost the queue occurrence",
                ));
            }
            if files.len() > 3 {
                worker.request(3)?;
                let (_, item) = worker.receive()?;
                if !player.append(generation, 2, &item)? || player.snapshot().queued != [2, 3, 4] {
                    return Err(io::Error::other(
                        "Native refill replaced the buffered next item",
                    ));
                }
                if player.append(generation + 1, 2, &item)? {
                    return Err(io::Error::other(
                        "Native prequeue accepted a stale generation",
                    ));
                }
            }
            if files.len() > 3
                && (!player.trim(generation, 2, 1)?
                    || player.snapshot().queued != [2, 3, 0]
                    || player.trim(generation + 1, 2, 0)?
                    || player.trim(generation, 2, 3).is_ok())
            {
                return Err(io::Error::other(
                    "Native prefix trimming lost a buffered item or accepted an invalid/stale prefix",
                ));
            }
            if !player.upcoming(generation, 2, [None, None])?
                || player.snapshot().queued != [2, 0, 0]
            {
                return Err(io::Error::other(
                    "Clearing upcoming items replaced the current item",
                ));
            }
        }
        player.pause();
        player.set_volume(0.25)?;
        if (player.snapshot().volume - 0.25).abs() > f32::EPSILON {
            return Err(io::Error::other("Native volume was not retained"));
        }
        if player.seek(f64::NAN).is_ok() {
            return Err(io::Error::other("Native seek accepted a non-finite value"));
        }
        player.stop();
        if player.upcoming(generation, 0, [None, None])? {
            return Err(io::Error::other(
                "An empty engine accepted upcoming items without a current track",
            ));
        }
        if player.snapshot().token != 0 || player.snapshot().queued != [0; 3] {
            return Err(io::Error::other("Native stop retained playback items"));
        }
        println!("native_queue_smoke=ok rust_poll_allocations={allocations}");
        return Ok(());
    }
    player.play();
    let (mut requested, mut installed) = (initial, initial);
    let mut pending = None;
    let mut reported = 0;
    let mut ended = false;
    let mut finished_last = false;
    let mut final_duration = 0.0;
    let mut starts = 0;
    loop {
        player.run_loop(Duration::from_millis(20));
        while let Some(event) = player.event()? {
            if event.generation() != generation {
                continue;
            }
            if matches!(event.kind, Kind::Failed | Kind::EngineFailed) {
                return Err(io::Error::other(format!(
                    "Audio item {} failed: {}",
                    event.occurrence(),
                    event.message()
                )));
            }
            if event.kind == Kind::Started {
                starts += 1;
                println!(
                    "started token={} occurrence={}",
                    event.token(),
                    event.occurrence()
                );
            }
            if event.kind == Kind::Current {
                ended = event.token() == 0;
                if event.token() != 0 {
                    transport.adopt(generation, event.token());
                    queue.select(event.occurrence());
                }
            }
            if event.kind == Kind::Finished && event.token() == files.len() as u64 {
                finished_last = true;
                final_duration = event.duration();
            }
        }
        if pending.is_none() {
            pending = worker.try_receive()?;
        }
        if let Some((index, item)) = pending.take() {
            if index != installed {
                return Err(io::Error::other("Audio preparation order changed"));
            }
            let current = player.snapshot();
            let accepted = if current.token == 0 {
                let token = item.token();
                player.replace(generation, item)?;
                transport.adopt(generation, token);
                ended = false;
                player.play();
                true
            } else if player.append(generation, current.token, &item)? {
                true
            } else {
                pending = Some((index, item));
                false
            };
            if accepted {
                installed += 1;
            }
        }
        let current = player.snapshot();
        if current.token != 0 {
            transport.adopt(current.generation, current.token);
            queue.select(current.occurrence);
            transport.update(Reading {
                generation: current.generation,
                token: current.token,
                position: current.position,
                duration: current.duration,
                ready: current.status == 1,
                playing: current.playing(),
                waiting: current.time_control == 1,
            });
            if current.playing() && reported != current.token {
                println!(
                    "playing token={} occurrence={}",
                    current.token, current.occurrence
                );
                reported = current.token;
            }
        }
        let queued = current.queued.iter().filter(|token| **token != 0).count();
        while requested < files.len() && queued + requested - installed < 3 {
            worker.request(requested)?;
            requested += 1;
        }
        if current.token == 0 && installed == files.len() && ended && finished_last {
            if starts != files.len() {
                return Err(io::Error::other(
                    "Native start events did not match the completed queue",
                ));
            }
            transport.finish(
                generation,
                transport.token(),
                if final_duration > 0.0 {
                    final_duration
                } else {
                    transport.duration()
                },
            );
            println!(
                "completed={} final_track_elapsed={:.3} starts={starts}",
                files.len(),
                transport.position()
            );
            return Ok(());
        }
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("LocalWave playback: {error}");
        std::process::exit(1);
    }
}
