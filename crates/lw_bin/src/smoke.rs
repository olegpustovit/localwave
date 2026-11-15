//! Startup and settled idle checks; clocks are supplied by the host.

/// Observe a full idle interval after startup motion, with the same five-second total bound.
pub struct Smoke {
    timeout: f64,
    quiet_from: Option<f64>,
    baseline: Option<(f64, usize, usize)>,
}

impl Smoke {
    pub fn new(now: f64) -> Self {
        Self {
            timeout: now + 5.0,
            quiet_from: None,
            baseline: None,
        }
    }
    pub fn observe(
        &mut self,
        now: f64,
        frames: usize,
        draws: usize,
        pending_frame: bool,
    ) -> Result<bool, &'static str> {
        if now >= self.timeout {
            return Err("Smoke test did not finish its idle observation before timeout");
        }
        if draws == 0 {
            return Ok(false);
        }
        if self.baseline.is_none() && pending_frame {
            self.quiet_from = Some(now);
            return Ok(false);
        }
        let first = *self.quiet_from.get_or_insert(now);
        if self.baseline.is_none() && now >= first + 0.25 {
            self.baseline = Some((now, frames, draws));
        }
        if let Some((started, built, rendered)) = self.baseline
            && now >= started + 0.5
        {
            if (frames, draws) != (built, rendered) {
                return Err("Idle window rebuilt or rendered a frame");
            }
            return Ok(true);
        }
        Ok(false)
    }
    pub fn deadline(&self) -> f64 {
        self.baseline
            .map(|(started, _, _)| started + 0.5)
            .or_else(|| self.quiet_from.map(|started| started + 0.25))
            .unwrap_or(self.timeout)
            .min(self.timeout)
    }
    pub fn idle_ms(&self, now: f64) -> f64 {
        self.baseline
            .map_or(0.0, |(started, _, _)| (now - started) * 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_delay_never_consumes_idle_observation() {
        let mut smoke = Smoke::new(0.0);
        assert!(!smoke.observe(2.0, 1, 1, false).unwrap());
        assert_eq!(smoke.deadline(), 2.25);
        assert!(!smoke.observe(2.3, 1, 1, false).unwrap());
        assert!(!smoke.observe(2.79, 1, 1, false).unwrap());
        assert!(smoke.observe(2.8, 1, 1, false).unwrap());
        assert!(smoke.idle_ms(2.8) >= 500.0);
        let mut changed = Smoke::new(0.0);
        assert!(!changed.observe(0.0, 1, 1, false).unwrap());
        assert!(!changed.observe(0.25, 1, 1, false).unwrap());
        assert!(changed.observe(0.75, 2, 2, false).is_err());
        let mut empty = Smoke::new(0.0);
        assert!(!empty.observe(4.99, 0, 0, false).unwrap());
        assert!(empty.observe(5.0, 0, 0, false).is_err());
    }
    #[test]
    fn startup_motion_must_finish_before_idle_and_keeps_the_original_timeout() {
        let mut smoke = Smoke::new(0.0);
        for frame in 1..=10 {
            assert!(
                !smoke
                    .observe(frame as f64 * 0.05, frame, frame, true)
                    .unwrap()
            );
        }
        assert!(!smoke.observe(0.75, 10, 10, false).unwrap());
        assert!(!smoke.observe(1.24, 10, 10, false).unwrap());
        assert!(smoke.observe(1.25, 10, 10, false).unwrap());
        assert_eq!(smoke.idle_ms(1.25), 500.0);
        let mut endless = Smoke::new(0.0);
        assert!(!endless.observe(4.99, 100, 100, true).unwrap());
        assert!(endless.observe(5.0, 101, 101, true).is_err());
    }
}
