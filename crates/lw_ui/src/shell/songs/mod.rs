//! prepare_tables on the library worker prepares column preferences, number labels and widths, pins, action
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
    role: Role,
) -> Handle {
    let node = ui.node(key, Node::column().at(rect.x, rect.y).w_px(rect.w.max(0.0)).h_px(rect.h).role(role).look(text_look(style)));
    ui.text_line_box(&node, value, style, &[]).unwrap();
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
    (number, number_label): (usize, &str),
    numbered: bool,
    widths: [f32; 8],
    current: bool,
) -> RowLayout {
    let row = catalog.row(index);
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

/// The heading's text for `column`, with the sort arrow while the songs are sorted by it (`sorted`: ascending).
fn heading_value(column: Column, sorted: Option<bool>) -> &'static str {
    match (column, sorted) {
        (Column::Title, Some(true)) => "Title ↑",
        (Column::Title, Some(false)) => "Title ↓",
        (Column::Artist, Some(true)) => "Artist ↑",
        (Column::Artist, Some(false)) => "Artist ↓",
        (Column::Album, Some(true)) => "Album ↑",
        (Column::Album, Some(false)) => "Album ↓",
        (Column::Format, Some(true)) => "Format ↑",
        (Column::Format, Some(false)) => "Format ↓",
        (Column::Time, Some(true)) => "Time ↑",
        (Column::Time, Some(false)) => "Time ↓",
        _ => column.title(),
    }
}
/// The heading's height: its tallest (wrapped) title between 10 points above and 10.5 below.
fn heading_height(
    ui: &mut Ui,
    catalog: &Catalog,
    widths: [f32; 8],
    applied: Option<(Sort, bool)>,
) -> f32 {
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
        let size = heading_size(
            ui,
            heading_value(column, selected),
            available,
            catalog.song_heading_width(column, selected),
        );
        height = height.max(size.1);
    }
    height + 20.5
}
/// LocalWave's table heading (registered as `localwave-heading`): a song column's (`SONG_HEADING`) as below, any other the kit's
/// built-in heading (U2b.4.36). The song heading is Swift's
/// 12-point medium secondary heading wrapping at words, 32 points in on "#" and 8 on the others, 8 from the column's end, centred down
/// the heading between 10 points above and 10.5 below; Time is fit to its pixel-rounded width and ends at its column's end; the sorted
/// column shows its arrow.
pub(super) fn heading_body(ui: &mut Ui, c: &pxui::bodies::TableHeadConfig) {
    use pxui::bodies::BodyConfig;
    if c.variant != super::theme::variant::SONG_HEADING {
        ui.slot(c.built_in);
        return;
    }
    let Some(&column) = Column::ALL.get(c.index) else { return };
    let left = if column == Column::Number { 32.0 } else { 8.0 };
    // 7 at the end: the heading row's 1-point separator follows the heading, a point narrower than its column (`heading_columns`)
    let layout = ui.node_layout(&c.node).padding(Edges::new(left, 10.0, 7.0, 10.5));
    ui.set_node_layout(&c.node, &if column == Column::Time { layout.justify(Justify::End) } else { layout });
    if c.text_of(&c.title).is_empty() {
        return;
    }
    let value = heading_value(column, c.ascending());
    let style = TABLE_HEADING.color(SECONDARY).role(Role::Unknown);
    let Size::Px(w) = layout.w else { return };
    let _label = if column == Column::Time {
        // Swift's trailing heading: its natural width rounded to pixels, as tall as it wraps in the column (the heading is as wide
        // as its column less the 1-point separator)
        let natural = ui.text_service().measure(value, style, None).map_or(0.0, |m| m.width);
        let (fit, h) = heading_size(ui, value, (w + 1.0 - 16.0).max(0.0), natural);
        ui.label(value, style.w_px(fit).h_px(h))
    } else {
        // the text's own width (inside the padding): a pixel-wide label keeps its layout across frames
        ui.label(value, style.w_px((w - left - 7.0).max(0.0)))
    };
    #[cfg(test)]
    if column == Column::Time {
        tests::time_heading_built(_label);
    }
}

/// The desktop songs list: multi-selection, measured rows, the overlay scrollbar, LocalWave's song rows, no frame.
const SONGS: pxui::SelectionList = pxui::SelectionList::DEFAULT.mode(pxui::SelectMode::Multi).row_h(38.0).measured().overlay_scrollbar().variant(super::theme::variant::SONG_ROW).bare();

