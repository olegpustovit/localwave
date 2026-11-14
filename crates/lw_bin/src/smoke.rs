pub const FRAME_BUILD_LIMIT_US: u128 = 1_000;

/// Real external hover/completion routing without filesystem mutations in the fixture.
pub fn drop_events(frame: usize, events: &mut Vec<input::Event>) {
    match frame % 20 {
        0..=9 => events.push(input::Event::FileDragMoved((
            640.0 + (frame % 10) as f64,
            400.0,
        ))),
        10 => events.push(input::Event::FileDragEnded),
        _ => {}
    }
}

/// Exercise real Settings hit targets, all eight themes and both advanced menus in two cycles.
pub fn settings_events(
    frame: usize,
    top: f64,
    events: &mut Vec<input::Event>,
) -> Option<lw_ui::Intent> {
    use input::{Event, MouseButton};
    use lw_core::preferences::{Appearance, Change, Signal};
    use lw_ui::{Intent, SettingsMenu};
    let phase = frame % 40;
    let (button, at, action) = match phase & !1 {
        0 | 8 | 14 | 22 | 30 => (
            MouseButton::RightButton,
            (1330.0, top + 320.0),
            Intent::SettingsMenu(SettingsMenu::Appearance, (1330.0, top as f32 + 320.0)),
        ),
        4 | 18 => (
            MouseButton::RightButton,
            (1330.0, top + 260.0),
            Intent::SettingsMenu(SettingsMenu::Cache, (1330.0, top as f32 + 260.0)),
        ),
        2 => (
            MouseButton::LeftButton,
            (1300.0, top + 375.0),
            Intent::Setting(Change::Signal(Signal::Red)),
        ),
        6 => (
            MouseButton::LeftButton,
            (1300.0, top + 315.0),
            Intent::Setting(Change::CacheGB(32)),
        ),
        10 => (
            MouseButton::LeftButton,
            (1300.0, top + 411.0),
            Intent::Setting(Change::Signal(Signal::Green)),
        ),
        12 => (
            MouseButton::LeftButton,
            (
                if (frame / 40).is_multiple_of(2) {
                    1390.0
                } else {
                    1322.0
                },
                top + 320.0,
            ),
            Intent::Setting(Change::Appearance(if (frame / 40).is_multiple_of(2) {
                Appearance::Dark
            } else {
                Appearance::Light
            })),
        ),
        16 => (
            MouseButton::LeftButton,
            (1300.0, top + 447.0),
            Intent::Setting(Change::Signal(Signal::Mono)),
        ),
        20 => (
            MouseButton::LeftButton,
            (1300.0, top + 279.0),
            Intent::Setting(Change::CacheGB(2)),
        ),
        24 => (
            MouseButton::LeftButton,
            (1300.0, top + 339.0),
            Intent::Setting(Change::Signal(Signal::Blue)),
        ),
        26 => (
            MouseButton::LeftButton,
            (1390.0, top + 140.0),
            Intent::Setting(Change::ToggleDownloadAhead),
        ),
        28 => (
            MouseButton::LeftButton,
            (1390.0, top + 200.0),
            Intent::Setting(Change::ToggleNoNetwork),
        ),
        32 => (
            MouseButton::LeftButton,
            (1300.0, top + 483.0),
            Intent::Setting(Change::ToggleReduceMotion),
        ),
        _ => return None,
    };
    let pressed = phase.is_multiple_of(2);
    events.push(Event::PointerMoved(at));
    events.push(if pressed {
        Event::PointerPressed(button, at)
    } else {
        Event::PointerReleased(button, at)
    });
    ((button == MouseButton::RightButton) == pressed).then_some(action)
}

#[derive(Clone, Copy, Default)]
pub struct FrameTiming {
    pub wall: u128,
    pub cpu: u64,
    pub phases: [u64; 7],
}
impl FrameTiming {
    /// Called after measurement; phase durations are wall times and can include preemption.
    pub fn write_outlier(&self, frame: usize, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        if self.wall >= FRAME_BUILD_LIMIT_US {
            writeln!(
                out,
                "LocalWave frame outlier: frame={frame} wall_us={} cpu_us={} phase_wall_us={:?}",
                self.wall, self.cpu, self.phases
            )?;
        }
        Ok(())
    }
}

