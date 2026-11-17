//! prepare_tables on the library worker prepares column preferences, numeric widths, pins, action
//! labels and per-song occurrence keys. Frames select maxima in bounded work, never scanning
//! collections or constructing text engines. Formats compare original case and render lowercase.
//! Stable occurrence scopes survive sorting; duplicate queue entries retain distinct visual state.
//! Playing bars and the 2 Hz pending-copy pulse use retained pxui repeat keyframes; ticks never
//! rebuild row content. Start on identity/activity changes, stop when hidden/paused, and show static
//! bars/left-half fill with reduced motion. File-state pops use the 240 ms spring without initial animation. Preserve responsive
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
use crate::song_text::{body_label, body_line_height, body_value};

fn label(
    ui: &mut Ui,
    key: impl Into<pxui::Key>,
    value: &str,
    rect: Rect,
    style: Label,
    color: Rgba,
    role: Role,
) -> Handle {
    let node = ui.node(
        key.into(),
        Node {
            at: Some((rect.x, rect.y)).into(),
            look: Look::NONE.text(color),
            ..Node::column().w_px(rect.w.max(0.0)).h_px(rect.h).role(role)
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
    let mut digits = [0u8; 24];
    let number_label = stack_fmt(&mut digits, format_args!("{number}"));
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
            if matches!(column, Column::Number | Column::Format | Column::Time) {
                // the cell's own label: measuring keys match drawing and prewarm
                table_text_size(
                    ui,
                    body_value(&row, column, numbered, number_label),
                    available,
                    catalog.song_body_width(index, column, number, numbered),
                    &body_label(column, current),
                    body_line_height(column),
                )
            } else {
                (available, body_line_height(column))
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
    intent: &mut Option<Intent>,
) -> Handle {
    let root = ui.node(
        "song-columns".into(),
        Node {
            look: PAPER_FILL,
            ..Node::column().w(Size::Grow).h_px(layout.height)
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
                let node = Node {
                    at: Some((x + left, 10.0 + (layout.height - 20.5 - height) / 2.0)).into(),
                    look: SECONDARY_TEXT,
                    ..Node::column()
                        .w_px(available)
                        .h_px(height)
                        .role(if column.sort().is_some() { Role::Button } else { Role::Label })
                };
                let value = heading_value(column, applied);
                let style = TABLE_HEADING;
                // The Time heading is a right-aligned child label; the others are the node's own line box.
                let time_label = |ui: &mut Ui| {
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
                        ui.color(SECONDARY),
                        Role::Unknown,
                    );
                };
                // Only sortable headings are buttons; the rest are plain labels.
                let (button, clicked) = if column.sort().is_some() {
                    let heading = ui.pressable(column as i64, node, &PLAIN, |ui, _| {
                        if column == Column::Time {
                            time_label(ui);
                        }
                    });
                    (heading.handle, heading.clicked)
                } else {
                    let heading = ui.node((column as i64).into(), node);
                    if column == Column::Time {
                        ui.within(&heading, &mut |ui| time_label(ui));
                    }
                    (heading, false)
                };
                if column != Column::Time {
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
                    if clicked {
                        *intent = Some(Intent::Sort(sort));
                    }
                }
            }
            x += w;
        }
    });
    ui.draw_over(&root, move |paint, rect| {
        paint.fill_fractional(rect.x, (rect.y + rect.h).floor(), rect.w, 1.0, paint.color(LINE))
    });
    root
}

