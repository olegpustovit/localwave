//! Settings/About preserve route/sort/selection and exclude playback clock revisions from retention
//! keys. Use eight pre-parsed themes; never parse in frames. The choice menus are `ui.menu` rows drawn by the app body.
//! Command-comma uses scalar held-key state rather than allocating routed
//! vectors. Wrapped paragraph keys include content, including alternating busy/cache summaries.
//! Preserve overlapping menu presses and warm storage_busy alternation without extending warm-up.
//! Expose only connected actions.
//!
use pxui::bodies::BodyConfig;
use super::*;
use crate::SettingsMenu;
use lw_core::preferences::{Appearance, Change, Signal};

/// A settings link (‹ Library, Support…; `localwave-button` / `ROW`): secondary 15-point text across the page from its start, 44 points high.
const SETTINGS_LINK: pxui::Button = pxui::Button::DEFAULT.variant(variant::ROW).size(Size::Grow, Size::Px(44.0)).align(Justify::Start).pad(Edges::new(0.0, 4.0, 0.0, 0.0)).label(TEXT_15.color(SECONDARY).role(Role::Unknown));
/// A secondary press on a settings row or its chips (`secondary`) opens the row's context menu where the pointer is.
fn context(ui: &Ui, secondary: bool, node: &Handle, menu: SettingsMenu, intent: &mut Option<Intent>) {
    if secondary {
        *intent = Some(Intent::SettingsMenu(menu, ui.signal(node).pointer));
    }
}
/// A settings row's name, context menu and hairline.
fn setting_row(ui: &mut Ui, row: &pxui::Response, title: &str, menu: SettingsMenu, intent: &mut Option<Intent>) {
    ui.a11y_name(&row.handle, title);
    context(ui, row.secondary, &row.handle, menu, intent);
    hairline(ui, &row.handle);
}
/// Running text in a node measured at `width`. Kept for the dialogs: a wrapped label re-lays the frame, which places a
/// modal at its final position one frame earlier than this fixed-height node does (a visible change).
pub(super) fn paragraph(ui: &mut Ui, key: &'static str, value: &str, width: f32, style: Label) -> Handle {
    let metrics = ui.text_service().measure(value, style, Some(width.max(1.0))).unwrap();
    let line = style.fields().line_height.unwrap_or(0.0);
    let node = ui.node(key, Node::column().w(Size::Grow).h_px(metrics.height.max(line)).role(Role::Paragraph).look(text_look(style)));
    ui.text_line_box(&node, value, style, &[]).unwrap();
    node
}
/// A playback or settings message: 14 points in the theme's accent, 42 points high.
const NOTICE: Label = TEXT_14.grow().h_px(42.0).color(ColorToken::ACCENT);
/// Swift's settings row: controls beside the labels at every width (the labels wrap), at least 60 tall, a hairline under it.
const ROW: Node =
    Node::column().w(Size::Grow).h(Size::Fit).gap(8.0).pad_x(16.0).align(Align::Center).axis(Axis::Row).min_h(60.0);
