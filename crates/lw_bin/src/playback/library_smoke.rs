//! Main-thread integration fixture for the library worker, session and native queue.
use lw_apple_sys::history::Record;
use lw_apple_sys::player::{Kind, Player};
use lw_apple_sys::preferences::{AppPaths, Preferences};
use lw_bin::library::{LibraryWorker, PreparedPlayback};
use lw_core::player::{Action, Session, Ticket};
use lw_core::{Song, Source};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::time::{Duration, Instant};

enum Command {
    Prepare(Ticket),
    Active(Option<u64>),
    Release(u64),
    Started(u64, f64),
    Evict,
    Stop,
}
// Eight inline slots avoid a heap allocation for each prepared-item envelope.
#[allow(clippy::large_enum_variant)]
enum Response {
    Prepared(Ticket, io::Result<PreparedPlayback>),
    Evicted(usize),
    Recorded(bool, Record),
    Fault(io::Error),
}
fn error(message: &str) -> io::Error {
    io::Error::other(message)
}

fn start_and_pause(native: &mut Player, ticket: Ticket) -> io::Result<()> {
    native.play();
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        native.run_loop(Duration::from_millis(5));
        while let Some(event) = native.event()? {
            if matches!(event.kind, Kind::Failed | Kind::EngineFailed) {
                return Err(error(event.message()));
            }
            if event.kind == Kind::Started
                && event.generation() == ticket.generation
                && event.token() == ticket.token
            {
                native.pause();
                return Ok(());
            }
        }
    }
    Err(error(
        "The native engine did not report the actual playback start",
    ))
}

fn resume_and_pause(native: &mut Player, token: u64) -> io::Result<()> {
    native.play();
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        native.run_loop(Duration::from_millis(5));
        while let Some(event) = native.event()? {
            if matches!(event.kind, Kind::Failed | Kind::EngineFailed) {
                return Err(error(event.message()));
            }
            if event.kind == Kind::Started {
                return Err(error(
                    "Resuming the same physical item reported another start",
                ));
            }
        }
        let snapshot = native.snapshot();
        if snapshot.token == token && snapshot.playing() {
            native.pause();
            return Ok(());
        }
    }
    Err(error("The native engine did not resume the paused item"))
}

