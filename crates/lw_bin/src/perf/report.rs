//! Keep correlated frame/CPU/phase samples in fixed storage. Print outliers after measurement and
//! before sorting percentile copies. Preserve failures alongside passing evidence and never
//! substitute CPU/percentiles for existing wall assertions. Trace thread-state claims need frame-
//! specific interval overlap: the first thread column is the owner; later columns may identify a
//! waker/preemptor. Do not infer duration from adjacent context-switch records.
//!
//! Reports run only after measurement and GPU retirement; failures stay failures.
use super::*;
const FRAME_BUILD_LIMIT_US: u128 = 1_000;

#[derive(Clone, Copy, Default)]
pub struct FrameTiming {
    pub wall: u128,
    pub cpu: u64,
    pub frame_wall: u128,
    pub frame_cpu: u64,
    pub phases: [u64; 7],
}
impl FrameTiming {
    /// Called after measurement; phase durations are wall times and can include preemption.
    pub fn write_outlier(&self, frame: usize, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        if self.wall >= FRAME_BUILD_LIMIT_US || self.frame_cpu >= FRAME_BUILD_LIMIT_US as u64 {
            writeln!(
                out,
                "LocalWave frame outlier: frame={frame} wall_us={} cpu_us={} frame_wall_us={} frame_cpu_us={} phase_wall_us={:?}",
                self.wall, self.cpu, self.frame_wall, self.frame_cpu, self.phases
            )?;
        }
        Ok(())
    }
}

