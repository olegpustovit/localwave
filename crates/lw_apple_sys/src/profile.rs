//! Swift-compatible native keys; application models live in lw_core.
use crate::bookmarks::failure;
use crate::preferences::Preferences;
use std::ffi::{CString, c_char, c_void};
use std::{io, ptr};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingValues {
    pub download_ahead: i32,
    pub show_non_audio: i32,
    pub no_network: i32,
    pub appearance: i32,
    pub signal: i32,
    pub reduce_motion: i32,
    pub cache_gb: i64,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerValues {
    pub song_ids: Vec<i64>,
    pub current_index: i64,
    pub volume: Option<f32>,
    pub context_data: Option<Vec<u8>>,
}
unsafe extern "C" {
    fn lw_preferences_save_player_index(raw: *mut c_void, index: i64);
    fn lw_preferences_save_player_volume(raw: *mut c_void, volume: f32);
    fn lw_preferences_for_ui_test(test_id: *const c_char) -> *mut c_void;
    #[cfg(test)]
    fn lw_preferences_is_standard(raw: *mut c_void) -> i32;
    fn lw_preferences_settings(raw: *mut c_void, settings: *mut SettingValues);
    fn lw_preferences_save_settings(raw: *mut c_void, settings: *const SettingValues);
    fn lw_preferences_player(raw: *mut c_void, error: *mut *mut c_char) -> *mut c_void;
    fn lw_player_preferences_count(raw: *mut c_void) -> usize;
    fn lw_player_preferences_song(raw: *mut c_void, index: usize) -> i64;
    fn lw_player_preferences_index(raw: *mut c_void) -> i64;
    fn lw_player_preferences_volume(raw: *mut c_void) -> f64;
    fn lw_player_preferences_context(
        raw: *mut c_void,
        len: *mut usize,
        present: *mut i32,
    ) -> *const u8;
    fn lw_preferences_save_player(
        raw: *mut c_void,
        queue: *const i64,
        count: usize,
        index: i64,
        volume: f64,
        context: *const u8,
        context_len: usize,
    );
    fn lw_preferences_release(raw: *mut c_void);
}
struct Snapshot(*mut c_void);
impl Drop for Snapshot {
    fn drop(&mut self) {
        unsafe { lw_preferences_release(self.0) };
    }
}
impl Preferences {
    pub fn save_player_index(&self, index: i64) -> io::Result<()> {
        if index < 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        unsafe { lw_preferences_save_player_index(self.raw(), index) };
        Ok(())
    }
    pub fn save_player_volume(&self, volume: f32) -> io::Result<()> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        unsafe { lw_preferences_save_player_volume(self.raw(), volume) };
        Ok(())
    }
    pub fn current() -> io::Result<Self> {
        let test = if cfg!(debug_assertions) {
            std::env::var("LOCALWAVE_UI_TEST_ID").ok()
        } else {
            None
        };
        Self::for_ui_test(test.as_deref())
    }
    pub fn for_ui_test(test_id: Option<&str>) -> io::Result<Self> {
        let test = test_id
            .map(CString::new)
            .transpose()
            .map_err(io::Error::other)?;
        let raw = unsafe {
            lw_preferences_for_ui_test(test.as_ref().map_or(ptr::null(), |value| value.as_ptr()))
        };
        Self::from_raw(raw)
    }
    pub fn load_settings(&self) -> SettingValues {
        let _guard = self.cache_guard();
        let mut values = SettingValues::default();
        unsafe { lw_preferences_settings(self.raw(), &mut values) };
        values
    }
    pub fn save_settings(&self, settings: &SettingValues) -> io::Result<()> {
        if !(0..=2).contains(&settings.appearance)
            || !(0..=3).contains(&settings.signal)
            || ![2, 4, 8, 16, 32].contains(&settings.cache_gb)
            || [
                settings.download_ahead,
                settings.show_non_audio,
                settings.no_network,
                settings.reduce_motion,
            ]
            .iter()
            .any(|value| !(0..=1).contains(value))
        {
            return Err(io::Error::other("Invalid native settings"));
        }
        let _guard = self.cache_guard();
        unsafe { lw_preferences_save_settings(self.raw(), settings) };
        Ok(())
    }
    pub fn load_player(&self) -> io::Result<PlayerValues> {
        let _guard = self.cache_guard();
        let mut error = ptr::null_mut();
        let snapshot = Snapshot(unsafe { lw_preferences_player(self.raw(), &mut error) });
        if snapshot.0.is_null() {
            return Err(failure(error));
        }
        let count = unsafe { lw_player_preferences_count(snapshot.0) };
        let mut ids = Vec::new();
        ids.try_reserve_exact(count).map_err(io::Error::other)?;
        for i in 0..count {
            ids.push(unsafe { lw_player_preferences_song(snapshot.0, i) });
        }
        let volume = unsafe { lw_player_preferences_volume(snapshot.0) };
        let (mut len, mut present) = (0, 0);
        let data = unsafe { lw_player_preferences_context(snapshot.0, &mut len, &mut present) };
        let context_data = if present == 0 {
            None
        } else if len == 0 {
            Some(vec![])
        } else {
            if data.is_null() || len > 64 * 1024 * 1024 {
                return Err(io::Error::other("Invalid native playback context"));
            }
            Some(unsafe { std::slice::from_raw_parts(data, len) }.to_vec())
        };
        Ok(PlayerValues {
            song_ids: ids,
            current_index: unsafe { lw_player_preferences_index(snapshot.0) },
            volume: volume.is_finite().then(|| (volume as f32).clamp(0.0, 1.0)),
            context_data,
        })
    }
    pub fn save_player(
        &self,
        song_ids: &[i64],
        current_index: i64,
        volume: Option<f32>,
        context_data: Option<&[u8]>,
    ) -> io::Result<()> {
        if song_ids.len() > 1_000_000
            || current_index < 0
            || (!song_ids.is_empty() && current_index as usize >= song_ids.len())
            || (song_ids.is_empty() && current_index != 0)
            || volume.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
            || context_data.is_some_and(|data| data.len() > 64 * 1024 * 1024)
        {
            return Err(io::Error::other("Invalid native player state"));
        }
        let (data, len) = context_data.map_or((ptr::null(), 0), |data| (data.as_ptr(), data.len()));
        let _guard = self.cache_guard();
        unsafe {
            lw_preferences_save_player(
                self.raw(),
                song_ids.as_ptr(),
                song_ids.len(),
                current_index,
                volume.map_or(f64::NAN, f64::from),
                data,
                len,
            )
        };
        Ok(())
    }
}

