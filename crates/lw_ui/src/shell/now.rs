use super::*;
use crate::catalog::Catalog;
use crate::{Intent, PlayerView};

pub(super) fn now_panel(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    catalog: &Catalog,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let root = ui.node(
        "now",
        Node {
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges {
                l: 20.0,
                r: 20.0,
                t: 8.0,
                b: 0.0,
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let head = ui.node(
            "header",
            Node {
                axis: Axis::Row,
                align: Align::Center,
                w: Size::Px(289.0),
                h: Size::Px(36.0),
                ..Node::DEFAULT
            },
        );
        ui.within(&head, &mut |ui| {
            text(
                ui,
                "name",
                "Now playing",
                13,
                p.secondary,
                (Size::Grow, 18.2),
                false,
            );
            let favorite = player.current_id.map(|id| {
                (
                    id,
                    catalog
                        .row_index(id)
                        .is_some_and(|i| catalog.row(i).favorite),
                )
            });
            if favorite_button(ui, favorite, p)
                && let Some((song, on)) = favorite
            {
                *intent = Some(Intent::SetFavorite { song, on: !on });
            }
            if icon_button(
                ui,
                "hide-panel",
                "Minimise to small player",
                Icon::Panel,
                (36.0, 36.0),
                false,
                p,
            ) {
                *intent = Some(Intent::TogglePanel);
            }
        });
        spacer(ui, "artwork-top-gap", 4.0);
        let art = container(
            ui,
            "artwork",
            Axis::Stack,
            Size::Px(200.0),
            Size::Px(200.0),
            p.panel,
        );
        paint_artwork(
            ui,
            &art,
            player
                .current_id
                .and_then(|id| catalog.row_index(id))
                .map(|index| catalog.row(index)),
            200.0,
            player.artwork,
        );
        ui.a11y_name(&art, "Current track artwork");
        spacer(ui, "artwork-bottom-gap", 14.0);
        let information_revision = pxui::hash!((
            player.revision,
            std::sync::Arc::as_ptr(&catalog.library) as usize,
        ));
        let information = ui.view(
            "track-information",
            information_revision,
            Size::Grow,
            Size::Fit,
            &mut |ui| {
                let title = ui.node(
                    "current-track",
                    Node {
                        w: Size::Grow,
                        h: Size::Fit,
                        min_h: 22.8,
                        role: Role::Label,
                        ..Node::DEFAULT
                    },
                );
                let value = if !player.initialized {
                    "Loading library"
                } else if player.title.is_empty() {
                    "Nothing playing"
                } else {
                    player.title
                };
                ui.text_line_box(
                    &title,
                    value,
                    &pxui::TextStyle {
                        size_points: 19.0,
                        weight: 500,
                        tracking: -0.285,
                        line_height: 22.8,
                        wrap: 1,
                        ..Default::default()
                    },
                    &[],
                )
                .unwrap();
                if player.current_id.is_some() {
                    byline::draw(ui, player, catalog, 279.0, p, intent);
                }
                if player.initialized && player.title.is_empty() {
                    settings::paragraph(
                        ui,
                        "nothing-playing-hint",
                        "Double-click a song, or drop audio files anywhere in this window.",
                        279.0,
                        14.0,
                        p.secondary,
                    );
                }
            },
        );
        crate::motion::enter(
            ui,
            &information,
            1,
            (0.0, 10.0),
            crate::motion::BASE,
            crate::motion::QUICK + 0.045,
        );
        spacer(ui, "title-clock-gap", 14.0);
        let clock_group = ui.view(
            "playback-clock",
            pxui::hash!((player.revision, position.to_bits())),
            Size::Grow,
            Size::Px(43.0),
            &mut |ui| {
                let clock = ui.node(
                    "clock",
                    Node {
                        axis: Axis::Row,
                        align: Align::End,
                        w: Size::Grow,
                        h: Size::Px(43.0),
                        ..Node::DEFAULT
                    },
                );
                ui.within(&clock, &mut |ui| {
                    matrix_clock(ui, position.max(0.0) as u64, p.ink);
                    ui.box_("clock-flex", BoxOpts::default().w(Size::Grow));
                    small_clock(
                        ui,
                        "panel-remaining",
                        (player.duration - position).max(0.0) as u64,
                        true,
                        50.0,
                        p.secondary,
                    );
                });
            },
        );
        crate::motion::enter(
            ui,
            &clock_group,
            1,
            (0.0, 10.0),
            crate::motion::BASE,
            crate::motion::QUICK + 2.0 * 0.045,
        );
        spacer(ui, "clock-bottom-gap", 14.0);
        rule(ui, "context-divider", false, p);
        spacer(ui, "context-top-gap", 13.0);
        text(
            ui,
            "playback-context",
            player.context,
            12,
            p.secondary,
            (Size::Grow, 24.0),
            false,
        );
        // Keep the queue viewport stable when playback reports a notice.
        text(
            ui,
            "playback-notice",
            player.message,
            13,
            p.secondary,
            (Size::Grow, 44.0),
            false,
        );
        spacer(ui, "queue-gap", 14.0);
        rule(ui, "queue-divider", false, p);
        let queue_revision = pxui::hash!((
            catalog.queue.as_ref().map(|queue| queue.revision),
            std::sync::Arc::as_ptr(&catalog.library) as usize,
            player.artwork.map_or(0, crate::artwork::Images::revision),
        ));
        ui.view(
            "up-next",
            queue_revision,
            Size::Grow,
            Size::Grow,
            &mut |ui| queue_rows(ui, catalog, false, 279.0, player.artwork, p, intent),
        );
    });
}

/// Heart for the playing song. The burst's six rays are retained and animated once on off→on.
fn favorite_button(ui: &mut Ui, current: Option<(i64, bool)>, p: Palette) -> bool {
    use pxui::kit::{AnimationTarget as Target, Keyframe};
    const RAYS: [(&str, f32, f32); 6] = [
        ("ray0", 0.0, -1.0),
        ("ray1", 0.9, -0.5),
        ("ray2", 0.9, 0.5),
        ("ray3", 0.0, 1.0),
        ("ray4", -0.9, 0.5),
        ("ray5", -0.9, -0.5),
    ];
    let on = current.is_some_and(|(_, on)| on);
    let node = ui.node(
        "favorite",
        Node {
            axis: Axis::Stack,
            flags: if current.is_some() {
                CLICKABLE | TAB_STOP
            } else {
                0
            },
            w: Size::Px(36.0),
            h: Size::Px(36.0),
            role: Role::Button,
            state: State {
                disabled: current.is_none(),
                ..State::default()
            },
            ..Node::DEFAULT
        },
    );
    let signal = ui.signal_state(&node);
    ui.a11y_name(
        &node,
        if on {
            "Unfavorite current song"
        } else {
            "Favorite current song"
        },
    );
    // Remember (song, state) so a track change never reads as a favourite toggle.
    let seen = current.map_or(0, |(id, on)| id.wrapping_mul(2) + i64::from(on) + 1);
    let was = ui.state(&node, 0);
    let burst = current.is_some_and(|(id, on)| on && was == id.wrapping_mul(2) + 1)
        && !ui.tokens().reduced_motion;
    if was != seen {
        ui.set_state(&node, seen);
    }
    let color = if on { p.signal } else { p.secondary };
    let revision = pxui::hash!((on, signal.hovered, signal.held, current.is_some()));
    ui.within(&node, &mut |ui| {
        let heart = ui.view("heart", revision, Size::Grow, Size::Grow, &mut |ui| {
            let surface = ui.node(
                "surface",
                Node {
                    w: Size::Grow,
                    h: Size::Grow,
                    ..Node::DEFAULT
                },
            );
            ui.draw(&surface, move |paint, rect| {
                let side = 20.0;
                let area = Rect {
                    x: rect.x + (rect.w - side) / 2.0,
                    y: rect.y + (rect.h - side) / 2.0,
                    w: side,
                    h: side,
                };
                crate::icons::heart(paint, area, on, color);
            });
        });
        let _ = heart;
        for (key, dx, dy) in RAYS {
            let slot = ui.node(
                key,
                Node {
                    w: Size::Px(4.0),
                    h: Size::Px(4.0),
                    at: Some((16.0, 16.0)).into(),
                    ..Node::DEFAULT
                },
            );
            let mut ray = None;
            ui.within(&slot, &mut |ui| {
                ray = Some(ui.view("ray", 0, Size::Grow, Size::Grow, &mut |ui| {
                    let dot = ui.node(
                        "dot",
                        Node {
                            w: Size::Grow,
                            h: Size::Grow,
                            ..Node::DEFAULT
                        },
                    );
                    ui.draw(&dot, move |paint, rect| {
                        paint.fill(rect.x, rect.y, rect.w, rect.h, p.signal)
                    });
                }));
            });
            let ray = ray.unwrap();
            let curve = crate::motion::out(crate::motion::SLOW);
            if burst {
                let pose = |time, opacity, k: f32, scale| {
                    [
                        Keyframe {
                            time,
                            target: Target::Opacity(opacity),
                            curve,
                        },
                        Keyframe {
                            time,
                            target: Target::Offset(dx * 18.0 * k, dy * 18.0 * k),
                            curve,
                        },
                        Keyframe {
                            time,
                            target: Target::Scale {
                                x: scale,
                                y: scale,
                                anchor: (0.5, 0.5),
                            },
                            curve,
                        },
                    ]
                };
                let [a, b, c] = pose(0.0, 1.0, 0.0, 1.0);
                let [d, e, f] = pose(crate::motion::SLOW, 0.0, 1.0, 0.25);
                ui.animate_keys(&ray, &[a, b, c, d, e, f]);
                ui.set_state(&ray, 1);
            } else if ui.state(&ray, 0) == 0 {
                ui.set_state(&ray, 1);
                ui.animate_keys(
                    &ray,
                    &[Keyframe {
                        time: 0.0,
                        target: Target::Opacity(0.0),
                        curve,
                    }],
                );
            }
        }
    });
    signal.clicked
}
