use super::*;
/// The page heading: 34 points, medium, tracked in, on a 34-point line.
/// The desktop Find field (`localwave-search` / `DESKTOP`): search traits (no autocorrect or capitalization), Done ends editing.
const FIND_DESKTOP: pxui::SearchField = pxui::SearchField::DEFAULT.name("Find songs").action(pxui::ReturnKey::Done).variant(variant::DESKTOP);
/// Swift's phone Find field (`localwave-search` / `COMPACT`), with its prompt.
const FIND_COMPACT: pxui::SearchField = FIND_DESKTOP.variant(variant::COMPACT).placeholder("Songs, albums, folders");
const HEADING: Label = Label::BODY.size(34.0).line_height(34.0).weight(500).tracking(-0.025).lines(1).grow().h_px(34.0).whole_pixels().color(INK);
use crate::browse::Query;
use crate::catalog::{Catalog, Route};
use crate::{Intent, PlayerView};

pub(super) fn header(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let collection = state.collection(catalog);
    // Swift's desktop results page: "Search", “query”, "N songs · M albums · F folders", no field.
    let results = state.route == Route::Find && state.find_results;
    let find_summary = collection
        .and(catalog.view.as_ref())
        .and_then(|view| view.find.as_ref());
    let kind = list_kind(catalog, state);
    let root = Node::column().w(Size::Grow).h(Size::Fit).padding(Edges::new(32.0, 28.0, 32.0, 0.0));
    ui.column(root, |ui| {
        if results {
            spacer(ui, "results-caption-gap", 6.0);
            ui.label("Search", BODY_DIM.grow());
        }
        if let Some(kind) = kind {
            spacer(ui, "kind-gap", 6.0);
            ui.label(kind, CAPTION.grow());
        }
        let title = match state.route {
            Route::Find if results => find_summary.map_or("“…”", |summary| summary.title.as_str()),
            Route::Albums => "Albums",
            Route::Artists => "Artists",
            Route::Find => "Find",
            _ => collection.map_or("Songs", |group| group.title.as_str()),
        };
        let heading = container(
            ui,
            "heading-row",
            Axis::Row,
            Size::Grow,
            Size::Px(34.0),
            &PAPER_FILL,
        );
        // Swift's desktop heading has no refresh key: catalogs refresh on library changes (the phone pulls to refresh)
        ui.within(&heading, &mut |ui| { ui.label(title, HEADING); });
        if state.route == Route::Find && !results {
            ui.id("find-query", |ui| { ui.column(Node::column().h_px(44.0), |ui| { ui.view_of("find-query", &state.query_revision, |ui| {
                if let Some(value) = ui.search_field("find-songs", state.query.as_str(), &FIND_DESKTOP).changed {
                    *intent = Some(Query::new(value).map_or(Intent::SearchTooLong, Intent::Search));
                }
            }); }); });
            find_tokens(ui, state, (width - 64.0).max(1.0), intent);
        }
        spacer(ui, "heading-subtitle-gap", 6.0);
        ui.label(if state.query_too_long && state.route == Route::Find { "Search text is too long. Shorten it." } else if state.route == Route::Find && state.query.as_str().trim().is_empty() { "Search titles, artists, albums or add a filter" } else if state.route == Route::Find && collection.is_none() { "Searching…" } else if state.route == Route::Find { catalog .view .as_ref() .and_then(|view| view.find.as_ref()) .map_or("Searching…", |summary| summary.summary.as_str()) } else if state.route == Route::Songs { "Click a header to sort. Double-click or Return plays." } else if state.route == Route::Artists { "Sorted ignoring “The”" } else if state.route == Route::Albums { &catalog.library_subtitle } else { collection.map_or("Browse your library", |group| group.subtitle.as_str()) }, BODY_DIM.grow());
        spacer(ui, "heading-bottom-gap", 18.0);
        if state.route == Route::Albums {
            albums::toolbar(ui, catalog, state, intent);
        }
        if state.route == Route::Albums {
            ui.node(
                "heading-divider",
                Node::column().w_px(width).h_px(1.0).margin(Edges::ZERO.l(-32.0).r(-32.0)).look(LINE_FILL),
            );
            spacer(ui, "album-header-bottom", 1.0);
        } else if state.route == Route::Find {
            find::toolbar(ui, catalog, state, intent);
        } else if kind.is_some() {
            list_tools(ui, collection.is_some_and(|group| !group.ids.is_empty()), intent);
        }
    });
}

