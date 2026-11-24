//! Real playback and deterministic input in a counting, isolated macOS bundle.
use super::{Workload, drive, measured};
use input::Event;
use lw_bin::controller::{Command, Controller, Status};
use lw_core::{playback::Phase, preferences::Signal, queue::RepeatMode};
use pxui_sdl::{Window, app::Schedule};
use std::time::{Duration, Instant};

const PHASES: [&str; 3] = ["playback", "scroll", "churn"];
const PLAYBACK_WARMUP_SECONDS: f64 = 4.0;
// Scroll repeats both fling directions. Churn repeats both sizes, all appearance/accent
// combinations and both overlays. A display callback can arrive later than a scripted tick.
const SCRIPT_CYCLE: usize = 240;
const TICK_SECONDS: f64 = 1.0 / 60.0;
const MEMORY_TOLERANCE_BYTES: i64 = 64 * 1024 * 1024;
const BUCKET_US: u64 = 100;
const PRECISE_BUCKETS: usize = 1_001;
const BUCKETS: usize = PRECISE_BUCKETS + u64::BITS as usize;

#[derive(Clone, Copy)]
pub(crate) struct Config {
    seconds: [u32; 3],
    sample_seconds: u32,
    verify_every: usize,
}

impl Config {
    pub(crate) fn from_args(args: &[String]) -> Result<Option<Self>, String> {
        match args.iter().skip(1).filter(|arg| *arg == "--soak").count() {
            0 => return Ok(None),
            1 if cfg!(target_os = "macos") => {}
            _ => return Err("Select one macOS soak".into()),
        }
        let paths =
            lw_apple_sys::preferences::AppPaths::load().map_err(|error| error.to_string())?;
        if !paths
            .documents
            .ends_with("Library/Containers/com.olegpustovit.localwave.soak/Data/Documents")
        {
            return Err("Soak needs the isolated signed bundle created by make soak".into());
        }
        Ok(Some(Self {
            seconds: [
                positive_env("LOCALWAVE_SOAK_PLAYBACK_SECONDS", 21_600)?,
                positive_env("LOCALWAVE_SOAK_SCROLL_SECONDS", 7_200)?,
                positive_env("LOCALWAVE_SOAK_CHURN_SECONDS", 1_800)?,
            ],
            sample_seconds: positive_env("LOCALWAVE_SOAK_SAMPLE_SECONDS", 60)?,
            verify_every: positive_env("LOCALWAVE_SOAK_VERIFY_EVERY", 60)? as usize,
        }))
    }
}

fn positive_env(key: &str, default: u32) -> Result<u32, String> {
    match std::env::var(key) {
        Err(std::env::VarError::NotPresent) => Ok(default),
        value => value
            .ok()
            .and_then(|text| text.parse().ok())
            .filter(|n| *n > 0)
            .ok_or_else(|| format!("{key} must be a positive integer")),
    }
}

pub(crate) struct Frame {
    started: Instant,
    cpu_ns: u64,
}

impl Frame {
    pub(crate) fn start() -> Result<Self, String> {
        mem_sys::local_reset();
        Ok(Self {
            started: Instant::now(),
            cpu_ns: thread_cpu_ns()?,
        })
    }
}

fn thread_cpu_ns() -> Result<u64, String> {
    mem_sys::profile::thread_cpu_time_ns().map_err(|error| error.to_string())
}

struct Timing {
    counts: [u64; BUCKETS],
    samples: u64,
}

impl Timing {
    const EMPTY: Self = Self {
        counts: [0; BUCKETS],
        samples: 0,
    };

    fn record(&mut self, us: u64) {
        let units = us.div_ceil(BUCKET_US);
        let bucket = if units < PRECISE_BUCKETS as u64 {
            units as usize
        } else {
            PRECISE_BUCKETS + (units - 1).ilog2() as usize + 1
        };
        self.counts[bucket] += 1;
        self.samples += 1;
    }

