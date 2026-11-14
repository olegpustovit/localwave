#[path = "../src/events.rs"]
#[allow(unused_imports)] // The harness-free target omits events.rs's libtest functions.
mod events;
#[path = "../src/gpu.rs"]
mod gpu;
#[path = "../../../pxui/tools/t/window_harness.rs"]
#[allow(clippy::type_complexity)] // Shared test registry signatures.
#[rustfmt::skip] // Keep the shared harness's upstream formatting.
mod harness;

fn release_keepalive() {} // This target creates no shared GPU device.

fn cloud_identity_reads_reject_the_initial_thread() {
    assert!(lw_apple_sys::cloud::current_identity().is_err());
}

fn removing_a_source_keeps_current_native_item_metadata_and_cache_lease_until_retirement() {
    // Native paused-item installation and AppKit callbacks require the initial-thread integration host.
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    use lw_bin::{
        controller::{Command, Controller},
        library::LibraryWorker,
    };
    use std::{
        sync::{Arc, mpsc},
        time::{Duration, Instant},
    };
    let base = std::env::temp_dir().join(format!("lw-source-player-{}", std::process::id()));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(base.clone());
    let path = base.join("Documents/Added Files/track.m4a");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        include_bytes!("../../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a"),
    )
    .unwrap();
    let path = path.canonicalize().unwrap();
    let suite = format!("localwave.source-player.{}", std::process::id());
    let factory_base = base.clone();
    let factory_suite = suite.clone();
    let (send, receive) = mpsc::sync_channel(1);
    extern "C" fn wake() {}
    let mut controller = Controller::open(
        move || {
            let mut library = LibraryWorker::open(
                &factory_base.join("library.sqlite"),
                Preferences::open(Some(&factory_suite))?,
                AppPaths {
                    documents: factory_base.join("Documents"),
                    caches: factory_base.join("Caches"),
                    temporary: factory_base.join("tmp"),
                    test_database: None,
                },
            )?;
            let user = library
                .repository
                .get_or_create_user(-1)
                .map_err(std::io::Error::other)?;
            let source = library
                .repository
                .create_source(&lw_core::Source {
                    user_id: user.id,
                    dir_path: "/Original music".into(),
                    path_id: 77,
                    ..Default::default()
                })
                .map_err(std::io::Error::other)?;
            let song = library
                .repository
                .upsert_song(
                    &lw_core::Song {
                        song_key: 1,
                        source_id: source.id,
                        title: "Retained current item".into(),
                        artist: "Retained artist".into(),
                        local_file_path: Some("Added Files/track.m4a".into()),
                        file_state: lw_core::FileState::Copied,
                        ..Default::default()
                    },
                    1.0,
                )
                .map_err(std::io::Error::other)?;
            send.send((song.id.unwrap(), source))
                .map_err(std::io::Error::other)?;
            Ok(library)
        },
        wake,
    )
    .unwrap();
    let (id, source) = receive.recv_timeout(Duration::from_secs(1)).unwrap();
    controller
        .command(Command::Configure {
            songs: Arc::from([id]),
            start: 0,
            autoplay: false,
            context: None,
        })
        .unwrap();
    let until = |controller: &mut Controller, deleted: bool| {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            controller.run_loop(Duration::from_millis(1));
            controller.poll().unwrap();
            if controller.native_snapshot().token != 0
                && controller.status().title.as_ref() == "Retained current item"
                && (!deleted || controller.catalog().row_index(id).is_none())
            {
                break;
            }
            assert!(Instant::now() < deadline, "{:?}", controller.status());
        }
    };
    until(&mut controller, false);
    let native = controller.native_snapshot();
    let database = base.join("library.sqlite");
    std::thread::spawn(move || {
        let mut repository = lw_core::LibraryRepository::open(database.to_str().unwrap()).unwrap();
        repository
            .delete_source_with_locations(&source, &[])
            .unwrap();
    })
    .join()
    .unwrap();
    controller.command(Command::RefreshCatalog).unwrap();
    until(&mut controller, true);
    assert_eq!(controller.native_snapshot().token, native.token);
    assert_eq!(controller.native_snapshot().occurrence, native.occurrence);
    assert_eq!(controller.status().artist.as_ref(), "Retained artist");
    assert!(controller.status().fault.is_none());
    let inspect_suite = suite.clone();
    let inspect_path = path.clone();
    std::thread::spawn(move || {
        let preferences = Preferences::open(Some(&inspect_suite)).unwrap();
        assert!(
            preferences
                .cache_protection()
                .path_protected(&inspect_path)
                .unwrap()
        );
    })
    .join()
    .unwrap();
    assert!(path.is_file());
    drop(controller);
    std::thread::spawn(move || {
        let preferences = Preferences::open(Some(&suite)).unwrap();
        assert!(
            !preferences
                .cache_protection()
                .path_protected(&path)
                .unwrap()
        );
    })
    .join()
    .unwrap();
}

