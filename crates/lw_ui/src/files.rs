//! Worker-prepared source and directory labels; frames only borrow these immutable snapshots.
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Location {
    pub user: i64,
    pub source: i64,
    pub parent: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    pub user: i64,
    pub source: i64,
    pub root: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Roots,
    Browse(Location),
    /// Browse the folder that holds a saved song ("Show in folder").
    Locate {
        user: i64,
        song: i64,
    },
    Rescan(Location),
    Import(Location),
    ImportPath {
        user: i64,
        source: i64,
        path: i64,
    },
    Remove(Target),
    Cancel,
}

/// A source card. Every label is prepared on the FilesWorker (Swift's wording and formats).
#[derive(Default)]
pub struct Folder {
    pub id: i64,
    pub root: i64,
    pub name: String,
    pub path: String,
    /// "34 songs", or "N indexed items" until the source has saved songs.
    pub count: String,
    /// "0 kept · 28 on device · 6 in cloud"; empty without saved songs.
    pub stats: String,
    /// Relative scan age ("2 min. ago") or "Not scanned".
    pub scanned: String,
    /// The source's sync error (access expired, moved, ...).
    pub error: Option<String>,
    /// Share of saved songs on this device, 0..=1, for the card's bar.
    pub local: f32,
    pub current: bool,
}

/// The per-file availability glyph's inputs (Swift's LWStateGlyph).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub state: lw_core::FileState,
    pub availability: Option<lw_core::FileAvailability>,
    pub pinned: bool,
}

#[derive(Default)]
pub struct Entry {
    pub id: i64,
    pub name: String,
    /// 1-based audio number; empty for folders and other files.
    pub number: String,
    /// "flac audio", "Folder · 3 songs" or "Not audio".
    pub kind: String,
    /// "20.3 MB", folder totals, or "—".
    pub size: String,
    /// "2:29"; empty without a saved song.
    pub time: String,
    /// Compact row detail: "FLAC · 20.3 MB · 2:29", "Folder" or "not audio · 1.0 MB".
    pub detail: String,
    /// The saved song behind an audio row (menu and play); None until imported.
    pub song: Option<i64>,
    /// None for audio that is not imported yet.
    pub glyph: Option<Glyph>,
    /// Saved songs below a folder row in natural path order (Play folder / Shuffle folder); empty elsewhere.
    pub songs: Arc<[i64]>,
    pub directory: bool,
    pub audio: bool,
}

#[derive(Default)]
pub struct Directory {
    pub location: Location,
    pub source_root: i64,
    pub up: Option<i64>,
    pub title: String,
    /// "Source / Folder / Sub"; empty at the source root.
    pub breadcrumb: String,
    /// "N items" (compact layout).
    pub detail: String,
    /// "8 songs · 310.9 MB · 7 offline".
    pub summary: String,
    /// The source's sync error, shown with a Reconnect action.
    pub error: Option<String>,
    /// Saved songs under this folder in natural path order: Play folder, Shuffle and Keep offline.
    pub songs: Arc<[i64]>,
    pub context: Arc<lw_core::context::PlaybackContext>,
    /// Every saved song here is pinned (Swift's "Kept offline").
    pub kept: bool,
    /// Swift's Show non-audio files setting: the compact list hides the trailing `non_audio` entries without it.
    pub show_non_audio: bool,
    pub non_audio: usize,
    pub entries: Vec<Entry>,
    /// Entry positions each availability tab shows (Offline, In cloud): folders plus matching audio.
    pub filters: [Vec<u32>; 2],
    /// Tab labels' counts for All, Offline and In cloud (audio files only, as Swift's filterCount).
    pub filter_counts: [String; 3],
    /// "3 non-audio files hidden ·" for the compact footer; empty without hidden files.
    pub hidden_label: String,
}

impl Directory {
    /// The compact list's row at a position of the tab: positions into `entries`.
    pub fn entry(&self, filter: crate::FilesFilter, index: usize) -> Option<&Entry> {
        let at = match filter {
            crate::FilesFilter::All => index,
            crate::FilesFilter::Offline => *self.filters[0].get(index)? as usize,
            crate::FilesFilter::Cloud => *self.filters[1].get(index)? as usize,
        };
        self.entries.get(at)
    }
    /// Rows the compact tab shows; non-audio files trail the full list and hide unless the setting shows them.
    pub fn shown(&self, filter: crate::FilesFilter, show_non_audio: bool) -> usize {
        match filter {
            crate::FilesFilter::All => {
                self.entries.len() - if show_non_audio { 0 } else { self.non_audio }
            }
            crate::FilesFilter::Offline => self.filters[0].len(),
            crate::FilesFilter::Cloud => self.filters[1].len(),
        }
    }
}

#[derive(Clone)]
pub struct Selection {
    pub revision: u64,
    pub songs: Arc<[i64]>,
    pub context: Arc<lw_core::context::PlaybackContext>,
    pub dropped: bool,
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub revision: u64,
    pub account: Option<i64>,
    pub sources: Arc<[Folder]>,
    pub source_count: String,
    /// "0.0 of 8 GB" cached bytes against the offline limit, and its meter fraction.
    pub offline_usage: String,
    pub offline_fraction: f32,
    pub directory: Option<Arc<Directory>>,
    pub message: String,
    pub progress: String,
    pub busy: bool,
    pub catalog_revision: u64,
    pub selection: Option<Selection>,
}