/// The song columns' headings this frame: every column (hidden ones 0 wide), the sortable ones sort keys other than the original order.
fn heading_columns(widths: [f32; 8]) -> [pxui::Column<'static>; 8] {
    std::array::from_fn(|i| {
        let column = Column::ALL[i];
        // each heading is followed by the heading row's 1-point separator: a point narrower, it starts where the rows' column does
        let w = if widths[i] > 0.0 { widths[i] - 1.0 } else { 0.0 };
        pxui::Column::new(if column == Column::Actions { "" } else { column.title() }).id(i as i64).w_px(w).grip(false).sortable(column.sort().is_some_and(|s| s != Sort::Original))
    })
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
    let heading_height = heading_height(ui, catalog, widths, applied);
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
    let scope = ui.node("song-results-scope", Node::column().w(Size::Grow).h(Size::Grow));
    // the rows the list declares this frame (kept ones too: every row in its window is keyed), for the text prepared ahead
    let (first, end) = (std::cell::Cell::new(usize::MAX), std::cell::Cell::new(0));
    ui.within(&scope, &mut |ui| {
        let results = ui.node(
            Key::Id(if find { state.query_revision as i64 } else { 0 }),
            Node::column().w(Size::Grow).h(Size::Grow),
        );
        ui.within(&results, &mut |ui| {
            let mut viewport = Viewport {
                leading: if find { 0.0 } else { leading },
                heading: heading_height,
                offset: 0.0,
                height: 0.0,
            };
            // the heading shows the applied sort; a click cycles it (none, ascending, descending) and the app sorts
            state.song_table.borrow_mut().sort = applied.filter(|(sort, _)| *sort != Sort::Original).and_then(|(sort, up)| {
                let id = Column::ALL.iter().position(|c| c.sort() == Some(sort))? as i64;
                Some((id, if up { pxui::Sort::Ascending } else { pxui::Sort::Descending }))
            });
            let columns = heading_columns(widths);
            let rows = Rows { catalog, collection, table, state, player, widths, revision, numbered, entry: &entry, viewport };
            let reorder = rows.reorderable();
            let count = if placeholder { 0 } else { collection.rows.len() };
            let list = SONGS.lead(if find { 0.0 } else { leading }).header(heading_height).trail(if placeholder { 83.6 } else { 0.0 }).reorder(reorder);
            let mut song_rows = state.song_rows.borrow_mut();
            // a kept row shows what these hold; the playing and copying rows follow their place on screen, so they rebuild every frame
            song_rows.revision = Some(crate::hash((revision, player.current_id, reorder, state.entry_revision)));
            {
                let (live, found) = &mut *state.song_live.borrow_mut();
                for &i in live.iter() { if let Some(f) = song_rows.rows.get_mut(i) { *f &= !pxui::ListState::LIVE; } }
                live.clear();
                for &i in found.iter() { if let Some(f) = song_rows.rows.get_mut(i) { *f |= pxui::ListState::LIVE; live.push(i); } }
                found.clear();
            }
            let r = ui.selection_list(
                "songs",
                count,
                &|i| {
                    first.set(first.get().min(i));
                    end.set(end.get().max(i + 1));
                    collection.row_keys[i]
                },
                &mut song_rows,
                &list,
                &mut |ui, part, cell| match part {
                    SectionPart::Leading if detail => {
                        super::detail::header(ui, catalog, state, player, width, intent)
                    }
                    SectionPart::Leading => header_view(ui, catalog, state, width, intent),
                    SectionPart::Trailing if placeholder => {
                        ui.id("empty", |ui| { ui.column(Node::column().w(Size::Grow).h_px(83.6).pad(32.0), |ui| {
                            ui.label(if hint { onboarding::FIND_HINT } else if catalog.is_empty() { onboarding::LIBRARY_EMPTY } else { "No songs here yet." }, BODY_DIM.grow());
                        }); });
                    }
                    SectionPart::Trailing => {}
                    SectionPart::Header => {
                        // the scroll box the list reported last frame (its rows' window for the playing marks)
                        if let Some(list) = ui.box_of_key(Key::Resolved(state.song_list.get())) {
                            viewport.offset = ui.scroll(&list).painted;
                            viewport.height = ui.rect(&list).h;
                        }
                        let mut table = state.song_table.borrow_mut();
                        // kept while the columns and the sort hold (the heading pins over the rows: a scroll moves it, nothing more)
                        let sort = table.sort.map(|(id, dir)| (id, dir == pxui::Sort::Ascending));
                        ui.view_of("song-heading", &(widths.map(f32::to_bits), sort, heading_height.to_bits()), |ui| {
                            // paper under a 1-point line rule: the theme's `table_head` look
                            let h = ui.table_head("song-columns", &columns, &mut table, &pxui::TableHead::DEFAULT.h_px(heading_height).variant(super::theme::variant::SONG_HEADING));
                            if h.sorted {
                                let by = table.sort.and_then(|(id, dir)| Some((Column::ALL[id as usize].sort()?, dir == pxui::Sort::Ascending)));
                                *intent = Some(Intent::SortBy(by));
                            }
                        });
                    }
                    SectionPart::Row(index) => {
                        Rows { viewport, ..rows }.row(ui, index, cell, intent);
                    }
                },
            );
            drop(song_rows);
            state.song_list.set(r.handle.box_key);
            if let Some(i) = r.selected
                && intent.is_none()
            {
                // a change that leaves no row selected (Escape) is the app's Clear (read on a change frame only)
                let none = !state.song_rows.borrow().rows.iter().any(|f| f & pxui::ListState::SELECTED != 0);
                *intent = Some(if none { Intent::Select(crate::Selection::Clear) } else { Intent::Selected(i) });
            }
            // Swift plays the whole selection; one row keeps its playback position
            if let Some(i) = r.activated {
                *intent = Some(if state.marked_count() > 1 { Intent::PlaySelection } else { Intent::PlayRow(i) });
            }
            if let (Some(i), Some(row)) = (r.secondary, r.row) {
                let id = catalog.row(collection.rows[i]).id;
                *intent = Some(Intent::SongMenu(i, id, collection.key, ui.signal(&row).pointer));
            }
            if let Some((from, to)) = r.moved {
                *intent = Some(Intent::MoveListRow { from, to });
            }
            let list = r.handle;
            // the window's default key target: keys no other widget holds reach the songs list
            ui.focus_default(&list);
            // Keep a song drag alive while its source row scrolls out of the virtual window.
            if ui
                .carrying()
                .is_some_and(|payload| payload.kind.starts_with(SONG_DRAG))
            {
                ui.transfer_carry(&list);
            }
        });
    });
    state.song_window.set(crate::song_text::Window::new(
        catalog,
        state.route,
        widths,
        ui.density(),
        player.current_id,
        first.get(),
        end.get(),
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