pub(super) fn table(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let Some(collection) = state.collection(catalog) else {
        header_view(ui, catalog, state, width, intent);
        return;
    };
    let Some(table) = &collection.song_table else {
        header_view(ui, catalog, state, width, intent);
        ui.label("Loading songs…", TEXT_14.grow().h_px(40.0).color(SECONDARY));
        return;
    };
    let applied = catalog
        .view
        .as_ref()
        .filter(|view| Some(view.request.scope) == catalog.scope(state.route))
        .map(|view| (view.request.sort, view.request.ascending));
    let numbered = matches!(state.route, Route::Album(_));
    let detail = matches!(state.route, Route::Album(_) | Route::Artist(_));
    let leading = if detail {
        super::detail::height(
            width,
            state.route,
            match state.route {
                Route::Artist(index) => catalog.artist_albums.get(index).map_or(0, Vec::len),
                _ => 0,
            },
            state.collection(catalog).is_some_and(|group| group.folder.is_some()),
        )
    } else {
        105.6
    };
    let widths = table.columns(
        catalog,
        width,
        player.current_id.and_then(|id| catalog.row_index(id)),
        applied.map_or(Sort::Original, |value| value.0),
        state.route,
    );
    let heading_layout = heading_layout(ui, catalog, widths, applied);
    let revision = crate::hash((
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        widths.map(f32::to_bits),
    ));
    let entry = crate::motion::Page::new(ui, state.entry_revision);
    let find = state.route == Route::Find;
    // Swift shows no empty text for Find results; an empty query gets its hint instead.
    let hint = find && state.query.as_str().trim().is_empty();
    let placeholder = collection.rows.is_empty() && (!find || hint);
    if find {
        header_view(ui, catalog, state, width, intent);
    }
    let scope = ui.node("song-results-scope".into(), Node::column().w(Size::Grow).h(Size::Grow));
    let (mut first, mut end) = (usize::MAX, 0);
    ui.within(&scope, &mut |ui| {
        let results = ui.node(
            Key::Id(if find { state.query_revision as i64 } else { 0 }),
            Node::column().w(Size::Grow).h(Size::Grow),
        );
        ui.within(&results, &mut |ui| {
            let list_key = pxui::ui::key_of(results.box_key, "songs");
            let mut viewport = Viewport {
                leading: if find { 0.0 } else { leading },
                heading: heading_layout.height,
                offset: 0.0,
                height: 0.0,
            };
            let list = ui.section_list(
                "songs",
                &SectionList::new(if placeholder { 1 } else { collection.rows.len() }).row_h(if placeholder { 83.6 } else { 38.0 }).lead(if find { 0.0 } else { leading }).header(heading_layout.height).overlay_scrollbar().measured(),
                &|i| if placeholder { 0 } else { collection.row_keys[i] },
                &mut |ui, part, content_y| match part {
                    SectionPart::Leading if detail => {
                        super::detail::header(ui, catalog, state, player, width, intent)
                    }
                    SectionPart::Leading => header_view(ui, catalog, state, width, intent),
                    SectionPart::Header => {
                        let list = ui.box_of_key(list_key).unwrap();
                        viewport.offset = ui.scroll(&list).painted;
                        viewport.height = ui.rect(&list).h;
                        heading(ui, widths, applied, heading_layout, intent);
                    }
                    SectionPart::Row(_) if placeholder => {
                        ui.column("empty", Node::column().w(Size::Grow).h_px(83.6).pad(32.0), |ui| {
                            ui.label(if hint { onboarding::FIND_HINT } else if catalog.is_empty() { onboarding::LIBRARY_EMPTY } else { "No songs here yet." }, BODY_DIM.grow());
                        });
                    }
                    SectionPart::Row(index) => {
                        first = first.min(index);
                        end = end.max(index + 1);
                        let identity = ui.node(
                            Key::Id(collection.row_keys[index]),
                            Node::column().w(Size::Grow).h(Size::Fit),
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
                                entry: &entry,
                                viewport,
                            }
                            .row(ui, index, content_y, intent)
                        });
                    }
                },
            );
            if state.reveal_selected.take()
                && let Some(row) = state.selected
            {
                ui.reveal(&list, row);
            }
            // Keep a list-row drag alive while its source row scrolls out of the virtual window.
            if ui
                .carrying()
                .is_some_and(|payload| {
                    payload.kind.starts_with(LIST_ROW) || payload.kind.starts_with(SONG_DRAG)
                })
            {
                ui.transfer_carry(&list);
            }
        });
    });
    state.song_window.set(crate::song_text::Window::new(
        catalog,
        state.route,
        widths,
        ui.density() as f32,
        player.current_id,
        first,
        end,
    ));
}

pub(in crate::shell) mod row;
use row::*;
pub(super) use row::SONG_DRAG;

/// A song drag hovering or dropped on `target`: (dropped, dragged row's song id).
pub(super) fn song_drop(ui: &mut Ui, target: &Handle) -> Option<(bool, i64)> {
    let (dropped, payload) = match ui.drop_target(target)? {
        pxui::DropRef::Hover(p) => (false, p),
        pxui::DropRef::Dropped(p) => (true, p),
    };
    let id = payload.kind.strip_prefix(SONG_DRAG)?.parse().ok()?;
    Some((dropped, id))
}
mod menu;
pub(super) use menu::{list_rows, popup, LISTS_MENU, LIST_ROWS, MENU, NEW_LIST};
#[cfg(test)]
mod tests;
