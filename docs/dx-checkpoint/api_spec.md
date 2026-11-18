THE TARGET PUBLIC API (from the approved proposal; the code must read like this — one lean shape per thing,
const values with const builders, no option structs written as struct literals at call sites, no `_with`
twins where one call suffices). Names may differ only where an existing public type already owns the name.

    const ROW: Node = Node::row().h_px(44.0).gap(8.0).center();
    const TITLE: Label = Label::BODY.size(15.0).bold();        // app-defined label styles are consts
    const DIM: Label = Label::BODY.color(DIM_INK);              // colour = a ColorToken (DX3.2)

    const PRESSABLE: Pressable = Pressable::DEFAULT;   // looks ride in the style value, like KEY.look(..) below (DX3.15)
    let row = ui.pressable(song.id, ROW, &PRESSABLE.look(&theme.list_row), |ui, state| {
        ui.label(&song.title, TITLE.grow());
        ui.label(&song.duration_text, DIM);
    });
    if row.clicked { intent = Some(Intent::Play(song.id)); }

    ui.label_fmt(format_args!("{} of {}", index + 1, total), DIM);   // 128-byte stack buffer, no heap
    ui.label_digits(format_args!("{m}:{s:02}"), CLOCK);              // pinned glyphs, no shaping

    ui.view_of("visual", &(icon, inverse, row.hovered, row.held), |ui| { /* build */ });

    const KEY: IconButton = IconButton::sized(36.0, 36.0).press_scale(0.96);
    if ui.icon_button("play", Icon::Play, "Play", &KEY.look(&theme.key)).clicked { .. }

    const ITEMS: [MenuItem; 3] = [MenuItem::new("Play next"), MenuItem::new("Add to playlist"), MenuItem::new("Delete").danger()];
    let menu = ui.menu("song-menu", at, &ITEMS, &Menu::DEFAULT.row_h(36.0));
    if let Some(i) = menu.chosen { .. }

    ui.list("songs", songs.len(), 44.0, &|i| songs[i].id, |ui, i| { song_row(ui, &songs[i]); });

    const SETTING: Toggle = Toggle::DEFAULT.row_h(52.0);
    if ui.toggle("offline", "Offline mode", prefs.offline, &SETTING).changed { .. }
    ui.disabled(!prefs.offline, |ui| { ui.button("clear", "Clear offline cache", &Button::DEFAULT.danger()); });

Host setup and input in U1a

    let desc = UiDesc::new(Platform::Apple)
        .font_size(15.0)
        .fonts(&[FontData(FONT), FontData(MONO)]);
    let mut ui = Ui::new(&desc)?;
    register_bodies(&mut ui)?;
    let mut theme = ThemeHandle::parse(APP_THEME)?;
    theme.set_color(SURFACE, paper);
    ui.set_theme(&theme)?;

    let mut input = FrameIn::new(width, height).scale(pixel_scale).time(now);
    input.wire.events = events.as_ptr();
    input.wire.event_count = events.len();
    ui.frame(&input, &mut |ui| {
        if ui.key_pressed(KeyCode::Char('o'), Mods::META) { /* open */ }
        ui.themed(&theme, &mut |ui| { /* build a scoped subtree */ });
    })?;

    for layer in ui.layers() {
        /* consume the borrowed draw instances and dirty ranges */
    }

Keep event records and payloads alive during the frame. A host with a full `Input`
can borrow it through `unsafe FrameIn::snapshot(&input)` with its backing spans live.
Output queries copy the descriptor, and a renderer's acknowledgement preserves
the bytes already lent to a layer reader. Body names resolve in the destination UI,
with an error on root installation and a built-in fallback in a scoped theme.
`Mods::ANY` accepts every modifier state, and `Mods::NONE` requires no modifiers.
`Mods::META.with(Mods::SHIFT)` matches both modifiers on the same press.
Accessible names clip at a UTF-8 boundary and report a live update as a boolean.
The value and number setters return `Result<bool, Error>`.