impl Perf {
    pub(super) fn report(
        &self,
        trace_frames: bool,
        counts: (usize, usize),
        artwork_bytes: usize,
    ) -> Result<(), String> {
        let Self {
            workload,
            waveform_seeks,
            settings_changes,
            settings_themes,
            min_artist_rows,
            max_artist_rows,
            first_artist_min,
            first_artist_max,
            min_waveform_bars,
            indicator_playing,
            indicator_paused,
            indicator_changes,
            drop_overlay_frames,
            warm_allocations,
            warm_build_us,
            timings,
            frame_trace,
            min_artwork_quads,
            warm_artwork_requests,
            min_files_rows,
            max_files_rows,
            ..
        } = self;
        let workload = Some(*workload);
        let frames = &counts.0;
        let draws = &counts.1;
        if trace_frames {
            for (frame, (ready, keys, host, build, draw, gpu, output)) in
                (*frame_trace).iter().enumerate()
            {
                if matches!(workload, Some(Workload::Artwork))
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
            for (index, sample) in (*timings).iter().enumerate() {
                sample
                    .write_outlier(WARMUP_FRAMES + index + 1, &mut out)
                    .map_err(|error| error.to_string())?;
            }
        }
        let mut build_samples = (*timings).map(|sample| sample.wall);
        let mut cpu_samples = (*timings).map(|sample| sample.cpu);
        let mut frame_samples = (*timings).map(|sample| sample.frame_wall);
        let mut frame_cpu_samples = (*timings).map(|sample| sample.frame_cpu);
        frame_samples.sort_unstable();
        frame_cpu_samples.sort_unstable();
        build_samples.sort_unstable();
        cpu_samples.sort_unstable();
        let name = workload.map_or("songs-original", Workload::name);
        println!(
            "LocalWave performance: workload={name} songs=10000 frames={frames} draws={draws} max_warm_rust_allocations={warm_allocations} max_warm_build_us={warm_build_us} max_measured_text_layouts={}",
            self.warm_layouts
        );
        println!(
            "LocalWave waveform: peaks=512 min_bars={min_waveform_bars} scrub_commits={waveform_seeks}"
        );
        if matches!(workload, Some(Workload::SongIndicators)) {
            println!(
                "LocalWave song indicator: visible_playing_frames={indicator_playing} visible_paused_frames={indicator_paused} changed_frames={indicator_changes}"
            );
            if (*indicator_playing) < 20 || (*indicator_paused) < 20 || (*indicator_changes) < 10 {
                return Err(
                    "Song indicator workload missed playback, pause or changing native draws"
                        .into(),
                );
            }
        }
        if matches!(workload, Some(Workload::Settings)) {
            println!(
                "LocalWave settings: changes={settings_changes} themes={settings_themes:#04x}"
            );
            if (*settings_changes) != 45 || (*settings_themes) != 255 {
                return Err("Settings workload missed an action or theme".into());
            }
        }
        if matches!(workload, Some(Workload::Artists)) {
            println!(
                "LocalWave artists: prepared_rows=10000 min_rows={min_artist_rows} max_rows={max_artist_rows} first_min={first_artist_min} first_max={first_artist_max}"
            );
            if (*min_artist_rows) < 12
                || (*max_artist_rows) > 40
                || (*first_artist_min) >= (*first_artist_max)
            {
                return Err("Artist workload did not scroll its measured virtual table".into());
            }
        }
        if matches!(workload, Some(Workload::Files)) {
            println!(
                "LocalWave files: indexed_rows=10000 min_drawn_rows={min_files_rows} max_drawn_rows={max_files_rows}"
            );
            if (*min_files_rows) < 8 || (*max_files_rows) > 40 {
                return Err("Files workload did not draw its virtualized rows".into());
            }
        }
        if matches!(workload, Some(Workload::Drop)) {
            println!(
                "LocalWave external drop: measured_overlay_frames={drop_overlay_frames} mutation_jobs=0"
            );
            if (*drop_overlay_frames) != 50 {
                return Err("External drag workload missed an overlay/cancellation cycle".into());
            }
        }
        if matches!(workload, Some(Workload::Waveform))
            && ((*waveform_seeks) != 9 || (*min_waveform_bars) < 200)
        {
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
        println!(
            "LocalWave frame timing: wall_us_p50={} wall_us_p95={} wall_us_p99={} wall_us_max={} cpu_us_p50={} cpu_us_p95={} cpu_us_p99={} cpu_us_max={}",
            frame_samples[49],
            frame_samples[94],
            frame_samples[98],
            frame_samples[99],
            frame_cpu_samples[49],
            frame_cpu_samples[94],
            frame_cpu_samples[98],
            frame_cpu_samples[99]
        );
        if matches!(workload, Some(Workload::Artwork)) {
            println!(
                "LocalWave artwork: texture_bytes={artwork_bytes} min_image_quads={min_artwork_quads} max_warm_requests={warm_artwork_requests}"
            );
            if artwork_bytes == 0 || (*min_artwork_quads) < 10 || (*warm_artwork_requests) != 0 {
                return Err("Artwork workload did not retain its warmed image working set".into());
            }
        }
        if trace_frames {
            let mut phase_samples: [[u64; MEASURED_FRAMES]; 7] =
                std::array::from_fn(|phase| (*timings).map(|sample| sample.phases[phase]));
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
        if let Some(warmup) = self.warmup {
            println!(
                "LocalWave warm-up: frame={WARMUP_FRAMES} pending_artwork={} catalog_in_flight={} text_layouts={} requests={} ready={}",
                warmup.artwork,
                warmup.catalog,
                warmup.layouts,
                warmup.requests,
                warmup.ready()
            );
        } else {
            println!("LocalWave warm-up: boundary was not observed");
        }
        if workload == Some(Workload::LongScroll) {
            println!(
                "LocalWave long scroll: first_row={} last_row={} advancing_frames={}",
                self.long_first, self.long_last, self.long_advances
            );
            if self.long_first == usize::MAX
                || self.long_last < self.long_first + 250
                || self.long_advances < 90
            {
                return Err("LongScroll did not visit new rows monotonically".into());
            }
        }
        if (*draws) != WARMUP_FRAMES + MEASURED_FRAMES
            || (*warm_allocations) != 0
            || (*warm_build_us) >= FRAME_BUILD_LIMIT_US
            || cpu_samples[98] >= FRAME_BUILD_LIMIT_US as u64
            || frame_cpu_samples[98] >= FRAME_BUILD_LIMIT_US as u64
            || !self.warmup.is_some_and(super::Warmup::ready)
        {
            return Err("Warm frame performance budget exceeded".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_gate_both_cpu_spans_wall_max_and_observed_cold_work() {
        let mut perf = Perf::new(Workload::Songs, None);
        perf.warmup = Some(super::super::Warmup::default());
        let counts = (
            WARMUP_FRAMES + MEASURED_FRAMES,
            WARMUP_FRAMES + MEASURED_FRAMES,
        );
        assert!(perf.report(false, counts, 0).is_ok());
        for frame_span in [false, true] {
            for sample in &mut perf.timings {
                if frame_span {
                    sample.frame_cpu = 999;
                } else {
                    sample.cpu = 999;
                }
            }
            assert!(perf.report(false, counts, 0).is_ok());
            for sample in &mut perf.timings[98..] {
                if frame_span {
                    sample.frame_cpu = 1000;
                } else {
                    sample.cpu = 1000;
                }
            }
            assert!(perf.report(false, counts, 0).is_err());
            perf.timings.fill(Default::default());
        }
        perf.warm_build_us = 1000;
        assert!(perf.report(false, counts, 0).is_err());
        perf.warm_build_us = 0;
        for cold in [
            super::super::Warmup {
                artwork: true,
                ..Default::default()
            },
            super::super::Warmup {
                catalog: true,
                ..Default::default()
            },
            super::super::Warmup {
                layouts: 1,
                ..Default::default()
            },
            super::super::Warmup {
                requests: 1,
                ..Default::default()
            },
        ] {
            perf.warmup = Some(cold);
            assert!(perf.report(false, counts, 0).is_err());
        }
        perf.warmup = None;
        assert!(perf.report(false, counts, 0).is_err());
    }
    #[test]
    fn outliers_keep_frame_cpu_and_phases_together_at_the_exact_gate_boundary() {
        let mut output = Vec::new();
        for (index, sample) in [
            FrameTiming {
                wall: 999,
                cpu: 200,
                phases: [9; 7],
                ..Default::default()
            },
            FrameTiming {
                wall: 1000,
                cpu: 300,
                phases: [1, 2, 3, 4, 5, 6, 7],
                ..Default::default()
            },
            FrameTiming {
                wall: 1500,
                cpu: 400,
                phases: [7, 6, 5, 4, 3, 2, 1],
                ..Default::default()
            },
        ]
        .iter()
        .enumerate()
        {
            sample.write_outlier(83 + index, &mut output).unwrap();
        }
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "LocalWave frame outlier: frame=84 wall_us=1000 cpu_us=300 frame_wall_us=0 frame_cpu_us=0 phase_wall_us=[1, 2, 3, 4, 5, 6, 7]\nLocalWave frame outlier: frame=85 wall_us=1500 cpu_us=400 frame_wall_us=0 frame_cpu_us=0 phase_wall_us=[7, 6, 5, 4, 3, 2, 1]\n"
        );
    }
}
