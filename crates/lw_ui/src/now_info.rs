//! The Now panel's metadata block, formatted once on the worker with Swift's rules
//! (WavePlayerViews `fileFormat`, `sizeLabel`, `locationLabel`, `folderLabel`); frames only draw it.
use lw_core::models::{FileState, Song};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NowInfo {
    pub song: i64,
    pub format: String,
    /// Compact player's `flac 96/24` (Swift `formatSummary`).
    pub summary: String,
    pub size: String,
    pub file: String,
    /// Full folder label for the accessibility name.
    pub folder: String,
    pub plays: String,
}

/// Swift's `%g` for the sample-rate values it prints (at most four decimals).
pub(crate) fn general(value: f64) -> String {
    let text = format!("{value:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

impl NowInfo {
    /// `source` is the source folder's last path component (`None`: Swift's "Source folder").
    pub fn new(song: &Song, source: Option<&str>, plays: i64) -> Self {
        let mut fields = Vec::with_capacity(3);
        match &song.file_extension {
            Some(extension) => {
                fields.push(extension.to_lowercase());
                if let Some(rate) = song
                    .sample_rate
                    .filter(|rate| rate.is_finite() && *rate > 0.0)
                {
                    fields.push(format!("{} kHz", general(rate / 1000.0)));
                }
                if let Some(bits) = song.bit_depth.filter(|bits| *bits > 0) {
                    fields.push(format!("{bits} bit"));
                }
            }
            None => fields.push("audio".into()),
        }
        let summary = match (&song.file_extension, song.sample_rate, song.bit_depth) {
            (Some(extension), Some(rate), Some(bits)) => {
                format!(
                    "{} {}/{bits}",
                    extension.to_lowercase(),
                    general(rate / 1000.0)
                )
            }
            (Some(extension), ..) => extension.to_lowercase(),
            _ => "audio".into(),
        };
        let size = song.file_size.map_or("—".into(), |bytes| {
            format!("{:.1} MB", bytes as f64 / 1_048_576.0)
        });
        let location = if song.file_state == FileState::Copied {
            "on device"
        } else {
            "in cloud"
        };
        let mut folder = source.unwrap_or("Source folder").to_string();
        if let Some(relative) = &song.relative_path {
            let mut parts: Vec<&str> = relative
                .split('/')
                .filter(|part| !part.is_empty())
                .collect();
            parts.pop();
            for part in parts {
                folder.push_str(" / ");
                folder.push_str(part);
            }
        }
        let characters: Vec<char> = folder.chars().collect();
        let file = if characters.len() <= 30 {
            folder.clone()
        } else {
            let head: String = characters[..14].iter().collect();
            let tail: String = characters[characters.len() - 15..].iter().collect();
            format!("{head}…{tail}")
        };
        let (file, folder) = (crate::clip_owned(file), crate::clip_owned(folder));
        Self {
            song: song.id.unwrap_or_default(),
            format: crate::clip_owned(fields.join(" · ")),
            summary: crate::clip_owned(summary),
            size: format!("{size} · {location}"),
            file,
            folder,
            plays: plays.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_swift() {
        let song = Song {
            id: Some(7),
            file_extension: Some("FLAC".into()),
            sample_rate: Some(96000.0),
            bit_depth: Some(24),
            file_size: Some(108_113_920),
            file_state: FileState::Copied,
            relative_path: Some("Weather Systems/Field Day.flac".into()),
            ..Song::default()
        };
        let info = NowInfo::new(&song, Some("Google Drive"), 16);
        assert_eq!(info.format, "flac · 96 kHz · 24 bit");
        assert_eq!(info.summary, "flac 96/24");
        assert_eq!(info.size, "103.1 MB · on device");
        assert_eq!(info.file, "Google Drive / Weather Systems");
        assert_eq!(info.plays, "16");
        let bare = Song {
            file_extension: Some("mp3".into()),
            sample_rate: Some(44100.0),
            ..Song::default()
        };
        let info = NowInfo::new(&bare, None, 0);
        assert_eq!(
            (info.format.as_str(), info.size.as_str()),
            ("mp3 · 44.1 kHz", "— · in cloud")
        );
        assert_eq!(info.file, "Source folder");
        let long = Song {
            relative_path: Some("Abcdefghijklmnopqrstuvwxyz/x.mp3".into()),
            ..Song::default()
        };
        assert_eq!(
            NowInfo::new(&long, Some("Google Drive"), 0).file,
            "Google Drive /…lmnopqrstuvwxyz"
        );
    }
}
