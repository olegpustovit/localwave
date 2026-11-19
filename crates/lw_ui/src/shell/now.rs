use super::*;
use crate::catalog::Catalog;
use crate::{Intent, PlayerView};
/// The now panel's track line: ink, at least one 22.8-point line tall.
const CURRENT_TRACK: Node = Node::column().w(Size::Grow).h(Size::Fit).role(Role::Label).min_h(22.8).look(INK_TEXT);
/// A favourite burst ray's 4-point slot at the heart's centre.
const RAY_SLOT: Node = Node::column().w_px(4.0).h_px(4.0).at(16.0, 12.0);

pub(super) fn now_panel(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    catalog: &Catalog,
    carry: u64,
    intent: &mut Option<Intent>,
) {
    ui.column(Node::column().w(Size::Grow).h(Size::Grow).padding(Edges::new(20.0, 8.0, 20.0, 0.0)),
        |ui| {
            ui.row(Node::row().w_px(289.0).h_px(36.0).align(Align::Center), |ui| {
                ui.label("Now playing", CAPTION.grow());
                if ui.icon_button("hide-panel", icon_drawing(Icon::Panel), "Minimise to small player", &ICON_36.look(&ICON_PLATE_LOOKS)).clicked {
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
                &PANEL_FILL,
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
            let favorite = player.current_id.map(|id| {
                (
                    id,
                    catalog
                        .row_index(id)
                        .is_some_and(|i| catalog.row(i).favorite),
                )
            });
            // Swift's heart sits at the end of the title row, level with the title.
            ui.row(Node::row().w(Size::Grow).h(Size::Fit), |ui| {
                let information_revision = crate::hash((
                    player.revision,
                    std::sync::Arc::as_ptr(&catalog.library) as usize,
                ));
                let information_box = ui.node("track-information", Node::column());
                let mut information = information_box;
                ui.within(&information_box, &mut |ui| information = ui.view_of("track-information", &information_revision, |ui| {
                        let title = ui.node(
                            "current-track",
                            CURRENT_TRACK,
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
                            Label::BODY.size(19.0).weight(500).tracking(-0.015).line_height(22.8).wrap(),
                            &[],
                        )
                        .unwrap();
                        if player.current_id.is_some() {
                            byline::draw(ui, player, catalog, 243.0, intent);
                        }
                        if player.initialized && player.title.is_empty() {
                            ui.label("Double-click a song, or drop audio files anywhere in this window.", PARAGRAPH_14.color(SECONDARY));
                        }
                    }));
                crate::motion::enter(
                    ui,
                    &information,
                    1,
                    (0.0, 10.0),
                    crate::motion::BASE,
                    crate::motion::QUICK + 0.045,
                );
                if favorite_button(ui, favorite)
                    && let Some((song, on)) = favorite
                {
                    *intent = Some(Intent::SetFavorite { song, on: !on });
                }
            });
            spacer(ui, "title-clock-gap", 14.0);
            let clock_group_box = ui.node("playback-clock", Node::column().h_px(43.0));
            let mut clock_group = clock_group_box;
            ui.within(&clock_group_box, &mut |ui| clock_group = ui.view_of("playback-clock", &(crate::hash((player.revision, position.to_bits()))), |ui| {
                    ui.row(Node::row().w(Size::Grow).h_px(43.0).align(Align::End), |ui| {
                        matrix_clock(ui, position.max(0.0) as u64, 43.0, INK);
                        ui.node("clock-flex", Node::DEFAULT.w(Size::Grow));
                        small_clock(
                            ui,
                            "panel-remaining",
                            (player.duration - position).max(0.0) as u64,
                            true,
                            50.0,
                            SECONDARY,
                        );
                    });
                }));
            crate::motion::enter(
                ui,
                &clock_group,
                1,
                (0.0, 10.0),
                crate::motion::BASE,
                crate::motion::QUICK + 2.0 * 0.045,
            );
            spacer(ui, "clock-bottom-gap", 14.0);
            metadata(ui, player.info);
            // A playback notice takes room from the (growing) queue viewport only while it shows.
            if !player.message.is_empty() {
                ui.label(player.message, TEXT_13.grow().h_px(44.0).color(SECONDARY));
            }
            spacer(ui, "queue-gap", 14.0);
            rule(ui, "queue-divider", false);
            ui.view_of(
                "up-next",
                &(
                    catalog.queue.as_ref().map(|queue| queue.revision),
                    std::sync::Arc::as_ptr(&catalog.library) as usize,
                    player.artwork.map_or(0, crate::artwork::Images::revision),
                    carry,
                ),
                |ui| queue_rows(ui, catalog, false, player.artwork, intent),
            );
        },
    );
}

/// The heart key: 36 by 28 points, no look of its own.
const FAVORITE: pxui::Button = pxui::Button::sized(36.0, 28.0).look(&BARE);
/// Heart for the playing song. The burst's six rays are retained and animated once on off→on.
pub(super) fn favorite_button(ui: &mut Ui, current: Option<(i64, bool)>) -> bool {
    use pxui::{ui::Keyframe, Keyframes};
    const RAYS: [(&str, f32, f32); 6] = [
        ("ray0", 0.0, -1.0),
        ("ray1", 0.9, -0.5),
        ("ray2", 0.9, 0.5),
        ("ray3", 0.0, 1.0),
        ("ray4", -0.9, 0.5),
        ("ray5", -0.9, -0.5),
    ];
    let on = current.is_some_and(|(_, on)| on);
    let favorite = FAVORITE.disabled(!current.is_some()).name(if on { "Unfavorite current song" } else { "Favorite current song" });
    // the heart and its burst are a stack of their own, built with the press state
    let signal = ui.button_content("favorite", &favorite, |ui, press| {
        let node = ui.node("layers", Node::stack().w(Size::Grow).h(Size::Grow));
        // Remember (song, state) so a track change never reads as a favourite toggle.
        let seen = current.map_or(0, |(id, on)| id.wrapping_mul(2) + i64::from(on) + 1);
        let was = ui.state(&node, 0);
        let burst = current.is_some_and(|(id, on)| on && was == id.wrapping_mul(2) + 1)
            && !ui.tokens().reduced_motion;
        if was != seen {
            ui.set_state(&node, seen);
        }
        let color = if on { ui.tokens().accent } else { Rgba::token(SECONDARY) };
        ui.within(&node, &mut |ui| {
            let heart = ui.view_of("heart", &(on, press.hovered, press.pressed, current.is_some()), |ui| {
                let surface = ui.node("surface", Node::column().w(Size::Grow).h(Size::Grow));
                ui.canvas(&surface, move |paint, rect| {
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
                    RAY_SLOT,
                );
                let mut ray = None;
                ui.within(&slot, &mut |ui| {
                    ray = Some(ui.view_of("ray", &0, |ui| {
                        let dot = ui.node("dot", Node::column().w(Size::Grow).h(Size::Grow));
                        let accent = ui.tokens().accent;
                        ui.canvas(&dot, move |paint, rect| {
                            paint.fill(rect, Fill::new(accent))
                        });
                    }));
                });
                let ray = ray.unwrap();
                let curve = crate::motion::out(crate::motion::SLOW);
                if burst {
                    let pose = |time, opacity, k: f32, scale| {
                        [
                            Keyframe::opacity(time, opacity).curve(curve),
                            Keyframe::offset(time, dx * 18.0 * k, dy * 18.0 * k).curve(curve),
                            Keyframe::scale(time, scale, scale).curve(curve),
                        ]
                    };
                    let [a, b, c] = pose(0.0, 1.0, 0.0, 1.0);
                    let [d, e, f] = pose(crate::motion::SLOW, 0.0, 1.0, 0.25);
                    ui.animate_keys(&ray, &[a, b, c, d, e, f], &Keyframes::DEFAULT);
                    ui.set_state(&ray, 1);
                } else if ui.state(&ray, 0) == 0 {
                    ui.set_state(&ray, 1);
                    ui.animate_keys(
                        &ray,
                        &[Keyframe::opacity(0.0, 0.0).curve(curve)],
                        &Keyframes::DEFAULT,
                    );
                }
            }
        });
    });
    signal.clicked
}

/// Format / Size / File / Plays under a hairline, drawn from the worker-prepared strings.
fn metadata(ui: &mut Ui, info: Option<&crate::now_info::NowInfo>) {
    let Some(info) = info else {
        return;
    };
    spacer(ui, "metadata-gap", 14.0);
    rule(ui, "metadata-divider", false);
    spacer(ui, "metadata-top", 13.0);
    let revision = crate::hash(info as *const crate::now_info::NowInfo as usize);
    ui.column(Node::column().h_px(79.2), |ui| { ui.view_of("now-metadata", &revision, |ui| {
            ui.column(Node::column().w(Size::Grow).h(Size::Fit).gap(4.0), |ui| {
                for (key, label, value, ink) in [
                    ("format", "Format", &info.format, SECONDARY),
                    ("size", "Size", &info.size, SECONDARY),
                    ("file", "File", &info.file, INK),
                    ("plays", "Plays", &info.plays, SECONDARY),
                ] {
                    let row = ui.node(key, Node::row().w(Size::Grow).h_px(16.8).gap(8.0).role(Role::Label));
                    ui.within(&row, &mut |ui| {
                        ui.label(label, FOOTNOTE.w_px(64.0));
                        ui.label(value, MONO_12.grow().color(ink));
                    });
                    ui.a11y_name(&row, if key == "file" { &info.folder } else { value });
                }
            });
        }); });
}