fn native_folder_dialog_delivery_matches_owner_and_handles_cancel_failure_once() {
    // Initial-thread AppKit/SDL startup can exceed one second; no real picker or timed wait is used.
    let mut window = pxui_sdl::Window::open_with(
        "LocalWave folder delivery check",
        800,
        600,
        false,
        true,
        true,
    )
    .unwrap();
    let mut events = Vec::with_capacity(32);
    assert!(window.pump(&mut events));
    let chosen = sdl3_sys::test_util::dialog_reserve();
    let mut pending = Some((i64::from(chosen), events::FolderPurpose::Add(42)));
    assert!(sdl3_sys::test_util::dialog_finish(
        chosen,
        Some(&["/Music/名前"])
    ));
    assert!(sdl3_sys::wait_event_timeout(0));
    events.clear();
    assert!(window.pump(&mut events));
    assert_eq!(
        events::folder_selection(&mut pending, &mut events),
        Some(Ok(Some((
            events::FolderPurpose::Add(42),
            "/Music/名前".into()
        ))))
    );
    assert!(events::folder_selection(&mut pending, &mut events).is_none());
    let reconnect = sdl3_sys::test_util::dialog_reserve();
    let target = lw_ui::files::Target {
        user: 42,
        source: 7,
        root: 99,
    };
    pending = Some((
        i64::from(reconnect),
        events::FolderPurpose::Reconnect(target),
    ));
    assert!(sdl3_sys::test_util::dialog_finish(
        reconnect,
        Some(&["/Music/Reconnected"])
    ));
    events.clear();
    assert!(window.pump(&mut events));
    assert_eq!(
        events::folder_selection(&mut pending, &mut events),
        Some(Ok(Some((
            events::FolderPurpose::Reconnect(target),
            "/Music/Reconnected".into()
        ))))
    );
    for failed in [false, true] {
        let id = sdl3_sys::test_util::dialog_reserve();
        pending = Some((i64::from(id), events::FolderPurpose::Add(43)));
        assert!(if failed {
            sdl3_sys::test_util::dialog_fail(id)
        } else {
            sdl3_sys::test_util::dialog_finish(id, Some(&[]))
        });
        assert!(sdl3_sys::wait_event_timeout(0));
        events.clear();
        assert!(window.pump(&mut events));
        let result = events::folder_selection(&mut pending, &mut events).unwrap();
        if failed {
            assert!(result.is_err());
        } else {
            assert_eq!(result, Ok(None));
        }
        assert!(pending.is_none());
    }
    window.destroy().unwrap();
}

