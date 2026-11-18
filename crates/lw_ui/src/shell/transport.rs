//! pxui owns scrub capture, mapping, ordered keys and adjustable accessibility. LocalWave sizes
//! and paints the controls: seek previews its held cursor and sends release/digital commits;
//! volume previews live through twelve cells with 1/12 steps. Keep waveform max-pooling, the
//! 200 quarter-width bars and five-second seeking; no input logic belongs in these paint bodies.
use super::*;
use crate::catalog::{Catalog, Route};
use crate::{Intent, PlayerView};

/// The playback scrubber: arrow keys move five seconds.
const SCRUBBER: pxui::Scrubber = pxui::Scrubber::DEFAULT.step(5.0).name("Playback position");

pub(super) fn transport(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    panel: bool,
    intent: &mut Option<Intent>,
) {
    let narrow = ui.view_size().width < 1100.0;
    let key_width = if narrow { 44.0 } else { 68.0 };
    ui.row(Node {
            padding: Edges {
                r: if narrow { 8.0 } else { 20.0 },
                ..Edges::ZERO
            },
            look: PANEL_FILL,
            ..Node::row().w(Size::Grow).h(Size::Grow).gap(if narrow { 4.0 } else { 16.0 }).align(Align::Center)
        },
        |ui| {
            ui.row(Node::row().w_px(if narrow { 138.0 } else { 210.0 }).h_px(71.0).gap(2.0), |ui| {
                if ui.icon_button("previous", icon_drawing(Icon::Previous), "Previous", &icon_key(key_width, 71.0).look(&ICON_PLATE_LOOKS)).clicked {
                    *intent = Some(Intent::Previous);
                }
                if ui.icon_button(
                    "toggle",
                    icon_drawing(if player.playing { Icon::Pause } else { Icon::Play }),
                    if player.playing { "Pause" } else { "Play" },
                    &icon_key(key_width, 71.0).look(&ICON_INVERSE_LOOKS),
                ).clicked {
                    *intent = Some(if player.playing {
                        Intent::Pause
                    } else {
                        Intent::Play
                    });
                }
                if ui.icon_button("next", icon_drawing(Icon::Next), "Next", &icon_key(key_width, 71.0).look(&ICON_PLATE_LOOKS)).clicked {
                    *intent = Some(Intent::Next);
                }
            });
            let elapsed = position.max(0.0) as u64;
            let remaining = (player.duration - position).max(0.0) as u64;
            small_clock(ui, "elapsed", elapsed, false, 44.0, SECONDARY);
            seek(ui, player, position, intent);
            small_clock(ui, "remaining", remaining, true, 50.0, SECONDARY);
            // Swift's WaveModeKeys: shuffle, repeat and the panel toggle. Volume is ⌘↑/⌘↓ and the
            // queue sheet opens from the panel; the route button is iOS-only (an invisible image on macOS).
            ui.row(Node {
                    look: PANEL_FILL,
                    ..Node::row().w_px(if panel { 140.0 } else { 186.0 }).h_px(44.0).gap(2.0).pad(2.0)
                },
                |ui| {
                    if ui.icon_button(
                        "shuffle",
                        icon_drawing(Icon::Shuffle),
                        if player.shuffled { "Shuffle on" } else { "Shuffle off" },
                        &ICON_44X40.look(if player.shuffled { &ICON_INVERSE_LOOKS } else { &ICON_PLATE_LOOKS }),
                    ).clicked {
                        *intent = Some(Intent::Shuffle);
                    }
                    ui.stack(Node::stack().w_px(44.0).h_px(40.0), |ui| {
                        if ui.icon_button(
                            "repeat",
                            icon_drawing(Icon::Repeat),
                            match player.repeat {
                                lw_core::queue::RepeatMode::None => "Repeat off",
                                lw_core::queue::RepeatMode::All => "Repeat all",
                                lw_core::queue::RepeatMode::One => "Repeat one",
                            },
                            &ICON_44X40.look(if player.repeat != lw_core::queue::RepeatMode::None { &ICON_INVERSE_LOOKS } else { &ICON_PLATE_LOOKS }),
                        ).clicked {
                            *intent = Some(Intent::Repeat);
                        }
                        if player.repeat == lw_core::queue::RepeatMode::One {
                            ui.id("repeat-one", |ui| { ui.column(Node {
                                    at: Some((31.0, 25.0)).into(),
                                    ..Node::column().w_px(8.0).h_px(12.0)
                                },
                                |ui| {
                                    ui.label("1", MONO_9.w_px(8.0).h_px(12.0).color(PAPER));
                                },
                            ); });
                        }
                    });
                    if ui.icon_button(
                        "panel",
                        icon_drawing(Icon::Panel),
                        "Toggle now playing panel",
                        &ICON_44X40.look(if panel { &ICON_INVERSE_LOOKS } else { &ICON_PLATE_LOOKS }),
                    ).clicked {
                        *intent = Some(Intent::TogglePanel);
                    }
                    // The queue sheet is otherwise opened from the panel, which may be hidden or collapsed.
                    if !panel
                        && ui.icon_button("queue", icon_drawing(Icon::Queue), "Show full queue", &ICON_44X40.look(&ICON_PLATE_LOOKS)).clicked
                    {
                        *intent = Some(Intent::ShowQueue);
                    }
                },
            );
        },
    );
}

