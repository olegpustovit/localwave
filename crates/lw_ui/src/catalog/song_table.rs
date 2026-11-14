//! Worker-prepared SwiftUI desktop song-column measurements.
use super::{Library, Route, Row};
type MeasureWidth<'a> = dyn FnMut(&str, bool, f32, u16) -> Result<f32, String> + 'a;
use crate::browse::Sort;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Column {
    Number,
    Title,
    Artist,
    Album,
    Format,
    State,
    Time,
    Actions,
}
impl Column {
    pub const ALL: [Self; 8] = [
        Self::Number,
        Self::Title,
        Self::Artist,
        Self::Album,
        Self::Format,
        Self::State,
        Self::Time,
        Self::Actions,
    ];
    pub fn title(self) -> &'static str {
        [
            "#",
            "Title",
            "Artist",
            "Album",
            "Format",
            "On device",
            "Time",
            "Actions",
        ][self as usize]
    }
    pub fn sort(self) -> Option<Sort> {
        match self {
            Self::Number => Some(Sort::Original),
            Self::Title => Some(Sort::Title),
            Self::Artist => Some(Sort::Artist),
            Self::Album => Some(Sort::Album),
            Self::Format => Some(Sort::Format),
            Self::Time => Some(Sort::Duration),
            _ => None,
        }
    }
}

pub(super) struct RowWidths {
    regular: [f32; 3],
    medium: [f32; 3],
    format: f32,
    time: f32,
    track: Option<f32>,
    track_body: Option<f32>,
}
pub(super) struct Metrics {
    rows: Vec<RowWidths>,
    headings: [[f32; 3]; 8],
    digit: f32,
    body_digit: f32,
}
impl Metrics {
    pub(super) fn body_width(
        &self,
        index: usize,
        column: Column,
        number: usize,
        numbered: bool,
    ) -> f32 {
        let row = &self.rows[index];
        match column {
            Column::Number => if numbered { row.track_body } else { None }.unwrap_or_else(|| {
                number.max(1).ilog10() as f32 * self.body_digit + self.body_digit
            }),
            Column::Format => row.format - 16.0,
            Column::Time => row.time - 16.0,
            _ => 0.0,
        }
    }
    pub(super) fn heading_width(&self, column: Column, ascending: Option<bool>) -> f32 {
        self.headings[column as usize][match ascending {
            None => 0,
            Some(true) => 1,
            Some(false) => 2,
        }] - if column == Column::Number { 40.0 } else { 16.0 }
    }
    pub fn prepare(library: &Library, width: &mut MeasureWidth<'_>) -> Result<Self, String> {
        let mut headings = [[0.0; 3]; 8];
        for column in Column::ALL {
            let inset = if column == Column::Number { 40.0 } else { 16.0 };
            headings[column as usize] = [
                width(column.title(), false, 12.0, 500)? + inset,
                width(&format!("{} ↑", column.title()), false, 12.0, 500)? + inset,
                width(&format!("{} ↓", column.title()), false, 12.0, 500)? + inset,
            ];
        }
        let mut rows = Vec::with_capacity(library.len());
        for index in 0..library.len() {
            let row = library.row(index);
            let mut regular = [0.0; 3];
            let mut medium = [0.0; 3];
            for (column, value) in [row.title, row.artist, row.album].into_iter().enumerate() {
                regular[column] = (width(value, false, 14.0, 400)? + 16.0).min(240.0);
                medium[column] = (width(value, false, 14.0, 500)? + 16.0).min(240.0);
            }
            rows.push(RowWidths {
                regular,
                medium,
                format: width(
                    if row.has_format { row.format } else { "—" },
                    true,
                    12.0,
                    400,
                )? + 16.0,
                time: width(row.duration, true, 12.0, 400)? + 16.0,
                track_body: row
                    .track_number
                    .map(|_| width(row.track_label, false, 13.0, 400))
                    .transpose()?,
                track: row
                    .track_number
                    .map(|_| width(row.track_label, true, 13.0, 400))
                    .transpose()?,
            });
        }
        Ok(Self {
            rows,
            headings,
            digit: width("0", true, 13.0, 400)?,
            body_digit: width("0", false, 13.0, 400)?,
        })
    }
}

// Keep the runner-up so changing the widest row to medium can also shrink a column.
#[derive(Clone, Copy, Default)]
struct Widest {
    row: Option<usize>,
    first: f32,
    second: f32,
}
impl Widest {
    fn add(&mut self, row: usize, width: f32) {
        if self.row == Some(row) {
            return;
        }
        if width > self.first {
            self.second = self.first;
            self.first = width;
            self.row = Some(row);
        } else {
            self.second = self.second.max(width);
        }
    }
    fn without(self, row: Option<usize>) -> f32 {
        if row.is_some() && self.row == row {
            self.second
        } else {
            self.first
        }
    }
}