    fn p99(&self) -> u64 {
        let rank = (self.samples * 99).div_ceil(100);
        let mut seen = 0;
        for (bucket, count) in self.counts.iter().enumerate() {
            seen += count;
            if seen >= rank {
                return if bucket >= PRECISE_BUCKETS {
                    (1_u64 << (bucket - PRECISE_BUCKETS)).saturating_mul(BUCKET_US)
                } else {
                    bucket as u64 * BUCKET_US
                };
            }
        }
        0
    }
}

struct Stats {
    cpu: Timing,
    wall: Timing,
    frames: usize,
    draws: usize,
    max_allocations: u64,
    allocating_frames: usize,
    oracle_checks: usize,
    mismatches: usize,
    rss: [i64; 2],
    footprint: [i64; 2],
}

impl Stats {
    const EMPTY: Self = Self {
        cpu: Timing::EMPTY,
        wall: Timing::EMPTY,
        frames: 0,
        draws: 0,
        max_allocations: 0,
        allocating_frames: 0,
        oracle_checks: 0,
        mismatches: 0,
        rss: [i64::MAX, 0],
        footprint: [i64::MAX, 0],
    };

    fn sample(&mut self, kind: &str, phase: &str, elapsed: f64) -> Result<bool, String> {
        let (rss, footprint) = memory()?;
        self.rss = [self.rss[0].min(rss), self.rss[1].max(rss)];
        self.footprint = [
            self.footprint[0].min(footprint),
            self.footprint[1].max(footprint),
        ];
        let rss_span = self.rss[1] - self.rss[0];
        let footprint_span = self.footprint[1] - self.footprint[0];
        println!(
            concat!(
                r#"{{"kind":"{}","phase":"{}","seconds":{:.3},"frames":{},"draws":{},"allocating_frames":{},"max_warm_rust_allocations":{},"oracle_checks":{},"oracle_mismatches":{},"rss_bytes":{},"footprint_bytes":{},"rss_span_bytes":{},"footprint_span_bytes":{},"memory_tolerance_bytes":{},"frame_cpu_p99_us":{},"work_wall_p99_us":{}}}"#
            ),
            kind,
            phase,
            elapsed,
            self.frames,
            self.draws,
            self.allocating_frames,
            self.max_allocations,
            self.oracle_checks,
            self.mismatches,
            rss,
            footprint,
            rss_span,
            footprint_span,
            MEMORY_TOLERANCE_BYTES,
            self.cpu.p99(),
            self.wall.p99(),
        );
        Ok(self.max_allocations == 0
            && self.mismatches == 0
            && rss_span <= MEMORY_TOLERANCE_BYTES
            && footprint_span <= MEMORY_TOLERANCE_BYTES)
    }
}

fn memory() -> Result<(i64, i64), String> {
    #[cfg(target_os = "macos")]
    if let (Some(rss), Some(footprint)) = (mem_sys::resident_size(), mem_sys::phys_footprint()) {
        return Ok((rss, footprint));
    }
    Err("Soak could not read RSS and physical footprint".into())
}

pub(crate) struct Run {
    config: Config,
    startup: f64,
    last_playing: f64,
    stage: Stage,
    next_sample: f64,
    phase: usize,
    next_tick: f64,
    tick: usize,
    pointer_parked: bool,
    verified_warmup: bool,
    failed: bool,
    stats: Stats,
}

#[derive(Clone, Copy)]
enum Stage {
    Starting,
    Warmup { until: f64 },
    Measuring { started: f64 },
}

