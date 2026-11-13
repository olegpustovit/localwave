//! Metadata-only availability probes and coordinated worker-side audio reads.
use crate::bookmarks::failure;
use std::ffi::{CStr, CString, c_char, c_void};
use std::io;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Unknown,
    OnDevice,
    InCloud,
}

#[derive(Clone, Copy, Debug)]
pub struct FileInfo {
    pub availability: Availability,
    pub provider: bool,
    pub size: Option<i64>,
    pub modified: Option<f64>,
}

#[repr(C)]
#[derive(Default)]
struct RawFileInfo {
    availability: i32,
    provider: i32,
    size: i64,
    modified: f64,
}

#[derive(Debug)]
pub struct Metadata {
    pub warning: Option<String>,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<i64>,
    pub track: Option<i64>,
    pub disc: Option<i64>,
    pub duration: Option<f64>,
    pub sample_rate: Option<f64>,
    pub bit_depth: Option<i64>,
    pub artwork: Option<Vec<u8>>,
    pub sha256: [u8; 32],
}

unsafe extern "C" {
    fn lw_file_probe(
        path: *const c_char,
        known: i32,
        info: *mut RawFileInfo,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_audio_read(
        path: *const c_char,
        known: i32,
        cancelled: extern "C" fn(*mut c_void) -> i32,
        user: *mut c_void,
        extra: extern "C" fn(*mut c_void, *const c_char) -> i32,
        extra_user: *mut c_void,
        error: *mut *mut c_char,
    ) -> *mut c_void;
    fn lw_audio_text(audio: *mut c_void, slot: u32, len: *mut usize) -> *const u8;
    fn lw_audio_number(audio: *mut c_void, slot: u32) -> f64;
    fn lw_audio_data(audio: *mut c_void, slot: u32, len: *mut usize) -> *const u8;
    fn lw_audio_release(audio: *mut c_void);
}

fn native_path(path: &Path) -> io::Result<CString> {
    CString::new(
        path.to_str()
            .ok_or_else(|| io::Error::other("Audio path is not UTF-8"))?,
    )
    .map_err(|_| io::Error::other("Audio path contains NUL"))
}

/// Worker-only resource attributes, without opening audio or requesting a provider download.
pub fn probe(path: &Path, known_local_copy: bool) -> io::Result<FileInfo> {
    let path = native_path(path)?;
    let mut info = RawFileInfo::default();
    let mut error = ptr::null_mut();
    if unsafe {
        lw_file_probe(
            path.as_ptr(),
            known_local_copy as i32,
            &mut info,
            &mut error,
        )
    } == 0
    {
        return Err(failure(error));
    }
    Ok(FileInfo {
        availability: match info.availability {
            1 => Availability::OnDevice,
            2 => Availability::InCloud,
            _ => Availability::Unknown,
        },
        provider: info.provider != 0,
        size: (info.size >= 0).then_some(info.size),
        modified: info.modified.is_finite().then_some(info.modified),
    })
}

struct Raw(*mut c_void);
impl Drop for Raw {
    fn drop(&mut self) {
        unsafe {
            lw_audio_release(self.0);
        }
    }
}
impl Raw {
    fn text(&self, slot: u32) -> io::Result<Option<String>> {
        let mut len = 0;
        let text = unsafe { lw_audio_text(self.0, slot, &mut len) };
        if text.is_null() {
            return Ok(None);
        }
        if len > 64 * 1024 * 1024 {
            return Err(io::Error::other("Native audio tag exceeds metadata limit"));
        }
        std::str::from_utf8(unsafe { std::slice::from_raw_parts(text, len) })
            .map(|text| Some(text.to_owned()))
            .map_err(io::Error::other)
    }
    fn number(&self, slot: u32) -> Option<f64> {
        let number = unsafe { lw_audio_number(self.0, slot) };
        number.is_finite().then_some(number)
    }
    fn data(&self, slot: u32) -> io::Result<&[u8]> {
        let mut len = 0;
        let bytes = unsafe { lw_audio_data(self.0, slot, &mut len) };
        if len == 0 {
            return Ok(&[]);
        }
        if bytes.is_null() || len > 64 * 1024 * 1024 {
            return Err(io::Error::other("Invalid native audio metadata buffer"));
        }
        Ok(unsafe { std::slice::from_raw_parts(bytes, len) })
    }
}

/// AVFoundation tags/measurements and streaming SHA-256 inside one coordinated read.
/// Main-thread calls and unconfirmed provider content are rejected by the native boundary.
pub fn read(path: &Path, known_local_copy: bool, cancelled: &AtomicBool) -> io::Result<Metadata> {
    read_with(path, known_local_copy, cancelled, &mut |_| Ok(()))
}

/// Run the core's FLAC parser on the coordinator-supplied URL while its read grant is live.
/// Callback errors propagate and panics resume only after returning from Objective-C.
pub fn read_with(
    path: &Path,
    known_local_copy: bool,
    cancelled: &AtomicBool,
    extra: &mut dyn FnMut(&Path) -> io::Result<()>,
) -> io::Result<Metadata> {
    struct Extra<'a> {
        operation: &'a mut dyn FnMut(&Path) -> io::Result<()>,
        outcome: Option<std::thread::Result<io::Result<()>>>,
    }
    extern "C" fn read_extra(raw: *mut c_void, path: *const c_char) -> i32 {
        let extra = unsafe { &mut *raw.cast::<Extra<'_>>() };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if path.is_null() {
                return Err(io::Error::other("Coordinator supplied no audio path"));
            }
            let path = unsafe { CStr::from_ptr(path) }
                .to_str()
                .map_err(io::Error::other)?;
            (extra.operation)(Path::new(path))
        }));
        let success = matches!(&outcome, Ok(Ok(())));
        extra.outcome = Some(outcome);
        success as i32
    }
    extern "C" fn is_cancelled(raw: *mut c_void) -> i32 {
        unsafe { &*raw.cast::<AtomicBool>() }.load(Ordering::Relaxed) as i32
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(io::ErrorKind::Interrupted.into());
    }
    let path = native_path(path)?;
    let mut error = ptr::null_mut();
    let mut extra = Extra {
        operation: extra,
        outcome: None,
    };
    let raw = Raw(unsafe {
        lw_audio_read(
            path.as_ptr(),
            known_local_copy as i32,
            is_cancelled,
            ptr::from_ref(cancelled).cast_mut().cast(),
            read_extra,
            ptr::from_mut(&mut extra).cast(),
            &mut error,
        )
    });
    let native_error = raw.0.is_null().then(|| failure(error));
    match extra.outcome {
        Some(Err(panic)) => std::panic::resume_unwind(panic),
        Some(Ok(Err(error))) => return Err(error),
        _ => {}
    }
    if let Some(error) = native_error {
        return Err(if cancelled.load(Ordering::Relaxed) {
            io::ErrorKind::Interrupted.into()
        } else {
            error
        });
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(io::ErrorKind::Interrupted.into());
    }
    let artwork = raw.data(0)?;
    Ok(Metadata {
        warning: raw.text(4)?,
        artist: raw.text(0)?,
        title: raw.text(1)?,
        album: raw.text(2)?,
        album_artist: raw.text(3)?,
        duration: raw.number(0).filter(|number| *number >= 0.0),
        sample_rate: raw.number(1).filter(|number| *number > 0.0),
        bit_depth: raw
            .number(2)
            .filter(|number| *number > 0.0)
            .map(|number| number as i64),
        year: raw.number(3).map(|number| number as i64),
        track: raw.number(4).map(|number| number as i64),
        disc: raw.number(5).map(|number| number as i64),
        artwork: (!artwork.is_empty()).then(|| artwork.to_vec()),
        sha256: raw
            .data(1)?
            .try_into()
            .map_err(|_| io::Error::other("Native SHA-256 length is invalid"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_audio_probe_metadata_artwork_and_streaming_digest() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a");
        let info = probe(&fixture, false).unwrap();
        assert_eq!(
            info.size,
            Some(std::fs::metadata(&fixture).unwrap().len() as i64)
        );
        assert!(!info.provider);
        let cancelled = AtomicBool::new(false);
        let metadata = read(&fixture, false, &cancelled).unwrap();
        assert_eq!(metadata.artist.as_deref(), Some("LocalWave Tests"));
        assert_eq!(metadata.title.as_deref(), Some("M4A Fixture"));
        assert_eq!(metadata.album.as_deref(), Some("M4A Album"));
        assert_eq!(metadata.album_artist.as_deref(), Some("AAC Ensemble"));
        assert_eq!(metadata.year, Some(2025));
        assert_eq!(metadata.track, Some(3));
        assert_eq!(metadata.disc, Some(2));
        assert!(metadata.duration.unwrap() > 0.9);
        assert!(metadata.sample_rate.unwrap() >= 44100.0);
        assert!(metadata.artwork.as_deref().unwrap().starts_with(b"\x89PNG"));
        assert_eq!(
            metadata.sha256,
            [
                0x7a, 0x80, 0xe4, 0x9d, 0x16, 0xa2, 0x15, 0x95, 0xf3, 0xa2, 0xbf, 0xf4, 0x3f, 0x35,
                0xc8, 0xfd, 0xc6, 0x99, 0xbe, 0x50, 0xda, 0x50, 0xd8, 0x47, 0x4b, 0xaf, 0x11, 0x06,
                0xb6, 0xe3, 0xc6, 0x16
            ]
        );
        cancelled.store(true, Ordering::Relaxed);
        assert_eq!(
            read(&fixture, false, &cancelled).unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert!(probe(Path::new("relative/audio.flac"), false).is_err());
        assert!(probe(Path::new("/invalid\0path"), false).is_err());
    }

    #[test]
    fn unknown_provider_is_never_opened_for_metadata() {
        let root = std::env::temp_dir().join(format!(
            "lw-provider-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let folder = root.join("Library/FileProviderStorage");
        std::fs::create_dir_all(&folder).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let path = folder.join("provider.flac");
        std::fs::write(&path, []).unwrap();
        let info = probe(&path, false).unwrap();
        assert_eq!(info.availability, Availability::Unknown);
        assert!(info.provider);
        assert!(
            read(&path, false, &AtomicBool::new(false))
                .unwrap_err()
                .to_string()
                .contains("not confirmed local")
        );
        let local = root.join("local.flac");
        std::fs::write(&local, []).unwrap();
        assert_eq!(
            probe(&local, false).unwrap().availability,
            Availability::OnDevice
        );
        let damaged = read(&local, false, &AtomicBool::new(false)).unwrap();
        assert!(damaged.warning.is_some());
        assert!(damaged.duration.is_none());
        assert!(damaged.title.is_none());
        let mut tagged =
            include_bytes!("../../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a").to_vec();
        let title = tagged
            .windows(b"M4A Fixture".len())
            .position(|bytes| bytes == b"M4A Fixture")
            .unwrap();
        tagged[title + 3] = 0;
        let nul_title = root.join("nul-title.m4a");
        std::fs::write(&nul_title, tagged).unwrap();
        let metadata = read(&nul_title, false, &AtomicBool::new(false)).unwrap();
        assert_eq!(metadata.title.as_deref(), Some("M4A\0Fixture"));
    }

    #[test]
    fn coordinated_callback_errors_and_panics_stay_on_the_rust_side() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../Tests/LocalWaveDataTests/Fixtures/tagged-m4a.m4a");
        let cancelled = AtomicBool::new(false);
        let error = read_with(&fixture, false, &cancelled, &mut |_| {
            Err(io::ErrorKind::PermissionDenied.into())
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        let panic = std::panic::catch_unwind(|| {
            let _ = read_with(&fixture, false, &cancelled, &mut |_| {
                panic!("callback panic")
            });
        });
        assert!(panic.is_err());
    }
}