Rules
- Style/option types (Label, Button, IconButton, Menu, MenuItem, Toggle, Dialog...) are Copy values with a `DEFAULT`
  (and named presets like Label::BODY/CAPTION/TITLE/MONO), and `const fn` builders taking self by value. A call site
  never writes `&Struct { field: .., ..Default::default() }`.
- Labels and row, column and stack use sibling positions. A conditional or animated layout uses `ui.id(key, |ui| ...)` when it needs
  stable identity. This scope changes descendant keys without adding a layout node. Interactive and virtual row keys use fixed labels
  or item ids. Use `Key::Resolved` for a precomputed box key in a lookup.
- Interactive widgets return `Response` (clicked, hovered, held, focused, activated, changed where it applies, handle) or a
  small result struct (`menu.chosen`).
- Button and toggle each take a key, runtime data and one style. Style builders use `.disabled(bool)`. A pick-one widget reports the
  current selection, the chosen item's id and the index where a secondary press began. Slider and scrub share one result with a
  display value, held preview and final commit.
- In-frame text measurement uses `text_service()` on Ui or Paint, with a `Label`. Worker `Text` takes the style resolved by `Ui::resolve`.
- Every public item documented per pxui/AGENTS.md "Public API documentation".
- Widget bodies (structural customization) stay the mechanism: every widget that has a body kind (Label, Button, IconButton,
  Toggle, Segment, Dialog, MenuItem, MenuRow, Row, TreeRow, Scroll, Tab, Dropdown, SearchField, TextField, ... — see the
  Pxui*Config list in crates/public/pxui/src/bodies.rs and body_kinds! in pxui_types theme.rs) is built THROUGH its body, so an
  app-registered body replaces its structure. New style values (Label, Button, Toggle, Menu...) are carried into the body's
  config record (append-only fields behind the size prefix if a field is new). A new public widget call that bypasses its body is
  a defect. Proof: the gallery bodies page and the abi_bench bodies page (every kind overridden by a registered copy) still
  pass, and a test registers a custom body for the widget you touched and asserts the custom structure is used.

How an APP uses pxui (owner, 2026-11-17 — LocalWave is the reference app; drift here is a blocker)
- One public API: the calls above. The app never re-implements a pxui widget (no hand-built clickable rows where `pressable`
  fits, no hand-built menus/toggles/dialogs/buttons, no wrapper helpers that recreate an old or second call shape).
- Appearance = the theme: colour tokens and theme looks (theme files / looks naming ColorTokens). No app-side shadow theme that
  resolves tokens into colours itself.
- Structure = registered app bodies (structural customization) + variants. A distinctive LocalWave menu row / toggle / sheet is a
  registered body selected by the theme or a `.variant(n)`, reached through the standard call.
- Style values are app `const`s built with const builders (`const CAPTION: Label = …`), named once and reused; no struct literals
  of pxui style/option types; no per-frame construction of styles.
- Keys are &'static str ids or item ids — never display text (tabs get "tab-songs", rows get item ids).
- Zero warm allocations: fixed arrays for menus, no per-frame String/Vec/format!/sort/hash in build paths.
- If the app cannot express its design through these, that is a pxui gap: report it (file:line, what's missing) — never work around it.

OS-grade (owner, 2026-11-17): pxui is the future OS base — see os_criteria.md beside this file; it is part of the spec.
- Nodes in app code (owner concern, 2026-11-17): raw `ui.node` is allowed (a) inside a REGISTERED BODY function (that is how an app
  customizes a widget's structure) and (b) as a plain layout container/leaf (row/column/stack/spacer, no interaction). Building an
  interactive control from nodes or `pressable` when pxui has a widget for that job (tabs, segmented, selection list, table header,
  checkbox, toggle, menu, dialog, button, slider/scrub, text field, tree, picker, tooltip) is DRIFT: use the widget + a registered body
  + theme looks/variants; if the widget can't express it, report a pxui gap.
