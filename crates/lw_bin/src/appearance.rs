//! Parsed themes are retained; system changes select a value without parsing in the frame loop.
use lw_core::preferences::{Appearance, Settings, Signal};
use pxui::{ThemeHandle, Ui};
use pxui_sdl::PreferencesSnapshot as Snapshot;

fn prepare_theme_pair(source: &str, accent: &str, dark: bool) -> Result<(ThemeHandle, ThemeHandle), pxui::Error> {
    let mut text = String::with_capacity(source.len() + 80);
    // Deriving looks from the selected accent keeps the source's tokens and body names.
    for line in source.lines() {
        let Some((key, _)) = line.split_once('=') else { continue };
        if key.trim().starts_with("tokens.") || key.trim().ends_with(".body") {
            text.push_str(line);
            text.push('\n');
        }
    }
    text.push_str("tokens.accent = ");
    text.push_str(accent);
    let mut root = ThemeHandle::parse(&text)?;
    text.push_str("\ntext_field.body = localwave-command");
    let mut field = ThemeHandle::parse(&text)?;
    lw_ui::apply_palette(&mut root, dark);
    lw_ui::apply_palette(&mut field, dark);
    Ok((root, field))
}

pub struct Themes {
    themes: [[(ThemeHandle, ThemeHandle); 4]; 2],
    selected: Option<(bool, Signal)>,
}
impl Themes {
    pub fn new(ui: &mut Ui) -> Result<Self, pxui::Error> {
        lw_ui::commands::register(ui)?;
        Ok(Self {
            themes: [
                [
                    prepare_theme_pair(lw_ui::LIGHT, "#2340FF", false)?,
                    prepare_theme_pair(lw_ui::LIGHT, "#E3241B", false)?,
                    prepare_theme_pair(lw_ui::LIGHT, "#00A86B", false)?,
                    prepare_theme_pair(lw_ui::LIGHT, "#111111", false)?,
                ],
                [
                    prepare_theme_pair(lw_ui::DARK, "#6F7DFF", true)?,
                    prepare_theme_pair(lw_ui::DARK, "#E3241B", true)?,
                    prepare_theme_pair(lw_ui::DARK, "#00A86B", true)?,
                    prepare_theme_pair(lw_ui::DARK, "#EDEDED", true)?,
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
            ui.set_theme(&self.themes[usize::from(dark)][settings.signal as usize].0)
                .unwrap_or_else(|error| panic!("LocalWave theme ({error}): {}", ui.last_error()));
            self.selected = Some((dark, settings.signal));
        }
    }
    pub fn command_field(&self) -> Option<&ThemeHandle> {
        self.selected
            .map(|(dark, signal)| &self.themes[usize::from(dark)][signal as usize].1)
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
                            reduced_motion: 0,
                        },
                    );
                    lw_ui::testing::frame(
                        &mut ui,
                        &{
                            let mut frame = FrameIn::new(320.0, 240.0).time(round as f64).reduced_motion(true);
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
                    assert_eq!(ui.theme().popup.idle.border, ui.tokens().edge());
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
