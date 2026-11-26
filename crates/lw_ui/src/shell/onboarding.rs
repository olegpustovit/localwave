//! Port of WaveOnboardingView for a library with no sources, plus the empty-library page and the
//! Find hint. All strings are static; the screen is one retained view keyed by its width and the
//! Google Drive disclosure. Deferred against Swift: the live song count (onboarding only shows for an
//! empty library, so it is always "00").
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

/// Shorter windows (a phone in landscape) scroll the page.
const SHORT: f32 = 640.0;

pub(super) fn view(ui: &mut Ui, top_inset: f32, demo: bool, intent: &mut Option<Intent>) -> Handle {
    let pxui::ViewSize { width, height } = ui.view_size();
    let [_, safe_right, safe_bottom, safe_left] = ui.environment().safe_area;
    let column = (width - 32.0 - safe_left - safe_right).max(1.0);
    let root = container(
        ui,
        "onboarding",
        Axis::Column,
        Size::Grow,
        Size::Grow,
        &PAPER_FILL,
    );
    ui.set_role(&root, Role::Window, pxui::A11yStates::NONE);
    ui.a11y_name(&root, "LocalWave");
    // The disclosure state lives on the root so the retained page rebuilds when it flips.
    let drive = ui.state(&root, 0) != 0;
    let padding = Edges::new(
        16.0 + safe_left,
        16.0 + top_inset,
        16.0 + safe_right,
        34.0f32.max(safe_bottom),
    );
    let key = (
        column.to_bits(),
        height.to_bits(),
        top_inset.to_bits(),
        safe_left.to_bits(),
        safe_right.to_bits(),
        safe_bottom.to_bits(),
        drive,
        demo,
    );
    let scrolls = height < SHORT;
    let mut body = |ui: &mut Ui, node: Node| {
        let mut content = |ui: &mut Ui| {
            ui.column(node.padding(padding), |ui| {
                if page(ui, width < compact::BREAKPOINT, scrolls, drive, demo, intent) {
                    ui.set_state(&root, i64::from(!drive));
                }
            });
        };
        // shortcut: a retained view inside the scroll view stops finger scrolling (the wheel still works), so the
        // landscape page rebuilds each frame; retain it again once pxui scrolls retained content by touch.
        if scrolls {
            content(ui);
        } else {
            ui.view_of("onboarding-page", &key, |ui| content(ui));
        }
    };
    ui.within(&root, &mut |ui| {
        // A phone in landscape is shorter than the page, so it scrolls there; taller windows keep the plain page, its flex
        // spacer dropping the keys to the bottom.
        if scrolls {
            ui.scroll_view("scroll", Node::DEFAULT.w(Size::Grow).h(Size::Grow), |ui| {
                body(ui, Node::column().w(Size::Grow).h(Size::Fit).min_h(height));
            });
        } else {
            body(ui, Node::column().w(Size::Grow).h(Size::Grow));
        }
    });
    root
}

/// An underlined text link (`localwave-button` / `LINK`): as wide as its text, 44 points high, no look of its own.
/// Swift's `underlined`: the 15-point ink text over a hairline-coloured rule.
const UNDERLINED: pxui::Button = pxui::Button::DEFAULT
    .variant(variant::LINK)
    .size(Size::Fit, Size::Px(44.0))
    .centered()
    .pad(Edges::ZERO)
    .label(LEAD_INK)
    .look(&LINE_RULE);
/// The primary key (`localwave-button` / `PRIMARY`): paper 15-point text centred on ink across the page, 52 points high.
/// Swift's headline: 32 points medium, tracked -1.12 (-0.035 em), in 36.8-point lines.
const HEADLINE: Label = TEXT_32
    .weight(500)
    .tracking(-0.035)
    .grow()
    .h_px(36.8)
    .color(INK);
/// Swift's Choose a folder: the 20-point folder and the 15-point medium label 8 apart, centred on ink.
const CHOOSE: pxui::Button = pxui::Button::DEFAULT
    .variant(variant::PRIMARY)
    .size(Size::Grow, Size::Px(52.0))
    .icon(icon_sized(Icon::Folder, 20.0))
    .icon_size(20.0)
    .gap(8.0)
    .label(TEXT_15.color(PAPER).weight(500))
    .look(&INK_LOOKS);
/// The Google Drive disclosure (`localwave-disclosure`): the 1-pixel sliced triangle and the 13-point title.
const DRIVE: pxui::Disclosure = pxui::Disclosure::DEFAULT;
const DRIVE_HELP: &str =
    "Install the Google Drive app, then in Files tap Browse → ⋯ → Edit and turn on Google Drive.";

