//! Real-time scenarios use the qualification catalog and normal playback/frame deadlines.
use lw_bin::controller::{Command, Controller, Status};
use lw_core::{playback::Phase, queue::RepeatMode};
use pxui_sdl::{Window, app::Schedule};
use std::path::Path;

const REPLAY_FRAMES: u64 = 160;
const REPLAY_HZ: f64 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scenario {
    PlayingList,
    Idle,
    Pointer,
    Hidden,
}

impl Scenario {
    pub(crate) fn from_args(args: &[String]) -> Result<Option<Self>, String> {
        let mut energy = args.iter().filter(|arg| arg.starts_with("--energy-"));
        let Some(option) = energy.next() else {
            return Ok(None);
        };
        if energy.next().is_some() || cfg!(target_os = "ios") {
            return Err("Select one macOS energy scenario".into());
        }
        Ok(Some(match option.as_str() {
            "--energy-playing-list" => Self::PlayingList,
            "--energy-idle" => Self::Idle,
            "--energy-pointer" => Self::Pointer,
            "--energy-hidden" => Self::Hidden,
            _ => return Err(format!("Unknown option: {option}")),
        }))
    }

    pub(crate) fn start(self, now: f64, player: &mut Controller) -> Result<Run, String> {
        let seconds =
            std::env::var("LOCALWAVE_ENERGY_SECONDS").map_err(|_| "Use tools/energy/run.sh")?;
        let seconds = seconds
            .parse::<u32>()
            .map_err(|_| "Energy duration must be an integer")?;
        if !(5..=120).contains(&seconds) || player.catalog().len() != 10_000 {
            return Err("Energy runs require 5–120 seconds and a 10,000-song fixture".into());
        }
        let fixed = std::env::var_os("LOCALWAVE_ENERGY_FIXED").is_some_and(|value| value == "1");
        if fixed && (seconds < 14 || !matches!(self, Self::PlayingList | Self::Pointer)) {
            return Err("Fixed replay requires playing-list or pointer and a trace of at least 14 seconds".into());
        }
        if self != Self::Idle {
            player
                .command(Command::Repeat(RepeatMode::One))
                .map_err(|e| e.to_string())?;
            player.command(Command::Play).map_err(|e| e.to_string())?;
        }
        Ok(Run {
            scenario: self,
            seconds,
            startup: now,
            settle: None,
            stop: None,
            next_pointer: now,
            pointer_tick: 0,
            fixed,
            replay_start: 0.0,
            replay_frame: 0,
            composite_samples: 0,
            composite_cpu_ns: 0,
        })
    }
}

pub(crate) struct Run {
    scenario: Scenario,
    seconds: u32,
    startup: f64,
    settle: Option<f64>,
    stop: Option<f64>,
    next_pointer: f64,
    pointer_tick: u64,
    fixed: bool,
    replay_start: f64,
    replay_frame: u64,
    composite_samples: u64,
    composite_cpu_ns: u64,
}

impl Run {
    pub(crate) fn measuring(&self) -> bool {
        self.stop.is_some()
    }

    pub(crate) fn record_composite_cpu(&mut self, cpu_ns: u64) {
        self.composite_samples += 1;
        self.composite_cpu_ns += cpu_ns;
    }

    pub(crate) fn input(&mut self, now: f64, events: &mut Vec<input::Event>) {
        events.retain(|event| {
            matches!(
                event,
                input::Event::Resized(_)
                    | input::Event::PixelSizeChanged(_)
                    | input::Event::FocusLost
                    | input::Event::FocusGained
                    | input::Event::Closed
            )
        });
        if let Some(due) = self.replay_due(now) {
            if due && self.scenario == Scenario::Pointer {
                let x = 550.0 + (self.replay_frame % 8) as f64;
                events.push(input::Event::PointerMoved((x, 184.0)));
            }
            return;
        }
        if self.pointer_tick == 0
            || (self.scenario == Scenario::Pointer && now >= self.next_pointer)
        {
            let position = if self.scenario == Scenario::Pointer {
                (550.0 + (self.pointer_tick % 8) as f64, 184.0)
            } else {
                (-100.0, -100.0)
            };
            events.push(input::Event::PointerMoved(position));
            self.pointer_tick += 1;
            self.next_pointer = now + 1.0 / 60.0;
        }
    }

    fn next_replay(&self) -> f64 {
        self.replay_start + self.replay_frame as f64 / REPLAY_HZ
    }

    pub(crate) fn replay_due(&self, now: f64) -> Option<bool> {
        (self.fixed && self.stop.is_some())
            .then(|| self.replay_frame < REPLAY_FRAMES && now >= self.next_replay())
    }

    pub(crate) fn replay_sample(&self) -> Option<(f64, f64)> {
        (self.fixed && self.stop.is_some())
            .then(|| (self.next_replay(), self.replay_frame as f64 / REPLAY_HZ))
    }

    pub(crate) fn record_replay(&mut self) {
        self.replay_frame += 1;
    }