fn media_commands_wait_for_controller_capacity_without_losing_accepted_events() {
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    use lw_bin::{
        controller::{Command, Controller, Status},
        library::LibraryWorker,
    };
    use std::{
        sync::{Arc, mpsc},
        time::{Duration, Instant},
    };
    let base = std::env::temp_dir().join(format!("lw-media-{}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(base.clone());
    let worker_base = base.clone();
    let (release, blocked) = mpsc::sync_channel(1);
    extern "C" fn wake() {}
    let mut controller = Controller::open(
        move || {
            blocked.recv().map_err(std::io::Error::other)?;
            LibraryWorker::open(
                &worker_base.join("library.sqlite"),
                Preferences::open(Some(&format!(
                    "com.olegpustovit.localwave.media-{}",
                    std::process::id()
                )))?,
                AppPaths {
                    documents: worker_base.join("Documents"),
                    caches: worker_base.join("Caches"),
                    temporary: worker_base.join("tmp"),
                    test_database: None,
                },
            )
        },
        wake,
    )
    .unwrap();
    struct Release(std::sync::mpsc::SyncSender<()>);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = self.0.try_send(());
        }
    }
    let release = Release(release);
    controller.enable_media_controls(false).unwrap();
    for _ in 0..32 {
        controller.command(Command::DismissNotice).unwrap();
    }
    let status = Status {
        phase: lw_core::playback::Phase::Paused,
        queue_len: 1,
        title: Arc::from("Isolated"),
        ..Default::default()
    };
    let media = controller.media_controls().unwrap();
    media.sync(&status).unwrap();
    for _ in 0..32 {
        assert_eq!(media.native.inject(1, 0.0), 0);
    }
    assert_eq!(media.native.inject(1, 0.0), 200);
    controller.poll().unwrap();
    assert_eq!(
        controller
            .media_controls()
            .unwrap()
            .native
            .peek()
            .unwrap()
            .kind,
        1,
        "busy worker must not consume the remote head"
    );
    release.0.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        controller.poll().unwrap();
        if controller.status().processed_command >= 64 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "accepted remote commands were lost"
        );
        std::thread::yield_now();
    }
    assert!(controller.media_controls().unwrap().native.peek().is_none());
    assert_eq!(controller.status().queue_len, 0);
}

