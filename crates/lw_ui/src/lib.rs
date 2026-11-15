//! LocalWave's pxui views. Application state changes are returned as intents.
pub mod artwork;
pub mod browse;
pub mod catalog;
pub mod commands;
pub mod files;
mod icons;
pub mod motion;
pub mod queue;
mod shell;
pub mod song_text;
pub use shell::{ShellState, shell};

pub const FONT: &[u8] = include_bytes!("../../../localwave/Fonts/HostGrotesk.ttf");
pub const MONO: &[u8] = include_bytes!("../../../localwave/Fonts/GeistMono.ttf");
pub const LIGHT: &str = include_str!("../themes/light.theme");
pub const DARK: &str = include_str!("../themes/dark.theme");

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsMenu {
    Cache,
    Appearance,
}

/// Multi-selection edits over row positions of the displayed collection (its occurrences).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Selection {
    Toggle(usize),
    /// Replace the selection with the span from the anchor to this row in the displayed order.
    Range(usize),
    /// Every row of a collection with this many rows.
    All(usize),
    Clear,
    Move {
        delta: isize,
        extend: bool,
        len: usize,
    },
}

// Fixed UTF-8 packets avoid heap allocations when handing searches or playlist names to the worker.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Intent {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Navigate(catalog::Route),
    SelectRow(usize),
    Select(Selection),
    PlayRow(usize),
    AppendSong(i64),
    SetFavorite {
        song: i64,
        on: bool,
    },
    PlayNext(i64),
    AddToList {
        song: i64,
        list: i64,
    },
    RemoveFromList {
        song: i64,
        list: i64,
    },
    KeepOffline(i64),
    RetryDownload(i64),
    RevealSong(i64),
    /// Swap the open track menu for its playlist choices.
    SongMenuLists,
    TogglePins,
    SongMenu(usize, i64, u64, (f32, f32)),
    CloseSongMenu,
    TogglePanel,
    TogglePlayer,
    Shuffle,
    ShuffleAll,
    /// Shuffle the displayed album or artist, keeping its playback context.
    ShuffleCollection,
    SortAlbums(browse::AlbumSort),
    Repeat,
    SeekFraction(f64),
    Volume(f32),
    RefreshLibrary,
    Search(browse::Query),
    SearchTooLong,
    ShowCommands,
    CloseCommands,
    CommandQuery(browse::Query),
    CommandTooLong,
    CommandSelect(usize),
    ActivateCommand(u64),
    PlayCommand(u64),
    ShowShortcuts,
    CloseShortcuts,
    Sort(browse::Sort),
    PlayQueue(u64),
    RemoveQueue(u64),
    MoveQueue {
        occurrence: u64,
        before: Option<u64>,
    },
    MoveFullQueue {
        occurrence: u64,
        before: Option<u64>,
    },
    ClearQueue,
    UndoQueue,
    ShowQueue,
    CloseQueue,
    NamePlaylist,
    EditPlaylistName(browse::Query),
    PlaylistNameTooLong,
    CancelPlaylistName,
    SaveQueue(browse::Query),
    ShowSettings,
    CloseSettings,
    ShowPrivacy,
    ClosePrivacy,
    SettingsMenu(SettingsMenu, (f32, f32)),
    CloseSettingsMenu,
    Setting(lw_core::preferences::Change),
    PrivacyPolicy,
    Support,
    Files(files::Request),
    ChooseFolder,
    ChooseAudio,
    SourceMenu(files::Target, (f32, f32)),
    CloseSourceMenu,
    ReconnectSource(files::Target),
    ConfirmRemoveSource(files::Target),
    CloseRemoveSource,
}

/// Stable player data. Advance revision when these facts change; position is a separate shell input.
#[derive(Default)]
pub struct PlayerView<'a> {
    pub command_theme: Option<&'a pxui::Theme>,
    pub files: Option<&'a files::Snapshot>,
    pub file_dragging: bool,
    pub settings: lw_core::preferences::Settings,
    pub cache_usage: &'a str,
    pub settings_message: &'a str,
    pub storage_busy: bool,
    pub artwork: Option<&'a artwork::Images>,
    pub waveform: Option<&'a lw_core::waveform::Peaks>,
    pub revision: u64,
    pub title: &'a str,
    pub context: &'a str,
    pub message: &'a str,
    pub playing: bool,
    pub initialized: bool,
    pub current_id: Option<i64>,
    pub artist: &'a str,
    pub album: &'a str,
    pub duration: f64,
    pub volume: f32,
    pub shuffled: bool,
    pub repeat: lw_core::queue::RepeatMode,
}

#[cfg(test)]
mod tests;
