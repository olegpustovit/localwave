//! Song-cell prewarm policy over pxui's prepare queue (`Ui::prewarm_text`). Capture the actual
//! measured-list row window; request a bounded margin around it, never the whole library: rows
//! nearest the built rows first, below then above. Requests use the exact label, width and density
//! rounding of the ordinary line-box nodes.
//!
//! Budget (a lane holds `LANE_REQUESTS` requests in flight and installs 32 a frame; a request past a
//! full lane is dropped and that cell shapes when shown):
//! - while the list moves (its row window changed since the last frame), `ROWS_PER_FRAME` rows;
//! - on a frame after it stopped (the same window twice), the window's remaining rows up to one
//!   full lane, so the worker keeps preparing while the app idles. No frame is scheduled for this:
//!   idle draws zero frames, and a list that stops in the very frame before idle waits for the
//!   next one.
use crate::catalog::{Catalog, Route, Row, SongColumn as Column};
use std::sync::Arc;

pub const MAX_ROWS: usize = 256;
/// Rows a moving list's frame hands to `Ui::prewarm_text`: about one frame of installs.
const ROWS_PER_FRAME: usize = 4;
/// Requests one prepare-queue lane holds in flight (pxui `QUEUE_ROWS`): a stopped list's budget.
const LANE_REQUESTS: usize = 128;
/// Requests a row can make: every column but State and Actions.
const ROW_CELLS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    library: usize,
    view: usize,
    route: Route,
    widths: [f32; 8],
    density: f32,
    current: Option<i64>,
    /// rows the list built (shaped, or requested by the list itself)
    built: (usize, usize),
    first: usize,
    end: usize,
}
impl Window {
    pub fn route(&self) -> Route {
        self.route
    }
    pub fn rows(&self) -> std::ops::Range<usize> {
        self.first..self.end
    }
    pub(crate) fn new(
        catalog: &Catalog,
        route: Route,
        widths: [f32; 8],
        density: f32,
        current: Option<i64>,
        first: usize,
        end: usize,
    ) -> Option<Self> {
        let collection = catalog.collection(route)?;
        if first >= end || first >= collection.rows.len() {
            return None;
        }
        let built = (first, end.min(collection.rows.len()));
        let visible = end.saturating_sub(first).clamp(1, MAX_ROWS / 4);
        let first = first.saturating_sub(visible);
        let end = end
            .saturating_add(visible * 3)
            .min(collection.rows.len())
            .min(first + MAX_ROWS);
        Some(Self {
            library: Arc::as_ptr(&catalog.library) as usize,
            view: catalog
                .view
                .as_ref()
                .map_or(0, |view| Arc::as_ptr(view) as usize),
            route,
            widths,
            density,
            current,
            built: (built.0, built.1.min(end)),
            first,
            end,
        })
    }
    pub fn matches(&self, catalog: &Catalog, route: Route) -> bool {
        self.route == route
            && self.library == Arc::as_ptr(&catalog.library) as usize
            && self.view
                == catalog
                    .view
                    .as_ref()
                    .map_or(0, |view| Arc::as_ptr(view) as usize)
    }
    pub fn same_layout(&self, other: &Self) -> bool {
        self.library == other.library
            && self.view == other.view
            && self.route == other.route
            && self.widths == other.widths
            && self.density == other.density
            && self.current == other.current
    }
    /// Requests every cell layout of row `position` the table measures or draws. A refused hint
    /// (text over the queue's 16 KiB) is dropped: that cell shapes when shown.
    fn prewarm_row(&self, ui: &mut pxui::Ui, catalog: &Catalog, position: usize) {
        let Some(&index) = catalog.collection(self.route).and_then(|c| c.rows.get(position)) else {
            return;
        };
        let row = catalog.row(index);
        let current = self.current == Some(row.id);
        let numbered = matches!(self.route, Route::Album(_));
        let mut digits = [0u8; 24];
        let number = crate::shell::stack_fmt(&mut digits, format_args!("{}", position + 1));
        for column in Column::ALL {
            let column_width = self.widths[column as usize];
            if column_width <= 0.0
                || matches!(column, Column::State | Column::Actions)
                || (column == Column::Number && current)
            {
                continue;
            }
            let available =
                (column_width - if column == Column::Number { 40.0 } else { 16.0 }).max(0.0);
            let label = body_label(column, current);
            let value = body_value(&row, column, numbered, number);
            let mut width = available;
            if matches!(column, Column::Number | Column::Format | Column::Time) {
                let natural = catalog.song_body_width(index, column, position + 1, numbered);
                if natural > available {
                    // An overflowing cell draws at its measured width: request the measurement; the
                    // drawing layout shapes when the row is shown.
                    let _ = ui.prewarm_text(value, &label, available.max(0.01));
                    continue;
                }
                width = (natural * self.density).ceil() / self.density;
            }
            let _ = ui.prewarm_text(value, &label, width);
        }
    }
}

