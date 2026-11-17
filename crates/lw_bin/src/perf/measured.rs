//! Input drivers of the DX0.3 workloads. First navigation and first menu open are measured-only
//! (they report warm Rust allocations and timing columns but never fail the run); the others are
//! gated. Drivers are pure functions of the frame number: no allocation, no clocks.
use super::Workload;
use input::{Event, ScrollPhase};

/// Frames of silence between warm-up and the return input of the idle-600 workload.
pub(super) const IDLE_FRAMES: usize = 600;

/// The ten navigation targets are visited once each in the measured window, `NAV_STEP` frames apart.
pub(super) const NAV_STEP: usize = 12;
const SEARCH_TEXT: &str = "Track m4a year:2019";
const TYPED: &str = "Track 00001-remastered";

fn wheel(frame: usize, events: &mut Vec<Event>) {
    events.push(Event::PointerMoved((650.0, 400.0)));
    events.push(Event::Scrolled((0.0, if frame % 40 < 20 { -1.0 } else { 1.0 })));
}

/// Trackpad touch, five drags, lift, then decaying momentum; direction flips each 40-frame cycle.
fn fling(frame: usize, events: &mut Vec<Event>) {
    let phase = frame % 40;
    let dir = if (frame / 40) % 2 == 0 { -1.0 } else { 1.0 };
    let time = frame as f64 / 60.0;
    let (phase_kind, delta) = match phase {
        0 => (ScrollPhase::Touched, 0.0),
        1..=5 => (ScrollPhase::Moved, 30.0),
        6 => (ScrollPhase::Lifted, 0.0),
        7..=26 => (ScrollPhase::Momentum, 30.0 * 0.85f64.powi(phase as i32 - 6)),
        _ => return,
    };
    if phase == 0 {
        events.push(Event::PointerMoved((650.0, 400.0)));
    }
    events.push(Event::TrackpadScrolled { delta: (0.0, dir * delta), phase: phase_kind, time });
}

/// Fill `events` for a measured workload at `frame` (counted from the first driven frame).
/// `from` is the first measured frame index; `ui` is needed only to find the first menu target.
pub(super) fn events(
    workload: Workload,
    frame: usize,
    from: usize,
    ui: &pxui::Ui,
    events: &mut Vec<Event>,
) -> Result<(), String> {
    match workload {
        Workload::Fling => fling(frame, events),
        Workload::ReverseScroll => {
            events.push(Event::PointerMoved((650.0, 400.0)));
            events.push(Event::Scrolled((0.0, if (frame / 5) % 2 == 0 { -3.0 } else { 3.0 })));
        }
        Workload::Jump => {
            events.push(Event::PointerMoved((650.0, 400.0)));
            if frame % 10 == 0 {
                events.push(Event::Scrolled((0.0, if (frame / 10) % 2 == 0 { -500.0 } else { 500.0 })));
            }
        }
        Workload::SearchTyping | Workload::TextTyping | Workload::Resize => {
            events.push(Event::PointerMoved((720.0, 400.0)));
        }
        Workload::FirstMenuOpen => {
            // The menu is opened for the first time at the first measured frame, never in warm-up.
            events.push(Event::PointerMoved((720.0, 400.0)));
            if let Some(menu_frame) = (frame + 2).checked_sub(from)
                && menu_frame <= 10
            {
                events.clear();
                super::drive::list_dialog_events(menu_frame, ui, events)?;
            }
        }
        Workload::Idle600 => {
            // Warm up, stay silent for 600 frames, then return with input.
            if frame + 2 < super::WARMUP_FRAMES || frame + 2 >= from {
                wheel(frame, events);
            }
        }
        _ => wheel(frame, events),
    }
    Ok(())
}

/// Typing as the Intents a real text field emits (owned TextInput Strings would allocate).
pub(super) fn state_input(
    workload: Workload,
    frame: usize,
    from: usize,
    state: &mut lw_ui::ShellState,
) {
    use lw_ui::{Intent, browse::Query, catalog::Route};
    match workload {
        Workload::SearchTyping => {
            let phase = frame % 40;
            let typed = if phase < 20 { phase } else { 39 - phase };
            if let Some(query) = Query::new(&SEARCH_TEXT[..typed.min(SEARCH_TEXT.len())]) {
                state.apply(Intent::Search(query));
            }
        }
        Workload::TextTyping => {
            let phase = frame % 22;
            let typed = if phase < 11 { phase + 1 } else { 21 - phase };
            if let Some(title) = Query::new(&TYPED[..super::drive::TITLE_TEXT.len() + typed]) {
                state.apply(Intent::MetadataField(0, title));
            }
        }
        Workload::NonSongLists => {
            let route = match frame % 40 {
                0 => Route::Albums,
                14 => Route::Lists,
                27 => Route::Artists,
                _ => return,
            };
            state.apply(Intent::Navigate(route));
        }
        Workload::FirstNavigation => {
            if let Some(step) = (frame + 2).checked_sub(from)
                && step % NAV_STEP == 0
                && let Some(route) = [
                    Route::Albums,
                    Route::Artists,
                    Route::Lists,
                    Route::Find,
                    Route::Album(0),
                    Route::Artist(0),
                ]
                .get(step / NAV_STEP)
            {
                state.apply(Intent::Navigate(*route));
            }
        }
        _ => {}
    }
}

