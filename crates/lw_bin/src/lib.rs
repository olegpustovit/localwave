//! Application services owned by the library worker, separate from the frame loop.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod appearance;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod controller;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod files;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod library;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod media;
