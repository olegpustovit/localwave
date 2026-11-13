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
                let library = LibraryWorker::open(
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
                    smoke::seed_catalog(&base.join("library.sqlite"))?;
                    if performance_artwork {
                        smoke::seed_artwork(&base.join("library.sqlite"), &base.join("Documents"))?;
                    }
                    if performance_queue {
                        library.save_player_state(&lw_core::preferences::PlayerState {
                            song_ids: (1..=10_000).collect(),
                            current_index: 0,
                            volume: Some(0.0),
                            context_data: None,
                        })?;
                    }
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
    // Both workers are already spawned, so they do not inherit the UI's interactive QoS.
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
                if !performance_queue || player.catalog().queue.is_some() {
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
    let mut extent = window.extent();
    let mut events: Vec<HostEvent> = Vec::with_capacity(256);
    let mut wire = Vec::with_capacity(256);
    let mut scratch = Vec::new();
    let mut first = true;
    let mut repaint = None;
    let mut revision = u64::MAX;
    let (mut frames, mut draws) = (0, 0);
    let (mut warm_allocations, mut warm_build_us) = (0, 0);
    let mut timings = [smoke::FrameTiming::default(); MEASURED_FRAMES];
    let (mut min_artwork_quads, mut warm_artwork_requests) = (usize::MAX, 0usize);
    let mut smoke_check = smoke::Smoke::new(window.time());
    let mut smoke_finished = false;
    let mut shell_state = lw_ui::ShellState::default();
    if performance_artwork {
        shell_state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Albums));
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
    let mut command_error: Option<&'static str> = None;
    if performance_queue_sheet {
        shell_state.apply(lw_ui::Intent::ShowQueue);
        shell_state.apply(lw_ui::Intent::NamePlaylist);
        shell_state.apply(lw_ui::Intent::EditPlaylistName(
            lw_ui::browse::Query::new("Qualification queue").unwrap(),
        ));
    }
    let mut error_revision = 0;
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
        let next_preferences = environment.snapshot();
        let preferences_changed = next_preferences != preferences;
        preferences = next_preferences;
        let player_changed = player
            .poll_with_catalog(&mut |previous, next| shell_state.refresh(previous, next))
            .map_err(|error| error.to_string())?;
        let artwork_changed = artwork.poll();
        let artwork_updated = graphics.sync_artwork(
            std::sync::Arc::as_ptr(&player.catalog().library) as usize,
            artwork.ready(),
        )?;
        if performance && frames >= 2 {
            if performance_queue_drag {
                smoke::queue_drag_events(frames - 2, window_chrome.top_inset, &mut events);
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
            let build_started = std::time::Instant::now();
            graphics.configure_artwork(
                extent.logical.0 as f32,
                extent.logical.1 as f32,
                window.scale() as f32,
            )?;
            graphics.artwork.images.begin_frame();
            let mut intent = None;
            let status = player.status();
            themes.apply(&mut ui, &status.settings, preferences);
            let view = lw_ui::PlayerView {
                artwork: Some(&graphics.artwork.images),
                revision: if performance {
                    frames as u64
                } else {
                    status.revision + error_revision
                },
                title: &status.title,
                context: status.context.title(),
                message: command_error.unwrap_or(&status.message),
                playing: status.wants_playback,
                initialized: status.initialized,
                current_id: status.current.map(|entry| entry.song_id),
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
                        preferences.reduced_motion != 0 || status.settings.reduce_motion,
                    ),
                    events: wire.as_ptr(),
                    event_count: wire.len(),
                    ..FrameIn::default()
                },
                &mut |ui| lw_ui::shell(ui, &view, player.catalog(), &shell_state, &mut intent),
            )
            .map_err(|code| format!("Frame failed ({code})"))?;
            let build_us = build_started.elapsed().as_micros();
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
                .request(graphics.artwork.images.requests())
                .map_err(|error| error.to_string())?;
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
                    return Err(
                        "Queue carry did not survive the cross-page scroll or cancel correctly"
                            .into(),
                    );
                }
                if intent.is_some() {
                    return Err("Cancelled queue carry produced an application action".into());
                }
            }
            let platform = &ui.output().platform;
            if platform.has_background != 0 {
                let color = platform.background;
                window.set_background((
                    f64::from(color.r) / 255.0,
                    f64::from(color.g) / 255.0,
                    f64::from(color.b) / 255.0,
                ))?;
            }
            frames += 1;
            repaint = ui.repaint_at();
            if performance || ui.draw_revision() != revision {
                if graphics.draw(&mut window, &ui)? {
                    draws += 1;
                    revision = ui.draw_revision();
                } else {
                    repaint = Some(now + 1.0 / 60.0);
                }
            }
            if let Some(intent) = intent {
                if matches!(
                    intent,
                    lw_ui::Intent::Navigate(_)
                        | lw_ui::Intent::TogglePanel
                        | lw_ui::Intent::CloseQueue
                ) {
                    artwork.cancel().map_err(|error| error.to_string())?;
                }
                let mut browse = None;
                let command = if shell_state.apply(intent) {
                    if matches!(
                        intent,
                        lw_ui::Intent::Navigate(_)
                            | lw_ui::Intent::Search(_)
                            | lw_ui::Intent::Sort(_)
                    ) {
                        browse = shell_state.request(player.catalog());
                        None
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
                        lw_ui::Intent::SeekFraction(fraction) => {
                            Command::Seek(fraction * player.status().duration)
                        }
                        lw_ui::Intent::Navigate(_)
                        | lw_ui::Intent::SelectRow(_)
                        | lw_ui::Intent::Search(_)
                        | lw_ui::Intent::SearchTooLong
                        | lw_ui::Intent::Sort(_)
                        | lw_ui::Intent::TogglePanel
                        | lw_ui::Intent::ShowQueue
                        | lw_ui::Intent::CloseQueue
                        | lw_ui::Intent::NamePlaylist
                        | lw_ui::Intent::EditPlaylistName(_)
                        | lw_ui::Intent::PlaylistNameTooLong
                        | lw_ui::Intent::CancelPlaylistName => {
                            unreachable!("handled by ShellState")
                        }
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
                warm_artwork_requests =
                    warm_artwork_requests.max(graphics.artwork.images.requests().keys().len());
            }
            if performance && frames > WARMUP_FRAMES {
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
                if performance_artwork {
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
        if smoke && smoke_check.observe(observed_at, frames, draws)? {
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
