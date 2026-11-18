use super::*;
/// The page heading: 34 points, medium, tracked in, on a 34-point line.
/// The phone Find field: no autocorrect or capitalization, Done ends editing, the Find body variant.
const FIND: pxui::TextField = pxui::TextField::SEARCH
    .action(pxui::ReturnKey::Done)
    .variant(commands::FIND_FIELD)
    .name("Find songs");
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
    let root = Node {
        padding: Edges {
            l: 32.0,
            r: 32.0,
            t: 28.0,
            b: 0.0,
        },
        ..Node::column().w(Size::Grow).h(Size::Fit)
    };
    ui.column("library-header", root, |ui| {
        if results {
            spacer(ui, "results-caption-gap", 6.0);
            ui.label("Search", BODY_DIM.grow());
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
            ui.color(PAPER),
        );
        ui.within(&heading, &mut |ui| {
            ui.label(title, HEADING);
            if !results
                && !matches!(state.route, Route::Albums | Route::Artists)
                && ui.icon_button("refresh", icon_drawing(Icon::Refresh), "Refresh library", &ICON_36.look(&ICON_PLATE_LOOKS)).clicked
            {
                *intent = Some(Intent::RefreshLibrary);
            }
        });
        if state.route == Route::Find && !results {
            ui.column("find-query", Node::column().h_px(44.0), |ui| { ui.view_of("find-query", &state.query_revision, |ui| {
                    let row =
                        container(ui, "query-row", Axis::Row, Size::Grow, Size::Grow, ui.color(PAPER));
                    ui.within(&row, &mut |ui| {
                        let field = ui.node("query-field", Node::DEFAULT.w(Size::Grow).h(Size::Grow));
                        ui.within(&field, &mut |ui| {
                            if let Some(value) = ui
                                .text_field("find-songs", state.query.as_str(), &FIND)
                                .changed
                            {
                                *intent = Some(
                                    Query::new(value).map_or(Intent::SearchTooLong, Intent::Search),
                                );
                            }
                        });
                        if ui.icon_button("clear-query", icon_drawing(Icon::Clear), "Clear search", &ICON_36.look(&ICON_PLATE_LOOKS)).clicked {
                            *intent = Some(Intent::Search(Query::default()));
                        }
                    });
                }); });
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
                Node {
                    margin: Edges {
                        l: -32.0,
                        r: -32.0,
                        ..Edges::ZERO
                    },
                    look: LINE_FILL,
                    ..Node::column().w_px(width).h_px(1.0)
                },
            );
            spacer(ui, "album-header-bottom", 1.0);
        } else if state.route == Route::Find {
            find::toolbar(ui, catalog, state, intent);
        }
    });
}

