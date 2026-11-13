//! Queue origin and display title, matching the shipping Swift PlaybackContext cases.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PlaybackContext {
    #[default]
    Library,
    Album {
        name: String,
        artist: String,
    },
    Artist(String),
    Playlist {
        id: i64,
        name: String,
    },
    Folder {
        source_id: i64,
        path_id: Option<i64>,
        name: String,
    },
    Automatic(String),
    Selection(String),
}

impl PlaybackContext {
    pub fn text_len(&self) -> usize {
        match self {
            Self::Library => 0,
            Self::Album { name, artist } => name.len().saturating_add(artist.len()),
            _ => self.title().len(),
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Library => "Library",
            Self::Album { name, .. }
            | Self::Playlist { name, .. }
            | Self::Folder { name, .. }
            | Self::Artist(name)
            | Self::Automatic(name)
            | Self::Selection(name) => name,
        }
    }
}
