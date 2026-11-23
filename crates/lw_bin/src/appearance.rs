//! Parsed themes are retained; system changes select a value without parsing in the frame loop.
use lw_core::preferences::{Appearance, Settings, Signal};
use pxui::{ThemeHandle, Ui};
use pxui_sdl::PreferencesSnapshot as Snapshot;

fn prepare_theme(source: &str, accent: &str, dark: bool) -> Result<ThemeHandle, pxui::Error> {
    let mut text = String::with_capacity(source.len() + 80);
    // The source with the selected accent: its tokens, body names and part looks (the heading and popup hairlines); the kit
    // derives the other looks from the tokens
    for line in source.lines() {
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "tokens.accent" {
            text.push_str(line);
            text.push('\n');
        }
    }
    text.push_str("tokens.accent = ");
    text.push_str(accent);
    let mut root = ThemeHandle::parse(&text)?;
    lw_ui::apply_palette(&mut root, dark);
    root.animate_swap(false);
    Ok(root)
}

pub struct Themes {
    themes: [[ThemeHandle; 4]; 2],
    selected: Option<(bool, Signal)>,
}
impl Themes {
    pub fn new(ui: &mut Ui) -> Result<Self, pxui::Error> {
        lw_ui::commands::register(ui)?;
        Ok(Self {
            themes: [
                [
                    prepare_theme(lw_ui::LIGHT, "#2340FF", false)?,
                    prepare_theme(lw_ui::LIGHT, "#E3241B", false)?,
                    prepare_theme(lw_ui::LIGHT, "#00A86B", false)?,
                    prepare_theme(lw_ui::LIGHT, "#111111", false)?,
                ],
                [
                    prepare_theme(lw_ui::DARK, "#6F7DFF", true)?,
                    prepare_theme(lw_ui::DARK, "#E3241B", true)?,
                    prepare_theme(lw_ui::DARK, "#00A86B", true)?,
                    prepare_theme(lw_ui::DARK, "#EDEDED", true)?,
                ],
            ],
            selected: None,
        })
    }
    pub fn apply(&mut self, ui: &mut Ui, settings: &Settings, system: Snapshot) {
        let dark = match settings.appearance {
            Appearance::System => system.appearance == pxui::Appearance::Dark as u32,
            Appearance::Light => false,
            Appearance::Dark => true,
        };
        if self.selected != Some((dark, settings.signal)) {
            ui.set_theme(&self.themes[usize::from(dark)][settings.signal as usize])
                .unwrap_or_else(|error| panic!("LocalWave theme ({error}): {}", ui.last_error()));
            self.selected = Some((dark, settings.signal));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::FrameIn;
    #[test]
    fn cached_themes_follow_system_and_explicit_preferences_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = Ui::new(&pxui::UiDesc::DEFAULT).unwrap();
        let mut themes = Themes::new(&mut ui).unwrap();
        let cases = [
            (Appearance::System, 0, false),
            (Appearance::System, 1, false),
            (Appearance::System, 2, true),
            (Appearance::Light, 2, false),
            (Appearance::Dark, 1, true),
        ];
        for round in 0..5 {
            mem_sys::local_reset();
            for (appearance, system, dark) in cases {
                for signal in [Signal::Blue, Signal::Red, Signal::Green, Signal::Mono] {
                    let settings = Settings {
                        appearance,
                        signal,
                        ..Settings::default()
                    };
                    themes.apply(
                        &mut ui,
                        &settings,
                        Snapshot {
                            appearance: system,
                            ..Snapshot::default()
                        },
                    );
                    lw_ui::testing::frame(
                        &mut ui,
                        &{
                            let mut frame = FrameIn::new(320.0, 240.0)
                                .time(round as f64)
                                .reduced_motion(true);
                            frame.wire.appearance = system;
                            frame
                        },
                        &mut |_| {},
                    )
                    .unwrap();
                    assert_eq!(ui.tokens().is_dark(), dark);
                    assert!(ui.tokens().reduced_motion);
                    let expected = match signal {
                        Signal::Blue if dark => [0x6f, 0x7d, 0xff],
                        Signal::Blue => [0x23, 0x40, 0xff],
                        Signal::Red => [0xe3, 0x24, 0x1b],
                        Signal::Green => [0, 0xa8, 0x6b],
                        Signal::Mono if dark => [0xed; 3],
                        Signal::Mono => [0x11; 3],
                    };
                    // the theme files' part looks reach the app: the popup and table heading hairlines
                    let line = if dark {
                        pxui::geom::Rgba::rgb(0x26, 0x26, 0x26)
                    } else {
                        pxui::geom::Rgba::rgb(0xda, 0xda, 0xda)
                    };
                    assert_eq!(
                        (
                            ui.theme().popup.idle.border,
                            ui.theme().table_head.idle.border,
                            ui.theme().table_head.hover.border
                        ),
                        (line, line, line)
                    );
                    let accent = ui.tokens().accent;
                    assert_eq!([accent.r, accent.g, accent.b], expected);
                }
            }
            if round >= 2 {
                assert_eq!(
                    mem_sys::local_alloc_count(),
                    0,
                    "cached theme changes allocate nothing"
                );
            }
        }
    }
}
