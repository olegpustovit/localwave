use super::*;
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
            text(ui, "results-caption", "Search", 14, ui.color(SECONDARY), (Size::Grow, 19.6), false);
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
            let heading = text(ui, "heading", title, 34, ui.color(INK), (Size::Grow, 34.0), false);
            ui.text_line_box(
                &heading,
                title,
                &Label::BODY.size(34.0).line_height(34.0).weight(500).tracking(-0.025).lines(1),
                &[],
            )
            .unwrap();
            if !results
                && !matches!(state.route, Route::Albums | Route::Artists)
                && icon_button(
                    ui,
                    "refresh",
                    "Refresh library",
                    Icon::Refresh,
                    (36.0, 36.0),
                    false,
                    true,
                )
            {
                *intent = Some(Intent::RefreshLibrary);
            }
        });
        if state.route == Route::Find && !results {
            ui.view(
                "find-query",
                state.query_revision,
                Size::Grow,
                Size::Px(44.0),
                &mut |ui| {
                    let row =
                        container(ui, "query-row", Axis::Row, Size::Grow, Size::Grow, ui.color(PAPER));
                    ui.within(&row, &mut |ui| {
                        let field = ui.box_(
                            "query-field",
                            BoxOpts::default().w(Size::Grow).h(Size::Grow),
                        );
                        ui.within(&field, &mut |ui| {
                            if let Some(value) = ui.text_field_with_options(
                                "Find songs",
                                state.query.as_str(),
                                pxui::TextInputOptions::SEARCH_FIELD.action(pxui::TextInputOptions::DONE).variant(commands::FIND_FIELD),
                            ) {
                                *intent = Some(
                                    Query::new(value).map_or(Intent::SearchTooLong, Intent::Search),
                                );
                            }
                        });
                        if icon_button(
                            ui,
                            "clear-query",
                            "Clear search",
                            Icon::Clear,
                            (36.0, 36.0),
                            false,
                            true,
                        ) {
                            *intent = Some(Intent::Search(Query::default()));
                        }
                    });
                },
            );
            find_tokens(ui, state, (width - 64.0).max(1.0), intent);
        }
        spacer(ui, "heading-subtitle-gap", 6.0);
        text(
            ui,
            "subtitle",
            if state.query_too_long && state.route == Route::Find {
                "Search text is too long. Shorten it."
            } else if state.route == Route::Find && state.query.as_str().trim().is_empty() {
                "Search titles, artists, albums or add a filter"
            } else if state.route == Route::Find && collection.is_none() {
                "Searching…"
            } else if state.route == Route::Find {
                catalog
                    .view
                    .as_ref()
                    .and_then(|view| view.find.as_ref())
                    .map_or("Searching…", |summary| summary.summary.as_str())
            } else if state.route == Route::Songs {
                "Click a header to sort. Double-click or Return plays."
            } else if state.route == Route::Artists {
                "Sorted ignoring “The”"
            } else if state.route == Route::Albums {
                &catalog.library_subtitle
            } else {
                collection.map_or("Browse your library", |group| group.subtitle.as_str())
            },
            14,
            ui.color(SECONDARY),
            (Size::Grow, 19.6),
            false,
        );
        spacer(ui, "heading-bottom-gap", 18.0);
        if state.route == Route::Albums {
            albums::toolbar(ui, catalog, state, intent);
        }
        if state.route == Route::Albums {
            ui.node(
                "heading-divider".into(),
                Node {
                    margin: Edges {
                        l: -32.0,
                        r: -32.0,
                        ..Edges::ZERO
                    },
                    look: Look {
                        fill: ui.color(LINE),
                        ..Look::NONE
                    },
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
    theme: Option<&pxui::Theme>,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let revision = crate::hash((
        state.revision,
        (state.selecting, state.selection_revision),
        theme.map_or(0, |t| t as *const pxui::Theme as usize),
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        catalog
            .view
            .as_ref()
            .map_or(0, |view| std::sync::Arc::as_ptr(view) as usize),
    ));
    ui.view("page-heading", revision, Size::Grow, Size::Fit, &mut |ui| {
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
                text(
                    ui,
                    "caption",
                    "Find",
                    13,
                    ui.color(SECONDARY),
                    (Size::Grow, 21.0),
                    false,
                );
                ui.view(
                    "find-query",
                    state.query_revision,
                    Size::Grow,
                    Size::Px(56.0),
                    &mut |ui| {
                        let row = Node::row().align(Align::Center).w(Size::Grow).h(Size::Grow);
                        ui.row("query-row", row, |ui| {
                            let field = ui.box_(
                                "query-field",
                                BoxOpts::default().w(Size::Grow).h(Size::Grow),
                            );
                            ui.within(&field, &mut |ui| {
                                let mut edit = |ui: &mut Ui| {
                                    if let Some(value) = ui.text_field_with_options(
                                        "Find songs",
                                        state.query.as_str(),
                                        pxui::TextInputOptions::SEARCH_FIELD.action(pxui::TextInputOptions::DONE).variant(commands::FIND_FIELD),
                                    ) {
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
                                if icon_button(
                                    ui,
                                    "clear-query",
                                    "Clear search",
                                    Icon::Clear,
                                    (44.0, 44.0),
                                    false,
                                    // Swift's clear glyph has no resting plate.
                                    false,
                                ) {
                                    *intent = Some(Intent::Search(Query::default()));
                                }
                            });
                        });
                    },
                );
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
    });
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
    let style = ui.resolve(&HINT);
    let height = ui
        .text_service()
        .measure_spans(TEXT, &style, width, &spans)
        .map_or(19.6, |m| m.height.max(19.6));
    let node = ui.node(
        "hint".into(),
        Node {
            margin: Edges {
                t: 28.0, // 40 less find_tokens' 12 pt tail
                ..Edges::ZERO
            },
            look: Look {
                text: ui.color(SECONDARY),
                ..Look::NONE
            },
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
    ui.view("page-heading", revision, Size::Grow, Size::Fit, &mut |ui| {
        header(ui, catalog, state, width, intent)
    });
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
    ui.view(
        "find-tokens",
        revision,
        Size::Px(width),
        Size::Fit,
        &mut |ui| {
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
                    let chip = ui.node(
                        token.into(),
                        Node {
                            max_w: width,
                            look: Look {
                                fill: if selected { ui.color(INK) } else { ui.color(PANEL) },
                                ..Look::NONE
                            },
                            ..Node::column()
                                .w(Size::Fit)
                                .h_px(32.0)
                                .pad_x(12.0)
                                .role(Role::Toggle)
                                .flags(Flags::CLICKABLE | Flags::TAB_STOP | Flags::CLIP)
                        },
                    );
                    ui.set_role(
                        &chip,
                        Role::Toggle,
                        pxui::a11y::state::CHECKABLE
                            | if selected {
                                pxui::a11y::state::CHECKED
                            } else {
                                0
                            },
                    );
                    ui.a11y_name(&chip, token);
                    ui.within(&chip, &mut |ui| {
                        let label = ui.node(
                            "label".into(),
                            Node {
                                look: Look {
                                    text: if selected { ui.color(PAPER) } else { ui.color(SECONDARY) },
                                    font: Font {
                                        size: 13,
                                        ..Font::BODY
                                    },
                                    ..Look::NONE
                                },
                                ..Node::column().w(Size::Fit).h(Size::Grow).flags(Flags::FIT_TEXT)
                            },
                        );
                        ui.text(&label, token);
                    });
                    if ui.signal(&chip).clicked {
                        *intent = Some(
                            state
                                .query
                                .toggle_token(token)
                                .map_or(Intent::SearchTooLong, Intent::Search),
                        );
                    }
                }
            });
        },
    );
}