impl Run {
    fn measuring(&self) -> bool {
        matches!(self.stage, Stage::Measuring { .. })
    }
    pub(crate) fn new(config: Config, now: f64, player: &mut Controller) -> Result<Self, String> {
        if player.catalog().len() != 10_000 {
            return Err("Soak requires the 10,000-song fixture".into());
        }
        player
            .command(Command::Repeat(RepeatMode::One))
            .map_err(|error| error.to_string())?;
        player
            .command(Command::Play)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            config,
            startup: now,
            last_playing: now,
            stage: Stage::Starting,
            next_sample: now,
            phase: 0,
            next_tick: now,
            tick: 0,
            pointer_parked: false,
            verified_warmup: false,
            failed: false,
            stats: Stats::EMPTY,
        })
    }

    pub(crate) fn input(
        &mut self,
        window: &mut Window,
        ui: &mut pxui::Ui,
        events: &mut Vec<Event>,
        drive: bool,
    ) -> Result<bool, String> {
        events.retain(|event| {
            matches!(
                event,
                Event::Resized(_)
                    | Event::PixelSizeChanged(_)
                    | Event::FocusLost
                    | Event::FocusGained
                    | Event::Closed
            )
        });
        if !drive {
            return Ok(false);
        }
        if !self.pointer_parked {
            events.push(Event::PointerMoved((-100.0, -100.0)));
            self.pointer_parked = true;
        }
        let now = window.time();
        if self.phase == 0 || now < self.next_tick {
            return Ok(false);
        }
        let tick = self.tick;
        self.tick += 1;
        self.next_tick = (self.next_tick + TICK_SECONDS).max(now + TICK_SECONDS / 2.0);
        if self.phase == 1 {
            let workload =
                [Workload::Fling, Workload::ReverseScroll, Workload::Jump][(tick / 40) % 3];
            if let Some(offset) = scroll_jump(tick, workload) {
                let list = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| {
                        node.is_role(pxui::ui::Role::List) && node.rect.contains((650.0, 400.0))
                    })
                    .and_then(|node| ui.box_of_a11y(node.id))
                    .ok_or("Soak Songs list is missing")?;
                ui.set_scroll_offset(&list, offset);
            }
            if workload == Workload::Jump {
                events.push(Event::PointerMoved((650.0, 400.0)));
            } else {
                measured::events(workload, tick, 0, ui, events)?;
            }
        } else {
            if tick % 120 == 0 {
                window.resize(1440, 872)?;
            } else if tick % 120 == 60 {
                window.resize(1240, 772)?;
            }
            drive::list_dialog_events(tick, ui, events)?;
        }
        Ok(true)
    }

    pub(crate) fn theme(&self) -> Option<(u32, Signal)> {
        (self.phase == 2).then(|| {
            let accent = [Signal::Blue, Signal::Green][self.tick / 40 % 2];
            (measured::appearance(self.tick / 3), accent)
        })
    }

    /// Snapshot input/build/draw costs before invoking the oracle or printing a sample.
    pub(crate) fn record(
        &mut self,
        frame: Frame,
        built: bool,
        drew: bool,
        acquire: Duration,
    ) -> Result<bool, String> {
        let measured = self.measuring();
        if !measured {
            let verify = drew && !self.verified_warmup;
            self.verified_warmup |= verify;
            return Ok(verify);
        }
        let allocations = mem_sys::local_alloc_count();
        self.stats.max_allocations = self.stats.max_allocations.max(allocations);
        self.stats.allocating_frames += usize::from(allocations > 0);
        if built {
            self.stats.frames += 1;
            self.stats
                .cpu
                .record((thread_cpu_ns()? - frame.cpu_ns) / 1000);
            self.stats
                .wall
                .record(frame.started.elapsed().saturating_sub(acquire).as_micros() as u64);
        }
        self.stats.draws += usize::from(drew);
        Ok(drew && (self.stats.draws - 1) % self.config.verify_every == 0)
    }

    pub(crate) fn oracle(&mut self, matches: bool) -> Result<(), String> {
        if self.measuring() {
            self.stats.oracle_checks += 1;
            self.stats.mismatches += usize::from(!matches);
        } else if !matches {
            return Err(format!(
                "Soak {} warm-up canvas differs from the full redraw",
                PHASES[self.phase]
            ));
        }
        Ok(())
    }

    pub(crate) fn deadline(&self) -> f64 {
        match self.stage {
            Stage::Starting => self.startup + 5.0,
            Stage::Warmup { until } if self.phase == 0 => until,
            Stage::Warmup { .. } => self.next_tick,
            Stage::Measuring { started } => {
                let stop = started + f64::from(self.config.seconds[self.phase]);
                let deadline = self.next_sample.min(stop);
                if self.phase == 0 {
                    deadline
                } else {
                    deadline.min(self.next_tick)
                }
            }
        }
    }

    pub(crate) fn advance(
        &mut self,
        now: f64,
        status: &Status,
        draws: usize,
    ) -> Result<Schedule, String> {
        if status.fault.is_some() {
            return Err(format!("Soak playback failed: {}", status.message));
        }
        if status.phase == Phase::Playing {
            self.last_playing = now;
        } else if !matches!(self.stage, Stage::Starting)
            && (!status.wants_playback || now - self.last_playing >= 5.0)
        {
            return Err(format!(
                "Soak playback stopped during {}",
                PHASES[self.phase]
            ));
        }
        let start = match self.stage {
            Stage::Starting => {
                if status.phase == Phase::Playing && draws > 0 {
                    self.stage = Stage::Warmup {
                        until: now + PLAYBACK_WARMUP_SECONDS,
                    };
                } else if now >= self.startup + 5.0 {
                    return Err("Soak playback or rendering did not start".into());
                }
                return Ok(Schedule::WaitUntil(Some(self.deadline())));
            }
            Stage::Warmup { until } => {
                if now >= until && (self.phase == 0 || self.tick >= SCRIPT_CYCLE) {
                    self.stage = Stage::Measuring { started: now };
                    self.next_sample = now;
                }
                return Ok(Schedule::WaitUntil(Some(self.deadline())));
            }
            Stage::Measuring { started } => started,
        };
        let elapsed = now - start;
        let done = elapsed >= f64::from(self.config.seconds[self.phase]);
        if done || now >= self.next_sample {
            let kind = if done { "phase_end" } else { "sample" };
            self.failed |= !self
                .stats
                .sample(kind, PHASES[self.phase], elapsed.max(0.0))?;
            self.next_sample = now + f64::from(self.config.sample_seconds);
        }
        if done {
            self.failed |= self.stats.draws == 0 || self.stats.oracle_checks == 0;
            if self.phase + 1 == PHASES.len() {
                println!(r#"{{"kind":"complete","passed":{}}}"#, !self.failed);
                return if self.failed {
                    Err("Soak qualification failed; see the JSON samples".into())
                } else {
                    Ok(Schedule::Exit)
                };
            }
            self.phase += 1;
            self.stats = Stats::EMPTY;
            self.stage = Stage::Warmup { until: now };
            self.next_tick = now;
            self.tick = 0;
            self.verified_warmup = false;
        }
        Ok(Schedule::WaitUntil(Some(self.deadline())))
    }
}

