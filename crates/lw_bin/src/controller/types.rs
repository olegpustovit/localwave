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
        context: Option<Arc<PlaybackContext>>,
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
    Append(Arc<[i64]>),
    ClearUpcoming,
    UndoClear,
    SaveQueue(lw_ui::browse::Query),
    DismissNotice,
    RefreshCatalog,
    Setting(SettingChange),
    ReadStorage,
    TogglePins(Arc<[i64]>),
    SetFavorite {
        song: i64,
        on: bool,
    },
    /// Swift's queueNext: append occurrences, then move them after the current one.
    InsertNext(Arc<[i64]>),
    AddToList(i64, Arc<[i64]>),
    RemoveFromList(i64, Arc<[i64]>),
    /// A new list, holding `songs` when given (the track menu's New list...).
    CreateList(lw_ui::browse::Query, Arc<[i64]>),
    RenameList(i64, lw_ui::browse::Query),
    /// Playback that came from the list continues and falls back to the Library context.
    DeleteList(i64),
    ReorderList(i64, Arc<[i64]>),
    RetryDownload(i64),
    /// Metadata editor save (song row and FTS5 in one transaction).
    EditMetadata(Box<lw_ui::MetadataEdit>),
    /// Remove the library entry only; the original file and any current native item stay.
    DeleteSong(i64),
    /// Show the song's file in Finder through its source grant.
    Reveal(i64),
    /// Copy the songs' files under their source grants, then show the share picker at the anchor.
    Share(Arc<[i64]>, (f32, f32)),
}
impl Command {
    pub(super) fn changes_catalog(&self) -> bool {
        matches!(
            self,
            Self::RefreshCatalog
                | Self::SaveQueue(_)
                | Self::TogglePins(_)
                | Self::SetFavorite { .. }
                | Self::AddToList(..)
                | Self::RemoveFromList(..)
                | Self::CreateList(..)
                | Self::RenameList(..)
                | Self::DeleteList(_)
                | Self::ReorderList(..)
                | Self::RetryDownload(_)
                | Self::EditMetadata(_)
                | Self::DeleteSong(_)
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
            Self::SetFavorite { song, .. } => *song <= 0,
            Self::Append(songs) | Self::InsertNext(songs) => bad_ids(songs),
            Self::AddToList(list, songs) | Self::RemoveFromList(list, songs) => {
                *list <= 0 || bad_ids(songs)
            }
            Self::CreateList(name, songs) => {
                name.as_str().trim().is_empty() || (!songs.is_empty() && bad_ids(songs))
            }
            Self::RenameList(list, name) => *list <= 0 || name.as_str().trim().is_empty(),
            Self::DeleteList(list) => *list <= 0,
            Self::ReorderList(list, order) => *list <= 0 || bad_ids(order),
            Self::RetryDownload(song) | Self::Reveal(song) | Self::DeleteSong(song) => *song <= 0,
            Self::EditMetadata(edit) => !edit.valid(),
            Self::Share(songs, anchor) => {
                bad_ids(songs) || !anchor.0.is_finite() || !anchor.1.is_finite()
            }
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
                | Self::SetFavorite { .. }
                | Self::AddToList(..)
                | Self::RemoveFromList(..)
                | Self::CreateList(..)
                | Self::RenameList(..)
                | Self::DeleteList(_)
                | Self::ReorderList(..)
                | Self::RetryDownload(_)
                | Self::EditMetadata(_)
                | Self::DeleteSong(_)
                | Self::Reveal(_)
                | Self::Share(..)
                | Self::Setting(
                    SettingChange::Appearance(_)
                        | SettingChange::Signal(_)
                        | SettingChange::ToggleReduceMotion
                        | SettingChange::ToggleShowNonAudio
                        | SettingChange::DidStart
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

fn bad_ids(songs: &[i64]) -> bool {
    songs.is_empty() || songs.len() > 1_000_000 || songs.iter().any(|song| *song <= 0)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub settings: Settings,
    pub cache_usage: Arc<str>,
    pub settings_message: Arc<str>,
    pub storage_busy: bool,
    pub sharing: bool,
    pub waveform: Option<lw_core::waveform::Peaks>,
    pub revision: u64,
    pub stable_revision: u64,
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
    pub info: Option<Arc<lw_ui::now_info::NowInfo>>,
    pub context: Arc<PlaybackContext>,
    /// The queue sheet's "From <context>[ · shuffled]" line, formatted here when the context or shuffle changes.
    pub queue_from: Arc<str>,
    pub message: Arc<str>,
}
impl Status {
    /// Position ticks publish to the controller without invalidating stable UI data.
    pub(super) fn update(&mut self, mut next: Self) -> bool {
        next.revision = self.revision;
        next.stable_revision = self.stable_revision;
        let position_changed = self.position != next.position;
        self.position = next.position;
        let stable_changed = *self != next;
        if stable_changed || position_changed {
            next.revision += 1;
            next.stable_revision += u64::from(stable_changed);
            *self = next;
            true
        } else {
            false
        }
    }
}
impl Default for Status {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            cache_usage: Arc::from(""),
            settings_message: Arc::from(""),
            storage_busy: false,
            sharing: false,
            waveform: None,
            revision: 0,
            stable_revision: 0,
            context: Arc::new(PlaybackContext::Library),
            queue_from: Arc::from("From Library"),
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
            info: None,
            message: Arc::from(""),
        }
    }
}