    fn wait_until(&self, deadline: f64) -> Schedule {
        Schedule::WaitUntil(Some(if self.fixed && self.stop.is_some() {
            if self.replay_frame < REPLAY_FRAMES {
                deadline.min(self.next_replay())
            } else {
                deadline
            }
        } else if self.scenario == Scenario::Pointer {
            deadline.min(self.next_pointer)
        } else {
            deadline
        }))
    }

    pub(crate) fn advance(
        &mut self,
        window: &mut Window,
        status: &Status,
        draws: usize,
        ui: &pxui::Ui,
    ) -> Result<Schedule, String> {
        let now = window.time();
        if let Some(stop) = self.stop {
            if status.fault.is_some() {
                return Err(format!("Energy playback failed: {}", status.message));
            }
            if now >= stop {
                if self.fixed && self.replay_frame != REPLAY_FRAMES {
                    return Err(format!(
                        "Fixed replay presented {} of {REPLAY_FRAMES} frames",
                        self.replay_frame
                    ));
                }
                let reasons = ui.frame_reasons();
                println!(
                    "LOCALWAVE_ENERGY_DONE draws={draws} transition={} finite_keys={} stepped={}",
                    reasons.transition, reasons.finite_keys, reasons.stepped
                );
                if self.composite_samples > 0 {
                    println!("LOCALWAVE_ENERGY_COMPOSITE samples={} cpu_ns={}",
                        self.composite_samples, self.composite_cpu_ns);
                }
                return Ok(Schedule::Exit);
            }
            return Ok(self.wait_until(stop));
        }
        let playing = self.scenario == Scenario::Idle || status.phase == Phase::Playing;
        if !playing || draws == 0 {
            if now >= self.startup + 5.0 || status.fault.is_some() {
                return Err(format!(
                    "Energy fixture did not start: {:?} {}",
                    status.phase, status.message
                ));
            }
            return Ok(Schedule::WaitUntil(Some(self.startup + 5.0)));
        }
        if self.settle.is_none() {
            if self.scenario == Scenario::Hidden {
                window.hide()?;
            }
            self.settle = Some(now + 3.0);
        }
        let ready = self.settle.unwrap();
        if now < ready {
            return Ok(self.wait_until(ready));
        }
        let visible = window.visible()?;
        if visible == (self.scenario == Scenario::Hidden) {
            return Err("Energy window visibility differs from the requested scenario".into());
        }
        let (width, height) = window.extent().drawable;
        let reasons = ui.frame_reasons();
        println!(
            "LOCALWAVE_ENERGY_READY width={width} height={height} draws={draws} transition={} finite_keys={} stepped={}",
            reasons.transition, reasons.finite_keys, reasons.stepped
        );
        // Leave time for the trace to attach and flush before the native player shuts down.
        let stop = now + f64::from(self.seconds) + 8.0;
        self.replay_start = now + 3.0;
        self.stop = Some(stop);
        Ok(self.wait_until(stop))
    }
}

pub(super) fn seed(database: &Path, base: &Path) -> std::io::Result<()> {
    let folder = base.join("Documents/Added Files");
    std::fs::create_dir_all(&folder)?;
    std::fs::write(
        folder.join("silence.wav"),
        include_bytes!("../../../../tools/energy/silence.wav"),
    )?;
    let db = lw_sqlite::Connection::open(
        database
            .to_str()
            .ok_or_else(|| std::io::Error::other("Fixture path is not UTF-8"))?,
    )
    .map_err(std::io::Error::other)?;
    db.execute_batch("UPDATE songs SET localFilePath='Added Files/silence.wav', duration=8, fileExtension='wav' WHERE songKey=1")
        .map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::Scenario;

    #[test]
    fn fixed_replay_has_160_deadlines_and_an_exact_clock() {
        let mut run = super::Run {
            scenario: Scenario::PlayingList,
            seconds: 14,
            startup: 0.0,
            settle: None,
            stop: Some(22.0),
            next_pointer: 0.0,
            pointer_tick: 0,
            fixed: true,
            replay_start: 3.0,
            replay_frame: 0,
            composite_samples: 0,
            composite_cpu_ns: 0,
        };
        assert_eq!(run.replay_due(2.999), Some(false));
        for frame in 0..160 {
            assert_eq!(run.replay_due(3.0 + frame as f64 * 0.05 + 0.0001), Some(true));
            let position = frame as f64 / 20.0;
            assert_eq!(run.replay_sample(), Some((3.0 + position, position)));
            run.record_replay();
        }
        assert_eq!(run.replay_due(22.0), Some(false));
    }

    #[test]
    fn energy_scenarios_require_a_single_known_flag() {
        let args = |flags: &[&str]| {
            flags
                .iter()
                .map(|flag| flag.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            Scenario::from_args(&args(&["--energy-playing-list"])),
            Ok(Some(Scenario::PlayingList))
        );
        assert!(Scenario::from_args(&args(&["--energy-idle", "--energy-hidden"])).is_err());
        assert!(Scenario::from_args(&args(&["--energy-play"])).is_err());
    }
}