pub struct SongTable {
    widest: [Widest; 3],
    preferred: [f32; 8],
    numbered: f32,
    members: Vec<usize>,
    mixed_formats: bool,
    mixed_states: bool,
}

fn state(row: Row<'_>) -> (u8, bool, Option<lw_core::FileAvailability>) {
    let exceptional = match row.state {
        lw_core::FileState::CopyPending => 1,
        lw_core::FileState::Failed => 2,
        _ => 0,
    };
    (
        exceptional,
        exceptional == 0 && row.pinned,
        if exceptional == 0 && !row.pinned {
            Some(
                row.availability
                    .unwrap_or(lw_core::FileAvailability::Unknown),
            )
        } else {
            None
        },
    )
}
impl SongTable {
    pub(super) fn prepare(library: &Library, rows: &[usize], metrics: &Metrics) -> Self {
        let mut preferred = metrics.headings.map(|width| width[0]);
        preferred[0] = preferred[0].max(44.0);
        let numbered = preferred[0];
        if !rows.is_empty() {
            preferred[0] =
                preferred[0].max(((rows.len().ilog10() + 1) as f32) * metrics.digit + 40.0);
        }
        preferred[7] = 52.0;
        let mut out = Self {
            widest: [Widest::default(); 3],
            numbered,
            preferred,
            members: rows.to_vec(),
            mixed_formats: false,
            mixed_states: false,
        };
        out.members.sort_unstable();
        out.members.dedup();
        let first = rows.first().map(|index| library.row(*index));
        for (index, &key) in rows.iter().enumerate() {
            let row = library.row(key);
            let widths = &metrics.rows[key];
            for column in 0..3 {
                out.widest[column].add(key, widths.regular[column]);
            }
            out.preferred[4] = out.preferred[4].max(widths.format);
            out.preferred[6] = out.preferred[6].max(widths.time);
            out.numbered = out.numbered.max(
                widths
                    .track
                    .unwrap_or_else(|| ((index + 1).ilog10() + 1) as f32 * metrics.digit)
                    + 40.0,
            );
            out.mixed_formats |= first.is_some_and(|first| first.format_key != row.format_key);
            out.mixed_states |= first.is_some_and(|first| state(first) != state(row));
        }
        out
    }

