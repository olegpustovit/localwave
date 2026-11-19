//! Port of WaveOnboardingView for a library with no sources, plus the empty-library page and the
//! Find hint. All strings are static; the screen is one retained view keyed by its width and the
//! Google Drive disclosure. Deferred against Swift: the demo-library installer (92 bundled tracks;
//! the second link stays "Explore the library"), the folder glyph on the primary button and the
//! live song count (onboarding only shows for an empty library, so it is always "00").
use super::*;

/// Swift's `didStart` gate (persisted as `lwDidStart`): shown until a source is added or it is
/// explored. An empty library alone never re-onboards a user who already started.
pub(super) fn wanted(catalog: &Catalog, state: &ShellState, did_start: bool) -> bool {
    !did_start && catalog.sources == Some(0) && catalog.is_empty() && !state.explored
}

/// A source exists but the flag is unset (import finished): persist it once, as Swift's `onImported`.
pub(super) fn needs_start_flag(catalog: &Catalog, did_start: bool) -> bool {
    !did_start && catalog.sources.is_some_and(|count| count > 0)
}

pub(super) const LIBRARY_EMPTY: &str =
    "Your music starts with a folder. Add music in Folders to build your library.";
pub(super) const FIND_HINT: &str = "Search spans every source, including folders that are not in an album. Add a word like flac or cloud to filter.";

pub(super) fn view(ui: &mut Ui, top_inset: f32, intent: &mut Option<Intent>) -> Handle {
    let pxui::ViewSize { width, .. } = ui.view_size();
    let column = (width - 32.0).max(1.0);
    let root = container(ui, "onboarding", Axis::Column, Size::Grow, Size::Grow, &PAPER_FILL);
    ui.set_role(&root, Role::Window, pxui::A11yStates::NONE);
    ui.a11y_name(&root, "LocalWave");
    // The disclosure state lives on the root so the retained page rebuilds when it flips.
    let drive = ui.state(&root, 0) != 0;
    ui.within(&root, &mut |ui| {
        ui.view_of(
            "onboarding-page",
            &(column.to_bits(), top_inset.to_bits(), drive),
            |ui| {
                ui.column(Node::column().w(Size::Grow).h(Size::Grow).padding(Edges::new(16.0, 16.0 + top_inset, 16.0, 34.0)),
                    |ui| {
                        if page(ui, drive, intent) {
                            ui.set_state(&root, i64::from(!drive));
                        }
                    },
                );
            },
        );
    });
    root
}

/// An underlined text link (`localwave-button` / `LINK`): as wide as its text, 44 points high, no look of its own.
const UNDERLINED: pxui::Button = pxui::Button::DEFAULT.variant(variant::LINK).size(Size::Fit, Size::Px(44.0)).centered().pad(Edges::ZERO).label(LEAD_INK);
/// The primary key (`localwave-button` / `PRIMARY`): paper 15-point text centred on ink across the page, 52 points high.
const CHOOSE: pxui::Button = pxui::Button::DEFAULT.variant(variant::PRIMARY).size(Size::Grow, Size::Px(52.0)).label(TEXT_15.color(PAPER)).look(&INK_LOOKS);
/// The Google Drive disclosure (`localwave-disclosure`): the 1-pixel sliced triangle and the 13-point title.
const DRIVE: pxui::Disclosure = pxui::Disclosure::DEFAULT;
const DRIVE_HELP: &str = "Install the Google Drive app, then in Files tap Browse → ⋯ → Edit and turn on Google Drive.";