pub fn run(fixture: &Path) -> io::Result<()> {
    let base = std::env::temp_dir().join(format!(
        "lw-native-library-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos()
    ));
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(base.clone());
    let (commands, input) = mpsc::sync_channel(8);
    let (output, responses) = mpsc::sync_channel(8);
    let (initialized, startup) = mpsc::sync_channel(1);
    std::thread::scope(|scope| -> io::Result<()> {
        scope.spawn(move || {
            let create = || -> io::Result<(LibraryWorker, i64, i64)> {
                let source_path = base.join("Originals");
                std::fs::create_dir_all(&source_path)?;
                std::fs::copy(fixture, source_path.join("track.m4a"))?;
                let suite = format!(
                    "com.olegpustovit.localwave.native-library-{}",
                    base.file_name()
                        .ok_or_else(|| error("Fixture has no name"))?
                        .to_string_lossy()
                );
                let mut worker = LibraryWorker::open(
                    &base.join("library.sqlite"),
                    Preferences::open(Some(&suite))?,
                    AppPaths {
                        documents: base.join("Documents"),
                        caches: base.join("Caches"),
                        temporary: base.join("tmp"),
                        test_database: None,
                    },
                )?;
                let user = worker
                    .repository
                    .get_or_create_user(1)
                    .map_err(io::Error::other)?;
                let source = worker
                    .repository
                    .create_source(&Source {
                        dir_path: source_path.to_string_lossy().into_owned(),
                        path_id: 42,
                        user_id: user.id,
                        bookmark: Some(lw_apple_sys::bookmarks::create(&source_path)?),
                        ..Source::default()
                    })
                    .map_err(io::Error::other)?
                    .id
                    .ok_or_else(|| error("Missing source ID"))?;
                let mut song = Song {
                    song_key: 1,
                    source_id: Some(source),
                    relative_path: Some("missing.m4a".into()),
                    ..Song::default()
                };
                let bad = worker
                    .repository
                    .upsert_song(&song, 1.0)
                    .map_err(io::Error::other)?
                    .id
                    .ok_or_else(|| error("Missing song ID"))?;
                song.song_key = 2;
                song.relative_path = Some("track.m4a".into());
                let good = worker
                    .repository
                    .upsert_song(&song, 1.0)
                    .map_err(io::Error::other)?
                    .id
                    .ok_or_else(|| error("Missing song ID"))?;
                worker
                    .repository
                    .set_favorite(good, true)
                    .map_err(io::Error::other)?;
                Ok((worker, bad, good))
            };
            let (mut worker, bad, good) = match create() {
                Ok(value) => value,
                Err(error) => {
                    let _ = initialized.send(Err(error));
                    return;
                }
            };
            if initialized.send(Ok((bad, good))).is_err() {
                return;
            }
            let cancel = AtomicBool::new(false);
            while let Ok(command) = input.recv() {
                let result = match command {
                    Command::Prepare(ticket) => {
                        let prepared = worker.prepare_playback(
                            ticket.entry.song_id,
                            ticket.token,
                            ticket.entry.id,
                            2.0,
                            &cancel,
                        );
                        output
                            .send(Response::Prepared(ticket, prepared))
                            .map_err(|_| error("Playback response receiver closed"))
                    }
                    Command::Active(token) => worker.set_active_playback(token),
                    Command::Release(token) => worker.release_playback(token).map(|_| ()),
                    Command::Started(token, now) => worker
                        .record_playback_start(token, now)
                        .and_then(|counted| {
                            output
                                .send(Response::Recorded(
                                    counted,
                                    worker.playback_history().get(good),
                                ))
                                .map_err(|_| error("Playback response receiver closed"))
                        }),
                    Command::Evict => worker.enforce_cache_bytes(0).and_then(|evicted| {
                        output
                            .send(Response::Evicted(evicted.files))
                            .map_err(|_| error("Playback response receiver closed"))
                    }),
                    Command::Stop => break,
                };
                if let Err(error) = result
                    && output.send(Response::Fault(error)).is_err()
                {
                    break;
                }
            }
            let _ = worker.release_all_playback();
        });
        let result = (|| -> io::Result<()> {
            let (bad, good) = startup.recv().map_err(io::Error::other)??;
            let mut session = Session::default();
            session.configure(&[bad, good, good, good], 0);
            let selected = session
                .queue()
                .current()
                .ok_or_else(|| error("Missing queue"))?
                .id;
            session.select(selected, true);
            let mut native = Player::open()?;
            native.set_volume(0.0)?;
            let mut installed = Vec::with_capacity(3);
            while installed.len() < 3 {
                let requests = session.requests();
                let pending: Vec<_> = requests.into_iter().flatten().collect();
                if pending.is_empty() {
                    return Err(error(
                        "Preparation stopped before the native prequeue was filled",
                    ));
                }
                for ticket in &pending {
                    commands
                        .send(Command::Prepare(*ticket))
                        .map_err(io::Error::other)?;
                }
                for _ in 0..pending.len() {
                    let response = responses.recv().map_err(io::Error::other)?;
                    let Response::Prepared(ticket, prepared) = response else {
                        return Err(match response {
                            Response::Fault(error) => error,
                            _ => error("Unexpected worker response"),
                        });
                    };
                    match prepared {
                        Err(_) => {
                            if !matches!(session.file_failed(ticket), Action::Load(_)) {
                                return Err(error(
                                    "A missing first file did not advance the queue",
                                ));
                            }
                        }
                        Ok(prepared) => {
                            if !session.installable(ticket) {
                                return Err(error("Playback ticket order changed"));
                            }
                            if installed.is_empty() {
                                native.replace(ticket.generation, prepared.item)?;
                            } else if !native.append(
                                ticket.generation,
                                native.snapshot().token,
                                &prepared.item,
                            )? {
                                return Err(error("Native queue changed while paused"));
                            }
                            if !session.installed(ticket) {
                                return Err(error("Installed playback ticket was rejected"));
                            }
                            if prepared.song.is_favorite != 1 {
                                return Err(error("Playback materialization lost Favorites"));
                            }
                            installed.push((ticket, prepared.path));
                        }
                    }
                }
            }
            let expected = [
                installed[0].0.token,
                installed[1].0.token,
                installed[2].0.token,
            ];
            if native.snapshot().queued != expected
                || session.queue().current_index() != Some(1)
                || session.notice().map(|entry| entry.song_id) != Some(bad)
            {
                return Err(error(
                    "Library/native queue identities or skip notice changed",
                ));
            }
            if installed[0].1 != installed[1].1 || installed[1].1 != installed[2].1 {
                return Err(error(
                    "Duplicate playback did not reuse the materialized file",
                ));
            }
            commands
                .send(Command::Active(Some(expected[0])))
                .map_err(io::Error::other)?;
            start_and_pause(&mut native, installed[0].0)?;
            resume_and_pause(&mut native, expected[0])?;
            for (now, expected_counted) in [(100.125, true), (101.0, false)] {
                commands
                    .send(Command::Started(expected[0], now))
                    .map_err(io::Error::other)?;
                match responses.recv().map_err(io::Error::other)? {
                    Response::Recorded(
                        counted,
                        Record {
                            plays: 1,
                            last_played: Some(100.125),
                        },
                    ) if counted == expected_counted => {}
                    Response::Fault(error) => return Err(error),
                    _ => {
                        return Err(error(
                            "A physical playback start was not recorded exactly once",
                        ));
                    }
                }
            }
            commands.send(Command::Evict).map_err(io::Error::other)?;
            match responses.recv().map_err(io::Error::other)? {
                Response::Evicted(0) => {}
                Response::Fault(error) => return Err(error),
                _ => return Err(error("A queued file was evicted")),
            }
            native.advance();
            if !session.adopt(installed[1].0.generation, expected[1])
                || native.snapshot().token != expected[1]
            {
                return Err(error("Duplicate occurrence did not advance"));
            }
            commands
                .send(Command::Active(Some(expected[1])))
                .map_err(io::Error::other)?;
            start_and_pause(&mut native, installed[1].0)?;
            commands
                .send(Command::Started(expected[1], 102.5))
                .map_err(io::Error::other)?;
            match responses.recv().map_err(io::Error::other)? {
                Response::Recorded(
                    true,
                    Record {
                        plays: 2,
                        last_played: Some(102.5),
                    },
                ) => {}
                Response::Fault(error) => return Err(error),
                _ => {
                    return Err(error(
                        "The second occurrence did not record its own playback start",
                    ));
                }
            }
            commands
                .send(Command::Release(expected[0]))
                .map_err(io::Error::other)?;
            commands.send(Command::Evict).map_err(io::Error::other)?;
            match responses.recv().map_err(io::Error::other)? {
                Response::Evicted(0) => {}
                Response::Fault(error) => return Err(error),
                _ => {
                    return Err(error(
                        "Retiring one occurrence released another occurrence's file",
                    ));
                }
            }
            native.stop();
            session.stop();
            commands
                .send(Command::Active(None))
                .map_err(io::Error::other)?;
            for token in &expected[1..] {
                commands
                    .send(Command::Release(*token))
                    .map_err(io::Error::other)?;
            }
            commands.send(Command::Evict).map_err(io::Error::other)?;
            match responses.recv().map_err(io::Error::other)? {
                Response::Evicted(1) => {}
                Response::Fault(error) => return Err(error),
                _ => return Err(error("Retired playback cache was not released")),
            }
            println!(
                "library_native_smoke=ok skipped_occurrence=1 queued_occurrences=2,3,4 protected_until_retired=true actual_starts=2 history_plays=2"
            );
            Ok(())
        })();
        let _ = commands.send(Command::Stop);
        drop(commands);
        drop(responses);
        result
    })
}
