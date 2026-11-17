//! Keep correlated frame/CPU/phase samples in fixed storage. Print outliers after measurement and
//! before sorting percentile copies. Preserve failures alongside passing evidence and never
//! substitute CPU/percentiles for existing wall assertions. Trace thread-state claims need frame-
//! specific interval overlap: the first thread column is the owner; later columns may identify a
//! waker/preemptor. Do not infer duration from adjacent context-switch records.
//!
//! Reports run only after measurement and GPU retirement; failures stay failures.
use super::*;
// M1 Mac mini limits chosen by the owner after traces showed E-core residency and preemption
// (P6): CPU p99 budgets with E-core headroom; frame work wall p99 under half a 60 Hz vsync.
const BUILD_CPU_LIMIT_US: u64 = 1_500;
const FRAME_CPU_LIMIT_US: u64 = 2_500;
const WORK_WALL_LIMIT_US: u128 = 8_000;
const CPU_PHASE_NAMES: [&str; 9] = [
    "begin",
    "build",
    "intrinsic",
    "arrange",
    "paint",
    "a11y",
    "finish",
    "output",
    "draw",
];

#[derive(Clone, Copy, Default)]
pub struct FrameTiming {
    pub wall: u128,
    pub cpu: u64,
    pub frame_wall: u128,
    pub frame_cpu: u64,
    pub work_wall: u128,
    pub phases: [u64; 7],
    pub phase_cpu: [u64; 9],
}
impl FrameTiming {
    /// Called after measurement; wall and thread CPU phases stay labeled and correlated.
    pub fn write_outlier(&self, frame: usize, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        if self.cpu >= BUILD_CPU_LIMIT_US
            || self.frame_cpu >= FRAME_CPU_LIMIT_US
            || self.work_wall >= WORK_WALL_LIMIT_US
        {
            writeln!(
                out,
                "LocalWave frame outlier: frame={frame} wall_us={} cpu_us={} frame_wall_us={} work_wall_us={} frame_cpu_us={} phase_wall_us={:?} phase_cpu_us={:?}",
                self.wall, self.cpu, self.frame_wall, self.work_wall, self.frame_cpu, self.phases, self.phase_cpu
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
        let from = self.measured_from();
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
                    .write_outlier(from + index + 1, &mut out)
                    .map_err(|error| error.to_string())?;
            }
        }
        let mut build_samples = (*timings).map(|sample| sample.wall);
        let mut cpu_samples = (*timings).map(|sample| sample.cpu);
        let mut frame_samples = (*timings).map(|sample| sample.frame_wall);
        let mut frame_cpu_samples = (*timings).map(|sample| sample.frame_cpu);
        let mut work_samples = (*timings).map(|sample| sample.work_wall);
        work_samples.sort_unstable();
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
        println!(
            "LocalWave work timing: wall_us_p50={} wall_us_p95={} wall_us_p99={} wall_us_max={}",
            work_samples[49], work_samples[94], work_samples[98], work_samples[99]
        );
        if matches!(workload, Some(Workload::Artwork | Workload::AlbumDetail)) {
            println!(
                "LocalWave artwork: texture_bytes={artwork_bytes} min_image_quads={min_artwork_quads} max_warm_requests={warm_artwork_requests}"
            );
            let needed = if workload == Some(Workload::Artwork) { 10 } else { 1 };
            if artwork_bytes == 0 || (*min_artwork_quads) < needed || (*warm_artwork_requests) != 0
            {
                return Err("Artwork workload did not retain its warmed image working set".into());
            }
        }
        let (marker, menu) = (self.marker_frames, self.menu_frames);
        let (first_min, first_max) = (self.detail_first_min, self.detail_first_max);
        let exercised = match workload {
            Some(Workload::AlbumDetail | Workload::ArtistDetail) => {
                println!(
                    "LocalWave detail: header_frames={marker} first_row_min={first_min} first_row_max={first_max}"
                );
                marker >= 50 && first_min < first_max
            }
            Some(Workload::ListDialogs) => {
                println!("LocalWave list dialogs: menu_frames={menu} name_dialog_frames={marker}");
                menu >= 15 && marker >= 35
            }
            Some(Workload::MetadataEditor) => {
                println!("LocalWave metadata editor: sheet_frames={marker}");
                marker == MEASURED_FRAMES
            }
            Some(Workload::SongDrag) => {
                println!("LocalWave song drag: carry_frames={marker} mutation_jobs=0");
                marker >= 50
            }
            _ => true,
        };
        if !exercised {
            return Err("Workload did not exercise its screen".into());
        }
        if trace_frames {
            let totals: [u64; 9] = std::array::from_fn(|phase| {
                timings.iter().map(|sample| sample.phase_cpu[phase]).sum()
            });
            let mut order = std::array::from_fn::<_, 9, _>(|phase| phase);
            order.sort_by_key(|&phase| std::cmp::Reverse(totals[phase]));
            print!("LocalWave phase CPU mean us:");
            for (phase, name) in CPU_PHASE_NAMES.iter().enumerate() {
                print!(
                    " {name}={:.2}",
                    totals[phase] as f64 / MEASURED_FRAMES as f64
                );
            }
            println!();
            print!("LocalWave top CPU phases: workload={name}");
            for phase in &order[..3] {
                print!(
                    " {}={:.2}",
                    CPU_PHASE_NAMES[*phase],
                    totals[*phase] as f64 / MEASURED_FRAMES as f64
                );
            }
            println!();
            let mut phase_samples: [[u64; MEASURED_FRAMES]; 7] =
                std::array::from_fn(|phase| (*timings).map(|sample| sample.phases[phase]));
            for samples in &mut phase_samples {
                samples.sort_unstable();
            }
            println!(
                "LocalWave phase wall medians us: begin={} build={} intrinsic={} arrange={} paint={} a11y={} finish={}",
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
        if workload.is_some_and(Workload::measured) {
            println!(
                "LocalWave measured: workload={name} gate=measured warm_rust_allocations={warm_allocations} warm_ready={}",
                self.warmup.is_some_and(super::Warmup::ready)
            );
            return Ok(());
        }
        if (*draws) != from + MEASURED_FRAMES
            || (*warm_allocations) != 0
            || (matches!(workload, Some(Workload::Clock120 | Workload::Idle600)) && self.warm_layouts != 0)
            || cpu_samples[98] >= BUILD_CPU_LIMIT_US
            || frame_cpu_samples[98] >= FRAME_CPU_LIMIT_US
            || work_samples[98] >= WORK_WALL_LIMIT_US
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
    fn reports_gate_cpu_spans_work_wall_p99_and_observed_cold_work() {
        let mut perf = Perf::new(Workload::Songs, None);
        perf.warmup = Some(super::super::Warmup::default());
        let counts = (
            WARMUP_FRAMES + MEASURED_FRAMES,
            WARMUP_FRAMES + MEASURED_FRAMES,
        );
        assert!(perf.report(false, counts, 0).is_ok());
        let limits = [
            BUILD_CPU_LIMIT_US as u128,
            FRAME_CPU_LIMIT_US as u128,
            WORK_WALL_LIMIT_US,
        ];
        for (span, limit) in limits.into_iter().enumerate() {
            let set = |sample: &mut FrameTiming, value: u128| match span {
                0 => sample.cpu = value as u64,
                1 => sample.frame_cpu = value as u64,
                _ => sample.work_wall = value,
            };
            perf.timings.iter_mut().for_each(|sample| set(sample, limit - 1));
            assert!(perf.report(false, counts, 0).is_ok());
            // p99 is the 99th of 100 sorted samples; one outlier above it is reported, not gated.
            set(&mut perf.timings[99], limit);
            assert!(perf.report(false, counts, 0).is_ok());
            set(&mut perf.timings[98], limit);
            assert!(perf.report(false, counts, 0).is_err());
            perf.timings.fill(Default::default());
        }
        // Wall maximum stays recorded but is not gated on this machine.
        perf.warm_build_us = 10_000;
        assert!(perf.report(false, counts, 0).is_ok());
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
    fn clock_120_and_idle_600_are_gated_on_allocations_and_text_layouts() {
        for workload in [Workload::Clock120, Workload::Idle600] {
            let mut perf = Perf::new(workload, None);
            perf.warmup = Some(super::super::Warmup::default());
            let frames = perf.measured_from() + MEASURED_FRAMES;
            let counts = (frames, frames);
            assert!(perf.report(false, counts, 0).is_ok(), "{workload:?}");
            perf.warm_allocations = 1;
            assert!(perf.report(false, counts, 0).is_err(), "{workload:?}");
            perf.warm_allocations = 0;
            perf.warm_layouts = 1;
            assert!(perf.report(false, counts, 0).is_err(), "{workload:?}");
        }
    }
    #[test]
    fn measured_workloads_report_but_never_fail_even_over_every_budget() {
        let mut perf = Perf::new(Workload::Fling, None);
        let counts = (WARMUP_FRAMES + MEASURED_FRAMES, 0);
        perf.warmup = None;
        perf.warm_allocations = 7;
        perf.timings.fill(FrameTiming {
            cpu: u64::MAX,
            frame_cpu: u64::MAX,
            work_wall: u128::MAX,
            ..Default::default()
        });
        assert!(perf.report(false, counts, 0).is_ok());
        perf.workload = Workload::Songs;
        assert!(perf.report(false, counts, 0).is_err());
    }
    #[test]
    fn outliers_keep_frame_cpu_and_phases_together_at_the_exact_gate_boundary() {
        let mut output = Vec::new();
        for (index, sample) in [
            FrameTiming {
                wall: 5000,
                cpu: 1499,
                frame_cpu: 2499,
                work_wall: 7999,
                phases: [9; 7],
                ..Default::default()
            },
            FrameTiming {
                wall: 1000,
                cpu: 1500,
                phases: [1, 2, 3, 4, 5, 6, 7],
                ..Default::default()
            },
            FrameTiming {
                wall: 1500,
                cpu: 400,
                work_wall: 8000,
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
            "LocalWave frame outlier: frame=84 wall_us=1000 cpu_us=1500 frame_wall_us=0 work_wall_us=0 frame_cpu_us=0 phase_wall_us=[1, 2, 3, 4, 5, 6, 7] phase_cpu_us=[0, 0, 0, 0, 0, 0, 0, 0, 0]\nLocalWave frame outlier: frame=85 wall_us=1500 cpu_us=400 frame_wall_us=0 work_wall_us=8000 frame_cpu_us=0 phase_wall_us=[7, 6, 5, 4, 3, 2, 1] phase_cpu_us=[0, 0, 0, 0, 0, 0, 0, 0, 0]\n"
        );
    }
}