/// Swift's compact Find header: caption, 26 pt query field with an ink rule, filter chips, then
/// either the usage hint or the "Songs N" section label (with Play/Keep offline beneath).
pub(super) fn compact_find(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    theme: Option<&pxui::ThemeHandle>,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let revision = crate::hash((
        state.revision,
        (state.selecting, state.selection_revision),
        theme.map_or(0, |t| t as *const pxui::ThemeHandle as usize),
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        catalog
            .view
            .as_ref()
            .map_or(0, |view| std::sync::Arc::as_ptr(view) as usize),
    ));
    ui.column("page-heading", Node::column(), |ui| { ui.view_of("page-heading", &revision, |ui| {
        // Select mode swaps the whole heading for Swift's selection header.
        if state.selecting {
            let rows = state.collection(catalog).map_or(0, |c| c.rows.len());
            super::compact::list::selection_header(ui, state.marked.len(), rows, rows, intent);
            return;
        }
        ui.column(
            "compact-find",
            Node {
                padding: Edges {
                    l: 16.0,
                    r: 16.0,
                    t: 20.0,
                    b: 0.0,
                },
                ..Node::column().w(Size::Grow).h(Size::Fit)
            },
            |ui| {
                ui.label("Find", TEXT_13.grow().h_px(21.0).color(SECONDARY));
                ui.column("find-query", Node::column().h_px(56.0), |ui| { ui.view_of("find-query", &state.query_revision, |ui| {
                        let row = Node::row().align(Align::Center).w(Size::Grow).h(Size::Grow);
                        ui.row("query-row", row, |ui| {
                            let field = ui.node("query-field", Node::DEFAULT.w(Size::Grow).h(Size::Grow));
                            ui.within(&field, &mut |ui| {
                                let mut edit = |ui: &mut Ui| {
                                    if let Some(value) = ui
                                        .text_field("find-songs", state.query.as_str(), &FIND)
                                        .changed
                                    {
                                        *intent = Some(
                                            Query::new(value)
                                                .map_or(Intent::SearchTooLong, Intent::Search),
                                        );
                                    }
                                };
                                match theme {
                                    Some(theme) => ui.themed(theme, &mut edit),
                                    None => edit(ui),
                                }
                            });
                            // Swift offsets the 44 pt target 10 pt past the gutter.
                            let clear = Node {
                                margin: Edges {
                                    r: -10.0,
                                    ..Edges::ZERO
                                },
                                ..Node::column().w_px(44.0).h_px(44.0).align(Align::Center)
                            };
                            ui.column("clear-slot", clear, |ui| {
                                // Swift's clear glyph has no resting plate.
                                if ui.icon_button("clear-query", icon_drawing(Icon::Clear), "Clear search", &ICON_44.look(&ICON_LOOKS)).clicked {
                                    *intent = Some(Intent::Search(Query::default()));
                                }
                            });
                        });
                    }); });
                container(
                    ui,
                    "query-rule",
                    Axis::Column,
                    Size::Grow,
                    Size::Px(1.5),
                    ui.color(INK),
                );
                spacer(ui, "tokens-gap", 12.0);
                find_tokens(ui, state, (width - 32.0).max(1.0), intent);
                if state.query.as_str().trim().is_empty() {
                    hint(ui, (width - 32.0).max(1.0));
                } else if let Some(summary) = state
                    .collection(catalog)
                    .and(catalog.view.as_ref())
                    .and_then(|view| view.find.as_ref())
                {
                    spacer(ui, "songs-gap", 7.0);
                    ui.row("songs-row", Node::row().align(Align::Center).w(Size::Grow).h_px(44.0), |ui| {
                        ui.column("songs-text", Node::column().w(Size::Grow).h(Size::Fit), |ui| {
                            detail::line(ui, "songs-label", &summary.label, (13.0, 18.2, 500, 0.0), ui.color(SECONDARY), Flags::NONE);
                        });
                        if super::compact::list::word(ui, "select", "Select", false) {
                            *intent = Some(Intent::SelectMode(true));
                        }
                    });
                }
            },
        );
    }); });
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
        .measure_spans(TEXT, &HINT, width, &spans)
        .map_or(19.6, |m| m.height.max(19.6));
    let node = ui.node(
        "hint",
        Node {
            margin: Edges {
                t: 28.0, // 40 less find_tokens' 12 pt tail
                ..Edges::ZERO
            },
            look: SECONDARY_TEXT,
            ..Node::column().w(Size::Grow).h_px(height).role(Role::Paragraph)
        },
    );
    ui.text_line_box(&node, TEXT, &HINT, &spans).unwrap();
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
    ui.column("page-heading", Node::column(), |ui| { ui.view_of("page-heading", &revision, |ui| {
        header(ui, catalog, state, width, intent)
    }); });
}

pub(super) fn content(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    intent: &mut Option<Intent>,
) {
    ui.column("content", Node::column().w(Size::Grow).h(Size::Grow), |ui| match state.route {
        Route::Albums => albums::gallery(ui, catalog, state, player, width, intent),
        Route::Artists => artists::table(ui, catalog, state, player, width, intent),
        Route::Lists => compact::collections(ui, catalog, player, width, intent),
        _ => songs::table(ui, catalog, state, player, width, intent),
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
    ui.column("find-tokens", Node::column().w_px(width), |ui| { ui.view_of("find-tokens", &revision, |ui| {
            let flow = Node {
                wrap: true,
                row_gap: 6.0,
                h: Size::Fit,
                padding: Edges {
                    b: 12.0,
                    ..Edges::ZERO
                },
                ..Node::row().gap(6.0).w_px(width)
            };
            ui.row("flow", flow, |ui| {
                for token in TOKENS {
                    let selected = state.query.has_token(token);
                    let node = Node {
                        max_w: width,
                        ..Node::column().w(Size::Fit).h_px(32.0).pad_x(12.0).role(Role::Toggle).flags(Flags::CLIP)
                    };
                    let chip = ui.pressable(token, node, &PLAIN.look(if selected { &INK_LOOKS } else { &PANEL_LOOKS }), |ui, _| {
                        let label = ui.node(
                            "label",
                            Node {
                                look: if selected { CHIP_TEXT_ON } else { CHIP_TEXT },
                                ..Node::column().w(Size::Fit).h(Size::Grow).flags(Flags::FIT_TEXT)
                            },
                        );
                        ui.text(&label, token);
                    });
                    ui.set_role(
                        &chip.handle,
                        Role::Toggle,
                        pxui::ui::state::CHECKABLE
                            | if selected {
                                pxui::ui::state::CHECKED
                            } else {
                                0
                            },
                    );
                    ui.a11y_name(&chip.handle, token);
                    if chip.clicked {
                        *intent = Some(
                            state
                                .query
                                .toggle_token(token)
                                .map_or(Intent::SearchTooLong, Intent::Search),
                        );
                    }
                }
            });
        }); });
}
