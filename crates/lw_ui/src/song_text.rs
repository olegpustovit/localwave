//! Song-cell source/size policy for worker preparation through public pxui. Capture the actual
//! measured-list row window; prepare a bounded margin, never the whole library. Match numeric
//! wrapping, density rounding and the exact style/width used by ordinary line-box nodes.
use crate::catalog::{Catalog, Route, Row, SongColumn as Column};
use pxui::{PreparedText, Text};
use std::sync::Arc;

pub const MAX_ROWS: usize = 256;
pub const ROW_LAYOUTS: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    library: usize,
    view: usize,
    route: Route,
    widths: [f32; 8],
    density: f32,
    current: Option<i64>,
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
    /// Worker only. Return all possible measurement/drawing layouts for this occurrence.
    pub fn prepare_row(
        &self,
        catalog: &Catalog,
        position: usize,
        text: &mut Text,
    ) -> Result<[Option<(u64, PreparedText)>; ROW_LAYOUTS], i32> {
        let collection = catalog.collection(self.route).ok_or(-1)?;
        let index = *collection.rows.get(position).ok_or(-1)?;
        let row = catalog.row(index);
        let current = self.current == Some(row.id);
        let numbered = matches!(self.route, Route::Album(_));
        let number = (position + 1).to_string();
        let mut records = [const { None }; ROW_LAYOUTS];
        let mut next = 0;
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
            let style = body_style(column, current);
            let value = body_value(&row, column, numbered, &number);
            let mut width = available;
            if matches!(column, Column::Number | Column::Format | Column::Time) {
                let natural = catalog.song_body_width(index, column, position + 1, numbered);
                width = if natural > available {
                    let record = text.prepare(value, &style, available.max(0.01), &[])?;
                    let width = record.metrics().width;
                    records[next] = Some((
                        crate::hash((column as u8, current, available.max(0.01).to_bits(), value)),
                        record,
                    ));
                    next += 1;
                    width
                } else {
                    natural
                };
                width = (width * self.density).ceil() / self.density;
            }
            records[next] = Some((
                crate::hash((column as u8, current, width.to_bits(), value)),
                text.prepare(value, &style, width, &[])?,
            ));
            next += 1;
        }
        Ok(records)
    }
}

pub(crate) fn body_style(column: Column, current: bool) -> pxui::TextStyle {
    let number = column == Column::Number;
    let mono = matches!(column, Column::Format | Column::Time);
    let wrap = number || mono;
    pxui::TextStyle {
        font: u16::from(mono),
        size_points: if number {
            13.0
        } else if mono {
            12.0
        } else {
            14.0
        },
        line_height: if number {
            18.2
        } else if mono {
            16.8
        } else {
            19.6
        },
        weight: if current && !number && !mono {
            500
        } else {
            400
        },
        features: if number {
            pxui::TextStyle::FEATURE_TABULAR_NUMBERS
        } else {
            0
        },
        wrap: if wrap {
            pxui::TextStyle::WRAP_WORD_SPACES
        } else {
            pxui::TextStyle::WRAP_NONE
        },
        max_lines: if wrap { 0 } else { 1 },
        ellipsis: if wrap {
            pxui::TextStyle::ELLIPSIS_NONE
        } else {
            pxui::TextStyle::ELLIPSIS_END
        },
        ..Default::default()
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