/// Returns true when the Google Drive disclosure was toggled.
/// `scrolls`: the page sits in a scroll view, so it takes its content's height (a growing page would be exactly as tall as
/// the window, leaving nothing to scroll).
fn page(
    ui: &mut Ui,
    compact: bool,
    scrolls: bool,
    drive: bool,
    demo: bool,
    intent: &mut Option<Intent>,
) -> bool {
    let mut toggled = false;
    ui.column(Node::column().w(Size::Grow).h(if scrolls { Size::Fit } else { Size::Grow }), |ui| {
        let top = container(
            ui,
            "brand",
            Axis::Row,
            Size::Grow,
            Size::Px(18.2),
            &PAPER_FILL,
        );
        ui.within(&top, &mut |ui| {
            ui.label("LocalWave", CAPTION.grow());
            ui.label("v2", MONO_DIM.w_px(16.0));
        });
        spacer(ui, "count-gap", 28.0);
        let count = container(
            ui,
            "count-row",
            Axis::Row,
            Size::Grow,
            Size::Px(79.0),
            &PAPER_FILL,
        );
        ui.set_role(&count, Role::Label, pxui::A11yStates::NONE);
        ui.a11y_name(&count, "0 songs so far");
        ui.within(&count, &mut |ui| {
            clock::matrix_digits(ui, "count", b"00", 79.0, INK)
        });
        spacer(ui, "count-label-gap", 8.0);
        ui.label("songs so far", LEAD_DIM.grow());
        spacer(ui, "heading-gap", 28.0);
        let headline = if compact {
            HEADLINE.line_height(36.8)
        } else {
            HEADLINE
        };
        ui.label("Point it at a folder.", headline);
        ui.label("It stays in sync.", headline);
        spacer(ui, "promise-gap", 8.0);
        ui.label(
            "Your files stay where they are. Nothing is uploaded.",
            PARAGRAPH_15.color(SECONDARY),
        );
        spacer(ui, "steps-gap", 28.0);
        rule(ui, "steps-rule", false);
        for (key, number, line) in [
            ("step-1", "1", "Choose a folder in Files"),
            ("step-2", "2", "LocalWave indexes it in the background"),
            ("step-3", "3", "Play at once; cloud songs fetch on play"),
        ] {
            if compact {
                ui.id(key, |ui| {
                    ui.row(
                        Node::row()
                            .w(Size::Grow)
                            .h(Size::Fit)
                            .min_h(45.0)
                            .pad_y(12.0)
                            .gap(14.0),
                        |ui| {
                            ui.label(number, TEXT_13.w_px(12.0).color(SECONDARY));
                            ui.label(line, PARAGRAPH_15.color(INK));
                        },
                    );
                });
            } else {
                let row = container(ui, key, Axis::Row, Size::Grow, Size::Px(46.0), &PAPER_FILL);
                ui.within(&row, &mut |ui| {
                    ui.label(number, TEXT_13.w_px(26.0).h_px(46.0).color(SECONDARY));
                    ui.label(line, TEXT_15.grow().h_px(46.0).color(INK));
                });
            }
            rule(ui, "step-rule", false);
        }
        // a growing node inside a fitted page would pin it to the window height: a fixed gap while scrolling
        ui.node("onboarding-flex", Node::DEFAULT.h(if scrolls { Size::Px(24.0) } else { Size::Grow }));
        ui.label(
            "iCloud Drive, Google Drive, On My iPhone, USB, SMB — anything in Files.",
            PARAGRAPH_13.color(SECONDARY),
        );
        spacer(ui, "choose-gap", 12.0);
        if ui
            .button("choose-folder", "Choose a folder", &CHOOSE)
            .clicked
        {
            *intent = Some(Intent::ChooseFolder);
        }
        spacer(ui, "links-gap", 4.0);
        let links = container(
            ui,
            "links",
            Axis::Row,
            Size::Grow,
            Size::Px(44.0),
            &PAPER_FILL,
        );
        ui.within(&links, &mut |ui| {
            if ui
                .button("pick-files", "Pick some files", &UNDERLINED)
                .clicked
            {
                *intent = Some(Intent::ChooseAudio);
            }
            ui.node("links-flex", Node::DEFAULT.w(Size::Grow));
            // Swift offers its bundled demo while the library is empty (onboarding shows only then)
            let label = if demo {
                "Try the demo library"
            } else {
                "Explore the library"
            };
            if ui.button("explore", label, &UNDERLINED).clicked {
                *intent = Some(if demo {
                    Intent::TryDemo
                } else {
                    Intent::Navigate(Route::Albums)
                });
            }
        });
        spacer(ui, "drive-gap", 4.0);
        // Swift's "Google Drive not in the list?" disclosure in its bordered 1px frame
        let frame = ui.node(
            "drive-frame",
            Node::column()
                .w(Size::Grow)
                .h(Size::Fit)
                .pad(1.0)
                .look(LINE_FILL),
        );
        ui.within(&frame, &mut |ui| {
            ui.column(
                Node::column()
                    .w(Size::Grow)
                    .h(Size::Fit)
                    .pad(14.0)
                    .look(PAPER_FILL),
                |ui| {
                    toggled = ui
                        .disclosure(
                            "drive-toggle",
                            "Google Drive not in the list?",
                            drive,
                            &DRIVE,
                            &mut |ui| {
                                spacer(ui, "drive-help-gap", 8.0);
                                ui.label(DRIVE_HELP, PARAGRAPH_13.color(SECONDARY));
                            },
                        )
                        .changed;
                },
            );
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
            let (w, h) = if expanded {
                (25.0 / 3.0, 7.0)
            } else {
                (7.0, 25.0 / 3.0)
            };
            let top = rect.y + 14.0 - h;
            for i in 0..h as i32 {
                let t = (i as f32 + 0.5) / h;
                let span = if expanded {
                    w * (1.0 - t)
                } else {
                    w * (1.0 - (2.0 * t - 1.0).abs())
                };
                let x = if expanded {
                    rect.x + (w - span) / 2.0
                } else {
                    rect.x
                };
                paint.fill(
                    Rect::new(x, top + i as f32, span, 1.0),
                    Fill::new(ink).fractional(),
                );
            }
        });
        ui.label(
            c.text_of(&c.label),
            TEXT_13.grow().h_px(19.5).color(INK).role(Role::Unknown),
        );
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
    ui.column(
        Node::column()
            .w(Size::Grow)
            .h(Size::Fit)
            .padding(Edges::ZERO.l(inset).r(inset).t(24.0)),
        |ui| {
            ui.label(LIBRARY_EMPTY, PARAGRAPH_14.color(SECONDARY));
        },
    );
}
