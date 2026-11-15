//! prepare_tables on the library worker prepares column preferences, numeric widths, pins, action
//! labels and per-song occurrence keys. Frames select maxima in bounded work, never scanning
//! collections or constructing text engines. Formats compare original case and render lowercase.
//! Stable occurrence scopes survive sorting; duplicate queue entries retain distinct visual state.
//! Playing bars reuse glyphs and request frames only while visible/playing; pause/reduced motion
//! settles. File-state pops use the 240 ms spring without initial animation. Preserve responsive
//! thresholds, wrapped baselines and stale-popup guards.
//!
//! Page headings scroll with section content; Artists column headings pin and overlay thumbs
//! reserve no gutter. Retain headings independently from playback/artwork revisions, preserving row
//! identity, entrance motion and overscan. Keep full-width gallery rules, 172.6/105.6-point lead-
//! ins and fractional row strides. Numeric headings/cells may wrap; use measured rows and baseline
//! alignment.
//!
use super::*;
use crate::catalog::SongColumn as Column;

fn label(
    ui: &mut Ui,
    key: &str,
    value: &str,
    rect: Rect,
    style: pxui::TextStyle,
    color: Rgba,
    role: Role,
) -> Handle {
    let node = ui.node(
        key,
        Node {
            role,
            w: Size::Px(rect.w.max(0.0)),
            h: Size::Px(rect.h),
            at: Some((rect.x, rect.y)).into(),
            look: Look {
                text: color,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.text_line_box(&node, value, &style, &[]).unwrap();
    node
}
#[derive(Clone, Copy)]
struct RowLayout {
    sizes: [(f32, f32); 8],
    ascent: f32,
    descent: f32,
    height: f32,
}
fn baseline(column: Column) -> f32 {
    match column {
        Column::Number => 13.0,
        Column::Format | Column::Time => 12.0,
        Column::State => 10.0,
        _ => 14.0,
    }
}
impl RowLayout {
    fn y(self, baseline: f32) -> f32 {
        (self.height - self.ascent - self.descent) * 0.5 + self.ascent - baseline
    }
}
fn row_layout(
    ui: &mut Ui,
    catalog: &Catalog,
    index: usize,
    number: usize,
    numbered: bool,
    widths: [f32; 8],
    current: bool,
) -> RowLayout {
    let row = catalog.row(index);
    let number_label = ClockLabel::integer(number as u64);
    let number_label = std::str::from_utf8(number_label.bytes()).unwrap();
    let mut layout = RowLayout {
        sizes: [(0.0, 0.0); 8],
        ascent: 0.0,
        descent: 0.0,
        height: 38.0,
    };
    for column in Column::ALL {
        if widths[column as usize] <= 0.0 || column == Column::Actions {
            continue;
        }
        let available =
            (widths[column as usize] - if column == Column::Number { 40.0 } else { 16.0 }).max(0.0);
        let size = if column == Column::Number && current {
            (14.0, 11.0)
        } else if column == Column::State {
            (10.0, 10.0)
        } else {
            let style = body_style(column, current);
            if matches!(column, Column::Number | Column::Format | Column::Time) {
                table_text_size(
                    ui,
                    body_value(&row, column, numbered, number_label),
                    available,
                    catalog.song_body_width(index, column, number, numbered),
                    &style,
                )
            } else {
                (available, style.line_height)
            }
        };
        let base = if column == Column::Number && current {
            11.0
        } else {
            baseline(column)
        };
        layout.sizes[column as usize] = size;
        layout.ascent = layout.ascent.max(base);
        layout.descent = layout.descent.max(size.1 - base);
    }
    layout.height = layout.height.max(layout.ascent + layout.descent);
    layout
}

fn heading_value(column: Column, applied: Option<(Sort, bool)>) -> &'static str {
    match (
        column,
        applied.filter(|(sort, _)| Some(*sort) == column.sort() && *sort != Sort::Original),
    ) {
        (Column::Title, Some((_, true))) => "Title ↑",
        (Column::Title, Some(_)) => "Title ↓",
        (Column::Artist, Some((_, true))) => "Artist ↑",
        (Column::Artist, Some(_)) => "Artist ↓",
        (Column::Album, Some((_, true))) => "Album ↑",
        (Column::Album, Some(_)) => "Album ↓",
        (Column::Format, Some((_, true))) => "Format ↑",
        (Column::Format, Some(_)) => "Format ↓",
        (Column::Time, Some((_, true))) => "Time ↑",
        (Column::Time, Some(_)) => "Time ↓",
        _ => column.title(),
    }
}
#[derive(Clone, Copy)]
struct HeadingLayout {
    sizes: [(f32, f32); 8],
    height: f32,
}
fn heading_layout(
    ui: &mut Ui,
    catalog: &Catalog,
    widths: [f32; 8],
    applied: Option<(Sort, bool)>,
) -> HeadingLayout {
    let mut sizes = [(0.0, 16.8); 8];
    let mut height = 16.8f32;
    for column in Column::ALL {
        if widths[column as usize] <= 0.0 || column == Column::Actions {
            continue;
        }
        let available =
            (widths[column as usize] - if column == Column::Number { 40.0 } else { 16.0 }).max(0.0);
        let selected = applied
            .filter(|(sort, _)| Some(*sort) == column.sort() && *sort != Sort::Original)
            .map(|value| value.1);
        sizes[column as usize] = heading_size(
            ui,
            heading_value(column, applied),
            available,
            catalog.song_heading_width(column, selected),
        );
        height = height.max(sizes[column as usize].1);
    }
    HeadingLayout {
        sizes,
        height: height + 20.5,
    }
}
fn heading(
    ui: &mut Ui,
    widths: [f32; 8],
    applied: Option<(Sort, bool)>,
    layout: HeadingLayout,
    p: Palette,
    intent: &mut Option<Intent>,
) -> Handle {
    let root = ui.node(
        "song-columns",
        Node {
            w: Size::Grow,
            h: Size::Px(layout.height),
            look: Look {
                fill: p.paper,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let mut x = 0.0;
        for column in Column::ALL {
            let w = widths[column as usize];
            if w == 0.0 {
                continue;
            }
            if column != Column::Actions {
                let left = if column == Column::Number { 32.0 } else { 8.0 };
                let available = (w - left - 8.0).max(0.0);
                let (fit, height) = layout.sizes[column as usize];
                let button = ui.node(
                    column.title(),
                    Node {
                        flags: if column.sort().is_some() {
                            CLICKABLE | TAB_STOP
                        } else {
                            0
                        },
                        role: if column.sort().is_some() {
                            Role::Button
                        } else {
                            Role::Label
                        },
                        w: Size::Px(available),
                        h: Size::Px(height),
                        at: Some((x + left, 10.0 + (layout.height - 20.5 - height) / 2.0)).into(),
                        look: Look {
                            text: p.secondary,
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
                let value = heading_value(column, applied);
                let style = pxui::TextStyle {
                    size_points: 12.0,
                    line_height: 16.8,
                    weight: 500,
                    wrap: pxui::TextStyle::WRAP_WORD_SPACES,
                    ..Default::default()
                };
                if column == Column::Time {
                    ui.within(&button, &mut |ui| {
                        label(
                            ui,
                            "time-label",
                            value,
                            Rect {
                                x: available - fit,
                                y: 0.0,
                                w: fit,
                                h: height,
                            },
                            style,
                            p.secondary,
                            Role::Unknown,
                        );
                    });
                } else {
                    ui.text_line_box(&button, value, &style, &[]).unwrap();
                }
                if let Some(sort) = column.sort() {
                    let selected =
                        applied.filter(|(active, _)| *active == sort && sort != Sort::Original);
                    let action = match (sort, selected) {
                        (Sort::Original, _) => "Sort by original order",
                        (Sort::Title, None) => "Sort by Title",
                        (Sort::Title, Some((_, true))) => "Sort by Title, ascending",
                        (Sort::Title, _) => "Sort by Title, descending",
                        (Sort::Artist, None) => "Sort by Artist",
                        (Sort::Artist, Some((_, true))) => "Sort by Artist, ascending",
                        (Sort::Artist, _) => "Sort by Artist, descending",
                        (Sort::Album, None) => "Sort by Album",
                        (Sort::Album, Some((_, true))) => "Sort by Album, ascending",
                        (Sort::Album, _) => "Sort by Album, descending",
                        (Sort::Format, None) => "Sort by Format",
                        (Sort::Format, Some((_, true))) => "Sort by Format, ascending",
                        (Sort::Format, _) => "Sort by Format, descending",
                        (Sort::Duration, None) => "Sort by Time",
                        (Sort::Duration, Some((_, true))) => "Sort by Time, ascending",
                        (Sort::Duration, _) => "Sort by Time, descending",
                    };
                    ui.a11y_name(&button, action);
                    if ui.signal_state(&button).clicked {
                        *intent = Some(Intent::Sort(sort));
                    }
                }
            }
            x += w;
        }
    });
    ui.draw_over(&root, move |paint, rect| {
        paint.fill_fractional(rect.x, (rect.y + rect.h).floor(), rect.w, 1.0, p.line)
    });
    root
}

pub(super) fn table(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let Some(collection) = state.collection(catalog) else {
        header_view(ui, catalog, state, width, p, intent);
        return;
    };
    let Some(table) = &collection.song_table else {
        header_view(ui, catalog, state, width, p, intent);
        text(
            ui,
            "songs-loading",
            "Loading songs…",
            14,
            p.secondary,
            (Size::Grow, 40.0),
            false,
        );
        return;
    };
    let applied = catalog
        .view
        .as_ref()
        .filter(|view| Some(view.request.scope) == catalog.scope(state.route))
        .map(|view| (view.request.sort, view.request.ascending));
    let numbered = matches!(state.route, Route::Album(_));
    let widths = table.columns(
        catalog,
        width,
        player.current_id.and_then(|id| catalog.row_index(id)),
        applied.map_or(Sort::Original, |value| value.0),
        state.route,
    );
    let heading_layout = heading_layout(ui, catalog, widths, applied);
    let mut revision = FastHasher::default();
    (
        state.revision,
        player.current_id,
        player.playing,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        widths.map(f32::to_bits),
    )
        .hash(&mut revision);
    let revision = revision.finish();
    let entry = crate::motion::Page::new(ui, state.entry_revision);
    let find = state.route == Route::Find;
    if find {
        header_view(ui, catalog, state, width, p, intent);
    }
    let scope = ui.node(
        "song-results-scope",
        Node {
            w: Size::Grow,
            h: Size::Grow,
            ..Node::DEFAULT
        },
    );
    ui.within(&scope, &mut |ui| {
        let results = ui.node_keyed(
            int_key(
                scope.box_key,
                if find { state.query_revision as i64 } else { 0 },
            ),
            Node {
                w: Size::Grow,
                h: Size::Grow,
                ..Node::DEFAULT
            },
        );
        ui.within(&results, &mut |ui| {
            let list_key = Ui::key_of(results.box_key, "songs");
            let mut viewport = Viewport {
                leading: if find { 0.0 } else { 105.6 },
                heading: heading_layout.height,
                offset: 0.0,
                height: 0.0,
            };
            ui.section_list_measured(
                "songs",
                &SectionList {
                    count: if collection.rows.is_empty() && !find {
                        1
                    } else {
                        collection.rows.len()
                    },
                    row_height: if collection.rows.is_empty() && !find {
                        83.6
                    } else {
                        38.0
                    },
                    leading_height: if find { 0.0 } else { 105.6 },
                    header_height: heading_layout.height,
                    trailing_height: 0.0,
                    overlay_scrollbar: true,
                },
                &mut |ui, part, content_y| match part {
                    SectionPart::Leading => header_view(ui, catalog, state, width, p, intent),
                    SectionPart::Header => {
                        let list = ui.box_of_key(list_key).unwrap();
                        viewport.offset = ui.scroll_position(&list);
                        viewport.height = ui.rect(&list).h;
                        heading(ui, widths, applied, heading_layout, p, intent);
                    }
                    SectionPart::Row(_) if collection.rows.is_empty() => {
                        let empty = ui.node(
                            "empty",
                            Node {
                                w: Size::Grow,
                                h: Size::Px(83.6),
                                padding: Edges::all(32.0),
                                ..Node::DEFAULT
                            },
                        );
                        ui.within(&empty, &mut |ui| {
                            text(
                                ui,
                                "no-songs",
                                "No songs here yet.",
                                14,
                                p.secondary,
                                (Size::Grow, 19.6),
                                false,
                            );
                        });
                    }
                    SectionPart::Row(index) => {
                        let identity = ui.node_keyed(
                            int_key(list_key, collection.row_keys[index]),
                            Node {
                                w: Size::Grow,
                                h: Size::Fit,
                                ..Node::DEFAULT
                            },
                        );
                        ui.within(&identity, &mut |ui| {
                            Rows {
                                catalog,
                                collection,
                                state,
                                player,
                                widths,
                                revision,
                                numbered,
                                p,
                                entry: &entry,
                                viewport,
                            }
                            .row(ui, index, content_y, intent)
                        });
                    }
                },
            );
        });
    });
}

mod row;
use row::*;
mod menu;
pub(super) use menu::popup;
#[cfg(test)]
mod tests;
