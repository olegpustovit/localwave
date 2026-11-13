//! Persisted values and queue restoration policy; the application supplies native storage.
use crate::{LibraryRepository, Song};
use lw_apple_sys::sqlite::Result;
use std::io;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Signal {
    #[default]
    Blue,
    Red,
    Green,
    Mono,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Settings {
    pub download_ahead: bool,
    pub show_non_audio: bool,
    pub simulate_no_network: bool,
    pub appearance: Appearance,
    pub signal: Signal,
    pub reduce_motion: bool,
    pub cache_gb: i64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            download_ahead: true,
            show_non_audio: false,
            simulate_no_network: false,
            appearance: Appearance::System,
            signal: Signal::Blue,
            reduce_motion: false,
            cache_gb: 8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    ToggleDownloadAhead,
    ToggleShowNonAudio,
    ToggleNoNetwork,
    Appearance(Appearance),
    Signal(Signal),
    ToggleReduceMotion,
    CacheGB(i64),
}
impl Settings {
    pub fn applying(mut self, change: Change) -> io::Result<Self> {
        match change {
            Change::ToggleDownloadAhead => self.download_ahead = !self.download_ahead,
            Change::ToggleShowNonAudio => self.show_non_audio = !self.show_non_audio,
            Change::ToggleNoNetwork => self.simulate_no_network = !self.simulate_no_network,
            Change::Appearance(value) => self.appearance = value,
            Change::Signal(value) => self.signal = value,
            Change::ToggleReduceMotion => self.reduce_motion = !self.reduce_motion,
            Change::CacheGB(value) if [2, 4, 8, 16, 32].contains(&value) => self.cache_gb = value,
            Change::CacheGB(_) => return Err(io::ErrorKind::InvalidInput.into()),
        }
        Ok(self)
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerState {
    pub song_ids: Vec<i64>,
    pub current_index: i64,
    pub volume: Option<f32>,
    /// Preserve Swift's Codable payload until the playback context changes.
    pub context_data: Option<Vec<u8>>,
}
impl PlayerState {
    pub fn validate(&self) -> io::Result<()> {
        if self.song_ids.len() > 1_000_000
            || self.current_index < 0
            || (!self.song_ids.is_empty() && self.current_index as usize >= self.song_ids.len())
            || (self.song_ids.is_empty() && self.current_index != 0)
            || self
                .volume
                .is_some_and(|volume| !volume.is_finite() || !(0.0..=1.0).contains(&volume))
            || self
                .context_data
                .as_ref()
                .is_some_and(|data| data.len() > 64 * 1024 * 1024)
        {
            return Err(io::Error::other("Invalid persisted player state"));
        }
        Ok(())
    }
}
pub struct RestoredQueue {
    pub songs: Vec<Song>,
    pub current_index: Option<usize>,
}
impl LibraryRepository {
    pub fn restore_queue(&self, state: &PlayerState) -> Result<RestoredQueue> {
        let songs = self.get_songs(&state.song_ids)?;
        let mut restored_index = None;
        let mut next = 0;
        for (index, id) in state.song_ids.iter().enumerate() {
            if songs.get(next).is_some_and(|song| song.id == Some(*id)) {
                if i64::try_from(index).ok() == Some(state.current_index) {
                    restored_index = Some(next);
                }
                next += 1;
            }
        }
        // Keep the selected occurrence when deleted songs precede it; a missing selection falls back.
        let current_index = (!songs.is_empty()).then_some(restored_index.unwrap_or(0));
        Ok(RestoredQueue {
            songs,
            current_index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_deltas_preserve_other_values_and_reject_unsupported_cache_limits() {
        let mut settings = Settings::default();
        for change in [
            Change::ToggleDownloadAhead,
            Change::ToggleShowNonAudio,
            Change::ToggleNoNetwork,
            Change::Appearance(Appearance::Dark),
            Change::Signal(Signal::Green),
            Change::ToggleReduceMotion,
            Change::CacheGB(32),
        ] {
            settings = settings.applying(change).unwrap();
        }
        assert_eq!(
            settings,
            Settings {
                download_ahead: false,
                show_non_audio: true,
                simulate_no_network: true,
                appearance: Appearance::Dark,
                signal: Signal::Green,
                reduce_motion: true,
                cache_gb: 32
            }
        );
        let unchanged = settings;
        for value in [i64::MIN, -1, 0, 1, 3, 6, 64, i64::MAX] {
            assert!(settings.applying(Change::CacheGB(value)).is_err());
        }
        assert_eq!(settings, unchanged);
        assert_eq!(
            settings
                .applying(Change::ToggleDownloadAhead)
                .unwrap()
                .applying(Change::ToggleDownloadAhead)
                .unwrap(),
            settings
        );
    }
    #[test]
    fn restore_keeps_duplicate_occurrences_after_deletions_and_rejects_invalid_writes() {
        let mut repo = LibraryRepository::open(":memory:").unwrap();
        let a = repo
            .upsert_song(
                &Song {
                    song_key: 1,
                    title: "One".into(),
                    ..Song::default()
                },
                0.0,
            )
            .unwrap()
            .id
            .unwrap();
        let b = repo
            .upsert_song(
                &Song {
                    song_key: 2,
                    title: "Two".into(),
                    ..Song::default()
                },
                0.0,
            )
            .unwrap()
            .id
            .unwrap();
        let mut state = PlayerState {
            song_ids: vec![-1, a, a, b],
            current_index: 2,
            volume: Some(0.5),
            context_data: None,
        };
        let restored = repo.restore_queue(&state).unwrap();
        assert_eq!(restored.current_index, Some(1));
        assert_eq!(
            restored
                .songs
                .iter()
                .map(|song| song.id.unwrap())
                .collect::<Vec<_>>(),
            [a, a, b]
        );
        state.current_index = 0;
        assert_eq!(repo.restore_queue(&state).unwrap().current_index, Some(0));
        state.song_ids.clear();
        assert_eq!(repo.restore_queue(&state).unwrap().current_index, None);
        state.volume = Some(f32::NAN);
        assert!(state.validate().is_err());
        state.volume = Some(1.01);
        assert!(state.validate().is_err());
        state.volume = None;
        state.current_index = -1;
        assert!(state.validate().is_err());
        state.current_index = 0;
        assert!(state.validate().is_ok());
        assert_eq!(Settings::default().cache_gb, 8);
        assert!(Settings::default().download_ahead);
    }
}
