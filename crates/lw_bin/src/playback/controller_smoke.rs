//! Real main-thread qualification of the production controller, isolated from user files/defaults.
use lw_apple_sys::preferences::{AppPaths, Preferences};
use lw_bin::controller::{Command, Controller};
use lw_bin::library::LibraryWorker;
use lw_core::playback::Phase;
use lw_core::preferences::PlayerState;
use lw_core::queue::RepeatMode;
use lw_core::{Song, Source};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

extern "C" fn wake() {}
fn error(message: &str) -> io::Error {
    io::Error::other(message)
}
fn open_library(base: &Path, suite: &str) -> io::Result<LibraryWorker> {
    LibraryWorker::open(
        &base.join("library.sqlite"),
        Preferences::open(Some(suite))?,
        AppPaths {
            documents: base.join("Documents"),
            caches: base.join("Caches"),
            temporary: base.join("tmp"),
            test_database: None,
        },
    )
}
fn until(
    player: &mut Controller,
    deadline: Instant,
    predicate: &mut dyn FnMut(&Controller) -> bool,
) -> io::Result<()> {
    loop {
        player.run_loop(Duration::from_millis(1));
        player.poll()?;
        if predicate(player) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(error(&format!(
                "Controller timed out: {:?}; native={:?}",
                player.status(),
                player.native_snapshot()
            )));
        }
    }
}
pub fn run(fixture: &Path) -> io::Result<()> {
    let base = std::env::temp_dir().join(format!(
        "lw-controller-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(base.clone());
    let fixture = fixture.to_owned();
    let worker_base = base.clone();
    let suite = format!(
        "com.olegpustovit.localwave.{}",
        base.file_name().unwrap().to_string_lossy()
    );
    let worker_suite = suite.clone();
    let context = Arc::new(lw_core::context::PlaybackContext::Album {
        name: "Fixture".into(),
        artist: "Example".into(),
    });
    let worker_context = context.clone();
    let (ids, receive) = mpsc::sync_channel(1);
    let mut player = Controller::open(
        move || {
            let original = worker_base.join("Originals");
            std::fs::create_dir_all(&original)?;
            std::fs::copy(fixture, original.join("track.m4a"))?;
            let mut library = open_library(&worker_base, &worker_suite)?;
            let user = library
                .repository
                .get_or_create_user(1)
                .map_err(io::Error::other)?;
            let source = library
                .repository
                .create_source(&Source {
                    dir_path: original.to_string_lossy().into_owned(),
                    path_id: 42,
                    user_id: user.id,
                    bookmark: Some(lw_apple_sys::bookmarks::create(&original)?),
                    ..Source::default()
                })
                .map_err(io::Error::other)?
                .id
                .unwrap();
            let mut song = Song {
                song_key: 1,
                source_id: Some(source),
                relative_path: Some("missing.m4a".into()),
                ..Song::default()
            };
            let bad = library
                .repository
                .upsert_song(&song, 1.0)
                .map_err(io::Error::other)?
                .id
                .unwrap();
            song.song_key = 2;
            song.title = "Controller fixture".into();
            song.relative_path = Some("track.m4a".into());
            let good = library
                .repository
                .upsert_song(&song, 1.0)
                .map_err(io::Error::other)?
                .id
                .unwrap();
            library.save_player_state(&PlayerState {
                song_ids: vec![good, good, good],
                current_index: 1,
                volume: Some(0.25),
                context_data: Some(library.playback_context_data(&worker_context)?),
            })?;
            library.save_settings(&lw_core::preferences::Settings {
                appearance: lw_core::preferences::Appearance::Dark,
                reduce_motion: true,
                ..Default::default()
            })?;
            ids.send((bad, good)).map_err(io::Error::other)?;
            Ok(library)
        },
        wake,
    )?;
    let deadline = Instant::now() + Duration::from_secs(4);
    let (bad, good) = receive
        .recv_timeout(Duration::from_secs(1))
        .map_err(io::Error::other)?;
    until(&mut player, deadline, &mut |player| {
        player.status().initialized
    })?;
    if player.status().queue_len != 3
        || player.status().settings.appearance != lw_core::preferences::Appearance::Dark
        || !player.status().settings.reduce_motion
        || player.status().context.as_ref() != context.as_ref()
        || player.status().current_index != Some(1)
        || player.status().wants_playback
        || player.native_snapshot().token != 0
    {
        return Err(error(
            "Persisted queue restoration changed or started playback",
        ));
    }
    if player.catalog().len() != 2
        || player
            .catalog()
            .collection(lw_ui::catalog::Route::Songs)
            .is_none()
    {
        return Err(error("Worker catalog snapshot was not delivered"));
    }
    player.command(Command::Volume(0.0))?;
    until(&mut player, deadline, &mut |player| {
        player.native_snapshot().volume == 0.0
    })?;
    player.command(Command::Configure {
        songs: Arc::from([bad, good, good, good]),
        start: 0,
        autoplay: true,
        context: Some(context.clone()),
    })?;
    // Repeated control updates must let current and upcoming preparation finish.
    loop {
        player.command(Command::Volume(0.0))?;
        player.command(Command::Seek(0.0))?;
        player.run_loop(Duration::from_millis(1));
        player.poll()?;
        if player.status().phase == Phase::Playing && player.native_snapshot().queued[2] != 0 {
            break;
        }
        if Instant::now() >= deadline {
            return Err(error("Continuous seek/volume starved preparation"));
        }
    }
    if player.status().current_index != Some(1) || player.status().message.is_empty() {
        return Err(error("A missing song did not skip with a notice"));
    }
    let initial = player.native_snapshot();
    player.command(Command::Pause)?;
    until(&mut player, deadline, &mut |player| {
        player.status().phase == Phase::Paused && !player.native_snapshot().playing()
    })?;
    let revision = player.command(Command::Reorder {
        indices: Arc::from([3]),
        destination: 2,
    })?;
    until(&mut player, deadline, &mut |player| {
        let native = player.native_snapshot();
        player.status().processed_command >= revision
            && native.token == initial.token
            && native.queued[2] != 0
            && native.queued[1] != initial.queued[1]
    })?;
    let next = player.native_snapshot().queued[1];
    player.command(Command::Next)?;
    until(&mut player, deadline, &mut |player| {
        player.status().current_index == Some(2)
            && player.native_snapshot().token == next
            && player.status().phase == Phase::Playing
    })?;
    player.command(Command::Pause)?;
    until(&mut player, deadline, &mut |player| {
        player.status().phase == Phase::Paused
    })?;
    let revision = player.command(Command::ClearUpcoming)?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= revision
            && player.status().queue_len == 3
            && player.native_snapshot().queued == [next, 0, 0]
    })?;
    let revision = player.command(Command::UndoClear)?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= revision
            && player.status().queue_len == 4
            && player.native_snapshot().token == next
            && player.native_snapshot().queued[1] != 0
    })?;
    let revision = player.command(Command::Repeat(RepeatMode::One))?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= revision
            && player.native_snapshot().token == next
            && player.native_snapshot().queued[2] != 0
    })?;
    player.command(Command::Next)?;
    player.command(Command::Volume(0.0))?;
    until(&mut player, deadline, &mut |player| {
        player.native_snapshot().token != next
            && player.status().current_index == Some(2)
            && player.status().phase == Phase::Playing
    })?;
    player.command(Command::Pause)?;
    until(&mut player, deadline, &mut |player| {
        player.status().phase == Phase::Paused
    })?;
    player.command(Command::Repeat(RepeatMode::None))?;
    player.command(Command::Seek(0.125))?;
    until(&mut player, deadline, &mut |player| {
        (player.status().position - 0.125).abs() < 0.01
    })?;
    mem_sys::local_reset();
    for _ in 0..1000 {
        player.poll()?;
        std::hint::black_box(player.status());
    }
    if mem_sys::local_alloc_count() != 0 {
        return Err(error("Warm controller polling allocated Rust objects"));
    }
    player.command(Command::Next)?;
    let revision = player.command(Command::Next)?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= revision
            && player.status().current_index == Some(3)
            && player.status().phase == Phase::Stopped
            && player.native_snapshot().token == 0
    })?;
    player.command(Command::Stop)?;
    until(&mut player, deadline, &mut |player| {
        player.status().phase == Phase::Stopped && player.native_snapshot().token == 0
    })?;
    // A newer stop must reject prepared items and pending native play commands.
    player.command(Command::Play)?;
    let revision = player.command(Command::Stop)?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= revision
            && player.status().phase == Phase::Stopped
            && player.native_snapshot().token == 0
    })?;
    drop(player);
    let prefs = Preferences::open(Some(&suite))?;
    let state = prefs.load_player()?;
    if state.song_ids != [bad, good, good, good]
        || state.current_index != 3
        || state.volume != Some(0.0)
        || state
            .context_data
            .as_deref()
            .map(lw_apple_sys::context::decode)
            .transpose()?
            .is_none_or(|value| {
                value.kind != 1 || value.name != "Fixture" || value.artist != "Example"
            })
    {
        return Err(error(
            "Controller persistence lost queue occurrences, index, volume or context",
        ));
    }
    let history = prefs.history().get(good);
    if history.plays != 3 {
        return Err(error(&format!(
            "Controller counted {} starts instead of three",
            history.plays
        )));
    }
    if prefs.cache_protection().owner_protected(good) {
        return Err(error("Controller shutdown retained an active cache lease"));
    }
    let reopen_base = base.clone();
    let reopen_suite = suite.clone();
    let (release, gate) = mpsc::sync_channel(1);
    let mut blocked = Controller::open(
        move || {
            gate.recv().map_err(io::Error::other)?;
            open_library(&reopen_base, &reopen_suite)
        },
        wake,
    )?;
    blocked.command(Command::Configure {
        songs: Arc::from([good, good]),
        start: 0,
        autoplay: true,
        context: None,
    })?;
    for _ in 0..31 {
        blocked.command(Command::Stop)?;
    }
    if blocked.command(Command::Play).unwrap_err().kind() != io::ErrorKind::WouldBlock {
        return Err(error(
            "Controller did not apply bounded command backpressure",
        ));
    }
    blocked.command(Command::Stop)?;
    let revision = blocked.command(Command::Pause)?;
    if blocked
        .command(Command::Volume(f32::NAN))
        .unwrap_err()
        .kind()
        != io::ErrorKind::InvalidInput
    {
        return Err(error("Controller accepted an invalid volume"));
    }
    release.send(()).map_err(io::Error::other)?;
    until(&mut blocked, deadline, &mut |player| {
        player.status().processed_command == revision
            && player.status().phase == Phase::Stopped
            && player.native_snapshot().token == 0
    })?;
    blocked.command(Command::Play)?;
    until(&mut blocked, deadline, &mut |player| {
        player.status().phase == Phase::Playing && player.native_snapshot().queued[1] != 0
    })?;
    // Simulate external removal while native items still own the shared cache file.
    for cached in std::fs::read_dir(base.join("Documents/Music"))? {
        std::fs::remove_file(cached?.path())?;
    }
    blocked.command(Command::Next)?;
    until(&mut blocked, deadline, &mut |player| {
        player.status().phase == Phase::Failed && player.native_snapshot().token == 0
    })?;
    drop(blocked);
    if prefs.history().get(good).plays != 4 {
        return Err(error("Failure shutdown counted an extra playback start"));
    }
    if prefs.cache_protection().owner_protected(good) {
        return Err(error(
            "Failure shutdown released its cache protection incorrectly",
        ));
    }
    if !base.join("Originals/track.m4a").is_file() {
        return Err(error("Controller changed the original audio file"));
    }
    let end_base = base.clone();
    let end_suite = suite.clone();
    let mut ended = Controller::open(move || open_library(&end_base, &end_suite), wake)?;
    ended.command(Command::Configure {
        songs: Arc::from([good]),
        start: 0,
        autoplay: true,
        context: None,
    })?;
    until(&mut ended, deadline, &mut |player| {
        player.status().phase == Phase::Ended
            && player.status().duration > 0.9
            && player.status().position == player.status().duration
            && player.native_snapshot().token == 0
            && player.native_snapshot().rate == 0.0
    })?;
    drop(ended);
    if prefs.history().get(good).plays != 5 {
        return Err(error(
            "Natural completion did not preserve the final playback start",
        ));
    }
    let mut failed = Controller::open(
        || Err(error("Injected library initialization failure")),
        wake,
    )?;
    until(&mut failed, deadline, &mut |player| {
        player.status().initialized && player.status().phase == Phase::Failed
    })?;
    if failed.command(Command::Play).unwrap_err().kind() != io::ErrorKind::BrokenPipe {
        return Err(error("A failed library accepted a playback command"));
    }
    println!(
        "controller_smoke=ok restored_paused=true skipped_missing=true preserved_prequeue=true continuous_controls_prepare=true history_plays=5 natural_end_paused=true rapid_next_ordered=true stop_priority=true stale_stop_rejected=true bounded_commands=32 failure_stop_ack=true warm_poll_allocations=0"
    );
    Ok(())
}
