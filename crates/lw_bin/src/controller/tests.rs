use super::*;

#[test]
fn position_ticks_publish_without_advancing_stable_revision_or_allocating() {
    let mut status = Status::default();
    assert!(!status.update(status.clone()));
    mem_sys::local_reset();
    for tick in 1..=20 {
        let mut next = status.clone();
        next.position = tick as f64 * 0.05;
        assert!(status.update(next));
        assert_eq!((status.revision, status.stable_revision), (tick, 0));
        assert!(!status.update(status.clone()));
    }
    assert_eq!(mem_sys::local_alloc_count(), 0);
    let mut next = status.clone();
    next.wants_playback = true;
    next.title = Arc::from("Updated metadata");
    next.position = 2.0;
    assert!(status.update(next));
    assert_eq!((status.revision, status.stable_revision), (21, 1));
    let mut next = status.clone();
    next.duration = 240.0;
    next.waveform = Some([0.5; 512]);
    assert!(status.update(next));
    assert_eq!((status.revision, status.stable_revision), (22, 2));
    assert_eq!(status.position, 2.0);
}
use lw_core::preferences::Change as SettingChange;
use lw_ui::catalog::Catalog;
extern "C" fn wake() {}
#[test]
fn audio_session_events_fence_preparation_and_only_pause_for_interruptions_or_faults() {
    use lw_apple_sys::session::Event;
    for event in [
        Event::Began,
        Event::OutputUnavailable,
        Event::Overflow,
        Event::Reset,
        Event::Ended { resume: false },
        Event::Ended { resume: true },
    ] {
        let command = Command::AudioSession(event);
        assert!(command.validate().is_ok());
        assert!(command.fences_preparation());
        assert_eq!(command.interrupts(), event.pauses());
    }
}
#[test]
fn fresh_shuffle_commands_validate_and_interrupt_the_previous_selection() {
    let command = Command::ShuffleAll {
        songs: Arc::from([1, 1, 2]),
        seed: 42,
        context: None,
    };
    assert!(command.validate().is_ok());
    assert!(command.fences_preparation());
    assert!(command.interrupts());
    assert!(
        Command::ShuffleAll {
            songs: Arc::from([]),
            seed: 0,
            context: None,
        }
        .validate()
        .is_err()
    );
}
#[test]
fn settings_requests_validate_limits_and_only_file_policy_changes_fence_preparation() {
    for change in [
        SettingChange::Appearance(lw_core::preferences::Appearance::Light),
        SettingChange::Signal(lw_core::preferences::Signal::Green),
        SettingChange::ToggleReduceMotion,
        SettingChange::ToggleShowNonAudio,
        SettingChange::CacheGB(2),
    ] {
        let command = Command::Setting(change);
        assert!(command.validate().is_ok());
        assert!(!command.fences_preparation());
        assert!(!command.interrupts());
    }
    for change in [
        SettingChange::ToggleDownloadAhead,
        SettingChange::ToggleNoNetwork,
    ] {
        let command = Command::Setting(change);
        assert!(command.fences_preparation());
        assert!(!command.interrupts());
    }
    assert!(
        Command::Setting(SettingChange::CacheGB(3))
            .validate()
            .is_err()
    );
    assert!(!Command::ReadStorage.fences_preparation());
    assert!(!Command::ReadStorage.interrupts());
}
#[test]
fn saving_queue_names_is_bounded_and_does_not_fence_or_interrupt_preparation() {
    for name in ["", "  \t\n"] {
        assert!(
            Command::SaveQueue(lw_ui::browse::Query::new(name).unwrap())
                .validate()
                .is_err()
        );
    }
    let command = Command::SaveQueue(lw_ui::browse::Query::new("  Playlist 🎵  ").unwrap());
    assert!(command.validate().is_ok());
    assert!(!command.fences_preparation());
    assert!(!command.interrupts());
}
fn shared() -> Arc<Shared> {
    Arc::new(Shared {
        #[cfg(target_os = "ios")]
        audio_session: lw_apple_sys::session::Session::open(wake).unwrap(),
        inbox: Mutex::new(Inbox {
            catalog: None,
            presented_catalog: Arc::new(Catalog::default()),
            catalog_pending: false,
            retired_catalog: None,
            commands: VecDeque::new(),
            priority: None,
            browse: None,
            events: VecDeque::with_capacity(128),
            native: Snapshot::default(),
            dirty: false,
            ack: None,
            output: None,
            fault: None,
            status: Status::default(),
            closed: false,
            stop_requested: false,
            stopped: false,
        }),
        changed: Condvar::new(),
        cancelled: AtomicBool::new(false),
        waveform_cancelled: AtomicBool::new(false),
        preparation: AtomicU64::new(0),
        wake,
    })
}
#[test]
fn catalog_readiness_covers_queued_active_and_undelivered_work_without_blocking_controls() {
    let shared = shared();
    let mut inbox = shared.inbox.lock().unwrap();
    assert!(!inbox.has_pending_catalog());
    inbox.commands.push_back(Envelope {
        audio_notice: None,
        revision: 1,
        preparation: 0,
        command: Command::Volume(0.5),
    });
    assert!(!inbox.has_pending_catalog());
    inbox.commands.push_back(Envelope {
        audio_notice: None,
        revision: 2,
        preparation: 0,
        command: Command::RefreshCatalog,
    });
    assert!(inbox.has_pending_catalog());
    inbox.commands.clear();
    inbox.catalog_pending = true;
    assert!(inbox.has_pending_catalog());
    inbox.catalog_pending = false;
    inbox.catalog = Some(Arc::new(Catalog::default()));
    assert!(inbox.has_pending_catalog());
    inbox.catalog = None;
    assert!(!inbox.has_pending_catalog());
}
#[test]
fn continuous_volume_and_seek_delivery_alternates_with_current_and_upcoming_preparation() {
    let base = std::env::temp_dir().join(format!("lw-control-fairness-{}", std::process::id()));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(base.clone());
    let shared = shared();
    shared.preparation.store(1, Ordering::SeqCst);
    shared.inbox.lock().unwrap().native.volume = 1.0;
    let owner = shared.clone();
    let worker_base = base.clone();
    let thread = std::thread::spawn(move || {
        Worker::run(
            move || {
                let documents = worker_base.join("Documents");
                std::fs::create_dir_all(documents.join("Added Files"))?;
                std::fs::write(
                    documents.join("Added Files/track.m4a"),
                    include_bytes!("../../../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a"),
                )?;
                let mut library = LibraryWorker::open(
                    &worker_base.join("library.sqlite"),
                    lw_apple_sys::preferences::Preferences::open(Some(&format!(
                        "localwave.control-fairness.{}",
                        std::process::id()
                    )))?,
                    lw_apple_sys::preferences::AppPaths {
                        documents,
                        caches: worker_base.join("Caches"),
                        temporary: worker_base.join("tmp"),
                        test_database: None,
                    },
                )?;
                for key in 1..=3 {
                    library
                        .repository
                        .upsert_song(
                            &lw_core::Song {
                                song_key: key,
                                path_hash: key,
                                local_file_path: Some("Added Files/track.m4a".into()),
                                file_state: lw_core::FileState::Copied,
                                ..Default::default()
                            },
                            1.0,
                        )
                        .map_err(io::Error::other)?;
                }
                Ok(library)
            },
            owner,
        )
    });
    struct Close(Arc<Shared>, Option<JoinHandle<()>>);
    impl Drop for Close {
        fn drop(&mut self) {
            self.0.inbox.lock().unwrap().closed = true;
            self.0.changed.notify_one();
            self.1.take().unwrap().join().unwrap();
        }
    }
    let _close = Close(shared.clone(), Some(thread));
    shared.inbox.lock().unwrap().commands.push_back(Envelope {
        audio_notice: None,
        revision: 1,
        preparation: 1,
        command: Command::Configure {
            songs: Arc::from([1, 2, 3]),
            start: 0,
            autoplay: true,
            context: None,
        },
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let (mut installs, mut seeks, mut volumes) = (0, 0, 0);
    for step in 0..16 {
        let output = loop {
            if let Some(output) = shared.inbox.lock().unwrap().output.take() {
                break output;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "worker did not produce its next operation"
            );
            std::thread::yield_now();
        };
        let mut native = shared.inbox.lock().unwrap().native;
        match output.operation {
            Operation::Volume(value) => {
                native.volume = value;
                volumes += 1;
            }
            Operation::Seek(seconds) => {
                native.position = seconds;
                seeks += 1;
            }
            Operation::Install {
                ticket,
                item,
                replace,
            } => {
                drop(item);
                if replace {
                    native.queued = [0; 3];
                    native.token = ticket.token;
                    native.occurrence = ticket.entry.id;
                }
                *native.queued.iter_mut().find(|token| **token == 0).unwrap() = ticket.token;
                native.generation = ticket.generation;
                native.status = 1;
                installs += 1;
            }
            Operation::Intent(playing) => {
                native.time_control = if playing { 2 } else { 0 };
            }
            Operation::Reset | Operation::Stop => {
                native.token = 0;
                native.queued = [0; 3];
            }
            _ => panic!("unexpected operation in steady preparation"),
        }
        let mut inbox = shared.inbox.lock().unwrap();
        inbox.native = native;
        inbox.ack = Some(Ack {
            result: Ok(true),
            snapshot: native,
        });
        for (index, command) in [
            Command::Volume(if native.volume == 0.25 { 0.75 } else { 0.25 }),
            Command::Seek(0.0),
            Command::RefreshCatalog,
        ]
        .into_iter()
        .enumerate()
        {
            inbox.commands.push_back(Envelope {
                audio_notice: None,
                revision: 2 + step * 3 + index as u64,
                preparation: 1,
                command,
            });
        }
        drop(inbox);
        shared.changed.notify_one();
        if installs == 3 && seeks >= 2 && volumes >= 2 {
            break;
        }
    }
    assert_eq!(
        installs, 3,
        "continuous controls starved current/upcoming preparation"
    );
    assert!(
        seeks >= 2 && volumes >= 2,
        "control delivery must also keep progressing"
    );
}
#[test]
fn waveform_start_yields_to_controls_and_never_clears_playback_cancellation() {
    let shared = shared();
    shared.cancelled.store(true, Ordering::SeqCst);
    shared.waveform_cancelled.store(true, Ordering::SeqCst);
    assert!(shared.begin_background(0, 0));
    assert!(!shared.waveform_cancelled.load(Ordering::SeqCst));
    assert!(shared.cancelled.load(Ordering::SeqCst));
    shared.waveform_cancelled.store(true, Ordering::SeqCst);
    shared.inbox.lock().unwrap().commands.push_back(Envelope {
        audio_notice: None,
        revision: 1,
        preparation: 0,
        command: Command::Seek(5.0),
    });
    assert!(!shared.begin_background(0, 0));
    assert!(shared.waveform_cancelled.load(Ordering::SeqCst));
    shared.inbox.lock().unwrap().commands.clear();
    assert!(!shared.begin_background(1, 0));
    assert!(!shared.begin_background(0, 1));
    shared.inbox.lock().unwrap().browse = Some((
        2,
        Request {
            commands: None,
            scope: lw_ui::browse::Scope::Find,
            query: lw_ui::browse::Query::default(),
            sort: lw_ui::browse::Sort::Original,
            ascending: true,
        },
    ));
    assert!(!shared.begin_background(0, 0));
    shared.inbox.lock().unwrap().browse = None;
    let status = Status {
        waveform: Some([0.5; 512]),
        ..Status::default()
    };
    mem_sys::local_reset();
    for _ in 0..1000 {
        assert!(shared.begin_background(0, 0));
        assert_eq!(
            std::hint::black_box(status.clone()).waveform,
            status.waveform
        );
    }
    assert_eq!(mem_sys::local_alloc_count(), 0);
}

#[test]
fn unchanged_native_polls_need_no_worker_notification_or_warm_allocation() {
    let shared = shared();
    let mut inbox = shared.inbox.lock().unwrap();
    mem_sys::local_reset();
    for _ in 0..1000 {
        assert!(!inbox.update_native(&[], Snapshot::default(), None).unwrap());
    }
    assert_eq!(mem_sys::local_alloc_count(), 0);
    assert!(!inbox.dirty);
    let next = Snapshot {
        volume: 0.5,
        ..Default::default()
    };
    assert!(inbox.update_native(&[], next, None).unwrap());
    assert!(inbox.dirty);
    inbox.dirty = false;
    assert!(!inbox.update_native(&[], next, None).unwrap());
    assert!(
        inbox.update_native(&[None], next, None).unwrap(),
        "a delivered batch must wake the worker"
    );
    assert!(
        inbox
            .update_native(&[], next, Some(io::ErrorKind::Other.into()))
            .unwrap()
    );
    assert!(inbox.fault.is_some());
    inbox.fault = None;
    assert!(
        inbox
            .update_native(
                &[None; 129],
                Snapshot::default(),
                Some(io::ErrorKind::Other.into())
            )
            .is_err()
    );
    assert_eq!(
        inbox.native, next,
        "overflow must refuse the whole delivery"
    );
    assert!(inbox.fault.is_none());
}

#[test]
fn one_poll_exchange_delivers_bounded_replies_and_preserves_newer_native_readings() {
    let shared = shared();
    let current = shared.inbox.lock().unwrap().presented_catalog.clone();
    let mut feedback = Feedback::default();
    shared.changed.notify_one();
    mem_sys::local_reset();
    for _ in 0..20 {
        let poll = shared
            .exchange(
                &mut feedback,
                &[],
                Snapshot::default(),
                None,
                true,
                &current,
                0,
            )
            .unwrap();
        assert!(!poll.worker_changed);
        assert!(poll.output.is_none() && poll.catalog.is_none() && poll.status.is_none());
        assert!(!feedback.pending());
        assert!(shared.inbox.try_lock().is_ok());
    }
    assert_eq!(mem_sys::local_alloc_count(), 0);
    {
        let mut inbox = shared.inbox.lock().unwrap();
        inbox.output = Some(Output {
            preparation: 7,
            operation: Operation::Seek(4.0),
        });
        inbox.status.revision = 1;
    }
    let poll = shared
        .exchange(
            &mut feedback,
            &[],
            Snapshot::default(),
            None,
            true,
            &current,
            0,
        )
        .unwrap();
    assert_eq!(poll.output.unwrap().preparation, 7);
    assert_eq!(poll.status.unwrap().revision, 1);
    assert!(shared.inbox.lock().unwrap().ack.is_none());
    feedback.ack = Some(Ack {
        result: Ok(true),
        snapshot: Snapshot {
            position: 4.0,
            ..Default::default()
        },
    });
    let now = Snapshot {
        position: 4.5,
        ..Default::default()
    };
    assert!(
        shared
            .exchange(&mut feedback, &[None; 129], now, None, true, &current, 1)
            .is_err()
    );
    assert!(
        feedback.ack.is_some(),
        "a refused event batch must retain the pending acknowledgement"
    );
    assert!(shared.inbox.lock().unwrap().ack.is_none());
    mem_sys::local_reset();
    let poll = shared
        .exchange(&mut feedback, &[], now, None, true, &current, 1)
        .unwrap();
    assert!(poll.worker_changed);
    assert!(!feedback.pending());
    {
        let mut inbox = shared.inbox.lock().unwrap();
        assert_eq!(
            inbox.native.position, 4.5,
            "the later reading must not regress to the acknowledgement"
        );
        assert_eq!(inbox.ack.take().unwrap().snapshot.position, 4.0);
        inbox.dirty = false;
        inbox.stop_requested = true;
    }
    let poll = shared
        .exchange(&mut feedback, &[], now, None, false, &current, 1)
        .unwrap();
    assert!(poll.stop_requested && !poll.stopped);
    assert!(
        !shared.inbox.lock().unwrap().stopped,
        "copying the request cannot acknowledge native stop"
    );
    feedback.stopped = true;
    let poll = shared
        .exchange(&mut feedback, &[], now, None, false, &current, 1)
        .unwrap();
    assert!(poll.stop_requested && poll.stopped && poll.worker_changed);
    let poll = shared
        .exchange(&mut feedback, &[], now, None, false, &current, 1)
        .unwrap();
    assert!(
        poll.stopped && !poll.worker_changed,
        "an acknowledged stop must not cause a wake loop"
    );
    assert_eq!(mem_sys::local_alloc_count(), 0);
}
fn exchange_catalog(
    shared: &Shared,
    feedback: &mut Feedback,
    current: &mut Arc<Catalog>,
    changed: &mut dyn FnMut(&Catalog, &Catalog),
) -> bool {
    let poll = shared
        .exchange(feedback, &[], Snapshot::default(), None, true, current, 0)
        .unwrap();
    assert!(poll.output.is_none());
    if poll.worker_changed {
        shared.changed.notify_one();
    }
    feedback.adopt(current, poll.catalog, changed)
}
#[test]
fn catalog_handoff_is_bounded_and_leaves_old_storage_for_the_worker() {
    let shared = shared();
    let mut feedback = Feedback::default();
    let entries: Arc<[lw_core::queue::Entry]> = (0..10_000)
        .map(|id| lw_core::queue::Entry { id, song_id: 7 })
        .collect();
    let old_entries = Arc::downgrade(&entries);
    let catalog = Catalog {
        queue: Some(Arc::new(lw_ui::queue::Snapshot::new(
            1,
            entries,
            Some(0),
            Arc::new(PlaybackContext::Library),
            false,
        ))),
        ..Catalog::default()
    };
    let mut current = Arc::new(catalog);
    let old = Arc::downgrade(&current);
    shared.inbox.lock().unwrap().presented_catalog = current.clone();
    shared.publish_catalog(&Catalog::new(Vec::new()));
    // Warm the condition variable before measuring exchanges.
    shared.changed.notify_one();
    mem_sys::local_reset();
    assert!(exchange_catalog(
        &shared,
        &mut feedback,
        &mut current,
        &mut |previous, next| {
            assert!(previous.all.is_none());
            assert!(next.all.is_some());
            assert!(
                shared.inbox.try_lock().is_ok(),
                "view-state repair must not hold the mailbox lock"
            );
        }
    ));
    assert_eq!(mem_sys::local_alloc_count(), 0);
    assert_eq!(
        old.strong_count(),
        1,
        "only the bounded return slot keeps the old catalog alive"
    );
    shared.publish_catalog(&Catalog::default());
    assert!(!exchange_catalog(
        &shared,
        &mut feedback,
        &mut current,
        &mut |_, _| panic!("retirement backpressure")
    ));
    assert!(shared.inbox.lock().unwrap().catalog.is_some());
    assert_eq!(
        old_entries.strong_count(),
        1,
        "the queue array stays in the bounded retirement slot"
    );
    let worker = shared.clone();
    std::thread::spawn(move || {
        let retired = worker.inbox.lock().unwrap().retired_catalog.take();
        drop(retired);
        assert!(
            old.upgrade().is_none(),
            "the worker releases the final strong owner"
        );
        assert!(
            old_entries.upgrade().is_none(),
            "the worker releases large queue arrays too"
        );
    })
    .join()
    .unwrap();
    assert!(exchange_catalog(
        &shared,
        &mut feedback,
        &mut current,
        &mut |previous, next| {
            assert!(previous.all.is_some());
            assert!(next.all.is_none());
        }
    ));
    {
        let mut inbox = shared.inbox.lock().unwrap();
        inbox.retired_catalog.take();
        inbox.catalog = Some(Arc::new(Catalog::new(Vec::new())));
        inbox.closed = true;
    }
    assert!(!exchange_catalog(
        &shared,
        &mut feedback,
        &mut current,
        &mut |_, _| panic!("closed mailbox")
    ));
}

#[test]
fn coalesced_catalog_remaps_follow_the_presented_base_even_during_repair() {
    let catalog = |ids: &[i64]| {
        Catalog::new(
            ids.iter()
                .map(|id| lw_core::Song {
                    id: Some(*id),
                    ..Default::default()
                })
                .collect(),
        )
    };
    let shared = shared();
    let mut feedback = Feedback::default();
    let mut current = Arc::new(catalog(&[1, 2]));
    shared.inbox.lock().unwrap().presented_catalog = current.clone();
    let mut state = lw_ui::ShellState::default();
    state.selected = Some(1);
    shared.publish_catalog(&catalog(&[2, 1, 0]));
    let mut latest = catalog(&[0, 1, 2]);
    shared.publish_catalog(&latest);
    let remap = shared
        .inbox
        .lock()
        .unwrap()
        .catalog
        .as_ref()
        .unwrap()
        .remap
        .clone()
        .unwrap();
    latest.source_version = Some(42);
    shared.publish_catalog(&latest);
    assert!(
        Arc::ptr_eq(
            &remap,
            shared
                .inbox
                .lock()
                .unwrap()
                .catalog
                .as_ref()
                .unwrap()
                .remap
                .as_ref()
                .unwrap()
        ),
        "unchanged page publications reuse the prepared remap while the UI is behind"
    );
    assert!(exchange_catalog(
        &shared,
        &mut feedback,
        &mut current,
        &mut |previous, next| {
            state.refresh(previous, next);
            assert_eq!(state.selected, Some(2));
            assert!(shared.inbox.try_lock().is_ok());
            // Deterministic worker publication while UI repair still borrows the previous page.
            shared.publish_catalog(&catalog(&[2, 1]));
        }
    ));
    {
        let inbox = shared.inbox.lock().unwrap();
        let pending = inbox.catalog.as_ref().unwrap();
        assert!(pending.remap.as_ref().unwrap().matches(&current, pending));
        assert!(inbox.retired_catalog.is_none());
        assert!(feedback.retired_catalog.is_some());
    }
    assert!(!exchange_catalog(
        &shared,
        &mut feedback,
        &mut current,
        &mut |_, _| panic!("retirement backpressure")
    ));
    let retired = shared.inbox.lock().unwrap().retired_catalog.take();
    drop(retired);
    assert!(exchange_catalog(
        &shared,
        &mut feedback,
        &mut current,
        &mut |previous, next| { state.refresh(previous, next) }
    ));
    assert_eq!(state.selected, Some(0));
    assert_eq!(
        current
            .collection(lw_ui::catalog::Route::Songs)
            .unwrap()
            .ids[0],
        2
    );
}
#[test]
fn offline_commands_copy_on_the_library_worker_preserve_playback_and_release_pins() {
    use lw_core::{FileState, Song};
    let base = std::env::temp_dir().join(format!("lw-offline-command-{}", std::process::id()));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(base.clone());
    let shared = shared();
    shared.inbox.lock().unwrap().native.volume = 1.0;
    let worker_base = base.clone();
    let owner = shared.clone();
    let thread = std::thread::spawn(move || {
        Worker::run(
            move || {
                std::fs::create_dir_all(&worker_base)?;
                let original = worker_base.join("original.flac");
                std::fs::write(&original, [7; 64])?;
                let mut library = LibraryWorker::open(
                    &worker_base.join("library.sqlite"),
                    lw_apple_sys::preferences::Preferences::open(Some(&format!(
                        "localwave.offline-command.{}",
                        std::process::id()
                    )))?,
                    lw_apple_sys::preferences::AppPaths {
                        documents: worker_base.join("Documents"),
                        caches: worker_base.join("Caches"),
                        temporary: worker_base.join("tmp"),
                        test_database: None,
                    },
                )?;
                for id in 1..=3 {
                    library
                        .repository
                        .upsert_song(
                            &Song {
                                song_key: id,
                                title: format!("Track {id}"),
                                file_state: if id == 3 {
                                    FileState::CopyPending
                                } else {
                                    FileState::BookmarkOnly
                                },
                                bookmark: Some(lw_apple_sys::bookmarks::create(&original)?),
                                ..Default::default()
                            },
                            1.0,
                        )
                        .map_err(io::Error::other)?;
                }
                Ok(library)
            },
            owner,
        )
    });
    struct Close(Arc<Shared>, Option<JoinHandle<()>>);
    impl Drop for Close {
        fn drop(&mut self) {
            self.0.waveform_cancelled.store(true, Ordering::SeqCst);
            self.0.cancelled.store(true, Ordering::SeqCst);
            self.0.inbox.lock().unwrap().closed = true;
            self.0.changed.notify_one();
            self.1.take().unwrap().join().unwrap();
        }
    }
    let _close = Close(shared.clone(), Some(thread));
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    loop {
        let resumed = shared
            .inbox
            .lock()
            .unwrap()
            .catalog
            .as_ref()
            .is_some_and(|catalog| {
                catalog
                    .row_index(3)
                    .is_some_and(|index| catalog.row(index).state == FileState::Copied)
                    && catalog
                        .row_index(1)
                        .is_some_and(|index| catalog.row(index).state == FileState::BookmarkOnly)
            });
        if resumed {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "persisted copy requests resume on startup"
        );
        std::thread::yield_now();
    }
    for (revision, ids, pinned) in [
        (1, Arc::<[i64]>::from([1, 2, 1]), true),
        (2, Arc::from([1, 2]), false),
        (3, Arc::from([1, 9000]), false),
    ] {
        let command = Command::TogglePins(ids);
        assert!(command.validate().is_ok());
        assert!(!command.interrupts() && !command.fences_preparation());
        shared.inbox.lock().unwrap().commands.push_back(Envelope {
            audio_notice: None,
            revision,
            preparation: 0,
            command,
        });
        shared.waveform_cancelled.store(true, Ordering::SeqCst);
        shared.changed.notify_one();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        loop {
            let done = {
                let inbox = shared.inbox.lock().unwrap();
                assert!(
                    inbox.output.is_none(),
                    "offline jobs cannot start or pause native playback"
                );
                if revision == 3 {
                    inbox.status.processed_command >= revision && !inbox.status.message.is_empty()
                } else {
                    inbox.catalog.as_ref().is_some_and(|catalog| {
                        catalog.all.as_ref().is_some_and(|all| {
                            all.rows.len() == 3
                                && all.rows.iter().all(|index| {
                                    let row = catalog.row(*index);
                                    row.state == FileState::Copied
                                        && row.pinned == (pinned && row.id != 3)
                                })
                        })
                    })
                }
            };
            if done {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "offline command {revision} did not finish"
            );
            std::thread::yield_now();
        }
    }
    let inbox = shared.inbox.lock().unwrap();
    assert_eq!(inbox.native.token, 0);
    assert_eq!(inbox.status.queue_len, 0);
    let catalog = inbox.catalog.as_ref().unwrap();
    assert!(
        catalog
            .all
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .all(|index| !catalog.row(*index).pinned)
    );
    assert_eq!(std::fs::read(base.join("original.flac")).unwrap(), [7; 64]);
    let files = std::fs::read_dir(base.join("Documents/Music"))
        .unwrap()
        .count();
    assert_eq!(files, 1, "identical content shares a managed copy");
}

#[test]
fn set_favorite_persists_on_the_library_worker_and_refreshes_the_catalog() {
    use lw_core::Song;
    assert!(
        Command::SetFavorite { song: 0, on: true }
            .validate()
            .is_err()
    );
    let base = std::env::temp_dir().join(format!("lw-favorite-command-{}", std::process::id()));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(base.clone());
    let shared = shared();
    shared.inbox.lock().unwrap().native.volume = 1.0;
    let (worker_base, owner) = (base.clone(), shared.clone());
    let thread = std::thread::spawn(move || {
        Worker::run(
            move || {
                std::fs::create_dir_all(&worker_base)?;
                let mut library = LibraryWorker::open(
                    &worker_base.join("library.sqlite"),
                    lw_apple_sys::preferences::Preferences::open(Some(&format!(
                        "localwave.favorite-command.{}",
                        std::process::id()
                    )))?,
                    lw_apple_sys::preferences::AppPaths {
                        documents: worker_base.join("Documents"),
                        caches: worker_base.join("Caches"),
                        temporary: worker_base.join("tmp"),
                        test_database: None,
                    },
                )?;
                library
                    .repository
                    .upsert_song(
                        &Song {
                            song_key: 1,
                            title: "Track 1".into(),
                            ..Default::default()
                        },
                        1.0,
                    )
                    .map_err(io::Error::other)?;
                Ok(library)
            },
            owner,
        )
    });
    struct Close(Arc<Shared>, Option<JoinHandle<()>>);
    impl Drop for Close {
        fn drop(&mut self) {
            self.0.inbox.lock().unwrap().closed = true;
            self.0.changed.notify_one();
            self.1.take().unwrap().join().unwrap();
        }
    }
    let _close = Close(shared.clone(), Some(thread));
    let favorite = |shared: &Shared| {
        shared
            .inbox
            .lock()
            .unwrap()
            .catalog
            .as_ref()
            .and_then(|catalog| {
                catalog
                    .row_index(1)
                    .map(|index| catalog.row(index).favorite)
            })
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let wait = |done: &dyn Fn() -> bool| {
        while !done() {
            assert!(
                std::time::Instant::now() < deadline,
                "favorite command stalled"
            );
            std::thread::yield_now();
        }
    };
    for (revision, song, on) in [(0, 0, false), (1, 1, true), (2, 1, false), (3, 9000, true)] {
        let command = if revision == 0 {
            Command::RefreshCatalog
        } else {
            Command::SetFavorite { song, on }
        };
        assert!(command.validate().is_ok() && command.changes_catalog());
        assert!(!command.interrupts() && !command.fences_preparation());
        shared.inbox.lock().unwrap().commands.push_back(Envelope {
            audio_notice: None,
            revision,
            preparation: 0,
            command,
        });
        shared.waveform_cancelled.store(true, Ordering::SeqCst);
        shared.changed.notify_one();
        match revision {
            0 => wait(&|| favorite(&shared) == Some(false)),
            1 => wait(&|| favorite(&shared) == Some(true)),
            2 => wait(&|| favorite(&shared) == Some(false)),
            _ => wait(&|| {
                let inbox = shared.inbox.lock().unwrap();
                inbox.status.processed_command >= revision && !inbox.status.message.is_empty()
            }),
        }
    }
}
