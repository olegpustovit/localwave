//! LocalWave's pxui views. Application state changes are returned as intents.
pub mod artwork;
pub mod browse;
pub mod catalog;
pub mod commands;
pub mod files;
mod icons;
pub mod motion;
pub mod now_info;
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

/// Whole-page actions of the compact "…" menu; the controller applies them to the displayed collection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageAction {
    Play,
    Shuffle,
    PlayNext,
    AddToQueue,
    KeepOffline,
    AddToList(i64),
}

/// The compact folder page's availability tabs (Swift: All / Offline / In cloud).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilesFilter {
    #[default]
    All,
    Offline,
    Cloud,
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
    /// Phone Select mode (rows show checkboxes, a bottom bar acts on the marks); false is Done.
    SelectMode(bool),
    /// The selection bar's Add to list: the song menu's list choices for the first marked row.
    SelectionLists(usize, i64, u64, (f32, f32)),
    /// The compact page "…" menu, its list choices and whole-page actions.
    PageMenu((f32, f32)),
    PageLists,
    ClosePageMenu,
    Page(PageAction),
    /// Play a saved file of the open folder page inside the folder's context.
    PlayFile(i64),
    FilesFilter(FilesFilter),
    ShowAddMusic,
    PlayRow(usize),
    /// Return over a multi-row selection: play exactly the selected songs in displayed order.
    PlaySelection,
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
    /// Open the metadata editor prefilled from the row (track menu "Edit info…").
    OpenMetadata(MetadataEdit),
    MetadataField(usize, browse::Query),
    MetadataTooLong,
    CloseMetadata,
    SaveMetadata(MetadataEdit),
    /// Ask before removing the song's library entry; files are never deleted.
    ConfirmDeleteSong(i64),
    CloseDeleteSong,
    DeleteSong(i64),
    /// Share the song (or its selection) from the menu opened at this window point.
    ShareSong(i64, (f32, f32)),
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
    ClearQueue,
    UndoQueue,
    ShowQueue,
    CloseQueue,
    /// Save the queue as a list (queue sheet).
    NamePlaylist,
    /// Open the standalone name dialog for an empty list, or one holding the menu's song(s).
    NewList(Option<i64>),
    /// Open the name dialog prefilled to rename a list.
    RenameListDialog(i64, browse::Query),
    CreateList {
        name: browse::Query,
        song: Option<i64>,
    },
    RenameList {
        list: i64,
        name: browse::Query,
    },
    ListMenu(i64, (f32, f32)),
    CloseListMenu,
    ConfirmDeleteList(i64),
    CloseDeleteList,
    DeleteList(i64),
    /// Move a row of the displayed list to this final position (drag reorder).
    MoveListRow {
        from: usize,
        to: usize,
    },
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
    /// Play or shuffle the open folder page's saved songs, or toggle keeping them offline.
    PlayFolder,
    ShuffleFolder,
    KeepFolder,
    ChooseFolder,
    ChooseAudio,
    SourceMenu(files::Target, (f32, f32)),
    CloseSourceMenu,
    ReconnectSource(files::Target),
    ConfirmRemoveSource(files::Target),
    CloseRemoveSource,
}

/// The editable metadata of one song as bounded text packets; year and track stay text until saved.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MetadataEdit {
    pub song: i64,
    pub title: browse::Query,
    pub artist: browse::Query,
    pub album: browse::Query,
    pub album_artist: browse::Query,
    pub year: browse::Query,
    pub track: browse::Query,
}
impl MetadataEdit {
    pub const MAX_NUMBER: i64 = 9999;
    pub fn field_mut(&mut self, index: usize) -> Option<&mut browse::Query> {
        Some(match index {
            0 => &mut self.title,
            1 => &mut self.artist,
            2 => &mut self.album,
            3 => &mut self.album_artist,
            4 => &mut self.year,
            5 => &mut self.track,
            _ => return None,
        })
    }
    /// Blank means unset; otherwise a whole number within 0..=9999.
    fn number(text: &browse::Query) -> Option<Option<i64>> {
        let text = text.as_str().trim();
        if text.is_empty() {
            return Some(None);
        }
        text.parse::<i64>()
            .ok()
            .filter(|n| {
                (0..=Self::MAX_NUMBER).contains(n) && text.bytes().all(|b| b.is_ascii_digit())
            })
            .map(Some)
    }
    /// (year, track) when both fields are valid.
    pub fn numbers(&self) -> Option<(Option<i64>, Option<i64>)> {
        Some((Self::number(&self.year)?, Self::number(&self.track)?))
    }
    /// A song needs an id and a title; numbers must parse.
    pub fn valid(&self) -> bool {
        self.song > 0 && !self.title.as_str().trim().is_empty() && self.numbers().is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ListDialog {
    /// New list, optionally holding the track menu's song (and its selection).
    New(Option<i64>),
    Rename(i64),
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
    /// A share is being prepared; Swift's `sharing` flag disables Share…
    pub sharing: bool,
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
    /// Worker-formatted metadata of the current song, when known.
    pub info: Option<&'a now_info::NowInfo>,
    pub duration: f64,
    pub volume: f32,
    pub shuffled: bool,
    pub repeat: lw_core::queue::RepeatMode,
}

#[cfg(test)]
mod tests;
