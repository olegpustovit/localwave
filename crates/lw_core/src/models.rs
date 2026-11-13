#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i64)]
pub enum FileState {
    #[default]
    BookmarkOnly,
    CopyPending,
    Copied,
    Failed,
}

impl FileState {
    pub fn from_raw(value: i64) -> Self {
        match value {
            1 => Self::CopyPending,
            2 => Self::Copied,
            3 => Self::Failed,
            _ => Self::BookmarkOnly,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileAvailability {
    OnDevice,
    InCloud,
    Unknown,
}
impl FileAvailability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OnDevice => "onDevice",
            Self::InCloud => "inCloud",
            Self::Unknown => "unknown",
        }
    }
    pub fn from_raw(value: &str) -> Option<Self> {
        match value {
            "onDevice" => Some(Self::OnDevice),
            "inCloud" => Some(Self::InCloud),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Song {
    pub id: Option<i64>,
    pub song_key: i64,
    pub artist: String,
    pub title: String,
    pub album: String,
    pub album_artist: String,
    pub release_year: Option<i64>,
    pub disc_number: Option<i64>,
    pub track_number: Option<i64>,
    pub cover_art_path: Option<String>,
    pub bookmark: Option<Vec<u8>>,
    pub path_hash: i64,
    pub created_at: f64,
    pub updated_at: Option<f64>,
    pub local_file_path: Option<String>,
    pub file_state: FileState,
    pub is_favorite: i64,
    pub source_id: Option<i64>,
    pub relative_path: Option<String>,
    pub duration: Option<f64>,
    pub file_size: Option<i64>,
    pub file_extension: Option<String>,
    pub sample_rate: Option<f64>,
    pub bit_depth: Option<i64>,
    pub source_availability: Option<FileAvailability>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub id: i64,
    pub icloud_id: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub created_at: f64,
    pub updated_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Album {
    pub name: String,
    pub artist: String,
    pub cover_art_path: Option<String>,
}

pub fn valid_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\0')
        && !path.split('/').any(|component| component == "..")
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Source {
    pub id: Option<i64>,
    pub dir_path: String,
    pub path_id: i64,
    pub user_id: i64,
    pub source_type: Option<String>,
    pub bookmark: Option<Vec<u8>>,
    pub total_paths: Option<i64>,
    pub sync_error: Option<String>,
    pub is_current: bool,
    pub created_at: f64,
    pub last_synced_at: Option<f64>,
    pub updated_at: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourcePath {
    pub id: Option<i64>,
    pub source_id: i64,
    pub path_id: i64,
    pub parent_path_id: Option<i64>,
    pub name: String,
    pub relative_path: String,
    pub is_directory: bool,
    pub file_size: Option<i64>,
    pub file_hash_sha256: Option<Vec<u8>>,
    pub run_id: i64,
    pub created_at: f64,
    pub updated_at: Option<f64>,
}

impl SourcePath {
    pub fn is_audio(&self) -> bool {
        !self.is_directory && supports_audio(&self.name)
    }
}

/// Matches Swift's masked FNV-1a over the original UTF-8, without Unicode normalization.
pub fn hash_string(text: &str) -> i64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.bytes() {
        hash = (hash ^ byte as u64).wrapping_mul(0x100_0000_01b3);
    }
    (hash & 0x7fff_ffff_ffff_ffff) as i64
}

pub fn song_key(artist: &str, title: &str, album: &str) -> i64 {
    // Swift lowercases scalars: Rust's str lowercase applies final-sigma context and changes saved keys.
    let lower = |text: &str| {
        text.chars()
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    hash_string(&format!(
        "{}__{}__{}",
        lower(artist),
        lower(title),
        lower(album)
    ))
}

pub fn supports_audio(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, extension)| {
        ["mp3", "wav", "m4a", "flac", "aac", "aiff", "aif"]
            .iter()
            .any(|valid| extension.eq_ignore_ascii_case(valid))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_preserves_utf8_case_and_overflow_contract() {
        assert_eq!(hash_string(""), 0x4bf2_9ce4_8422_2325);
        assert_eq!(hash_string("hello"), 0x2430_d846_80aa_bd0b);
        assert_ne!(hash_string("é"), hash_string("e\u{301}"));
        assert_eq!(
            song_key("ARTIST", "TITLE", "ALBUM"),
            song_key("artist", "title", "album")
        );
        assert_eq!(song_key("ΟΣ", "ΟΣ", "ΟΣ"), 389064132268368008);
        assert!(supports_audio("Album.TRACK.FLAC"));
        assert!(!supports_audio("mp3"));
        assert!(!supports_audio("track.mp3.txt"));
    }
}
