//! Prepared on the library worker; palette frames borrow labels, actions and the ranked queue.
use crate::{browse::Query, catalog::Route, files::Location};
use lw_core::context::PlaybackContext;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    PlayPause,
    Shuffle,
    Repeat,
    AddMusic,
    Rescan(Location),
    Network,
    Theme,
    Panel,
    Shortcuts,
    Settings,
}
impl Command {
    pub fn label(self) -> &'static str {
        match self {
            Self::PlayPause => "Play / pause",
            Self::Shuffle => "Shuffle on or off",
            Self::Repeat => "Cycle repeat",
            Self::AddMusic => "Add folder or files…",
            Self::Rescan(_) => "Rescan Google Drive",
            Self::Network => "Simulate no network on / off",
            Self::Theme => "Switch light / dark",
            Self::Panel => "Minimise or restore the player",
            Self::Shortcuts => "Show keyboard shortcuts",
            Self::Settings => "Settings",
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            Self::PlayPause => "Space",
            Self::Shuffle => "S",
            Self::Repeat => "R",
            Self::AddMusic => "⌘O",
            Self::Panel => "I",
            Self::Shortcuts => "?",
            Self::Settings => "⌘,",
            _ => "",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Play(usize),
    Navigate(Route),
    Folder(Location),
    Find,
    Command(Command),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Chevron,
    Queue,
    Person,
    Folder,
    Rows,
}
pub struct Item {
    pub key: u64,
    pub group: &'static str,
    pub label: String,
    pub hint: String,
    pub icon: Icon,
    pub artwork: Option<usize>,
    pub action: Action,
}
impl Item {
    pub fn new(
        group: &'static str,
        label: &str,
        hint: &str,
        icon: Icon,
        artwork: Option<usize>,
        action: Action,
    ) -> Self {
        let mut key = DefaultHasher::new();
        (group, label, hint, action).hash(&mut key);
        Self {
            key: key.finish(),
            group,
            label: label.into(),
            hint: hint.into(),
            icon,
            artwork,
            action,
        }
    }
}
pub struct Results {
    pub query: Query,
    pub items: Vec<Item>,
    pub songs: Arc<[i64]>,
    pub context: Arc<PlaybackContext>,
}