/// Logical window size of the resize workload: shrink in ten steps and grow back each 20 frames.
pub(super) fn resize_extent(index: usize) -> (f32, f32) {
    let step = index % 20;
    let step = if step < 10 { step } else { 20 - step } as f32;
    (1440.0 - step * 40.0, 872.0 - step * 20.0)
}

/// Frame time of the 120 fps clock workload.
pub(super) fn clock_120(index: usize) -> f64 {
    index as f64 / 120.0
}

/// Forced system appearance of the theme-switch workload: flips every ten frames.
pub(super) fn appearance(index: usize) -> u32 {
    if (index / 10) % 2 == 0 {
        pxui::FrameIn::APPEARANCE_LIGHT
    } else {
        pxui::FrameIn::APPEARANCE_DARK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEASURED: [Workload; 11] = [
        Workload::Fling,
        Workload::ReverseScroll,
        Workload::Jump,
        Workload::NonSongLists,
        Workload::SearchTyping,
        Workload::FirstNavigation,
        Workload::TextTyping,
        Workload::FirstMenuOpen,
        Workload::Resize,
        Workload::ThemeSwitch,
        Workload::Idle600,
    ];

    fn run(workload: Workload) -> Vec<Vec<Event>> {
        let ui = pxui::Ui::new().unwrap();
        let mut buffer = Vec::with_capacity(4);
        let mut out = Vec::new();
        let from = super::super::Perf::new(workload, None).measured_from();
        for frame in 0..from + super::super::MEASURED_FRAMES {
            buffer.clear();
            mem_sys::local_reset();
            // The menu target needs a real a11y tree; its events are covered by the drive tests.
            if workload == Workload::FirstMenuOpen && (frame + 2).checked_sub(from).is_some_and(|step| step <= 10) {
                out.push(Vec::new());
                continue;
            }
            events(workload, frame, from, &ui, &mut buffer).unwrap();
            assert_eq!(mem_sys::local_alloc_count(), 0, "{workload:?} frame {frame}");
            out.push(buffer.clone());
        }
        out
    }

    #[test]
    fn every_driver_is_deterministic_and_allocation_free() {
        for workload in MEASURED {
            assert_eq!(run(workload), run(workload), "{workload:?}");
        }
    }

    #[test]
    fn fling_touches_drags_lifts_and_decays_in_both_directions() {
        let out = run(Workload::Fling);
        assert!(matches!(out[0].last(), Some(Event::TrackpadScrolled { phase: ScrollPhase::Touched, .. })));
        assert!(matches!(out[6].last(), Some(Event::TrackpadScrolled { phase: ScrollPhase::Lifted, .. })));
        let momentum = |frame: usize| match out[frame].last() {
            Some(Event::TrackpadScrolled { delta, phase: ScrollPhase::Momentum, .. }) => delta.1,
            other => panic!("{other:?}"),
        };
        assert!(momentum(7).abs() > momentum(20).abs() && momentum(7) < 0.0);
        assert!(momentum(47) > 0.0);
        assert!(out[30].is_empty());
    }

    #[test]
    fn reverse_and_jump_change_direction_on_schedule() {
        let reverse = run(Workload::ReverseScroll);
        assert_eq!(reverse[0][1], Event::Scrolled((0.0, -3.0)));
        assert_eq!(reverse[5][1], Event::Scrolled((0.0, 3.0)));
        let jump = run(Workload::Jump);
        assert_eq!(jump[0][1], Event::Scrolled((0.0, -500.0)));
        assert_eq!(jump[1].len(), 1);
        assert_eq!(jump[10][1], Event::Scrolled((0.0, 500.0)));
    }

    #[test]
    fn idle_return_is_silent_for_six_hundred_frames_then_resumes() {
        let from = super::super::Perf::new(Workload::Idle600, None).measured_from();
        assert_eq!(from, super::super::WARMUP_FRAMES + IDLE_FRAMES);
        let out = run(Workload::Idle600);
        assert!(!out[0].is_empty() && !out[super::super::WARMUP_FRAMES - 3].is_empty());
        assert!(out[super::super::WARMUP_FRAMES - 2..from - 2].iter().all(Vec::is_empty));
        assert!(!out[from - 2].is_empty());
    }

    #[test]
    fn typing_navigation_and_clock_inputs_follow_their_schedules() {
        let mut state = lw_ui::ShellState::default();
        for workload in [Workload::SearchTyping, Workload::TextTyping, Workload::NonSongLists, Workload::FirstNavigation] {
            if workload == Workload::TextTyping {
                state.apply(lw_ui::Intent::NewList(None));
            }
            for frame in 0..super::super::WARMUP_FRAMES + super::super::MEASURED_FRAMES {
                mem_sys::local_reset();
                state_input(workload, frame, super::super::WARMUP_FRAMES, &mut state);
                assert_eq!(mem_sys::local_alloc_count(), 0, "{workload:?} frame {frame}");
            }
        }
        assert_eq!(resize_extent(0), (1440.0, 872.0));
        assert_eq!(resize_extent(10), (1040.0, 672.0));
        assert_eq!(resize_extent(20), resize_extent(0));
        assert_eq!(clock_120(120), 1.0);
        assert_eq!(appearance(0), pxui::FrameIn::APPEARANCE_LIGHT);
        assert_eq!(appearance(10), pxui::FrameIn::APPEARANCE_DARK);
    }
}
