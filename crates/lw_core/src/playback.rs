//! Transport intent and stale-load guards. Queue traversal and file work belong to the controller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Stopped,
    Loading,
    Buffering,
    Paused,
    Playing,
    Ended,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    File,
    Session,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    Continue,
    Wait,
    Load,
}
#[derive(Clone, Copy, Debug)]
pub struct Reading {
    pub generation: u64,
    pub token: u64,
    pub position: f64,
    pub duration: f64,
    pub ready: bool,
    pub playing: bool,
    pub waiting: bool,
}
#[derive(Default)]
pub struct Transport {
    generation: u64,
    token: u64,
    phase: Phase,
    wants_playback: bool,
    preparing: bool,
    resume_after_interruption: bool,
    position: f64,
    duration: f64,
    fault: Option<Fault>,
}
impl Transport {
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn token(&self) -> u64 {
        self.token
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn wants_playback(&self) -> bool {
        self.wants_playback
    }
    pub fn position(&self) -> f64 {
        self.position
    }
    pub fn duration(&self) -> f64 {
        self.duration
    }
    pub fn fault(&self) -> Option<Fault> {
        self.fault
    }
    fn invalidate(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("Playback generation exhausted");
    }
    pub fn begin(&mut self, autoplay: bool) -> u64 {
        self.stop();
        self.phase = Phase::Loading;
        self.preparing = true;
        self.wants_playback = autoplay;
        self.generation
    }
    pub fn adopt(&mut self, generation: u64, token: u64) -> bool {
        if generation != self.generation
            || token == 0
            || matches!(self.phase, Phase::Stopped | Phase::Ended | Phase::Failed)
        {
            return false;
        }
        if self.token == token {
            return true;
        }
        self.position = 0.0;
        self.duration = 0.0;
        self.token = token;
        self.preparing = false;
        self.phase = if self.wants_playback {
            Phase::Buffering
        } else {
            Phase::Paused
        };
        true
    }
    pub fn play(&mut self) -> Play {
        self.resume_after_interruption = false;
        self.wants_playback = true;
        if self.preparing {
            Play::Wait
        } else if self.token != 0
            && !matches!(self.phase, Phase::Stopped | Phase::Ended | Phase::Failed)
        {
            if self.phase != Phase::Playing {
                self.phase = Phase::Buffering;
            }
            Play::Continue
        } else {
            Play::Load
        }
    }
    pub fn pause(&mut self) {
        self.resume_after_interruption = false;
        if self.preparing {
            self.stop();
        } else {
            self.wants_playback = false;
            if !matches!(self.phase, Phase::Stopped | Phase::Ended | Phase::Failed) {
                self.phase = Phase::Paused;
            }
        }
    }
    pub fn stop(&mut self) {
        self.invalidate();
        self.token = 0;
        self.phase = Phase::Stopped;
        self.wants_playback = false;
        self.preparing = false;
        self.resume_after_interruption = false;
        self.position = 0.0;
        self.duration = 0.0;
        self.fault = None;
    }
    pub fn update(&mut self, reading: Reading) -> bool {
        if reading.generation != self.generation
            || reading.token == 0
            || reading.token != self.token
            || self.preparing
            || matches!(self.phase, Phase::Stopped | Phase::Ended | Phase::Failed)
        {
            return false;
        }
        if reading.duration.is_finite() && reading.duration > 0.0 {
            self.duration = reading.duration;
        }
        if reading.position.is_finite() && reading.position >= 0.0 {
            self.position = if self.duration > 0.0 {
                reading.position.min(self.duration)
            } else {
                reading.position
            };
        }
        self.phase = if self.wants_playback && reading.ready && reading.playing {
            Phase::Playing
        } else if !reading.ready || (self.wants_playback && reading.waiting) {
            Phase::Buffering
        } else {
            Phase::Paused
        };
        true
    }
    pub fn finish(&mut self, generation: u64, token: u64, duration: f64) -> bool {
        if generation != self.generation
            || token == 0
            || token != self.token
            || matches!(self.phase, Phase::Stopped | Phase::Failed)
        {
            return false;
        }
        if duration.is_finite() && duration > 0.0 {
            self.duration = duration;
        }
        self.position = self.duration;
        self.phase = Phase::Ended;
        self.wants_playback = false;
        self.preparing = false;
        true
    }
    pub fn fail(&mut self, generation: u64, token: u64, fault: Fault) -> bool {
        if generation != self.generation
            || token != self.token
            || matches!(self.phase, Phase::Stopped | Phase::Ended | Phase::Failed)
        {
            return false;
        }
        self.phase = Phase::Failed;
        self.wants_playback = false;
        self.preparing = false;
        self.resume_after_interruption = false;
        self.fault = Some(fault);
        true
    }
    pub fn interruption_began(&mut self) {
        let resume = self.wants_playback || self.phase == Phase::Playing;
        self.pause();
        self.resume_after_interruption = resume;
    }
    pub fn interruption_ended(&mut self, allowed: bool) -> Option<Play> {
        let resume = allowed && self.resume_after_interruption;
        self.resume_after_interruption = false;
        resume.then(|| self.play())
    }
    pub fn output_unavailable(&mut self) {
        self.pause();
    }
    pub(crate) fn await_next(&mut self) {
        self.token = 0;
        self.position = 0.0;
        self.duration = 0.0;
        self.preparing = true;
        self.phase = Phase::Loading;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reading(generation: u64, token: u64) -> Reading {
        Reading {
            generation,
            token,
            position: 0.25,
            duration: 1.25,
            ready: true,
            playing: true,
            waiting: false,
        }
    }
    #[test]
    fn generations_reject_late_preparation_activation_and_completion_after_pause_stop_or_replacement()
     {
        let mut state = Transport::default();
        let old = state.begin(true);
        state.pause();
        assert!(!state.adopt(old, 1));
        assert!(!state.wants_playback());
        let generation = state.begin(false);
        assert!(state.adopt(generation, 2));
        state.update(reading(generation, 2));
        assert_ne!(state.phase(), Phase::Playing);
        assert_eq!(state.play(), Play::Continue);
        state.update(reading(generation, 2));
        assert_eq!(state.phase(), Phase::Playing);
        assert!(!state.finish(old, 1, 8.0));
        assert!(!state.fail(old, 1, Fault::Session));
        let replacement = state.begin(true);
        assert!(!state.adopt(generation, 2));
        state.stop();
        assert!(!state.adopt(replacement, 3));
        assert!(!state.fail(replacement, 0, Fault::Session));
    }
    #[test]
    fn interruption_resume_respects_manual_pause_stop_route_loss_and_session_failures() {
        for action in 0..5 {
            let mut state = Transport::default();
            let generation = state.begin(true);
            state.adopt(generation, 1);
            state.update(reading(generation, 1));
            state.interruption_began();
            match action {
                1 => state.pause(),
                2 => state.stop(),
                3 => state.output_unavailable(),
                4 => {
                    state.fail(generation, 1, Fault::Session);
                }
                _ => {}
            }
            assert_eq!(
                state.interruption_ended(true),
                if action == 0 {
                    Some(Play::Continue)
                } else {
                    None
                }
            );
        }
        let mut state = Transport::default();
        state.begin(true);
        state.interruption_began();
        assert_eq!(state.interruption_ended(true), Some(Play::Load));
    }
    #[test]
    fn completion_retains_duration_replay_restarts_at_zero_and_warm_readings_allocate_nothing() {
        let mut state = Transport::default();
        let generation = state.begin(true);
        state.adopt(generation, 1);
        mem_sys::local_reset();
        for _ in 0..10000 {
            state.update(reading(generation, 1));
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        assert!(state.finish(generation, 1, f64::NAN));
        assert_eq!(
            (state.phase(), state.position(), state.duration()),
            (Phase::Ended, 1.25, 1.25)
        );
        assert!(!state.update(reading(generation, 1)));
        assert_eq!(state.play(), Play::Load);
        state.begin(true);
        assert_eq!(state.position(), 0.0);
        let generation = state.generation();
        state.adopt(generation, 2);
        let mut invalid = reading(generation, 2);
        invalid.position = f64::INFINITY;
        invalid.duration = -1.0;
        state.update(invalid);
        assert_eq!((state.position(), state.duration()), (0.0, 0.0));
    }
}