/// Returns true when the Google Drive disclosure was toggled.
fn page(ui: &mut Ui, drive: bool, intent: &mut Option<Intent>) -> bool {
    let mut toggled = false;
    ui.column(Node::column().w(Size::Grow).h(Size::Grow), |ui| {
        let top = container(ui, "brand", Axis::Row, Size::Grow, Size::Px(18.2), &PAPER_FILL);
        ui.within(&top, &mut |ui| {
            ui.label("LocalWave", CAPTION.grow());
            ui.label("v2", MONO_DIM.w_px(16.0));
        });
        spacer(ui, "count-gap", 28.0);
        let count = container(ui, "count-row", Axis::Row, Size::Grow, Size::Px(79.0), &PAPER_FILL);
        ui.set_role(&count, Role::Label, pxui::A11yStates::NONE);
        ui.a11y_name(&count, "0 songs so far");
        ui.within(&count, &mut |ui| clock::matrix_digits(ui, "count", b"00", 79.0, INK));
        spacer(ui, "count-label-gap", 8.0);
        ui.label("songs so far", LEAD_DIM.grow());
        spacer(ui, "heading-gap", 28.0);
        ui.label("Point it at a folder.", TEXT_32.grow().h_px(36.8).color(INK));
        ui.label("It stays in sync.", TEXT_32.grow().h_px(36.8).color(INK));
        spacer(ui, "promise-gap", 8.0);
        ui.label("Your files stay where they are. Nothing is uploaded.", PARAGRAPH_15.color(SECONDARY));
        spacer(ui, "steps-gap", 28.0);
        rule(ui, "steps-rule", false);
        for (key, number, line) in [
            ("step-1", "1", "Choose a folder in Files"),
            ("step-2", "2", "LocalWave indexes it in the background"),
            ("step-3", "3", "Play at once; cloud songs fetch on play"),
        ] {
            let row = container(ui, key, Axis::Row, Size::Grow, Size::Px(46.0), &PAPER_FILL);
            ui.within(&row, &mut |ui| {
                ui.label(number, TEXT_13.w_px(26.0).h_px(46.0).color(SECONDARY));
                ui.label(line, TEXT_15.grow().h_px(46.0).color(INK));
            });
            rule(ui, "step-rule", false);
        }
        ui.node("onboarding-flex", Node::DEFAULT.h(Size::Grow));
        ui.label("iCloud Drive, Google Drive, On My iPhone, USB, SMB — anything in Files.", PARAGRAPH_13.color(SECONDARY));
        spacer(ui, "choose-gap", 12.0);
        if ui.button("choose-folder", "Choose a folder", &CHOOSE).clicked {
            *intent = Some(Intent::ChooseFolder);
        }
        spacer(ui, "links-gap", 4.0);
        let links = container(ui, "links", Axis::Row, Size::Grow, Size::Px(44.0), &PAPER_FILL);
        ui.within(&links, &mut |ui| {
            if ui.button("pick-files", "Pick some files", &UNDERLINED).clicked {
                *intent = Some(Intent::ChooseAudio);
            }
            ui.node("links-flex", Node::DEFAULT.w(Size::Grow));
            if ui.button("explore", "Explore the library", &UNDERLINED).clicked {
                *intent = Some(Intent::Navigate(Route::Albums));
            }
        });
        spacer(ui, "drive-gap", 4.0);
        // Swift's "Google Drive not in the list?" disclosure in its bordered 1px frame
        let frame = ui.node("drive-frame", Node::column().w(Size::Grow).h(Size::Fit).pad(1.0).look(LINE_FILL));
        ui.within(&frame, &mut |ui| {
            ui.column(Node::column().w(Size::Grow).h(Size::Fit).pad(14.0).look(PAPER_FILL), |ui| {
                toggled = ui.disclosure("drive-toggle", "Google Drive not in the list?", drive, &DRIVE, &mut |ui| {
                    spacer(ui, "drive-help-gap", 8.0);
                    ui.label(DRIVE_HELP, PARAGRAPH_13.color(SECONDARY));
                }).changed;
            });
        });
    });
    toggled
}

/// LocalWave's disclosure header (registered as `localwave-disclosure`, the `inspector_section` kind): a 19.5-point row of the
/// triangle, drawn as 1-pixel slices (7 x 8⅓ closed, 8⅓ x 7 open), and the 13-point ink title; the content under it while open.
pub(crate) fn disclosure_body(ui: &mut Ui, c: &pxui::bodies::InspectorSectionConfig) {
    use pxui::bodies::BodyConfig;
    let expanded = c.open != 0;
    ui.set_node_layout(&c.title, &Node::row().w(Size::Grow).h_px(19.5));
    ui.within(&c.title, &mut |ui| {
        let marker = ui.node("marker", Node::column().w_px(40.0 / 3.0).h_px(19.5));
        let ink = Rgba::token(INK);
        ui.canvas(&marker, move |paint, rect| {
            let (w, h) = if expanded { (25.0 / 3.0, 7.0) } else { (7.0, 25.0 / 3.0) };
            let top = rect.y + 14.0 - h;
            for i in 0..h as i32 {
                let t = (i as f32 + 0.5) / h;
                let span = if expanded { w * (1.0 - t) } else { w * (1.0 - (2.0 * t - 1.0).abs()) };
                let x = if expanded { rect.x + (w - span) / 2.0 } else { rect.x };
                paint.fill(Rect::new(x, top + i as f32, span, 1.0), Fill::new(ink).fractional());
            }
        });
        ui.label(c.text_of(&c.label), TEXT_13.grow().h_px(19.5).color(INK).role(Role::Unknown));
    });
    if expanded {
        ui.slot(c.content);
    }
}

/// Swift's "Your music starts with a folder" message under the Albums/Artists header.
pub(super) fn empty_library(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let compact = ui.view_size().width < compact::BREAKPOINT;
    if !compact {
        header_view(ui, catalog, state, width, intent);
    }
    let inset = if compact { 16.0 } else { 32.0 };
    ui.column(Node::column().w(Size::Grow).h(Size::Fit).padding(Edges::ZERO.l(inset).r(inset).t(24.0)),
        |ui| {
            ui.label(LIBRARY_EMPTY, PARAGRAPH_14.color(SECONDARY));
        },
    );
}
