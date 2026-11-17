# LocalWave codemod for DX1.12E: text inputs (run from the LocalWave worktree root).
import re,sys
base='crates/lw_ui/src/shell/'
def edit(p, pairs):
    s=open(base+p).read()
    for a,b in pairs:
        if a not in s: print("MISSING",p,a[:60]); continue
        s=s.replace(a,b)
    open(base+p,'w').write(s)
# ui.text_input_options() is the platform traits now
edit('commands.rs',[("            ui.text_input_options(),\n            pxui::TextField::SEARCH\n","            ui.text_input_options(),\n            pxui::TextTraits::SEARCH\n")])
edit('compact.rs',[("pxui::TextField::SEARCH.action(pxui::ReturnKey::Done)","pxui::TextTraits::SEARCH.action(pxui::ReturnKey::Done)")])
# scrubber: key + Scrubber style carries step and name
edit('transport.rs',[('''    let scrub = ui.scrub(
        "playback-position",
        position.min(f64::from(upper)) as f32,
        0.0..=upper,
        5.0,
    );''','''    let scrub = ui.scrub(
        "playback-position",
        position.min(f64::from(upper)) as f32,
        0.0..=upper,
        &SCRUBBER,
    );'''),('    ui.a11y_name(&scrubber, "Playback position");\n',''),
 ('pub(super) fn ','/// The playback scrubber: arrow keys move five seconds.\nconst SCRUBBER: pxui::Scrubber = pxui::Scrubber::DEFAULT.step(5.0).name("Playback position");\n\npub(super) fn ')])
# metadata: names constant per field, keys per field
s=open(base+'metadata.rs').read()
s=s.replace('''/// A metadata text input: Done ends editing; each is named by its label.
const META_FIELD: pxui::TextField = pxui::TextField::DEFAULT.action(pxui::ReturnKey::Done);

/// Label, field key. Swift edits disc numbers too; the sheet keeps them untouched.
const FIELDS: [(&str, &str); 6] = [
    ("Title", "meta-title"),
    ("Artist", "meta-artist"),
    ("Album", "meta-album"),
    ("Album artist", "meta-album-artist"),
    ("Release year", "meta-year"),
    ("Track number", "meta-track"),
];''','''/// A metadata text input: Done ends editing.
const META_FIELD: pxui::TextField = pxui::TextField::DEFAULT.action(pxui::ReturnKey::Done);

/// Label, field key, input. Swift edits disc numbers too; the sheet keeps them untouched.
const FIELDS: [(&str, &str, pxui::TextField); 6] = [
    ("Title", "meta-title", META_FIELD.name("Title")),
    ("Artist", "meta-artist", META_FIELD.name("Artist")),
    ("Album", "meta-album", META_FIELD.name("Album")),
    ("Album artist", "meta-album-artist", META_FIELD.name("Album artist")),
    ("Release year", "meta-year", META_FIELD.name("Release year")),
    ("Track number", "meta-track", META_FIELD.name("Track number")),
];''')
s=s.replace("for (index, (label, key)) in FIELDS.into_iter().enumerate()","for (index, (label, key, input)) in FIELDS.into_iter().enumerate()")
s=s.replace('.text_field("field", current.as_str(), &META_FIELD.name(label))','.text_field(key, current.as_str(), &input)')
open(base+'metadata.rs','w').write(s)