/// The rows of one table layout already requested: a range that grows from the built rows toward
/// both ends of the window, below and above in turn. A new layout (widths, density, playing row,
/// catalog or route) starts again from the rows its frame built.
#[derive(Debug, Default)]
pub struct Prewarm {
    layout: Option<Window>,
    done: std::ops::Range<usize>,
}
impl Prewarm {
    /// Between frames, on the UI's thread: request the next rows of `window` (the previous
    /// frame's, `ShellState::song_text_window`). `None` or a window of another catalog forgets
    /// progress. Allocates: never.
    pub fn request(&mut self, ui: &mut pxui::Ui, catalog: &Catalog, window: Option<Window>) {
        let Some(window) = window.filter(|window| window.matches(catalog, window.route)) else {
            self.layout = None;
            return;
        };
        let rows = window.rows();
        let done = self.done.start.max(rows.start)..self.done.end.min(rows.end);
        let built = window.built.0..window.built.1;
        // Union only when the requested rows still touch the built ones: no gap is marked done.
        self.done = if self.layout.is_some_and(|layout| layout.same_layout(&window))
            && done.start < done.end
            && built.start <= done.end
            && done.start <= built.end
        {
            done.start.min(built.start)..done.end.max(built.end)
        } else {
            built
        };
        let stopped = self.layout == Some(window);
        self.layout = Some(window);
        // Counted as if every cell were requested: a row never pushes a stopped frame past the lane.
        let mut budget = if stopped { LANE_REQUESTS } else { ROWS_PER_FRAME * ROW_CELLS };
        while budget >= ROW_CELLS && (self.done.end < rows.end || self.done.start > rows.start) {
            if self.done.end < rows.end {
                window.prewarm_row(ui, catalog, self.done.end);
                self.done.end += 1;
                budget -= ROW_CELLS;
            }
            if budget >= ROW_CELLS && self.done.start > rows.start {
                self.done.start -= 1;
                window.prewarm_row(ui, catalog, self.done.start);
                budget -= ROW_CELLS;
            }
        }
    }
}

const NUMBER: pxui::Label = pxui::Label::MONO.size(13.0).line_height(18.2).wrap_spaces();
const MONO_CELL: pxui::Label = pxui::Label::BODY.font(1).size(12.0).line_height(16.8).wrap_spaces();
const CELL: pxui::Label = pxui::Label::BODY.line_height(19.6).weight(400).lines(1).ellipsis();
const CURRENT_CELL: pxui::Label = CELL.weight(500);

/// The authoring style of a song cell: drawn by the table and requested ahead by `Prewarm`.
pub(crate) fn body_label(column: Column, current: bool) -> pxui::Label {
    match column {
        Column::Number => NUMBER,
        Column::Format | Column::Time => MONO_CELL,
        _ if current => CURRENT_CELL,
        _ => CELL,
    }
}
pub(crate) fn body_value<'a>(
    row: &Row<'a>,
    column: Column,
    numbered: bool,
    number: &'a str,
) -> &'a str {
    match column {
        Column::Number if numbered && row.track_number.is_some() => row.track_label,
        Column::Number => number,
        Column::Title => row.title,
        Column::Artist => row.artist,
        Column::Album => row.album,
        Column::Format => {
            if row.has_format {
                row.format
            } else {
                "—"
            }
        }
        Column::Time => row.duration,
        _ => "",
    }
}

#[cfg(test)]
mod resolve_tests {
    use pxui::Label;

