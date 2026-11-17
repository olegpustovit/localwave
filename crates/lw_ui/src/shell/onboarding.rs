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
    let (width, _) = ui.view_size();
    let column = (width - 32.0).max(1.0);
    let root = container(ui, "onboarding", Axis::Column, Size::Grow, Size::Grow, ui.color(PAPER));
    ui.set_role(&root, Role::Window, 0);
    ui.a11y_name(&root, "LocalWave");
    // The disclosure state lives on the root so the retained page rebuilds when it flips.
    let drive = ui.state(&root, 0) != 0;
    ui.within(&root, &mut |ui| {
        ui.view_of(
            "onboarding-page",
            &(column.to_bits(), top_inset.to_bits(), drive),
            |ui| {
                ui.column(
                    "onboarding-outer",
                    Node {
                        padding: Edges { l: 16.0, r: 16.0, t: 16.0 + top_inset, b: 34.0 },
                        ..Node::column().w(Size::Grow).h(Size::Grow)
                    },
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

fn underlined(ui: &mut Ui, key: &'static str, label: &'static str) -> bool {
    let link = pxui::Button::DEFAULT.size(Size::Fit, Size::Px(44.0)).centered().look(&BARE).name(label);
    ui.button_content(key, &link, |ui, _| {
        ui.column("link", Node::column().w(Size::Fit).h(Size::Fit), |ui| {
            ui.label(label, LEAD_INK);
            rule(ui, "underline", false);
        });
    })
    .clicked
}

/// Returns true when the Google Drive disclosure was toggled.
fn page(ui: &mut Ui, drive: bool, intent: &mut Option<Intent>) -> bool {
    let mut toggled = false;
    ui.column("onboarding-column", Node::column().w(Size::Grow).h(Size::Grow), |ui| {
        let top = container(ui, "brand", Axis::Row, Size::Grow, Size::Px(18.2), ui.color(PAPER));
        ui.within(&top, &mut |ui| {
            ui.label("LocalWave", CAPTION.w(Size::Grow));
            ui.label("v2", MONO_DIM.w(Size::Px(16.0)));
        });
        spacer(ui, "count-gap", 28.0);
        let count = container(ui, "count-row", Axis::Row, Size::Grow, Size::Px(79.0), ui.color(PAPER));
        ui.set_role(&count, Role::Label, 0);
        ui.a11y_name(&count, "0 songs so far");
        ui.within(&count, &mut |ui| clock::matrix_digits(ui, "count", b"00", 79.0, ui.color(INK)));
        spacer(ui, "count-label-gap", 8.0);
        ui.label("songs so far", LEAD_DIM.w(Size::Grow));
        spacer(ui, "heading-gap", 28.0);
        ui.label("Point it at a folder.", TEXT_32.w(Size::Grow).h(36.8).color(INK));
        ui.label("It stays in sync.", TEXT_32.w(Size::Grow).h(36.8).color(INK));
        spacer(ui, "promise-gap", 8.0);
        ui.label("Your files stay where they are. Nothing is uploaded.", PARAGRAPH_15.color(SECONDARY));
        spacer(ui, "steps-gap", 28.0);
        rule(ui, "steps-rule", false);
        for (key, number, line) in [
            ("step-1", "1", "Choose a folder in Files"),
            ("step-2", "2", "LocalWave indexes it in the background"),
            ("step-3", "3", "Play at once; cloud songs fetch on play"),
        ] {
            let row = container(ui, key, Axis::Row, Size::Grow, Size::Px(46.0), ui.color(PAPER));
            ui.within(&row, &mut |ui| {
                ui.label(number, TEXT_13.w(Size::Px(26.0)).h(46.0).color(SECONDARY));
                ui.label(line, TEXT_15.w(Size::Grow).h(46.0).color(INK));
            });
            rule(ui, "step-rule", false);
        }
        ui.box_("onboarding-flex", BoxOpts::default().h(Size::Grow));
        ui.label("iCloud Drive, Google Drive, On My iPhone, USB, SMB — anything in Files.", PARAGRAPH_13.color(SECONDARY));
        spacer(ui, "choose-gap", 12.0);
        let look = Look { fill: ui.color(INK), ..Look::NONE };
        let node = Node::row().w(Size::Grow).h_px(52.0).center().role(Role::Button);
        let choose = ui.pressable("choose-folder", node, &look, |ui| {
            ui.label("Choose a folder", TEXT_15.color(PAPER));
        });
        ui.a11y_name(&choose.handle, "Choose a folder");
        if choose.clicked {
            *intent = Some(Intent::ChooseFolder);
        }
        spacer(ui, "links-gap", 4.0);
        let links = container(ui, "links", Axis::Row, Size::Grow, Size::Px(44.0), ui.color(PAPER));
        ui.within(&links, &mut |ui| {
            if underlined(ui, "pick-files", "Pick some files") {
                *intent = Some(Intent::ChooseAudio);
            }
            ui.box_("links-flex", BoxOpts::default().w(Size::Grow));
            if underlined(ui, "explore", "Explore the library") {
                *intent = Some(Intent::Navigate(Route::Albums));
            }
        });
        spacer(ui, "drive-gap", 4.0);
        toggled = drive_box(ui, drive);
    });
    toggled
}

/// Swift's "Google Drive not in the list?" disclosure with its bordered 1px frame.
fn drive_box(ui: &mut Ui, expanded: bool) -> bool {
    let frame = ui.node(
        "drive-frame".into(),
        Node {
            look: Look { fill: ui.color(LINE), ..Look::NONE },
            ..Node::column().w(Size::Grow).h(Size::Fit).pad(1.0)
        },
    );
    let mut clicked = false;
    ui.within(&frame, &mut |ui| {
        ui.column(
            "drive-inner",
            Node {
                look: Look { fill: ui.color(PAPER), ..Look::NONE },
                ..Node::column().w(Size::Grow).h(Size::Fit).pad(14.0)
            },
            |ui| {
                let header = ui.pressable("drive-toggle", Node::row().w(Size::Grow).h_px(19.5).role(Role::Button), &Look::NONE, |ui| {
                    let marker = ui.node("marker".into(), Node::column().w_px(40.0 / 3.0).h_px(19.5));
                    let ink = ui.color(INK);
                    ui.draw(&marker, move |paint, rect| {
                        // Filled triangle, 7×8⅓ closed / 8⅓×7 open, drawn as 1px slices.
                        let (w, h) = if expanded { (25.0 / 3.0, 7.0) } else { (7.0, 25.0 / 3.0) };
                        let top = rect.y + 14.0 - h;
                        for i in 0..h as i32 {
                            let t = (i as f32 + 0.5) / h;
                            let span = if expanded { w * (1.0 - t) } else { w * (1.0 - (2.0 * t - 1.0).abs()) };
                            let x = if expanded { rect.x + (w - span) / 2.0 } else { rect.x };
                            paint.fill_fractional(x, top + i as f32, span, 1.0, ink);
                        }
                    });
                    ui.label("Google Drive not in the list?", TEXT_13.w(Size::Grow).h(19.5).color(INK));
                });
                ui.a11y_name(&header.handle, "Google Drive not in the list?");
                clicked = header.clicked;
                if expanded {
                    spacer(ui, "drive-help-gap", 8.0);
                    ui.label("Install the Google Drive app, then in Files tap Browse → ⋯ → Edit and turn on Google Drive.", PARAGRAPH_13.color(SECONDARY));
                }
            },
        );
    });
    clicked
}

/// Swift's "Your music starts with a folder" message under the Albums/Artists header.
pub(super) fn empty_library(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    intent: &mut Option<Intent>,
) {
    let compact = ui.view_size().0 < compact::BREAKPOINT;
    if !compact {
        header_view(ui, catalog, state, width, intent);
    }
    let inset = if compact { 16.0 } else { 32.0 };
    ui.column(
        "empty-library-inset",
        Node {
            padding: Edges { l: inset, r: inset, t: 24.0, ..Edges::ZERO },
            ..Node::column().w(Size::Grow).h(Size::Fit)
        },
        |ui| {
            ui.label(LIBRARY_EMPTY, PARAGRAPH_14.color(SECONDARY));
        },
    );
}