/// Repeat one pointer carry across 30 rows and cancel before returning to its source page.
pub fn queue_drag_events(frame: usize, grip: (f64, f64), events: &mut Vec<input::Event>) {
    use input::{Event, Key, MouseButton};
    match frame % 40 {
        0 => {
            events.push(Event::PointerMoved(grip));
            events.push(Event::PointerPressed(MouseButton::LeftButton, grip));
        }
        1 => events.push(Event::PointerMoved((grip.0, grip.1 + 88.0))),
        2 => events.push(Event::Scrolled((0.0, -30.0))),
        20 => events.push(Event::KeyPressed(Key::Escape)),
        21 => {
            events.push(Event::KeyReleased(Key::Escape));
            events.push(Event::PointerReleased(
                MouseButton::LeftButton,
                (grip.0, grip.1 + 88.0),
            ));
        }
        22 => events.push(Event::Scrolled((0.0, 30.0))),
        _ => {}
    }
}

/// Exercise a held waveform preview/release, then resume the 10,000-song wheel workload.
pub fn waveform_events(frame: usize, events: &mut Vec<input::Event>) {
    use input::{Event, MouseButton};
    match frame % 20 {
        0 => {
            events.push(Event::PointerMoved((650.0, 836.0)));
            events.push(Event::PointerPressed(
                MouseButton::LeftButton,
                (650.0, 836.0),
            ));
        }
        phase @ 1..=8 => events.push(Event::PointerMoved((650.0 + phase as f64 * 20.0, 836.0))),
        9 => events.push(Event::PointerReleased(
            MouseButton::LeftButton,
            (810.0, 836.0),
        )),
        _ => {
            events.push(Event::PointerMoved((650.0, 400.0)));
            events.push(Event::Scrolled((
                0.0,
                if frame % 40 < 20 { -1.0 } else { 1.0 },
            )));
        }
    }
}

/// A valid persisted waveform with no audio copy exercises the same cache delivery as eviction.
pub fn seed_waveform(caches: &std::path::Path) -> std::io::Result<()> {
    let directory = caches.join("LocalWave");
    std::fs::create_dir_all(&directory)?;
    let database = directory.join("waveforms.sqlite");
    let peaks = std::array::from_fn(|index| (index % 64) as f32 / 63.0);
    lw_core::waveform::Repository::open(
        database
            .to_str()
            .ok_or_else(|| std::io::Error::other("Fixture path is not UTF-8"))?,
    )
    .map_err(std::io::Error::other)?
    .put(1, "qualification-cached-waveform", &peaks)
    .map_err(std::io::Error::other)
}

/// Populate only the isolated qualification database, after its normal migration.
pub fn seed_catalog(path: &std::path::Path) -> std::io::Result<()> {
    let path = path
        .to_str()
        .ok_or_else(|| std::io::Error::other("Fixture path is not UTF-8"))?;
    let db = lw_apple_sys::sqlite::Connection::open(path).map_err(std::io::Error::other)?;
    db.execute_batch("BEGIN IMMEDIATE; WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM seq WHERE n<10000) INSERT INTO songs (songKey,artist,title,album,albumArtist,pathHash,createdAt,duration,fileExtension,isFavorite,releaseYear) SELECT n,'Artist '||(n%10),printf('Track %05d',n),'Album '||(n%100),'Artist '||(n%10),n,n,240,'m4a',n%2,2019 FROM seq; INSERT INTO songs_fts(songId,artist,title,album,albumArtist) SELECT id,artist,title,album,albumArtist FROM songs; COMMIT").map_err(std::io::Error::other)
}

/// Indexed metadata only: Files performance must never enumerate a user's real folders.
pub fn seed_files(library: &mut lw_bin::library::LibraryWorker) -> std::io::Result<()> {
    let account = library.current_sources()?;
    let source = library
        .repository
        .create_source(&lw_core::Source {
            user_id: account.user.id,
            dir_path: "/qualification/original-music".into(),
            path_id: 1_000_000,
            is_current: true,
            total_paths: Some(10_000),
            created_at: 1.0,
            ..Default::default()
        })
        .map_err(std::io::Error::other)?;
    let paths: Vec<_> = (1..=10_000)
        .map(|id| lw_core::SourcePath {
            source_id: source.id.unwrap(),
            path_id: id,
            parent_path_id: Some(source.path_id),
            name: format!("Track {id:05}.flac"),
            relative_path: format!("Track {id:05}.flac"),
            file_size: Some(2_000_000),
            created_at: 1.0,
            ..Default::default()
        })
        .collect();
    library
        .repository
        .upsert_source_paths(&paths)
        .map_err(std::io::Error::other)
}