pub(super) fn desktop(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    catalog: &Catalog,
    state: &ShellState,
    intent: &mut Option<Intent>,
) -> Handle {
    let pxui::ViewSize { width, height } = ui.view_size();
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
            padding: Edges {
                l: safe_left,
                r: safe_right,
                b: safe_bottom,
                ..Edges::ZERO
            },
            look: PAPER_FILL,
            ..Node::column().w(Size::Grow).h(Size::Grow)
        },
    );
    ui.set_role(&root, Role::Window, pxui::A11yStates::NONE);
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
        let work = container(ui, "workspace", Axis::Row, Size::Grow, Size::Grow, ui.color(PAPER));
        ui.within(&work, &mut |ui| {
            // A carry hovering a list row or a queue gap highlights it and a drop reads once; the engine
            // does not say which, so the views showing drop targets rebuild on every frame of a carry
            // and the one after it.
            let carry = if ui.carrying().is_some() {
                crate::hash((1u8, ui.frame_index()))
            } else {
                u64::from(state.carried.get()) * 2
            };
            let revision = crate::hash((state.revision, catalog.all.is_some(), carry));
            ui.column(Node::column().w_px(sidebar_width).h(Size::Grow), |ui| { ui.view_of("sidebar", &revision, |ui| sidebar(ui, catalog, state, sidebar_width, intent)); });
            rule(ui, "sidebar-divider", true);
            let library_revision = library_revision(player, catalog, state);
            let page = ui.view_of("library", &library_revision, |ui| {
                    let route_key = if let Some(group) = catalog
                        .collection(state.route)
                        .filter(|_| state.route != Route::Find)
                    {
                        crate::hash((true, group.key))
                    } else {
                        crate::hash((false, state.route))
                    };
                    let route = ui.node(
                        Key::Id(route_key as i64),
                        Node::column().w(Size::Grow).h(Size::Grow),
                    );
                    ui.within(&route, &mut |ui| {
                        if state.settings_visible {
                            settings::view(ui, player, state, content_width, intent);
                        } else if state.route == Route::Files {
                            files::view(ui, player, state, content_width, intent);
                        } else {
                            content(ui, catalog, state, player, content_width, intent);
                        }
                    });
                });
            ui.set_view(
                &page,
                &pxui::ViewTransform::DEFAULT.clip(Some(Rect {
                    x: safe_left + sidebar_width + 1.0,
                    y: top,
                    w: content_width,
                    h: (height - top - 72.0).max(0.0),
                })),
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
                rule(ui, "panel-divider", true);
                // The drop target wraps the cached panel: hover and drops never rebuild its view.
                // Pressable so drops on its empty areas reach it; no tab stop.
                let target = ui.pressable("up-next-drop", Node::column().w_px(319.0).h(Size::Grow), &PLAIN.tab_stop(false), |ui, _| {
                    let panel = ui.view_of(
                        "now-panel",
                        &(
                            (player.revision, position.to_bits(), carry),
                            player.artwork.map_or(0, crate::artwork::Images::revision),
                            catalog.queue.as_ref().map(|queue| queue.revision),
                            std::sync::Arc::as_ptr(&catalog.library) as usize,
                        ),
                        |ui| now_panel(ui, player, position, catalog, carry, intent),
                    );
                    ui.set_view(
                        &panel,
                        &pxui::ViewTransform::DEFAULT.clip(Some(Rect {
                            x: safe_left + width - 319.0,
                            y: top,
                            w: 319.0,
                            h: (height - top - 72.0).max(0.0),
                        })),
                    );
                    crate::motion::enter(ui, &panel, 1, (32.0, 0.0), crate::motion::SLOW, 0.0);
                });
                let target = target.handle;
                let accent = ui.tokens().accent;
                match songs::song_drop(ui, &target) {
                    Some((true, song)) => *intent = Some(Intent::AppendSong(song)),
                    Some((false, _)) => ui.canvas_over(&target, move |paint, rect| {
                        for (x, y, w, h) in [
                            (0.0, 0.0, rect.w, 2.0),
                            (0.0, rect.h - 2.0, rect.w, 2.0),
                            (0.0, 0.0, 2.0, rect.h),
                            (rect.w - 2.0, 0.0, 2.0, rect.h),
                        ] {
                            paint.fill(Rect::new(rect.x + x, rect.y + y, w, h), Fill::new(accent));
                        }
                    }),
                    None => {}
                }
            }
        });
        rule(ui, "transport-divider", false);
        ui.column(Node::column().h_px(71.0), |ui| { ui.view_of("transport", &(crate::hash(( (player.revision, position.to_bits(), panel), (player.playing, player.shuffled, player.repeat as u8), (player.duration.to_bits(), width.to_bits()), ))), |ui| transport(ui, player, position, panel, intent)); });
    });
    // A carry a row starts mid-build shows from the next frame; one that ended shows its last view once.
    state.carried.set(ui.carrying().is_some());
    root
}

