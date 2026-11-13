use super::LibraryWorker;
use lw_apple_sys::profile::SettingValues;
use lw_core::context::PlaybackContext;
use lw_core::preferences::{Appearance, PlayerState, Settings, Signal};
use std::io;

impl LibraryWorker {
    pub fn playback_context(&self, data: Option<&[u8]>) -> PlaybackContext {
        let Some(value) = data.and_then(|data| lw_apple_sys::context::decode(data).ok()) else {
            return PlaybackContext::Library;
        };
        match value.kind {
            1 => PlaybackContext::Album {
                name: value.name,
                artist: value.artist,
            },
            2 => PlaybackContext::Artist(value.name),
            3 => PlaybackContext::Playlist {
                id: value.id,
                name: value.name,
            },
            4 => PlaybackContext::Folder {
                source_id: value.id,
                path_id: value.path_id,
                name: value.name,
            },
            5 => PlaybackContext::Automatic(value.name),
            6 => PlaybackContext::Selection(value.name),
            _ => PlaybackContext::Library,
        }
    }
    pub fn playback_context_data(&self, context: &PlaybackContext) -> io::Result<Vec<u8>> {
        use lw_apple_sys::context::encode;
        match context {
            PlaybackContext::Library => encode(0, "", "", 0, None),
            PlaybackContext::Album { name, artist } => encode(1, name, artist, 0, None),
            PlaybackContext::Artist(name) => encode(2, name, "", 0, None),
            PlaybackContext::Playlist { id, name } => encode(3, name, "", *id, None),
            PlaybackContext::Folder {
                source_id,
                path_id,
                name,
            } => encode(4, name, "", *source_id, *path_id),
            PlaybackContext::Automatic(name) => encode(5, name, "", 0, None),
            PlaybackContext::Selection(name) => encode(6, name, "", 0, None),
        }
    }
    pub fn save_player_index(&self, index: i64) -> io::Result<()> {
        self.preferences.save_player_index(index)
    }
    pub fn save_player_volume(&self, volume: f32) -> io::Result<()> {
        self.preferences.save_player_volume(volume)
    }
    pub fn load_settings(&self) -> Settings {
        let values = self.preferences.load_settings();
        Settings {
            download_ahead: values.download_ahead != 0,
            show_non_audio: values.show_non_audio != 0,
            simulate_no_network: values.no_network != 0,
            reduce_motion: values.reduce_motion != 0,
            cache_gb: values.cache_gb,
            appearance: match values.appearance {
                1 => Appearance::Light,
                2 => Appearance::Dark,
                _ => Appearance::System,
            },
            signal: match values.signal {
                1 => Signal::Red,
                2 => Signal::Green,
                3 => Signal::Mono,
                _ => Signal::Blue,
            },
        }
    }
    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        self.preferences.save_settings(&SettingValues {
            download_ahead: settings.download_ahead as i32,
            show_non_audio: settings.show_non_audio as i32,
            no_network: settings.simulate_no_network as i32,
            reduce_motion: settings.reduce_motion as i32,
            cache_gb: settings.cache_gb,
            appearance: settings.appearance as i32,
            signal: settings.signal as i32,
        })
    }
    pub fn load_player_state(&self) -> io::Result<PlayerState> {
        let state = self.preferences.load_player()?;
        Ok(PlayerState {
            song_ids: state.song_ids,
            current_index: state.current_index,
            volume: state.volume,
            context_data: state.context_data,
        })
    }
    pub fn save_player_state(&self, state: &PlayerState) -> io::Result<()> {
        state.validate()?;
        self.preferences.save_player(
            &state.song_ids,
            state.current_index,
            state.volume,
            state.context_data.as_deref(),
        )
    }
}
