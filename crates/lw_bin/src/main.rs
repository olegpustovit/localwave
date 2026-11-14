mod events;
mod gpu;
mod smoke;

use input::Event as HostEvent;
use lw_bin::{
    controller::{Command, Controller},
    library::LibraryWorker,
};
use pxui::{FrameIn, Platform, Ui};
use pxui_sdl::Window;

extern "C" fn wake() {
    pxui_sdl::wake();
}

struct Fixture(Option<std::path::PathBuf>);
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

fn run() -> Result<(), String> {
    // Two layout frames, then two complete wheel cycles before the measured cycles.
    const WARMUP_FRAMES: usize = 82;
    const MEASURED_FRAMES: usize = 100;
    let smoke = std::env::args().any(|arg| arg == "--smoke-test");
    let performance = std::env::args().any(|arg| arg == "--performance-check");
    let trace_frames = std::env::args().any(|arg| arg == "--trace-frames");
    let performance_find = std::env::args().any(|arg| arg == "--performance-find");
    let performance_settings = std::env::args().any(|arg| arg == "--performance-settings");
    let performance_files = std::env::args().any(|arg| arg == "--performance-files");
    let performance_artists = std::env::args().any(|arg| arg == "--performance-artists");
    let performance_song_indicators =
        std::env::args().any(|arg| arg == "--performance-song-indicators");
    if performance_song_indicators
        && (!performance
            || std::env::args().any(|arg| {
                arg.starts_with("--performance-")
                    && !matches!(
                        arg.as_str(),
                        "--performance-check" | "--performance-song-indicators"
                    )
            }))
    {
        return Err(
            "--performance-song-indicators requires --performance-check and its own workload"
                .into(),
        );
    }

    if performance_artists
        && (!performance
            || std::env::args().any(|arg| {
                arg.starts_with("--performance-")
                    && !matches!(
                        arg.as_str(),
                        "--performance-check" | "--performance-artists"
                    )
            }))
    {
        return Err(
            "--performance-artists requires --performance-check and its own workload".into(),
        );
    }
    let performance_drop = std::env::args().any(|arg| arg == "--performance-drop");
    if performance_drop
        && (!performance
            || std::env::args().any(|arg| {
                arg.starts_with("--performance-")
                    && !matches!(arg.as_str(), "--performance-check" | "--performance-drop")
            }))
    {
        return Err("--performance-drop requires --performance-check and its own workload".into());
    }
    if performance_files
        && (!performance
            || std::env::args().any(|arg| {
                matches!(
                    arg.as_str(),
                    "--performance-settings"
                        | "--performance-find"
                        | "--performance-waveform"
                        | "--performance-artwork"
                        | "--performance-queue"
                        | "--performance-queue-drag"
                        | "--performance-queue-sheet"
                )
            }))
    {
        return Err("--performance-files requires --performance-check and its own workload".into());
    }
    if performance_settings
        && (!performance
            || std::env::args().any(|arg| {
                matches!(
                    arg.as_str(),
                    "--performance-find"
                        | "--performance-waveform"
                        | "--performance-artwork"
                        | "--performance-queue"
                        | "--performance-queue-drag"
                        | "--performance-queue-sheet"
                        | "--performance-files"
                )
            }))
    {
        return Err(
            "--performance-settings requires --performance-check and its own workload".into(),
        );
    }
    let performance_waveform = std::env::args().any(|arg| arg == "--performance-waveform");
    if performance_waveform && !performance {
        return Err("--performance-waveform requires --performance-check".into());
    }
    let performance_artwork = std::env::args().any(|arg| arg == "--performance-artwork");
    if performance_artwork && !performance {
        return Err("--performance-artwork requires --performance-check".into());
    }
    if performance_find && !performance {
        return Err("--performance-find requires --performance-check".into());
    }
    let performance_queue_drag = std::env::args().any(|arg| arg == "--performance-queue-drag");
    let performance_queue_sheet = std::env::args().any(|arg| arg == "--performance-queue-sheet");
    if performance_queue_sheet && !performance {
        return Err("--performance-queue-sheet requires --performance-check".into());
    }
    if performance_queue_drag && !performance {
        return Err("--performance-queue-drag requires --performance-check".into());
    }
    let performance_queue = performance_queue_drag
        || performance_queue_sheet
        || performance_artwork
        || std::env::args().any(|arg| arg == "--performance-queue");
    if performance_queue && !performance {
        return Err("--performance-queue requires --performance-check".into());
    }
    mem_sys::set_peer(pxui::library_counter);
    let mut window = Window::open_with("LocalWave", 1440, 872, true, false, true)?;
    window.set_resizable(true)?;
    let chrome =
        lw_apple_sys::window::Chrome::open(window.token()?).map_err(|error| error.to_string())?;
    window.resize(1440, 872)?;
    let mut window_chrome = chrome.snapshot().map_err(|error| error.to_string())?;
    let environment =
        lw_apple_sys::environment::Environment::open(wake).map_err(|error| error.to_string())?;
    let mut preferences = environment.snapshot();
    let mut ui = Ui::create(Platform::Apple, Some(15), &[lw_ui::FONT, lw_ui::MONO]);
    if performance_queue_drag || performance_artists || performance_song_indicators {
        ui.set_a11y(true);
    }
    ui.enable_system_fonts()
        .map_err(|code| format!("System fonts failed ({code}): {}", ui.last_error()))?;
    let mut themes = lw_bin::appearance::Themes::new(&ui)?;
    themes.apply(
        &mut ui,
        &lw_core::preferences::Settings::default(),
        preferences,
    );
    if trace_frames {
        ui.set_timing(true);
    }
    let mut graphics = gpu::Graphics::open(&mut window, &ui)?;
    let temporary = if smoke || performance {
        Some(
            lw_apple_sys::preferences::AppPaths::load()
                .map_err(|error| error.to_string())?
                .temporary,
        )
    } else {
        None
    };
    let fixture = Fixture(temporary.map(|temporary| {
        temporary.join(format!(
            "lw-gui-check-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }));
    let mut player = if let Some(base) = fixture.0.clone() {
        Controller::open(
            move || {
                let documents = base.join("Documents");
                std::fs::create_dir_all(&documents)?;
                let suite = format!(
                    "com.olegpustovit.localwave.{}",
                    base.file_name().unwrap().to_string_lossy()
                );
                let mut library = LibraryWorker::open(
                    &base.join("library.sqlite"),
                    lw_apple_sys::preferences::Preferences::open(Some(&suite))?,
                    lw_apple_sys::preferences::AppPaths {
                        documents,
                        caches: base.join("Caches"),
                        temporary: base.join("tmp"),
                        test_database: None,
                    },
                )?;
                if performance {
                    if performance_settings {
                        library.save_settings(&lw_core::preferences::Settings {
                            appearance: lw_core::preferences::Appearance::Light,
                            ..Default::default()
                        })?;
                    }
                    smoke::seed_catalog(&base.join("library.sqlite"))?;
                    if performance_artists {
                        smoke::seed_artists(&base.join("library.sqlite"))?;
                    }
                    if performance_artwork {
                        smoke::seed_artwork(&base.join("library.sqlite"), &base.join("Documents"))?;
                    }
                    smoke::seed_waveform(&base.join("Caches"))?;
                    if performance_files {
                        smoke::seed_files(&mut library)?;
                    }
                    library.save_player_state(&lw_core::preferences::PlayerState {
                        song_ids: if performance_queue {
                            (1..=10_000).collect()
                        } else {
                            vec![1]
                        },
                        current_index: 0,
                        volume: Some(0.0),
                        context_data: None,
                    })?;
                }
                Ok(library)
            },
            wake,
        )
    } else {
        Controller::open(LibraryWorker::current, wake)
    }
    .map_err(|error| error.to_string())?;
    let mut artwork = lw_bin::library::ArtworkWorker::open(
        fixture.0.as_ref().map(|base| base.join("Documents")),
        wake,
    )
    .map_err(|error| error.to_string())?;
    let files_base = fixture.0.clone();
    let mut files = lw_bin::files::FilesWorker::open(
        move || {
            if let Some(base) = files_base {
                LibraryWorker::open(
                    &base.join("library.sqlite"),
                    lw_apple_sys::preferences::Preferences::open(Some(&format!(
                        "com.olegpustovit.localwave.{}",
                        base.file_name().unwrap().to_string_lossy()
                    )))?,
                    lw_apple_sys::preferences::AppPaths {
                        documents: base.join("Documents"),
                        caches: base.join("Caches"),
                        temporary: base.join("tmp"),
                        test_database: None,
                    },
                )
            } else {
                LibraryWorker::current()
            }
        },
        wake,
    )
    .map_err(|error| error.to_string())?;
    // Workers do not inherit the UI's interactive QoS.
    lw_apple_sys::ui_thread_priority().map_err(|error| error.to_string())?;
    if smoke || performance {
        let deadline = window.time() + 5.0;
        loop {
            player.poll().map_err(|error| error.to_string())?;
            if player.status().initialized {
                if !player.status().message.is_empty() {
                    return Err(player.status().message.to_string());
                }
                // Queue and status travel through separate bounded handoffs.
                if (!performance_queue || player.catalog().queue.is_some())
                    && (!performance || player.status().waveform.is_some())
                {
                    break;
                }
            }
            if window.time() >= deadline {
                return Err("Library initialization timed out during qualification".into());
            }
            window.wait(Some(window.time() + 0.05));
        }
    }
    if performance && player.catalog().len() != 10_000 {
        return Err("Performance fixture must contain 10,000 songs".into());
    }
    if performance_queue
        && player
            .catalog()
            .queue
            .as_ref()
            .map(|queue| queue.upcoming().len())
            != Some(9_999)
    {
        return Err("Queue performance fixture must contain 9,999 upcoming occurrences".into());
    }
    let indicator_song = if performance_song_indicators {
        Some(
            player
                .catalog()
                .row(
                    *player
                        .catalog()
                        .all
                        .as_ref()
                        .unwrap()
                        .rows
                        .get(15)
                        .ok_or("Song indicator fixture needs sixteen rows")?,
                )
                .id,
        )
    } else {
        None
    };
    let mut extent = window.extent();
    let mut events: Vec<HostEvent> = Vec::with_capacity(256);
    let mut wire = Vec::with_capacity(256);
    let mut scratch = Vec::new();
    let mut first = true;
    let mut repaint = None;
    let mut revision = u64::MAX;
    let (mut frames, mut draws) = (0, 0);
    let mut waveform_seeks = 0;
    let (mut settings_changes, mut settings_themes) = (0, 0u8);
    let (mut min_artist_rows, mut max_artist_rows) = (usize::MAX, 0usize);
    let (mut first_artist_min, mut first_artist_max) = (usize::MAX, 0usize);
    let mut min_waveform_bars = usize::MAX;
    let (mut indicator_playing, mut indicator_paused, mut indicator_changes) =
        (0usize, 0usize, 0usize);
    let mut indicator_previous = None::<[u32; 3]>;

    let mut drop_overlay_frames = 0usize;
    let (mut warm_allocations, mut warm_build_us) = (0, 0);
    let mut timings = [smoke::FrameTiming::default(); MEASURED_FRAMES];
    let mut frame_trace =
        [(0usize, [0usize; 8], 0u64, 0u64, 0u64, [0u64; 4], 0u64); WARMUP_FRAMES + MEASURED_FRAMES];
    let (mut min_artwork_quads, mut warm_artwork_requests) = (usize::MAX, 0usize);
    let (mut min_files_rows, mut max_files_rows) = (usize::MAX, 0usize);
    let mut smoke_check = smoke::Smoke::new(window.time());
    let mut smoke_finished = false;
    let mut shell_state = lw_ui::ShellState::default();
    if performance_settings {
        shell_state.apply(lw_ui::Intent::ShowSettings);
        player
            .command(Command::ReadStorage)
            .map_err(|error| error.to_string())?;
    }
    if performance_files {
        use lw_ui::files::{Location, Request};
        shell_state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Files));
        files
            .request(Request::Roots)
            .map_err(|error| error.to_string())?;
        let deadline = window.time() + 5.0;
        let mut browsing = false;
        loop {
            files.poll();
            let snapshot = files.snapshot();
            if !browsing
                && let Some(user) = snapshot.account
                && let Some(source) = snapshot.sources.first()
            {
                files
                    .request(Request::Browse(Location {
                        user,
                        source: source.id,
                        parent: None,
                    }))
                    .map_err(|error| error.to_string())?;
                browsing = true;
            }
            if files
                .snapshot()
                .directory
                .as_ref()
                .is_some_and(|directory| directory.entries.len() == 10_000)
            {
                break;
            }
            if window.time() >= deadline {
                return Err(format!(
                    "Files fixture preparation failed: {}",
                    files.snapshot().message
                ));
            }
            window.wait(Some(window.time() + 0.05));
        }
    }
    if performance_artwork {
        shell_state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Albums));
    }
    if performance_artists {
        if player.catalog().artists.len() != 10_000
            || player
                .catalog()
                .artist_table
                .as_ref()
                .is_none_or(|table| table.rows.len() != 10_000)
        {
            return Err("Artist performance fixture must contain 10,000 prepared artists".into());
        }
        shell_state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Artists));
    }
    if performance_find {
        use lw_ui::{
            Intent,
            browse::{Query, Sort},
            catalog::Route,
        };
        shell_state.apply(Intent::Navigate(Route::Find));
        shell_state.apply(Intent::Search(
            Query::new("Track m4a year:2019 unplayed").unwrap(),
        ));
        shell_state.apply(Intent::Sort(Sort::Title));
        shell_state.apply(Intent::Sort(Sort::Title));
        let request = shell_state.request(player.catalog()).unwrap();
        player.browse(request).map_err(|error| error.to_string())?;
        let deadline = window.time() + 5.0;
        loop {
            player
                .poll_with_catalog(&mut |previous, next| shell_state.refresh(previous, next))
                .map_err(|error| error.to_string())?;
            if player
                .catalog()
                .view
                .as_ref()
                .is_some_and(|view| view.request == request)
            {
                if shell_state
                    .collection(player.catalog())
                    .map(|group| group.rows.len())
                    != Some(10_000)
                {
                    return Err("Find performance fixture must match 10,000 songs".into());
                }
                break;
            }
            if window.time() >= deadline {
                return Err("Find preparation timed out during qualification".into());
            }
            window.wait(Some(window.time() + 0.05));
        }
    }
    player
        .enable_media_controls(!smoke && !performance)
        .map_err(|error| error.to_string())?;
    let mut command_error: Option<&'static str> = None;
    if performance_queue_sheet {
        shell_state.apply(lw_ui::Intent::ShowQueue);
        shell_state.apply(lw_ui::Intent::NamePlaylist);
        shell_state.apply(lw_ui::Intent::EditPlaylistName(
            lw_ui::browse::Query::new("Qualification queue").unwrap(),
        ));
    }
    let mut error_revision = 0;
    let mut folder_picker = None;
    let mut audio_picker = None;
    let mut file_drop = events::FileDrop::default();
    let mut files_catalog_revision = 0;
    let mut files_selection_revision = 0;
    loop {
        if performance {
            mem_sys::local_reset();
        }
        events.clear();
        if !window.pump(&mut events) {
            break;
        }
        if let Some(error) = window.take_failure() {
            return Err(error);
        }
        if performance_drop && frames >= 2 {
            smoke::drop_events(frames - 2, &mut events);
        }
        for event in &mut events {
            if let Some(drop) = file_drop.event(event, files.snapshot().account) {
                command_error = match drop {
                    Err(message) => Some(message),
                    Ok(drop) => match files.drop_files(drop.user, drop.paths) {
                        Ok(()) => {
                            shell_state.apply(lw_ui::Intent::CloseQueue);
                            shell_state
                                .apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Files));
                            None
                        }
                        Err(_) => Some(
                            "Folder work is busy or unavailable. Wait or cancel it, then drop the files again.",
                        ),
                    },
                };
                error_revision += 1;
            }
        }
        if let Some(selection) = events::folder_selection(&mut folder_picker, &mut events) {
            command_error = match selection {
                Ok(None) => None,
                Ok(Some((purpose, path))) => match purpose {
                    events::FolderPurpose::Add(user) => files.add_folder(user, path),
                    events::FolderPurpose::Reconnect(target) => {
                        files.reconnect_folder(target, path)
                    }
                }
                .err()
                .map(|_| "Could not update this folder. Try again."),
                Err(message) => Some(message),
            };
            error_revision += 1;
        }
        if let Some(selection) = events::audio_selection(&mut audio_picker, &mut events) {
            command_error = match selection {
                Ok(None) => None,
                Ok(Some((user, paths))) => files.add_audio(user, paths).err().map(
                    |_| "Could not add these audio files. Folder work is busy or unavailable.",
                ),
                Err(message) => Some(message),
            };
            error_revision += 1;
        }
        let next_preferences = environment.snapshot();
        let preferences_changed = next_preferences != preferences;
        preferences = next_preferences;
        let player_changed = player
            .poll_with_catalog(&mut |previous, next| shell_state.refresh(previous, next))
            .map_err(|error| error.to_string())?;
        let files_changed = files.poll() && shell_state.files_visible();
        if files.snapshot().catalog_revision != files_catalog_revision
            && player.command(Command::RefreshCatalog).is_ok()
        {
            files_catalog_revision = files.snapshot().catalog_revision;
        }
        if let Some(selection) = &files.snapshot().selection
            && selection.revision != files_selection_revision
            && (selection.songs.is_empty()
                || player
                    .command(Command::Configure {
                        songs: selection.songs.clone(),
                        start: 0,
                        autoplay: true,
                        context: Some(selection.context.clone()),
                    })
                    .is_ok())
        {
            files_selection_revision = selection.revision;
            if selection.dropped {
                shell_state.apply(lw_ui::Intent::CloseQueue);
                shell_state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Files));
            }
        }
        let artwork_changed = artwork.poll();
        let artwork_updated = graphics.sync_artwork(
            std::sync::Arc::as_ptr(&player.catalog().library) as usize,
            artwork.ready_images(),
        )?;
        let mut expected_settings_intent = None;
        if performance && frames >= 2 && !performance_drop {
            if performance_settings {
                expected_settings_intent =
                    smoke::settings_events(frames - 2, window_chrome.top_inset, &mut events);
            } else if performance_waveform {
                smoke::waveform_events(frames - 2, &mut events);
            } else if performance_queue_drag {
                let grip = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| {
                        ui.a11y_name_of(node) == "Reorder queued track"
                            && node.states & pxui::a11y::state::VISIBLE != 0
                    })
                    .ok_or("Queue drag fixture has no visible reorder target")?
                    .rect;
                smoke::queue_drag_events(
                    frames - 2,
                    (f64::from(grip.x + 10.0), f64::from(grip.y + 10.0)),
                    &mut events,
                );
            } else {
                events.push(HostEvent::PointerMoved(
                    if performance_queue_sheet || performance_artwork {
                        (650.0, 400.0)
                    } else if performance_queue {
                        (1300.0, 690.0)
                    } else {
                        (650.0, 400.0)
                    },
                ));
                events.push(HostEvent::Scrolled((
                    0.0,
                    (if (frames - 2) % 40 < 20 { -1.0 } else { 1.0 })
                        * if performance_artwork { 0.2 } else { 1.0 },
                )));
            }
        }
        let now = window.time();
        let current = window.extent();
        if current.drawable.0 <= 0 || current.drawable.1 <= 0 {
            window.wait(
                player
                    .poll_interval()
                    .map(|wait| window.time() + wait.as_secs_f64()),
            );
            continue;
        }
        let resized = current != extent;
        if resized {
            extent = current;
            window_chrome = chrome.snapshot().map_err(|error| error.to_string())?;
        }
        shell_state.set_window_inset(window_chrome.top_inset as f32);
        if performance && extent.logical != (1440, 872) {
            return Err("Performance qualification requires a 1440x872 logical window".into());
        }
        if first
            || performance
            || resized
            || !events.is_empty()
            || player_changed
            || files_changed
            || artwork_changed
            || artwork_updated
            || preferences_changed
            || repaint.is_some_and(|time| now >= time)
        {
            first = false;
            events::encode(&events, &mut scratch, &mut wire);
            let interval =
                trace_frames.then(|| lw_apple_sys::timing::FrameInterval::begin(frames as u64 + 1));
            let cpu_started = if performance {
                lw_apple_sys::thread_cpu_time_ns().map_err(|error| error.to_string())?
            } else {
                0
            };
            let allocations_before = if trace_frames {
                mem_sys::local_alloc_count()
            } else {
                0
            };
            let build_started = std::time::Instant::now();
            graphics.configure_artwork(
                extent.logical.0 as f32,
                extent.logical.1 as f32,
                window.scale() as f32,
            )?;
            player
                .sync_media_artwork(graphics.artwork.images.epoch(), artwork.ready_images())
                .map_err(|error| error.to_string())?;
            let mut gpu_trace = [0u64; 4];
            let mut intent = None;
            let status = player.status();
            if performance && status.waveform.is_none() {
                return Err("Waveform disappeared during performance qualification".into());
            }
            themes.apply(&mut ui, &status.settings, preferences);
            let view = lw_ui::PlayerView {
                files: Some(files.snapshot()),
                file_dragging: file_drop.hovered(),
                settings: status.settings,
                cache_usage: &status.cache_usage,
                settings_message: &status.settings_message,
                storage_busy: status.storage_busy,
                artwork: Some(&graphics.artwork.images),
                waveform: status.waveform.as_ref(),
                revision: if performance {
                    frames as u64
                } else {
                    status.revision + error_revision
                },
                title: &status.title,
                context: status.context.title(),
                message: command_error.unwrap_or(&status.message),
                playing: if performance_song_indicators {
                    frames % 40 < 20
                } else {
                    status.wants_playback
                },
                initialized: status.initialized,
                current_id: if performance_song_indicators {
                    indicator_song
                } else {
                    status.current.map(|entry| entry.song_id)
                },
                artist: &status.artist,
                album: &status.album,
                position: if performance {
                    frames as f64
                } else {
                    status.position
                },
                volume: status.volume,
                shuffled: status.shuffled,
                repeat: status.repeat,
                duration: if performance { 240.0 } else { status.duration },
            };
            ui.frame(
                &FrameIn {
                    width: extent.logical.0 as f32,
                    height: extent.logical.1 as f32,
                    scale: window.scale() as f32,
                    time: now,
                    appearance: preferences.appearance,
                    reduced_motion: u32::from(
                        !performance_song_indicators
                            && (preferences.reduced_motion != 0 || status.settings.reduce_motion),
                    ),
                    events: wire.as_ptr(),
                    event_count: wire.len(),
                    ..FrameIn::default()
                },
                &mut |ui| lw_ui::shell(ui, &view, player.catalog(), &shell_state, &mut intent),
            )
            .map_err(|code| format!("Frame failed ({code})"))?;
            if performance_settings {
                if intent != expected_settings_intent {
                    return Err(format!(
                        "Settings input at frame {frames}: expected {expected_settings_intent:?}, got {intent:?}"
                    ));
                }
                settings_changes += usize::from(matches!(intent, Some(lw_ui::Intent::Setting(_))));
                settings_themes |= 1
                    << (usize::from(
                        status.settings.appearance == lw_core::preferences::Appearance::Dark,
                    ) * 4
                        + status.settings.signal as usize);
            }
            if performance_drop && intent.is_some() {
                return Err("External drag hover activated an application action".into());
            }
            let mut image_requests = graphics.artwork.images.visible_requests(&ui);
            if let Some(media) = player.media_controls() {
                media.request_artwork(&mut image_requests);
            }
            let build_us = build_started.elapsed().as_micros();
            let allocations_after = if trace_frames {
                mem_sys::local_alloc_count()
            } else {
                0
            };
            let cpu_us = if performance {
                (lw_apple_sys::thread_cpu_time_ns().map_err(|error| error.to_string())?
                    - cpu_started)
                    / 1000
            } else {
                0
            };
            if let Some(interval) = interval {
                interval.finish(build_us as u64, cpu_us);
            }
            artwork
                .request(image_requests)
                .map_err(|error| error.to_string())?;
            if performance_waveform && frames >= 2 {
                let phase = (frames - 2) % 20;
                if ui.captured() != (phase < 9) {
                    return Err("Waveform qualification lost held pointer capture".into());
                }
                match (phase, intent) {
                    (9, Some(lw_ui::Intent::SeekFraction(value)))
                        if value.is_finite() && (0.0..=1.0).contains(&value) =>
                    {
                        waveform_seeks += 1
                    }
                    (9, _) => return Err("Waveform release did not commit a seek".into()),
                    (_, None) => {}
                    _ => {
                        return Err(
                            "Waveform preview or scrolling produced an unexpected action".into(),
                        );
                    }
                }
            }
            if performance_queue_drag && frames >= 2 {
                let carrying = ui.carrying_ref();
                let expected = (1..20).contains(&((frames - 2) % 40));
                if expected != carrying.is_some()
                    || (expected
                        && (!ui.captured()
                            || carrying.is_none_or(|p| {
                                p.kind != "localwave.queue-occurrence:2" || p.value != "Track 00002"
                            })))
                {
                    return Err(format!(
                        "Queue carry did not survive the cross-page scroll or cancel correctly: frame={frames} phase={} expected={expected} captured={} kind={:?}",
                        (frames - 2) % 40,
                        ui.captured(),
                        carrying.map(|p| p.kind)
                    ));
                }
                if intent.is_some() {
                    return Err("Cancelled queue carry produced an application action".into());
                }
            }
            let before_output = if trace_frames {
                mem_sys::local_alloc_count()
            } else {
                0
            };
            let platform = &ui.output().platform;
            if platform.has_background != 0 {
                let color = platform.background;
                window.set_background((
                    f64::from(color.r) / 255.0,
                    f64::from(color.g) / 255.0,
                    f64::from(color.b) / 255.0,
                ))?;
            }
            let output_allocations = if trace_frames {
                mem_sys::local_alloc_count() - before_output
            } else {
                0
            };
            frames += 1;
            repaint = ui.repaint_at();
            if performance || ui.draw_revision() != revision {
                if graphics.draw(&mut window, &ui, trace_frames.then_some(&mut gpu_trace))? {
                    draws += 1;
                    revision = ui.draw_revision();
                } else {
                    repaint = Some(now + 1.0 / 60.0);
                }
            }
            if let Some(intent) = intent {
                if let lw_ui::Intent::Files(request) = intent {
                    let result = files.request(request);
                    if result.is_ok() {
                        shell_state.apply(lw_ui::Intent::CloseSourceMenu);
                        if matches!(request, lw_ui::files::Request::Remove(_)) {
                            shell_state.apply(lw_ui::Intent::CloseRemoveSource);
                        }
                    }
                    command_error = result
                        .err()
                        .map(|_| "Folder work is busy or unavailable. Try again.");
                    error_revision += 1;
                    repaint = Some(now);
                    continue;
                }
                if matches!(
                    intent,
                    lw_ui::Intent::ChooseFolder
                        | lw_ui::Intent::ChooseAudio
                        | lw_ui::Intent::ReconnectSource(_)
                ) {
                    let audio = intent == lw_ui::Intent::ChooseAudio;
                    let purpose = match intent {
                        lw_ui::Intent::ReconnectSource(target) => {
                            Some(events::FolderPurpose::Reconnect(target))
                        }
                        _ => files.snapshot().account.map(events::FolderPurpose::Add),
                    };
                    shell_state.apply(lw_ui::Intent::CloseSourceMenu);
                    if folder_picker.is_none()
                        && audio_picker.is_none()
                        && let Some(purpose) = purpose
                    {
                        let filters = if audio {
                            vec![pxui_sdl::DialogFilter {
                                name: "Audio files".into(),
                                pattern: "mp3;wav;m4a;flac;aac;aiff;aif".into(),
                            }]
                        } else {
                            Vec::new()
                        };
                        let kind = if audio {
                            pxui_sdl::DialogKind::OpenFiles
                        } else {
                            pxui_sdl::DialogKind::OpenFolder
                        };
                        match window.show_dialog(&filters, None, kind) {
                            Ok(id) => {
                                if audio {
                                    if let events::FolderPurpose::Add(user) = purpose {
                                        audio_picker = Some((id, user));
                                    }
                                } else {
                                    folder_picker = Some((id, purpose));
                                }
                                command_error = None;
                            }
                            Err(_) => {
                                command_error = Some("Could not open the music picker.");
                            }
                        }
                        error_revision += 1;
                        repaint = Some(now);
                    }
                    continue;
                }
                if intent == lw_ui::Intent::Navigate(lw_ui::catalog::Route::Files) {
                    command_error = files
                        .request(lw_ui::files::Request::Roots)
                        .err()
                        .map(|_| "Folders are unavailable. Try again.");
                    error_revision += 1;
                }
                if matches!(
                    intent,
                    lw_ui::Intent::Navigate(_)
                        | lw_ui::Intent::TogglePanel
                        | lw_ui::Intent::CloseQueue
                ) {
                    graphics.artwork.images.cancel_requests();
                    artwork.cancel().map_err(|error| error.to_string())?;
                }
                let mut browse = None;
                if matches!(
                    intent,
                    lw_ui::Intent::PrivacyPolicy | lw_ui::Intent::Support
                ) {
                    let link = if matches!(intent, lw_ui::Intent::PrivacyPolicy) {
                        lw_apple_sys::window::Link::PrivacyPolicy
                    } else {
                        lw_apple_sys::window::Link::Support
                    };
                    command_error = lw_apple_sys::window::open_link(link)
                        .err()
                        .map(|_| "Could not open this link.");
                    error_revision += 1;
                    repaint = Some(now);
                    continue;
                }
                let command = if shell_state.apply(intent) {
                    if matches!(
                        intent,
                        lw_ui::Intent::Navigate(_)
                            | lw_ui::Intent::Search(_)
                            | lw_ui::Intent::Sort(_)
                    ) {
                        browse = shell_state.request(player.catalog());
                        None
                    } else if matches!(intent, lw_ui::Intent::ShowSettings) {
                        Some(Command::ReadStorage)
                    } else {
                        None
                    }
                } else {
                    Some(match intent {
                        lw_ui::Intent::Play => Command::Play,
                        lw_ui::Intent::Pause => Command::Pause,
                        lw_ui::Intent::Stop => Command::Stop,
                        lw_ui::Intent::Next => Command::Next,
                        lw_ui::Intent::Previous => Command::Previous,
                        lw_ui::Intent::PlayRow(index) => {
                            shell_state.apply(lw_ui::Intent::CloseSongMenu);
                            let Some(collection) = shell_state
                                .collection(player.catalog())
                                .filter(|collection| index < collection.ids.len())
                            else {
                                continue;
                            };
                            Command::Configure {
                                songs: collection.ids.clone(),
                                start: index,
                                autoplay: true,
                                context: Some(collection.context.clone()),
                            }
                        }
                        lw_ui::Intent::AppendSong(id) => {
                            shell_state.apply(lw_ui::Intent::CloseSongMenu);
                            Command::Append(id)
                        }
                        lw_ui::Intent::ShuffleAll => {
                            let Some(collection) = player
                                .catalog()
                                .all
                                .as_ref()
                                .filter(|collection| !collection.ids.is_empty())
                            else {
                                continue;
                            };
                            Command::ShuffleAll {
                                songs: collection.ids.clone(),
                                seed: now.to_bits(),
                            }
                        }
                        lw_ui::Intent::Shuffle => Command::Shuffle {
                            enabled: !player.status().shuffled,
                            seed: now.to_bits(),
                        },
                        lw_ui::Intent::Repeat => Command::Repeat(match player.status().repeat {
                            lw_core::queue::RepeatMode::None => lw_core::queue::RepeatMode::All,
                            lw_core::queue::RepeatMode::All => lw_core::queue::RepeatMode::One,
                            lw_core::queue::RepeatMode::One => lw_core::queue::RepeatMode::None,
                        }),
                        lw_ui::Intent::Volume(value) => Command::Volume(value),
                        lw_ui::Intent::RefreshLibrary => Command::RefreshCatalog,
                        lw_ui::Intent::Setting(change) => Command::Setting(change),
                        lw_ui::Intent::PlayQueue(occurrence) => Command::Select {
                            occurrence,
                            autoplay: true,
                        },
                        lw_ui::Intent::RemoveQueue(occurrence) => Command::Remove(occurrence),
                        lw_ui::Intent::MoveQueue { occurrence, before } => {
                            Command::MoveUpcoming { occurrence, before }
                        }
                        lw_ui::Intent::MoveFullQueue { occurrence, before } => {
                            Command::MoveEntry { occurrence, before }
                        }
                        lw_ui::Intent::ClearQueue => Command::ClearUpcoming,
                        lw_ui::Intent::UndoQueue => Command::UndoClear,
                        lw_ui::Intent::SaveQueue(name) => Command::SaveQueue(name),
                        lw_ui::Intent::SeekFraction(fraction) => Command::Seek(
                            fraction
                                * if performance {
                                    240.0
                                } else {
                                    player.status().duration
                                },
                        ),
                        lw_ui::Intent::Navigate(_)
                        | lw_ui::Intent::SongMenu(_, _, _, _)
                        | lw_ui::Intent::CloseSongMenu
                        | lw_ui::Intent::SelectRow(_)
                        | lw_ui::Intent::Search(_)
                        | lw_ui::Intent::SearchTooLong
                        | lw_ui::Intent::Sort(_)
                        | lw_ui::Intent::SortAlbums(_)
                        | lw_ui::Intent::TogglePanel
                        | lw_ui::Intent::ShowQueue
                        | lw_ui::Intent::CloseQueue
                        | lw_ui::Intent::NamePlaylist
                        | lw_ui::Intent::EditPlaylistName(_)
                        | lw_ui::Intent::PlaylistNameTooLong
                        | lw_ui::Intent::CancelPlaylistName => {
                            unreachable!("handled by ShellState")
                        }
                        lw_ui::Intent::ShowSettings
                        | lw_ui::Intent::CloseSettings
                        | lw_ui::Intent::ShowPrivacy
                        | lw_ui::Intent::ClosePrivacy
                        | lw_ui::Intent::SettingsMenu(_, _)
                        | lw_ui::Intent::CloseSettingsMenu => unreachable!("handled by ShellState"),
                        lw_ui::Intent::PrivacyPolicy | lw_ui::Intent::Support => {
                            unreachable!("handled above")
                        }
                        lw_ui::Intent::Files(_)
                        | lw_ui::Intent::ChooseFolder
                        | lw_ui::Intent::ChooseAudio
                        | lw_ui::Intent::ReconnectSource(_) => {
                            unreachable!("handled above")
                        }
                        lw_ui::Intent::SourceMenu(_, _)
                        | lw_ui::Intent::CloseSourceMenu
                        | lw_ui::Intent::ConfirmRemoveSource(_)
                        | lw_ui::Intent::CloseRemoveSource => unreachable!("handled by ShellState"),
                    })
                };
                let result = if let Some(request) = browse {
                    player.browse(request)
                } else {
                    command.map_or(Ok(0), |command| player.command(command))
                };
                command_error = match result {
                    Ok(_) => {
                        if matches!(intent, lw_ui::Intent::SaveQueue(_)) {
                            shell_state.apply(lw_ui::Intent::CancelPlaylistName);
                        }
                        None
                    }
                    Err(error) => Some(match error.kind() {
                        std::io::ErrorKind::WouldBlock => "Playback is busy. Try again.",
                        std::io::ErrorKind::InvalidInput => "This action is unavailable.",
                        std::io::ErrorKind::BrokenPipe => "Playback is unavailable.",
                        _ => return Err(error.to_string()),
                    }),
                };
                error_revision += 1;
                repaint = Some(now);
            }
            if trace_frames && performance {
                let keys = image_requests;
                let mut ids = [0usize; 8];
                for (out, key) in ids.iter_mut().zip(keys.keys()) {
                    *out = key.id;
                }
                frame_trace[frames - 1] = (
                    if artwork_updated {
                        artwork.ready().map_or(0, |ready| ready.key.id)
                    } else {
                        0
                    },
                    ids,
                    allocations_before,
                    allocations_after - allocations_before,
                    mem_sys::local_alloc_count() - allocations_after,
                    gpu_trace,
                    output_allocations,
                );
            }
            if performance_artwork && frames > WARMUP_FRAMES {
                let image_quads = ui
                    .layers()
                    .map(|layer| {
                        layer
                            .instances
                            .iter()
                            .filter(|instance| instance.kind == pxui::draw::kind::IMAGE)
                            .count()
                    })
                    .sum();
                min_artwork_quads = min_artwork_quads.min(image_quads);
                warm_artwork_requests = warm_artwork_requests.max(image_requests.keys().len());
            }
            if performance_song_indicators && frames > WARMUP_FRAMES {
                let visible = ui.a11y_nodes().iter().any(|node| {
                    node.is_role(pxui::ui::Role::Image)
                        && matches!(ui.a11y_name_of(node), "Playing" | "Paused")
                        && node.states & pxui::a11y::state::VISIBLE != 0
                        && node.rect.y >= 37.3
                        && node.rect.y + node.rect.h <= 800.0
                });
                if visible {
                    let accent = ui.theme().tokens.accent;
                    let mut heights = [0u32; 3];
                    let mut found = 0;
                    for quad in
                        ui.layers()
                            .flat_map(|layer| layer.instances.iter())
                            .filter(|quad| {
                                quad.kind == pxui::draw::kind::RECT
                                    && quad.color == [accent.r, accent.g, accent.b, accent.a]
                                    && (quad.rect[0] - 264.0).abs() < 9.0
                                    && (quad.rect[2] - quad.rect[0] - 2.0).abs() < 0.001
                                    && (4.39..11.01).contains(&(quad.rect[3] - quad.rect[1]))
                            })
                    {
                        if found >= 3 {
                            return Err("Song indicator workload drew extra bars".into());
                        }
                        heights[found] = (quad.rect[3] - quad.rect[1]).to_bits();
                        found += 1;
                    }
                    if found != 3 {
                        return Err("Song indicator workload lost its three drawn bars".into());
                    }
                    if (frames - 1) % 40 < 20 {
                        indicator_playing += 1;
                        indicator_changes += usize::from(
                            indicator_previous.is_some_and(|previous| previous != heights),
                        );
                        indicator_previous = Some(heights);
                    } else {
                        indicator_paused += 1;
                        if heights
                            .iter()
                            .any(|height| (f32::from_bits(*height) - 4.4).abs() > 0.001)
                        {
                            return Err("Paused song indicator still animates".into());
                        }
                    }
                }
            }
            if performance_artists && frames > WARMUP_FRAMES {
                let mut rows = 0;
                for node in ui
                    .a11y_nodes()
                    .iter()
                    .filter(|node| node.is_role(pxui::ui::Role::Button) && node.rect.x >= 232.0)
                {
                    if let Some(index) = ui
                        .a11y_name_of(node)
                        .strip_prefix("Artist ")
                        .and_then(|label| label.parse::<usize>().ok())
                    {
                        if rows == 0 {
                            first_artist_min = first_artist_min.min(index);
                            first_artist_max = first_artist_max.max(index);
                        }
                        rows += 1;
                    }
                }
                min_artist_rows = min_artist_rows.min(rows);
                max_artist_rows = max_artist_rows.max(rows);
            }
            if performance_files && frames > WARMUP_FRAMES {
                let rows = ui
                    .layers()
                    .flat_map(|layer| layer.instances.iter())
                    .filter(|instance| {
                        instance.kind == pxui::draw::kind::RECT
                            && (instance.rect[2] - instance.rect[0] - 72.0).abs() < 0.01
                            && (instance.rect[3] - instance.rect[1] - 36.0).abs() < 0.01
                    })
                    .count();
                min_files_rows = min_files_rows.min(rows);
                max_files_rows = max_files_rows.max(rows);
            }
            if performance_drop {
                let overlays = ui
                    .layers()
                    .flat_map(|layer| layer.instances.iter())
                    .filter(|instance| {
                        instance.kind == pxui::draw::kind::RECT
                            && instance.color[3] == 240
                            && (instance.rect[0] - 12.0).abs() < 0.01
                            && (instance.rect[1] - 12.0).abs() < 0.01
                            && (instance.rect[2] - 1428.0).abs() < 0.01
                            && (instance.rect[3] - 860.0).abs() < 0.01
                    })
                    .count();
                if overlays != usize::from(file_drop.hovered()) {
                    return Err("External drag workload lost its actual overlay quad".into());
                }
                if frames > WARMUP_FRAMES {
                    drop_overlay_frames += usize::from(file_drop.hovered());
                }
            }
            if performance && frames > WARMUP_FRAMES {
                let bars = ui
                    .layers()
                    .flat_map(|layer| layer.instances.iter())
                    .filter(|instance| {
                        instance.kind == pxui::draw::kind::RECT && instance.color[3] == 107
                    })
                    .count();
                min_waveform_bars = min_waveform_bars.min(bars);
                warm_allocations = warm_allocations.max(mem_sys::local_alloc_count());
                warm_build_us = warm_build_us.max(build_us);
                timings[frames - WARMUP_FRAMES - 1] = smoke::FrameTiming {
                    wall: build_us,
                    cpu: cpu_us,
                    phases: if trace_frames {
                        ui.phase_times().map(|time| time.as_micros() as u64)
                    } else {
                        [0; 7]
                    },
                };
            }
        }
        if performance && frames >= WARMUP_FRAMES + MEASURED_FRAMES {
            let artwork_bytes = graphics.artwork.resident_bytes();
            graphics.close()?;
            if trace_frames {
                for (frame, (ready, keys, host, build, draw, gpu, output)) in
                    frame_trace.iter().enumerate()
                {
                    if performance_artwork
                        && (*ready != 0 || *host + *build + *draw != 0 || keys[0] != 0)
                    {
                        println!(
                            "LocalWave image diagnostic: frame={} ready={ready} keys={keys:?} allocs_host_build_draw={host}/{build}/{draw}",
                            frame + 1
                        );
                    } else if *host + *build + *draw != 0 {
                        println!(
                            "LocalWave allocation diagnostic: frame={} allocs_host_build_after={host}/{build}/{draw} gpu_begin_prepare_draw_submit={gpu:?} output_getter={output}",
                            frame + 1
                        );
                    }
                }
            }
            if trace_frames {
                let mut out = std::io::stdout().lock();
                for (index, sample) in timings.iter().enumerate() {
                    sample
                        .write_outlier(WARMUP_FRAMES + index + 1, &mut out)
                        .map_err(|error| error.to_string())?;
                }
            }
            let mut build_samples = timings.map(|sample| sample.wall);
            let mut cpu_samples = timings.map(|sample| sample.cpu);
            build_samples.sort_unstable();
            cpu_samples.sort_unstable();
            println!(
                "LocalWave performance: workload={} songs=10000 frames={frames} draws={draws} max_warm_rust_allocations={warm_allocations} max_warm_build_us={warm_build_us}",
                if performance_song_indicators {
                    "songs-playing-indicator"
                } else if performance_artists {
                    "artists-measured-columns"
                } else if performance_drop {
                    "external-drop-hover-and-cancel"
                } else if performance_settings {
                    "settings-menus-appearance"
                } else if performance_files {
                    "files-indexed-directory"
                } else if performance_waveform {
                    "waveform-scrub-and-scroll"
                } else if performance_artwork {
                    "albums-artwork"
                } else if performance_queue_sheet {
                    "full-queue-playlist-editor"
                } else if performance_queue_drag {
                    "queue-carry-scroll-cancel"
                } else if performance_queue {
                    "queue-occurrences"
                } else if performance_find {
                    "find-filters-title-descending"
                } else {
                    "songs-original"
                }
            );
            println!(
                "LocalWave waveform: peaks=512 min_bars={min_waveform_bars} scrub_commits={waveform_seeks}"
            );
            if performance_song_indicators {
                println!(
                    "LocalWave song indicator: visible_playing_frames={indicator_playing} visible_paused_frames={indicator_paused} changed_frames={indicator_changes}"
                );
                if indicator_playing < 20 || indicator_paused < 20 || indicator_changes < 10 {
                    return Err(
                        "Song indicator workload missed playback, pause or changing native draws"
                            .into(),
                    );
                }
            }
            if performance_settings {
                println!(
                    "LocalWave settings: changes={settings_changes} themes={settings_themes:#04x}"
                );
                if settings_changes != 45 || settings_themes != 255 {
                    return Err("Settings workload missed an action or theme".into());
                }
            }
            if performance_artists {
                println!(
                    "LocalWave artists: prepared_rows=10000 min_rows={min_artist_rows} max_rows={max_artist_rows} first_min={first_artist_min} first_max={first_artist_max}"
                );
                if min_artist_rows < 12
                    || max_artist_rows > 40
                    || first_artist_min >= first_artist_max
                {
                    return Err("Artist workload did not scroll its measured virtual table".into());
                }
            }
            if performance_files {
                println!(
                    "LocalWave files: indexed_rows=10000 min_drawn_rows={min_files_rows} max_drawn_rows={max_files_rows}"
                );
                if min_files_rows < 8 || max_files_rows > 40 {
                    return Err("Files workload did not draw its virtualized rows".into());
                }
            }
            if performance_drop {
                println!(
                    "LocalWave external drop: measured_overlay_frames={drop_overlay_frames} mutation_jobs=0"
                );
                if drop_overlay_frames != 50 {
                    return Err(
                        "External drag workload missed an overlay/cancellation cycle".into(),
                    );
                }
            }
            if performance_waveform && (waveform_seeks != 9 || min_waveform_bars < 200) {
                return Err("Waveform qualification did not exercise all nine scrub cycles".into());
            }
            println!(
                "LocalWave build timing: wall_us_p50={} wall_us_p95={} wall_us_p99={} cpu_us_p50={} cpu_us_p95={} cpu_us_p99={} cpu_us_max={}",
                build_samples[49],
                build_samples[94],
                build_samples[98],
                cpu_samples[49],
                cpu_samples[94],
                cpu_samples[98],
                cpu_samples[99]
            );
            if performance_artwork {
                println!(
                    "LocalWave artwork: texture_bytes={artwork_bytes} min_image_quads={min_artwork_quads} max_warm_requests={warm_artwork_requests}"
                );
                if artwork_bytes == 0 || min_artwork_quads < 10 || warm_artwork_requests != 0 {
                    return Err(
                        "Artwork workload did not retain its warmed image working set".into(),
                    );
                }
            }
            if trace_frames {
                let mut phase_samples: [[u64; MEASURED_FRAMES]; 7] =
                    std::array::from_fn(|phase| timings.map(|sample| sample.phases[phase]));
                for samples in &mut phase_samples {
                    samples.sort_unstable();
                }
                println!(
                    "LocalWave phase medians us: begin={} build={} intrinsic={} arrange={} paint={} a11y={} finish={}",
                    phase_samples[0][49],
                    phase_samples[1][49],
                    phase_samples[2][49],
                    phase_samples[3][49],
                    phase_samples[4][49],
                    phase_samples[5][49],
                    phase_samples[6][49]
                );
            }
            if draws != WARMUP_FRAMES + MEASURED_FRAMES
                || warm_allocations != 0
                || warm_build_us >= smoke::FRAME_BUILD_LIMIT_US
            {
                return Err("Warm frame performance budget exceeded".into());
            }
            return Ok(());
        }
        let observed_at = if smoke { window.time() } else { now };
        if smoke && smoke_check.observe(observed_at, frames, draws, repaint.is_some())? {
            println!(
                "LocalWave smoke: frames={frames} draws={draws} size={}x{} idle_ms={:.1}",
                extent.logical.0,
                extent.logical.1,
                smoke_check.idle_ms(observed_at)
            );
            smoke_finished = true;
            break;
        }
        if performance {
            continue;
        }
        let deadline = if smoke {
            let deadline = smoke_check.deadline();
            Some(repaint.unwrap_or(deadline).min(deadline))
        } else {
            repaint
        };
        let progress = player
            .poll_interval()
            .map(|wait| window.time() + wait.as_secs_f64());
        window.wait(match (deadline, progress) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        });
    }
    if performance || (smoke && !smoke_finished) {
        return Err("Window closed before qualification finished".into());
    }
    graphics.close()
}

fn main() {
    if let Err(error) = run() {
        eprintln!("LocalWave: {error}");
        std::process::exit(1);
    }
}
