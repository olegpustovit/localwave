//! pxui owns scrub capture, mapping, ordered keys and adjustable accessibility. LocalWave sizes
//! and paints the controls: seek previews its held cursor and sends release/digital commits;
//! volume previews live through twelve cells with 1/12 steps. Keep waveform max-pooling, the
//! 200 quarter-width bars and five-second seeking; no input logic belongs in these paint bodies.
use super::*;
use crate::catalog::{Catalog, Route};
use crate::{Intent, PlayerView};

pub(super) fn transport(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    panel: bool,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let narrow = ui.view_size().0 < 1100.0;
    let key_width = if narrow { 44.0 } else { 68.0 };
    let root = ui.node(
        "transport-content",
        Node {
            axis: Axis::Row,
            align: Align::Center,
            w: Size::Grow,
            h: Size::Grow,
            gap: if narrow { 4.0 } else { 16.0 },
            padding: Edges {
                r: if narrow { 8.0 } else { 20.0 },
                ..Edges::ZERO
            },
            look: Look {
                fill: p.panel,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.within(&root, &mut |ui| {
        let keys = ui.node(
            "keys",
            Node {
                axis: Axis::Row,
                gap: 2.0,
                w: Size::Px(if narrow { 138.0 } else { 210.0 }),
                h: Size::Px(71.0),
                ..Node::DEFAULT
            },
        );
        ui.within(&keys, &mut |ui| {
            if icon_button(
                ui,
                "previous",
                "Previous",
                Icon::Previous,
                (key_width, 71.0),
                false,
                p,
            ) {
                *intent = Some(Intent::Previous);
            }
            if icon_button(
                ui,
                "toggle",
                if player.playing { "Pause" } else { "Play" },
                if player.playing {
                    Icon::Pause
                } else {
                    Icon::Play
                },
                (key_width, 71.0),
                true,
                p,
            ) {
                *intent = Some(if player.playing {
                    Intent::Pause
                } else {
                    Intent::Play
                });
            }
            if icon_button(ui, "next", "Next", Icon::Next, (key_width, 71.0), false, p) {
                *intent = Some(Intent::Next);
            }
        });
        let elapsed = position.max(0.0) as u64;
        let remaining = (player.duration - position).max(0.0) as u64;
        small_clock(ui, "elapsed", elapsed, false, 44.0, p.secondary);
        seek(ui, player, position, p, intent);
        small_clock(ui, "remaining", remaining, true, 50.0, p.secondary);
        // Swift's WaveModeKeys: shuffle, repeat and the panel toggle. Volume is ⌘↑/⌘↓ and the
        // queue sheet opens from the panel; the route button is iOS-only (an invisible image on macOS).
        let modes = ui.node(
            "modes",
            Node {
                axis: Axis::Row,
                gap: 2.0,
                padding: Edges::all(2.0),
                w: Size::Px(if panel { 140.0 } else { 186.0 }),
                h: Size::Px(44.0),
                look: Look {
                    fill: p.panel,
                    ..Look::NONE
                },
                ..Node::DEFAULT
            },
        );
        ui.within(&modes, &mut |ui| {
            if icon_button(
                ui,
                "shuffle",
                if player.shuffled {
                    "Shuffle on"
                } else {
                    "Shuffle off"
                },
                Icon::Shuffle,
                (44.0, 40.0),
                player.shuffled,
                p,
            ) {
                *intent = Some(Intent::Shuffle);
            }
            let again = ui.node(
                "repeat-key",
                Node {
                    axis: Axis::Stack,
                    w: Size::Px(44.0),
                    h: Size::Px(40.0),
                    ..Node::DEFAULT
                },
            );
            ui.within(&again, &mut |ui| {
                if icon_button(
                    ui,
                    "repeat",
                    match player.repeat {
                        lw_core::queue::RepeatMode::None => "Repeat off",
                        lw_core::queue::RepeatMode::All => "Repeat all",
                        lw_core::queue::RepeatMode::One => "Repeat one",
                    },
                    Icon::Repeat,
                    (44.0, 40.0),
                    player.repeat != lw_core::queue::RepeatMode::None,
                    p,
                ) {
                    *intent = Some(Intent::Repeat);
                }
                if player.repeat == lw_core::queue::RepeatMode::One {
                    let badge = ui.node(
                        "repeat-one",
                        Node {
                            at: Some((31.0, 25.0)).into(),
                            w: Size::Px(8.0),
                            h: Size::Px(12.0),
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&badge, &mut |ui| {
                        text(ui, "one", "1", 9, p.paper, (Size::Px(8.0), 12.0), true);
                    });
                }
            });
            if icon_button(
                ui,
                "panel",
                "Toggle now playing panel",
                Icon::Panel,
                (44.0, 40.0),
                panel,
                p,
            ) {
                *intent = Some(Intent::TogglePanel);
            }
            // The queue sheet is otherwise opened from the panel, which may be hidden or collapsed.
            if !panel
                && icon_button(
                    ui,
                    "queue",
                    "Show full queue",
                    Icon::Queue,
                    (44.0, 40.0),
                    false,
                    p,
                )
            {
                *intent = Some(Intent::ShowQueue);
            }
        });
    });
}

pub(super) fn desktop(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    catalog: &Catalog,
    state: &ShellState,
    p: Palette,
    intent: &mut Option<Intent>,
) -> Handle {
    let (width, height) = ui.view_size();
    let [safe_top, safe_right, safe_bottom, safe_left] = ui.environment().safe_area;
    let width = (width - safe_left - safe_right).max(1.0);
    let height = (height - safe_bottom).max(1.0);
    let top = state.top_inset.max(safe_top);
    let panel = state.panel_visible
        && width >= 900.0
        && height - top >= 640.0;
    let sidebar_width = if width >= 900.0 { 231.0 } else { 180.0 };
    let content_width = (width - sidebar_width - 1.0 - if panel { 320.0 } else { 0.0 }).max(100.0);
    let root = ui.node(
        "shell",
        Node {
            axis: Axis::Column,
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges {
                l: safe_left,
                r: safe_right,
                b: safe_bottom,
                ..Edges::ZERO
            },
            look: Look {
                fill: p.paper,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.set_role(&root, Role::Window, 0);
    ui.a11y_name(
        &root,
        if player.file_dragging {
            "LocalWave — Drop to play. Folders are indexed. Audio files join Added Files."
        } else {
            "LocalWave"
        },
    );
    ui.within(&root, &mut |ui| {
        spacer(ui, "titlebar-clearance", top);
        let work = container(ui, "workspace", Axis::Row, Size::Grow, Size::Grow, p.paper);
        ui.within(&work, &mut |ui| {
            let revision = state.revision.wrapping_mul(2) | catalog.all.is_some() as u64;
            ui.view(
                "sidebar",
                revision,
                Size::Px(sidebar_width),
                Size::Grow,
                &mut |ui| sidebar(ui, catalog, state, sidebar_width, p, intent),
            );
            rule(ui, "sidebar-divider", true, p);
            let library_revision = library_revision(player, catalog, state);
            let page = ui.view(
                "library",
                library_revision,
                Size::Grow,
                Size::Grow,
                &mut |ui| {
                    let route_key = if let Some(group) = catalog
                        .collection(state.route)
                        .filter(|_| state.route != Route::Find)
                    {
                        crate::hash((true, group.key))
                    } else {
                        crate::hash((false, state.route))
                    };
                    let route = ui.node_keyed(
                        int_key(Ui::key_of(0, "localwave-library-route"), route_key as i64),
                        Node {
                            w: Size::Grow,
                            h: Size::Grow,
                            ..Node::DEFAULT
                        },
                    );
                    ui.within(&route, &mut |ui| {
                        if state.settings_visible {
                            settings::view(ui, player, state, content_width, p, intent);
                        } else if state.route == Route::Files {
                            files::view(ui, player, state, content_width, p, intent);
                        } else {
                            content(ui, catalog, state, player, content_width, p, intent);
                        }
                    });
                },
            );
            ui.view_clip(
                &page,
                Some(Rect {
                    x: safe_left + sidebar_width + 1.0,
                    y: top,
                    w: content_width,
                    h: (height - top - 72.0).max(0.0),
                }),
            );
            crate::motion::enter(
                ui,
                &page,
                state.entry_revision,
                (if width < 900.0 { 32.0 } else { 0.0 }, 0.0),
                crate::motion::BASE,
                0.0,
            );
            if panel {
                rule(ui, "panel-divider", true, p);
                let panel_revision = crate::hash((
                    (player.revision, position.to_bits()),
                    player.artwork.map_or(0, crate::artwork::Images::revision),
                    catalog.queue.as_ref().map(|queue| queue.revision),
                    std::sync::Arc::as_ptr(&catalog.library) as usize,
                ));
                // The drop target wraps the cached panel: hover and drops never rebuild its view.
                let target = ui.node(
                    "up-next-drop",
                    Node {
                        // Hit-registered so drops on its empty areas reach it.
                        flags: CLICKABLE,
                        w: Size::Px(319.0),
                        h: Size::Grow,
                        ..Node::DEFAULT
                    },
                );
                ui.within(&target, &mut |ui| {
                    let panel = ui.view(
                        "now-panel",
                        panel_revision,
                        Size::Grow,
                        Size::Grow,
                        &mut |ui| now_panel(ui, player, position, catalog, p, intent),
                    );
                    ui.view_clip(
                        &panel,
                        Some(Rect {
                            x: safe_left + width - 319.0,
                            y: top,
                            w: 319.0,
                            h: (height - top - 72.0).max(0.0),
                        }),
                    );
                    crate::motion::enter(ui, &panel, 1, (32.0, 0.0), crate::motion::SLOW, 0.0);
                });
                match songs::song_drop(ui, &target) {
                    Some((true, song)) => *intent = Some(Intent::AppendSong(song)),
                    Some((false, _)) => ui.draw_over(&target, move |paint, rect| {
                        for (x, y, w, h) in [
                            (0.0, 0.0, rect.w, 2.0),
                            (0.0, rect.h - 2.0, rect.w, 2.0),
                            (0.0, 0.0, 2.0, rect.h),
                            (rect.w - 2.0, 0.0, 2.0, rect.h),
                        ] {
                            paint.fill(rect.x + x, rect.y + y, w, h, p.signal);
                        }
                    }),
                    None => {}
                }
            }
        });
        rule(ui, "transport-divider", false, p);
        ui.view(
            "transport",
            crate::hash((player.revision, position.to_bits(), panel)),
            Size::Grow,
            Size::Px(71.0),
            &mut |ui| transport(ui, player, position, panel, p, intent),
        );
    });
    root
}

pub(super) fn seek(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let duration = if player.duration.is_finite() {
        player.duration.max(0.0)
    } else {
        0.0
    };
    let position = if position.is_finite() {
        position.clamp(0.0, duration)
    } else {
        0.0
    };
    let upper = duration.min(f64::from(f32::MAX)) as f32;
    let scrub = ui.scrub(
        "Playback position",
        position.min(f64::from(upper)) as f32,
        0.0..=upper,
        5.0,
    );
    let scrubber = scrub.node;
    ui.set_node_layout(
        &scrubber,
        &Node {
            w: Size::Grow,
            h: Size::Px(if ui.view_size().0 < compact::BREAKPOINT {
                56.0
            } else {
                40.0
            }),
            ..Node::DEFAULT
        },
    );
    let fraction = if let Some(preview) = scrub.preview {
        preview / upper
    } else if duration > 0.0 {
        (position / duration) as f32
    } else {
        0.0
    };
    if let Some(seconds) = scrub.commit {
        let fraction = if seconds == upper {
            1.0
        } else {
            (f64::from(seconds) / duration).clamp(0.0, 1.0)
        };
        *intent = Some(Intent::SeekFraction(fraction));
    }
    let peaks = player.waveform.copied();
    ui.draw(&scrubber, move |paint, rect| {
        const BARS: usize = 200;
        let baseline = (rect.h - 4.0).max(0.0);
        let step = rect.w / BARS as f32;
        if let Some(peaks) = &peaks {
            for index in 0..BARS {
                let lower = index * peaks.len() / BARS;
                let upper = ((index + 1) * peaks.len() / BARS).max(lower + 1);
                let peak = peaks[lower..upper]
                    .iter()
                    .copied()
                    .fold(0.0, f32::max)
                    .clamp(0.0, 1.0);
                let height = (peak * baseline).max(2.0);
                paint.fill_fractional(
                    rect.x + index as f32 * step,
                    rect.y + baseline - height,
                    step / 4.0,
                    height,
                    Rgba {
                        a: (p.ink.a as f32 * 0.42).round() as u8,
                        ..p.ink
                    },
                );
            }
        }
        paint.fill(rect.x, rect.y + baseline + 1.0, rect.w, 1.0, p.line);
        paint.fill_fractional(rect.x, rect.y + baseline, rect.w * fraction, 3.0, p.ink);
        paint.fill_fractional(
            rect.x + rect.w * fraction - step / 4.0,
            rect.y,
            step / 2.0,
            rect.h,
            p.signal,
        );
    });
}
