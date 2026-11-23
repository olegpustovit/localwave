//! Build/draw only on Signal::Frame. Resumed/LowMemory request a later frame; Paused/Control
//! service controller/files status without SDL input consumption or image uploads. Background
//! worker wakes use the framework run-loop source, never app polling or a second display link. A
//! source consumed by a nested native run loop must be re-posted after the control callback. Drop
//! application resources before Fixture and close GPU resources on Finish before UI/window teardown.
//! Set interactive QoS only on the initial UI thread, after spawning workers.
//!
//! One `pxui::TextWorker` (pxui's prepare-queue thread) is created at startup beside the other
//! workers, before the UI's QoS is raised, and attached to the one `Ui` between frames. The app
//! owns it for the process: it drops after the `Ui` (detaching does not wait) and joins after at
//! most one shaping call. Between frames the UI thread hands it the song table's next rows
//! (`lw_ui::song_text::Prewarm`); the `Ui` installs deliveries before a frame builds.
//!
//! Main-thread application state. Window/GPU integration stays in pxui_host/pxui_sdl.
use crate::{events, gpu, perf, reference, smoke, wake};
use ::input::Event as HostEvent;
use lw_bin::{
    controller::{Command, Controller},
    library::LibraryWorker,
};
use pxui::{FontData, FrameIn, Platform, Ui, UiDesc};
use pxui_sdl::{
    Window,
    app::{Schedule, Signal},
};
mod actions;
mod frame_reasons;
mod input;

fn graphics_error(ui: &Ui, error: pxui::Error) -> String {
    if matches!(error, pxui::Error::Host(_)) {
        return error.to_string();
    }
    let detail = ui.last_error();
    if detail.is_empty() { error.to_string() } else { format!("{error}: {detail}") }
}

struct Fixture(Option<std::path::PathBuf>);
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

pub(crate) struct App {
    // Native GPU resources and themes retire before the UI.
    graphics_slot: Option<gpu::Graphics>,
    themes: lw_bin::appearance::Themes,
    ui: Ui,
    // Held for its lifetime, after `ui`: the Ui detaches first, then the worker's thread joins.
    _text_worker: pxui::TextWorker,
    #[cfg(target_os = "macos")]
    chrome: pxui_host::Chrome,
    #[cfg(target_os = "macos")]
    accessibility: pxui_host::Accessibility,
    environment: pxui_sdl::Preferences,
    preferences: pxui_sdl::PreferencesSnapshot,
    pub(super) player: Controller,
    artwork: lw_bin::library::ArtworkWorker,
    text: lw_ui::song_text::Prewarm,
    pub(super) files: lw_bin::files::FilesWorker,
    top_inset: f64,
    perf: Option<perf::Perf>,
    energy: Option<perf::energy::Run>,
    pub(super) smoke: bool,
    trace_frames: bool,
    frame_reasons: Option<frame_reasons::Report>,
    extent: pxui_sdl::Extent,
    events: Vec<HostEvent>,
    encoded: pxui_host::Events,
    platform: pxui_host::Platform,
    first: bool,
    repaint: Option<f64>,
    revision: u64,
    frames: usize,
    draws: usize,
    smoke_check: smoke::Smoke,
    smoke_finished: bool,
    /// Reference-golden mode: (last draw count, time it changed, READY printed).
    reference: Option<(usize, f64, bool)>,
    reference_capture: Option<std::path::PathBuf>,
    /// Reference overlay screen still to open once the page below it has drawn.
    reference_overlay: Option<String>,
    /// The folder golden asked the FilesWorker for its directory: the capture waits until the directory has arrived.
    reference_directory: bool,
    /// Goldens' system appearance: 2 (dark) with LOCALWAVE_REFERENCE_DARK, else 1 (Swift's light canvas).
    reference_appearance: u32,
    /// The bundle carries the demo library (checked once at startup; reference captures show it as Swift's fixture does).
    demo: bool,
    /// Reference position still to seek to once the prepared item reports its duration.
    reference_seek: Option<f64>,
    pub(super) shell_state: lw_ui::ShellState,
    command_error: Option<&'static str>,
    error_revision: u64,
    folder_picker: Option<(i64, events::FolderPurpose)>,
    audio_picker: Option<(i64, i64)>,
    file_drop: events::FileDrop,
    files_catalog_revision: u64,
    files_selection_revision: u64,
    // Workers stop before their isolated files are removed.
    _fixture: Fixture,
}

