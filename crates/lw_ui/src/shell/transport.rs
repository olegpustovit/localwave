use super::*;
use crate::catalog::{Catalog, Route};
use crate::{Intent, PlayerView};
use pxui::geom::hash::FastHasher;
use std::hash::{Hash, Hasher};

pub(super) fn transport(
    ui: &mut Ui,
    player: &PlayerView<'_>,
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
        let elapsed = player.position.max(0.0) as u64;
        let remaining = (player.duration - player.position).max(0.0) as u64;
        small_clock(ui, "elapsed", elapsed, false, 44.0, p.secondary);
        seek(ui, player, p, intent);
        small_clock(ui, "remaining", remaining, true, 50.0, p.secondary);
        if icon_button(
            ui,
            "shuffle",
            if player.shuffled {
                "Shuffle on"
            } else {
                "Shuffle off"
            },
            Icon::Shuffle,
            (36.0, 40.0),
            player.shuffled,
            p,
        ) {
            *intent = Some(Intent::Shuffle);
        }
        if icon_button(
            ui,
            "repeat",
            match player.repeat {
                lw_core::queue::RepeatMode::None => "Repeat off",
                lw_core::queue::RepeatMode::All => "Repeat all",
                lw_core::queue::RepeatMode::One => "Repeat one",
            },
            Icon::Repeat,
            (36.0, 40.0),
            player.repeat != lw_core::queue::RepeatMode::None,
            p,
        ) {
            *intent = Some(Intent::Repeat);
        }
        volume(
            ui,
            player,
            Size::Px(if narrow { 60.0 } else { 80.0 }),
            p,
            intent,
        );
        if icon_button(ui, "stop", "Stop", Icon::Stop, (36.0, 40.0), false, p) {
            *intent = Some(Intent::Stop);
        }
        if icon_button(
            ui,
            "queue",
            "Show full queue",
            Icon::Queue,
            (36.0, 40.0),
            false,
            p,
        ) {
            *intent = Some(Intent::ShowQueue);
        }
        if icon_button(
            ui,
            "panel",
            "Toggle player panel",
            Icon::Panel,
            (36.0, 40.0),
            false,
            p,
        ) {
            *intent = Some(Intent::TogglePanel);
        }
    });
}

pub(super) fn desktop(
    ui: &mut Ui,
    player: &PlayerView<'_>,
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
    let panel = !state.settings_visible
        && state.route != Route::Files
        && state.panel_visible
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
            let mut library_revision = FastHasher::default();
            (
                revision,
                (player.current_id, player.playing),
                player.artwork.map_or(0, crate::artwork::Images::revision),
                player.settings,
                player.cache_usage.as_ptr() as usize,
                player.cache_usage.len(),
                player.storage_busy,
                player.settings_message.as_ptr() as usize,
                player.settings_message.len(),
                player.message.as_ptr() as usize,
                player.message.len(),
                if state.files_visible() {
                    player.files.map_or(0, |files| files.revision)
                } else {
                    0
                },
            )
                .hash(&mut library_revision);
            let page = ui.view(
                "library",
                library_revision.finish(),
                Size::Grow,
                Size::Grow,
                &mut |ui| {
                    let mut route_key = FastHasher::default();
                    if let Some(group) = catalog
                        .collection(state.route)
                        .filter(|_| state.route != Route::Find)
                    {
                        (true, group.key).hash(&mut route_key);
                    } else {
                        (false, state.route).hash(&mut route_key);
                    }
                    let route = ui.node_keyed(
                        int_key(
                            Ui::key_of(0, "localwave-library-route"),
                            route_key.finish() as i64,
                        ),
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
                            files::view(ui, player, content_width, p, intent);
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
                let mut panel_revision = FastHasher::default();
                (
                    player.revision,
                    player.artwork.map_or(0, crate::artwork::Images::revision),
                    catalog.queue.as_ref().map(|queue| queue.revision),
                    std::sync::Arc::as_ptr(&catalog.library) as usize,
                )
                    .hash(&mut panel_revision);
                let panel = ui.view(
                    "now-panel",
                    panel_revision.finish(),
                    Size::Px(319.0),
                    Size::Grow,
                    &mut |ui| now_panel(ui, player, catalog, p, intent),
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
            }
        });
        rule(ui, "transport-divider", false, p);
        ui.view(
            "transport",
            player.revision,
            Size::Grow,
            Size::Px(71.0),
            &mut |ui| transport(ui, player, p, intent),
        );
    });
    root
}