fn native_media_callbacks_metadata_artwork_and_warm_delivery_are_isolated_and_bounded() {
    use lw_bin::controller::Status;
    use lw_core::{Song, playback::Phase, queue::Entry};
    use lw_ui::{
        artwork::{Key, Request},
        browse::Query,
        catalog::Catalog,
    };
    use std::sync::Arc;
    assert_eq!(
        lw_apple_sys::media::verify_callbacks(),
        0,
        "native callback stage"
    );
    extern "C" fn wake() {}
    let mut media = lw_bin::media::Media::open(false, wake).unwrap();
    assert_eq!(media.native.inject(1, 0.0), 110);
    let mut status = Status {
        current: Some(Entry { id: 1, song_id: 7 }),
        queue_len: 2,
        generation: 9,
        phase: Phase::Paused,
        title: Arc::from("Track 🎵"),
        artist: Arc::from("Artist"),
        album: Arc::from("Album"),
        position: 1.25,
        duration: 30.0,
        ..Default::default()
    };
    let catalog = Catalog::new(vec![Song {
        id: Some(7),
        title: status.title.to_string(),
        cover_art_path: Some("cover.png".into()),
        ..Default::default()
    }]);
    media.sync(&status).unwrap();
    media.artwork(&catalog, 0, std::iter::empty()).unwrap();
    let mut request = Request::default();
    for id in 2..=9 {
        assert!(request.insert(id, "other.png", 512));
    }
    media.request_artwork(&mut request);
    assert_eq!(request.keys().len(), 8);
    assert_eq!(
        request.keys()[0].id,
        1,
        "Now Playing must not starve behind a full visible packet"
    );
    let ready = lw_bin::library::ArtworkReady {
        epoch: 0,
        revision: 10,
        key: Key {
            id: 1,
            path: Query::new("cover.png").unwrap(),
            max_pixels: 256,
        },
        thumbnail: Some(lw_apple_sys::artwork::Thumbnail {
            width: 2,
            height: 1,
            pixels: vec![255, 0, 0, 255, 0, 128, 255, 128],
        }),
    };
    media.artwork(&catalog, 0, std::iter::once(&ready)).unwrap();
    assert_eq!(media.native.state().artwork_width, 2);
    let baseline = media.native.state();
    mem_sys::local_reset();
    for _ in 0..1000 {
        media.sync(&status).unwrap();
        media.artwork(&catalog, 0, std::iter::once(&ready)).unwrap();
        let mut request = Request::default();
        media.request_artwork(&mut request);
        assert!(request.keys().is_empty());
        assert!(media.native.peek().is_none());
    }
    assert_eq!(mem_sys::local_alloc_count(), 0);
    assert_eq!(
        media.native.state(),
        baseline,
        "quiet polls must not republish metadata"
    );
    for kind in 1..=7 {
        assert_eq!(media.native.inject(kind, 3.5), 0);
    }
    for kind in 1..=7 {
        let head = media.native.peek().unwrap();
        assert_eq!(head.kind, kind);
        assert_eq!(
            media.native.peek(),
            Some(head),
            "backpressure must retain the head"
        );
        media.native.acknowledge();
    }
    assert!(media.native.peek().is_none());
    assert_eq!(media.native.inject(7, f64::NAN), 200);
    status.position += 0.001;
    media.sync(&status).unwrap();
    assert_eq!(
        media.native.state().updates,
        baseline.updates + 1,
        "small paused seeks must publish"
    );
    assert_eq!(
        media.native.state().artwork_width,
        2,
        "clock changes must retain artwork"
    );
    status.phase = Phase::Playing;
    media.sync(&status).unwrap();
    mem_sys::local_reset();
    for _ in 0..1000 {
        media.sync(&status).unwrap();
    }
    assert_eq!(mem_sys::local_alloc_count(), 0);
    status.position += 5.0;
    media.sync(&status).unwrap();
    assert_eq!(media.native.state().position, status.position);
    media.artwork(&catalog, 1, std::iter::once(&ready)).unwrap();
    assert_eq!(
        media.native.state().artwork_width,
        0,
        "old-epoch artwork must be cleared"
    );
    status.phase = Phase::Stopped;
    media.sync(&status).unwrap();
    assert_eq!(media.native.state().song, 0);
    assert_eq!(media.native.state().playback_state, 3);
    status.queue_len = 0;
    media.sync(&status).unwrap();
    assert_eq!(media.native.inject(1, 0.0), 110);
}

