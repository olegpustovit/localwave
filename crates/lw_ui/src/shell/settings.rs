//! Settings/About preserve route/sort/selection and exclude playback clock revisions from retention
//! keys. Use eight pre-parsed themes; never parse in frames. Dismiss custom popup acceptance after
//! Ui::popup returns. Command-comma uses scalar held-key state rather than allocating routed
//! vectors. Wrapped paragraph keys include content, including alternating busy/cache summaries.
//! Preserve overlapping menu presses and warm storage_busy alternation without extending warm-up.
//! Expose only connected actions.
//!
use super::*;
use crate::SettingsMenu;
use lw_core::preferences::{Appearance, Change, Signal};

fn back(ui: &mut Ui, name: &'static str) -> bool {
    let button = ui.node(
        name.into(),
        Node {
            look: Look {
                text: ui.color(SECONDARY),
                font: Font {
                    size: 15,
                    ..Font::BODY
                },
                ..Look::NONE
            },
            ..Node::column().w(Size::Grow).h_px(44.0).role(Role::Button).flags(Flags::CLICKABLE | Flags::FOCUSABLE)
        },
    );
    ui.text(&button, name);
    ui.a11y_name(&button, name);
    ui.signal(&button).clicked
}
fn context(ui: &Ui, node: &Handle, menu: SettingsMenu, intent: &mut Option<Intent>) {
    let signal = ui.signal(node);
    if signal.pressed && signal.button == Some(1) {
        *intent = Some(Intent::SettingsMenu(menu, signal.pointer));
    }
}
/// Running text in a node measured at `width`. Kept for the dialogs: a wrapped label re-lays the frame, which places a
/// modal at its final position one frame earlier than this fixed-height node does (a visible change).
pub(super) fn paragraph(
    ui: &mut Ui,
    key: &'static str,
    value: &str,
    width: f32,
    size: f32,
    color: Rgba,
) -> Handle {
    let label = Label::BODY.size(size).line_height(size * 1.4).wrap();
    let metrics = ui
        .text_service()
        .measure(value, &label, width.max(1.0))
        .unwrap();
    let node = ui.node(
        key.into(),
        Node {
            look: Look {
                text: color,
                ..Look::NONE
            },
            ..Node::column().w(Size::Grow).h_px(metrics.height.max(size * 1.4)).role(Role::Paragraph)
        },
    );
    ui.text_line_box(&node, value, &label, &[]).unwrap();
    node
}
/// A playback or settings message in the theme's accent, which is no colour token, so the label takes it as a look.
fn notice(ui: &mut Ui, text: &str) {
    let node = ui.label(text, TEXT_14.grow().h_px(42.0));
    let accent = ui.tokens().accent;
    ui.set_look(&node, Look { text: accent, font: Font { size: 14, ..Font::BODY }, ..Look::NONE }, State::default());
}
fn row(
    ui: &mut Ui,
    key: &'static str,
    labels: (&'static str, &str),
    width: f32,
    control: (Option<(bool, Change)>, Option<SettingsMenu>, bool),
    intent: &mut Option<Intent>,
    controls: &mut dyn FnMut(&mut Ui, &mut Option<Intent>),
) {
    let (title, detail) = labels;
    let (toggle, menu, tap) = control;
    // Swift keeps the control beside the label at every width; the label wraps instead.
    let stacked = false;
    let label_width = (width
        - 32.0
        - if stacked {
            0.0
        } else if toggle.is_some() {
            56.0
        } else if tap {
            26.0
        } else if menu == Some(SettingsMenu::Appearance) {
            212.0
        } else {
            182.0
        })
    .max(1.0);
    const DETAIL: Label = Label::BODY.size(13.0).line_height(18.2).wrap();
    let detail_height = if detail.is_empty() {
        0.0
    } else {
        ui.text_service()
            .measure(detail, &DETAIL, label_width)
            .unwrap()
            .height
    };
    let label_height = 21.0 + detail_height;
    let row_height = if stacked {
        92.0f32.max(label_height + 40.0)
    } else {
        60.0f32.max(label_height)
    };
    let node = ui.node(
        key.into(),
        Node {
            axis: if stacked { Axis::Column } else { Axis::Row },
            ..Node::column()
                .w(Size::Grow)
                .h_px(row_height)
                .gap(8.0)
                .pad_x(16.0)
                .align(if stacked { Align::Start } else { Align::Center })
                .role(if toggle.is_some() {
                    Role::Toggle
                } else if tap {
                    Role::Button
                } else {
                    Role::Group
                })
                .flags(if toggle.is_some() {
                    Flags::CLICKABLE | Flags::FOCUSABLE
                } else if menu.is_some() || tap {
                    Flags::CLICKABLE | Flags::FOCUSABLE
                } else {
                    Flags::NONE
                })
        },
    );
    ui.a11y_name(&node, title);
    if let Some((on, _)) = toggle {
        ui.set_role(
            &node,
            Role::Toggle,
            pxui::a11y::state::CHECKABLE | if on { pxui::a11y::state::CHECKED } else { 0 },
        );
    }
    if let Some(menu) = menu {
        context(ui, &node, menu, intent);
    }
    ui.within(&node, &mut |ui| {
        ui.column(
            "labels",
            Node {
                justify: Justify::Center,
                ..Node::column()
                    .w(Size::Grow)
                    .h(if stacked {
                        Size::Px(label_height)
                    } else {
                        Size::Grow
                    })
            },
            |ui| {
                ui.label(title, TEXT_15.grow().h_px(21.0).color(INK));
                if !detail.is_empty() {
                    let label = ui.node(
                        "detail".into(),
                        Node {
                            look: Look {
                                text: ui.color(SECONDARY),
                                ..Look::NONE
                            },
                            ..Node::column().w(Size::Grow).h_px(detail_height).role(Role::Label)
                        },
                    );
                    ui.text_line_box(&label, detail, &DETAIL, &[]).unwrap();
                }
            },
        );
        if let Some((on, _)) = toggle {
            ui.column(
                "switch",
                Node {
                    look: Look {
                        fill: if on { ui.color(INK) } else { ui.color(LINE) },
                        ..Look::NONE
                    },
                    ..Node::column().w_px(48.0).h_px(28.0)
                },
                |ui| {
                    ui.node(
                        "knob".into(),
                        Node {
                            at: Some((if on { 23.0 } else { 3.0 }, 3.0)).into(),
                            look: Look {
                                fill: ui.color(PAPER),
                                ..Look::NONE
                            },
                            ..Node::column().w_px(22.0).h_px(22.0)
                        },
                    );
                },
            );
        } else {
            controls(ui, intent);
        }
    });
    if let Some((_, change)) = toggle {
        let signal = ui.signal(&node);
        if signal.clicked && signal.button != Some(1) {
            *intent = Some(Intent::Setting(change));
        }
    }
    if tap && ui.signal(&node).clicked {
        *intent = Some(Intent::Files(crate::files::Request::Roots));
    }
    ui.draw(&node, move |paint, rect| {
        paint.fill(
            rect.x + 16.0,
            rect.y + rect.h - 1.0,
            (rect.w - 16.0).max(0.0),
            1.0,
            paint.color(LINE),
        )
    });
}
fn option(
    ui: &mut Ui,
    labels: (&'static str, &str),
    selected: bool,
    action: (Change, SettingsMenu),
    intent: &mut Option<Intent>,
) {
    let (label, name) = labels;
    let (change, menu) = action;
    let button = ui.node(
        label.into(),
        Node {
            min_w: 40.0,
            look: Look {
                fill: if selected { ui.color(INK) } else { ui.color(PANEL) },
                text: if selected { ui.color(PAPER) } else { ui.color(SECONDARY) },
                font: Font {
                    size: 13,
                    ..Font::BODY
                },
                ..Look::NONE
            },
            ..Node::column()
                .w(Size::Grow)
                .h_px(28.0)
                .pad_x(8.0)
                .role(Role::Toggle)
                .flags(Flags::CLICKABLE | Flags::FOCUSABLE | Flags::FIT_TEXT)
        },
    );
    ui.text(&button, label);
    ui.a11y_name(&button, name);
    ui.set_role(
        &button,
        Role::Toggle,
        pxui::a11y::state::CHECKABLE
            | if selected {
                pxui::a11y::state::CHECKED
            } else {
                0
            },
    );
    context(ui, &button, menu, intent);
    let signal = ui.signal(&button);
    if signal.clicked && signal.button != Some(1) {
        *intent = Some(Intent::Setting(change));
    }
}
pub(super) fn view(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    state: &ShellState,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let page = ui.node(
        "settings-page".into(),
        Node {
            padding: Edges {
                t: 10.0,
                b: 24.0,
                ..Edges::ZERO
            },
            ..Node::column().w(Size::Grow).h(Size::Grow).flags(Flags::SCROLL | Flags::CLIP)
        },
    );
    ui.set_role(&page, Role::Group, 0);
    ui.a11y_name(
        &page,
        if state.privacy_visible {
            "About and Privacy"
        } else {
            "Settings"
        },
    );
    ui.within(&page, &mut |ui| {
        if back(ui, if state.privacy_visible { "‹ Settings" } else { "‹ Library" }) {
            *intent = Some(if state.privacy_visible { Intent::ClosePrivacy } else { Intent::CloseSettings });
        }
        let heading = if state.privacy_visible { "About & Privacy" } else { "Settings" };
        let title = ui.node("settings-title".into(), Node::column().w(Size::Grow).h_px(56.0).pad_x(16.0).role(Role::Heading));
        ui.within(&title, &mut |ui| { detail::line(ui, "heading", heading, (32.0, 33.6, 500, -0.035), ui.color(INK), Flags::NONE); });
        ui.a11y_name(&title, heading);
        if state.privacy_visible {
            for (title, body) in [
                ("LocalWave", "A music player for your files."),
                ("Your library stays on this device", "LocalWave stores song metadata, artwork, playlists, Favorites, playback settings, folder bookmarks and cached audio locally. It does not send your music or listening history to the developer and includes no advertising or analytics service."),
                ("Files and iCloud Drive", "Files you select from iCloud Drive or another provider may need downloading through that provider. LocalWave does not receive your Apple Account password."),
                ("Removing your data", "Removing a library entry removes its database record; cached audio or artwork may remain. Removing LocalWave’s local app data erases its library and logs. Your original files stay where they are."),
            ] {
                ui.column(title, Node::column().w(Size::Grow).h(Size::Fit).gap(8.0).pad(16.0), |ui| {
                    ui.label(title, TEXT_18.grow().h_px(26.0).color(INK));
                    ui.label(body, PARAGRAPH_14.color(SECONDARY));
                });
            }
            if back(ui, "Full privacy policy") { *intent = Some(Intent::PrivacyPolicy); }
            if back(ui, "Support") { *intent = Some(Intent::Support); }
            if !player.message.is_empty() { notice(ui, player.message); }
            return;
        }
        let settings = player.settings;
        row(ui, "download-ahead", ("Download ahead", "Fetch the next two cloud songs in the queue"), width, (Some((settings.download_ahead, Change::ToggleDownloadAhead)), None, false), intent, &mut |_, _| {});
        // Swift shows this on phones only: desktop folder tables always list every file type.
        let compact = width < compact::BREAKPOINT;
        if compact {
            row(ui, "show-non-audio", ("Show non-audio files", "Covers, logs and text in folders"), width, (Some((settings.show_non_audio, Change::ToggleShowNonAudio)), None, false), intent, &mut |_, _| {});
        }
        row(ui, "no-network", ("Simulate no network", "Prototype only — shows skipped songs and failed downloads"), width, (Some((settings.simulate_no_network, Change::ToggleNoNetwork)), None, false), intent, &mut |_, _| {});
        let cache_detail = match settings.cache_gb { 2 => "2 GB limit · Oldest unpinned downloads go first", 32 => "32 GB limit · Oldest unpinned downloads go first", _ => "Oldest unpinned downloads go first" };
        row(ui, "cache", ("Offline space", cache_detail), width, (None, Some(SettingsMenu::Cache), false), intent, &mut |ui, intent| {
            ui.row("segments", Node { look: Look { fill: ui.color(PANEL), ..Look::NONE }, ..Node::row().w_px(174.0).h_px(32.0).pad(2.0) }, |ui| { for (value, label, name) in [(4, "4 GB", "Use 4 GB"), (8, "8 GB", "Use 8 GB"), (16, "16 GB", "Use 16 GB")] {
                option(ui, (label, name), settings.cache_gb == value, (Change::CacheGB(value), SettingsMenu::Cache), intent);
            }});
        });
        row(ui, "appearance", ("Appearance", ""), width, (None, Some(SettingsMenu::Appearance), false), intent, &mut |ui, intent| {
            ui.row("segments", Node { look: Look { fill: ui.color(PANEL), ..Look::NONE }, ..Node::row().w_px(204.0).h_px(32.0).pad(2.0) }, |ui| { for (value, label, name) in [(Appearance::System, "System", "Use system appearance"), (Appearance::Light, "Light", "Use light appearance"), (Appearance::Dark, "Dark", "Use dark appearance")] {
                option(ui, (label, name), settings.appearance == value, (Change::Appearance(value), SettingsMenu::Appearance), intent);
            }});
        });
        row(ui, "rescan", ("Rescan all sources", "Usually not needed — folders are watched"), width, (None, None, true), intent, &mut |ui, _| {
            let glyph = ui.node("chevron".into(), Node::column().w_px(18.0).h_px(18.0));
            let color = ui.color(INK);
            ui.draw(&glyph, move |paint, rect| {
                for step in 0..6 {
                    let step = step as f32 * 1.5;
                    paint.fill(rect.x + 5.0 + step, rect.y + 3.0 + step, 1.5, 1.5, color);
                    paint.fill(rect.x + 5.0 + step, rect.y + 14.0 - step, 1.5, 1.5, color);
                }
            });
        });
        ui.column("settings-footer", Node::column().w(Size::Grow).h(Size::Fit).gap(8.0).pad(16.0), |ui| {
            ui.label("LocalWave keeps folder bookmarks and caches songs as needed. Individually added files are copied; library data stays on this device.", PARAGRAPH_14.color(SECONDARY));
            if back(ui, "About & Privacy") { *intent = Some(Intent::ShowPrivacy); }
            ui.label(if player.storage_busy || player.cache_usage.is_empty() { "Reading offline space…" } else { player.cache_usage }, PARAGRAPH_14.color(SECONDARY));
            if !player.settings_message.is_empty() { notice(ui, player.settings_message); }
            if !player.message.is_empty() { notice(ui, player.message); }
        });
    });
}
pub(super) fn popup(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    menu: SettingsMenu,
    at: (f32, f32),
    intent: &mut Option<Intent>,
) {
    let rows = if menu == SettingsMenu::Cache { 2 } else { 5 };
    let (width, height) = ui.view_size();
    let at = (
        at.0.clamp(0.0, (width - 220.0).max(0.0)),
        at.1.clamp(0.0, (height - rows as f32 * 36.0).max(0.0)),
    );
    let open = ui.popup(
        Some(ui.color(LINE)),
        None,
        false,
        false,
        at,
        220.0,
        rows as f32 * 36.0,
        "Settings options",
        &mut |ui| {
            let items: &[(Change, &str, bool)] = if menu == SettingsMenu::Cache {
                &[
                    (
                        Change::CacheGB(2),
                        "Use 2 GB",
                        player.settings.cache_gb == 2,
                    ),
                    (
                        Change::CacheGB(32),
                        "Use 32 GB",
                        player.settings.cache_gb == 32,
                    ),
                ]
            } else {
                &[
                    (
                        Change::Signal(Signal::Blue),
                        "Use blue signal",
                        player.settings.signal == Signal::Blue,
                    ),
                    (
                        Change::Signal(Signal::Red),
                        "Use red signal",
                        player.settings.signal == Signal::Red,
                    ),
                    (
                        Change::Signal(Signal::Green),
                        "Use green signal",
                        player.settings.signal == Signal::Green,
                    ),
                    (
                        Change::Signal(Signal::Mono),
                        "Use monochrome signal",
                        player.settings.signal == Signal::Mono,
                    ),
                    (
                        Change::ToggleReduceMotion,
                        "Reduce motion",
                        player.settings.reduce_motion,
                    ),
                ]
            };
            for &(change, label, selected) in items {
                let node = ui.node(
                    label.into(),
                    Node {
                        look: Look {
                            fill: if selected { ui.color(SELECTED) } else { ui.color(PAPER) },
                            text: ui.color(INK),
                            font: Font {
                                size: 13,
                                ..Font::BODY
                            },
                            ..Look::NONE
                        },
                        ..Node::column()
                            .w(Size::Grow)
                            .h_px(36.0)
                            .pad_x(12.0)
                            .role(Role::Toggle)
                            .flags(Flags::CLICKABLE | Flags::FOCUSABLE)
                    },
                );
                ui.text(&node, label);
                ui.set_role(
                    &node,
                    Role::Toggle,
                    pxui::a11y::state::CHECKABLE
                        | if selected {
                            pxui::a11y::state::CHECKED
                        } else {
                            0
                        },
                );
                if ui.signal(&node).clicked {
                    *intent = Some(Intent::Setting(change));
                }
            }
        },
    );
    if matches!(intent, Some(Intent::Setting(_))) {
        ui.dismiss_popup();
    }
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseSettingsMenu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::{Event, FrameIn, Platform};

    fn build(
        ui: &mut Ui,
        state: &ShellState,
        player: &PlayerView<'_>,
        position: f64,
        frame: &FrameIn,
    ) -> Option<Intent> {
        static CATALOG: std::sync::LazyLock<crate::catalog::Catalog> =
            std::sync::LazyLock::new(|| {
                crate::tests::prepared(vec![lw_core::Song {
                    id: Some(7),
                    title: "Selected song".into(),
                    ..Default::default()
                }])
            });
        let mut intent = None;
        ui.frame(frame, &mut |ui| {
            crate::shell(ui, player, position, &CATALOG, state, &mut intent)
        })
        .unwrap();
        intent
    }
    fn point(ui: &Ui, name: &str) -> (f64, f64) {
        let rect = ui
            .a11y_nodes()
            .iter()
            .find(|node| {
                ui.a11y_name_of(node) == name
                    && (node.is_role(Role::Button) || node.is_role(Role::Toggle))
            })
            .unwrap_or_else(|| panic!("missing {name}"))
            .rect;
        (
            f64::from(rect.x + rect.w * 0.5),
            f64::from(rect.y + rect.h * 0.5),
        )
    }
    fn send(
        ui: &mut Ui,
        state: &ShellState,
        player: &PlayerView<'_>,
        frame: &mut FrameIn,
        event: Event,
    ) -> Option<Intent> {
        frame.events = &event;
        frame.event_count = 1;
        frame.time += 0.02;
        let result = build(ui, state, player, 0.0, frame);
        frame.events = std::ptr::null();
        frame.event_count = 0;
        result
    }
    fn click(
        ui: &mut Ui,
        state: &ShellState,
        player: &PlayerView<'_>,
        frame: &mut FrameIn,
        at: (f64, f64),
        button: u32,
    ) -> Option<Intent> {
        let mut result = None;
        for kind in [
            input::wire::POINTER_MOVED,
            input::wire::POINTER_PRESSED,
            input::wire::POINTER_RELEASED,
        ] {
            let next = send(
                ui,
                state,
                player,
                frame,
                Event {
                    kind,
                    code: button,
                    x: at.0,
                    y: at.1,
                    ..Event::default()
                },
            );
            if next.is_some() {
                result = next;
            }
        }
        result
    }
    #[test]
    fn settings_navigation_controls_context_choices_and_privacy_keep_library_selection() {
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = ShellState {
            selected: Some(0),
            ..Default::default()
        };
        state.apply(Intent::ShowSettings);
        let mut player = PlayerView {
            initialized: true,
            cache_usage: "0 bytes cached. Kept songs may exceed the limit; Added Files are never removed.",
            ..Default::default()
        };
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..Default::default()
        };
        for _ in 0..3 {
            build(&mut ui, &state, &player, 0.0, &frame);
        }
        let at = point(&ui, "Download ahead");
        assert_eq!(
            click(&mut ui, &state, &player, &mut frame, at, 0),
            Some(Intent::Setting(Change::ToggleDownloadAhead))
        );
        player.settings = player
            .settings
            .applying(Change::ToggleDownloadAhead)
            .unwrap();
        let key = Event {
            kind: input::wire::KEY_PRESSED,
            code: input::Key::Enter.code(),
            ..Event::default()
        };
        assert_eq!(
            send(&mut ui, &state, &player, &mut frame, key),
            Some(Intent::Setting(Change::ToggleDownloadAhead)),
            "Enter must activate the focused setting, without playing the selected song"
        );
        send(
            &mut ui,
            &state,
            &player,
            &mut frame,
            Event {
                kind: input::wire::KEY_RELEASED,
                code: input::Key::Enter.code(),
                ..Event::default()
            },
        );
        let at = point(&ui, "Use 16 GB");
        assert_eq!(
            click(&mut ui, &state, &player, &mut frame, at, 0),
            Some(Intent::Setting(Change::CacheGB(16)))
        );
        let at = point(&ui, "Use system appearance");
        let intent = click(&mut ui, &state, &player, &mut frame, at, 1).unwrap();
        assert!(matches!(
            intent,
            Intent::SettingsMenu(SettingsMenu::Appearance, _)
        ));
        state.apply(intent);
        for _ in 0..3 {
            build(&mut ui, &state, &player, 0.0, &frame);
        }
        let at = point(&ui, "Use red signal");
        let intent = click(&mut ui, &state, &player, &mut frame, at, 0).unwrap();
        assert_eq!(intent, Intent::Setting(Change::Signal(Signal::Red)));
        assert!(!state.apply(intent));
        assert!(state.settings_menu.is_none());
        let at = point(&ui, "Use 8 GB");
        let next_menu = send(
            &mut ui,
            &state,
            &player,
            &mut frame,
            Event {
                kind: input::wire::POINTER_PRESSED,
                code: 1,
                x: at.0,
                y: at.1,
                ..Event::default()
            },
        );
        assert!(
            matches!(
                next_menu,
                Some(Intent::SettingsMenu(SettingsMenu::Cache, _))
            ),
            "the next press after accepting a menu must reach Settings"
        );
        state.apply(next_menu.unwrap());
        send(
            &mut ui,
            &state,
            &player,
            &mut frame,
            Event {
                kind: input::wire::POINTER_RELEASED,
                code: 1,
                x: at.0,
                y: at.1,
                ..Event::default()
            },
        );
        for _ in 0..3 {
            build(&mut ui, &state, &player, 0.0, &frame);
        }
        let at = point(&ui, "Use 32 GB");
        assert_eq!(
            click(&mut ui, &state, &player, &mut frame, at, 0),
            Some(Intent::Setting(Change::CacheGB(32)))
        );
        state.apply(Intent::CloseSettingsMenu);
        state.apply(Intent::ShowPrivacy);
        frame.width = 640.0;
        let start = frame.time;
        for offset in [0.0, 0.5, 1.0] {
            frame.time = start + offset;
            build(&mut ui, &state, &player, 0.0, &frame);
        }
        let paragraph = ui
            .a11y_nodes()
            .iter()
            .find(|node| {
                ui.a11y_name_of(node)
                    .starts_with("LocalWave stores song metadata")
            })
            .unwrap();
        assert!(
            paragraph.rect.h > 40.0,
            "privacy text must wrap: {:?}",
            paragraph.rect
        );
        assert!(paragraph.rect.x >= 180.0 && paragraph.rect.x + paragraph.rect.w <= 640.1);
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Full privacy policy")
        );
        assert!(
            ui.a11y_nodes()
                .iter()
                .any(|node| ui.a11y_name_of(node) == "Support")
        );
        state.apply(Intent::ClosePrivacy);
        state.apply(Intent::CloseSettings);
        assert_eq!(state.selected, Some(0));
        assert_eq!(state.route, Route::Songs);
        send(
            &mut ui,
            &state,
            &player,
            &mut frame,
            Event {
                kind: input::wire::KEY_PRESSED,
                code: input::Key::Meta.code(),
                ..Event::default()
            },
        );
        assert_eq!(
            send(
                &mut ui,
                &state,
                &player,
                &mut frame,
                Event {
                    kind: input::wire::KEY_PRESSED,
                    code: ',' as u32,
                    ..Event::default()
                }
            ),
            Some(Intent::ShowSettings)
        );
        state.apply(Intent::SettingsMenu(SettingsMenu::Cache, (f32::NAN, 0.0)));
        assert!(state.settings_menu.is_none());
    }
    #[test]
    fn settings_and_context_menus_reuse_warm_views_and_allocate_nothing() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        let mut state = ShellState::default();
        state.apply(Intent::ShowSettings);
        let mut player = PlayerView {
            initialized: true,
            cache_usage: "0 bytes cached. Kept songs may exceed the limit; Added Files are never removed.",
            ..Default::default()
        };
        let mut frame = FrameIn {
            width: 1440.0,
            height: 872.0,
            ..Default::default()
        };
        for round in 0..8 {
            mem_sys::local_reset();
            let mut trace = [(0u64, 0u64); 18];
            let mut index = 0usize;
            for cache_gb in [2, 32] {
                player.settings.cache_gb = cache_gb;
                for menu in [
                    None,
                    Some(SettingsMenu::Cache),
                    Some(SettingsMenu::Appearance),
                ] {
                    state.settings_menu = menu.map(|menu| (menu, (500.0, 200.0)));
                    state.revision += 1;
                    for _ in 0..3 {
                        player.storage_busy = index.is_multiple_of(2);
                        player.revision += 1;
                        let position = player.revision as f64;
                        frame.time += 0.02;
                        let own = mem_sys::own(mem_sys::COUNT_LOCAL_ALLOCS);
                        let peer = pxui::library_counter(mem_sys::COUNT_LOCAL_ALLOCS);
                        build(&mut ui, &state, &player, position, &frame);
                        trace[index] = (
                            mem_sys::own(mem_sys::COUNT_LOCAL_ALLOCS) - own,
                            pxui::library_counter(mem_sys::COUNT_LOCAL_ALLOCS) - peer,
                        );
                        index += 1;
                    }
                }
            }
            if round >= 3 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "settings/menu warm rebuild round {round}: own/peer={trace:?}"
                );
            }
        }
    }
    #[test]
    fn compact_settings_add_phone_rows_and_keep_controls_inline_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        state.apply(Intent::ShowSettings);
        let player = PlayerView { initialized: true, cache_usage: "6.6 MB cached.", ..Default::default() };
        let mut frame = FrameIn { width: 390.0, height: 778.0, ..Default::default() };
        for _ in 0..3 {
            build(&mut ui, &state, &player, 0.0, &frame);
        }
        let at = point(&ui, "Show non-audio files");
        assert_eq!(click(&mut ui, &state, &player, &mut frame, at, 0), Some(Intent::Setting(Change::ToggleShowNonAudio)));
        let at = point(&ui, "Rescan all sources");
        assert_eq!(click(&mut ui, &state, &player, &mut frame, at, 0), Some(Intent::Files(crate::files::Request::Roots)));
        // The 8 GB segment sits beside its label, not under it.
        let label = ui.a11y_nodes().iter().find(|n| ui.a11y_name_of(n) == "Offline space").unwrap().rect;
        let segment = ui.a11y_nodes().iter().find(|n| ui.a11y_name_of(n) == "Use 8 GB").unwrap().rect;
        assert!(segment.x >= label.x + label.w * 0.4 && segment.y < label.y + label.h);
        for round in 0..5 {
            mem_sys::local_reset();
            for _ in 0..10 {
                build(&mut ui, &state, &player, 0.0, &frame);
            }
            if round >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm compact settings allocate");
            }
        }
    }

    #[test]
    fn desktop_settings_keep_rescan_and_hide_the_phone_only_row() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = crate::test_ui(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
        ui.set_a11y(true);
        let mut state = ShellState::default();
        state.apply(Intent::ShowSettings);
        let player = PlayerView { initialized: true, ..Default::default() };
        let mut frame = FrameIn { width: 1440.0, height: 872.0, ..Default::default() };
        for _ in 0..3 {
            build(&mut ui, &state, &player, 0.0, &frame);
        }
        assert!(!ui.a11y_nodes().iter().any(|n| ui.a11y_name_of(n) == "Show non-audio files"));
        let at = point(&ui, "Rescan all sources");
        assert_eq!(click(&mut ui, &state, &player, &mut frame, at, 0), Some(Intent::Files(crate::files::Request::Roots)));
    }
}
