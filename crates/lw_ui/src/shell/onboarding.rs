//! Port of WaveOnboardingView for a library with no sources, plus the empty-library page and the
//! Find hint. All strings are static; the screen is one retained view keyed by its width.
//! Deferred against Swift: the animated MatrixText counter (a static "00"), the iOS-only Google
//! Drive disclosure and the demo-library installer ("Explore the library" just leaves onboarding).
use super::*;

/// Swift's `didStart` gate: shown until a source exists, the library has songs or it is explored.
pub(super) fn wanted(catalog: &Catalog, state: &ShellState) -> bool {
    catalog.sources == Some(0) && catalog.is_empty() && !state.explored
}

pub(super) const LIBRARY_EMPTY: &str =
    "Your music starts with a folder. Add music in Folders to build your library.";
pub(super) const FIND_HINT: &str = "Search spans every source, including folders that are not in an album. Add a word like flac or cloud to filter.";

pub(super) fn view(ui: &mut Ui, p: Palette, intent: &mut Option<Intent>) -> Handle {
    let (width, _) = ui.view_size();
    let column = (width - 32.0).clamp(1.0, 560.0);
    let root = container(ui, "onboarding", Axis::Column, Size::Grow, Size::Grow, p.paper);
    ui.set_role(&root, Role::Window, 0);
    ui.a11y_name(&root, "LocalWave");
    ui.within(&root, &mut |ui| {
        ui.view(
            "onboarding-page",
            pxui::hash!(column.to_bits()),
            Size::Grow,
            Size::Grow,
            &mut |ui| {
                let outer = ui.node(
                    "onboarding-outer",
                    Node {
                        w: Size::Grow,
                        h: Size::Grow,
                        align: Align::Center,
                        padding: Edges { t: 16.0, b: 34.0, ..Edges::ZERO },
                        ..Node::DEFAULT
                    },
                );
                ui.within(&outer, &mut |ui| page(ui, column, p, intent));
            },
        );
    });
    root
}

fn page(ui: &mut Ui, column: f32, p: Palette, intent: &mut Option<Intent>) {
    let page = ui.node(
        "onboarding-column",
        Node { w: Size::Px(column), h: Size::Grow, ..Node::DEFAULT },
    );
    ui.within(&page, &mut |ui| {
        let top = container(ui, "brand", Axis::Row, Size::Grow, Size::Px(18.2), p.paper);
        ui.within(&top, &mut |ui| {
            text(ui, "name", "LocalWave", 13, p.secondary, (Size::Grow, 18.2), false);
            text(ui, "version", "v2", 12, p.secondary, (Size::Px(24.0), 16.8), true);
        });
        spacer(ui, "count-gap", 28.0);
        text(ui, "count", "00", 72, p.ink, (Size::Grow, 79.0), true);
        spacer(ui, "count-label-gap", 8.0);
        text(ui, "count-label", "songs so far", 15, p.secondary, (Size::Grow, 21.0), false);
        spacer(ui, "heading-gap", 28.0);
        text(ui, "heading-1", "Point it at a folder.", 32, p.ink, (Size::Grow, 36.8), false);
        text(ui, "heading-2", "It stays in sync.", 32, p.ink, (Size::Grow, 36.8), false);
        spacer(ui, "promise-gap", 8.0);
        settings::paragraph(
            ui,
            "promise",
            "Your files stay where they are. Nothing is uploaded.",
            column,
            15.0,
            p.secondary,
        );
        spacer(ui, "steps-gap", 28.0);
        rule(ui, "steps-rule", false, p);
        for (key, number, line) in [
            ("step-1", "1", "Choose a folder in Files"),
            ("step-2", "2", "LocalWave indexes it in the background"),
            ("step-3", "3", "Play at once; cloud songs fetch on play"),
        ] {
            let row = container(ui, key, Axis::Row, Size::Grow, Size::Px(46.0), p.paper);
            ui.within(&row, &mut |ui| {
                text(ui, "number", number, 13, p.secondary, (Size::Px(26.0), 46.0), false);
                text(ui, "line", line, 15, p.ink, (Size::Grow, 46.0), false);
            });
            rule(ui, "step-rule", false, p);
        }
        ui.box_("onboarding-flex", BoxOpts::default().h(Size::Grow));
        settings::paragraph(
            ui,
            "sources-note",
            "iCloud Drive, Google Drive, On My iPhone, USB, SMB — anything in Files.",
            column,
            13.0,
            p.secondary,
        );
        spacer(ui, "choose-gap", 12.0);
        let choose = ui.node(
            "choose-folder",
            Node {
                axis: Axis::Row,
                align: Align::Center,
                flags: CLICKABLE | TAB_STOP,
                role: Role::Button,
                w: Size::Grow,
                h: Size::Px(52.0),
                padding: Edges { l: 16.0, r: 16.0, ..Edges::ZERO },
                look: Look { fill: p.ink, text: p.paper, font: Font { size: 15, ..Font::BODY }, ..Look::NONE },
                ..Node::DEFAULT
            },
        );
        ui.a11y_name(&choose, "Choose a folder");
        ui.text(&choose, "Choose a folder");
        if ui.signal_state(&choose).clicked {
            *intent = Some(Intent::ChooseFolder);
        }
        let links = container(ui, "links", Axis::Row, Size::Grow, Size::Px(44.0), p.paper);
        ui.within(&links, &mut |ui| {
            for (key, label, action) in [
                ("pick-files", "Pick some files", Intent::ChooseAudio),
                ("explore", "Explore the library", Intent::Navigate(Route::Albums)),
            ] {
                let link = ui.node(
                    key,
                    Node {
                        flags: CLICKABLE | TAB_STOP,
                        role: Role::Button,
                        w: Size::Grow,
                        h: Size::Px(44.0),
                        look: Look { text: p.ink, font: Font { size: 15, ..Font::BODY }, ..Look::NONE },
                        ..Node::DEFAULT
                    },
                );
                ui.a11y_name(&link, label);
                ui.text(&link, label);
                if ui.signal_state(&link).clicked {
                    *intent = Some(action);
                }
            }
        });
    });
}

/// Swift's "Your music starts with a folder" message under the Albums/Artists header.
pub(super) fn empty_library(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    width: f32,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let compact = ui.view_size().0 < compact::BREAKPOINT;
    if !compact {
        header_view(ui, catalog, state, width, p, intent);
    }
    let inset = if compact { 16.0 } else { 32.0 };
    let page = ui.node(
        "empty-library-inset",
        Node {
            w: Size::Grow,
            h: Size::Fit,
            padding: Edges { l: inset, r: inset, t: 24.0, ..Edges::ZERO },
            ..Node::DEFAULT
        },
    );
    ui.within(&page, &mut |ui| {
        settings::paragraph(ui, "empty-library", LIBRARY_EMPTY, width - inset * 2.0, 14.0, p.secondary);
    });
}
