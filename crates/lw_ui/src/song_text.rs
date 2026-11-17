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
    ) -> Result<[Option<(u64, PreparedText)>; ROW_LAYOUTS], pxui::Error> {
        let collection = catalog.collection(self.route).ok_or(pxui::Error::InvalidInput)?;
        let index = *collection.rows.get(position).ok_or(pxui::Error::InvalidInput)?;
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

/// The authoring style of a song cell, drawn by the UI; `body_style` is the same style resolved for the worker's shaper, and
/// `Ui::resolve(&body_label(..))` equals it bit for bit (`body_label_resolves_to_the_worker_style`).
pub(crate) fn body_label(column: Column, current: bool) -> pxui::Label {
    use pxui::Label;
    let number = column == Column::Number;
    let mono = matches!(column, Column::Format | Column::Time);
    if number {
        Label::MONO.size(13.0).line_height(18.2).wrap_spaces()
    } else if mono {
        Label::BODY.font(1).size(12.0).line_height(16.8).wrap_spaces()
    } else {
        Label::BODY.line_height(19.6).weight(if current { 500 } else { 400 }).lines(1).ellipsis()
    }
}

/// The resolved shaping style of a song cell, built on the worker (no `Ui` there): equal to `Ui::resolve(&body_label(..))`.
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

#[cfg(test)]
mod resolve_tests {
    use super::{body_label, body_style};
    use crate::catalog::SongColumn as Column;
    use pxui::{Label, TextStyle};

    /// Every field of a resolved style, floats as bits: equal means the shaper sees the same style (and the same layout key).
    fn bits(style: &TextStyle) -> (u16, u16, u32, u32, u32, u32, u16, u8, u8, u32, u32) {
        (style.font, style.weight, style.size_points.to_bits(), style.line_height.to_bits(), style.wrap, style.align, style.max_lines,
            style.italic, style.ellipsis, style.features, style.tracking.to_bits())
    }

    /// The UI draws song cells with `body_label`; the worker prepares them with `body_style`. The two resolve bit for bit, so a
    /// worker-prepared layout is the one the UI's text finds. LocalWave's tracked labels (tracking in ems) resolve to the point
    /// values the TextStyle literals used before they were labels.
    #[test]
    fn body_label_resolves_to_the_worker_style_and_tracking_keeps_its_points() {
        let ui = crate::test_ui(pxui::Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        for column in Column::ALL {
            for current in [false, true] {
                assert_eq!(bits(&ui.resolve(&body_label(column, current))), bits(&body_style(column, current)), "{column:?} current {current}");
            }
        }
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
