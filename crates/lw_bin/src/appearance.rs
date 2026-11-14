//! Parsed themes are retained; system changes select a value without parsing in the frame loop.
use lw_apple_sys::environment::Snapshot;
use lw_core::preferences::{Appearance, Settings, Signal};
use pxui::{FrameIn, Theme, Ui};

pub struct Themes {
    themes: [[(Theme, Theme); 4]; 2],
    selected: Option<(bool, Signal)>,
}
impl Themes {
    pub fn new(ui: &mut Ui) -> Result<Self, String> {
        lw_ui::commands::register(ui)?;
        let parsed = |source, accent| -> Result<(Theme, Theme), String> {
            let make = || -> Result<Theme, String> {
                let mut theme = Theme::default();
                theme.apply(ui, source)?;
                theme.set("tokens.accent", accent)?;
                Ok(theme)
            };
            let root = make()?;
            let mut field = make()?;
            field.apply(ui, "text_field.body = localwave-command")?;
            Ok((root, field))
        };
        Ok(Self {
            themes: [
                [
                    parsed(lw_ui::LIGHT, "#2340FF")?,
                    parsed(lw_ui::LIGHT, "#E3241B")?,
                    parsed(lw_ui::LIGHT, "#00A86B")?,
                    parsed(lw_ui::LIGHT, "#111111")?,
                ],
                [
                    parsed(lw_ui::DARK, "#6F7DFF")?,
                    parsed(lw_ui::DARK, "#E3241B")?,
                    parsed(lw_ui::DARK, "#00A86B")?,
                    parsed(lw_ui::DARK, "#EDEDED")?,
                ],
            ],
            selected: None,
        })
    }
    pub fn apply(&mut self, ui: &mut Ui, settings: &Settings, system: Snapshot) {
        let dark = match settings.appearance {
            Appearance::System => system.appearance == FrameIn::APPEARANCE_DARK,
            Appearance::Light => false,
            Appearance::Dark => true,
        };
        if self.selected != Some((dark, settings.signal)) {
            ui.use_theme(&self.themes[usize::from(dark)][settings.signal as usize].0);
            self.selected = Some((dark, settings.signal));
        }
    }
    pub fn command_field(&self) -> Option<&Theme> {
        self.selected
            .map(|(dark, signal)| &self.themes[usize::from(dark)][signal as usize].1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_themes_follow_system_and_explicit_preferences_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = Ui::new().unwrap();
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
                    ui.frame(
                        &FrameIn {
                            width: 320.0,
                            height: 240.0,
                            time: round as f64,
                            appearance: system,
                            reduced_motion: 1,
                            ..FrameIn::default()
                        },
                        &mut |_| {},
                    )
                    .unwrap();
                    assert_eq!(ui.tokens().foreground.r > 128, dark);
                    assert!(ui.tokens().reduced_motion);
                    let expected = match signal {
                        Signal::Blue if dark => [0x6f, 0x7d, 0xff],
                        Signal::Blue => [0x23, 0x40, 0xff],
                        Signal::Red => [0xe3, 0x24, 0x1b],
                        Signal::Green => [0, 0xa8, 0x6b],
                        Signal::Mono if dark => [0xed; 3],
                        Signal::Mono => [0x11; 3],
                    };
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
