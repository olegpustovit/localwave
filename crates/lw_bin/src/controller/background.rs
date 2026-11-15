//! Background cache/waveform work yields to playback preparation and controls.
use super::*;

impl Worker {
    pub(super) fn load_storage(&mut self) {
        if !std::mem::take(&mut self.storage_pending) {
            return;
        }
        match self.library.enforce_cache_limit() {
            Ok(eviction) => {
                if eviction.files != 0 {
                    self.dirty_catalog();
                }
                let bytes = eviction.remaining_bytes;
                let usage = if bytes >= 1_000_000_000 {
                    format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
                } else if bytes >= 1_000_000 {
                    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
                } else if bytes >= 1_000 {
                    format!("{:.1} KB", bytes as f64 / 1_000.0)
                } else {
                    format!("{bytes} bytes")
                };
                let summary = format!(
                    "{usage} cached. Kept songs may exceed the limit; Added Files are never removed."
                );
                if self.cache_usage.as_ref() != summary {
                    self.cache_usage = Arc::from(summary);
                }
                self.settings_message = self.empty.clone();
            }
            Err(error) => {
                eprintln!("LocalWave offline space: {error}");
                self.settings_message =
                    Arc::from("Could not read or trim offline space. Try again.");
            }
        }
    }
    pub(super) fn load_copies(&mut self) -> bool {
        if !self.copies_pending
            || !self
                .shared
                .begin_background(self.preparation, self.native.token)
        {
            return false;
        }
        match self.library.process_copy_queue(
            now(),
            &self.shared.waveform_cancelled,
            &mut |_, _| {},
        ) {
            Ok(summary) => {
                self.copies_pending = false;
                if !summary.failed.is_empty() {
                    self.message = Arc::from(
                        "Some songs could not be kept offline. Reconnect the source and try again.",
                    );
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                eprintln!("LocalWave offline copies: {error}");
                self.copies_pending = false;
                self.message = Arc::from("Could not finish keeping songs offline. Try again.");
            }
        }
        self.dirty_catalog();
        true
    }
    pub(super) fn load_waveform(&mut self) {
        let Some(song) = self.session.queue().current().map(|entry| entry.song_id) else {
            return;
        };
        let key = (song, self.native.token);
        if self.waveform_key == Some(key) || !self.shared.begin_background(self.preparation, key.1)
        {
            return;
        }
        let result = self
            .library
            .waveform_peaks(song, now(), &self.shared.waveform_cancelled);
        if self.shared.waveform_cancelled.load(Ordering::SeqCst) {
            return;
        }
        self.waveform_key = Some(key);
        self.waveform = match result {
            Ok(peaks) => peaks,
            Err(error) => {
                eprintln!("LocalWave waveform: {error}");
                None
            }
        };
    }
}
