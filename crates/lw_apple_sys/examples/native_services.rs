//! Manual Apple-runtime qualification; never opens the user's library or standard defaults.
use lw_apple_sys::{bookmarks, media};
use std::{io, path::Path};

fn worker() -> io::Result<()> {
    lw_apple_sys::require_worker_thread()?;
    let directory =
        std::env::temp_dir().join(format!("localwave-native-services-{}", std::process::id()));
    std::fs::create_dir(&directory)?;
    let result = (|| {
        let path = directory.join("名前.flac");
        std::fs::write(&path, b"bookmark fixture")?;
        let bookmark = bookmarks::create(&path)?;
        let access = bookmarks::Access::resolve(&bookmark)?;
        assert_eq!(std::fs::read(access.path())?, b"bookmark fixture");
        assert_eq!(
            std::fs::canonicalize(access.path())?,
            std::fs::canonicalize(&path)?
        );
        assert!(bookmarks::Access::resolve(&[]).is_err());
        assert!(bookmarks::Access::resolve(&[1, 2, 3]).is_err());
        assert!(bookmarks::create(Path::new("relative/path")).is_err());
        Ok(())
    })();
    std::fs::remove_dir_all(directory)?;
    result
}

fn main() -> io::Result<()> {
    assert!(lw_apple_sys::require_worker_thread().is_err());
    assert_eq!(
        media::verify_callbacks(),
        0,
        "native media queue/artwork qualification"
    );
    std::thread::spawn(worker)
        .join()
        .expect("native service worker panicked")?;
    println!(
        "LOCALWAVE_NATIVE_SERVICES_OK platform={} media_callbacks=ok artwork_pixels=ok bookmark_roundtrip=ok",
        std::env::consts::OS
    );
    Ok(())
}
