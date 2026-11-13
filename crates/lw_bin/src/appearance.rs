//! Parsed themes are retained; system changes select a value without parsing in the frame loop.
use lw_apple_sys::environment::Snapshot;
use lw_core::preferences::{Appearance, Settings};
use pxui::{FrameIn, Theme, Ui};

pub struct Themes {
    light: Theme,
    dark: Theme,
    selected: Option<bool>,
}
impl Themes {
    pub fn new(ui: &Ui) -> Result<Self, String> {
        let mut light = Theme::default();
        light.apply(ui, lw_ui::LIGHT)?;
        let mut dark = Theme::default();
        dark.apply(ui, lw_ui::DARK)?;
        Ok(Self {
            light,
            dark,
            selected: None,
        })
    }
    pub fn apply(&mut self, ui: &mut Ui, settings: &Settings, system: Snapshot) {
        let dark = match settings.appearance {
            Appearance::System => system.appearance == FrameIn::APPEARANCE_DARK,
            Appearance::Light => false,
            Appearance::Dark => true,
        };
        if self.selected != Some(dark) {
            ui.use_theme(if dark { &self.dark } else { &self.light });
            self.selected = Some(dark);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_themes_follow_system_and_explicit_preferences_without_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let mut ui = Ui::new().unwrap();
        let mut themes = Themes::new(&ui).unwrap();
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
                let settings = Settings {
                    appearance,
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