fn worker_artwork_uploads_exact_pixels_renders_warm_and_retires_after_gpu_completion() {
    // ImageIO, real Metal presentation and its frame fences require this main-thread integration process.
    mem_sys::set_peer(pxui::library_counter);
    let mut window =
        pxui_sdl::Window::open_with("LocalWave artwork check", 1440, 872, false, true, true)
            .unwrap();
    let mut ui = pxui::Ui::create(pxui::Platform::Apple, Some(15), &[lw_ui::FONT, lw_ui::MONO]);
    let mut graphics = gpu::Graphics::open(&mut window, &ui).unwrap();
    static DECODED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    extern "C" fn wake() {
        DECODED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        pxui_sdl::wake();
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../Tests/LocalWaveDataTests/Fixtures/artwork-rgba.png");
    let path = path.to_str().unwrap();
    let mut worker = lw_bin::library::ArtworkWorker::open(
        Some(std::path::Path::new(path).parent().unwrap().to_owned()),
        wake,
    )
    .unwrap();
    let catalog = lw_ui::catalog::Catalog::new(vec![lw_core::Song {
        id: Some(7),
        title: "Covered song".into(),
        album: "Covered album".into(),
        cover_art_path: Some(path.into()),
        ..Default::default()
    }]);
    let mut state = lw_ui::ShellState::default();
    state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Albums));
    graphics
        .sync_artwork(std::sync::Arc::as_ptr(&catalog.library) as usize, None)
        .unwrap();
    graphics.configure_artwork(1440.0, 872.0, 1.0).unwrap();
    for id in 1..=lw_ui::artwork::REQUEST_CAPACITY {
        graphics.artwork.images.get(id, path, 360.0);
    }
    worker.request(graphics.artwork.images.requests()).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    while DECODED.load(std::sync::atomic::Ordering::SeqCst) < lw_ui::artwork::REQUEST_CAPACITY {
        assert!(
            std::time::Instant::now() < deadline,
            "artwork worker timed out"
        );
        std::thread::yield_now();
    }
    assert!(worker.poll());
    assert_eq!(
        worker.ready_images().count(),
        lw_ui::artwork::REQUEST_CAPACITY
    );
    graphics
        .sync_artwork(
            std::sync::Arc::as_ptr(&catalog.library) as usize,
            worker.ready_images(),
        )
        .unwrap();
    for source in worker.ready_images() {
        let texture = graphics.artwork.texture(source.key.id).unwrap();
        let bytes = ogpu_core::backend::read_texture(texture, 8 * 4).unwrap();
        assert_eq!(bytes, source.thumbnail.as_ref().unwrap().pixels);
    }
    assert_eq!(
        graphics.artwork.resident_bytes(),
        8 * 4 * 4 * lw_ui::artwork::REQUEST_CAPACITY
    );
    for frame in 0..24 {
        mem_sys::local_reset();
        assert!(
            !graphics
                .sync_artwork(
                    std::sync::Arc::as_ptr(&catalog.library) as usize,
                    worker.ready_images()
                )
                .unwrap()
        );
        let player = lw_ui::PlayerView {
            artwork: Some(&graphics.artwork.images),
            waveform: Some(&[0.5; 512]),
            revision: frame,
            current_id: Some(7),
            initialized: true,
            title: "Covered song",
            position: frame as f64,
            ..Default::default()
        };
        ui.frame(
            &pxui::FrameIn {
                width: 1440.0,
                height: 872.0,
                time: frame as f64 * 0.02,
                ..Default::default()
            },
            &mut |ui| lw_ui::shell(ui, &player, &catalog, &state, &mut None),
        )
        .unwrap();
        assert!(graphics.draw(&mut window, &ui, None).unwrap());
        assert!(
            ui.layers()
                .map(|layer| layer
                    .instances
                    .iter()
                    .filter(|instance| instance.kind == pxui::draw::kind::IMAGE)
                    .count())
                .sum::<usize>()
                >= 2,
            "album and Now panel must contain actual image draws"
        );
        // The first ten clocks visit every digit before measuring the warm glyph/image paths.
        if frame >= 10 {
            assert_eq!(
                mem_sys::local_alloc_count(),
                0,
                "warm artwork build, glyphs, image wrappers and native draw"
            );
        }
        assert!(graphics.artwork.images.requests().keys().is_empty());
    }
    let original = graphics.artwork.texture(1).unwrap().clone();
    graphics.artwork.sync_library(42).unwrap();
    assert!(!graphics.sync_artwork(42, worker.ready_images()).unwrap());
    for id in 1..=lw_ui::artwork::REQUEST_CAPACITY {
        assert!(
            graphics.artwork.images.quality(id).is_none(),
            "a replayed batch must not resurrect evicted covers"
        );
    }
    assert!(!ogpu_core::backend::texture_destroyed(&original));
    graphics.retire_artwork(0).unwrap();
    assert!(!ogpu_core::backend::texture_destroyed(&original));
    for frame in 0..3 {
        let player = lw_ui::PlayerView {
            artwork: Some(&graphics.artwork.images),
            revision: 30 + frame,
            ..Default::default()
        };
        ui.frame(
            &pxui::FrameIn {
                width: 1440.0,
                height: 872.0,
                time: 1.0 + frame as f64 * 0.02,
                ..Default::default()
            },
            &mut |ui| lw_ui::shell(ui, &player, &catalog, &state, &mut None),
        )
        .unwrap();
        let image_quads: usize = ui
            .layers()
            .map(|layer| {
                layer
                    .instances
                    .iter()
                    .filter(|instance| instance.kind == pxui::draw::kind::IMAGE)
                    .count()
            })
            .sum();
        assert_eq!(
            image_quads, 0,
            "cleared artwork must not leave image instances"
        );
        assert!(graphics.draw(&mut window, &ui, None).unwrap());
    }
    assert!(ogpu_core::backend::texture_destroyed(&original));
    drop(worker);
    graphics.close().unwrap();
    window.destroy().unwrap();
}

