use super::*;
use crate::browse::Query;
use crate::catalog::{Catalog, Route};
use crate::{Intent, PlayerView};
use pxui::geom::hash::FastHasher;
use std::hash::{Hash, Hasher};

pub(super) fn header(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let collection = state.collection(catalog);
    let root = ui.node(
        "library-header",
        Node {
            w: Size::Grow,
            h: Size::Fit,
            padding: Edges {
                l: 32.0,
                r: 32.0,
                t: 28.0,
                b: 0.0,
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let title = match state.route {
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
            p.paper,
        );
        ui.within(&heading, &mut |ui| {
            let heading = text(ui, "heading", title, 34, p.ink, (Size::Grow, 34.0), false);
            ui.text_line_box(
                &heading,
                title,
                &pxui::TextStyle {
                    size_points: 34.0,
                    line_height: 34.0,
                    weight: 500,
                    tracking: -0.85,
                    max_lines: 1,
                    ..Default::default()
                },
                &[],
            )
            .unwrap();
            if !matches!(state.route, Route::Albums | Route::Artists)
                && icon_button(
                    ui,
                    "refresh",
                    "Refresh library",
                    Icon::Refresh,
                    (36.0, 36.0),
                    false,
                    p,
                )
            {
                *intent = Some(Intent::RefreshLibrary);
            }
        });
        if state.route == Route::Find {
            ui.view(
                "find-query",
                state.query_revision,
                Size::Grow,
                Size::Px(44.0),
                &mut |ui| {
                    let row =
                        container(ui, "query-row", Axis::Row, Size::Grow, Size::Grow, p.paper);
                    ui.within(&row, &mut |ui| {
                        let field = ui.box_(
                            "query-field",
                            BoxOpts::default().w(Size::Grow).h(Size::Grow),
                        );
                        ui.within(&field, &mut |ui| {
                            if let Some(value) = ui.text_field_with_options(
                                "Find songs",
                                state.query.as_str(),
                                pxui::TextInputOptions {
                                    action: pxui::TextInputOptions::DONE,
                                    ..pxui::TextInputOptions::SEARCH_FIELD
                                },
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
                            p,
                        ) {
                            *intent = Some(Intent::Search(Query::default()));
                        }
                    });
                },
            );
            find_tokens(ui, state, (width - 64.0).max(1.0), p, intent);
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
            p.secondary,
            (Size::Grow, 19.6),
            false,
        );
        spacer(ui, "heading-bottom-gap", 18.0);
        if state.route == Route::Albums {
            albums::toolbar(ui, catalog, state, p, intent);
        }
        if state.route == Route::Albums {
            ui.node(
                "heading-divider",
                Node {
                    w: Size::Px(width),
                    h: Size::Px(1.0),
                    margin: Edges {
                        l: -32.0,
                        r: -32.0,
                        ..Edges::ZERO
                    },
                    look: Look {
                        fill: p.line,
                        ..Look::NONE
                    },
                    ..Node::DEFAULT
                },
            );
            spacer(ui, "album-header-bottom", 1.0);
        } else if state.route == Route::Find {
            find::toolbar(ui, catalog, state, p, intent);
        }
    });
}

pub(super) fn header_view(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let mut revision = FastHasher::default();
    (
        state.revision,
        std::sync::Arc::as_ptr(&catalog.library) as usize,
        catalog
            .view
            .as_ref()
            .map_or(0, |view| std::sync::Arc::as_ptr(view) as usize),
    )
        .hash(&mut revision);
    ui.view(
        "page-heading",
        revision.finish(),
        Size::Grow,
        Size::Fit,
        &mut |ui| header(ui, catalog, state, width, p, intent),
    );
}

pub(super) fn content(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    player: &PlayerView<'_>,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let root = ui.node(
        "content",
        Node {
            w: Size::Grow,
            h: Size::Grow,
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| match state.route {
        Route::Albums => albums::gallery(ui, catalog, state, player, width, p, intent),
        Route::Artists => artists::table(ui, catalog, state, player, width, p, intent),
        Route::Lists => compact::collections(ui, catalog, player, width, p, intent),
        _ => songs::table(ui, catalog, state, player, width, p, intent),
    });
}

pub(super) fn find_tokens(
    ui: &mut Ui,
    state: &ShellState,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    const TOKENS: [&str; 5] = ["flac", "cloud", "offline", "year:2019", "unplayed"];
    let widths = TOKENS.map(|token| (ui.text_width(Some(13), token) + 24.0).min(width));
    let positions = {
        let (mut x, mut y) = (0.0, 0.0);
        widths.map(|w| {
            if x > 0.0 && x + w > width {
                x = 0.0;
                y += 38.0;
            }
            let at = (x, y);
            x += w + 6.0;
            at
        })
    };
    let mut revision = FastHasher::default();
    (state.query_revision, width.to_bits()).hash(&mut revision);
    ui.view(
        "find-tokens",
        revision.finish(),
        Size::Grow,
        Size::Px(positions[4].1 + 44.0),
        &mut |ui| {
            for (index, token) in TOKENS.into_iter().enumerate() {
                let selected = state.query.has_token(token);
                let chip = ui.node(
                    token,
                    Node {
                        flags: CLICKABLE | TAB_STOP | CLIP,
                        role: Role::Toggle,
                        w: Size::Px(widths[index]),
                        h: Size::Px(32.0),
                        at: Some(positions[index]).into(),
                        look: Look {
                            fill: if selected { p.ink } else { p.panel },
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
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
                        "label",
                        Node {
                            flags: FIT_TEXT,
                            w: Size::Grow,
                            h: Size::Grow,
                            inset: Edges {
                                l: 12.0,
                                ..Edges::ZERO
                            },
                            look: Look {
                                text: if selected { p.paper } else { p.secondary },
                                font: Font {
                                    size: 13,
                                    ..Font::BODY
                                },
                                ..Look::NONE
                            },
                            ..Node::DEFAULT
                        },
                    );
                    ui.text(&label, token);
                });
                if ui.signal_state(&chip).clicked {
                    *intent = Some(
                        state
                            .query
                            .toggle_token(token)
                            .map_or(Intent::SearchTooLong, Intent::Search),
                    );
                }
            }
        },
    );
}
