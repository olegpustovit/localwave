#[path = "../src/events.rs"]
#[allow(unused_imports)] // The harness-free target omits events.rs's libtest functions.
mod events;
#[path = "../../../pxui/tools/t/window_harness.rs"]
#[allow(clippy::type_complexity)] // Shared test registry signatures.
#[rustfmt::skip] // Keep the shared harness's upstream formatting.
mod harness;

fn release_keepalive() {} // This target creates no shared GPU device.

fn system_preferences_use_appkit_notifications_and_cached_warm_reads() {
    extern "C" fn wake() {}
    let mut window =
        pxui_sdl::Window::open_with("LocalWave preferences check", 320, 240, false, true, true)
            .unwrap();
    let environment = lw_apple_sys::environment::Environment::open(wake).unwrap();
    let first = environment.snapshot();
    assert!((1..=2).contains(&first.appearance) && first.reduced_motion <= 1);
    for _ in 0..20 {
        mem_sys::local_reset();
        assert_eq!(environment.snapshot(), first);
        assert_eq!(
            mem_sys::local_alloc_count(),
            0,
            "cached AppKit reads allocate nothing"
        );
    }
    assert!(
        lw_apple_sys::environment::verify_notifications(),
        "appearance KVO, workspace notifications, duplicate suppression and observer teardown"
    );
    drop(environment);
    window.destroy().unwrap();
}

// SDL startup and real event routing require a main-thread integration process, bounded by cargo t.
fn real_numeric_input_and_ui_build_allocate_nothing_when_warm() {
    mem_sys::set_peer(pxui::library_counter);
    let mut window =
        pxui_sdl::Window::open_with("LocalWave input check", 1440, 872, false, true, true).unwrap();
    let mut ui = pxui::Ui::create(pxui::Platform::Apple, Some(15), &[lw_ui::FONT, lw_ui::MONO]);
    ui.set_a11y(true);
    let mut incoming = Vec::with_capacity(256);
    let mut wire = Vec::with_capacity(256);
    let mut scratch = Vec::new();
    let catalog = lw_ui::catalog::Catalog::new(Vec::new());
    let state = lw_ui::ShellState::default();
    let mut player = lw_ui::PlayerView {
        revision: 0,
        title: "Input check",
        context: "Library",
        message: "",
        playing: false,
        initialized: true,
        duration: 300.0,
        ..lw_ui::PlayerView::default()
    };
    for frame in 0..120 {
        if frame == 20 {
            mem_sys::local_reset();
        }
        incoming.clear();
        assert!(sdl3_sys::test_util::push_numeric_trace());
        assert!(window.pump(&mut incoming));
        assert!(window.take_failure().is_none());
        if frame >= 20 {
            assert_eq!(mem_sys::local_alloc_count(), 0, "SDL pump allocated");
        }
        events::encode(&incoming, &mut scratch, &mut wire);
        if frame >= 20 {
            assert_eq!(mem_sys::local_alloc_count(), 0, "event encoding allocated");
        }
        player.revision = frame;
        player.position = frame as f64;
        player.playing = frame % 2 == 0;
        player.context = if player.playing {
            "Favorites"
        } else {
            "Library"
        };
        player.message = if player.playing {
            "Unavailable track skipped"
        } else {
            ""
        };
        let mut intent = None;
        ui.frame(
            &pxui::FrameIn {
                width: 1440.0,
                height: 872.0,
                time: frame as f64,
                events: wire.as_ptr(),
                event_count: wire.len(),
                ..pxui::FrameIn::default()
            },
            &mut |ui| lw_ui::shell(ui, &player, &catalog, &state, &mut intent),
        )
        .unwrap();
        if frame >= 20 {
            assert_eq!(mem_sys::local_alloc_count(), 0, "UI input/build allocated");
        }
    }
    let allocations = mem_sys::local_alloc_count();
    assert_eq!(
        allocations, 0,
        "native SDL input through app encoding and UI build allocated after warming"
    );
    window.destroy().unwrap();
}

fn main() {
    harness::run(&[
        (
            "system_preferences_use_appkit_notifications_and_cached_warm_reads",
            system_preferences_use_appkit_notifications_and_cached_warm_reads,
        ),
        (
            "real_numeric_input_and_ui_build_allocate_nothing_when_warm",
            real_numeric_input_and_ui_build_allocate_nothing_when_warm,
        ),
    ]);
}
