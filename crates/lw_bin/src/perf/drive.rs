//! Stack frame state preserves input, build, output and draw measurement boundaries.
use super::{MEASURED_FRAMES, Perf, WARMUP_FRAMES, Workload};
use crate::gpu::Graphics;
use input::Event as HostEvent;
use pxui::{FrameIn, Ui};
use pxui_sdl::app::Schedule;

pub(crate) struct Frame {
    workload: Option<Workload>,
    index: usize,
    trace_frames: bool,
    indicator_song: Option<i64>,
    expected_settings_intent: Option<lw_ui::Intent>,
    settings_changes: usize,
    settings_themes: u8,
    waveform_seeks: i32,
    timing: super::report::FrameTiming,
    allocations_before: u64,
    allocations_after: u64,
    gpu: [u64; 4],
    pub(crate) output_allocations: u64,
    ids: [usize; 8],
    requests: usize,
    pub(crate) artwork_updated: bool,
}

pub(crate) struct Build {
    interval: Option<lw_apple_sys::timing::FrameInterval>,
    cpu_started: u64,
    allocations_before: u64,
    started: std::time::Instant,
}
impl Frame {
    pub(crate) fn new(index: usize, trace_frames: bool) -> Self {
        Self {
            workload: None,
            index,
            trace_frames,
            indicator_song: None,
            expected_settings_intent: None,
            settings_changes: 0,
            settings_themes: 0,
            waveform_seeks: 0,
            timing: Default::default(),
            allocations_before: 0,
            allocations_after: 0,
            gpu: [0; 4],
            output_allocations: 0,
            ids: [0; 8],
            requests: 0,
            artwork_updated: false,
        }
    }
    pub(crate) fn external_input(&self, events: &mut Vec<HostEvent>) {
        if matches!(self.workload, Some(Workload::Drop)) && self.index >= 2 {
            drop_events(self.index - 2, events);
        }
    }
    pub(crate) fn input(
        &mut self,
        ui: &Ui,
        top_inset: f64,
        events: &mut Vec<HostEvent>,
    ) -> Result<(), String> {
        let workload = self.workload;
        let mut expected_settings_intent = None;
        if workload.is_some() && self.index >= 2 && !matches!(workload, Some(Workload::Drop)) {
            if matches!(workload, Some(Workload::Settings)) {
                expected_settings_intent = settings_events(self.index - 2, top_inset, &mut *events);
            } else if matches!(workload, Some(Workload::Waveform)) {
                waveform_events(self.index - 2, &mut *events);
            } else if matches!(workload, Some(Workload::QueueDrag)) {
                let grip = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| {
                        ui.a11y_name_of(node) == "Reorder queued track"
                            && node.states & pxui::a11y::state::VISIBLE != 0
                    })
                    .ok_or("Queue drag fixture has no visible reorder target")?
                    .rect;
                queue_drag_events(
                    self.index - 2,
                    (f64::from(grip.x + 10.0), f64::from(grip.y + 10.0)),
                    &mut *events,
                );
            } else {
                events.push(HostEvent::PointerMoved(
                    if matches!(workload, Some(Workload::QueueSheet))
                        || matches!(workload, Some(Workload::Artwork))
                    {
                        (650.0, 400.0)
                    } else if workload.is_some_and(Workload::uses_queue) {
                        (1300.0, 690.0)
                    } else {
                        (650.0, 400.0)
                    },
                ));
                events.push(HostEvent::Scrolled((
                    0.0,
                    (if (self.index - 2) % 40 < 20 {
                        -1.0
                    } else {
                        1.0
                    }) * if matches!(workload, Some(Workload::Artwork)) {
                        0.2
                    } else {
                        1.0
                    },
                )));
            }
        }
        self.expected_settings_intent = expected_settings_intent;
        Ok(())
    }
    pub(crate) fn require_extent(&self, extent: pxui_sdl::Extent) -> Result<(), String> {
        if self.workload.is_some() && extent.logical != (1440, 872) {
            return Err("Performance qualification requires a 1440x872 logical window".into());
        }
        Ok(())
    }
    pub(crate) fn require_waveform(&self, available: bool) -> Result<(), String> {
        if self.workload.is_some() && !available {
            return Err("Waveform disappeared during performance qualification".into());
        }
        Ok(())
    }
    pub(crate) fn view(&self, view: &mut lw_ui::PlayerView<'_>, input: &mut FrameIn) {
        if self.workload.is_some() {
            view.revision = self.index as u64;
            view.position = self.index as f64;
            view.duration = 240.0;
        }
        if matches!(self.workload, Some(Workload::SongIndicators)) {
            view.playing = self.index % 40 < 20;
            view.current_id = self.indicator_song;
            input.reduced_motion = 0;
        }
    }
    pub(crate) fn begin(&self) -> Result<Build, String> {
        let interval = self
            .trace_frames
            .then(|| lw_apple_sys::timing::FrameInterval::begin(self.index as u64 + 1));
        let cpu_started = if self.workload.is_some() {
            lw_apple_sys::thread_cpu_time_ns().map_err(|error| error.to_string())?
        } else {
            0
        };
        let allocations_before = self.allocations();
        Ok(Build {
            interval,
            cpu_started,
            allocations_before,
            started: std::time::Instant::now(),
        })
    }
    pub(crate) fn check_intent(
        &mut self,
        intent: &Option<lw_ui::Intent>,
        settings: &lw_core::preferences::Settings,
    ) -> Result<(), String> {
        let workload = self.workload;
        let expected_settings_intent = &self.expected_settings_intent;
        let frames = self.index;
        if matches!(workload, Some(Workload::Settings)) {
            if intent != expected_settings_intent {
                return Err(format!(
                    "Settings input at frame {frames}: expected {expected_settings_intent:?}, got {intent:?}"
                ));
            }
            self.settings_changes += usize::from(matches!(intent, Some(lw_ui::Intent::Setting(_))));
            self.settings_themes |= 1
                << (usize::from(settings.appearance == lw_core::preferences::Appearance::Dark) * 4
                    + settings.signal as usize);
        }
        if matches!(workload, Some(Workload::Drop)) && intent.is_some() {
            return Err("External drag hover activated an application action".into());
        }
        Ok(())
    }
    pub(crate) fn finish_build(
        &mut self,
        build: Build,
        requests: &pxui::images::Request,
    ) -> Result<(), String> {
        let wall = build.started.elapsed().as_micros();
        self.allocations_before = build.allocations_before;
        self.allocations_after = self.allocations();
        let cpu = if self.workload.is_some() {
            (lw_apple_sys::thread_cpu_time_ns().map_err(|error| error.to_string())?
                - build.cpu_started)
                / 1000
        } else {
            0
        };
        if let Some(interval) = build.interval {
            interval.finish(wall as u64, cpu);
        }
        self.timing = super::report::FrameTiming {
            wall,
            cpu,
            phases: [0; 7],
        };
        self.requests = requests.keys().len();
        if self.trace_frames && self.workload.is_some() {
            for (out, key) in self.ids.iter_mut().zip(requests.keys()) {
                *out = key.id;
            }
        }
        Ok(())
    }
    pub(crate) fn check_capture(
        &mut self,
        ui: &Ui,
        intent: &Option<lw_ui::Intent>,
    ) -> Result<(), String> {
        let workload = self.workload;
        let frames = self.index;
        if matches!(workload, Some(Workload::Waveform)) && self.index >= 2 {
            let phase = (self.index - 2) % 20;
            if ui.captured() != (phase < 9) {
                return Err("Waveform qualification lost held pointer capture".into());
            }
            match (phase, intent) {
                (9, Some(lw_ui::Intent::SeekFraction(value)))
                    if value.is_finite() && (0.0..=1.0).contains(value) =>
                {
                    self.waveform_seeks += 1
                }
                (9, _) => return Err("Waveform release did not commit a seek".into()),
                (_, None) => {}
                _ => {
                    return Err(
                        "Waveform preview or scrolling produced an unexpected action".into(),
                    );
                }
            }
        }
        if matches!(workload, Some(Workload::QueueDrag)) && self.index >= 2 {
            let carrying = ui.carrying_ref();
            let expected = (1..20).contains(&((self.index - 2) % 40));
            if expected != carrying.is_some()
                || (expected
                    && (!ui.captured()
                        || carrying.is_none_or(|p| {
                            p.kind != "localwave.queue-occurrence:2" || p.value != "Track 00002"
                        })))
            {
                return Err(format!(
                    "Queue carry did not survive the cross-page scroll or cancel correctly: frame={frames} phase={} expected={expected} captured={} kind={:?}",
                    (self.index - 2) % 40,
                    ui.captured(),
                    carrying.map(|p| p.kind)
                ));
            }
            if intent.is_some() {
                return Err("Cancelled queue carry produced an application action".into());
            }
        }
        Ok(())
    }
    pub(crate) fn allocations(&self) -> u64 {
        if self.trace_frames {
            mem_sys::local_alloc_count()
        } else {
            0
        }
    }
    pub(crate) fn gpu_trace(&mut self) -> Option<&mut [u64; 4]> {
        self.trace_frames.then_some(&mut self.gpu)
    }
}

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

