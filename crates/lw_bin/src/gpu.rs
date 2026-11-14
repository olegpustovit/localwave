//! Music-specific request sizing and decoded-result delivery through framework host APIs.
pub use pxui_host::Graphics;

pub fn configure_artwork(
    graphics: &mut Graphics,
    width: f32,
    height: f32,
    density: f32,
) -> Result<(), String> {
    graphics.configure_images(density, max_pixels(width, height))
}
fn max_pixels(width: f32, height: f32) -> u32 {
    let columns = (width.max(1.0) / 180.0).ceil();
    let rows = (height.max(1.0) / 216.0).ceil() + 4.0;
    let visible = columns * rows + 32.0;
    ((lw_ui::artwork::BYTE_LIMIT as f32 * 0.75 / (visible * 4.0)).sqrt() as u32 / 32 * 32)
        .clamp(32, 512)
}

pub fn sync_artwork<'a>(
    graphics: &mut Graphics,
    namespace: usize,
    ready: impl IntoIterator<Item = &'a lw_bin::library::ArtworkReady>,
) -> Result<bool, String> {
    let mut changed = graphics.sync_images(namespace)?;
    for ready in ready {
        let pixels = ready
            .thumbnail
            .as_ref()
            .map(|image| (image.width, image.height, image.pixels.as_slice()));
        changed |= graphics.upload_image(ready.key, ready.epoch, ready.revision, pixels)?;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    #[test]
    fn gallery_image_resolution_fits_framework_byte_budget() {
        for (width, height) in [(1440.0, 872.0), (3840.0, 2160.0), (7680.0, 4320.0)] {
            let count =
                ((width / 180.0_f32).ceil() * ((height / 216.0_f32).ceil() + 4.0) + 32.0) as usize;
            assert!(
                count * (super::max_pixels(width, height) as usize).pow(2) * 4
                    < lw_ui::artwork::BYTE_LIMIT
            );
        }
    }
}
