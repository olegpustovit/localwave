//! Application services owned by the library worker, separate from the frame loop.
#[cfg(target_os = "macos")]
pub mod appearance;
#[cfg(target_os = "macos")]
pub mod artwork_gpu;
#[cfg(target_os = "macos")]
pub mod controller;
#[cfg(target_os = "macos")]
pub mod library;
#[cfg(target_os = "macos")]
pub mod media;