pub(super) fn seek(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
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
        "playback-position",
        position.min(f64::from(upper)) as f32,
        0.0..=upper,
        &SCRUBBER,
    );
    let scrubber = scrub.response.handle;
    ui.set_node_layout(
        &scrubber,
        &Node::column()
            .w(Size::Grow)
            .h_px(if ui.view_size().width < compact::BREAKPOINT {
                56.0
            } else {
                40.0
            }),
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
    let accent = ui.tokens().accent;
    ui.canvas(&scrubber, move |paint, rect| {
        const BARS: usize = 200;
        let baseline = (rect.h - 4.0).max(0.0);
        let step = rect.w / BARS as f32;
        let ink = paint.color(INK);
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
                paint.fill(Rect::new(rect.x + index as f32 * step, rect.y + baseline - height, step / 4.0, height), Fill::new(Rgba { a: (ink.a as f32 * 0.42).round() as u8, ..ink }).fractional());
            }
        }
        paint.fill(Rect::new(rect.x, rect.y + baseline + 1.0, rect.w, 1.0), Fill::new(paint.color(LINE)));
        paint.fill(Rect::new(rect.x, rect.y + baseline, rect.w * fraction, 3.0), Fill::new(ink).fractional());
        paint.fill(Rect::new(rect.x + rect.w * fraction - step / 4.0, rect.y, step / 2.0, rect.h), Fill::new(accent).fractional());
    });
}
