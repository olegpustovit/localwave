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
    let (release, ready) = mpsc::sync_channel(1);
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
            ready
                .recv_timeout(Duration::from_secs(1))
                .map_err(io::Error::other)?;
            Ok(library)
        },
        wake,
    )?;
    let deadline = Instant::now() + Duration::from_secs(4);
    player.enable_media_controls(false)?;
    let (bad, good) = receive
        .recv_timeout(Duration::from_secs(1))
        .map_err(io::Error::other)?;
    // Fill ordinary delivery before releasing the worker, then exercise the reserved Stop and browse slots.
    player.command(Command::Volume(0.2))?;
    for _ in 0..31 {
        player.command(Command::DismissNotice)?;
    }
    player.command(Command::Stop)?;
    player.command(Command::Volume(0.25))?;
    let mut browse_revision = 0;
    let request = lw_ui::browse::Request {
        scope: lw_ui::browse::Scope::Find,
        query: lw_ui::browse::Query::new("Controller").unwrap(),
        sort: lw_ui::browse::Sort::Title,
        ascending: true,
    };
    for _ in 0..100 {
        browse_revision = player.browse(request)?;
    }
    release.send(()).map_err(io::Error::other)?;
    until(&mut player, deadline, &mut |player| {
        player.status().initialized
            && player.status().processed_command >= browse_revision
            && player
                .catalog()
                .view
                .as_ref()
                .is_some_and(|view| view.request == request)
    })?;
    if player
        .catalog()
        .view
        .as_ref()
        .unwrap()
        .collection
        .ids
        .as_ref()
        != [good]
    {
        return Err(error(
            "Coalesced browse request was lost behind the full playback queue",
        ));
    }

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
    until(&mut player, deadline, &mut |player| {
        player.status().waveform.is_some()
    })?;
    if !player
        .status()
        .waveform
        .unwrap()
        .iter()
        .all(|peak| peak.is_finite() && (0.0..=1.0).contains(peak))
        || !base.join("Caches/LocalWave/waveforms.sqlite").is_file()
        || player.native_snapshot().token != 0
    {
        return Err(error(
            "Paused restoration did not deliver and cache its waveform safely",
        ));
    }
    let cache = base.join("Caches/LocalWave/waveforms.sqlite");
    std::thread::spawn(move || -> io::Result<()> {
        let cache = lw_core::waveform::Repository::open(cache.to_str().unwrap())
            .map_err(io::Error::other)?;
        let cached = cache
            .get(2)
            .map_err(io::Error::other)?
            .ok_or_else(|| error("Waveform row was not persisted"))?;
        cache
            .put(2, &cached.signature, &[0.25; 512])
            .map_err(io::Error::other)
    })
    .join()
    .map_err(|_| error("Waveform cache fixture panicked"))??;
    player.command(Command::RefreshCatalog)?;
    until(&mut player, deadline, &mut |player| {
        player.status().waveform == Some([0.25; 512])
    })?;
    let queue = player
        .catalog()
        .queue
        .as_ref()
        .ok_or_else(|| error("Restored queue snapshot was not delivered"))?;
    if queue.current != Some(1)
        || queue.entries.len() != 3
        || queue.upcoming().len() != 1
        || queue.entries.iter().any(|entry| entry.song_id != good)
        || queue
            .entries
            .windows(2)
            .any(|pair| pair[0].id == pair[1].id)
    {
        return Err(error(
            "Restored duplicate occurrences were not projected correctly",
        ));
    }
    let database = base.join("library.sqlite");
    player.command(Command::Toggle)?;
    let toggled = player.command(Command::Toggle)?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= toggled
            && !player.status().wants_playback
            && player.native_snapshot().token == 0
    })?;
    // A real start must remove the song from an active Unplayed result without another browse request.
    let unplayed = lw_ui::browse::Request {
        query: lw_ui::browse::Query::new("Controller unplayed").unwrap(),
        ..request
    };
    player.browse(unplayed)?;
    until(&mut player, deadline, &mut |player| {
        player
            .catalog()
            .view
            .as_ref()
            .is_some_and(|view| view.request == unplayed && view.collection.ids.as_ref() == [good])
    })?;
    player.command(Command::Volume(0.0))?;
    player.command(Command::Play)?;
    until(&mut player, deadline, &mut |player| {
        player.status().phase == Phase::Playing
            && player
                .catalog()
                .view
                .as_ref()
                .is_some_and(|view| view.request == unplayed && view.collection.ids.is_empty())
    })?;
    player.command(Command::Stop)?;
    until(&mut player, deadline, &mut |player| {
        player.native_snapshot().token == 0
    })?;
    if Preferences::open(Some(&suite))?.history().get(good).plays != 1 {
        return Err(error(
            "Unplayed qualification must count exactly one actual start",
        ));
    }
    let overview = player.browse(lw_ui::browse::Request {
        scope: lw_ui::browse::Scope::Overview,
        query: lw_ui::browse::Query::default(),
        ..request
    })?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= overview && player.catalog().view.is_none()
    })?;
    std::thread::spawn(move || -> io::Result<()> {
        let mut repository = lw_core::LibraryRepository::open(database.to_str().unwrap())
            .map_err(io::Error::other)?;
        let mut song = repository
            .get_song(good)
            .map_err(io::Error::other)?
            .unwrap();
        song.title = "Refreshed controller fixture".into();
        repository
            .upsert_song(&song, 2.0)
            .map_err(io::Error::other)?;
        repository
            .set_favorite(good, true)
            .map_err(io::Error::other)?;
        Ok(())
    })
    .join()
    .unwrap()?;
    let generation = player.status().generation;
    let mut refreshed = 0;
    for _ in 0..100 {
        refreshed = player.command(Command::RefreshCatalog)?;
    }
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= refreshed
            && &*player.status().title == "Refreshed controller fixture"
            && player.catalog().collections[0].ids.as_ref() == [good]
    })?;
    if player.status().generation != generation
        || player.status().queue_len != 3
        || player.status().wants_playback
    {
        return Err(error("Catalog refresh changed the restored transport"));
    }
    let database = base.join("library.sqlite");
    std::thread::spawn(move || {
        lw_apple_sys::sqlite::Connection::open(database.to_str().unwrap())?
            .execute_batch("ALTER TABLE playlists RENAME TO unavailable_playlists")
    })
    .join()
    .unwrap()
    .map_err(io::Error::other)?;
    let failed_save = player.command(Command::SaveQueue(
        lw_ui::browse::Query::new("Failed save").unwrap(),
    ))?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= failed_save
            && player.status().message.as_ref() == "Could not save the playlist. Try again."
    })?;
    if player.status().fault.is_some()
        || player.status().queue_len != 3
        || player.native_snapshot().token != 0
    {
        return Err(error("Failed playlist save faulted or changed the player"));
    }
    let failed = player.command(Command::RefreshCatalog)?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= failed && !player.status().message.is_empty()
    })?;
    if player.catalog().len() != 2
        || player.status().fault.is_some()
        || player.status().queue_len != 3
    {
        return Err(error(
            "Failed catalog refresh discarded the last good catalog or faulted playback",
        ));
    }
    let database = base.join("library.sqlite");
    std::thread::spawn(move || {
        lw_apple_sys::sqlite::Connection::open(database.to_str().unwrap())?
            .execute_batch("ALTER TABLE unavailable_playlists RENAME TO playlists")
    })
    .join()
    .unwrap()
    .map_err(io::Error::other)?;
    let recovered = player.command(Command::RefreshCatalog)?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= recovered && player.status().message.is_empty()
    })?;
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
        player.command(Command::RefreshCatalog)?;
        player.browse(request)?;
        player.run_loop(Duration::from_millis(1));
        player.poll()?;
        if player.status().phase == Phase::Playing && player.native_snapshot().queued[2] != 0 {
            break;
        }
        if Instant::now() >= deadline {
            return Err(error("Continuous seek/volume/browse starved preparation"));
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
    until(&mut player, deadline, &mut |player| {
        player.catalog().row_index(bad).is_some_and(|index| {
            player.catalog().library.row(index).state == lw_core::FileState::Failed
        })
    })?;
    let original = player.catalog().queue.as_ref().unwrap().entries.clone();
    if original.len() != 4 {
        return Err(error("Configured queue snapshot is stale"));
    }
    let revision = player.command(Command::MoveUpcoming {
        occurrence: original[3].id,
        before: Some(original[2].id),
    })?;
    until(&mut player, deadline, &mut |player| {
        let native = player.native_snapshot();
        player.status().processed_command >= revision
            && native.token == initial.token
            && native.queued[2] != 0
            && native.queued[1] != initial.queued[1]
            && player.catalog().queue.as_ref().is_some_and(|queue| {
                queue.entries.as_ref() == [original[0], original[1], original[3], original[2]]
            })
    })?;
    let reordered = player.catalog().queue.as_ref().unwrap().entries.clone();
    let next = player.native_snapshot().queued[1];
    player.command(Command::Next)?;
    until(&mut player, deadline, &mut |player| {
        player.status().current_index == Some(2)
            && player.native_snapshot().token == next
            && player.status().phase == Phase::Playing
            && player.catalog().queue.as_ref().is_some_and(|queue| {
                queue.current == Some(2) && Arc::ptr_eq(&queue.entries, &reordered)
            })
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
            && player
                .catalog()
                .queue
                .as_ref()
                .is_some_and(|queue| queue.can_undo && queue.upcoming().is_empty())
    })?;
    let cleared_entries = player.catalog().queue.as_ref().unwrap().entries.clone();
    let revision = player.command(Command::SaveQueue(
        lw_ui::browse::Query::new("  Saved queue 🎵  ").unwrap(),
    ))?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= revision
            && player
                .catalog()
                .collections
                .iter()
                .any(|list| list.title == "Saved queue 🎵" && list.ids.as_ref() == [bad, good])
    })?;
    if player.status().phase != Phase::Paused
        || player.native_snapshot().token != next
        || player.catalog().queue.as_ref().is_none_or(|queue| {
            !queue.can_undo || queue.entries.as_ref() != cleared_entries.as_ref()
        })
    {
        return Err(error(
            "Saving the queue changed playback, occurrences or clear Undo",
        ));
    }
    let revision = player.command(Command::UndoClear)?;
    until(&mut player, deadline, &mut |player| {
        player.status().processed_command >= revision
            && player.status().queue_len == 4
            && player.native_snapshot().token == next
            && player.native_snapshot().queued[1] != 0
            && player.catalog().queue.as_ref().is_some_and(|queue| {
                !queue.can_undo && queue.entries.as_ref() == reordered.as_ref()
            })
    })?;
    for (before, expected_index) in [(reordered[1].id, 1), (reordered[3].id, 2)] {
        let revision = player.command(Command::MoveEntry {
            occurrence: reordered[2].id,
            before: Some(before),
        })?;
        until(&mut player, deadline, &mut |player| {
            player.status().processed_command >= revision
                && player.status().current_index == Some(expected_index)
                && player.native_snapshot().token == next
                && player.native_snapshot().queued[1] != 0
        })?;
        if player.status().phase != Phase::Paused {
            return Err(error(
                "Moving the current queue occurrence interrupted playback",
            ));
        }
    }
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
    if history.plays != 4 {
        return Err(error(&format!(
            "Controller counted {} starts instead of four",
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
    if prefs.history().get(good).plays != 5 {
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
    until(&mut ended, deadline, &mut |player| {
        player.status().waveform.is_some()
    })?;
    ended.command(Command::Configure {
        songs: Arc::from([]),
        start: 0,
        autoplay: false,
        context: None,
    })?;
    until(&mut ended, deadline, &mut |player| {
        player.status().current.is_none() && player.status().waveform.is_none()
    })?;
    drop(ended);
    if prefs.history().get(good).plays != 6 {
        return Err(error(
            "Natural completion did not preserve the final playback start",
        ));
    }
    let failed_base = base.clone();
    let failed_suite = suite.clone();
    let mut database_failed =
        Controller::open(move || open_library(&failed_base, &failed_suite), wake)?;
    until(&mut database_failed, deadline, &mut |player| {
        player.status().initialized
    })?;
    let database = base.join("library.sqlite");
    std::thread::spawn(move || {
        lw_apple_sys::sqlite::Connection::open(database.to_str().unwrap())?
            .execute_batch("ALTER TABLE songs RENAME TO unavailable_songs")
    })
    .join()
    .unwrap()
    .map_err(io::Error::other)?;
    database_failed.command(Command::Configure {
        songs: Arc::from([good, good]),
        start: 0,
        autoplay: true,
        context: None,
    })?;
    until(&mut database_failed, deadline, &mut |player| {
        player.status().phase == Phase::Failed
    })?;
    if database_failed.status().fault != Some(lw_core::playback::Fault::Session)
        || database_failed.status().current_index != Some(0)
        || database_failed.native_snapshot().token != 0
    {
        return Err(error(
            "A database read error was traversed as a broken audio file",
        ));
    }
    drop(database_failed);
    let database = base.join("library.sqlite");
    std::thread::spawn(move || {
        lw_apple_sys::sqlite::Connection::open(database.to_str().unwrap())?
            .execute_batch("ALTER TABLE unavailable_songs RENAME TO songs")
    })
    .join()
    .unwrap()
    .map_err(io::Error::other)?;
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
        "controller_smoke=ok immutable_queue=true occurrence_reorder=true queue_clear_undo=true restored_paused=true skipped_missing=true preserved_prequeue=true continuous_controls_prepare=true history_plays=6 live_unplayed=true natural_end_paused=true rapid_next_ordered=true stop_priority=true stale_stop_rejected=true bounded_commands=32 failure_stop_ack=true warm_poll_allocations=0"
    );
    Ok(())
}
