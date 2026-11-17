# LocalWave widget-reuse audit (origin/rust-port aed8a36, read-only)

Rule (api_spec.md, "How an APP uses pxui" + "Nodes in app code"): `pressable`/raw `ui.node` is correct only for app-specific
composite content with no pxui widget for the job, inside a registered body, or as a non-interactive container. Building a
control pxui already has (button, icon_button, tabs, segmented, checkbox, selection_list/table_view, search_field, picker,
progress, badge, disclosure...) is DRIFT. Paths are relative to `crates/lw_ui/src/shell/`. Verdict: a = correct primitive
use, b = drift. "Gap" names the pxui shortcoming (G-numbers in the gap list at the end) that blocks the clean form.

| # | file:line | current | v | widget + how LocalWave's look/structure is expressed | pxui gap? |
|---|---|---|---|---|---|
| 1 | sidebar.rs:45 (`side_item`) | `pressable` route/source/list rows; app picks `selected`/`side_row` look; secondary opens list menu; rows are song drop targets | b | `selection_list` (single, navigation) with a registered `list_view` row body (title, optional badge, mono count) and theme looks for idle/hover/selected | G4: rows keyed by index, not route/list id; `ListResponse` has no `secondary` and no per-row handle for `drop_target`; no group headers or mixed rows (add rows) |
| 2 | sidebar.rs:48 | "Reconnect" badge = `ui.label` + `set_look(Look{..})` struct literal | b | `ui.badge("reconnect", name, "Reconnect")` with a theme badge look or registered badge body | none |
| 3 | sidebar.rs:80 (`add_row`) | `button_content` wrapping a hand-padded label node, because "button_with centres it" | b | `button_with(key, title, &ADD_ROW.align(Justify::Start))` with the `add_row` look moved into the theme | none (`Button::align` exists) |
| 4 | sidebar.rs:101 | Search row: `pressable` with "Search" + "⌘K" labels | b | Button; registered button body variant `search-row` drawing the shortcut hint | G2: `PxuiButtonConfig` has no shortcut/secondary text (menu rows have `shortcut`) |
| 5 | sidebar.rs:159 | offline meter: hand-built track + `Size::Pct` fill nodes | b | `ui.progress("offline", "Offline", Some(f))` with a registered progress body (no label row; 4 pt ink bar) | none |
| 6 | sidebar.rs:175 | "? shortcuts" `pressable` with two labels | b | `button_with("shortcuts", ..)` with a theme look (or the `search-row`-style variant) | G2 (two-part label) |
| 7 | header.rs:80-91 | desktop find: `text_field` + separate `icon_button("clear-query")` in a hand-built row | b | `search_field("find-songs", ..)` with a registered `search_field` body (well, icon, `Part::CLEAR`) | none |
| 8 | header.rs:181-206 | compact find: same composition, clear key offset by margin | b | `search_field` with a compact variant of the same body | none |
| 9 | header.rs:348 | filter chips: `pressable` Role::Toggle + manual `set_role(CHECKABLE/CHECKED)` + label node with a `Look{font..}` literal; look swapped by `selected` | b | `checkbox(token, token, on, &Checkbox::DEFAULT.variant(CHIP))` with a registered `checkbox` body "chip" (pill, ink when on) | G5 (minor): `Checkbox` has only `variant` (no look/size/enabled), so everything rides in the body |
| 10 | albums.rs:27 | Artist/Year/Size sort: 3 `pressable`s, SELECTED a11y set by hand, 1.5 pt border hand-drawn with `draw_over` | b | `segmented_with("album-sort", &[Segment::new("Artist"),..], Some(i), &SORT)`; 8 pt gap via a registered `segment` body (it owns the strip since ABI 1.58); 1.5 pt border = `Look.border_w` in the theme's segment look | none |
| 11 | albums.rs:60 | "Shuffle all": `pressable` with drawn icon + label, `enabled(..)` | b | Button via `button_content` (or `button_with` + icon) with theme look | G2: Button has no icon field, so content is still a closure |
| 12 | albums.rs:143 | album cards (artwork + title + subtitle) in a grid | a | composite card; `grid_list` + `pressable` is the right shape | none |
| 13 | artists.rs:168 | artist rows (artwork + columns) in `section_list` | a | composite row. Appearance note: the hover wash is an app-animated node (`animate(Opacity)`); belongs in the theme look's hover state + transition | none (unless look transitions can't fade an overlay) |
| 14 | songs/mod.rs:232 | sortable headings: `pressable` keyed by `column.title()` (display text), non-sortable plain nodes, a11y "Sort by X, ascending" hand-built | b | `table_view` header / `table_head` body (`has_ascending`, `ascending` already in `PxuiTableHeadConfig`) + registered table_head body for the Swift heading look | G4: `table_view` is monolithic; no standalone table-head call usable as `section_list`'s Header part (leading detail header, measured rows, item-id keys, drags); no per-column "not sortable"; no third "original order" state. Also a key rule violation (display text) |
| 15 | songs/row.rs:297 | song row `pressable`: columns are fine, but Meta-toggle / Shift-range / click-select / double-click-play selection is re-implemented app-side | b | row content stays (spec sample); selection semantics belong to `table_view`/`selection_list(multi)` | G4: those widgets can't host this list (index keys, no sections/lead, no drag source/drop, no secondary press) |
| 16 | songs/row.rs:401 | "⋯" actions: `pressable` + three hand-painted circles | b | `icon_button("actions", icon_drawing(Icon::More), row.actions_label, &ICON_32.look(..))` | none |
| 17 | songs/row.rs:427 | reorder grip `pressable` + `carry` | a | no pxui widget for list reordering | G8 |
| 18 | songs/row.rs:194 | drop target with hand-drawn insertion line | a | `ui.drop_target` is the pxui primitive | G7: no insertion/drop-hover look |
| 19 | compact.rs:25 (`mode_tab`, used :227) | Albums/Artists/Songs: `button_content` per tab with label + count + 2 pt underline, look chosen by `active` | b | `tabs("library-mode", &[..], i, &Choice::DEFAULT.variant(MODE))` with a registered `tab` body (label, count, underline) | G1: `PxuiTabConfig` is `{state,index,node,text}`; no count/detail text, so counts can't reach the body |
| 20 | compact.rs:168 | settings gear: `pressable` + `paint_icon` in `draw` | b | `icon_button("settings", icon_drawing(Icon::Settings), "Settings", &ICON_44.look(..))` | none |
| 21 | compact.rs:335 | bottom tab bar: 4 `button_content`s with `tab`/`tab_on` looks, Role Button + selected | b | `tabs("main-tabs", &["Library","Folders","Lists","Find"], i, &Choice::DEFAULT.name("Main navigation").variant(TABBAR))` + registered tab body | G1: `tabs` needs a selected index; no "none selected" while Settings shows (segmented has `Option`) |
| 22 | compact.rs:366 | mini player "open": `pressable` with artwork + two lines | a | composite | none |
| 23 | compact.rs:453 | compact list/artist group cards | a | composite | none |
| 24 | compact.rs:485 (`mode_key`) | Shuffle/Repeat/Queue keys: `button_content` with icon + label | a | Button widget with content. Note: look is swapped app-side (`ink` vs `key`); should be `.selected(active)` + theme look `checked` state | G2 (icon) |
| 25 | compact.rs:524/700 | expanded-player root `pressable(tab_stop(false))` only to own the dismiss gesture | a | no pxui full-screen sheet | G6 |
| 26 | compact.rs:395,404,528,689,693 | prev/play/next/close `icon_button`s | a | correct widget | none |
| 27 | compact/list.rs:155/218 | compact song row `pressable` + swipe/long-press `GestureOpts` | a | composite row + pxui gestures (swipe_actions, long_press) | none |
| 28 | compact/list.rs:792 (also files.rs:683) | selection square hand-drawn (`stroke_w` / ink fill + check) inside a selectable row | b | `selection_list(multi)` whose registered row body draws the mark, or `checkbox` with a body variant | G5/G4: `checkbox` is a whole labelled interactive row (can't be a passive indicator inside a pressable row); list cells get no `selected` flag |
| 29 | compact/list.rs:334 | reorder grip `pressable` + `HELD_TOUCH` + `carry` | a | no reorder widget | G8 |
| 30 | compact/list.rs:363 (`swipe_action`) | `button_content` with a single label | b | `button_with(key, label, &action)`; armed/rest looks in theme | none |
| 31 | compact/list.rs:373 (`more_button`) | `button_content` with empty content + `paint_icon(More)` in `draw` | b | `icon_button("actions", icon_drawing(Icon::More), label, &ICON_44.look(..))` | none |
| 32 | compact/list.rs:391 | pull-to-refresh `gesture` | a | pxui gesture | none |
| 33 | compact/list.rs:459 (`rail`) | A–Z index: per-letter `pressable`s + held touch, letter from pointer y | a | no section-index widget | G9 |
| 34 | compact/list.rs:561 | artist page album cards | a | composite | none |
| 35 | compact/list.rs:639 (`back_row`) | Back: `button_content` chevron + label | a | Button with content | G2 (icon) |
| 36 | compact/list.rs:703 (`offline_key`) | Keep offline key: `button_content` icon + label | a | Button with content | G2 (icon) |
| 37 | compact/list.rs:782 (`word`) | Select/Done/Select all: `button_content` with one label | b | `button_with(key, label, &WORD)`; 16 pt ink via theme look font | none |
| 38 | compact/list.rs:826 (`deck`) | selection deck keys: `pressable` icon-over-label, `enabled(..)`, tone per key | b | Button (`button_content`, `Button::axis(Column)`), DANGER tone via `.danger()` | G2 (icon) |
| 39 | detail.rs:139 (`link`), callers detail.rs:264, compact/list.rs:891 | artist link: `pressable` + `text_line_box` with tracking/weight | b | `button_with(key, artist, &LINK)` + registered button body variant LINK (bare text) | G2: Button can't carry a `Label` style (tracking, line height, ellipsis), only `Look.font` |
| 40 | compact/list.rs:937 (`key`) | Play/Shuffle keys: `button_content` icon + label | a | Button with content | G2 (icon) |
| 41 | detail.rs:72 | Back: `button_content` chevron + label | a | Button with content | G2 (icon) |
| 42 | detail.rs:293 | folder link: `pressable` label + 1 pt underline node | b | Button LINK variant (body draws the underline) | none |
| 43 | detail.rs:363 | artist album cards | a | composite | none |
| 44 | byline.rs:89 | now-panel artist/album links: `pressable`s at app-computed positions in a wrapped byline | b | link (Button LINK variant) | G11: no inline-link / rich-text widget; Button is a box and can't wrap with surrounding text |
| 45 | now.rs:28 | hide-panel `icon_button` | a | correct | none |
| 46 | now.rs:187 | favourite heart: `button_content` with drawn heart + 6-ray burst animation | a | Button widget with live content. Note: on/off is carried in the name; should also be `.selected(on)` | none |
| 47 | transport.rs:37-52 | prev/play/next `icon_button`s | a | correct | none |
| 48 | transport.rs:73-117 | shuffle/repeat/panel/queue `icon_button`s; look swapped `icon_inverse`/`icon_plate` by state; repeat-one "1" overlay | a | correct widget | G3: `IconButton` has no `selected`/checked (no CHECKED a11y; look must be swapped app-side) |
| 49 | transport.rs:242 | up-next drop: `pressable(tab_stop(false))` only so drops hit empty areas; accent outline hand-drawn on hover | a | no drop-zone widget | G7: `drop_target` needs a hit-testable (pressable) node; no drop-hover look state |
| 50 | transport.rs:317 | `ui.scrub` + app-painted rail/fill | a | correct widget | G9: `scrub` has no body kind/look (caller-painted by design), so it isn't customizable like other widgets |
| 51 | queue_sheet.rs:21 (`queue_action`) | `button_content` wrapping a left-aligned label node | b | `button_with(key, label, &ACTION.align(Justify::Start))` + theme look | none |
| 52 | queue_sheet.rs:122/317 | queue sheet root `pressable(tab_stop(false))` for the dismiss gesture; Role::Dialog/MODAL set by hand | a | no pxui bottom sheet | G6 |
| 53 | queue_sheet.rs:154 | queue titles: `pressable` with "Queue" + "From … · shuffled", click/secondary = save as playlist | b | Button (`button_content`, response `.secondary`) | G2 (no detail/subtitle text) |
| 54 | queue_sheet.rs:176,403,529 | close/show-queue/remove `icon_button`s | a | correct | none |
| 55 | queue_sheet.rs:307 | "Clear up next": `pressable` with one label | b | `button_with("clear", "Clear up next", &CLEAR.align(Justify::Start))` | none |
| 56 | queue_sheet.rs:460/540 | queue row root `pressable(tab_stop(false))` (item id key) for drops + insertion line | a | composite row in `ui.list` | G7 |
| 57 | queue_sheet.rs:461 | reorder grip | a | no reorder widget | G8 |
| 58 | queue_sheet.rs:502 | play-queued body (title + artist) `pressable` | a | composite row body (spec sample shape) | none |
| 59 | onboarding.rs:56 (`underlined`) | links: `button_content` label + rule | b | `button_with(key, label, &LINK)`, body variant draws the underline | none |
| 60 | onboarding.rs:104 | "Choose a folder": `pressable` with one label on `ink` look | b | `button_with("choose-folder", "Choose a folder", &PRIMARY)` + theme primary look | none |
| 61 | onboarding.rs:146 | "Google Drive not in the list?" disclosure: `pressable` + hand-sliced triangle | b | disclosure: `inspector_section` with a registered `inspector_section` body | G10: no general disclosure widget in the lean API (`inspector_section` is inspector-scoped, `&str` key, no style value/variant) |
| 62 | settings.rs:13 (`back`; :193,212,213,278) | raw `ui.node` with CLICKABLE\|FOCUSABLE + `Look{..}` literal; key = display text ("‹ Settings") | b | `button_with("back", label, &BACK_LINK)` (LINK variant) | none. Key rule violation |
| 63 | settings.rs:124 (`option`; :241,253) | 4/8/16 GB and System/Light/Dark: raw nodes CLICKABLE, Role::Toggle, look literals, right-click context | b | `segmented_with("cache", &[Segment::new("4 GB").name("Use 4 GB"),..], Some(i), &CHIPS)`; `SegmentedResponse.secondary` for the menu | none |
| 64 | settings.rs:219-231 | `toggle_with` + registered `localwave-toggle` body | a | reference pattern | none |
| 65 | settings.rs:235/245 | cache/appearance rows: `pressable(focus_on_click)` Role::Group hosting the strip, secondary = context menu | a | composite settings row | none |
| 66 | settings.rs:259 | "Rescan all sources": `pressable` title + detail + hand-dotted chevron | b | Button with a registered "disclosure row" body | G2: Button has no detail text or trailing accessory (`Toggle` has `detail`) |
| 67 | files.rs:97-125 (`button`/`sized_button`/`button_node`/`action`; :336-363,401,409,500,587,792) | Import folder / Reconnect / Options / Import / Close: raw nodes CLICKABLE\|FOCUSABLE, manual role/name/ENABLED, tone tuples | b | `button_with(key, label, &FILES_KEY.size(w,h).enabled(e))` with theme looks for primary/secondary tones | none |
| 68 | files.rs:128 (`sheet_body`) | registered dialog body using `ui.part(Part::button(i), ..)` raw nodes | a | inside a registered body (allowed) | none |
| 69 | files.rs:158 (`link`; :397) | "‹ Back": raw node CLICKABLE + Look literal | b | Button LINK variant | none |
| 70 | files.rs:274 | "+" add music: raw node + two hand-painted bars | b | `icon_button("add-music", icon_drawing(Icon::Add), "Add music", ..)` (add an app icon) | none |
| 71 | files.rs:336 (`source_card`) | folder card: raw node CLICKABLE\|FOCUSABLE, `signal()` for click + right-click | b | `pressable(Key::Id(folder.id), CARD, &PLAIN.look(..))` (composite: the spec names "hand-built clickable rows where pressable fits") | none |
| 72 | files.rs:316, files.rs:371 | offline and per-source meters hand-built | b | `ui.progress` with the same registered progress body as #5 | none |
| 73 | files.rs:448/475 | desktop folder table: plain header cells + `grid_list` rows, directory rows raw CLICKABLE | b | `table_view` with text cells via `cell` and registered `table_head`/`table_view` row bodies | G4: rows keyed by index (not entry id); widths are points+grips (no weighted shares); no per-row enabled (plain files aren't clickable) |
| 74 | files.rs:543 | compact Back: raw node, icon + label | b | Button (`button_content`), same as detail back | G2 (icon) |
| 75 | files.rs:559 | "Select": raw node + label | b | `button_with("select", "Select", &WORD)` | none |
| 76 | files.rs:570, 687, 697 | "⋯" folder/file options: raw nodes + `paint_icon(More)` | b | `icon_button(.., icon_drawing(Icon::More), ..)` | none |
| 77 | files.rs:600 | Shuffle / Keep-offline 52 pt keys: raw nodes, ink fill when kept | b | `icon_button` | G3 (kept state) |
| 78 | files.rs:628 | All / Offline / In cloud filter tabs: raw nodes with label + count + underline, SELECTED set by hand | b | `tabs("files-filter", ..)` + the registered tab body from #19 | G1 (count text) |
| 79 | files.rs:654 | "show" non-audio: raw node + label | b | Button LINK variant | none |
| 80 | files.rs:665 | compact directory rows: raw CLICKABLE nodes (browse/play/import; toggle in select mode) | b | `pressable(Key::Id(entry.id), ..)` composite row; select mode = `selection_list(multi)` | G4 for the select-mode form |
| 81 | files.rs:784 | Add music choices (title + detail paragraph): raw CLICKABLE nodes in a `modal` | b | Button (`button_content`) or dialog buttons | G13/G2: `Dialog` buttons and Button are text-only (no detail line) |
| 82 | commands.rs:242-335 | command palette: `text_field` + app ArrowUp/Down/Enter selection + `section_list` + raw CLICKABLE result rows (SELECTED by hand) | b | `picker("palette", ..)` with a registered `picker` row body | G12: picker rows have no artwork/icon slot (swatch only) and no group headings; rows from an async worker query need checking against `fill(query, rows)` |
| 83 | find.rs:71 (`button`; used find.rs:113-123, files.rs:425-433) | Find/folder tools: `button_content` re-builds hover/press washes as animated nodes, selected look swapped, label width measured per call | b | Button `.selected(kept)` with theme look hover/pressed/checked states (and transitions) | G2 (icon); if `Look` transitions can't fade the wash, a look-transition gap |
| 84 | menus/dialogs/fields: settings.rs:303, songs/menu.rs:60,127, lists.rs:38,62,104, metadata.rs:48,96, files.rs:34,75,758, compact/page.rs:57,99, queue_sheet.rs:198, commands.rs:18,256 | `ui.menu`, `ui.dialog` (with registered `localwave-sheet` body), `text_field` (with `localwave-command` body) | a | correct widgets + registered bodies | none |
| 85 | lw_bin (appearance.rs:23; app/input.rs, actions.rs pickers are OS file dialogs) | no interactive UI construction; applies `text_field.body = localwave-command` | a | correct | none |

Counts: a = 35, b = 50 (85 rows; rows group repeated call sites of one helper).

## (b) grouped by the widget that should be used

- Button, text or LINK/primary variant (16): #3, #6, #30, #37, #39, #42, #44, #51, #55, #59, #60, #62, #67, #69, #75, #79
- Button with content or a body variant (icon/subtitle/detail) (8): #4, #11, #38, #53, #66, #74, #81, #83
- icon_button (6): #16, #20, #31, #70, #76, #77
- tabs (3): #19, #21, #78
- segmented (2): #10, #63
- checkbox (2): #9, #28
- selection_list / table_view (+ table_head) (4): #1, #14, #15, #73
- pressable instead of a raw interactive node (2): #71, #80
- search_field (2): #7, #8
- picker (1): #82
- progress (2): #5, #72
- badge (1): #2
- disclosure (inspector_section) (1): #61

## pxui gaps (what blocks the clean form)

- G1 tabs: `PxuiTabConfig { state, index, node, text }` has no count/detail/badge field (#19, #78); `tabs` needs a `usize` selection, so no tab can be unselected (#21; `segmented_with` takes `Option`); tabs and segments are keyed by position, never by `"tab-…"` ids.
- G2 Button: no icon field, no detail/subtitle/shortcut text, no trailing accessory and no `Label` text style (tracking, line height, ellipsis). Icon+label keys, two-line buttons and styled links fall back to `button_content` (#4, #6, #11, #24, #35, #36, #38, #39, #40, #41, #53, #66, #74, #81, #83).
- G3 IconButton: no `selected`/checked state (Button has one), so toggle keys (shuffle/repeat/panel/kept) swap looks app-side and expose no CHECKED state (#48, #77).
- G4 selection_list / table_view: rows keyed by index, not item id. No sections, leading header or measured rows. `ListResponse`/`TableResponse` have no `secondary` and no per-row handle for drag/drop. Cells don't get the selected flag. There is no standalone `table_head` to use as a `section_list` Header, no per-column "not sortable", no "original order" sort state, and no weighted column widths (#1, #14, #15, #28, #73, #80).
- G5 checkbox: always a labelled interactive row (it can't be a passive mark inside a row); `Checkbox` has only `variant` (no look/size/enabled) (#9, #28).
- G6 no full-screen or bottom sheet overlay with a built-in `DismissOpts` drag. `modal` is centred, so LocalWave makes a pressable root to host the gesture (#25, #52).
- G7 drop zones: `drop_target` only reaches hit-testable nodes, which forces `pressable(tab_stop(false))` on containers. There is no drop-hover look state and no insertion indicator, so both are hand-drawn (#18, #49, #56).
- G8 no reorderable list or drag-handle part (pxui `grip` is a splitter) (#17, #29, #57).
- G9 no section-index (A–Z) widget. `scrub` is horizontal, caller-painted and has no body kind (#33, #50).
- G10 no general disclosure/collapsible-section widget in the lean API; only `inspector_section` (#61).
- G11 no inline link inside wrapped text (#44).
- G12 picker rows lack an artwork/icon slot and group headings (#82).
- G13 dialog buttons are text-only (no detail line) (#81).

## Related findings (not counted above)

- Shadow theme: `theme.rs:54-140` (`Looks`, `resolve(dark)`, `token()` reading a `COLORS` table) resolves colour tokens into light/dark `Rgba` app-side, and `looks(ui)` picks `&LIGHT`/`&DARK`. The spec forbids this ("No app-side shadow theme that resolves tokens into colours itself"). Every `looks(ui).x` above is affected. `tinted()` (alpha 153 of a token) and artists.rs:177 suggest a missing pxui feature: a token with an alpha modifier in `Look`/`States`. Check this before calling it a gap; `Rgba::token(..)` already works in settings.rs consts.
- Look struct literals at call sites (`Look { text, font, ..Look::NONE }`) in settings.rs:15,127, header.rs:351, files.rs:162, sidebar.rs:49, and others. These are per-frame style construction instead of named theme looks.
- Keys from display text: songs/mod.rs:232 (`column.title()`), settings.rs:13 (`name.into()` = "‹ Settings"), settings.rs:124 (`label.into()` = "4 GB").