pub(super) fn seek(ui: &mut Ui, player: &PlayerView<'_>, p: Palette, intent: &mut Option<Intent>) {
    let duration = if player.duration.is_finite() {
        player.duration.max(0.0)
    } else {
        0.0
    };
    let position = if player.position.is_finite() {
        player.position.clamp(0.0, duration)
    } else {
        0.0
    };
    let scrubber = ui.node(
        "scrubber",
        Node {
            flags: CLIP
                | if duration > 0.0 {
                    CLICKABLE | FOCUSABLE | FOCUS_MARK
                } else {
                    0
                },
            w: Size::Grow,
            h: Size::Px(if ui.view_size().0 < compact::BREAKPOINT {
                44.0
            } else {
                40.0
            }),
            role: if duration > 0.0 {
                Role::Slider
            } else {
                Role::Progress
            },
            ..Node::DEFAULT
        },
    );
    ui.a11y_name(&scrubber, "Playback position");
    ui.a11y_number(&scrubber, position, 0.0, duration, 5.0);
    let signal = ui.signal_state(&scrubber);
    let rect = ui.rect(&scrubber);
    let mut fraction = if duration > 0.0 {
        (position / duration) as f32
    } else {
        0.0
    };
    if duration > 0.0 && rect.w > 0.0 {
        if signal.held || signal.released {
            let x = if signal.released {
                signal.release_point.0
            } else {
                signal.pointer.0
            };
            fraction = ((x - rect.x) / rect.w).clamp(0.0, 1.0);
        }
        if signal.released && signal.button == Some(0) {
            *intent = Some(Intent::SeekFraction(fraction as f64));
        }
        if let Some(value) = signal.a11y_value.filter(|value| value.is_finite()) {
            *intent = Some(Intent::SeekFraction((value / duration).clamp(0.0, 1.0)));
        }
        if ui.focused(&scrubber) {
            let seconds = if ui.key_pressed(input::Key::ArrowLeft) {
                Some(position - 5.0)
            } else if ui.key_pressed(input::Key::ArrowRight) {
                Some(position + 5.0)
            } else if ui.key_pressed(input::Key::Home) {
                Some(0.0)
            } else if ui.key_pressed(input::Key::End) {
                Some(duration)
            } else {
                None
            };
            if let Some(seconds) = seconds {
                *intent = Some(Intent::SeekFraction((seconds / duration).clamp(0.0, 1.0)));
            }
        }
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

pub(super) fn volume(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    width: Size,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let volume = ui.node(
        "volume",
        Node {
            flags: CLICKABLE | TAB_STOP,
            w: width,
            h: Size::Px(if ui.view_size().0 < compact::BREAKPOINT {
                44.0
            } else {
                40.0
            }),
            role: Role::Slider,
            ..Node::DEFAULT
        },
    );
    ui.a11y_name(&volume, "Volume");
    ui.a11y_number(&volume, player.volume as f64, 0.0, 1.0, 0.05);
    let fraction = player.volume.clamp(0.0, 1.0);
    ui.draw(&volume, move |paint, rect| {
        paint.fill(rect.x, rect.y + 19.0, rect.w, 2.0, p.line);
        paint.fill(rect.x, rect.y + 19.0, rect.w * fraction, 2.0, p.ink);
        paint.fill(
            rect.x + (rect.w - 2.0) * fraction,
            rect.y + 14.0,
            2.0,
            12.0,
            p.ink,
        );
    });
    let signal = ui.signal_state(&volume);
    let rect = ui.rect(&volume);
    if (signal.clicked || signal.dragging) && rect.w > 0.0 {
        *intent = Some(Intent::Volume(
            ((signal.pointer.0 - rect.x) / rect.w).clamp(0.0, 1.0),
        ));
    }
    if let Some(value) = signal.a11y_value {
        *intent = Some(Intent::Volume(value.clamp(0.0, 1.0) as f32));
    }
}
