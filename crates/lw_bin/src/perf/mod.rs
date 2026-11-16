//! Preserve all signed workloads and 182/82/100 frame counts at 1440x872, both Rust allocation
//! counters and unchanged strict timing assertions. Advance the separate position every frame;
//! stable PlayerView revisions follow actual state changes. Never freeze clocks to meet a gate.
//! Require 200 waveform bars/nine release-only seeks; queue carry traces warm twice, preserve
//! payload/capture and cancel without intents or fixture mutation. Filtered Find uses Track m4a
//! year:2019 unplayed and matches all 10,000 fixture rows. Files draws real virtual rows; Settings
//! visits all eight themes through real actions; drop checks draw the overlay without importing.
//!
//! CPU traces sum all engine settling passes; output spans platform.apply and draw spans the
//! public Graphics::draw call, including acquisition/preparation/submission/retirement. Other host
//! work remains in the full-frame total. Rank phase means only after measurement, with wall
//! phases separately labeled. Timing is opt-in, fixed-capacity and never substitutes wall gates.
//!
//! Startup selection for the isolated signed performance matrix; no argument work runs in frames.

mod drive;
mod measured;
mod report;
pub(crate) mod setup;
pub(crate) use drive::Frame;

pub(super) type FrameTrace = (usize, [usize; 8], u64, u64, u64, [u64; 4], u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workload {
    Songs,
    LongScroll,
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
    AlbumDetail,
    ArtistDetail,
    ListDialogs,
    MetadataEditor,
    SongDrag,
    // Measured-only: reported with allocations and timings, never gated.
    Fling,
    ReverseScroll,
    Jump,
    NonSongLists,
    SearchTyping,
    FirstNavigation,
    TextTyping,
    FirstMenuOpen,
    Resize,
    ThemeSwitch,
    Idle600,
    Clock120,
}

impl Workload {
    /// Workloads that report allocations and timings but never fail the run.
    pub fn measured(self) -> bool {
        matches!(
            self,
            Self::Fling
                | Self::ReverseScroll
                | Self::Jump
                | Self::NonSongLists
                | Self::SearchTyping
                | Self::FirstNavigation
                | Self::TextTyping
                | Self::FirstMenuOpen
                | Self::Resize
                | Self::ThemeSwitch
                | Self::Idle600
                | Self::Clock120
        )
    }

    pub fn uses_queue(self) -> bool {
        matches!(
            self,
            Self::Queue | Self::QueueDrag | Self::QueueSheet | Self::Artwork | Self::SongDrag
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Songs => "songs-original",
            Self::LongScroll => "songs-long-scroll",
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
            Self::AlbumDetail => "album-detail-scoped-scroll",
            Self::ArtistDetail => "artist-detail-scoped-scroll",
            Self::ListDialogs => "list-dialogs-menu-and-name",
            Self::MetadataEditor => "metadata-editor-typing",
            Self::SongDrag => "song-drag-carry-hover-cancel",
            Self::Fling => "measured-fling",
            Self::ReverseScroll => "measured-reverse-scroll",
            Self::Jump => "measured-scroll-jump",
            Self::NonSongLists => "measured-non-song-lists",
            Self::SearchTyping => "measured-search-typing",
            Self::FirstNavigation => "measured-first-navigation",
            Self::TextTyping => "measured-text-typing",
            Self::FirstMenuOpen => "measured-first-menu-open",
            Self::Resize => "measured-resize",
            Self::ThemeSwitch => "measured-theme-switch",
            Self::Idle600 => "measured-idle-600-return",
            Self::Clock120 => "measured-clock-120",
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
                "--performance-long-scroll" => Self::LongScroll,
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
                "--performance-album-detail" => Self::AlbumDetail,
                "--performance-artist-detail" => Self::ArtistDetail,
                "--performance-list-dialogs" => Self::ListDialogs,
                "--performance-metadata-editor" => Self::MetadataEditor,
                "--performance-song-drag" => Self::SongDrag,
                "--performance-fling" => Self::Fling,
                "--performance-reverse-scroll" => Self::ReverseScroll,
                "--performance-jump" => Self::Jump,
                "--performance-non-song-lists" => Self::NonSongLists,
                "--performance-search-typing" => Self::SearchTyping,
                "--performance-first-navigation" => Self::FirstNavigation,
                "--performance-text-typing" => Self::TextTyping,
                "--performance-first-menu" => Self::FirstMenuOpen,
                "--performance-resize" => Self::Resize,
                "--performance-theme-switch" => Self::ThemeSwitch,
                "--performance-idle-600" => Self::Idle600,
                "--performance-clock-120" => Self::Clock120,
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

// Two layout frames, then two complete wheel cycles before the measured cycles.
const WARMUP_FRAMES: usize = 82;
const MEASURED_FRAMES: usize = 100;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Warmup {
    artwork: bool,
    catalog: bool,
    layouts: u64,
    requests: usize,
}
impl Warmup {
    fn ready(self) -> bool {
        !self.artwork && !self.catalog && self.layouts == 0 && self.requests == 0
    }
}

pub(crate) struct Perf {
    workload: Workload,
    warmup: Option<Warmup>,
    long_first: usize,
    long_last: usize,
    long_advances: usize,
    indicator_song: Option<i64>,
    waveform_seeks: i32,
    settings_changes: usize,
    settings_themes: u8,
    min_artist_rows: usize,
    max_artist_rows: usize,
    first_artist_min: usize,
    first_artist_max: usize,
    min_waveform_bars: usize,
    indicator_playing: usize,
    indicator_paused: usize,
    indicator_changes: usize,
    indicator_previous: Option<[u32; 3]>,
    drop_overlay_frames: usize,
    warm_allocations: u64,
    warm_layouts: u64,
    warm_build_us: u128,
    timings: [report::FrameTiming; MEASURED_FRAMES],
    frame_trace: [FrameTrace; WARMUP_FRAMES + MEASURED_FRAMES],
    min_artwork_quads: usize,
    warm_artwork_requests: usize,
    min_files_rows: usize,
    max_files_rows: usize,
    detail_first_min: usize,
    detail_first_max: usize,
    marker_frames: usize,
    menu_frames: usize,
}
impl Perf {
    /// First measured frame index: warm-up, plus the silent stretch of the idle-600 workload.
    pub(crate) fn measured_from(&self) -> usize {
        WARMUP_FRAMES + if self.workload == Workload::Idle600 { measured::IDLE_FRAMES } else { 0 }
    }

    /// System appearance forced on this frame, for the theme-switch workload only.
    pub(crate) fn appearance(&self, frame: usize) -> Option<u32> {
        (self.workload == Workload::ThemeSwitch).then(|| measured::appearance(frame))
    }

    pub(crate) fn new(workload: Workload, indicator_song: Option<i64>) -> Self {
        let waveform_seeks = 0;
        let (settings_changes, settings_themes) = (0, 0u8);
        let (min_artist_rows, max_artist_rows) = (usize::MAX, 0usize);
        let (first_artist_min, first_artist_max) = (usize::MAX, 0usize);
        let min_waveform_bars = usize::MAX;
        let (indicator_playing, indicator_paused, indicator_changes) = (0usize, 0usize, 0usize);
        let indicator_previous = None::<[u32; 3]>;

        let drop_overlay_frames = 0usize;
        let (warm_allocations, warm_build_us) = (0, 0);
        let timings = [report::FrameTiming::default(); MEASURED_FRAMES];
        let frame_trace = [(0usize, [0usize; 8], 0u64, 0u64, 0u64, [0u64; 4], 0u64);
            WARMUP_FRAMES + MEASURED_FRAMES];
        let (min_artwork_quads, warm_artwork_requests) = (usize::MAX, 0usize);
        let (min_files_rows, max_files_rows) = (usize::MAX, 0usize);
        Self {
            workload,
            warmup: None,
            long_first: usize::MAX,
            long_last: 0,
            long_advances: 0,
            indicator_song,
            waveform_seeks,
            settings_changes,
            settings_themes,
            min_artist_rows,
            max_artist_rows,
            first_artist_min,
            first_artist_max,
            min_waveform_bars,
            indicator_playing,
            indicator_paused,
            indicator_changes,
            indicator_previous,
            drop_overlay_frames,
            warm_allocations,
            warm_layouts: 0,
            warm_build_us,
            timings,
            frame_trace,
            min_artwork_quads,
            warm_artwork_requests,
            min_files_rows,
            max_files_rows,
            detail_first_min: usize::MAX,
            detail_first_max: 0,
            marker_frames: 0,
            menu_frames: 0,
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
            (Workload::LongScroll, "songs-long-scroll", false),
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
            (Workload::AlbumDetail, "album-detail-scoped-scroll", false),
            (Workload::ArtistDetail, "artist-detail-scoped-scroll", false),
            (Workload::ListDialogs, "list-dialogs-menu-and-name", false),
            (Workload::MetadataEditor, "metadata-editor-typing", false),
            (Workload::SongDrag, "song-drag-carry-hover-cancel", true),
            (Workload::Fling, "measured-fling", false),
            (Workload::ReverseScroll, "measured-reverse-scroll", false),
            (Workload::Jump, "measured-scroll-jump", false),
            (Workload::NonSongLists, "measured-non-song-lists", false),
            (Workload::SearchTyping, "measured-search-typing", false),
            (Workload::FirstNavigation, "measured-first-navigation", false),
            (Workload::TextTyping, "measured-text-typing", false),
            (Workload::FirstMenuOpen, "measured-first-menu-open", false),
            (Workload::Resize, "measured-resize", false),
            (Workload::ThemeSwitch, "measured-theme-switch", false),
            (Workload::Idle600, "measured-idle-600-return", false),
            (Workload::Clock120, "measured-clock-120", false),
        ] {
            assert_eq!(workload.name(), name);
            assert_eq!(workload.uses_queue(), queue);
            assert_eq!(workload.measured(), name.starts_with("measured-"));
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
            ("--performance-long-scroll", Workload::LongScroll),
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
            ("--performance-album-detail", Workload::AlbumDetail),
            ("--performance-artist-detail", Workload::ArtistDetail),
            ("--performance-list-dialogs", Workload::ListDialogs),
            ("--performance-metadata-editor", Workload::MetadataEditor),
            ("--performance-song-drag", Workload::SongDrag),
            ("--performance-fling", Workload::Fling),
            ("--performance-reverse-scroll", Workload::ReverseScroll),
            ("--performance-jump", Workload::Jump),
            ("--performance-non-song-lists", Workload::NonSongLists),
            ("--performance-search-typing", Workload::SearchTyping),
            ("--performance-first-navigation", Workload::FirstNavigation),
            ("--performance-text-typing", Workload::TextTyping),
            ("--performance-first-menu", Workload::FirstMenuOpen),
            ("--performance-resize", Workload::Resize),
            ("--performance-theme-switch", Workload::ThemeSwitch),
            ("--performance-idle-600", Workload::Idle600),
            ("--performance-clock-120", Workload::Clock120),
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
