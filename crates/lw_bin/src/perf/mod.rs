//! Startup selection for the isolated signed performance matrix; no argument work runs in frames.

pub(crate) mod setup;

pub(super) type FrameTrace = (usize, [usize; 8], u64, u64, u64, [u64; 4], u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workload {
    Songs,
    SongIndicators,
    Artists,
    Drop,
    Files,
    Settings,
    Waveform,
    Artwork,
    Find,
    QueueSheet,
    QueueDrag,
    Queue,
}

impl Workload {
    pub fn uses_queue(self) -> bool {
        matches!(
            self,
            Self::Queue | Self::QueueDrag | Self::QueueSheet | Self::Artwork
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Songs => "songs-original",
            Self::SongIndicators => "songs-playing-indicator",
            Self::Artists => "artists-measured-columns",
            Self::Drop => "external-drop-hover-and-cancel",
            Self::Files => "files-indexed-directory",
            Self::Settings => "settings-menus-appearance",
            Self::Waveform => "waveform-scrub-and-scroll",
            Self::Artwork => "albums-artwork",
            Self::Find => "find-filters-title-descending",
            Self::QueueSheet => "full-queue-playlist-editor",
            Self::QueueDrag => "queue-carry-scroll-cancel",
            Self::Queue => "queue-occurrences",
        }
    }

    pub fn from_args(args: &[String]) -> Result<Option<Self>, String> {
        let mut enabled = false;
        let mut selected = None;
        for arg in args {
            let workload = match arg.as_str() {
                "--performance-check" => {
                    enabled = true;
                    continue;
                }
                "--performance-song-indicators" => Self::SongIndicators,
                "--performance-artists" => Self::Artists,
                "--performance-drop" => Self::Drop,
                "--performance-files" => Self::Files,
                "--performance-settings" => Self::Settings,
                "--performance-waveform" => Self::Waveform,
                "--performance-artwork" => Self::Artwork,
                "--performance-find" => Self::Find,
                "--performance-queue-sheet" => Self::QueueSheet,
                "--performance-queue-drag" => Self::QueueDrag,
                "--performance-queue" => Self::Queue,
                _ if arg.starts_with("--performance-") => {
                    return Err(format!("Unknown performance option: {arg}"));
                }
                _ => continue,
            };
            if let Some((previous, previous_arg)) = selected
                && previous != workload
            {
                return Err(format!(
                    "Performance workloads must run separately: {previous_arg} and {arg}"
                ));
            }
            selected = Some((workload, arg));
        }
        match (enabled, selected) {
            (true, selected) => Ok(Some(selected.map_or(Self::Songs, |(workload, _)| workload))),
            (false, Some((_, arg))) => Err(format!("{arg} requires --performance-check")),
            (false, None) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workload_reports_and_queue_fixtures_match_the_signed_matrix() {
        for (workload, name, queue) in [
            (Workload::Songs, "songs-original", false),
            (Workload::SongIndicators, "songs-playing-indicator", false),
            (Workload::Artists, "artists-measured-columns", false),
            (Workload::Drop, "external-drop-hover-and-cancel", false),
            (Workload::Files, "files-indexed-directory", false),
            (Workload::Settings, "settings-menus-appearance", false),
            (Workload::Waveform, "waveform-scrub-and-scroll", false),
            (Workload::Artwork, "albums-artwork", true),
            (Workload::Find, "find-filters-title-descending", false),
            (Workload::QueueSheet, "full-queue-playlist-editor", true),
            (Workload::QueueDrag, "queue-carry-scroll-cancel", true),
            (Workload::Queue, "queue-occurrences", true),
        ] {
            assert_eq!(workload.name(), name);
            assert_eq!(workload.uses_queue(), queue);
        }
    }

    #[test]
    fn workload_args_select_one_complete_matrix_case_and_reject_mixed_fixtures() {
        let args = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            Workload::from_args(&args(&["localwave", "--smoke-test"])),
            Ok(None)
        );
        assert_eq!(
            Workload::from_args(&args(&["--performance-check"])),
            Ok(Some(Workload::Songs))
        );
        let modes = [
            ("--performance-song-indicators", Workload::SongIndicators),
            ("--performance-artists", Workload::Artists),
            ("--performance-drop", Workload::Drop),
            ("--performance-files", Workload::Files),
            ("--performance-settings", Workload::Settings),
            ("--performance-waveform", Workload::Waveform),
            ("--performance-artwork", Workload::Artwork),
            ("--performance-find", Workload::Find),
            ("--performance-queue-sheet", Workload::QueueSheet),
            ("--performance-queue-drag", Workload::QueueDrag),
            ("--performance-queue", Workload::Queue),
        ];
        for (flag, expected) in modes {
            for values in [
                vec!["--performance-check", flag],
                vec![flag, "--trace-frames", "--performance-check", flag],
            ] {
                assert_eq!(Workload::from_args(&args(&values)), Ok(Some(expected)));
            }
            assert!(Workload::from_args(&args(&[flag])).is_err());
            for (other, _) in modes {
                if flag != other {
                    assert!(
                        Workload::from_args(&args(&["--performance-check", flag, other])).is_err()
                    );
                }
            }
        }
        assert!(
            Workload::from_args(&args(&["--performance-check", "--performance-typo"])).is_err()
        );
        assert!(Workload::from_args(&args(&["--performance-typo"])).is_err());
    }
}