/// Swift's list pages lead with their kind: "Automatic list" (Favorites and the smart lists) or "List" (a playlist).
pub(super) fn list_kind(catalog: &Catalog, state: &ShellState) -> Option<&'static str> {
    match (state.route, state.collection(catalog).map(|group| &*group.context)) {
        (Route::Collection(_), Some(lw_core::context::PlaybackContext::Automatic(_))) => Some("Automatic list"),
        (Route::Collection(_), Some(lw_core::context::PlaybackContext::Playlist { .. })) => Some("List"),
        _ => None,
    }
}

/// The desktop heading's height over a song table: 28 above the 34-point title, 6 and the 19.6-point line under it, 18 below; a
/// list page adds its kind's line (6 and 18.2) and its tools (30, 14 and the hairline).
pub(super) fn height(catalog: &Catalog, state: &ShellState) -> f32 {
    105.6 + if list_kind(catalog, state).is_some() { 6.0 + 18.2 + 30.0 + 15.0 } else { 0.0 }
}

/// Swift's `desktopPlayTools`: Play and Shuffle keys, 14 points over a hairline across the page.
fn list_tools(ui: &mut Ui, enabled: bool, intent: &mut Option<Intent>) {
    let tools = ui.node("list-tools", Node::column().w(Size::Grow).h_px(30.0 + 15.0));
    ui.within(&tools, &mut |ui| {
        ui.row(Node::row().w(Size::Grow).h_px(30.0).gap(8.0), |ui| {
            if ui.button("play", "Play", &TOOL.icon(icon_drawing(Icon::Play)).icon_size(16.0).disabled(!enabled)).clicked {
                *intent = Some(Intent::PlayRow(0));
            }
            if ui.button("shuffle", "Shuffle", &TOOL.icon(icon_drawing(Icon::Shuffle)).icon_size(16.0).disabled(!enabled)).clicked {
                *intent = Some(Intent::ShuffleCollection);
            }
        });
    });
    ui.canvas_over(&tools, move |paint, rect| {
        paint.fill(Rect::new(rect.x - 32.0, rect.y + rect.h - 1.0, rect.w + 64.0, 1.0), Fill::new(Rgba::token(LINE)).fractional())
    });
}

/// Swift's compact Find header: caption, 26 pt query field with an ink rule, filter chips, then
/// either the usage hint or the "Songs N" section label (with Play/Keep offline beneath).
pub(super) fn compact_find(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let revision = crate::hash((
        state.revision,
        (state.selecting, state.selection_revision),
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        catalog
            .view
            .as_ref()
            .map_or(0, |view| std::sync::Arc::as_ptr(view) as usize),
    ));
    ui.id("find-heading", |ui| { ui.column(Node::column(), |ui| { ui.view_of("page-heading", &revision, |ui| {
        // Select mode swaps the whole heading for Swift's selection header.
        if state.selecting {
            let rows = state.collection(catalog).map_or(0, |c| c.rows.len());
            super::compact::list::selection_header(ui, state.marked_count(), rows, rows, intent);
            return;
        }
        ui.column(Node::column().w(Size::Grow).h(Size::Fit).padding(Edges::new(16.0, 20.0, 16.0, 0.0)),
            |ui| {
                ui.label("Find", TEXT_13.grow().h_px(21.0).color(SECONDARY));
                ui.column(Node::column().h_px(56.0), |ui| { ui.view_of("find-query", &state.query_revision, |ui| {
                    let field = ui.search_field("find-songs", state.query.as_str(), &FIND_COMPACT);
                    let editor = field.response.handle;
                    if let Some(value) = field.changed {
                        *intent = Some(Query::new(value).map_or(Intent::SearchTooLong, Intent::Search));
                    }
                    // Swift's phone Find takes the keyboard when it opens: the query is focused on the field's first build
                    if ui.state(&editor, 0) == 0 {
                        ui.focus_box(&editor);
                        ui.set_state(&editor, 1);
                    }
                }); });
                spacer(ui, "tokens-gap", 12.0);
                find_tokens(ui, state, (width - 32.0).max(1.0), intent);
                if state.query.as_str().trim().is_empty() {
                    hint(ui, (width - 32.0).max(1.0));
                } else if let Some(summary) = state
                    .collection(catalog)
                    .and(catalog.view.as_ref())
                    .and_then(|view| view.find.as_ref())
                {
                    // Swift's `sectionLabel`: 20 above (find_tokens' 12-point tail and 8), the 18.2-point line, 8 under. Swift has no
                    // visible Select (an accessibility action only); its 44-point key centres on the line without moving it
                    spacer(ui, "songs-gap", 8.0);
                    ui.row(Node::row().align(Align::Center).w(Size::Grow).h_px(18.2), |ui| {
                        ui.column(Node::column().w(Size::Grow).h(Size::Fit), |ui| {
                            ui.label(&summary.label, SECTION_CAPTION);
                        });
                        if super::compact::list::select_for_assistive(ui) {
                            *intent = Some(Intent::SelectMode(true));
                        }
                    });
                    spacer(ui, "rows-gap", 8.0);
                }
            },
        );
    }); }); });
}

