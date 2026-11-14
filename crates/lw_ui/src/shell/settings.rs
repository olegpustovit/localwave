use super::*;
use crate::SettingsMenu;
use lw_core::preferences::{Appearance, Change, Signal};

fn back(ui: &mut Ui, name: &str, p: Palette) -> bool {
    let button = ui.node(
        name,
        Node {
            flags: CLICKABLE | FOCUSABLE,
            role: Role::Button,
            w: Size::Grow,
            h: Size::Px(44.0),
            look: Look {
                text: p.secondary,
                font: Font {
                    size: 15,
                    ..Font::BODY
                },
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.text(&button, name);
    ui.a11y_name(&button, name);
    ui.signal_state(&button).clicked
}
fn context(ui: &Ui, node: &Handle, menu: SettingsMenu, intent: &mut Option<Intent>) {
    let signal = ui.signal_state(node);
    if signal.pressed && signal.button == Some(1) {
        *intent = Some(Intent::SettingsMenu(menu, signal.pointer));
    }
}
pub(super) fn paragraph(
    ui: &mut Ui,
    key: &str,
    value: &str,
    width: f32,
    size: f32,
    color: Rgba,
) -> Handle {
    let style = pxui::TextStyle {
        size_points: size,
        line_height: size * 1.4,
        wrap: pxui::TextStyle::WRAP_WORD,
        ..Default::default()
    };
    let metrics = ui
        .text_service()
        .measure(
            Ui::key_of(Ui::key_of(0x5e771, key), value) as u64,
            value,
            &style,
            width.max(1.0),
        )
        .unwrap();
    let node = ui.node(
        key,
        Node {
            w: Size::Grow,
            h: Size::Px(metrics.height.max(style.line_height)),
            role: Role::Paragraph,
            look: Look {
                text: color,
                ..Look::NONE
            },
            ..Node::DEFAULT
        },
    );
    ui.text_spans(&node, value, &style, &[]).unwrap();
    node
}
fn row(
    ui: &mut Ui,
    key: &str,
    labels: (&str, &str),
    layout: (f32, Palette),
    control: (Option<(bool, Change)>, Option<SettingsMenu>),
    intent: &mut Option<Intent>,
    controls: &mut dyn FnMut(&mut Ui, &mut Option<Intent>),
) {
    let (title, detail) = labels;
    let (width, p) = layout;
    let (toggle, menu) = control;
    let stacked = width < 500.0;
    let label_width = (width
        - 32.0
        - if stacked {
            0.0
        } else if toggle.is_some() {
            56.0
        } else if menu == Some(SettingsMenu::Appearance) {
            212.0
        } else {
            182.0
        })
    .max(1.0);
    let label_style = pxui::TextStyle {
        size_points: 13.0,
        line_height: 18.2,
        wrap: pxui::TextStyle::WRAP_WORD,
        ..Default::default()
    };
    let detail_height = if detail.is_empty() {
        0.0
    } else {
        ui.text_service()
            .measure(
                Ui::key_of(Ui::key_of(0x5e772, key), detail) as u64,
                detail,
                &label_style,
                label_width,
            )
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
        key,
        Node {
            axis: if stacked { Axis::Column } else { Axis::Row },
            align: if stacked { Align::Start } else { Align::Center },
            flags: if toggle.is_some() {
                CLICKABLE | FOCUSABLE
            } else if menu.is_some() {
                CLICKABLE
            } else {
                0
            },
            role: if toggle.is_some() {
                Role::Toggle
            } else {
                Role::Group
            },
            w: Size::Grow,
            h: Size::Px(row_height),
            padding: Edges {
                l: 16.0,
                r: 16.0,
                t: 0.0,
                b: 0.0,
            },
            gap: 8.0,
            ..Node::DEFAULT
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
        let labels = ui.node(
            "labels",
            Node {
                w: Size::Grow,
                h: if stacked {
                    Size::Px(label_height)
                } else {
                    Size::Grow
                },
                justify: Justify::Center,
                ..Node::DEFAULT
            },
        );
        ui.within(&labels, &mut |ui| {
            text(ui, "title", title, 15, p.ink, (Size::Grow, 21.0), false);
            if !detail.is_empty() {
                let label = ui.node(
                    "detail",
                    Node {
                        w: Size::Grow,
                        h: Size::Px(detail_height),
                        role: Role::Label,
                        look: Look {
                            text: p.secondary,
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
                ui.text_spans(&label, detail, &label_style, &[]).unwrap();
            }
        });
        if let Some((on, _)) = toggle {
            let track = ui.node(
                "switch",
                Node {
                    w: Size::Px(48.0),
                    h: Size::Px(28.0),
                    look: Look {
                        fill: if on { p.ink } else { p.line },
                        ..Look::NONE
                    },
                    ..Node::DEFAULT
                },
            );
            ui.within(&track, &mut |ui| {
                ui.node(
                    "knob",
                    Node {
                        w: Size::Px(22.0),
                        h: Size::Px(22.0),
                        at: Some((if on { 23.0 } else { 3.0 }, 3.0)).into(),
                        look: Look {
                            fill: p.paper,
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
                    },
                );
            });
        } else {
            controls(ui, intent);
        }
    });
    if let Some((_, change)) = toggle {
        let signal = ui.signal_state(&node);
        if signal.clicked && signal.button != Some(1) {
            *intent = Some(Intent::Setting(change));
        }
    }
    ui.draw(&node, move |paint, rect| {
        paint.fill(
            rect.x + 16.0,
            rect.y + rect.h - 1.0,
            (rect.w - 16.0).max(0.0),
            1.0,
            p.line,
        )
    });
}
fn option(
    ui: &mut Ui,
    labels: (&str, &str),
    selected: bool,
    action: (Change, SettingsMenu),
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let (label, name) = labels;
    let (change, menu) = action;
    let button = ui.node(
        label,
        Node {
            flags: CLICKABLE | FOCUSABLE | FIT_TEXT,
            role: Role::Toggle,
            w: Size::Grow,
            h: Size::Px(28.0),
            min_w: 40.0,
            padding: Edges {
                l: 8.0,
                r: 8.0,
                ..Edges::ZERO
            },
            look: Look {
                fill: if selected { p.ink } else { p.panel },
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
    let signal = ui.signal_state(&button);
    if signal.clicked && signal.button != Some(1) {
        *intent = Some(Intent::Setting(change));
    }
}
pub(super) fn view(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    state: &ShellState,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let page = ui.node(
        "settings-page",
        Node {
            flags: SCROLL | CLIP,
            w: Size::Grow,
            h: Size::Grow,
            padding: Edges {
                t: 10.0,
                b: 24.0,
                ..Edges::ZERO
            },
            ..Node::DEFAULT
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
        if back(ui, if state.privacy_visible { "‹ Settings" } else { "‹ Library" }, p) {
            *intent = Some(if state.privacy_visible { Intent::ClosePrivacy } else { Intent::CloseSettings });
        }
        let heading = if state.privacy_visible { "About & Privacy" } else { "Settings" };
        let title = ui.node("settings-title", Node { flags: FIT_TEXT, role: Role::Heading, w: Size::Grow, h: Size::Px(56.0),
            inset: Edges { l: 16.0, r: 16.0, ..Edges::ZERO }, look: Look { text: p.ink,
                font: Font { size: 32, weight: 500, tracking: -1.12, ..Font::BODY }, ..Look::NONE }, ..Node::DEFAULT });
        ui.text(&title, heading);
        if state.privacy_visible {
            for (title, body) in [
                ("LocalWave", "A music player for your files."),
                ("Your library stays on this device", "LocalWave stores song metadata, artwork, playlists, Favorites, playback settings, folder bookmarks and cached audio locally. It does not send your music or listening history to the developer and includes no advertising or analytics service."),
                ("Files and iCloud Drive", "Files you select from iCloud Drive or another provider may need downloading through that provider. LocalWave does not receive your Apple Account password."),
                ("Removing your data", "Removing a library entry removes its database record; cached audio or artwork may remain. Removing LocalWave’s local app data erases its library and logs. Your original files stay where they are."),
            ] {
                let section = ui.node(title, Node { w: Size::Grow, h: Size::Fit, padding: Edges::all(16.0), gap: 8.0, ..Node::DEFAULT });
                ui.within(&section, &mut |ui| {
                    text(ui, "heading", title, 18, p.ink, (Size::Grow, 26.0), false);
                    paragraph(ui, title, body, width - 32.0, 14.0, p.secondary);
                });
            }
            if back(ui, "Full privacy policy", p) { *intent = Some(Intent::PrivacyPolicy); }
            if back(ui, "Support", p) { *intent = Some(Intent::Support); }
            if !player.message.is_empty() { text(ui, "link-error", player.message, 14, p.signal, (Size::Grow, 42.0), false); }
            return;
        }
        let settings = player.settings;
        row(ui, "download-ahead", ("Download ahead", "Fetch the next two cloud songs in the queue"), (width, p), (Some((settings.download_ahead, Change::ToggleDownloadAhead)), None), intent, &mut |_, _| {});
        row(ui, "no-network", ("Simulate no network", "Shows skipped songs and failed downloads"), (width, p), (Some((settings.simulate_no_network, Change::ToggleNoNetwork)), None), intent, &mut |_, _| {});
        let cache_detail = match settings.cache_gb { 2 => "2 GB limit · Oldest unpinned downloads go first", 32 => "32 GB limit · Oldest unpinned downloads go first", _ => "Oldest unpinned downloads go first" };
        row(ui, "cache", ("Offline space", cache_detail), (width, p), (None, Some(SettingsMenu::Cache)), intent, &mut |ui, intent| {
            let group = ui.node("segments", Node { axis: Axis::Row, w: Size::Px(174.0), h: Size::Px(32.0), padding: Edges::all(2.0), look: Look { fill: p.panel, ..Look::NONE }, ..Node::DEFAULT });
            ui.within(&group, &mut |ui| { for (value, label, name) in [(4, "4 GB", "Use 4 GB"), (8, "8 GB", "Use 8 GB"), (16, "16 GB", "Use 16 GB")] {
                option(ui, (label, name), settings.cache_gb == value, (Change::CacheGB(value), SettingsMenu::Cache), p, intent);
            }});
        });
        row(ui, "appearance", ("Appearance", ""), (width, p), (None, Some(SettingsMenu::Appearance)), intent, &mut |ui, intent| {
            let group = ui.node("segments", Node { axis: Axis::Row, w: Size::Px(204.0), h: Size::Px(32.0), padding: Edges::all(2.0), look: Look { fill: p.panel, ..Look::NONE }, ..Node::DEFAULT });
            ui.within(&group, &mut |ui| { for (value, label, name) in [(Appearance::System, "System", "Use system appearance"), (Appearance::Light, "Light", "Use light appearance"), (Appearance::Dark, "Dark", "Use dark appearance")] {
                option(ui, (label, name), settings.appearance == value, (Change::Appearance(value), SettingsMenu::Appearance), p, intent);
            }});
        });
        let footer = ui.node("settings-footer", Node { w: Size::Grow, h: Size::Fit, padding: Edges::all(16.0), gap: 8.0, ..Node::DEFAULT });
        ui.within(&footer, &mut |ui| {
            paragraph(ui, "storage-info", "LocalWave keeps folder bookmarks and caches songs as needed.", width - 32.0, 14.0, p.secondary);
            paragraph(ui, "local-info", "Individually added files are copied; library data stays on this device.", width - 32.0, 14.0, p.secondary);
            if back(ui, "About & Privacy", p) { *intent = Some(Intent::ShowPrivacy); }
            paragraph(ui, "cache-usage", if player.storage_busy || player.cache_usage.is_empty() { "Reading offline space…" } else { player.cache_usage }, width - 32.0, 14.0, p.secondary);
            if !player.settings_message.is_empty() { text(ui, "settings-message", player.settings_message, 14, p.signal, (Size::Grow, 42.0), false); }
            if !player.message.is_empty() { text(ui, "playback-message", player.message, 14, p.signal, (Size::Grow, 42.0), false); }
        });
    });
}
pub(super) fn popup(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    menu: SettingsMenu,
    at: (f32, f32),
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let rows = if menu == SettingsMenu::Cache { 2 } else { 5 };
    let (width, height) = ui.view_size();
    let at = (
        at.0.clamp(0.0, (width - 220.0).max(0.0)),
        at.1.clamp(0.0, (height - rows as f32 * 36.0).max(0.0)),
    );
    let open = ui.popup(
        Some(p.line),
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
                    label,
                    Node {
                        flags: CLICKABLE | FOCUSABLE,
                        role: Role::Toggle,
                        w: Size::Grow,
                        h: Size::Px(36.0),
                        padding: Edges {
                            l: 12.0,
                            r: 12.0,
                            ..Edges::ZERO
                        },
                        look: Look {
                            fill: if selected { p.selected } else { p.paper },
                            text: p.ink,
                            font: Font {
                                size: 13,
                                ..Font::BODY
                            },
                            ..Look::NONE
                        },
                        ..Node::DEFAULT
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
                if ui.signal_state(&node).clicked {
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
        frame: &FrameIn,
    ) -> Option<Intent> {
        static CATALOG: std::sync::LazyLock<crate::catalog::Catalog> =
            std::sync::LazyLock::new(|| {
                crate::catalog::Catalog::new(vec![lw_core::Song {
                    id: Some(7),
                    title: "Selected song".into(),
                    ..Default::default()
                }])
            });
        let mut intent = None;
        ui.frame(frame, &mut |ui| {
            crate::shell(ui, player, &CATALOG, state, &mut intent)
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
        let result = build(ui, state, player, frame);
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
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
            build(&mut ui, &state, &player, &frame);
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
            build(&mut ui, &state, &player, &frame);
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
            build(&mut ui, &state, &player, &frame);
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
            build(&mut ui, &state, &player, &frame);
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
        let mut ui = Ui::create(Platform::Apple, Some(15), &[crate::FONT, crate::MONO]);
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
                        player.position = player.revision as f64;
                        frame.time += 0.02;
                        let own = mem_sys::own(mem_sys::COUNT_LOCAL_ALLOCS);
                        let peer = pxui::library_counter(mem_sys::COUNT_LOCAL_ALLOCS);
                        build(&mut ui, &state, &player, &frame);
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
}