// Each jump segment returns the viewport to the same starting region before the next fling.
fn scroll_jump(tick: usize, workload: Workload) -> Option<f32> {
    if tick == 0 {
        Some(600.0)
    } else if workload == Workload::Jump && tick % 10 == 0 {
        Some(if tick % 20 == 0 { 19_600.0 } else { 600.0 })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, SCRIPT_CYCLE, Timing, Workload, scroll_jump};

    #[test]
    fn every_scroll_cycle_jumps_to_the_same_regions_and_returns_to_its_origin() {
        assert_eq!(scroll_jump(0, Workload::Fling), Some(600.0));
        for cycle in 0..2 {
            for start in [80, 200] {
                let first = cycle * SCRIPT_CYCLE + start;
                assert_eq!(scroll_jump(first, Workload::Jump), Some(19_600.0));
                assert_eq!(scroll_jump(first + 1, Workload::Jump), None);
                assert_eq!(scroll_jump(first + 30, Workload::Jump), Some(600.0));
            }
        }
    }

    #[test]
    fn soak_percentile_uses_all_frames_and_retains_overflow() {
        let mut timing = Timing::EMPTY;
        assert_eq!(timing.p99(), 0);
        for _ in 0..99 {
            timing.record(101);
        }
        timing.record(120_000);
        assert_eq!(timing.p99(), 200);
        timing.record(180_000);
        assert_eq!(timing.p99(), 204_800);
    }

    #[test]
    fn soak_rejects_duplicate_flags_before_checking_the_sandbox() {
        let args = ["localwave", "--soak", "--soak"].map(str::to_owned);
        assert!(Config::from_args(&args).is_err());
        assert!(Config::from_args(&args[..1]).unwrap().is_none());
    }
}
