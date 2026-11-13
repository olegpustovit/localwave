#[test]
fn native_main_thread_queue_preloads_refills_and_polls_without_rust_allocations() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_localwave-playback"))
        .arg("--smoke-test")
        .args([&fixture, &fixture, &fixture, &fixture])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("initial_queue=[1, 2, 3]"), "{text}");
    assert!(
        text.contains("native_queue_smoke=ok rust_poll_allocations=0"),
        "{text}"
    );
}

#[test]
fn library_bookmarks_materialization_failure_skip_and_native_queue_share_cache_leases() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_localwave-playback"))
        .arg("--library-smoke-test")
        .arg(fixture)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("library_native_smoke=ok")
    );
}

#[test]
fn actual_short_audio_items_report_one_native_start_per_physical_token() {
    // Real AVQueuePlayer startup and its audio clock require an integration process, bounded by cargo t.
    let base = std::env::temp_dir().join(format!("lw-native-starts-{}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(base.clone());
    let file = base.join("short.wav");
    let length = 1280u32;
    let mut wave = Vec::from(*b"RIFF");
    wave.extend((length + 36).to_le_bytes());
    wave.extend(*b"WAVEfmt \x10\0\0\0\x01\0\x01\0\x40\x1F\0\0\x80\x3E\0\0\x02\0\x10\0data");
    wave.extend(length.to_le_bytes());
    wave.resize(wave.len() + length as usize, 0);
    std::fs::write(&file, wave).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_localwave-playback"))
        .arg("--mute")
        .args([&file, &file, &file])
        .output()
        .unwrap();
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        output.status.success(),
        "{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for token in 1..=3 {
        assert_eq!(
            text.matches(&format!("started token={token} occurrence={token}"))
                .count(),
            1,
            "{text}"
        );
    }
    assert!(text.contains("completed=3"), "{text}");
    assert!(text.contains("starts=3"), "{text}");
}

#[test]
fn production_controller_restores_skips_reconciles_persists_and_retires_native_items() {
    // AVQueuePlayer startup/completion use a real audio clock and main-thread process, bounded by cargo t.
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_localwave-playback"))
        .arg("--controller-smoke-test")
        .arg(fixture)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("controller_smoke=ok")
    );
}
