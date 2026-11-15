use super::*;
use crate::catalog::Catalog;
use crate::{Intent, PlayerView};
use std::hash::{DefaultHasher, Hash, Hasher};

pub(super) fn now_panel(
    ui: &mut Ui,
    player: &PlayerView<'_>,
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
        let mut information_revision = DefaultHasher::new();
        (
            player.initialized,
            player.current_id,
            player.title,
            player.artist,
            player.album,
        )
            .hash(&mut information_revision);
        let information = ui.view(
            "track-information",
            information_revision.finish(),
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
            player.revision,
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
                    matrix_clock(ui, player.position.max(0.0) as u64, p.ink);
                    ui.box_("clock-flex", BoxOpts::default().w(Size::Grow));
                    small_clock(
                        ui,
                        "panel-remaining",
                        (player.duration - player.position).max(0.0) as u64,
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
        let mut queue_revision = DefaultHasher::new();
        (
            catalog.queue.as_ref().map(|queue| queue.revision),
            std::sync::Arc::as_ptr(&catalog.library) as usize,
            player.artwork.map_or(0, crate::artwork::Images::revision),
        )
            .hash(&mut queue_revision);
        ui.view(
            "up-next",
            queue_revision.finish(),
            Size::Grow,
            Size::Grow,
            &mut |ui| queue_rows(ui, catalog, false, 279.0, player.artwork, p, intent),
        );
    });
}
