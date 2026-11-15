use lw_apple_sys::session::Event as AudioEvent;
use lw_core::context::PlaybackContext;
use lw_core::playback::{Fault, Phase};
use lw_core::preferences::{Change as SettingChange, Settings};
use lw_core::queue::{Entry, RepeatMode};
use std::io;
use std::sync::Arc;

// Inline name packets keep delivery through the bounded mailbox allocation-free.
#[allow(clippy::large_enum_variant)]
pub enum Command {
    Configure {
        songs: Arc<[i64]>,
        start: usize,
        autoplay: bool,
        context: Option<Arc<PlaybackContext>>,
    },
    ShuffleAll {
        songs: Arc<[i64]>,
        seed: u64,
    },
    Select {
        occurrence: u64,
        autoplay: bool,
    },
    Play,
    AudioSession(AudioEvent),
    Toggle,
    Pause,
    Stop,
    Next,
    Previous,
    Volume(f32),
    Seek(f64),
    Repeat(RepeatMode),
    Shuffle {
        enabled: bool,
        seed: u64,
    },
    Reorder {
        indices: Arc<[usize]>,
        destination: usize,
    },
    MoveUpcoming {
        occurrence: u64,
        before: Option<u64>,
    },
    MoveEntry {
        occurrence: u64,
        before: Option<u64>,
    },
    Remove(u64),
    Append(i64),
    ClearUpcoming,
    UndoClear,
    SaveQueue(lw_ui::browse::Query),
    DismissNotice,
    RefreshCatalog,
    Setting(SettingChange),
    ReadStorage,
    TogglePins(Arc<[i64]>),
}
impl Command {
    pub(super) fn changes_catalog(&self) -> bool {
        matches!(
            self,
            Self::RefreshCatalog | Self::SaveQueue(_) | Self::TogglePins(_)
        )
    }
    pub(super) fn validate(&self) -> io::Result<()> {
        let invalid = match self {
            Self::Configure {
                songs,
                start,
                context,
                ..
            } => {
                songs.len() > 1_000_000
                    || (!songs.is_empty() && *start >= songs.len())
                    || (songs.is_empty() && *start != 0)
                    || context
                        .as_ref()
                        .is_some_and(|context| context.text_len() > (64 * 1024 * 1024 - 1024) / 6)
            }
            Self::ShuffleAll { songs, .. } => songs.is_empty() || songs.len() > 1_000_000,
            Self::Volume(volume) => !volume.is_finite() || !(0.0..=1.0).contains(volume),
            Self::Seek(seconds) => !seconds.is_finite(),
            Self::Reorder { indices, .. } => indices.len() > 1_000_000,
            Self::TogglePins(songs) => songs.is_empty() || songs.len() > 1_000_000,
            Self::SaveQueue(name) => name.as_str().trim().is_empty(),
            Self::Setting(change) => Settings::default().applying(*change).is_err(),
            _ => false,
        };
        if invalid {
            Err(io::ErrorKind::InvalidInput.into())
        } else {
            Ok(())
        }
    }
    pub(super) fn fences_preparation(&self) -> bool {
        !matches!(
            self,
            Self::Volume(_)
                | Self::Seek(_)
                | Self::DismissNotice
                | Self::RefreshCatalog
                | Self::SaveQueue(_)
                | Self::ReadStorage
                | Self::TogglePins(_)
                | Self::Setting(
                    SettingChange::Appearance(_)
                        | SettingChange::Signal(_)
                        | SettingChange::ToggleReduceMotion
                        | SettingChange::ToggleShowNonAudio
                        | SettingChange::CacheGB(_)
                )
        )
    }
    pub(super) fn interrupts(&self) -> bool {
        matches!(
            self,
            Self::Configure { .. }
                | Self::ShuffleAll { .. }
                | Self::Select { .. }
                | Self::Pause
                | Self::AudioSession(
                    AudioEvent::Began
                        | AudioEvent::OutputUnavailable
                        | AudioEvent::Overflow
                        | AudioEvent::Reset
                )
                | Self::Toggle
                | Self::Stop
                | Self::Next
                | Self::Previous
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub settings: Settings,
    pub cache_usage: Arc<str>,
    pub settings_message: Arc<str>,
    pub storage_busy: bool,
    pub waveform: Option<lw_core::waveform::Peaks>,
    pub revision: u64,
    pub initialized: bool,
    pub processed_command: u64,
    pub generation: u64,
    pub current: Option<Entry>,
    pub queue_len: usize,
    pub current_index: Option<usize>,
    pub phase: Phase,
    pub wants_playback: bool,
    pub fault: Option<Fault>,
    pub repeat: RepeatMode,
    pub shuffled: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f32,
    pub title: Arc<str>,
    pub artist: Arc<str>,
    pub album: Arc<str>,
    pub context: Arc<PlaybackContext>,
    pub message: Arc<str>,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            cache_usage: Arc::from(""),
            settings_message: Arc::from(""),
            storage_busy: false,
            waveform: None,
            revision: 0,
            context: Arc::new(PlaybackContext::Library),
            initialized: false,
            processed_command: 0,
            generation: 0,
            current: None,
            queue_len: 0,
            current_index: None,
            phase: Phase::Stopped,
            wants_playback: false,
            fault: None,
            repeat: RepeatMode::None,
            shuffled: false,
            position: 0.0,
            duration: 0.0,
            volume: 1.0,
            title: Arc::from(""),
            artist: Arc::from(""),
            album: Arc::from(""),
            message: Arc::from(""),
        }
    }
}
