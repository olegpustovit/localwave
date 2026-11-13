//! Main-thread texture ownership; frame builders borrow only `Images` descriptors.
use crate::library::ArtworkReady;
use lw_ui::artwork::{BYTE_LIMIT, CAPACITY, Images};
use ogpu_core::{
    backend as be,
    types::{Extent, Origin, TextureDescriptor, TextureFormat, TextureUsage},
};

const RETIRED_LIMIT: usize = CAPACITY * 3;
struct Texture {
    value: be::Texture,
    bytes: usize,
    serial: u64,
}
fn error(value: ogpu_core::error::Error) -> String {
    ogpu_core::error::to_string(&value)
}

pub struct ArtworkCache {
    pub images: Images,
    textures: Vec<Option<Texture>>,
    retired: Vec<Option<Texture>>,
    bytes: usize,
    library: usize,
    seen: u64,
}
impl Default for ArtworkCache {
    fn default() -> Self {
        Self {
            images: Images::default(),
            textures: (0..CAPACITY).map(|_| None).collect(),
            retired: (0..RETIRED_LIMIT).map(|_| None).collect(),
            bytes: 0,
            library: 0,
            seen: 0,
        }
    }
}
impl ArtworkCache {
    pub fn configure(
        &mut self,
        width: f32,
        height: f32,
        density: f32,
        completed: u64,
    ) -> Result<(), String> {
        if self.images.configure(width, height, density) {
            self.retire_completed(completed)?;
            for index in 0..CAPACITY {
                self.evict(index, completed)?;
            }
            self.images.clear();
        }
        Ok(())
    }
    pub fn resident_bytes(&self) -> usize {
        self.bytes
    }
    pub fn texture(&self, id: usize) -> Option<&be::Texture> {
        self.images
            .index(id)
            .and_then(|index| self.textures[index].as_ref())
            .map(|texture| &texture.value)
    }
    pub fn retire_completed(&mut self, completed: u64) -> Result<(), String> {
        for entry in &mut self.retired {
            if entry
                .as_ref()
                .is_some_and(|entry| entry.serial <= completed)
            {
                be::destroy_texture(&entry.as_ref().unwrap().value).map_err(error)?;
                entry.take();
            }
        }
        Ok(())
    }
    fn evict(&mut self, index: usize, completed: u64) -> Result<(), String> {
        if let Some(entry) = &self.textures[index] {
            let bytes = entry.bytes;
            if entry.serial <= completed {
                be::destroy_texture(&entry.value).map_err(error)?;
                self.textures[index].take();
            } else {
                let free = self
                    .retired
                    .iter()
                    .position(Option::is_none)
                    .ok_or("Artwork retirement exceeded its three-frame bound")?;
                self.retired[free] = self.textures[index].take();
            }
            self.bytes -= bytes;
        }
        self.images.remove(index);
        Ok(())
    }
    pub fn sync_library(&mut self, library: usize, completed: u64) -> Result<bool, String> {
        if self.library == library {
            return Ok(false);
        }
        self.retire_completed(completed)?;
        for index in 0..CAPACITY {
            self.evict(index, completed)?;
        }
        self.images.clear();
        self.library = library;
        Ok(true)
    }
    pub fn upload(
        &mut self,
        device: &be::Device,
        ready: &ArtworkReady,
        completed: u64,
    ) -> Result<bool, String> {
        if self.seen == ready.revision {
            return Ok(false);
        }
        self.retire_completed(completed)?;
        if ready.epoch != self.images.epoch() {
            self.seen = ready.revision;
            return Ok(false);
        }
        if self
            .images
            .quality(ready.key.id)
            .is_some_and(|quality| quality >= ready.key.max_pixels)
        {
            self.seen = ready.revision;
            return Ok(false);
        }
        let (mut owned, mut image, mut bytes) = (None, None, 0);
        if let Some(pixels) = &ready.thumbnail {
            if pixels.width == 0
                || pixels.height == 0
                || pixels.width > 1024
                || pixels.height > 1024
                || pixels.pixels.len() != pixels.width as usize * pixels.height as usize * 4
            {
                return Err("Invalid decoded artwork dimensions".into());
            }
            bytes = pixels.pixels.len();
            while self.bytes + bytes > BYTE_LIMIT {
                let oldest = (0..CAPACITY)
                    .filter(|index| self.textures[*index].is_some())
                    .min_by_key(|index| self.images.last_used(*index))
                    .ok_or("Invalid artwork byte accounting")?;
                self.evict(oldest, completed)?;
            }
            let texture = be::create_texture(
                device,
                TextureDescriptor {
                    label: None,
                    width: pixels.width as i64,
                    height: pixels.height as i64,
                    depth: 1,
                    mip_levels: 1,
                    sample_count: 1,
                    format: TextureFormat::Rgba8Unorm,
                    usage: vec![
                        TextureUsage::TextureBinding,
                        TextureUsage::TextureCopyDst,
                        TextureUsage::TextureCopySrc,
                    ],
                },
            )
            .map_err(error)?;
            let prepared = be::write_texture(
                &texture,
                None,
                Origin { x: 0, y: 0, z: 0 },
                Extent {
                    width: pixels.width as i64,
                    height: pixels.height as i64,
                    depth: 1,
                },
                pixels.width as i64 * 4,
                &pixels.pixels,
            )
            .and_then(|_| be::native_texture(&texture));
            let native = match prepared {
                Ok(native) => native,
                Err(failure) => {
                    let _ = be::destroy_texture(&texture);
                    return Err(error(failure));
                }
            };
            image = Some(pxui::Image {
                vk_image_view: native.vk_image_view,
                mtl_texture: native.mtl_texture,
                width: native.width,
                height: native.height,
                format: native.format,
                ..Default::default()
            });
            owned = Some(Texture {
                value: texture,
                bytes,
                serial: 0,
            });
        }
        let index = self
            .images
            .index(ready.key.id)
            .unwrap_or_else(|| self.images.oldest());
        if let Err(failure) = self.evict(index, completed) {
            if let Some(owned) = owned {
                let _ = be::destroy_texture(&owned.value);
            }
            return Err(failure);
        }
        self.textures[index] = owned;
        self.bytes += bytes;
        self.images.insert(index, ready.key, image);
        self.seen = ready.revision;
        Ok(true)
    }
    pub fn submitted(&mut self, serial: u64) {
        for texture in self.textures.iter_mut().flatten() {
            texture.serial = serial;
        }
    }
    /// The owner waits for frames and drops the renderer before destroying this cache.
    pub fn destroy(&mut self) -> Result<(), String> {
        for entry in self.textures.iter_mut().chain(self.retired.iter_mut()) {
            if let Some(texture) = entry {
                be::destroy_texture(&texture.value).map_err(error)?;
            }
            entry.take();
        }
        self.bytes = 0;
        self.images.clear();
        Ok(())
    }
}