/// # Safety
/// `suite` is NUL-terminated and `context` is readable for `len` bytes during this call.
#[cfg(feature = "interop-test")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lw_preferences_interop_fixture(
    suite: *const c_char,
    context: *const u8,
    len: usize,
) -> i32 {
    if suite.is_null() || context.is_null() || len > 64 * 1024 * 1024 {
        return -1;
    }
    let Ok(suite) = unsafe { std::ffi::CStr::from_ptr(suite) }.to_str() else {
        return -1;
    };
    let Ok(preferences) = Preferences::open(Some(suite)) else {
        return -1;
    };
    let Ok(state) = preferences.load_player() else {
        return -2;
    };
    let bytes = unsafe { std::slice::from_raw_parts(context, len) };
    if state.song_ids != [i64::MAX, 22, 22, -1]
        || state.current_index != 2
        || state.volume != Some(0.625)
        || state.context_data.as_deref() != Some(bytes)
    {
        return -3;
    }
    let settings = preferences.load_settings();
    if settings.appearance != 1
        || settings.signal != 2
        || settings.cache_gb != 4
        || settings.download_ahead != 0
    {
        return -4;
    }
    if preferences
        .save_player(&[22, i64::MAX, 22], 2, Some(0.0), Some(bytes))
        .is_err()
    {
        return -5;
    }
    if preferences
        .save_settings(&SettingValues {
            appearance: 2,
            signal: 3,
            cache_gb: 16,
            reduce_motion: 1,
            ..settings
        })
        .is_err()
    {
        return -6;
    }
    if preferences.save_player_index(1).is_err() || preferences.save_player_volume(0.5).is_err() {
        return -7;
    }
    let Ok(updated) = preferences.load_player() else {
        return -8;
    };
    if updated.song_ids != [22, i64::MAX, 22]
        || updated.current_index != 1
        || updated.volume != Some(0.5)
        || updated.context_data.as_deref() != Some(bytes)
    {
        return -9;
    }
    if preferences.save_player_index(-1).is_ok()
        || preferences.save_player_volume(f32::NAN).is_ok()
        || preferences.save_player_index(2).is_err()
        || preferences.save_player_volume(0.0).is_err()
    {
        return -10;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ui_test_profiles_are_isolated_and_player_values_preserve_duplicates_and_context_bytes() {
        let nonce = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64)
            & 0xfffffffffffe;
        let first_id = format!("00000000-0000-4000-8000-{nonce:012x}");
        let second_id = format!("00000000-0000-4000-8000-{:012x}", nonce + 1);
        let first = Preferences::for_ui_test(Some(&first_id)).unwrap();
        let second = Preferences::for_ui_test(Some(&second_id)).unwrap();
        let standard = Preferences::for_ui_test(None).unwrap();
        let invalid = Preferences::for_ui_test(Some("not-a-test-uuid")).unwrap();
        assert_ne!(unsafe { lw_preferences_is_standard(standard.raw()) }, 0);
        assert_ne!(unsafe { lw_preferences_is_standard(invalid.raw()) }, 0);
        let before = standard.load_player().unwrap();
        let saved = PlayerValues {
            song_ids: vec![i64::MAX, 2, 2, -1],
            current_index: 2,
            volume: Some(0.375),
            context_data: Some(vec![0, 255, 1, 0]),
        };
        first
            .save_player(
                &saved.song_ids,
                saved.current_index,
                saved.volume,
                saved.context_data.as_deref(),
            )
            .unwrap();
        second.save_player(&[42], 0, None, None).unwrap();
        assert_eq!(first.load_player().unwrap(), saved);
        assert_eq!(second.load_player().unwrap().song_ids, [42]);
        assert_eq!(standard.load_player().unwrap(), before);
        let mut settings = first.load_settings();
        settings.appearance = 2;
        settings.signal = 3;
        settings.download_ahead = 0;
        settings.reduce_motion = 1;
        settings.cache_gb = 16;
        first.save_settings(&settings).unwrap();
        assert_eq!(first.load_settings(), settings);
        let reopened = Preferences::for_ui_test(Some(&first_id)).unwrap();
        assert_eq!(reopened.load_player().unwrap(), saved);
        first.save_player(&[], 0, None, None).unwrap();
        second.save_player(&[], 0, None, None).unwrap();
        first
            .save_settings(&SettingValues {
                download_ahead: 1,
                cache_gb: 8,
                ..SettingValues::default()
            })
            .unwrap();
    }
}
