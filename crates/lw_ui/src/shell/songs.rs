use super::*;
use crate::catalog::{Collection, Row, SongColumn as Column};

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

fn body_style(column: Column, current: bool) -> pxui::TextStyle {
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
fn body_value<'a>(row: &Row<'a>, column: Column, numbered: bool, number: &'a str) -> &'a str {
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

#[derive(Clone, Copy)]
struct Viewport {
    leading: f32,
    heading: f32,
    offset: f32,
    height: f32,
}

fn equalizer(ui: &mut Ui, playing: bool, visible: bool, id: i64, p: Palette) {
    let root = ui.node(
        "equalizer",
        Node {
            w: Size::Px(14.0),
            h: Size::Px(11.0),
            role: Role::Image,
            ..Node::DEFAULT
        },
    );
    ui.a11y_name(&root, if playing { "Playing" } else { "Paused" });
    let clock = ui.node(
        "equalizer-clock",
        Node {
            w: Size::Px(0.0),
            h: Size::Px(0.0),
            ..Node::DEFAULT
        },
    );
    let song = ui.node(
        "equalizer-song",
        Node {
            w: Size::Px(0.0),
            h: Size::Px(0.0),
            ..Node::DEFAULT
        },
    );
    let now = ui.now();
    if ui.state(&root, -1) != i64::from(playing) || ui.state(&song, i64::MIN) != id {
        ui.set_state(&root, i64::from(playing));
        ui.set_state(&song, id);
        ui.set_state(&clock, now.to_bits() as i64);
    }
    let elapsed = (now - f64::from_bits(ui.state(&clock, now.to_bits() as i64) as u64)).max(0.0);
    let animate = playing && !ui.tokens().reduced_motion;
    let heights = std::array::from_fn::<_, 3, _>(|index| {
        let phase = (elapsed / 0.9 + index as f64 / 3.0).fract() as f32;
        let target = if phase < 0.5 {
            phase * 2.0
        } else {
            (1.0 - phase) * 2.0
        };
        11.0 * if animate {
            0.4 + 0.6
                * Curve::Bezier {
                    duration: 1.0,
                    points: [0.42, 0.0, 0.58, 1.0],
                }
                .at(target)
        } else {
            0.4
        }
    });
    ui.draw(&root, move |paint, rect| {
        for (index, height) in heights.into_iter().enumerate() {
            paint.fill_fractional(
                rect.x + index as f32 * 4.0,
                rect.y + 11.0 - height,
                2.0,
                height,
                p.signal,
            );
        }
    });
    if animate && visible {
        ui.repaint_after(1.0 / 60.0);
    }
}

fn availability(ui: &mut Ui, row: Row<'_>, visible: bool, p: Palette) {
    use lw_core::{FileAvailability, FileState};
    let root = ui.node(
        "availability",
        Node {
            w: Size::Px(10.0),
            h: Size::Px(10.0),
            role: Role::Image,
            ..Node::DEFAULT
        },
    );
    let label = match row.state {
        FileState::CopyPending => "Preparing offline copy",
        FileState::Failed => "File unavailable — retry",
        _ => match row.availability {
            Some(FileAvailability::OnDevice) if row.pinned => "Kept offline",
            Some(FileAvailability::OnDevice) => "On this device",
            Some(FileAvailability::InCloud) => "In cloud — downloads when played",
            Some(FileAvailability::Unknown) => "Availability unknown",
            None => "Checking availability",
        },
    };
    ui.a11y_name(&root, label);
    let alternate = row.state == FileState::CopyPending && !ui.tokens().reduced_motion;
    let left = !alternate || (ui.now() * 2.0).floor() as i64 % 2 == 0;
    if alternate && visible {
        ui.repaint_after(0.5 - (ui.now() % 0.5));
    }
    let (state, available, pinned) = (row.state, row.availability, row.pinned);
    ui.draw(&root, move |paint, rect| {
        let (x, y) = (rect.x, rect.y);
        if available == Some(FileAvailability::OnDevice)
            && !matches!(state, FileState::CopyPending | FileState::Failed)
        {
            if pinned {
                paint.fill_fractional(x - 3.5, y - 3.5, 17.0, 17.0, p.ink);
                paint.fill_fractional(x - 2.0, y - 2.0, 14.0, 14.0, p.paper);
            }
            paint.fill_fractional(x, y, 10.0, 10.0, p.ink);
        } else {
            let unknown = available.is_none_or(|state| state == FileAvailability::Unknown)
                && !matches!(state, FileState::CopyPending | FileState::Failed);
            let ink = if matches!(state, FileState::CopyPending | FileState::Failed) {
                p.signal
            } else {
                p.secondary
            };
            if unknown {
                for start in [0.75, 4.75, 8.75] {
                    let length = 2.0f32.min(9.25 - start);
                    paint.fill_fractional(x + start, y + 0.25, length, 1.0, p.tertiary);
                    paint.fill_fractional(x + start, y + 8.75, length, 1.0, p.tertiary);
                    paint.fill_fractional(x + 0.25, y + start, 1.0, length, p.tertiary);
                    paint.fill_fractional(x + 8.75, y + start, 1.0, length, p.tertiary);
                }
            } else {
                paint.fill_fractional(x, y, 10.0, 1.5, ink);
                paint.fill_fractional(x, y + 8.5, 10.0, 1.5, ink);
                paint.fill_fractional(x, y + 1.5, 1.5, 7.0, ink);
                paint.fill_fractional(x + 8.5, y + 1.5, 1.5, 7.0, ink);
            }
            if state == FileState::CopyPending {
                paint.fill_fractional(x + if left { 0.0 } else { 5.0 }, y, 5.0, 10.0, p.signal);
            }
            if state == FileState::Failed {
                paint.line((x + 1.0, y + 9.0), (x + 9.0, y + 1.0), p.signal);
            }
        }
    });
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
    let mut revision = DefaultHasher::new();
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

struct Rows<'a> {
    catalog: &'a Catalog,
    collection: &'a Collection,
    state: &'a ShellState,
    player: &'a PlayerView<'a>,
    widths: [f32; 8],
    revision: u64,
    numbered: bool,
    p: Palette,
    entry: &'a crate::motion::Page,
    viewport: Viewport,
}
impl Rows<'_> {
    fn row(&self, ui: &mut Ui, index: usize, content_y: f32, intent: &mut Option<Intent>) {
        let Self {
            catalog,
            collection,
            state,
            player,
            widths,
            revision,
            numbered,
            p,
            entry,
            viewport,
        } = *self;
        let row = catalog.row(collection.rows[index]);
        let current = player.current_id == Some(row.id);
        let layout = row_layout(
            ui,
            catalog,
            collection.rows[index],
            index + 1,
            numbered,
            widths,
            current,
        );
        let y = content_y - viewport.offset + layout.y(11.0);
        let visible = y + 11.0 > (viewport.leading - viewport.offset).max(0.0) + viewport.heading
            && y < viewport.height;
        let visual_revision =
            if player.current_id == Some(row.id) || row.state == lw_core::FileState::CopyPending {
                let mut hash = DefaultHasher::new();
                (revision, visible).hash(&mut hash);
                hash.finish()
            } else {
                revision
            };
        let visual = ui.view(
            "song-visual",
            visual_revision,
            Size::Grow,
            Size::Fit,
            &mut |ui| {
                let row = catalog.row(collection.rows[index]);
                let root = ui.node(
                    "song",
                    Node {
                        flags: CLICKABLE | TAB_STOP,
                        role: Role::ListItem,
                        w: Size::Grow,
                        h: Size::Px(layout.height),
                        ..Node::DEFAULT
                    },
                );
                ui.a11y_name(&root, row.title);
                let signal = ui.signal_state(&root);
                ui.set_look(
                    &root,
                    Look {
                        fill: if state.selected == Some(index) {
                            p.selected
                        } else if signal.hovered {
                            Rgba { a: 153, ..p.panel }
                        } else {
                            p.paper
                        },
                        ..Look::NONE
                    },
                    State::default(),
                );
                let current = player.current_id == Some(row.id);
                ui.within(&root, &mut |ui| {
                    ui.view("song-copy", revision, Size::Grow, Size::Grow, &mut |ui| {
                        let mut x = 0.0;
                        for column in Column::ALL {
                            let w = widths[column as usize];
                            if w == 0.0 {
                                continue;
                            }
                            if !(column == Column::Number && current)
                                && !matches!(column, Column::State | Column::Actions)
                            {
                                let number = ClockLabel::integer((index + 1) as u64);
                                let value = body_value(
                                    &row,
                                    column,
                                    numbered,
                                    std::str::from_utf8(number.bytes()).unwrap(),
                                );
                                let (fit, h) = layout.sizes[column as usize];
                                let y = layout.y(baseline(column));
                                let left = if column == Column::Number { 32.0 } else { 8.0 };
                                label(
                                    ui,
                                    column.title(),
                                    value,
                                    Rect {
                                        x: x + if column == Column::Time {
                                            w - 8.0 - fit
                                        } else {
                                            left
                                        },
                                        y,
                                        w: fit,
                                        h,
                                    },
                                    body_style(column, current),
                                    if column == Column::Title {
                                        p.ink
                                    } else {
                                        p.secondary
                                    },
                                    Role::Label,
                                );
                            }
                            x += w;
                        }
                    });
                    if current {
                        let at = ui.node(
                            "equalizer-position",
                            Node {
                                w: Size::Px(14.0),
                                h: Size::Px(11.0),
                                at: Some((32.0, layout.y(11.0))).into(),
                                ..Node::DEFAULT
                            },
                        );
                        ui.within(&at, &mut |ui| {
                            ui.view(
                                "equalizer-view",
                                visual_revision,
                                Size::Grow,
                                Size::Grow,
                                &mut |ui| equalizer(ui, player.playing, visible, row.id, p),
                            );
                        });
                    }
                    if widths[Column::State as usize] > 0.0 {
                        let x = widths[..Column::State as usize].iter().sum::<f32>() + 8.0;
                        let at = ui.node(
                            "availability-position",
                            Node {
                                w: Size::Px(10.0),
                                h: Size::Px(10.0),
                                at: Some((x, layout.y(10.0))).into(),
                                ..Node::DEFAULT
                            },
                        );
                        ui.within(&at, &mut |ui| {
                            let visual = ui.view(
                                "availability-view",
                                visual_revision,
                                Size::Grow,
                                Size::Grow,
                                &mut |ui| availability(ui, row, visible, p),
                            );
                            let previous = ui.state(&visual, -1);
                            let next = row.state as i64;
                            if previous != next {
                                ui.set_state(&visual, next);
                                if previous >= 0 && !ui.tokens().reduced_motion {
                                    ui.animate_keys(
                                        &visual,
                                        &[
                                            pxui::kit::Keyframe {
                                                time: 0.0,
                                                target: pxui::kit::AnimationTarget::Scale {
                                                    x: 0.3,
                                                    y: 0.3,
                                                    anchor: (0.5, 0.5),
                                                },
                                                curve: crate::motion::spring(crate::motion::BASE),
                                            },
                                            pxui::kit::Keyframe {
                                                time: crate::motion::BASE,
                                                target: pxui::kit::AnimationTarget::Scale {
                                                    x: 1.0,
                                                    y: 1.0,
                                                    anchor: (0.5, 0.5),
                                                },
                                                curve: crate::motion::spring(crate::motion::BASE),
                                            },
                                        ],
                                    );
                                }
                            }
                        });
                    }
                    let x = widths[..7].iter().sum::<f32>();
                    let more = ui.node(
                        "actions",
                        Node {
                            flags: CLICKABLE | TAB_STOP,
                            role: Role::Button,
                            w: Size::Px(32.0),
                            h: Size::Px(32.0),
                            at: Some((x, (layout.height - 32.0) * 0.5)).into(),
                            ..Node::DEFAULT
                        },
                    );
                    ui.a11y_name(&more, row.actions_label);
                    ui.draw(&more, move |paint, rect| {
                        for x in [5.5, 12.0, 18.5] {
                            paint.circle(
                                (rect.x + 8.0 + x * (2.0 / 3.0), rect.y + 16.0),
                                1.4 * (2.0 / 3.0),
                                Some(p.secondary),
                                None,
                            );
                        }
                    });
                    if ui.signal_state(&more).clicked {
                        let at = ui.rect(&more);
                        *intent = Some(Intent::SongMenu(
                            index,
                            row.id,
                            collection.key,
                            (at.x, at.y + at.h),
                        ));
                    }
                });
                let last = index + 1 == collection.rows.len();
                ui.draw_over(&root, move |paint, rect| {
                    paint.fill_fractional(rect.x, rect.y.floor(), rect.w, 1.0, p.line);
                    if last {
                        paint.fill_fractional(
                            rect.x,
                            (rect.y + rect.h).floor(),
                            rect.w,
                            1.0,
                            p.line,
                        );
                    }
                });
                if signal.pressed && signal.button == Some(1) {
                    *intent = Some(Intent::SongMenu(
                        index,
                        row.id,
                        collection.key,
                        signal.pointer,
                    ));
                }
                if intent.is_none() && signal.clicked {
                    *intent = Some(Intent::SelectRow(index));
                }
                if signal.double_clicked {
                    *intent = Some(Intent::PlayRow(index));
                }
            },
        );
        entry.row(ui, &visual, index);
    }
}

pub(super) fn popup(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    target: (usize, i64, u64),
    at: (f32, f32),
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let present = state.collection(catalog).is_some_and(|collection| {
        collection.key == target.2 && collection.ids.get(target.0) == Some(&target.1)
    });
    let (w, h) = ui.view_size();
    let at = (
        at.0.clamp(0.0, (w - 200.0).max(0.0)),
        at.1.clamp(0.0, (h - 72.0).max(0.0)),
    );
    let mut accepted = false;
    let open = ui.popup(
        Some(p.line),
        None,
        false,
        false,
        at,
        200.0,
        72.0,
        "Song actions",
        &mut |ui| {
            for (label, action) in [
                ("Play", Intent::PlayRow(target.0)),
                ("Add to queue", Intent::AppendSong(target.1)),
            ] {
                let item = ui.node(
                    label,
                    Node {
                        flags: if present { CLICKABLE | TAB_STOP } else { 0 },
                        role: Role::MenuItem,
                        state: State {
                            disabled: !present,
                            ..State::default()
                        },
                        w: Size::Grow,
                        h: Size::Px(36.0),
                        padding: Edges {
                            l: 12.0,
                            r: 12.0,
                            ..Edges::ZERO
                        },
                        look: Look {
                            text: p.ink,
                            font: Font {
                                size: 13,
                                ..Font::BODY
                            },
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.text(&item, label);
                if present && ui.signal_state(&item).clicked {
                    *intent = Some(action);
                    accepted = true;
                }
            }
        },
    );
    if accepted {
        ui.dismiss_popup();
    }
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseSongMenu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::{FrameIn, Platform};

    fn catalog(count: usize) -> Catalog {
        crate::tests::prepared(
            (0..count)
                .map(|index| lw_core::Song {
                    id: Some(index as i64),
                    track_number: Some(if index == 0 { 9 } else { 12 }),
                    title: if index == 0 {
                        "Quiet lights".into()
                    } else {
                        "An exceptionally long song title".into()
                    },
                    artist: if index == 0 {
                        "The National".into()
                    } else {
                        "Another artist".into()
                    },
                    album: if index == 0 {
                        "A midnight album".into()
                    } else {
                        "An earlier record".into()
                    },
                    file_extension: Some(if index == 0 {
                        "FLAC".into()
                    } else {
                        "mp3".into()
                    }),
                    duration: Some(if index == 0 { 240.0 } else { 61.0 }),
                    ..Default::default()
                })
                .collect(),
        )
    }

    #[test]
    fn song_table_matches_native_columns_baselines_and_fitted_durations_at_both_densities() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog(2);
        let state = ShellState::default();
        let player = PlayerView::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        for round in 0..6 {
            mem_sys::local_reset();
            for scale in [1.0, 2.0] {
                for width in [399.0, 400.0, 519.0, 520.0, 679.0, 680.0, 888.0] {
                    ui.frame(
                        &FrameIn {
                            width: width + 552.0,
                            height: 872.0,
                            scale,
                            reduced_motion: 1,
                            ..Default::default()
                        },
                        &mut |ui| shell(ui, &player, &catalog, &state, &mut None),
                    )
                    .unwrap();
                    let row = ui
                        .a11y_nodes()
                        .iter()
                        .find(|node| {
                            node.is_role(Role::ListItem) && ui.a11y_name_of(node) == "Quiet lights"
                        })
                        .unwrap();
                    assert!(
                        (row.rect.y
                            - if matches!(width, 400.0 | 520.0) {
                                159.7
                            } else {
                                142.9
                            })
                        .abs()
                            < 0.001
                            && (row.rect.w - width).abs() < 0.001,
                        "full-width song row: {:?}",
                        row.rect
                    );
                    let wrapped = matches!(width, 400.0 | 520.0);
                    for (name, y, h) in [
                        ("1", if wrapped { 2.2 } else { 10.2 }, 18.2),
                        ("Quiet lights", if wrapped { 1.2 } else { 9.2 }, 19.6),
                        (
                            "4:00",
                            if wrapped { 3.2 } else { 11.2 },
                            if wrapped { 33.6 } else { 16.8 },
                        ),
                    ] {
                        let label = ui
                            .a11y_nodes()
                            .iter()
                            .find(|node| node.is_role(Role::Label) && ui.a11y_name_of(node) == name)
                            .unwrap();
                        assert!(
                            (label.rect.y - row.rect.y - y).abs() < 0.001
                                && (label.rect.h - h).abs() < 0.001,
                            "native baseline box {name}: {:?}",
                            label.rect
                        );
                    }
                    let time = ui
                        .a11y_nodes()
                        .iter()
                        .find(|node| node.is_role(Role::Label) && ui.a11y_name_of(node) == "4:00")
                        .unwrap();
                    let native_width = if width == 400.0 {
                        if scale == 1.0 { 15.0 } else { 14.5 }
                    } else if matches!(width, 520.0 | 364.0 | 364.1) {
                        22.0
                    } else {
                        29.0
                    };
                    assert!(
                        (time.rect.w - native_width).abs() < 0.001
                            && (time.rect.x + time.rect.w - (232.0 + width - 60.0)).abs() < 0.001,
                        "native trailing Text rounds its intrinsic width: {:?}",
                        time.rect
                    );
                    for (name, visible) in [
                        ("The National", width >= 400.0),
                        ("A midnight album", width >= 520.0),
                        ("flac", width >= 680.0),
                    ] {
                        assert_eq!(
                            ui.a11y_nodes()
                                .iter()
                                .any(|node| node.is_role(Role::Label)
                                    && ui.a11y_name_of(node) == name),
                            visible
                        );
                    }
                    let action = ui
                        .a11y_nodes()
                        .iter()
                        .find(|node| {
                            node.is_role(Role::Button)
                                && ui.a11y_name_of(node) == "Actions for Quiet lights"
                        })
                        .unwrap();
                    assert!(
                        (action.rect.x - (232.0 + width - 52.0)).abs() < 0.001
                            && action.rect.w == 32.0
                    );
                    if width == 888.0 {
                        let title = ui
                            .a11y_nodes()
                            .iter()
                            .find(|node| {
                                node.is_role(Role::Button)
                                    && ui.a11y_name_of(node) == "Sort by Title"
                            })
                            .unwrap();
                        assert!(
                            (title.rect.x - 289.6).abs() < 0.01
                                && (title.rect.y - 115.6).abs() < 0.001
                        );
                        assert!((time.rect.x - 1031.0).abs() < 0.001);
                    }
                }
            }
            if round >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm responsive song table and 1x/2x fitting"
                );
            }
        }
    }

    #[test]
    fn trailing_time_heading_matches_native_rounding_and_fractional_overflow() {
        let catalog = catalog(2);
        let table = catalog.all.as_ref().unwrap().song_table.as_ref().unwrap();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        for scale in [1.0, 2.0] {
            for width in [
                399.0, 400.0, 519.0, 520.0, 679.0, 680.0, 888.0, 364.0, 364.1, 364.26, 364.3, 364.5,
            ] {
                let columns = table.columns(&catalog, width, None, Sort::Original, Route::Songs);
                let mut root = None;
                ui.frame(
                    &FrameIn {
                        width,
                        height: 100.0,
                        scale,
                        reduced_motion: 1,
                        ..Default::default()
                    },
                    &mut |ui| {
                        let component = ui.node(
                            "component",
                            Node {
                                w: Size::Px(width),
                                h: Size::Grow,
                                ..Node::DEFAULT
                            },
                        );
                        ui.within(&component, &mut |ui| {
                            let layout = heading_layout(ui, &catalog, columns, None);
                            let p = palette(ui);
                            root = Some(heading(ui, columns, None, layout, p, &mut None));
                        });
                    },
                )
                .unwrap();
                let button = ui
                    .box_of_key(Ui::key_of(root.unwrap().box_key, "Time"))
                    .unwrap();
                let text = ui
                    .box_of_key(Ui::key_of(button.box_key, "time-label"))
                    .unwrap();
                let rect = ui.rect(&text);
                let native = if matches!(width, 400.0 | 520.0) {
                    if scale == 1.0 { 21.0 } else { 20.5 }
                } else if scale == 1.0 {
                    28.0
                } else {
                    27.5
                };
                assert!(
                    (rect.x - (width - 60.0 - native)).abs() < 0.001
                        && (rect.w - native).abs() < 0.001
                        && (rect.y - 10.0).abs() < 0.001,
                    "native Time heading {width}/{scale}: {rect:?}"
                );
            }
        }
    }

    #[test]
    fn playing_song_bars_animate_cached_text_scroll_and_stop_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog(100);
        let state = ShellState::default();
        let mut player = PlayerView {
            current_id: Some(0),
            playing: true,
            ..Default::default()
        };
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..Default::default()
        };
        for time in [0.0, 1.0, 2.0] {
            frame.time = time;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut None)
            })
            .unwrap();
        }
        let list = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::List) && node.rect.x >= 232.0)
            .unwrap()
            .id;
        ui.inject_a11y(&[pxui::A11yAction {
            id: list,
            action_code: 8,
            payload: 0.0,
            ..Default::default()
        }]);
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut None)
        })
        .unwrap();
        // A viewport build includes culled overscan; the timer adds only its row and indicator.
        let viewport_rebuilds = ui.output().rebuilt_views;
        let glyphs: Vec<_> = ui
            .layers()
            .flat_map(|layer| layer.instances.iter())
            .filter(|i| i.kind == pxui::draw::kind::MASK)
            .copied()
            .collect();
        let mut previous = [0u32; 3];
        let mut changes = 0;
        for tick in 1..121 {
            frame.time = 2.0 + tick as f64 / 60.0;
            mem_sys::local_reset();
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut None)
            })
            .unwrap();
            let mut bars = [0u32; 3];
            let mut found = 0;
            for instance in ui
                .layers()
                .flat_map(|layer| layer.instances.iter())
                .filter(|i| {
                    i.kind == pxui::draw::kind::RECT
                        && (i.rect[0] - 264.0).abs() < 9.0
                        && (i.rect[2] - i.rect[0] - 2.0).abs() < 0.001
                        && (4.39..11.01).contains(&(i.rect[3] - i.rect[1]))
                })
            {
                assert!(found < 3);
                bars[found] = (instance.rect[3] - instance.rect[1]).to_bits();
                found += 1;
            }
            assert_eq!(found, 3);
            for (index, height) in bars.into_iter().enumerate() {
                let phase = (frame.time / 0.9 + index as f64 / 3.0) % 1.0;
                let target = if phase < 0.5 {
                    phase * 2.0
                } else {
                    (1.0 - phase) * 2.0
                };
                let (mut low, mut high) = (0.0, 1.0);
                for _ in 0..20 {
                    let t = (low + high) * 0.5;
                    let x = 3.0 * (1.0 - t) * (1.0 - t) * t * 0.42
                        + 3.0 * (1.0 - t) * t * t * 0.58
                        + t * t * t;
                    if x < target { low = t } else { high = t }
                }
                let t = (low + high) * 0.5;
                let native = 11.0 * (0.4 + 0.6 * (3.0 * (1.0 - t) * t * t + t * t * t));
                assert!(
                    (f64::from(f32::from_bits(height)) - native).abs() < 0.001,
                    "native 900ms/three-phase bar {index}"
                );
            }
            changes += usize::from(bars != previous);
            previous = bars;
            assert!(
                ui.layers()
                    .flat_map(|layer| layer.instances.iter())
                    .filter(|i| i.kind == pxui::draw::kind::MASK)
                    .copied()
                    .eq(glyphs.iter().copied()),
                "animation preserves all cached glyph instances"
            );
            assert!(
                ui.output().rebuilt_views <= viewport_rebuilds + 2,
                "only the current row and indicator add to a viewport build: {}",
                ui.output().rebuilt_views
            );
            assert!(ui.output().repaint_at.is_finite());
            if tick >= 10 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm playing indicator");
            }
        }
        assert!(changes > 60);
        player.playing = false;
        frame.time += 1.0;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut None)
        })
        .unwrap();
        assert!(
            ui.output().repaint_at.is_infinite(),
            "paused indicator stops requesting frames"
        );
        player.playing = true;
        frame.time += 1.0;
        frame.reduced_motion = 1;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut None)
        })
        .unwrap();
        assert!(
            ui.output().repaint_at.is_infinite(),
            "reduced motion stops the indicator"
        );
        frame.reduced_motion = 0;
        frame.time += 1.0;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut None)
        })
        .unwrap();
        let list = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::List) && node.rect.x >= 232.0)
            .unwrap()
            .id;
        for delta in [130.0, -130.0] {
            ui.inject_a11y(&[pxui::A11yAction {
                id: list,
                action_code: 8,
                payload: delta,
                ..Default::default()
            }]);
            frame.time += 1.0;
            ui.frame(&frame, &mut |ui| {
                shell(ui, &player, &catalog, &state, &mut None)
            })
            .unwrap();
            let heading = ui
                .a11y_nodes()
                .iter()
                .find(|node| {
                    node.is_role(Role::Label)
                        && node.rect.x >= 232.0
                        && ui.a11y_name_of(node) == "Songs"
                })
                .unwrap();
            let column = ui
                .a11y_nodes()
                .iter()
                .find(|node| node.is_role(Role::Button) && ui.a11y_name_of(node) == "Sort by Title")
                .unwrap();
            assert!((heading.rect.y - if delta > 0.0 { -102.0 } else { 28.0 }).abs() < 0.001);
            assert!((column.rect.y - if delta > 0.0 { 10.0 } else { 115.6 }).abs() < 0.001);
            assert_eq!(
                ui.output().repaint_at.is_finite(),
                delta < 0.0,
                "the retained indicator restarts after leaving the opaque heading"
            );
        }
        ui.inject_a11y(&[pxui::A11yAction {
            id: list,
            action_code: 8,
            payload: 650.0,
            ..Default::default()
        }]);
        frame.time += 1.0;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut None)
        })
        .unwrap();
        frame.time += 1.0;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut None)
        })
        .unwrap();
        assert!(
            ui.output().repaint_at.is_infinite(),
            "an offscreen song cannot keep its animation timer awake"
        );
    }

    #[test]
    fn song_actions_route_play_and_append_reject_stale_rows_and_reopen_without_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog(2);
        let mut state = ShellState::default();
        let player = PlayerView::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            reduced_motion: 1,
            ..Default::default()
        };
        let mut intent = None;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalog, &state, &mut intent)
        })
        .unwrap();
        for cycle in 0..8 {
            mem_sys::local_reset();
            for (label, expected) in [
                ("Play", Intent::PlayRow(0)),
                ("Add to queue", Intent::AppendSong(0)),
            ] {
                let actions = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| {
                        node.is_role(Role::Button)
                            && ui.a11y_name_of(node) == "Actions for Quiet lights"
                    })
                    .unwrap()
                    .id;
                ui.inject_a11y(&[pxui::A11yAction {
                    id: actions,
                    action_code: 1,
                    ..Default::default()
                }]);
                frame.time += 0.01;
                intent = None;
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut intent)
                })
                .unwrap();
                assert!(matches!(intent, Some(Intent::SongMenu(0, 0, _, _))));
                state.apply(intent.take().unwrap());
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut intent)
                })
                .unwrap();
                let item = ui
                    .a11y_nodes()
                    .iter()
                    .find(|node| node.is_role(Role::MenuItem) && ui.a11y_name_of(node) == label)
                    .unwrap()
                    .id;
                ui.inject_a11y(&[pxui::A11yAction {
                    id: item,
                    action_code: 1,
                    ..Default::default()
                }]);
                frame.time += 0.01;
                intent = None;
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut intent)
                })
                .unwrap();
                assert_eq!(intent, Some(expected));
                state.apply(Intent::CloseSongMenu);
                intent = None;
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalog, &state, &mut intent)
                })
                .unwrap();
            }
            if cycle >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm row action menus");
            }
        }
        let key = catalog.all.as_ref().unwrap().key;
        let mut changed = catalog.clone();
        changed.project(
            crate::browse::Request {
                commands: None,
                scope: Scope::Collection(key),
                query: Query::default(),
                sort: Sort::Title,
                ascending: true,
            },
            vec![1, 0],
        );
        state.apply(Intent::SongMenu(0, 0, key, (500.0, 200.0)));
        intent = None;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &changed, &state, &mut intent)
        })
        .unwrap();
        let item = ui
            .a11y_nodes()
            .iter()
            .find(|node| node.is_role(Role::MenuItem) && ui.a11y_name_of(node) == "Add to queue")
            .unwrap();
        assert_eq!(item.states & pxui::a11y::state::ENABLED, 0);
        ui.inject_a11y(&[pxui::A11yAction {
            id: item.id,
            action_code: 1,
            ..Default::default()
        }]);
        frame.time += 0.01;
        intent = None;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &changed, &state, &mut intent)
        })
        .unwrap();
        assert_eq!(intent, None, "stale menu cannot append a different row");
        state.refresh(&catalog, &changed);
        assert!(state.song_menu.is_none());
    }
    #[test]
    fn availability_state_pop_matches_native_scale_and_preserves_song_identity_without_warm_allocations()
     {
        use lw_core::{FileState, Song};
        mem_sys::set_peer(pxui::library_counter);
        let catalogs = [FileState::BookmarkOnly, FileState::Copied].map(|state| {
            super::super::super::tests::prepared(vec![
                Song {
                    id: Some(1),
                    title: "First".into(),
                    file_state: state,
                    ..Default::default()
                },
                Song {
                    id: Some(2),
                    title: "Second".into(),
                    file_state: FileState::Failed,
                    ..Default::default()
                },
            ])
        });
        let mut state = ShellState::default();
        let player = PlayerView::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..Default::default()
        };
        let glyph = |ui: &Ui| {
            ui.a11y_nodes()
                .iter()
                .find(|node| {
                    node.is_role(Role::Image) && ui.a11y_name_of(node) == "Checking availability"
                })
                .unwrap()
                .rect
        };
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalogs[0], &state, &mut None)
        })
        .unwrap();
        assert_eq!(glyph(&ui).w, 10.0, "first appearance does not pop");
        for cycle in 0..8 {
            mem_sys::local_reset();
            for next in [1, 0] {
                state.refresh(&catalogs[1 - next], &catalogs[next]);
                frame.time += 1.0;
                let start = frame.time;
                ui.frame(&frame, &mut |ui| {
                    shell(ui, &player, &catalogs[next], &state, &mut None)
                })
                .unwrap();
                for progress in [0.0, 0.044, 0.205, 0.245, 0.472, 1.0] {
                    frame.time = start + f64::from(crate::motion::BASE * progress);
                    ui.frame(&frame, &mut |ui| {
                        shell(ui, &player, &catalogs[next], &state, &mut None)
                    })
                    .unwrap();
                    let expected = 10.0
                        * (0.3
                            + 0.7
                                * crate::motion::spring(crate::motion::BASE)
                                    .at(crate::motion::BASE * progress));
                    assert!(
                        (glyph(&ui).w - expected).abs() < 0.002,
                        "native state pop at {progress}: {:?}, expected {expected}",
                        glyph(&ui)
                    );
                }
            }
            if cycle >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "warm availability state transitions"
                );
            }
        }
        state.refresh(&catalogs[0], &catalogs[1]);
        frame.reduced_motion = 1;
        frame.time += 1.0;
        ui.frame(&frame, &mut |ui| {
            shell(ui, &player, &catalogs[1], &state, &mut None)
        })
        .unwrap();
        assert_eq!(
            glyph(&ui).w,
            10.0,
            "reduced motion keeps the natural glyph size"
        );
        assert!(ui.output().repaint_at.is_infinite());
    }
}
