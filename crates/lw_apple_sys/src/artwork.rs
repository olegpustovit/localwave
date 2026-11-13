//! Atomic cover publication and bounded, orientation-aware worker thumbnails.
use crate::bookmarks::failure;
use std::ffi::{CStr, CString, c_char, c_void};
use std::{io, path::Path, ptr};

unsafe extern "C" {
    fn lw_artwork_thumbnail(
        path: *const c_char,
        max_pixels: u32,
        width: *mut u32,
        height: *mut u32,
        error: *mut *mut c_char,
    ) -> *mut c_void;
    fn lw_artwork_pixels(image: *mut c_void, pixels: *mut u8, length: usize) -> i32;
    fn lw_artwork_thumbnail_release(image: *mut c_void);
    fn lw_artwork_store(
        documents: *const c_char,
        bytes: *const u8,
        len: usize,
        relative: *mut *mut c_char,
        error: *mut *mut c_char,
    ) -> i32;
    fn lw_free_string(value: *mut c_char);
}

#[derive(Debug)]
pub struct Thumbnail {
    pub width: u32,
    pub height: u32,
    /// Top-to-bottom sRGB RGBA8 with straight alpha, ready for a host-owned texture.
    pub pixels: Vec<u8>,
}

pub fn thumbnail(path: &Path, max_pixels: u32) -> io::Result<Thumbnail> {
    crate::require_worker_thread()?;
    if !path.is_absolute() || !(1..=1024).contains(&max_pixels) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected an absolute artwork path and 1...1024 pixels",
        ));
    }
    let path = CString::new(
        path.to_str()
            .ok_or_else(|| io::Error::other("Artwork path is not UTF-8"))?,
    )
    .map_err(io::Error::other)?;
    let (mut width, mut height, mut error) = (0, 0, ptr::null_mut());
    let raw = unsafe {
        lw_artwork_thumbnail(
            path.as_ptr(),
            max_pixels,
            &mut width,
            &mut height,
            &mut error,
        )
    };
    if raw.is_null() {
        return Err(failure(error));
    }
    struct Image(*mut c_void);
    impl Drop for Image {
        fn drop(&mut self) {
            unsafe { lw_artwork_thumbnail_release(self.0) };
        }
    }
    let image = Image(raw);
    if width == 0 || height == 0 || width > max_pixels || height > max_pixels {
        return Err(io::Error::other("Invalid artwork dimensions"));
    }
    let mut pixels = vec![0; width as usize * height as usize * 4];
    if unsafe { lw_artwork_pixels(image.0, pixels.as_mut_ptr(), pixels.len()) } == 0 {
        return Err(io::Error::other("Could not rasterize cover art"));
    }
    Ok(Thumbnail {
        width,
        height,
        pixels,
    })
}
pub fn store(documents: &Path, bytes: &[u8]) -> io::Result<Option<String>> {
    let documents = CString::new(
        documents
            .to_str()
            .ok_or_else(|| io::Error::other("Artwork path is not UTF-8"))?,
    )
    .map_err(io::Error::other)?;
    let (mut relative, mut error) = (ptr::null_mut(), ptr::null_mut());
    match unsafe {
        lw_artwork_store(
            documents.as_ptr(),
            bytes.as_ptr(),
            bytes.len(),
            &mut relative,
            &mut error,
        )
    } {
        0 => Ok(None),
        1 => {
            let path = unsafe { CStr::from_ptr(relative) }
                .to_str()
                .map(str::to_owned)
                .map_err(io::Error::other);
            unsafe { lw_free_string(relative) };
            path.map(Some)
        }
        _ => Err(failure(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_thumbnails_preserve_pixels_alpha_orientation_and_size_bounds() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Tests/LocalWaveDataTests/Fixtures");
        let image = thumbnail(&fixture.join("artwork-rgba.png"), 32).unwrap();
        assert_eq!((image.width, image.height, image.pixels.len()), (8, 4, 128));
        let pixel = |x: usize, y: usize| &image.pixels[(y * 8 + x) * 4..(y * 8 + x + 1) * 4];
        assert_eq!(pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(pixel(0, 2), [0, 0, 255, 255]);
        for (actual, expected) in pixel(0, 3).iter().zip([200u8, 100, 50, 128]) {
            assert!(
                actual.abs_diff(expected) <= 1,
                "straight alpha: {:?}",
                pixel(0, 3)
            );
        }
        assert_eq!(pixel(7, 3), [0, 0, 0, 0]);
        let small = thumbnail(&fixture.join("artwork-rgba.png"), 4).unwrap();
        assert_eq!((small.width, small.height, small.pixels.len()), (4, 2, 32));
        let rotated = thumbnail(&fixture.join("artwork-rotated.tiff"), 32).unwrap();
        assert_eq!((rotated.width, rotated.height), (3, 2));
        assert_eq!(&rotated.pixels[..4], [0, 0, 255, 255]);
        assert!(thumbnail(Path::new("relative.png"), 32).is_err());
        for size in [0, 1025, u32::MAX] {
            assert!(thumbnail(&fixture.join("artwork-rgba.png"), size).is_err());
        }
        assert!(thumbnail(&fixture.join("not-present.png"), 32).is_err());
        assert!(thumbnail(&fixture, 32).is_err());
    }

    #[test]
    fn malformed_and_oversized_artwork_fails_without_publishing_pixels() {
        let base = std::env::temp_dir().join(format!(
            "lw-thumbnail-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&base).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let file = base.join("cover.png");
        for bytes in [b"".as_slice(), b"\x89PNG", b"not an image"] {
            std::fs::write(&file, bytes).unwrap();
            assert!(thumbnail(&file, 32).is_err());
        }
        let oversized = std::fs::File::create(&file).unwrap();
        oversized.set_len(64 * 1024 * 1024 + 1).unwrap();
        assert!(
            thumbnail(&file, 32)
                .unwrap_err()
                .to_string()
                .contains("64 MiB")
        );
    }
}