fn native_window_resizes_enables_zoom_and_measures_titlebar_clearance() {
    // Native AppKit/SDL startup and window geometry require a main-thread integration process.
    let mut window =
        pxui_sdl::Window::open_with("LocalWave chrome check", 800, 600, false, true, true).unwrap();
    window.set_resizable(true).unwrap();
    let chrome = lw_apple_sys::window::Chrome::open(window.token().unwrap()).unwrap();
    for (width, height) in [(800, 600), (1000, 720), (640, 480)] {
        window.resize(width, height).unwrap();
        let state = chrome.snapshot().unwrap();
        assert_eq!(
            (
                state.resizable,
                state.zoom_enabled,
                state.full_size_content,
                state.full_screen_primary
            ),
            (1, 1, 1, 1),
            "{state:?}"
        );
        assert!(state.top_inset > 0.0 && state.top_inset < 80.0, "{state:?}");
        assert_eq!(window.extent().logical, (width, height));
        assert_eq!(
            (state.content_width, state.content_height),
            (width as f64, height as f64)
        );
    }
    window.destroy().unwrap();
    assert!(
        chrome.snapshot().is_err(),
        "invalidated native tokens cannot be queried"
    );
}

fn system_preferences_use_appkit_notifications_and_cached_warm_reads() {
    extern "C" fn wake() {}
    let mut window =
        pxui_sdl::Window::open_with("LocalWave preferences check", 320, 240, false, true, true)
            .unwrap();
    let environment = lw_apple_sys::environment::Environment::open(wake).unwrap();
    let cover = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../Tests/LocalWaveDataTests/Fixtures/artwork-rgba.png");
    assert!(
        lw_apple_sys::artwork::thumbnail(&cover, 32).is_err(),
        "image decoding must never run on the UI thread"
    );
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
        waveform: Some(&[0.5; 512]),
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
            "removing_a_source_keeps_current_native_item_metadata_and_cache_lease_until_retirement",
            removing_a_source_keeps_current_native_item_metadata_and_cache_lease_until_retirement,
        ),
        (
            "native_folder_dialog_delivery_matches_owner_and_handles_cancel_failure_once",
            native_folder_dialog_delivery_matches_owner_and_handles_cancel_failure_once,
        ),
        (
            "cloud_identity_reads_reject_the_initial_thread",
            cloud_identity_reads_reject_the_initial_thread,
        ),
        (
            "media_commands_wait_for_controller_capacity_without_losing_accepted_events",
            media_commands_wait_for_controller_capacity_without_losing_accepted_events,
        ),
        (
            "native_media_callbacks_metadata_artwork_and_warm_delivery_are_isolated_and_bounded",
            native_media_callbacks_metadata_artwork_and_warm_delivery_are_isolated_and_bounded,
        ),
        (
            "worker_artwork_uploads_exact_pixels_renders_warm_and_retires_after_gpu_completion",
            worker_artwork_uploads_exact_pixels_renders_warm_and_retires_after_gpu_completion,
        ),
        (
            "native_window_resizes_enables_zoom_and_measures_titlebar_clearance",
            native_window_resizes_enables_zoom_and_measures_titlebar_clearance,
        ),
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
