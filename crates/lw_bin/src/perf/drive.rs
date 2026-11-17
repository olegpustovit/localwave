//! Stack frame state preserves input, build, output and draw measurement boundaries.
use super::{MEASURED_FRAMES, Perf, WARMUP_FRAMES, Workload};
use crate::gpu::Graphics;
use input::Event as HostEvent;
use pxui::{FrameIn, Ui};
use pxui_sdl::app::Schedule;

pub(crate) struct Frame {
    workload: Option<Workload>,
    index: usize,
    started: Option<(std::time::Instant, u64)>,
    trace_frames: bool,
    indicator_song: Option<i64>,
    expected_settings_intent: Option<lw_ui::Intent>,
    settings_changes: usize,
    settings_themes: u8,
    waveform_seeks: i32,
    timing: super::report::FrameTiming,
    layouts: u64,
    warmup: Option<super::Warmup>,
    first_row: Option<usize>,
    allocations_before: u64,
    allocations_after: u64,
    gpu: [u64; 4],
    pub(crate) output_allocations: u64,
    ids: [usize; 8],
    requests: usize,
    pub(crate) artwork_updated: bool,
    carry_frames: usize,
    from: usize,
}

pub(crate) struct Build {
    interval: Option<mem_sys::profile::FrameInterval>,
    cpu_started: u64,
    layouts: u64,
    allocations_before: u64,
    started: std::time::Instant,
}
impl Frame {
    pub(crate) fn new(index: usize, trace_frames: bool) -> Self {
        Self {
            workload: None,
            index,
            started: None,
            trace_frames,
            indicator_song: None,
            expected_settings_intent: None,
            settings_changes: 0,
            settings_themes: 0,
            waveform_seeks: 0,
            timing: Default::default(),
            layouts: 0,
            warmup: None,
            first_row: None,
            allocations_before: 0,
            allocations_after: 0,
            gpu: [0; 4],
            output_allocations: 0,
            ids: [0; 8],
            requests: 0,
            artwork_updated: false,
            carry_frames: 0,
            from: WARMUP_FRAMES,
        }
    }
    pub(crate) fn begin_frame(&mut self) -> Result<(), String> {
        if self.workload.is_some() {
            self.started = Some((
                std::time::Instant::now(),
                mem_sys::profile::thread_cpu_time_ns().map_err(|error| error.to_string())?,
            ));
        }
        Ok(())
    }
    pub(crate) fn cpu_time(&self) -> Result<u64, String> {
        if self.trace_frames && self.workload.is_some() {
            mem_sys::profile::thread_cpu_time_ns().map_err(|error| error.to_string())
        } else {
            Ok(0)
        }
    }
    pub(crate) fn finish_output(&mut self, started: u64) -> Result<(), String> {
        self.timing.phase_cpu[7] = (self.cpu_time()? - started) / 1000;
        Ok(())
    }
    pub(crate) fn finish_frame(
        &mut self,
        ui: &Ui,
        draw_started: u64,
        acquire: std::time::Duration,
    ) -> Result<(), String> {
        if self.workload.is_some() {
            let (started, cpu) = self
                .started
                .take()
                .ok_or("Performance frame did not include an input pump")?;
            self.timing.frame_wall = started.elapsed().as_micros();
            // Drawable/slot acquisition is display pacing; the rest of the frame is work users feel.
            self.timing.work_wall = self.timing.frame_wall.saturating_sub(acquire.as_micros());
            let ended =
                mem_sys::profile::thread_cpu_time_ns().map_err(|error| error.to_string())?;
            self.timing.frame_cpu = (ended - cpu) / 1000;
            if self.trace_frames {
                self.timing.phase_cpu[..7].copy_from_slice(
                    &ui.phase_cpu_times()
                        .map_err(|code| format!("CPU phase timing unavailable ({code})"))?
                        .map(|time| time.as_micros() as u64),
                );
                self.timing.phase_cpu[8] = (ended - draw_started) / 1000;
            }
        }
        Ok(())
    }
    pub(crate) fn external_input(&self, events: &mut Vec<HostEvent>) {
        if matches!(self.workload, Some(Workload::Drop)) && self.index >= 2 {
            drop_events(self.index - 2, events);
        }
    }
    pub(crate) fn input(
        &mut self,
        ui: &Ui,
        events: &mut Vec<HostEvent>,
    ) -> Result<(), String> {
        let workload = self.workload;
        let mut expected_settings_intent = None;
        if workload.is_some() && self.index >= 2 && !matches!(workload, Some(Workload::Drop | Workload::Clock120)) {
            if let Some(measured) = workload.filter(|workload| workload.measured() || *workload == Workload::Idle600) {
                super::measured::events(measured, self.index - 2, self.from, ui, &mut *events)?;
            } else if matches!(workload, Some(Workload::Settings)) {
                expected_settings_intent = settings_events(self.index - 2, ui, &mut *events)?;
            } else if matches!(workload, Some(Workload::Waveform)) {
                waveform_events(self.index - 2, &mut *events);
            } else if matches!(workload, Some(Workload::AlbumDetail | Workload::ArtistDetail)) {
                events.push(HostEvent::PointerMoved((650.0, 600.0)));
                // Small steps keep the header in view while the table moves. The artist page puts its
                // songs below the Back row, keys and album grid, so it also sweeps down to them and
                // back each cycle, leaving the header visible for the other half.
                let phase = (self.index - 2) % 40;
                let sweep = matches!(workload, Some(Workload::ArtistDetail)) && phase >= 24;
                let step = match (sweep, phase < 32, phase < 12) {
                    (true, true, _) => -3.0,
                    (true, false, _) => 3.0,
                    (_, _, true) => -0.2,
                    _ => 0.2,
                };
                events.push(HostEvent::Scrolled((0.0, step)));
            } else if matches!(workload, Some(Workload::ListDialogs)) {
                list_dialog_events(self.index - 2, ui, &mut *events)?;
            } else if matches!(workload, Some(Workload::MetadataEditor)) {
                events.push(HostEvent::PointerMoved((720.0, 400.0)));
            } else if matches!(workload, Some(Workload::SongDrag)) {
                song_drag_events(self.index - 2, ui, &mut *events)?;
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
                    (if matches!(workload, Some(Workload::LongScroll)) {
                        -3.0
                    } else if (self.index - 2) % 40 < 20 {
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
    /// Typing is driven as the Intents a real text field emits: TextInput events own Strings and
    /// would put host allocations inside the zero-allocation frame.
    pub(crate) fn state_input(&self, state: &mut lw_ui::ShellState) {
        use lw_ui::{Intent, browse::Query};
        if self.index < 2 {
            return;
        }
        let phase = (self.index - 2) % 40;
        if let Some(workload) = self.workload.filter(|workload| workload.measured()) {
            super::measured::state_input(workload, self.index - 2, self.from, state);
        }
        match self.workload {
            Some(Workload::ListDialogs) if (14..30).contains(&phase) => {
                let typed = if phase < 22 { phase - 13 } else { 29 - phase };
                if let Some(name) = Query::new(&NAME_TEXT[..typed]) {
                    state.apply(Intent::EditPlaylistName(name));
                }
            }
            Some(Workload::MetadataEditor) if phase < 22 => {
                let typed = if phase < 11 { phase + 1 } else { 21 - phase };
                if let Some(title) = Query::new(&TITLE_FULL[..TITLE_TEXT.len() + typed]) {
                    state.apply(Intent::MetadataField(0, title));
                }
            }
            _ => {}
        }
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
    pub(crate) fn view(
        &self,
        view: &mut lw_ui::PlayerView<'_>,
        position: &mut f64,
        input: &mut FrameIn,
    ) {
        if self.workload.is_some() {
            *position = self.index as f64;
            view.duration = 240.0;
        }
        match self.workload {
            Some(Workload::Resize) => {
                (input.width, input.height) = super::measured::resize_extent(self.index);
            }
            Some(Workload::Clock120) => input.time = super::measured::clock_120(self.index),
            _ => {}
        }
        if matches!(self.workload, Some(Workload::SongIndicators)) {
            view.revision = (self.index / 20) as u64;
            view.playing = self.index % 40 < 20;
            view.current_id = self.indicator_song;
            input.reduced_motion = 0;
        }
    }
    pub(crate) fn begin(&self, ui: &Ui) -> Result<Build, String> {
        let layouts = if self.workload.is_some() {
            ui.text_layout_count()
        } else {
            0
        };
        let interval = self
            .trace_frames
            .then(|| mem_sys::profile::FrameInterval::begin(self.index as u64 + 1));
        let cpu_started = if self.workload.is_some() {
            mem_sys::profile::thread_cpu_time_ns().map_err(|error| error.to_string())?
        } else {
            0
        };
        let allocations_before = self.allocations();
        Ok(Build {
            interval,
            layouts,
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
        use lw_ui::Intent::*;
        let allowed = match workload {
            Some(Workload::ListDialogs | Workload::FirstMenuOpen) => matches!(
                intent,
                None | Some(ListMenu(..) | CloseListMenu | NewList(None) | CancelPlaylistName)
            ),
            Some(
                Workload::AlbumDetail
                | Workload::ArtistDetail
                | Workload::MetadataEditor
                | Workload::TextTyping
                | Workload::SongDrag,
            ) => intent.is_none(),
            _ => true,
        };
        if !allowed {
            return Err(format!(
                "{} produced an unexpected or mutating action: {intent:?}",
                self.workload.map_or("", Workload::name)
            ));
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
        ui: &Ui,
    ) -> Result<(), String> {
        let wall = build.started.elapsed().as_micros();
        self.allocations_before = build.allocations_before;
        self.allocations_after = self.allocations();
        let cpu = if self.workload.is_some() {
            (mem_sys::profile::thread_cpu_time_ns().map_err(|error| error.to_string())?
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
            ..Default::default()
        };
        self.layouts = if self.workload.is_some() {
            ui.text_layout_count() - build.layouts
        } else {
            0
        };
        self.requests = requests.keys().len();
        if self.trace_frames && self.workload.is_some() {
            for (out, key) in self.ids.iter_mut().zip(requests.keys()) {
                *out = key.id;
            }
        }
        Ok(())
    }
    pub(crate) fn audit(
        &mut self,
        ui: &Ui,
        artwork: &lw_bin::library::ArtworkWorker,
        player: &lw_bin::controller::Controller,
    ) {
        if self.workload.is_some() && self.index + 1 == WARMUP_FRAMES {
            self.warmup = Some(super::Warmup {
                artwork: artwork.has_pending_work(),
                catalog: player.catalog_pending(),
                layouts: self.layouts,
                requests: self.requests,
            });
        }
        if self.workload == Some(Workload::LongScroll) && self.index >= WARMUP_FRAMES {
            self.first_row = ui
                .a11y_nodes()
                .iter()
                .filter(|node| {
                    node.is_role(pxui::ui::Role::ListItem)
                        && node.states & pxui::a11y::state::VISIBLE != 0
                })
                .filter_map(|node| {
                    ui.a11y_name_of(node)
                        .strip_prefix("Track ")
                        .and_then(|text| text.parse::<i64>().ok())
                })
                .filter_map(|id| player.catalog().row_index(id))
                .min();
        }
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
        if matches!(workload, Some(Workload::QueueDrag | Workload::SongDrag)) && self.index >= 2 {
            let song = workload == Some(Workload::SongDrag);
            let carrying = ui.carrying_ref();
            self.carry_frames = usize::from(carrying.is_some());
            let expected = (1..20).contains(&((self.index - 2) % 40));
            if expected != carrying.is_some()
                || (expected
                    && (!ui.captured()
                        || carrying.is_none_or(|p| {
                            if song {
                                !p.kind.starts_with("localwave.song:") || !p.value.starts_with("Track ")
                            } else {
                                p.kind != "localwave.queue-occurrence:2" || p.value != "Track 00002"
                            }
                        })))
            {
                return Err(format!(
                    "Carry did not survive hover/scroll or cancel correctly: frame={frames} phase={} expected={expected} captured={} kind={:?}",
                    (self.index - 2) % 40,
                    ui.captured(),
                    carrying.map(|p| p.kind)
                ));
            }
            if intent.is_some() {
                return Err("Cancelled carry produced an application action".into());
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

pub(crate) const TITLE_TEXT: &str = "Track 00001";
const TITLE_FULL: &str = "Track 00001-remastered";
const NAME_TEXT: &str = "Night drive mix";

fn center(
    ui: &Ui,
    role: pxui::ui::Role,
    wanted: impl Fn(&str) -> bool,
    what: &str,
) -> Result<(f64, f64), String> {
    ui.a11y_nodes()
        .iter()
        .find(|node| {
            node.is_role(role)
                && node.states & pxui::a11y::state::VISIBLE != 0
                && node.rect.y >= 60.0
                && node.rect.y + node.rect.h <= 800.0
                && wanted(ui.a11y_name_of(node))
        })
        .map(|node| {
            (
                f64::from(node.rect.x + node.rect.w * 0.5),
                f64::from(node.rect.y + node.rect.h * 0.5),
            )
        })
        .ok_or_else(|| format!("Performance fixture has no visible {what}"))
}

/// Right-click a sidebar list for its menu, dismiss it, open New list and leave it with Esc;
/// typing happens through `state_input`. Nothing is ever created, renamed or deleted.
pub(super) fn list_dialog_events(frame: usize, ui: &Ui, events: &mut Vec<input::Event>) -> Result<(), String> {
    use input::{Event, Key, MouseButton};
    use pxui::ui::Role;
    let phase = frame % 40;
    if phase <= 8 {
        let list = center(
            ui,
            Role::Button,
            |name| name.starts_with(super::setup::LIST_NAME),
            "sidebar list",
        )?;
        match phase {
            0 => {
                events.push(Event::PointerMoved(list));
                events.push(Event::PointerPressed(MouseButton::RightButton, list));
            }
            1 => events.push(Event::PointerReleased(MouseButton::RightButton, list)),
            _ => events.push(Event::PointerMoved((list.0 + 20.0, list.1 + 18.0 + (phase & 1) as f64))),
        }
    } else if matches!(phase, 11 | 12) {
        let new_list = center(ui, Role::Button, |name| name == "New list", "New list button")?;
        events.push(Event::PointerMoved(new_list));
        events.push(if phase == 11 {
            Event::PointerPressed(MouseButton::LeftButton, new_list)
        } else {
            Event::PointerReleased(MouseButton::LeftButton, new_list)
        });
    } else if matches!(phase, 9 | 30) {
        events.push(Event::KeyPressed(Key::Escape));
    } else if matches!(phase, 10 | 31) {
        events.push(Event::KeyReleased(Key::Escape));
    }
    Ok(())
}

/// Carry the first visible song across the table, over a sidebar list and Up next, then cancel
/// with Esc before release (no drop, no intents); the rest of the cycle scrolls the table.
fn song_drag_events(frame: usize, ui: &Ui, events: &mut Vec<input::Event>) -> Result<(), String> {
    use input::{Event, Key, MouseButton};
    use pxui::ui::Role;
    let phase = frame % 40;
    let jitter = (phase % 3) as f64;
    match phase {
        0 | 1 => {
            let row = center(ui, Role::ListItem, |name| name.starts_with("Track "), "song row")?;
            if phase == 0 {
                events.push(Event::PointerMoved(row));
                events.push(Event::PointerPressed(MouseButton::LeftButton, row));
            } else {
                events.push(Event::PointerMoved((row.0, row.1 + 8.0)));
            }
        }
        2..=5 => events.push(Event::PointerMoved((
            650.0 + phase as f64 * 10.0,
            200.0 + phase as f64 * 60.0,
        ))),
        6..=11 => {
            let list = center(
                ui,
                Role::Button,
                |name| name.starts_with(super::setup::LIST_NAME),
                "sidebar list",
            )?;
            events.push(Event::PointerMoved((list.0 + jitter, list.1)));
        }
        12..=19 => {
            let queue = center(ui, Role::List, |name| name == "Up next queue", "Up next list")?;
            // Sweep down across the list as a user aims a drop, after resting on its upper rows
            // (the highest instance count of the cycle).
            let y = if phase < 15 { queue.1 - 40.0 } else { queue.1 + (phase - 15) as f64 * 12.0 };
            events.push(Event::PointerMoved((queue.0 + jitter, y)));
        }
        20 => events.push(Event::KeyPressed(Key::Escape)),
        21 => {
            events.push(Event::KeyReleased(Key::Escape));
            events.push(Event::PointerReleased(MouseButton::LeftButton, (1300.0, 400.0)));
        }
        _ => {
            events.push(Event::PointerMoved((650.0, 400.0)));
            events.push(Event::Scrolled((0.0, if (frame / 40) % 2 == 0 { -1.0 } else { 1.0 })));
        }
    }
    Ok(())
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
/// Targets are found in the a11y tree each frame so the workload follows the layout.
pub fn settings_events(
    frame: usize,
    ui: &Ui,
    events: &mut Vec<input::Event>,
) -> Result<Option<lw_ui::Intent>, String> {
    use input::{Event, MouseButton};
    use lw_core::preferences::{Appearance, Change, Signal};
    use lw_ui::{Intent, SettingsMenu};
    use pxui::ui::Role;
    let phase = frame % 40;
    let dark = (frame / 40).is_multiple_of(2);
    let (role, name, button, action): (_, &str, _, _) = match phase & !1 {
        0 | 8 | 14 | 22 | 30 => (Role::Group, "Appearance", MouseButton::RightButton, Intent::SettingsMenu(SettingsMenu::Appearance, (0.0, 0.0))),
        4 | 18 => (Role::Group, "Offline space", MouseButton::RightButton, Intent::SettingsMenu(SettingsMenu::Cache, (0.0, 0.0))),
        2 => (Role::Toggle, "Use red signal", MouseButton::LeftButton, Intent::Setting(Change::Signal(Signal::Red))),
        6 => (Role::Toggle, "Use 32 GB", MouseButton::LeftButton, Intent::Setting(Change::CacheGB(32))),
        10 => (Role::Toggle, "Use green signal", MouseButton::LeftButton, Intent::Setting(Change::Signal(Signal::Green))),
        12 if dark => (Role::Toggle, "Use dark appearance", MouseButton::LeftButton, Intent::Setting(Change::Appearance(Appearance::Dark))),
        12 => (Role::Toggle, "Use light appearance", MouseButton::LeftButton, Intent::Setting(Change::Appearance(Appearance::Light))),
        16 => (Role::Toggle, "Use monochrome signal", MouseButton::LeftButton, Intent::Setting(Change::Signal(Signal::Mono))),
        20 => (Role::Toggle, "Use 2 GB", MouseButton::LeftButton, Intent::Setting(Change::CacheGB(2))),
        24 => (Role::Toggle, "Use blue signal", MouseButton::LeftButton, Intent::Setting(Change::Signal(Signal::Blue))),
        26 => (Role::Toggle, "Download ahead", MouseButton::LeftButton, Intent::Setting(Change::ToggleDownloadAhead)),
        28 => (Role::Toggle, "Simulate no network", MouseButton::LeftButton, Intent::Setting(Change::ToggleNoNetwork)),
        32 => (Role::Toggle, "Reduce motion", MouseButton::LeftButton, Intent::Setting(Change::ToggleReduceMotion)),
        _ => return Ok(None),
    };
    let at = center(ui, role, |n| n == name, name)?;
    let action = match action {
        Intent::SettingsMenu(menu, _) => Intent::SettingsMenu(menu, (at.0 as f32, at.1 as f32)),
        other => other,
    };
    let pressed = phase.is_multiple_of(2);
    events.push(Event::PointerMoved(at));
    events.push(if pressed {
        Event::PointerPressed(button, at)
    } else {
        Event::PointerReleased(button, at)
    });
    Ok(((button == MouseButton::RightButton) == pressed).then_some(action))
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
            from: self.measured_from(),
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
        let from = self.measured_from();
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
        if frame.warmup.is_some() {
            self.warmup = frame.warmup;
        }
        if *workload == Workload::LongScroll && counts.0 > from {
            let first = frame
                .first_row
                .ok_or("LongScroll has no visible song rows")?
                + 1;
            if self.long_first == usize::MAX {
                self.long_first = first;
            } else if first < self.long_last {
                return Err("LongScroll reversed its row order".into());
            }
            self.long_advances += usize::from(first > self.long_last);
            self.long_last = first;
        }
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
        let frame_wall = frame.timing.frame_wall;
        let frame_cpu = frame.timing.frame_cpu;
        *waveform_seeks += frame.waveform_seeks;
        *settings_changes += frame.settings_changes;
        *settings_themes |= frame.settings_themes;
        if trace_frames && workload.is_some() {
            let ids = frame.ids;
            if let Some(slot) = (*frame_trace).get_mut((*frames) - 1) {
            *slot = (
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
        }
        if (*frames) > from {
            let visible = |name: &str, below: f32| {
                ui.a11y_nodes().iter().any(|node| {
                    node.states & pxui::a11y::state::VISIBLE != 0
                        && node.rect.y < below
                        && ui.a11y_name_of(node) == name
                })
            };
            match workload {
                Some(Workload::AlbumDetail | Workload::ArtistDetail) => {
                    let marker = if workload == Some(Workload::AlbumDetail) {
                        "Album artwork"
                    } else {
                        "Shuffle"
                    };
                    self.marker_frames += usize::from(visible(marker, 400.0));
                    let first = ui
                        .a11y_nodes()
                        .iter()
                        .filter(|node| {
                            node.is_role(pxui::ui::Role::ListItem)
                                && node.states & pxui::a11y::state::VISIBLE != 0
                        })
                        .find(|node| ui.a11y_name_of(node).starts_with("Track "))
                        .map(|node| node.rect.y as usize);
                    if let Some(first) = first {
                        self.detail_first_min = self.detail_first_min.min(first);
                        self.detail_first_max = self.detail_first_max.max(first);
                    }
                }
                Some(Workload::ListDialogs) => {
                    self.menu_frames += usize::from(visible("List actions", f32::MAX));
                    self.marker_frames += usize::from(visible("List name", f32::MAX));
                }
                Some(Workload::MetadataEditor) => {
                    self.marker_frames += usize::from(visible("Release year", f32::MAX));
                }
                Some(Workload::SongDrag) => {
                    self.marker_frames += frame.carry_frames;
                }
                _ => {}
            }
        }
        if matches!(workload, Some(Workload::Artwork | Workload::AlbumDetail))
            && (*frames) > from
        {
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
        if matches!(workload, Some(Workload::SongIndicators)) && (*frames) > from {
            let indicator = ui.a11y_nodes().iter().find(|node| {
                node.is_role(pxui::ui::Role::Image)
                    && matches!(ui.a11y_name_of(node), "Playing" | "Paused")
                    && node.states & pxui::a11y::state::VISIBLE != 0
                    && node.rect.y >= 37.3
                    && node.rect.y + node.rect.h <= 800.0
            });
            if let Some(indicator) = indicator {
                let accent = ui.theme().tokens.accent;
                let drawn_bars = ui
                    .layers()
                    .flat_map(|layer| layer.instances.iter())
                    .filter(|quad| {
                        quad.kind == pxui::draw::kind::RECT
                            && quad.color == [accent.r, accent.g, accent.b, accent.a]
                            && (quad.rect[0] - 264.0).abs() < 9.0
                            && (quad.rect[2] - quad.rect[0] - 2.0).abs() < 0.001
                            && (quad.rect[3] - quad.rect[1] - 11.0).abs() < 0.001
                    })
                    .count();
                if drawn_bars != 3 {
                    return Err("Song indicator workload lost its three drawn bars".into());
                }
                let root = ui
                    .box_of_a11y(indicator.id)
                    .ok_or("Song indicator identity retired during its frame")?;
                let mut heights = [0u32; 3];
                for (index, key) in ["bar-0", "bar-1", "bar-2"].into_iter().enumerate() {
                    let position = pxui::Ui::key_of(root.box_key, key);
                    let visual = pxui::Ui::key_of(position, "bar");
                    let fill = ui
                        .box_of_key(pxui::Ui::key_of(visual, "fill"))
                        .ok_or("Song indicator workload lost a retained bar")?;
                    // Retained scale lives in the view table; source instances stay full height.
                    let rect = ui.rect(&fill);
                    if !(4.39..11.01).contains(&rect.h) || (rect.w - 2.0).abs() > 0.001 {
                        return Err(
                            "Song indicator workload has invalid transformed bar geometry".into(),
                        );
                    }
                    heights[index] = rect.h.to_bits();
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
        if matches!(workload, Some(Workload::Artists)) && (*frames) > from {
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
        if matches!(workload, Some(Workload::Files)) && (*frames) > from {
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
            if (*frames) > from {
                (*drop_overlay_frames) += usize::from(hovered);
            }
        }
        if workload.is_some() && (*frames) > from {
            let bars = ui
                .layers()
                .flat_map(|layer| layer.instances.iter())
                .filter(|instance| {
                    instance.kind == pxui::draw::kind::RECT && instance.color[3] == 107
                })
                .count();
            (*min_waveform_bars) = (*min_waveform_bars).min(bars);
            (*warm_allocations) = (*warm_allocations).max(mem_sys::local_alloc_count());
            self.warm_layouts = self.warm_layouts.max(frame.layouts);
            (*warm_build_us) = (*warm_build_us).max(build_us);
            (*timings)[(*frames) - from - 1] = super::report::FrameTiming {
                wall: build_us,
                cpu: cpu_us,
                frame_wall,
                frame_cpu,
                work_wall: frame.timing.work_wall,
                phase_cpu: frame.timing.phase_cpu,
                phases: if trace_frames {
                    ui.phase_times().map(|time| time.as_micros() as u64)
                } else {
                    [0; 7]
                },
            };
        }
        if counts.0 >= from + MEASURED_FRAMES {
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
    fn long_scroll_keeps_a_new_downward_three_row_step_without_warm_allocations() {
        let ui = Ui::new().unwrap();
        let mut perf = Perf::new(Workload::LongScroll, None);
        let mut events = Vec::with_capacity(4);
        for index in 2..WARMUP_FRAMES + MEASURED_FRAMES {
            events.clear();
            mem_sys::local_reset();
            let mut frame = perf.inject(index, false);
            frame.input(&ui, &mut events).unwrap();
            assert_eq!(
                events,
                [
                    HostEvent::PointerMoved((650.0, 400.0)),
                    HostEvent::Scrolled((0.0, -3.0))
                ]
            );
            assert_eq!(mem_sys::local_alloc_count(), 0);
        }
    }
    #[test]
    fn extracted_frame_phases_preserve_native_values_and_allocate_nothing_when_warm() {
        mem_sys::set_peer(pxui::library_counter);
        let mut view = lw_ui::PlayerView {
            revision: 77,
            duration: 99.0,
            current_id: Some(7),
            ..Default::default()
        };
        let mut position = 12.5;
        let mut input = FrameIn {
            reduced_motion: 1,
            ..Default::default()
        };
        let normal = Frame::new(0, false);
        normal.view(&mut view, &mut position, &mut input);
        assert_eq!((view.revision, position, view.duration), (77, 12.5, 99.0));
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
            frame.view(&mut view, &mut position, &mut input);
            assert_eq!(view.revision, 77);
            assert_eq!((position, view.duration), (index as f64, 240.0));
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
            frame.view(&mut view, &mut position, &mut input);
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
    fn typing_workloads_edit_bounded_text_without_allocating() {
        let mut state = lw_ui::ShellState::default();
        state.apply(lw_ui::Intent::NewList(None));
        let mut perf = Perf::new(Workload::ListDialogs, None);
        for index in 2..WARMUP_FRAMES + MEASURED_FRAMES {
            let frame = perf.inject(index, false);
            frame.state_input(&mut state);
            assert_eq!(mem_sys::local_alloc_count(), 0);
        }
        let mut perf = Perf::new(Workload::MetadataEditor, None);
        for index in 2..WARMUP_FRAMES + MEASURED_FRAMES {
            let frame = perf.inject(index, false);
            frame.state_input(&mut state);
            assert_eq!(mem_sys::local_alloc_count(), 0);
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