pub(super) fn open(
    window: &mut Window,
    workload: Option<perf::Workload>,
    energy: Option<perf::energy::Scenario>,
    smoke: bool,
    trace_frames: bool,
    reference: Option<String>,
    reference_capture: Option<std::path::PathBuf>,
) -> Result<Box<App>, String> {
    if cfg!(target_os = "ios") && workload.is_some() {
        return Err("The signed 1440x872 performance matrix requires the macOS host".into());
    }
    if workload.is_some() && !mem_sys::COUNTING {
        return Err("--performance-check needs the counting build (make performance)".into());
    }
    if energy.is_some() && (cfg!(debug_assertions) || mem_sys::COUNTING) {
        return Err("Energy measurement needs the non-counting release bundle (make bundle)".into());
    }
    mem_sys::set_peer(pxui::library_counter);
    #[cfg(target_os = "macos")]
    let chrome = {
        window.set_resizable(true)?;
        let chrome = pxui_host::Chrome::open(window.token()?, (320.0, 480.0))
            .map_err(|error| error.to_string())?;
        // Goldens compare the 1440x872 content area; tools/reference-goldens.sh crops the titlebar inset.
        let extra = if reference.is_some() { chrome.snapshot().map_err(|error| error.to_string())?.top_inset.ceil() as i64 } else { 0 };
        // LOCALWAVE_REFERENCE_MOBILE=1 (goldens only) opens the 390x778 compact canvas.
        let (width, height) = if reference.is_some() && std::env::var_os("LOCALWAVE_REFERENCE_MOBILE").is_some() { (390, 778) } else { (1440, 872) };
        window.resize(width, height + extra)?;
        chrome
    };
    #[cfg(target_os = "macos")]
    let accessibility = pxui_host::Accessibility::open(window).map_err(|error| error.to_string())?;
    #[cfg(target_os = "macos")]
    let top_inset = chrome
        .snapshot()
        .map_err(|error| error.to_string())?
        .top_inset;
    #[cfg(target_os = "ios")]
    let top_inset = 0.0;
    let environment = window.preferences(wake)?;
    let preferences = environment.snapshot();
    let desc = UiDesc::new(Platform::Apple)
        .font_size(if cfg!(target_os = "ios") { 18.0 } else { 15.0 })
        .fonts(&[FontData(lw_ui::FONT), FontData(lw_ui::MONO)]);
    let mut ui = Ui::new(&desc).map_err(|error| format!("UI creation failed ({error})"))?;
    perf::setup::configure_ui(&mut ui, workload);
    ui.text_service().enable_system_fonts()
        .map_err(|code| format!("System fonts failed ({code}): {}", ui.last_error()))?;
    let mut themes = lw_bin::appearance::Themes::new(&mut ui)
        .map_err(|error| format!("Theme setup failed ({error}): {}", ui.last_error()))?;
    themes.apply(
        &mut ui,
        &lw_core::preferences::Settings::default(),
        preferences,
    );
    if trace_frames {
        ui.set_timing(true);
    }
    let graphics = gpu::Graphics::open(window, &ui).map_err(|error| graphics_error(&ui, error))?;
    let temporary = if smoke || workload.is_some() || reference.is_some() || energy.is_some() {
        Some(
            lw_apple_sys::preferences::AppPaths::load()
                .map_err(|error| error.to_string())?
                .temporary,
        )
    } else {
        None
    };
    let fixture = Fixture(temporary.map(|temporary| {
        temporary.join(format!(
            "lw-gui-check-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }));
    let seeded = reference.is_some();
    // the catalog worker measures table text with styles resolved here, on the UI thread
    let table_styles = lw_ui::catalog::TableStyles::resolve(&ui);
    let screen = reference.clone().unwrap_or_default();
    let mut player = if let Some(base) = fixture.0.clone() {
        Controller::open(
            move || {
                if seeded {
                    reference::library(&base, &screen, table_styles)
                } else {
                    perf::setup::library(&base, workload, energy.is_some(), table_styles)
                }
            },
            wake,
        )
    } else {
        Controller::open(move || LibraryWorker::current(table_styles), wake)
    }
    .map_err(|error| error.to_string())?;
    let artwork = lw_bin::library::ArtworkWorker::open(
        fixture.0.as_ref().map(|base| base.join("Documents")),
        wake,
    )
    .map_err(|error| error.to_string())?;
    let files_base = fixture.0.clone();
    let files = lw_bin::files::FilesWorker::open(
        move || {
            if let Some(base) = files_base {
                let mut library = LibraryWorker::open(
                    &base.join("library.sqlite"),
                    lw_apple_sys::preferences::Preferences::open(Some(&format!(
                        "com.olegpustovit.localwave.{}",
                        base.file_name().unwrap().to_string_lossy()
                    )))?,
                    lw_apple_sys::preferences::AppPaths {
                        documents: base.join("Documents"),
                        caches: base.join("Caches"),
                        temporary: base.join("tmp"),
                        test_database: None,
                    },
                    table_styles,
                )?;
                // Reference fixtures store availability; probing their few real audio files would differ from Swift.
                library.trust_stored_availability = seeded;
                if seeded {
                    library.reference_sources = reference::source_display()?;
                }
                Ok(library)
            } else {
                LibraryWorker::current(table_styles)
            }
        },
        wake,
    )
    .map_err(|error| error.to_string())?;
    let text_worker = pxui::TextWorker::new().map_err(|error| error.to_string())?;
    ui.set_text_worker(Some(&text_worker))
        .map_err(|error| error.to_string())?;
    let text = lw_ui::song_text::Prewarm::default();
    // Workers do not inherit the UI's interactive QoS.
    mem_sys::profile::ui_thread_priority().map_err(|error| error.to_string())?;
    let indicator_song =
        perf::setup::wait_ready(window, &mut player, smoke || seeded || energy.is_some(), workload)?;
    let energy = energy
        .map(|scenario| scenario.start(window.time(), &mut player))
        .transpose()?;
    let extent = window.extent();
    let events: Vec<HostEvent> = Vec::with_capacity(256);
    let encoded = pxui_host::Events::default();
    let platform = pxui_host::Platform::default();
    let first = true;
    let repaint = None;
    let revision = u64::MAX;
    let (frames, draws) = (0, 0);
    let perf = workload.map(|workload| perf::Perf::new(workload, indicator_song));
    let smoke_check = smoke::Smoke::new(window.time());
    let smoke_finished = false;
    let reference_state = reference.as_ref().map(|_| (0, window.time(), false));
    let shell_state = lw_ui::ShellState::default();
    let command_error: Option<&'static str> = None;
    let error_revision = 0;
    let folder_picker = None;
    let audio_picker = None;
    let file_drop = events::FileDrop::default();
    let files_catalog_revision = 0;
    let files_selection_revision = 0;
    let graphics_slot = Some(graphics);
    let mut app = Box::new(App {
        graphics_slot,
        themes,
        ui,
        _text_worker: text_worker,
        #[cfg(target_os = "macos")]
        chrome,
        #[cfg(target_os = "macos")]
        accessibility,
        environment,
        preferences,
        player,
        artwork,
        text,
        files,
        top_inset,
        perf,
        energy,
        smoke,
        trace_frames,
        frame_reasons: frame_reasons::Report::from_env(window.time()),
        extent,
        events,
        encoded,
        platform,
        first,
        repaint,
        revision,
        frames,
        draws,
        smoke_check,
        smoke_finished,
        reference: reference_state,
        reference_capture,
        reference_seek: reference.as_ref().map(|_| reference::position()).transpose()?,
        reference_overlay: reference.clone().filter(|screen| reference::is_overlay(screen)),
        reference_directory: false,
        reference_appearance: if std::env::var_os("LOCALWAVE_REFERENCE_DARK").is_some() { 2 } else { 1 },
        demo: reference.is_some()
            || (workload.is_none() && !smoke
                && lw_apple_sys::preferences::AppPaths::resources().ok().and_then(|resources| lw_bin::demo::bundled(&resources)).is_some()),
        shell_state,
        command_error,
        error_revision,
        folder_picker,
        audio_picker,
        file_drop,
        files_catalog_revision,
        files_selection_revision,
        _fixture: fixture,
    });
    if let Some(workload) = workload {
        perf::setup::prepare(&mut app, workload, window)?;
    } else if app.energy.is_none() {
        if let Some(screen) = &reference {
            reference::route(&mut app, screen, false)?;
        }
        app.player
            .enable_media_controls(!smoke && !seeded)
            .map_err(|error| error.to_string())?;
    }
    Ok(app)
}

impl App {
    fn draw(
        &mut self,
        window: &mut Window,
        now: f64,
        frame: &mut perf::Frame,
    ) -> Result<(Option<lw_ui::Intent>, f64), String> {
        let Self {
            graphics_slot,
            ui,
            #[cfg(target_os = "macos")]
            accessibility,
            themes,
            player,
            files,
            file_drop,
            error_revision,
            command_error,
            preferences,
            extent,
            platform,
            encoded,
            artwork,
            text,
            frames,
            draws,
            repaint,
            revision,
            smoke,
            shell_state,
            perf,
            events,
            demo,
            ..
        } = self;
        let smoke = *smoke;
        let graphics = graphics_slot
            .as_mut()
            .ok_or("App graphics have already closed")?;
        let wire = encoded.encode(events);
        let clock = frame.begin(ui)?;
        let scale = window.scale() as f32;
        gpu::configure_artwork(
            graphics,
            extent.logical.0 as f32,
            extent.logical.1 as f32,
            scale,
        )?;
        player
            .sync_media_artwork(graphics.images().epoch(), artwork.ready_images())
            .map_err(|error| error.to_string())?;
        let mut intent = None;
        let status = player.status();
        frame.require_waveform(status.waveform.is_some())?;
        themes.apply(ui, &status.settings, *preferences);
        let mut view = lw_ui::PlayerView {
            files: Some(files.snapshot()),
            file_dragging: file_drop.hovered(),
            settings: status.settings,
            cache_usage: &status.cache_usage,
            settings_message: lw_ui::clip(&status.settings_message),
            storage_busy: status.storage_busy,
            sharing: status.sharing,
            artwork: Some(graphics.images()),
            waveform: status.waveform.as_ref(),
            revision: status.stable_revision + (*error_revision),
            title: lw_ui::clip(&status.title),
            context: lw_ui::clip(status.context.title()),
            queue_from: lw_ui::clip(&status.queue_from),
            message: lw_ui::clip((*command_error).unwrap_or(&status.message)),
            playing: status.wants_playback,
            initialized: status.initialized,
            current_id: status.current.map(|entry| entry.song_id),
            artist: lw_ui::clip(&status.artist),
            album: lw_ui::clip(&status.album),
            info: status.info.as_deref(),
            volume: status.volume,
            shuffled: status.shuffled,
            repeat: status.repeat,
            demo: *demo,
            route: lw_apple_sys::window::ROUTE_PICKER,
            duration: status.duration,
        };
        let mut input = FrameIn::new(extent.logical.0 as f32, extent.logical.1 as f32)
            .scale(scale)
            .time(now)
            .environment(platform.environment().overlay_scrollers(preferences.overlay_scrollers != 0))
            .keyboard(platform.keyboard())
            .reduced_motion(preferences.reduced_motion != 0 || status.settings.reduce_motion);
        input.wire.appearance = preferences.appearance;
        input = input.events(&wire[..]);
        let mut position = status.position;
        frame.view(&mut view, &mut position, &mut input);
        // A window resize changes the table's wrap widths. The list prepares rows for the new layout.
        let previous = ui.view_size();
        let same_extent = (previous.width, previous.height) == (input.wire.width, input.wire.height)
            && ui.density() == scale.round().max(1.0);
        let catalog = player.catalog();
        let lookahead = if same_extent { shell_state.song_text_window(catalog) } else { None };
        text.request(ui, catalog, lookahead);
        let seek_duration = view.duration;
        // VoiceOver follows the real client; workloads keep their own a11y setting.
        #[cfg(target_os = "macos")]
        if perf.is_none() {
            accessibility.frame(ui, input.wire.surface);
        }
        ui.frame(&input, &mut |ui| {
            lw_ui::shell(
                ui,
                &view,
                position,
                catalog,
                shell_state,
                &mut intent,
            )
        })
        .map_err(|code| format!("Frame failed ({code})"))?;
        frame.check_intent(&intent, &status.settings)?;
        let mut image_requests = lw_ui::artwork::Request::default();
        graphics.images().requests(Some(ui), &mut image_requests);
        if let Some(media) = player.media_controls() {
            media.request_artwork(&mut image_requests);
        }
        frame.finish_build(clock, &image_requests, ui)?;
        artwork
            .request(&image_requests)
            .map_err(|error| error.to_string())?;
        frame.check_capture(ui, &intent)?;
        let before_output = frame.allocations();
        let output_cpu = frame.cpu_time()?;
        platform.apply(window, ui).map_err(|error| error.to_string())?;
        frame.finish_output(output_cpu)?;
        frame.output_allocations = frame.allocations() - before_output;
        (*frames) += 1;
        (*repaint) = ui.repaint_at();
        if perf.is_some() || ui.draw_revision() != (*revision) {
            let draw_cpu = frame.cpu_time()?;
            let drawn = graphics.draw(window, ui, frame.gpu_trace())
                .map_err(|error| graphics_error(ui, error))?;
            frame.finish_frame(ui, draw_cpu, graphics.acquire_wall())?;
            frame.audit(ui, artwork, player);
            if drawn {
                (*draws) += 1;
                (*revision) = ui.draw_revision();
                if smoke && (*draws) == 1 {
                    println!(
                        "LocalWave first frame: size={}x{} appearance={} reduced_motion={}",
                        extent.logical.0,
                        extent.logical.1,
                        preferences.appearance,
                        preferences.reduced_motion
                    );
                }
            } else {
                (*repaint) = Some(now + 1.0 / 60.0);
            }
        }
        Ok((intent, seek_duration))
    }
}

impl App {
    /// Draws a frame `Ui::step` advanced: no build, so no intent, images, platform output or accessibility change.
    fn step_draw(&mut self, window: &mut Window, now: f64) -> Result<(), String> {
        self.frames += 1;
        self.repaint = self.ui.repaint_at();
        if self.ui.draw_revision() == self.revision {
            return Ok(());
        }
        let graphics = self.graphics_slot.as_mut().ok_or("App graphics have already closed")?;
        if graphics.draw(window, &self.ui, None).map_err(|error| graphics_error(&self.ui, error))? {
            self.draws += 1;
            self.revision = self.ui.draw_revision();
        } else {
            self.repaint = Some(now + 1.0 / 60.0);
        }
        Ok(())
    }

    pub(super) fn frame(
        &mut self,
        window: &mut Window,
        signal: Signal,
    ) -> Result<Schedule, String> {
        if signal == Signal::Finish {
            if let Some(graphics) = self.graphics_slot.take() {
                graphics.close().map_err(|error| error.to_string())?;
            }
            return Ok(Schedule::Exit);
        }
        if matches!(signal, Signal::Resumed | Signal::LowMemory) {
            self.first = true;
            return Ok(Schedule::NextFrame);
        }
        let mut frame = if let Some(p) = &mut self.perf {
            p.inject(self.frames, self.trace_frames)
        } else {
            perf::Frame::new(self.frames, self.trace_frames)
        };
        if signal == Signal::Frame
            && self
                .graphics_slot
                .as_ref()
                .ok_or("App graphics have already closed")?
                .can_draw()
        {
            frame.begin_frame()?;
            self.events.clear();
            if !self.platform.pump(window, &mut self.events).map_err(|error| error.to_string())? {
                if self.perf.is_some() || (self.smoke && !self.smoke_finished) {
                    return Err("Window closed before qualification finished".into());
                }
                return Ok(Schedule::Exit);
            }
            if let Some(error) = window.take_failure() {
                return Err(error);
            }
            frame.external_input(&mut self.events);
            if let Some(energy) = &mut self.energy {
                energy.input(window.time(), &mut self.events);
            }
            self.input(window)?;
        }
        let mut next_preferences = self.environment.snapshot();
        // goldens (Swift's iOS canvas) and workloads keep one scroller style whatever this Mac's pointing device sets
        if self.reference.is_some() || self.perf.is_some() {
            next_preferences.overlay_scrollers = 1;
        }
        // goldens render Swift's canvas appearance (light, or dark with LOCALWAVE_REFERENCE_DARK) whatever this Mac's
        if self.reference.is_some() {
            next_preferences.appearance = self.reference_appearance;
        }
        let preferences_changed = next_preferences != self.preferences;
        self.preferences = next_preferences;
        if let Some(appearance) = self.perf.as_ref().and_then(|perf| perf.appearance(self.frames)) {
            self.preferences.appearance = appearance;
        }
        let mut library_changed = false;
        let player_changed = self
            .player
            .poll_with_catalog(&mut |previous, next| {
                library_changed |= !std::sync::Arc::ptr_eq(&previous.library, &next.library);
                self.shell_state.refresh(previous, next)
            })
            .map_err(|error| error.to_string())?;
        // Kept/availability labels come from the FilesWorker; re-read them when songs or pins change.
        // A Roots refresh would replace the reference capture's pending Browse in the worker inbox.
        if library_changed && self.shell_state.files_visible() && self.files.snapshot().account.is_some()
            && (!self.reference_directory || self.files.snapshot().directory.is_some())
        {
            let request = self.files.snapshot().directory.as_ref().map_or(
                lw_ui::files::Request::Roots,
                |directory| lw_ui::files::Request::Browse(directory.location),
            );
            let _ = self.files.request(request);
        }
        let files_changed = self.files.poll() && self.shell_state.files_visible();
        if self.files.snapshot().catalog_revision != self.files_catalog_revision
            && self.player.command(Command::RefreshCatalog).is_ok()
        {
            self.files_catalog_revision = self.files.snapshot().catalog_revision;
        }
        if let Some(selection) = &self.files.snapshot().selection
            && selection.revision != self.files_selection_revision
            && (selection.songs.is_empty()
                || self
                    .player
                    .command(Command::Configure {
                        songs: selection.songs.clone(),
                        start: 0,
                        autoplay: true,
                        context: Some(selection.context.clone()),
                    })
                    .is_ok())
        {
            self.files_selection_revision = selection.revision;
            if selection.dropped {
                self.shell_state.apply(lw_ui::Intent::CloseQueue);
                self.shell_state
                    .apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Files));
            }
        }
        if signal == Signal::Control
            || signal == Signal::Paused
            || !self
                .graphics_slot
                .as_ref()
                .ok_or("App graphics have already closed")?
                .can_draw()
        {
            self.first = true;
            return Ok(Schedule::WaitUntil(None));
        }
        let artwork_changed = self.artwork.poll();
        let artwork_updated = gpu::sync_artwork(
            self.graphics_slot
                .as_mut()
                .ok_or("App graphics have already closed")?,
            std::sync::Arc::as_ptr(&self.player.catalog().library) as usize,
            self.artwork.ready_images(),
        )?;
        frame.artwork_updated = artwork_updated;
        frame.input(&self.ui, &mut self.events)?;
        frame.state_input(&mut self.shell_state);
        let now = window.time();
        let current = window.extent();
        if current.drawable.0 <= 0 || current.drawable.1 <= 0 {
            return Ok(Schedule::WaitUntil(
                self.player
                    .poll_interval()
                    .map(|wait| window.time() + wait.as_secs_f64()),
            ));
        }
        let resized = current != self.extent;
        if resized {
            self.extent = current;
            #[cfg(target_os = "macos")]
            {
                self.top_inset = self
                    .chrome
                    .snapshot()
                    .map_err(|error| error.to_string())?
                    .top_inset;
            }
        }
        self.shell_state.set_window_inset(self.top_inset as f32);
        frame.require_extent(self.extent)?;
        let changed = self.first
            || self.perf.is_some()
            || resized
            || !self.events.is_empty()
            || player_changed
            || files_changed
            || artwork_changed
            || artwork_updated
            || preferences_changed;
        // Only a running animation is due: pxui steps it in place (no shell build) and the host redraws its damage.
        if !changed && self.repaint.is_some_and(|time| now >= time) && self.ui.step(now) {
            self.step_draw(window, now)?;
        } else if self.first
            || self.perf.is_some()
            || resized
            || !self.events.is_empty()
            || player_changed
            || files_changed
            || artwork_changed
            || artwork_updated
            || preferences_changed
            || self.repaint.is_some_and(|time| now >= time)
        {
            if let Some(report) = &mut self.frame_reasons {
                report.build(
                    self.first || resized || preferences_changed,
                    !self.events.is_empty(),
                    player_changed,
                    files_changed || artwork_changed || artwork_updated,
                    self.perf.is_some(),
                );
            }
            self.first = false;
            let (intent, duration) = self.draw(window, now, &mut frame)?;
            if let Some(intent) = intent
                && let Some(schedule) = self.intent(intent, window, now, duration)?
            {
                return Ok(schedule);
            }
            if let Some(p) = &mut self.perf
                && let Some(schedule) = p.record(
                    frame,
                    &self.ui,
                    &self.artwork,
                    self.file_drop.hovered(),
                    &mut self.graphics_slot,
                    (self.frames, self.draws),
                )?
            {
                return Ok(schedule);
            }
        }
        if let Some(report) = &mut self.frame_reasons {
            report.print(now, self.draws, &self.ui);
        }
        let energy_deadline = if let Some(energy) = &mut self.energy {
            match energy.advance(window, self.player.status(), self.draws, &self.ui)? {
                Schedule::Exit => return Ok(Schedule::Exit),
                Schedule::WaitUntil(at) => at,
                Schedule::NextFrame => Some(now),
            }
        } else {
            None
        };
        let observed_at = if self.smoke { window.time() } else { now };
        if self.smoke
            && !self.player.status().initialized
            && observed_at >= self.smoke_check.deadline()
        {
            return Err("Library initialization timed out during qualification".into());
        }
        if self.smoke
            && self.player.status().initialized
            && !self.player.status().message.is_empty()
        {
            return Err(self.player.status().message.to_string());
        }
        if self.smoke
            && self.player.status().initialized
            && self.smoke_check.observe(
                observed_at,
                self.frames,
                self.draws,
                self.repaint.is_some(),
            )?
        {
            let (frames, draws) = (self.frames, self.draws);
            println!(
                "LocalWave smoke: frames={frames} draws={draws} size={}x{} idle_ms={:.1}",
                self.extent.logical.0,
                self.extent.logical.1,
                self.smoke_check.idle_ms(observed_at)
            );
            self.smoke_finished = true;
            return Ok(Schedule::Exit);
        }
        if self.perf.is_some() {
            return Ok(Schedule::NextFrame);
        }
        let mut settle = None;
        // a page that went idle in fewer frames (the phone's full-screen player) has drawn too
        let page_idle = self.reference.as_ref().is_some_and(|(draws, changed, _)| *draws == self.draws && observed_at >= *changed + 1.0);
        if self.reference_overlay.is_some()
            && (self.draws >= 10 || page_idle)
            && self.player.status().initialized
            && (self.reference_overlay.as_deref() != Some("folder") || self.files.snapshot().account.is_some())
        {
            let screen = self.reference_overlay.take().unwrap_or_default();
            self.reference_directory = screen == "folder";
            reference::route(self, &screen, true)?;
            self.repaint = Some(now);
        }
        if self.reference_seek.is_some() && self.player.status().duration > 0.0 {
            let at = self.reference_seek.take().unwrap_or_default();
            self.player.command(Command::Seek(at)).map_err(|error| error.to_string())?;
            self.repaint = Some(now);
        }
        if let Some((draws, changed, printed)) = &mut self.reference {
            if *draws != self.draws {
                (*draws, *changed) = (self.draws, observed_at);
            } else if !*printed && self.draws > 0 && observed_at >= *changed + 2.0 && self.reference_overlay.is_none()
                && (!self.reference_directory || self.files.snapshot().directory.is_some())
            {
                *printed = true;
                if let Some(path) = &self.reference_capture {
                    let (w, h) = self.extent.logical;
                    if self.extent.drawable != (w, h) || self.ui.density() != 1.0 {
                        return Err("Reference capture requires a 1× drawable and UI density".into());
                    }
                    let graphics = self.graphics_slot.as_mut().ok_or("App graphics have already closed")?;
                    let (width, height, pixels) = graphics.screenshot(window, &self.ui).map_err(|error| graphics_error(&self.ui, error))?;
                    if (i64::from(width), i64::from(height)) != (w, h) {
                        return Err(format!("Reference drawable {width}x{height} does not match the {w}x{h} point canvas"));
                    }
                    let png = reference::capture_png((width, height, &pixels), self.top_inset.ceil() as u32)?;
                    std::fs::write(path, png).map_err(|error| format!("Could not write {}: {error}", path.display()))?;
                    println!("LOCALWAVE_REFERENCE_CAPTURED={width}x{height} ui_density=1 drawable_scale=1");
                }
                println!("LOCALWAVE_REFERENCE_READY={}x{} inset={}", self.extent.logical.0, self.extent.logical.1, self.top_inset.ceil());
            }
            settle = (!*printed).then_some(*changed + 2.0);
        }
        let deadline = if self.smoke {
            let deadline = self.smoke_check.deadline();
            Some(self.repaint.unwrap_or(deadline).min(deadline))
        } else {
            match (self.repaint, settle) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            }
        };
        let progress = self
            .player
            .poll_interval()
            .map(|wait| window.time() + wait.as_secs_f64());
        Ok(Schedule::WaitUntil(
            [deadline, progress, energy_deadline]
                .into_iter()
                .flatten()
                .min_by(f64::total_cmp),
        ))
    }
}
