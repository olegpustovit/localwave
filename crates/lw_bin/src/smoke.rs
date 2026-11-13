/// Populate only the isolated qualification database, after its normal migration.
pub fn seed_catalog(path: &std::path::Path) -> std::io::Result<()> {
    let path = path
        .to_str()
        .ok_or_else(|| std::io::Error::other("Fixture path is not UTF-8"))?;
    let db = lw_apple_sys::sqlite::Connection::open(path).map_err(std::io::Error::other)?;
    db.execute_batch("BEGIN IMMEDIATE; WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM seq WHERE n<10000) INSERT INTO songs (songKey,artist,title,album,albumArtist,pathHash,createdAt,duration,fileExtension,isFavorite) SELECT n,'Artist '||(n%10),printf('Track %05d',n),'Album '||(n%100),'Artist '||(n%10),n,n,240,'m4a',n%2 FROM seq; INSERT INTO songs_fts(songId,artist,title,album,albumArtist) SELECT id,artist,title,album,albumArtist FROM songs; COMMIT").map_err(std::io::Error::other)
}

/// Observe a full idle interval after the first draw, independent of startup duration.
pub struct Smoke {
    timeout: f64,
    first_draw: Option<f64>,
    baseline: Option<(f64, usize, usize)>,
}

impl Smoke {
    pub fn new(now: f64) -> Self {
        Self {
            timeout: now + 5.0,
            first_draw: None,
            baseline: None,
        }
    }
    pub fn observe(&mut self, now: f64, frames: usize, draws: usize) -> Result<bool, &'static str> {
        if now >= self.timeout {
            return Err("Smoke test did not finish its idle observation before timeout");
        }
        if draws == 0 {
            return Ok(false);
        }
        let first = *self.first_draw.get_or_insert(now);
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
            .or_else(|| self.first_draw.map(|started| started + 0.25))
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
        assert!(!smoke.observe(2.0, 1, 1).unwrap());
        assert_eq!(smoke.deadline(), 2.25);
        assert!(!smoke.observe(2.3, 1, 1).unwrap());
        assert!(!smoke.observe(2.79, 1, 1).unwrap());
        assert!(smoke.observe(2.8, 1, 1).unwrap());
        assert!(smoke.idle_ms(2.8) >= 500.0);
        let mut changed = Smoke::new(0.0);
        assert!(!changed.observe(0.0, 1, 1).unwrap());
        assert!(!changed.observe(0.25, 1, 1).unwrap());
        assert!(changed.observe(0.75, 2, 2).is_err());
        let mut empty = Smoke::new(0.0);
        assert!(!empty.observe(4.99, 0, 0).unwrap());
        assert!(empty.observe(5.0, 0, 0).is_err());
    }
}
