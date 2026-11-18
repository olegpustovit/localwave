//! Prepare palette suggestions and full ranked queue/context payloads on the worker in the same
//! SQLite snapshot as the page. Keep palette/page requests separate; palette-only changes preserve
//! page identity. Refresh on actual playback starts. Command/album/artist matching preserves
//! accents; songs use localized standard containment. Most Played takes the top 24 positive history
//! counts with stable library-order ties. Frames borrow results and activate only keys matching the
//! current query; playback clones the whole ranked queue, not just visible suggestions. Register
//! the application field body at startup and select it through a cached scoped theme; pxui owns
//! editing.
//!
//! Prepared on the library worker; palette frames borrow labels, actions and the ranked queue.
use crate::{browse::Query, catalog::Route, files::Location};
use lw_core::context::PlaybackContext;
use pxui::{KeyCode, Mods};
use std::sync::Arc;

/// Register once, outside frames, before parsing the themes: the palette's field body (selected only in its retained theme) and
/// the context menu row, settings switch and confirmation sheet bodies the app themes select.
pub fn register(ui: &mut pxui::Ui) -> Result<(), pxui::Error> {
    ui.register_text_field_body("localwave-command", crate::shell::commands::field_body)?;
    ui.register_menu_row_body("localwave-menu", crate::shell::menu_row_body)?;
    ui.register_toggle_body("localwave-toggle", crate::shell::toggle_body)?;
    ui.register_dialog_body("localwave-sheet", crate::shell::sheet_body)?;
    static_items();
    Ok(())
}

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
    /// Shortcut-only: toggles the playing song, so the palette does not list it.
    Favorite,
}
impl Command {
    pub fn label(self) -> &'static str {
        match self {
            Self::PlayPause => "Play / pause",
            Self::Shuffle => "Shuffle on or off",
            Self::Repeat => "Cycle repeat",
            Self::AddMusic => "Add folder or files…",
            Self::Rescan(_) => "Rescan source",
            Self::Network => "Simulate no network on / off",
            Self::Theme => "Switch light / dark",
            Self::Panel => "Minimise or restore the player",
            Self::Shortcuts => "Show keyboard shortcuts",
            Self::Settings => "Settings",
            Self::Favorite => "Favorite current song",
        }
    }
    /// Returns the shortcut's `KeyCode` and press-time `Mods`.
    /// `Mods::ANY` accepts a press with any modifier state. `hint` derives from this table.
    pub fn key(self) -> Option<(KeyCode, Mods)> {
        Some(match self {
            Self::PlayPause => (KeyCode::Space, Mods::ANY),
            Self::Shuffle => (KeyCode::Char('s'), Mods::ANY),
            Self::Repeat => (KeyCode::Char('r'), Mods::ANY),
            Self::AddMusic => (KeyCode::Char('o'), Mods::META),
            Self::Panel => (KeyCode::Char('i'), Mods::ANY),
            Self::Shortcuts => (KeyCode::Char('?'), Mods::ANY),
            Self::Settings => (KeyCode::Char(','), Mods::META),
            Self::Favorite => (KeyCode::Char('l'), Mods::ANY),
            _ => return None,
        })
    }
    pub fn hint(self) -> &'static str {
        let Some((key, modifier)) = self.key() else {
            return "";
        };
        match (key, modifier) {
            (KeyCode::Space, _) => "Space",
            (KeyCode::Char('s'), Mods::ANY) => "S",
            (KeyCode::Char('r'), Mods::ANY) => "R",
            (KeyCode::Char('i'), Mods::ANY) => "I",
            (KeyCode::Char('l'), Mods::ANY) => "L",
            (KeyCode::Char('?'), Mods::ANY) => "?",
            (KeyCode::Char('o'), Mods::META) => "⌘O",
            (KeyCode::Char(','), Mods::META) => "⌘,",
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
        let (label, hint) = (crate::clip(label), crate::clip(hint));
        let key = crate::hash((group, label, hint, action));
        Self {
            key,
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

/// The empty-query Commands section without a source-specific Rescan row; built once.
pub fn static_items() -> &'static [Item] {
    static ITEMS: std::sync::OnceLock<Vec<Item>> = std::sync::OnceLock::new();
    ITEMS.get_or_init(|| {
        use Command::*;
        [PlayPause, Shuffle, Repeat, AddMusic, Network, Theme, Panel, Shortcuts]
            .into_iter()
            .map(|c| Item::new("Commands", c.label(), c.hint(), Icon::Chevron, None, Action::Command(c)))
            .collect()
    })
}
