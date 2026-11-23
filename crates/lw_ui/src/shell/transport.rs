//! pxui owns scrub capture, mapping, ordered keys and adjustable accessibility. LocalWave sizes
//! and paints the controls: seek previews its held cursor and sends release/digital commits;
//! volume previews live through twelve cells with 1/12 steps. Keep waveform max-pooling, the
//! 200 quarter-width bars and five-second seeking; no input logic belongs in these paint bodies.
use super::*;
use crate::catalog::{Catalog, Route};
use crate::{Intent, PlayerView};

/// The playback scrubber: arrow keys move five seconds.
const SCRUBBER: pxui::Scrubber = pxui::Scrubber::DEFAULT.step(5.0).name("Playback position");

/// The transport bar: three sibling views, so a playing song's tick rebuilds only the clocks and the scrubber (a view inside a kept
/// one cannot rebuild alone).
pub(super) fn transport(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    position: f64,
    panel: bool,
    intent: &mut Option<Intent>,
) {
    let narrow = ui.view_size().width < 1100.0;
    let width = ui.view_size().width.to_bits();
    let key_width = if narrow { 44.0 } else { 68.0 };
    let gap = if narrow { 4.0 } else { 16.0 };
    ui.row(Node::row().w(Size::Grow).h(Size::Grow).gap(gap).align(Align::Center).padding(Edges::ZERO.r(if narrow { 8.0 } else { 20.0 })).look(PANEL_FILL),
        |ui| {
            ui.row(Node::row().w_px(if narrow { 138.0 } else { 210.0 }).h_px(71.0).gap(2.0), |ui| { ui.view_of("transport-keys", &(player.playing, narrow), |ui| {
                ui.row(Node::row().w(Size::Grow).h(Size::Grow).gap(2.0), |ui| {
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
                ui.retain_view_on_resize();
            }); });
            ui.row(Node::row().w(Size::Grow).h(Size::Grow).align(Align::Center), |ui| {
                ui.view_of("transport-seek", &crate::hash((player.revision, ticks(player, position), player.duration.to_bits(), width)), |ui| {
                    ui.row(Node::row().w(Size::Grow).h(Size::Grow).gap(gap).align(Align::Center), |ui| {
                        let elapsed = position.max(0.0) as u64;
                        let remaining = (player.duration - position).max(0.0) as u64;
                        small_clock(ui, "elapsed", elapsed, false, 44.0, SECONDARY);
                        seek(ui, player, position, intent);
                        small_clock(ui, "remaining", remaining, true, 44.0, SECONDARY);
                    });
                });
            });
            // Swift's WaveModeKeys: shuffle, repeat and the panel toggle. Volume is ⌘↑/⌘↓ and the
            // queue sheet opens from the panel; the route key opens the system's audio outputs.
            // on Swift's `panel2` plate (the selected fill), 2 points round and between the keys
            let modes = (player.shuffled, player.repeat as u8, panel, player.route);
            ui.row(Node::row().w(Size::Fit).h(Size::Grow).align(Align::Center), |ui| { ui.view_of("transport-modes", &(modes, narrow), |ui| {
                ui.row(Node::row().w(Size::Fit).h(Size::Grow).gap(gap).align(Align::Center), |ui| {
                    ui.row(Node::row().w_px(if panel { 140.0 } else { 186.0 }).h_px(44.0).gap(2.0).pad(2.0).look(SELECTED_FILL),
                        |ui| {
                            if ui.icon_button("shuffle", icon_sized(Icon::Shuffle, 18.0), "Shuffle", &MODE_ICON.selected(player.shuffled)).clicked {
                                *intent = Some(Intent::Shuffle);
                            }
                            let repeat = if player.repeat == lw_core::queue::RepeatMode::One { icon_drawing(Icon::RepeatOne) } else { icon_sized(Icon::Repeat, 18.0) };
                            if ui.icon_button("repeat", repeat, REPEAT_NAMES[player.repeat as usize], &MODE_ICON.selected(player.repeat != lw_core::queue::RepeatMode::None)).clicked {
                                *intent = Some(Intent::Repeat);
                            }
                            if ui.icon_button("panel", icon_sized(Icon::Panel, 18.0), "Toggle now playing panel", &MODE_ICON.selected(panel)).clicked {
                                *intent = Some(Intent::TogglePanel);
                            }
                            // The queue sheet is otherwise opened from the panel, which may be hidden or collapsed.
                            if !panel
                                && ui.icon_button("queue", icon_drawing(Icon::Queue), "Show full queue", &MODE_ICON).clicked
                            {
                                *intent = Some(Intent::ShowQueue);
                            }
                        },
                    );
                    // Swift's audio route key after the keys (its shipping Mac app is the iOS build, which shows it too)
                    route_key(ui, player, intent);
                });
                ui.retain_view_on_resize();
            }); });
        },
    );
}

/// A playback mode key (shuffle, repeat, the panel, the queue): the key plate, inverse while on (`IconButton::selected`, which keeps
/// the checked look while hovered or held).
const MODE_ICON: pxui::IconButton = ICON_44X40.look(&MODE_LOOKS);
const MODE_LOOKS: States = ICON_PLATE_LOOKS.checked(ICON_INVERSE_LOOKS.idle);
/// The repeat key's name by `RepeatMode`.
pub(super) const REPEAT_NAMES: [&str; 3] = ["Repeat off", "Repeat all", "Repeat one"];
/// The up-next panel while a song is held over it: a 2-point accent frame.
const UP_NEXT_DROP: States = States::of(Look::NONE.border(Rgba::token(ColorToken::ACCENT), 2.0));

/// Swift's `WaveAudioRoute`: its 22-point route glyph on a 44-point key where the platform has the picker (iOS, macOS), else
/// the same frame empty.
pub(super) fn route_key(ui: &mut Ui, player: &PlayerView<'_>, intent: &mut Option<Intent>) {
    if !player.route {
        ui.node("route-slot", Node::column().w_px(44.0).h_px(44.0));
        return;
    }
    let key = ui.icon_button("route", icon_sized(Icon::Route, 22.0), "Audio output", &ICON_44.look(&ICON_LOOKS));
    if key.clicked {
        let at = ui.rect(&key.handle);
        *intent = Some(Intent::AudioRoute((at.x, at.y)));
    }
}

/// The playhead's steps across a song: what a playing song's views redraw for, under a point on any waveform.
const PLAYHEAD_STEPS: f64 = 2048.0;
/// What a playing position shows: the elapsed and remaining whole seconds (the clocks) and the playhead's step. Views of the playing
/// song key on these, not on the position, so they rebuild a few times a second instead of every frame.
pub(super) fn ticks(player: &PlayerView<'_>, position: f64) -> (u64, u64, u32) {
    let position = if position.is_finite() { position.max(0.0) } else { 0.0 };
    let step = if player.duration > 0.0 { ((position / player.duration).clamp(0.0, 1.0) * PLAYHEAD_STEPS) as u32 } else { 0 };
    (position as u64, (player.duration - position).max(0.0) as u64, step)
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
    let workspace_height = (height - top - 72.0).max(0.0);
    let panel = state.panel_visible
        && width >= 900.0
        && height - top >= 640.0;
    let sidebar_width = if width >= 900.0 { 231.0 } else { 180.0 };
    let content_width = (width - sidebar_width - 1.0 - if panel { 320.0 } else { 0.0 }).max(100.0);
    let root = ui.node(
        "shell",
        Node::column().w(Size::Grow).h(Size::Grow).padding(Edges::ZERO.l(safe_left).r(safe_right).b(safe_bottom)).look(PAPER_FILL),
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
        let work = container(ui, "workspace", Axis::Row, Size::Grow, Size::Px(workspace_height), &PAPER_FILL);
        ui.within(&work, &mut |ui| {
            // The sidebar's playlist targets track the pointer for every carry frame and clear after it.
            let carry = if ui.carrying().is_some() {
                crate::hash((1u8, ui.frame_index()))
            } else {
                u64::from(state.carried.get()) * 2
            };
            // the fit, not the height, keys the view: a resize that keeps it (a tall window's 1) keeps the sidebar
            let factor = super::sidebar::row_fit(workspace_height, super::sidebar::row_count(catalog));
            let revision = crate::hash((state.revision, catalog.all.is_some(), carry, factor.to_bits()));
            ui.column(Node::column().w_px(sidebar_width).h(Size::Grow), |ui| { ui.view_of("sidebar", &revision, |ui| sidebar(ui, catalog, state, workspace_height, factor, intent)); });
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
                            files::view(ui, player, state, intent);
                        } else {
                            content(ui, catalog, state, player, (content_width, workspace_height), intent);
                        }
                    });
                });
            ui.set_view(
                &page,
                &pxui::ViewTransform::DEFAULT.clip(Some(Rect {
                    x: safe_left + sidebar_width + 1.0,
                    y: top,
                    w: content_width,
                    h: workspace_height,
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
                let target = ui.node("up-next-drop", Node::column().w_px(319.0).h(Size::Grow));
                let drop = songs::song_drop(ui, &target);
                ui.within(&target, &mut |ui| {
                    // three sibling views: a tick rebuilds only the clock (a view inside a kept one cannot rebuild alone)
                    let artwork = player.artwork.map_or(0, crate::artwork::Images::revision);
                    let library = std::sync::Arc::as_ptr(&catalog.library) as usize;
                    let clip = pxui::ViewTransform::DEFAULT.clip(Some(Rect { x: safe_left + width - 319.0, y: top, w: 319.0, h: workspace_height }));
                    ui.column(Node::column().w(Size::Grow).h(Size::Grow).padding(Edges::new(20.0, 8.0, 20.0, 0.0)), |ui| {
                        let mut parts = [None; 2];
                        ui.column(Node::column().w(Size::Grow), |ui| { parts[0] = Some(ui.view_of("now-top", &(player.revision, artwork, library), |ui| {
                            ui.column(Node::column().w(Size::Grow), |ui| now_top(ui, player, catalog, intent));
                            ui.retain_view_on_resize();
                        })); });
                        // shortcut: the clock keeps its own entrance (10 points up), not the panel's 32-point slide; give it both if the entrance shows the gap
                        now_clock(ui, player, position);
                        ui.column(Node::column().w(Size::Grow).h(Size::Grow), |ui| { parts[1] = Some(ui.view_of("now-bottom", &(player.revision, artwork, catalog.queue.as_ref().map(|queue| queue.revision), library), |ui| {
                            ui.column(Node::column().w(Size::Grow).h(Size::Grow), |ui| now_bottom(ui, player, catalog, intent));
                        })); });
                        for part in parts.iter().flatten() {
                            ui.set_view(part, &clip);
                            crate::motion::enter(ui, part, 1, (32.0, 0.0), crate::motion::SLOW, 0.0);
                        }
                    });
                });
                match drop {
                    Some((true, song)) => *intent = Some(Intent::AppendSong(song)),
                    // the accent frame over the panel while a song is held over it
                    Some((false, _)) => ui.style(&target, &UP_NEXT_DROP, State::default()),
                    None => {}
                }
            }
        });
        rule(ui, "transport-divider", false);
        ui.column(Node::column().h_px(71.0), |ui| transport(ui, player, position, panel, intent));
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
    // the playhead at its step (`ticks`): the view keyed on that step draws what it shows
    let fraction = if let Some(preview) = scrub.preview {
        preview / upper
    } else {
        ticks(player, position).2 as f32 / PLAYHEAD_STEPS as f32
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
    let accent = Rgba::token(ColorToken::ACCENT);
    // Swift's WaveformView: 200 bars in the desktop transport, its default 96 in the phone player
    let bars: usize = if ui.view_size().width < compact::BREAKPOINT { 96 } else { 200 };
    ui.canvas(&scrubber, move |paint, rect| {
        let baseline = (rect.h - 4.0).max(0.0);
        let step = rect.w / bars as f32;
        let ink = Rgba::token(INK);
        if let Some(peaks) = &peaks {
            for index in 0..bars {
                let lower = index * peaks.len() / bars;
                let upper = ((index + 1) * peaks.len() / bars).max(lower + 1);
                let peak = peaks[lower..upper]
                    .iter()
                    .copied()
                    .fold(0.0, f32::max)
                    .clamp(0.0, 1.0);
                let height = (peak * baseline).max(2.0);
                paint.fill(Rect::new(rect.x + index as f32 * step, rect.y + baseline - height, step / 4.0, height), Fill::new(BAR_INK).fractional());
            }
        }
        paint.fill(Rect::new(rect.x, rect.y + baseline + 1.0, rect.w, 1.0), Fill::new(Rgba::token(LINE)));
        paint.fill(Rect::new(rect.x, rect.y + baseline, rect.w * fraction, 3.0), Fill::new(ink).fractional());
        paint.fill(Rect::new(rect.x + rect.w * fraction - step / 4.0, rect.y, step / 2.0, rect.h), Fill::new(accent).fractional());
    });
}