    pub fn columns(
        &self,
        library: &Library,
        width: f32,
        current: Option<usize>,
        sort: Sort,
        route: Route,
    ) -> [f32; 8] {
        let numbered = matches!(route, Route::Album(_));
        let hide_artist = matches!(route, Route::Album(_) | Route::Artist(_));
        let hide_album = numbered;
        let metrics = library
            .song_metrics
            .as_ref()
            .expect("song table metrics share their library");
        let current = current.filter(|index| self.members.binary_search(index).is_ok());
        let mut preferred = self.preferred;
        if numbered {
            preferred[0] = self.numbered;
        }
        if current.is_some() {
            preferred[0] = preferred[0].max(54.0);
        }
        for column in 0..3 {
            preferred[column + 1] = self.widest[column]
                .without(current)
                .max(current.map_or(0.0, |index| metrics.rows[index].medium[column]))
                .max(
                    metrics.headings[column + 1]
                        [usize::from(Column::ALL[column + 1].sort() == Some(sort))],
                );
        }
        for column in [Column::Format, Column::Time] {
            if column.sort() == Some(sort) {
                preferred[column as usize] =
                    preferred[column as usize].max(metrics.headings[column as usize][1]);
            }
        }
        for (column, visible) in [
            (Column::Artist, !hide_artist && width >= 400.0),
            (Column::Album, !hide_album && width >= 520.0),
            (Column::Format, self.mixed_formats && width >= 680.0),
            (Column::State, self.mixed_states),
        ] {
            if !visible {
                preferred[column as usize] = 0.0;
            }
        }
        let total = preferred[1..7].iter().sum::<f32>().max(1.0);
        let available = (width - preferred[0] - 52.0).max(0.0);
        for value in &mut preferred[1..7] {
            *value = *value * available / total;
        }
        preferred
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_core::{FileAvailability, FileState, Song};

    fn fixture() -> Vec<Song> {
        vec![
            Song {
                id: Some(1),
                track_number: Some(9),
                title: "Quiet lights".into(),
                artist: "The National".into(),
                album: "A midnight album".into(),
                file_extension: Some("FLAC".into()),
                duration: Some(240.0),
                ..Default::default()
            },
            Song {
                id: Some(2),
                track_number: Some(12),
                title: "An exceptionally long song title".into(),
                artist: "Another artist".into(),
                album: "An earlier record".into(),
                file_extension: Some("mp3".into()),
                duration: Some(61.0),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn song_columns_match_native_swift_measurements_and_responsive_thresholds() {
        let catalog = crate::tests::prepared(fixture());
        let table = catalog.all.as_ref().unwrap().song_table.as_ref().unwrap();
        let widths = table.columns(&catalog, 888.0, None, Sort::Original, Route::Songs);
        // Actual WaveSongColumns.make, bundled CoreText faces, two fixture songs at 888 points.
        for (width, native) in widths.into_iter().zip([
            49.6, 310.6764, 149.12798, 184.86348, 77.84806, 0.0, 63.88407, 52.0,
        ]) {
            assert!(
                (width - native).abs() < 0.01,
                "pxui {width}, native {native}"
            );
        }
        for width in [399.0, 400.0, 519.0, 520.0, 679.0, 680.0, 888.0] {
            let columns = table.columns(&catalog, width, None, Sort::Original, Route::Songs);
            assert_eq!(columns[Column::Artist as usize] > 0.0, width >= 400.0);
            assert_eq!(columns[Column::Album as usize] > 0.0, width >= 520.0);
            assert_eq!(columns[Column::Format as usize] > 0.0, width >= 680.0);
            assert!((columns.iter().sum::<f32>() - width).abs() < 0.001);
        }
        let number = table.columns(&catalog, 888.0, None, Sort::Original, Route::Album(0));
        assert!(
            (number[0] - 55.6).abs() < 0.01,
            "two-digit track number in 13pt Geist Mono"
        );
        assert_eq!((number[2], number[3]), (0.0, 0.0));
    }

    #[test]
    fn prepared_song_projections_keep_current_membership_track_numbers_states_and_empty_widths() {
        let mut songs = fixture();
        songs[0].source_availability = Some(FileAvailability::OnDevice);
        songs[1].source_availability = Some(FileAvailability::InCloud);
        let mut catalog = crate::tests::prepared(songs);
        let request = crate::browse::Request {
            commands: None,
            scope: crate::browse::Scope::Find,
            query: crate::browse::Query::default(),
            sort: Sort::Original,
            ascending: true,
        };
        catalog.project(request, vec![0, 0]);
        let table = catalog
            .view
            .as_ref()
            .unwrap()
            .collection
            .song_table
            .as_ref()
            .unwrap();
        let absent = table.columns(&catalog, 888.0, Some(1), Sort::Original, Route::Songs);
        let regular = table.columns(&catalog, 888.0, None, Sort::Original, Route::Songs);
        assert_eq!(
            absent, regular,
            "a current song outside the projection cannot change its columns"
        );
        assert_eq!(
            regular[5], 0.0,
            "one repeated state does not add a state column"
        );
        let all = catalog.all.as_ref().unwrap().song_table.as_ref().unwrap();
        assert!(all.columns(&catalog, 888.0, None, Sort::Original, Route::Songs)[5] > 0.0);
        let current = table.columns(&catalog, 888.0, Some(0), Sort::Title, Route::Songs);
        assert!(current[0] >= 54.0);
        assert_eq!(catalog.row(0).format, "flac");
        assert_eq!(catalog.row(0).track_label, "9");
        catalog.project(request, Vec::new());
        let empty = catalog
            .view
            .as_ref()
            .unwrap()
            .collection
            .song_table
            .as_ref()
            .unwrap();
        assert!(
            empty
                .columns(&catalog, 888.0, Some(0), Sort::Original, Route::Songs)
                .iter()
                .all(|width| width.is_finite())
        );
        let mut top = Widest::default();
        top.add(0, 100.0);
        top.add(1, 90.0);
        top.add(0, 100.0);
        assert_eq!(
            top.without(Some(0)),
            90.0,
            "changing the widest row may shrink it"
        );
        let unknown = Row {
            availability: None,
            ..catalog.row(0)
        };
        assert_eq!(
            state(unknown),
            state(Row {
                availability: Some(FileAvailability::Unknown),
                ..unknown
            })
        );
        assert_ne!(
            state(unknown),
            state(Row {
                state: FileState::CopyPending,
                ..unknown
            })
        );
        assert_ne!(
            state(unknown),
            state(Row {
                pinned: true,
                ..unknown
            })
        );
    }
}