/// "Search spans every source..." with Swift's bold filter examples, wrapped to the page width.
fn hint(ui: &mut Ui, width: f32) {
    const TEXT: &str = "Search spans every source, including folders that are not in an album. Add a word like flac or cloud to filter.";
    let bold = |word: &str| {
        let start = TEXT.find(word).unwrap_or(0) as u32;
        pxui::TextSpan {
            start,
            end: start + word.len() as u32,
            weight: 700,
            ..Default::default()
        }
    };
    let spans = [bold("flac"), bold("cloud")];
    const HINT: Label = Label::BODY.line_height(19.6).wrap();
    let height = ui
        .text_service()
        .measure_spans(TEXT, HINT, Some(width), &spans)
        .map_or(19.6, |m| m.height.max(19.6));
    let node = ui.node(
        "hint",
        // margin: 40 less find_tokens' 12 pt tail
        Node::column().w(Size::Grow).h_px(height).role(Role::Paragraph).margin(Edges::ZERO.t(28.0)).look(SECONDARY_TEXT),
    );
    ui.text_line_box(&node, TEXT, HINT, &spans).unwrap();
}

pub(super) fn header_view(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let revision = crate::hash((
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        catalog
            .view
            .as_ref()
            .map_or(0, |view| std::sync::Arc::as_ptr(view) as usize),
    ));
    ui.id("page-heading", |ui| { ui.column(Node::column(), |ui| { ui.view_of("page-heading", &revision, |ui| {
        header(ui, catalog, state, width, intent)
    }); }); });
}

pub(super) fn content(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    size: (f32, f32),
    intent: &mut Option<Intent>,
) {
    ui.column(Node::column().w(Size::Grow).h(Size::Grow), |ui| match state.route {
        Route::Albums => albums::gallery(ui, catalog, state, player, size.0, intent),
        Route::Artists => artists::table(ui, catalog, state, player, size.0, intent),
        Route::Lists => compact::collections(ui, catalog, player, true, intent),
        _ => songs::table(ui, catalog, state, player, size, intent),
    });
}

pub(super) fn find_tokens(
    ui: &mut Ui,
    state: &ShellState,
    width: f32,
    intent: &mut Option<Intent>,
) {
    const TOKENS: [&str; 5] = ["flac", "cloud", "offline", "year:2019", "unplayed"];
    let revision = crate::hash((state.query_revision, width.to_bits()));
    ui.id("find-tokens", |ui| { ui.column(Node::column().w_px(width), |ui| { ui.view_of("find-tokens", &revision, |ui| {
            let flow = Node::row().gap(6.0).w_px(width).wrap().row_gap(6.0).h(Size::Fit).padding(Edges::ZERO.b(12.0));
            ui.row(flow, |ui| {
                for (index, token) in TOKENS.into_iter().enumerate() {
                    if ui.checkbox(Key::Id(index as i64), token, state.query.has_token(token), &CHIP).changed {
                        *intent = Some(state.query.toggle_token(token).map_or(Intent::SearchTooLong, Intent::Search));
                    }
                }
            });
        }); }); });
}