    /// LocalWave's tracked labels (tracking in ems) resolve to the point values the TextStyle literals used before they were labels.
    #[test]
    fn tracked_labels_keep_their_points() {
        let ui = crate::test_ui(pxui::Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        let tight = crate::shell::TIGHT_3;
        for (label, size, points) in [
            (Label::BODY.size(24.0).tracking(tight), 24.0f32, -0.72f32),
            (Label::BODY.size(40.0).tracking(tight), 40.0, -1.2),
            (Label::BODY.size(19.0).tracking(-0.015), 19.0, -0.285),
            (Label::BODY.size(34.0).tracking(-0.025), 34.0, -0.85),
            (Label::BODY.size(32.0).tracking(-0.035), 32.0, -1.12),
            (Label::BODY.size(48.0).tracking(-0.025), 48.0, -1.2),
            (Label::BODY.size(26.0).tracking(-0.02), 26.0, -0.52),
        ] {
            let resolved = ui.resolve(&label);
            assert_eq!((resolved.size_points, resolved.tracking.to_bits()), (size, points.to_bits()), "{size} pt: {} not {points}", resolved.tracking);
        }
        // -0.2 points at 20 has no em whose product is that f32: -0.01 em resolves one f32 step from it
        let add_music = ui.resolve(&Label::BODY.size(20.0).tracking(-0.01)).tracking;
        assert_eq!(add_music.to_bits().abs_diff((-0.2f32).to_bits()), 1, "{add_music}");
    }
}

#[cfg(test)]
mod prewarm_tests {
    use super::{Prewarm, Window};
    use crate::catalog::Route;

    /// Rows grow from the built rows, below then above, without allocating: `ROWS_PER_FRAME` a frame
    /// while the list moves, the remainder up to one lane on a frame after it stopped; a jump past
    /// the requested rows or a new layout starts again from the rows its frame built.
    #[test]
    fn prewarm_grows_from_built_rows_bursts_when_stopped_and_restarts_after_a_jump_or_layout_change() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = crate::tests::prepared(
            (0..300)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Track {id:05}"),
                    artist: "Example artist".into(),
                    album: "Example album".into(),
                    duration: Some(240.0),
                    ..Default::default()
                })
                .collect(),
        );
        let mut ui = crate::test_ui(pxui::Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.frame(&pxui::FrameIn { width: 1440.0, height: 872.0, ..Default::default() }, &mut |_| {}).unwrap();
        let worker = pxui::TextWorker::new().unwrap();
        ui.set_text_worker(Some(&worker)).unwrap();
        let window = |density: f32, first: usize| {
            Window::new(&catalog, Route::Songs, [100.0; 8], density, None, first, first + 20)
        };
        let mut prewarm = Prewarm::default();
        let mut step = |prewarm: &mut Prewarm, window: Option<Window>| {
            mem_sys::local_reset();
            prewarm.request(&mut ui, &catalog, window);
            assert_eq!(mem_sys::local_alloc_count(), 0, "prewarm requests are fixed-buffer copies");
            prewarm.done.clone()
        };
        assert_eq!(window(1.0, 100).unwrap().rows(), 80..180);
        let moving = super::ROWS_PER_FRAME;
        let stopped = super::LANE_REQUESTS / super::ROW_CELLS;
        assert_eq!(stopped, 21);
        assert_eq!(step(&mut prewarm, window(1.0, 100)), 98..122, "a new window: {moving} rows");
        assert_eq!(step(&mut prewarm, window(1.0, 101)), 96..124, "moving: {moving} rows");
        // Stopped: the same window again requests the remainder, bounded by one lane (21 rows of six cells).
        assert_eq!(step(&mut prewarm, window(1.0, 101)), 86..135, "stopped: {stopped} rows");
        assert_eq!(step(&mut prewarm, window(1.0, 101)), 81..151);
        assert_eq!(step(&mut prewarm, window(1.0, 101)), 81..172);
        assert_eq!(step(&mut prewarm, window(1.0, 101)), 81..181, "the whole window, never past it");
        assert_eq!(step(&mut prewarm, window(1.0, 101)), 81..181, "nothing left to request");
        assert_eq!(step(&mut prewarm, window(1.0, 130)), 110..185, "a scroll inside the requested rows keeps them");
        assert_eq!(step(&mut prewarm, window(1.0, 200)), 198..222, "a jump past the requested rows restarts");
        assert_eq!(step(&mut prewarm, window(1.0, 202)), 196..224, "a small scroll keeps what it requested");
        assert_eq!(step(&mut prewarm, window(2.0, 202)), 200..224, "a new layout restarts");
        step(&mut prewarm, None);
        assert!(prewarm.layout.is_none());
    }
}
