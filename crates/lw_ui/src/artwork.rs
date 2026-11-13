//! Bounded host-owned image descriptors and coalesced requests; no decoding or GPU ownership.
use crate::browse::Query;
use pxui::Image;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

pub const CAPACITY: usize = 1024;
pub const BYTE_LIMIT: usize = 128 * 1024 * 1024;
pub const REQUEST_CAPACITY: usize = 8;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Key {
    pub id: usize,
    pub path: Query,
    pub max_pixels: u32,
}
#[derive(Clone, Copy, Default, Debug, Eq)]
pub struct Request {
    epoch: u64,
    keys: [Key; REQUEST_CAPACITY],
    count: u8,
}
impl PartialEq for Request {
    fn eq(&self, other: &Self) -> bool {
        self.epoch == other.epoch && self.keys() == other.keys()
    }
}
impl Request {
    pub fn clear(&mut self) {
        self.count = 0;
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn keys(&self) -> &[Key] {
        &self.keys[..usize::from(self.count)]
    }
    pub fn insert(&mut self, id: usize, path: &str, max_pixels: u32) -> bool {
        if id == 0 || path.is_empty() || !(1..=1024).contains(&max_pixels) {
            return false;
        }
        if let Some(key) = self.keys[..usize::from(self.count)]
            .iter_mut()
            .find(|key| key.id == id)
        {
            key.max_pixels = key.max_pixels.max(max_pixels);
            return true;
        }
        if usize::from(self.count) == REQUEST_CAPACITY {
            return false;
        }
        let Some(path) = Query::new(path) else {
            return false;
        };
        self.keys[usize::from(self.count)] = Key {
            id,
            path,
            max_pixels,
        };
        self.count += 1;
        true
    }
}

struct Entry {
    id: usize,
    image: Option<Image>,
    quality: u32,
    used: Cell<u64>,
}
pub struct Images {
    entries: Vec<Entry>,
    by_id: HashMap<usize, usize>,
    clock: Cell<u64>,
    requests: RefCell<Request>,
    revision: u64,
    epoch: u64,
    density: f32,
    max_pixels: u32,
}
impl Default for Images {
    fn default() -> Self {
        Self {
            entries: (0..CAPACITY)
                .map(|_| Entry {
                    id: 0,
                    image: None,
                    quality: 0,
                    used: Cell::new(0),
                })
                .collect(),
            by_id: HashMap::with_capacity(CAPACITY),
            clock: Cell::new(0),
            requests: RefCell::new(Request::default()),
            revision: 0,
            epoch: 0,
            density: 1.0,
            max_pixels: 512,
        }
    }
}
impl Images {
    /// Bound the visible grid's worst-case square texels, including overscan and the player/queue.
    pub fn configure(&mut self, width: f32, height: f32, density: f32) -> bool {
        let density = if density.is_finite() {
            density.clamp(1.0, 4.0)
        } else {
            1.0
        };
        let columns = (width.max(1.0) / 180.0).ceil();
        let rows = (height.max(1.0) / 216.0).ceil() + 4.0;
        let visible = columns * rows + 32.0;
        let max_pixels =
            ((BYTE_LIMIT as f32 * 0.75 / (visible * 4.0)).sqrt() as u32 / 32 * 32).clamp(32, 512);
        if self.density != density || self.max_pixels != max_pixels {
            self.density = density;
            self.max_pixels = max_pixels;
            self.revision = self.revision.wrapping_add(1);
            return true;
        }
        false
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn last_used(&self, index: usize) -> u64 {
        self.entries[index].used.get()
    }
    pub fn begin_frame(&self) {
        let mut requests = self.requests.borrow_mut();
        requests.count = 0;
        requests.epoch = self.epoch;
    }
    pub fn requests(&self) -> Request {
        *self.requests.borrow()
    }
    pub fn index(&self, id: usize) -> Option<usize> {
        self.by_id.get(&id).copied()
    }
    pub fn quality(&self, id: usize) -> Option<u32> {
        self.index(id).map(|index| self.entries[index].quality)
    }
    pub fn get(&self, id: usize, path: &str, points: f32) -> Option<Image> {
        let quality = ((points.max(1.0) * self.density).ceil() as u32)
            .div_ceil(32)
            .saturating_mul(32)
            .clamp(32, self.max_pixels);
        let image = if let Some(index) = self.index(id) {
            let entry = &self.entries[index];
            let tick = self.clock.get().saturating_add(1);
            self.clock.set(tick);
            entry.used.set(tick);
            if entry.image.is_none() || entry.quality >= quality {
                return entry.image;
            }
            entry.image
        } else {
            None
        };
        self.requests.borrow_mut().insert(id, path, quality);
        image
    }
    pub fn oldest(&self) -> usize {
        self.entries
            .iter()
            .position(|entry| entry.id == 0)
            .unwrap_or_else(|| {
                self.entries
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, entry)| entry.used.get())
                    .unwrap()
                    .0
            })
    }
    pub fn insert(&mut self, index: usize, key: Key, image: Option<Image>) {
        self.remove(index);
        let tick = self.clock.get().saturating_add(1);
        self.clock.set(tick);
        self.entries[index] = Entry {
            id: key.id,
            image,
            quality: key.max_pixels,
            used: Cell::new(tick),
        };
        self.by_id.insert(key.id, index);
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn clear(&mut self) {
        self.by_id.clear();
        for entry in &mut self.entries {
            entry.id = 0;
            entry.image = None;
        }
        self.revision = self.revision.wrapping_add(1);
        self.epoch = self.epoch.wrapping_add(1);
        self.begin_frame();
    }
    pub fn remove(&mut self, index: usize) {
        self.by_id.remove(&self.entries[index].id);
        self.entries[index].id = 0;
        self.entries[index].image = None;
        self.revision = self.revision.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requests_deduplicate_and_cached_or_missing_images_allocate_nothing() {
        let mut images = Images::default();
        let path = Query::new("CoverArt/cover.png").unwrap();
        images.insert(
            0,
            Key {
                id: 1,
                path,
                max_pixels: 512,
            },
            None,
        );
        mem_sys::local_reset();
        for _ in 0..1000 {
            images.begin_frame();
            assert!(images.get(1, path.as_str(), 184.0).is_none());
            assert!(
                images.requests().keys().is_empty(),
                "failed images are cached too"
            );
            assert!(images.get(2, "CoverArt/pending.png", 30.0).is_none());
            assert!(images.get(2, "CoverArt/pending.png", 184.0).is_none());
            assert_eq!(images.requests().keys().len(), 1);
            assert_eq!(images.requests().keys()[0].max_pixels, 192);
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        images.clear();
        assert_eq!(images.oldest(), 0);
        assert!(!Request::default().insert(1, &"x".repeat(1025), 512));
    }
    #[test]
    fn visible_galleries_larger_than_sixty_four_keep_all_descriptors_without_warm_allocations() {
        let mut images = Images::default();
        let path = Query::new("cover.png").unwrap();
        for index in 0..768 {
            images.insert(
                index,
                Key {
                    id: index + 1,
                    path,
                    max_pixels: 512,
                },
                Some(Image {
                    width: 8,
                    height: 4,
                    ..Default::default()
                }),
            );
        }
        images.begin_frame();
        mem_sys::local_reset();
        for _ in 0..10 {
            for id in 1..=768 {
                assert!(images.get(id, path.as_str(), 184.0).is_some());
            }
        }
        assert!(images.requests().keys().is_empty());
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
    #[test]
    fn gallery_resolutions_fit_the_byte_budget_and_do_not_retry_small_sources() {
        let mut images = Images::default();
        for (width, height, density) in [
            (1440.0, 872.0, 2.0),
            (3840.0, 2160.0, 2.0),
            (7680.0, 4320.0, 1.0),
        ] {
            images.configure(width, height, density);
            let count = ((width / 180.0).ceil() * ((height / 216.0).ceil() + 4.0) + 32.0) as usize;
            assert!(count * (images.max_pixels as usize).pow(2) * 4 < BYTE_LIMIT);
        }
        images.configure(1440.0, 872.0, 2.0);
        let key = Key {
            id: 1,
            path: Query::new("tiny.png").unwrap(),
            max_pixels: 384,
        };
        images.insert(
            0,
            key,
            Some(Image {
                width: 8,
                height: 4,
                ..Default::default()
            }),
        );
        images.begin_frame();
        assert!(images.get(1, "tiny.png", 184.0).is_some());
        assert!(
            images.requests().keys().is_empty(),
            "record requested quality, not source dimensions"
        );
    }
}