pub fn seed_artwork(
    database: &std::path::Path,
    documents: &std::path::Path,
) -> std::io::Result<()> {
    let directory = documents.join("CoverArt");
    std::fs::create_dir_all(&directory)?;
    for index in 0..100 {
        std::fs::write(
            directory.join(format!("benchmark-{index}.png")),
            include_bytes!("../../../Tests/LocalWaveDataTests/Fixtures/artwork-large.png"),
        )?;
    }
    let database = database
        .to_str()
        .ok_or_else(|| std::io::Error::other("Fixture path is not UTF-8"))?;
    lw_apple_sys::sqlite::Connection::open(database)
        .map_err(std::io::Error::other)?
        .execute_batch("UPDATE songs SET coverArtPath='CoverArt/benchmark-'||(songKey%100)||'.png'")
        .map_err(std::io::Error::other)
}

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
    fn waveform_trace_holds_previews_releases_and_scrolls_without_allocating() {
        use input::{Event, MouseButton};
        let mut events = Vec::with_capacity(2);
        for frame in 0..120 {
            events.clear();
            mem_sys::local_reset();
            super::waveform_events(frame, &mut events);
            assert_eq!(mem_sys::local_alloc_count(), 0);
            match frame % 20 {
                0 => assert_eq!(
                    events,
                    [
                        Event::PointerMoved((650.0, 836.0)),
                        Event::PointerPressed(MouseButton::LeftButton, (650.0, 836.0))
                    ]
                ),
                phase @ 1..=8 => assert_eq!(
                    events,
                    [Event::PointerMoved((650.0 + phase as f64 * 20.0, 836.0))]
                ),
                9 => assert_eq!(
                    events,
                    [Event::PointerReleased(
                        MouseButton::LeftButton,
                        (810.0, 836.0)
                    )]
                ),
                _ => assert_eq!(
                    events,
                    [
                        Event::PointerMoved((650.0, 400.0)),
                        Event::Scrolled((0.0, if frame % 40 < 20 { -1.0 } else { 1.0 }))
                    ]
                ),
            }
        }
    }

    #[test]
    fn outliers_keep_frame_cpu_and_phases_together_at_the_exact_gate_boundary() {
        let mut output = Vec::new();
        for (index, sample) in [
            FrameTiming {
                wall: 999,
                cpu: 200,
                phases: [9; 7],
            },
            FrameTiming {
                wall: 1000,
                cpu: 300,
                phases: [1, 2, 3, 4, 5, 6, 7],
            },
            FrameTiming {
                wall: 1500,
                cpu: 400,
                phases: [7, 6, 5, 4, 3, 2, 1],
            },
        ]
        .iter()
        .enumerate()
        {
            sample.write_outlier(83 + index, &mut output).unwrap();
        }
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "LocalWave frame outlier: frame=84 wall_us=1000 cpu_us=300 phase_wall_us=[1, 2, 3, 4, 5, 6, 7]\nLocalWave frame outlier: frame=85 wall_us=1500 cpu_us=400 phase_wall_us=[7, 6, 5, 4, 3, 2, 1]\n"
        );
    }
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

    #[test]
    fn queue_drag_trace_repeats_exactly_and_reuses_its_event_buffer() {
        use input::{Event, Key, MouseButton};
        let mut events = Vec::with_capacity(4);
        for frame in 0..120 {
            events.clear();
            mem_sys::local_reset();
            let top_inset = (frame / 40) as f64 * 14.0;
            super::queue_drag_events(frame, (1151.0, 558.0 + top_inset), &mut events);
            assert_eq!(mem_sys::local_alloc_count(), 0);
            let expected: &[Event] = match frame % 40 {
                0 => &[
                    Event::PointerMoved((1151.0, 558.0 + top_inset)),
                    Event::PointerPressed(MouseButton::LeftButton, (1151.0, 558.0 + top_inset)),
                ],
                1 => &[Event::PointerMoved((1151.0, 646.0 + top_inset))],
                2 => &[Event::Scrolled((0.0, -30.0))],
                20 => &[Event::KeyPressed(Key::Escape)],
                21 => &[
                    Event::KeyReleased(Key::Escape),
                    Event::PointerReleased(MouseButton::LeftButton, (1151.0, 646.0 + top_inset)),
                ],
                22 => &[Event::Scrolled((0.0, 30.0))],
                _ => &[],
            };
            assert_eq!(events.as_slice(), expected);
        }
    }
}