const DETAIL: Label = Label::BODY.size(13.0).line_height(18.2).wrap().w(Size::Grow).role(Role::Label).color(SECONDARY);
fn labels(ui: &mut Ui, title: &str, detail: &str) {
    ui.column(Node::column().w(Size::Grow).h(Size::Fit).justify(Justify::Center), |ui| {
        ui.label(title, TEXT_15.grow().h_px(21.0).color(INK));
        if !detail.is_empty() {
            ui.label(detail, DETAIL);
        }
    });
}
/// The row's hairline, from its title's start to its end.
fn hairline(ui: &mut Ui, node: &Handle) {
    ui.canvas(node, move |paint, rect| paint.fill(Rect::new(rect.x + 16.0, rect.y + rect.h - 1.0, (rect.w - 16.0).max(0.0), 1.0), Fill::new(Rgba::token(LINE))));
}
/// LocalWave's settings switch row (registered as `localwave-toggle`): a settings row whose control is the 48 x 28 switch, ink when
/// on, the line colour when off, with a 22-point paper knob.
pub(crate) fn toggle_body(ui: &mut Ui, c: &pxui::bodies::ToggleConfig) {
    // The switch shows the stored value until the controller applies the flip.
    let (on, title) = (c.stored != 0, c.text_of(&c.label));
    ui.set_node_layout(&c.row, &ROW.min_h(c.row_h));
    ui.a11y_name(&c.row, title);
    ui.set_role(&c.row, Role::Toggle, pxui::A11yStates::CHECKABLE.with(if on { pxui::A11yStates::CHECKED } else { pxui::A11yStates::NONE }));
    labels(ui, title, c.text_of(&c.detail));
    let track = if on { TRACK_ON } else { TRACK };
    ui.column(Node::column().w_px(48.0).h_px(28.0).look(track), |ui| {
        ui.node("knob", Node::column().w_px(22.0).h_px(22.0).at(if on { 23.0 } else { 3.0 }, 3.0).look(KNOB));
    });
    hairline(ui, &c.row);
}
// The switch: an ink track when on, the line colour when off, a paper knob.
const TRACK_ON: Look = INK_FILL;
const TRACK: Look = LINE_FILL;
const KNOB: Look = PAPER_FILL;
/// A settings switch row: the whole row flips it.
const SWITCH: pxui::Toggle<'static> = pxui::Toggle::DEFAULT.row_h(60.0).press_row(true);
/// The settings chips (`localwave-segmented` / `CHIPS`): 32 high, 2 points around the chips, a press focuses the chip pressed.
const CHIPS: pxui::Segmented = pxui::Segmented::DEFAULT.h_px(32.0).padding(2.0).variant(variant::CHIPS).focus_on_click(true);
// The chips; a cache chip's id is its size in GB, an appearance chip's the `Appearance`.
const CACHE_SEGMENTS: [Segment; 3] = [Segment::new("4 GB").id(4).name("Use 4 GB"), Segment::new("8 GB").id(8).name("Use 8 GB"), Segment::new("16 GB").id(16).name("Use 16 GB")];
const APPEARANCES: [Appearance; 3] = [Appearance::System, Appearance::Light, Appearance::Dark];
const APPEARANCE_SEGMENTS: [Segment; 3] = [
    Segment::new("System").id(Appearance::System as i64).name("Use system appearance"),
    Segment::new("Light").id(Appearance::Light as i64).name("Use light appearance"),
    Segment::new("Dark").id(Appearance::Dark as i64).name("Use dark appearance"),
];
/// The settings chips `items` (`selected` the stored choice): a primary press on a chip is its `change`, a secondary press opens `menu`.
#[allow(clippy::too_many_arguments)]
fn chips(ui: &mut Ui, key: &'static str, items: &[Segment], selected: Option<usize>, w: f32, change: impl Fn(i64) -> Change, menu: SettingsMenu, intent: &mut Option<Intent>) {
    let r = ui.segmented(key, items, selected, &CHIPS.w_px(w));
    if (r.response.changed || r.response.clicked) && let Some(id) = r.chosen_id {
        *intent = Some(Intent::Setting(change(id)));
    }
    context(ui, r.secondary.is_some(), &r.response.handle, menu, intent);
}
/// The rescan row (`localwave-button` / `SETTING`): a press focuses it, as Swift's rows take the focus.
const RESCAN: pxui::Button = pxui::Button::DEFAULT.variant(variant::SETTING).detail("Usually not needed — folders are watched").focus_on_click(true);
/// LocalWave's settings row button (`localwave-button` / `SETTING`): the settings row with the title and detail, the dotted chevron at
/// its end and the hairline under it.
pub(super) fn row_body(ui: &mut Ui, c: &pxui::bodies::ButtonConfig) {
    ui.set_node_layout(&c.line, &Node::row().w(Size::Grow).h(Size::Fit));
    ui.set_node_layout(&c.row, &ROW);
    labels(ui, c.text_of(&c.label), c.text_of(&c.detail));
    let glyph = ui.node("chevron", Node::column().w_px(18.0).h_px(18.0));
    let color = Rgba::token(INK);
    ui.canvas(&glyph, move |paint, rect| {
        for step in 0..6 {
            let step = step as f32 * 1.5;
            paint.fill(Rect::new(rect.x + 5.0 + step, rect.y + 3.0 + step, 1.5, 1.5), Fill::new(color));
            paint.fill(Rect::new(rect.x + 5.0 + step, rect.y + 14.0 - step, 1.5, 1.5), Fill::new(color));
        }
    });
    hairline(ui, &c.row);
}
/// A settings row: a press focuses it, as Swift's rows take the focus.
const SETTING: pxui::Pressable<'static> = PLAIN.focus_on_click(true);
pub(super) fn view(
    ui: &mut Ui,
    player: &PlayerView<'_>,
    state: &ShellState,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let page = ui.node(
        "settings-page",
        Node::column().w(Size::Grow).h(Size::Grow).flags(Flags::SCROLL | Flags::CLIP).padding(Edges::ZERO.t(10.0).b(24.0)),
    );
    ui.set_role(&page, Role::Group, pxui::A11yStates::NONE);
    ui.a11y_name(
        &page,
        if state.privacy_visible {
            "About and Privacy"
        } else {
            "Settings"
        },
    );
    ui.within(&page, &mut |ui| {
        if ui.button("back", if state.privacy_visible { "‹ Settings" } else { "‹ Library" }, &SETTINGS_LINK).clicked {
            *intent = Some(if state.privacy_visible { Intent::ClosePrivacy } else { Intent::CloseSettings });
        }
        let heading = if state.privacy_visible { "About & Privacy" } else { "Settings" };
        let title = ui.node("settings-title", Node::column().w(Size::Grow).h_px(56.0).pad_x(16.0).role(Role::Heading));
        ui.within(&title, &mut |ui| { ui.label(heading, PAGE_TITLE); });
        ui.a11y_name(&title, heading);
        if state.privacy_visible {
            for (index, (title, body)) in [
                ("LocalWave", "A music player for your files."),
                ("Your library stays on this device", "LocalWave stores song metadata, artwork, playlists, Favorites, playback settings, folder bookmarks and cached audio locally. It does not send your music or listening history to the developer and includes no advertising or analytics service."),
                ("Files and iCloud Drive", "Files you select from iCloud Drive or another provider may need downloading through that provider. LocalWave does not receive your Apple Account password."),
                ("Removing your data", "Removing a library entry removes its database record; cached audio or artwork may remain. Removing LocalWave’s local app data erases its library and logs. Your original files stay where they are."),
            ].into_iter().enumerate() {
                ui.id(index as u64, |ui| { ui.column(Node::column().w(Size::Grow).h(Size::Fit).gap(8.0).pad(16.0), |ui| {
                    ui.label(title, TEXT_18.grow().h_px(26.0).color(INK));
                    ui.label(body, PARAGRAPH_14.color(SECONDARY));
                }); });
            }
            if ui.button("full-privacy-policy", "Full privacy policy", &SETTINGS_LINK).clicked { *intent = Some(Intent::PrivacyPolicy); }
            if ui.button("support", "Support", &SETTINGS_LINK).clicked { *intent = Some(Intent::Support); }
            if !player.message.is_empty() { ui.label(player.message, NOTICE); }
            return;
        }
        let settings = player.settings;
        let ahead = SWITCH.detail("Fetch the next two cloud songs in the queue");
        if ui.toggle("download-ahead", "Download ahead", settings.download_ahead, &ahead).changed {
            *intent = Some(Intent::Setting(Change::ToggleDownloadAhead));
        }
        // Swift shows this on phones only: desktop folder tables always list every file type.
        let compact = width < compact::BREAKPOINT;
        if compact {
            let files = SWITCH.detail("Covers, logs and text in folders");
            if ui.toggle("show-non-audio", "Show non-audio files", settings.show_non_audio, &files).changed {
                *intent = Some(Intent::Setting(Change::ToggleShowNonAudio));
            }
        }
        let offline = SWITCH.detail("Prototype only — shows skipped songs and failed downloads");
        if ui.toggle("no-network", "Simulate no network", settings.simulate_no_network, &offline).changed {
            *intent = Some(Intent::Setting(Change::ToggleNoNetwork));
        }
        let cache_detail = match settings.cache_gb { 2 => "2 GB limit · Oldest unpinned downloads go first", 32 => "32 GB limit · Oldest unpinned downloads go first", _ => "Oldest unpinned downloads go first" };
        let cache = ui.pressable("cache", ROW.role(Role::Group), &SETTING, |ui, _| {
            labels(ui, "Offline space", cache_detail);
            let current = CACHE_SEGMENTS.iter().position(|s| s.id == Some(settings.cache_gb));
            chips(ui, "cache-size", &CACHE_SEGMENTS, current, 174.0, Change::CacheGB, SettingsMenu::Cache, intent);
        });
        setting_row(ui, &cache, "Offline space", SettingsMenu::Cache, intent);
        let appearance = ui.pressable("appearance", ROW.role(Role::Group), &SETTING, |ui, _| {
            labels(ui, "Appearance", "");
            let current = APPEARANCES.iter().position(|a| *a == settings.appearance);
            chips(ui, "appearance-chips", &APPEARANCE_SEGMENTS, current, 204.0, |id| Change::Appearance(APPEARANCES[id as usize]), SettingsMenu::Appearance, intent);
        });
        setting_row(ui, &appearance, "Appearance", SettingsMenu::Appearance, intent);
        if ui.button("rescan", "Rescan all sources", &RESCAN).clicked {
            *intent = Some(Intent::Files(crate::files::Request::Roots));
        }
        ui.column(Node::column().w(Size::Grow).h(Size::Fit).gap(8.0).pad(16.0), |ui| {
            ui.label("LocalWave keeps folder bookmarks and caches songs as needed. Individually added files are copied; library data stays on this device.", PARAGRAPH_14.color(SECONDARY));
            if ui.button("about-privacy", "About & Privacy", &SETTINGS_LINK).clicked { *intent = Some(Intent::ShowPrivacy); }
            ui.label(if player.storage_busy || player.cache_usage.is_empty() { "Reading offline space…" } else { player.cache_usage }, PARAGRAPH_14.color(SECONDARY));
            if !player.settings_message.is_empty() { ui.label(player.settings_message, NOTICE); }
            if !player.message.is_empty() { ui.label(player.message, NOTICE); }
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
    let s = player.settings;
    let choice = |item: MenuItem<'static>, current: bool| item.selected(current);
    let cache = [choice(CACHE_2, s.cache_gb == 2), choice(CACHE_32, s.cache_gb == 32)];
    let look = [
        choice(BLUE, s.signal == Signal::Blue),
        choice(RED, s.signal == Signal::Red),
        choice(GREEN, s.signal == Signal::Green),
        choice(MONO, s.signal == Signal::Mono),
        choice(MOTION, s.reduce_motion),
    ];
    let rows: &[MenuItem] = if menu == SettingsMenu::Cache { &cache } else { &look };
    let menu = ui.menu("settings-menu", pxui::Point::new(at.0, at.1), rows, &CHOICES);
    if let Some(id) = menu.chosen_id {
        *intent = Some(Intent::Setting(match id {
            2 | 32 => Change::CacheGB(id),
            BLUE_ID => Change::Signal(Signal::Blue),
            RED_ID => Change::Signal(Signal::Red),
            GREEN_ID => Change::Signal(Signal::Green),
            MONO_ID => Change::Signal(Signal::Mono),
            _ => Change::ToggleReduceMotion,
        }));
    }
    if menu.dismissed && intent.is_none() {
        *intent = Some(Intent::CloseSettingsMenu);
    }
}
/// The settings choice menus: the current choice on the selected fill (`MENU_CHOICE`), rows focused by a press.
const CHOICES: Menu<'static> = super::songs::MENU.w_px(220.0).name("Settings options").variant(MENU_CHOICE).focus_on_click(true);
// The choices; a cache row's id is its size in GB.
const CACHE_2: MenuItem = MenuItem::new("Use 2 GB").id(2);
const CACHE_32: MenuItem = MenuItem::new("Use 32 GB").id(32);
const BLUE_ID: i64 = 100;
const RED_ID: i64 = 101;
const GREEN_ID: i64 = 102;
const MONO_ID: i64 = 103;
const BLUE: MenuItem = MenuItem::new("Use blue signal").id(BLUE_ID);
const RED: MenuItem = MenuItem::new("Use red signal").id(RED_ID);
const GREEN: MenuItem = MenuItem::new("Use green signal").id(GREEN_ID);
const MONO: MenuItem = MenuItem::new("Use monochrome signal").id(MONO_ID);
const MOTION: MenuItem = MenuItem::new("Reduce motion").id(104);

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
        crate::testing::frame(ui, frame, &mut |ui| {
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
                    && (node.is_role(Role::Button) || node.is_role(Role::Toggle) || node.is_role(Role::Radio))
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
        frame.wire.time += 0.02;
        // a copy that borrows the event for this call only
        let mut input = *frame;
        input.events = std::slice::from_ref(&event);
        build(ui, state, player, 0.0, &mut input)
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
                Event::pointer(kind, button, at.0, at.1),
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
        let mut frame = FrameIn::new(1440.0, 872.0);
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
        let key = Event::key(input::wire::KEY_PRESSED, input::Key::Enter.code());
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
            Event::key(input::wire::KEY_RELEASED, input::Key::Enter.code()),
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
            Event::pointer(input::wire::POINTER_PRESSED, 1, at.0, at.1),
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
            Event::pointer(input::wire::POINTER_RELEASED, 1, at.0, at.1),
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
        frame.wire.width = 640.0;
        let start = frame.wire.time;
        for offset in [0.0, 0.5, 1.0] {
            frame.wire.time = start + offset;
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
            Event::key(input::wire::KEY_PRESSED, input::Key::Meta.code()),
        );
        assert_eq!(
            send(
                &mut ui,
                &state,
                &player,
                &mut frame,
                Event::key(input::wire::KEY_PRESSED, ',' as u32)
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
        let mut frame = FrameIn::new(1440.0, 872.0);
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
                        frame.wire.time += 0.02;
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
        let mut frame = FrameIn::new(390.0, 778.0);
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
        let mut frame = FrameIn::new(1440.0, 872.0);
        for _ in 0..3 {
            build(&mut ui, &state, &player, 0.0, &frame);
        }
        assert!(!ui.a11y_nodes().iter().any(|n| ui.a11y_name_of(n) == "Show non-audio files"));
        let at = point(&ui, "Rescan all sources");
        assert_eq!(click(&mut ui, &state, &player, &mut frame, at, 0), Some(Intent::Files(crate::files::Request::Roots)));
    }
}