impl Perf {
    pub(crate) fn inject(&mut self, index: usize, trace_frames: bool) -> Frame {
        mem_sys::local_reset();
        Frame {
            workload: Some(self.workload),
            indicator_song: self.indicator_song,
            ..Frame::new(index, trace_frames)
        }
    }

    pub(crate) fn record(
        &mut self,
        frame: Frame,
        ui: &Ui,
        artwork: &lw_bin::library::ArtworkWorker,
        hovered: bool,
        graphics_slot: &mut Option<Graphics>,
        counts: (usize, usize),
    ) -> Result<Option<Schedule>, String> {
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
            indicator_previous,
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
        let trace_frames = frame.trace_frames;
        let artwork_updated = frame.artwork_updated;
        let allocations_before = frame.allocations_before;
        let allocations_after = frame.allocations_after;
        let gpu_trace = frame.gpu;
        let output_allocations = frame.output_allocations;
        let build_us = frame.timing.wall;
        let cpu_us = frame.timing.cpu;
        *waveform_seeks += frame.waveform_seeks;
        *settings_changes += frame.settings_changes;
        *settings_themes |= frame.settings_themes;
        if trace_frames && workload.is_some() {
            let ids = frame.ids;
            (*frame_trace)[(*frames) - 1] = (
                if artwork_updated {
                    artwork.ready().map_or(0, |ready| ready.key.id)
                } else {
                    0
                },
                ids,
                allocations_before,
                allocations_after - allocations_before,
                mem_sys::local_alloc_count() - allocations_after,
                gpu_trace,
                output_allocations,
            );
        }
        if matches!(workload, Some(Workload::Artwork)) && (*frames) > WARMUP_FRAMES {
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
            (*min_artwork_quads) = (*min_artwork_quads).min(image_quads);
            (*warm_artwork_requests) = (*warm_artwork_requests).max(frame.requests);
        }
        if matches!(workload, Some(Workload::SongIndicators)) && (*frames) > WARMUP_FRAMES {
            let visible = ui.a11y_nodes().iter().any(|node| {
                node.is_role(pxui::ui::Role::Image)
                    && matches!(ui.a11y_name_of(node), "Playing" | "Paused")
                    && node.states & pxui::a11y::state::VISIBLE != 0
                    && node.rect.y >= 37.3
                    && node.rect.y + node.rect.h <= 800.0
            });
            if visible {
                let accent = ui.theme().tokens.accent;
                let mut heights = [0u32; 3];
                let mut found = 0;
                for quad in ui
                    .layers()
                    .flat_map(|layer| layer.instances.iter())
                    .filter(|quad| {
                        quad.kind == pxui::draw::kind::RECT
                            && quad.color == [accent.r, accent.g, accent.b, accent.a]
                            && (quad.rect[0] - 264.0).abs() < 9.0
                            && (quad.rect[2] - quad.rect[0] - 2.0).abs() < 0.001
                            && (4.39..11.01).contains(&(quad.rect[3] - quad.rect[1]))
                    })
                {
                    if found >= 3 {
                        return Err("Song indicator workload drew extra bars".into());
                    }
                    heights[found] = (quad.rect[3] - quad.rect[1]).to_bits();
                    found += 1;
                }
                if found != 3 {
                    return Err("Song indicator workload lost its three drawn bars".into());
                }
                if ((*frames) - 1) % 40 < 20 {
                    (*indicator_playing) += 1;
                    (*indicator_changes) += usize::from(
                        (*indicator_previous).is_some_and(|previous| previous != heights),
                    );
                    (*indicator_previous) = Some(heights);
                } else {
                    (*indicator_paused) += 1;
                    if heights
                        .iter()
                        .any(|height| (f32::from_bits(*height) - 4.4).abs() > 0.001)
                    {
                        return Err("Paused song indicator still animates".into());
                    }
                }
            }
        }
        if matches!(workload, Some(Workload::Artists)) && (*frames) > WARMUP_FRAMES {
            let mut rows = 0;
            for node in ui
                .a11y_nodes()
                .iter()
                .filter(|node| node.is_role(pxui::ui::Role::Button) && node.rect.x >= 232.0)
            {
                if let Some(index) = ui
                    .a11y_name_of(node)
                    .strip_prefix("Artist ")
                    .and_then(|label| label.parse::<usize>().ok())
                {
                    if rows == 0 {
                        (*first_artist_min) = (*first_artist_min).min(index);
                        (*first_artist_max) = (*first_artist_max).max(index);
                    }
                    rows += 1;
                }
            }
            (*min_artist_rows) = (*min_artist_rows).min(rows);
            (*max_artist_rows) = (*max_artist_rows).max(rows);
        }
        if matches!(workload, Some(Workload::Files)) && (*frames) > WARMUP_FRAMES {
            let rows = ui
                .layers()
                .flat_map(|layer| layer.instances.iter())
                .filter(|instance| {
                    instance.kind == pxui::draw::kind::RECT
                        && (instance.rect[2] - instance.rect[0] - 72.0).abs() < 0.01
                        && (instance.rect[3] - instance.rect[1] - 36.0).abs() < 0.01
                })
                .count();
            (*min_files_rows) = (*min_files_rows).min(rows);
            (*max_files_rows) = (*max_files_rows).max(rows);
        }
        if matches!(workload, Some(Workload::Drop)) {
            let overlays = ui
                .layers()
                .flat_map(|layer| layer.instances.iter())
                .filter(|instance| {
                    instance.kind == pxui::draw::kind::RECT
                        && instance.color[3] == 240
                        && (instance.rect[0] - 12.0).abs() < 0.01
                        && (instance.rect[1] - 12.0).abs() < 0.01
                        && (instance.rect[2] - 1428.0).abs() < 0.01
                        && (instance.rect[3] - 860.0).abs() < 0.01
                })
                .count();
            if overlays != usize::from(hovered) {
                return Err("External drag workload lost its actual overlay quad".into());
            }
            if (*frames) > WARMUP_FRAMES {
                (*drop_overlay_frames) += usize::from(hovered);
            }
        }
        if workload.is_some() && (*frames) > WARMUP_FRAMES {
            let bars = ui
                .layers()
                .flat_map(|layer| layer.instances.iter())
                .filter(|instance| {
                    instance.kind == pxui::draw::kind::RECT && instance.color[3] == 107
                })
                .count();
            (*min_waveform_bars) = (*min_waveform_bars).min(bars);
            (*warm_allocations) = (*warm_allocations).max(mem_sys::local_alloc_count());
            (*warm_build_us) = (*warm_build_us).max(build_us);
            (*timings)[(*frames) - WARMUP_FRAMES - 1] = super::report::FrameTiming {
                wall: build_us,
                cpu: cpu_us,
                phases: if trace_frames {
                    ui.phase_times().map(|time| time.as_micros() as u64)
                } else {
                    [0; 7]
                },
            };
        }
        if counts.0 >= WARMUP_FRAMES + MEASURED_FRAMES {
            let artwork_bytes = graphics_slot
                .as_ref()
                .ok_or("App graphics have already closed")?
                .resident_image_bytes();
            if let Some(graphics) = graphics_slot.take() {
                graphics.close()?;
            }
            self.report(frame.trace_frames, counts, artwork_bytes)?;
            return Ok(Some(Schedule::Exit));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracted_frame_phases_preserve_native_values_and_allocate_nothing_when_warm() {
        mem_sys::set_peer(pxui::library_counter);
        let mut view = lw_ui::PlayerView {
            revision: 77,
            position: 12.5,
            duration: 99.0,
            current_id: Some(7),
            ..Default::default()
        };
        let mut input = FrameIn {
            reduced_motion: 1,
            ..Default::default()
        };
        let normal = Frame::new(0, false);
        normal.view(&mut view, &mut input);
        assert_eq!(
            (view.revision, view.position, view.duration),
            (77, 12.5, 99.0)
        );
        assert_eq!(
            (view.current_id, view.playing, input.reduced_motion),
            (Some(7), false, 1)
        );
        normal.require_waveform(false).unwrap();
        normal
            .require_extent(pxui_sdl::Extent {
                logical: (390, 844),
                drawable: (1170, 2532),
            })
            .unwrap();

        let mut perf = Perf::new(Workload::Drop, None);
        let mut events = Vec::with_capacity(4);
        let mut drop = crate::events::FileDrop::default();
        let mut overlays = 0;
        for index in 0..WARMUP_FRAMES + MEASURED_FRAMES {
            let frame = perf.inject(index, false);
            events.clear();
            frame.external_input(&mut events);
            for event in &mut events {
                assert!(drop.event(event, None).is_none());
            }
            frame.view(&mut view, &mut input);
            assert_eq!(view.revision, index as u64);
            assert_eq!((view.position, view.duration), (index as f64, 240.0));
            assert_eq!((view.current_id, input.reduced_motion), (Some(7), 1));
            if index >= WARMUP_FRAMES {
                overlays += usize::from(drop.hovered());
            }
            assert_eq!(mem_sys::local_alloc_count(), 0);
        }
        assert_eq!(overlays, 50);
        assert!(perf.inject(0, false).require_waveform(false).is_err());
        assert!(
            perf.inject(0, false)
                .require_extent(pxui_sdl::Extent {
                    logical: (390, 844),
                    drawable: (1170, 2532)
                })
                .is_err()
        );

        let mut perf = Perf::new(Workload::SongIndicators, Some(99));
        for index in [0, 19, 20, 39, 40] {
            let frame = perf.inject(index, false);
            input.reduced_motion = 1;
            frame.view(&mut view, &mut input);
            assert_eq!(
                (view.current_id, view.playing, input.reduced_motion),
                (Some(99), index % 40 < 20, 0)
            );
            assert_eq!(mem_sys::local_alloc_count(), 0);
        }
    }
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
