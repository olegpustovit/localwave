mod events;
mod gpu;
mod smoke;

use input::Event as HostEvent;
use lw_bin::{
    controller::{Command, Controller},
    library::LibraryWorker,
};
use pxui::{FrameIn, Platform, Str, ThemeSource, Ui};
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
    mem_sys::set_peer(pxui::library_counter);
    let mut window = Window::open_with("LocalWave", 1440, 872, true, false, true)?;
    let mut ui = Ui::create(Platform::Apple, Some(15), &[lw_ui::FONT, lw_ui::MONO]);
    ui.enable_system_fonts()
        .map_err(|code| format!("System fonts failed ({code}): {}", ui.last_error()))?;
    ui.set_theme(&ThemeSource {
        source: Str {
            ptr: lw_ui::LIGHT.as_ptr(),
            len: lw_ui::LIGHT.len(),
        },
        ..ThemeSource::default()
    })
    .map_err(|code| format!("Theme failed ({code}): {}", ui.last_error()))?;
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
                }
                Ok(library)
            },
            wake,
        )
    } else {
        Controller::open(LibraryWorker::current, wake)
    }
    .map_err(|error| error.to_string())?;
    // The library worker is already spawned, so it does not inherit the UI's interactive QoS.
    lw_apple_sys::ui_thread_priority().map_err(|error| error.to_string())?;
    if smoke || performance {
        let deadline = window.time() + 5.0;
        while !player.status().initialized {
            player.poll().map_err(|error| error.to_string())?;
            if window.time() >= deadline {
                return Err("Library initialization timed out during qualification".into());
            }
            if !player.status().initialized {
                window.wait(Some(window.time() + 0.05));
            }
        }
        if !player.status().message.is_empty() {
            return Err(player.status().message.to_string());
        }
    }
    if performance && player.catalog().len() != 10_000 {
        return Err("Performance fixture must contain 10,000 songs".into());
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
    let mut build_samples = [0u128; MEASURED_FRAMES];
    let mut cpu_samples = [0u64; MEASURED_FRAMES];
    let mut phase_samples = [[0u64; MEASURED_FRAMES]; 7];
    let mut smoke_check = smoke::Smoke::new(window.time());
    let mut smoke_finished = false;
    let mut shell_state = lw_ui::ShellState::default();
    let mut command_error: Option<&'static str> = None;
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
        let player_changed = player.poll().map_err(|error| error.to_string())?;
        if performance && frames >= 2 {
            events.push(HostEvent::PointerMoved((650.0, 400.0)));
            events.push(HostEvent::Scrolled((
                0.0,
                if (frames - 2) % 40 < 20 { -1.0 } else { 1.0 },
            )));
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
        }
        if first
            || performance
            || resized
            || !events.is_empty()
            || player_changed
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
            let mut intent = None;
            let status = player.status();
            let view = lw_ui::PlayerView {
                revision: status.revision + error_revision,
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
                let command = if shell_state.apply(intent) {
                    None
                } else {
                    Some(match intent {
                        lw_ui::Intent::Play => Command::Play,
                        lw_ui::Intent::Pause => Command::Pause,
                        lw_ui::Intent::Stop => Command::Stop,
                        lw_ui::Intent::Next => Command::Next,
                        lw_ui::Intent::Previous => Command::Previous,
                        lw_ui::Intent::PlayRow(index) => {
                            let Some(collection) = player
                                .catalog()
                                .collection(shell_state.route)
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
                        lw_ui::Intent::SeekFraction(fraction) => {
                            Command::Seek(fraction * player.status().duration)
                        }
                        lw_ui::Intent::Navigate(_)
                        | lw_ui::Intent::SelectRow(_)
                        | lw_ui::Intent::TogglePanel => unreachable!("handled by ShellState"),
                    })
                };
                let result = command.map_or(Ok(0), |command| player.command(command));
                command_error = match result {
                    Ok(_) => None,
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
            if performance && frames > WARMUP_FRAMES {
                warm_allocations = warm_allocations.max(mem_sys::local_alloc_count());
                warm_build_us = warm_build_us.max(build_us);
                build_samples[frames - WARMUP_FRAMES - 1] = build_us;
                cpu_samples[frames - WARMUP_FRAMES - 1] = cpu_us;
                if trace_frames {
                    for (samples, value) in phase_samples.iter_mut().zip(ui.phase_times()) {
                        samples[frames - WARMUP_FRAMES - 1] = value.as_micros() as u64;
                    }
                }
            }
        }
        if performance && frames >= WARMUP_FRAMES + MEASURED_FRAMES {
            graphics.close()?;
            build_samples.sort_unstable();
            cpu_samples.sort_unstable();
            println!(
                "LocalWave performance: songs=10000 frames={frames} draws={draws} max_warm_rust_allocations={warm_allocations} max_warm_build_us={warm_build_us}"
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
            if trace_frames {
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
                || warm_build_us >= 1000
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
