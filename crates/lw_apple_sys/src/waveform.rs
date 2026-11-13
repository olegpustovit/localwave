//! Bounded PCM extraction and Swift-compatible source signatures, outside the frame loop.
use crate::bookmarks::failure;
use std::ffi::{CString, c_char, c_void};
use std::io;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

pub const PEAK_COUNT: usize = 512;
pub type Peaks = [f32; PEAK_COUNT];
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct FileInfo {
    availability: i32,
    provider: i32,
    size: i64,
    modified: f64,
}
#[derive(Clone, Debug)]
pub struct Version {
    pub digest: [u8; 32],
    file: FileInfo,
    path: CString,
    known_local_copy: bool,
}
unsafe extern "C" {
    fn lw_waveform_version(
        path: *const c_char,
        known: i32,
        info: *mut FileInfo,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_waveform_decode(
        path: *const c_char,
        known: i32,
        expected: *const FileInfo,
        cancelled: extern "C" fn(*mut c_void) -> i32,
        user: *mut c_void,
        peaks: *mut f32,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_sha256(bytes: *const u8, length: u32, digest: *mut u8);
}
fn native_path(path: &Path) -> io::Result<CString> {
    let text = path
        .to_str()
        .ok_or_else(|| io::Error::other("Waveform path is not UTF-8"))?;
    if !path.is_absolute() {
        return Err(io::Error::other("Waveform path must be absolute"));
    }
    CString::new(text).map_err(io::Error::other)
}
fn swift_double(value: f64) -> String {
    let value = format!("{value:?}");
    match value.split_once('e') {
        Some((number, exponent)) => format!(
            "{number}e{:+03}",
            exponent
                .parse::<i32>()
                .expect("Rust float exponent is an integer")
        ),
        None => value,
    }
}
fn signature(path: &Path, music: &Path, info: &FileInfo) -> io::Result<[u8; 32]> {
    let name = path.file_stem().and_then(|name| name.to_str());
    let content_addressed = music
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        && name.is_some_and(|name| {
            name.len() == 64 && name.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        && path
            .parent()
            .and_then(|parent| parent.canonicalize().ok())
            .zip(music.canonicalize().ok())
            .is_some_and(|(parent, music)| parent == music);
    let version = if content_addressed {
        name.expect("Content-addressed file has a stem").to_owned()
    } else {
        // Foundation's spelling and Swift's shortest Double text are part of the existing hash.
        let normalized = crate::preferences::file_path(&crate::normalized_file_url(
            path.to_str()
                .ok_or_else(|| io::Error::other("Waveform path is not UTF-8"))?,
        )?)?;
        format!(
            "{}|{}",
            normalized.display(),
            swift_double(if info.modified.is_finite() {
                info.modified
            } else {
                0.0
            })
        )
    };
    let identity = format!("{version}|Optional({})", info.size);
    let length = u32::try_from(identity.len()).map_err(io::Error::other)?;
    let mut digest = [0; 32];
    unsafe { lw_sha256(identity.as_ptr(), length, digest.as_mut_ptr()) };
    Ok(digest)
}
pub fn version(path: &Path, music: &Path, known_local_copy: bool) -> io::Result<Version> {
    if !music.is_absolute() {
        return Err(io::Error::other("Managed Music path must be absolute"));
    }
    let native = native_path(path)?;
    let mut file = FileInfo::default();
    let mut error = ptr::null_mut();
    if unsafe {
        lw_waveform_version(
            native.as_ptr(),
            known_local_copy as i32,
            &mut file,
            &mut error,
        )
    } == 0
    {
        return Err(failure(error));
    }
    Ok(Version {
        digest: signature(path, music, &file)?,
        file,
        path: native,
        known_local_copy,
    })
}

/// # Safety
/// Both paths are NUL-terminated; output buffers hold 32 bytes and 512 floats.
#[cfg(feature = "interop-test")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lw_waveform_interop_fixture(
    path: *const c_char,
    music: *const c_char,
    digest: *mut u8,
    peaks: *mut f32,
) -> i32 {
    if path.is_null() || music.is_null() || digest.is_null() || peaks.is_null() {
        return -1;
    }
    std::panic::catch_unwind(|| {
        let (Ok(path), Ok(music)) = (
            unsafe { std::ffi::CStr::from_ptr(path) }.to_str(),
            unsafe { std::ffi::CStr::from_ptr(music) }.to_str(),
        ) else {
            return -1;
        };
        let Ok(expected) = version(Path::new(path), Path::new(music), false) else {
            return -2;
        };
        let values = match decode(&expected, &AtomicBool::new(false)) {
            Ok(Some(values)) => values,
            Ok(None) => return -3,
            Err(error) => {
                eprintln!("Waveform interop: {error}");
                return -3;
            }
        };
        unsafe {
            ptr::copy_nonoverlapping(expected.digest.as_ptr(), digest, 32);
            ptr::copy_nonoverlapping(values.as_ptr(), peaks, PEAK_COUNT);
        }
        0
    })
    .unwrap_or(-4)
}
pub fn decode(expected: &Version, cancelled: &AtomicBool) -> io::Result<Option<Peaks>> {
    extern "C" fn is_cancelled(raw: *mut c_void) -> i32 {
        unsafe { &*raw.cast::<AtomicBool>() }.load(Ordering::Relaxed) as i32
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(io::ErrorKind::Interrupted.into());
    }
    let mut error = ptr::null_mut();
    let mut peaks = [0.0; PEAK_COUNT];
    let result = unsafe {
        lw_waveform_decode(
            expected.path.as_ptr(),
            expected.known_local_copy as i32,
            &expected.file,
            is_cancelled,
            ptr::from_ref(cancelled).cast_mut().cast(),
            peaks.as_mut_ptr(),
            &mut error,
        )
    };
    if result == 0 {
        let error = failure(error);
        return Err(if cancelled.load(Ordering::Relaxed) {
            io::ErrorKind::Interrupted.into()
        } else {
            error
        });
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(io::ErrorKind::Interrupted.into());
    }
    if result == 2 {
        return Ok(None);
    }
    if result != 1
        || peaks
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(io::Error::other("Invalid decoded waveform peaks"));
    }
    Ok(Some(peaks))
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(crate) fn signal(path: &Path, silent: bool) {
        let channels = 2u16;
        let rate = 48000u32;
        let data_size = rate * u32::from(channels) * 4;
        let mut data = Vec::with_capacity(data_size as usize + 44);
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&(data_size + 36).to_le_bytes());
        data.extend_from_slice(b"WAVEfmt ");
        data.extend_from_slice(&16u32.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes());
        data.extend_from_slice(&channels.to_le_bytes());
        data.extend_from_slice(&rate.to_le_bytes());
        data.extend_from_slice(&(rate * u32::from(channels) * 4).to_le_bytes());
        data.extend_from_slice(&(channels * 4).to_le_bytes());
        data.extend_from_slice(&32u16.to_le_bytes());
        data.extend_from_slice(b"data");
        data.extend_from_slice(&data_size.to_le_bytes());
        for frame in 0..rate {
            let value: f32 = if silent || frame < rate / 2 {
                0.0
            } else {
                -0.5
            };
            data.extend_from_slice(&0.0f32.to_le_bytes());
            data.extend_from_slice(&value.to_le_bytes());
        }
        std::fs::write(path, data).unwrap();
    }
    #[test]
    fn native_pcm_peaks_cover_all_channels_and_reject_cancelled_stale_and_provider_sources() {
        let root = std::env::temp_dir().join(format!(
            "lw-waveform-native-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let file = root.join("signal.wav");
        signal(&file, false);
        let music = root.join("Music");
        let expected = version(&file, &music, false).unwrap();
        let cancel = AtomicBool::new(false);
        let peaks = decode(&expected, &cancel).unwrap().unwrap();
        assert!(peaks[..256].iter().all(|peak| *peak == 0.0));
        assert!(peaks[256..].iter().all(|peak| *peak == 1.0));
        struct During {
            peaks: *const f32,
            file: Option<std::path::PathBuf>,
            changed: bool,
        }
        extern "C" fn during(raw: *mut c_void) -> i32 {
            use std::io::Write;
            let state = unsafe { &mut *raw.cast::<During>() };
            if unsafe { std::slice::from_raw_parts(state.peaks, PEAK_COUNT) }
                .iter()
                .any(|peak| *peak > 0.0)
            {
                if let Some(file) = &state.file {
                    if !state.changed {
                        state.changed = std::fs::OpenOptions::new()
                            .append(true)
                            .open(file)
                            .and_then(|mut file| file.write_all(&[0]))
                            .is_ok();
                    }
                } else {
                    return 1;
                }
            }
            0
        }
        let native = native_path(&file).unwrap();
        for mutate in [false, true] {
            let mut output = [0.0; PEAK_COUNT];
            let mut state = During {
                peaks: output.as_ptr(),
                file: mutate.then(|| file.clone()),
                changed: false,
            };
            let mut error = ptr::null_mut();
            let result = unsafe {
                lw_waveform_decode(
                    native.as_ptr(),
                    0,
                    &expected.file,
                    during,
                    ptr::from_mut(&mut state).cast(),
                    output.as_mut_ptr(),
                    &mut error,
                )
            };
            assert_eq!(result, 0);
            let error = failure(error).to_string();
            if mutate {
                assert!(state.changed);
                assert!(error.contains("changed during"), "{error}");
            } else {
                assert!(output.iter().any(|peak| *peak > 0.0));
                assert!(error.contains("cancelled"), "{error}");
            }
        }
        cancel.store(true, Ordering::Relaxed);
        assert_eq!(
            decode(&expected, &cancel).unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        cancel.store(false, Ordering::Relaxed);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap()
            .set_len(44)
            .unwrap();
        assert!(
            decode(&expected, &cancel)
                .unwrap_err()
                .to_string()
                .contains("changed before")
        );
        signal(&file, true);
        let silent = version(&file, &music, false).unwrap();
        assert_eq!(
            decode(&silent, &cancel).unwrap().unwrap(),
            [0.0; PEAK_COUNT]
        );
        let provider = root.join("Library/FileProviderStorage");
        std::fs::create_dir_all(&provider).unwrap();
        let provider = provider.join("signal.wav");
        std::fs::copy(&file, &provider).unwrap();
        assert!(
            version(&provider, &music, false)
                .unwrap_err()
                .to_string()
                .contains("not confirmed local")
        );
        assert!(version(&provider, &music, true).is_ok());
        assert!(version(Path::new("relative.wav"), &music, false).is_err());
        assert!(version(&root, &music, false).is_err());
    }
    #[test]
    fn swift_double_signature_spelling_matches_fixed_vectors() {
        for (value, expected) in [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (1.0, "1.0"),
            (1e-4, "0.0001"),
            (1e-5, "1e-05"),
            (1e-6, "1e-06"),
            (1e15, "1000000000000000.0"),
            (1e16, "1e+16"),
            (1e20, "1e+20"),
            (1794513600.1234567, "1794513600.1234567"),
        ] {
            assert_eq!(swift_double(value), expected);
        }
    }
}
