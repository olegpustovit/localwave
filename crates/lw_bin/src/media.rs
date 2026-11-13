//! MediaPlayer publication is outside drawing; ordinary clock ticks use the system's extrapolation.
use crate::controller::{Command, Controller, Status};
use crate::library::ArtworkReady;
use lw_apple_sys::media::{Media as Native, Metadata};
use lw_core::playback::Phase;
use lw_ui::artwork::{Key, Request};
use lw_ui::browse::Query;
use lw_ui::catalog::Catalog;
use std::io;
use std::sync::Arc;
use std::time::Instant;

struct Published {
    song: i64,
    generation: u64,
    title: Arc<str>,
    artist: Arc<str>,
    album: Arc<str>,
    playback_state: i32,
    has_queue: bool,
    position: f64,
    duration: f64,
    at: Instant,
}
impl Published {
    fn unchanged(&self, value: &Metadata<'_>, at: Instant) -> bool {
        let advance = if self.playback_state == 1 {
            at.saturating_duration_since(self.at).as_secs_f64()
        } else {
            0.0
        };
        let predicted = if self.duration > 0.0 {
            (self.position + advance).min(self.duration)
        } else {
            self.position + advance
        };
        self.song == value.song
            && self.generation == value.generation
            && self.title.as_ref() == value.title
            && self.artist.as_ref() == value.artist
            && self.album.as_ref() == value.album
            && self.playback_state == value.playback_state
            && self.duration == value.duration
            && self.has_queue == value.has_queue
            && (predicted - value.position).abs()
                < if value.playback_state == 1 {
                    0.25
                } else {
                    0.000_001
                }
    }
}
pub struct Media {
    pub native: Native,
    published: Option<Published>,
    cover: Option<(u64, Key)>,
    cover_ready: bool,
    cover_revision: u64,
}
impl Media {
    pub fn open(publish: bool, wake: extern "C" fn()) -> io::Result<Self> {
        Ok(Self {
            native: Native::open(publish, wake)?,
            published: None,
            cover: None,
            cover_ready: false,
            cover_revision: 0,
        })
    }
    pub fn deliver(&mut self, controller: &mut Controller) -> io::Result<()> {
        // Leave the head unacknowledged if the worker's bounded mailbox is busy.
        for _ in 0..32 {
            let Some(event) = self.native.peek() else {
                break;
            };
            match controller.command(command(event)?) {
                Ok(_) => self.native.acknowledge(),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
    pub fn sync(&mut self, status: &Status) -> io::Result<()> {
        self.sync_at(status, Instant::now())
    }
    fn sync_at(&mut self, status: &Status, at: Instant) -> io::Result<()> {
        let song = if matches!(status.phase, Phase::Stopped | Phase::Failed) {
            0
        } else {
            status.current.map_or(0, |entry| entry.song_id)
        };
        let playback_state = match status.phase {
            Phase::Playing => 1,
            Phase::Stopped | Phase::Ended | Phase::Failed => 3,
            _ => 2,
        };
        let has_queue = status.queue_len != 0;
        let position = finite_seconds(status.position);
        let duration = finite_seconds(status.duration);
        let identity = self
            .published
            .as_ref()
            .is_none_or(|old| old.song != song || old.generation != status.generation);
        let value = Metadata {
            song,
            generation: status.generation,
            title: &status.title,
            artist: &status.artist,
            album: &status.album,
            position,
            duration,
            playback_state,
            has_queue,
        };
        if self
            .published
            .as_ref()
            .is_some_and(|old| old.unchanged(&value, at))
        {
            return Ok(());
        }
        self.native.update(&value)?;
        if identity {
            self.cover = None;
            self.cover_ready = false;
            self.cover_revision = 0;
        }
        self.published = Some(Published {
            song,
            generation: status.generation,
            title: status.title.clone(),
            artist: status.artist.clone(),
            album: status.album.clone(),
            playback_state,
            has_queue,
            position,
            duration,
            at,
        });
        Ok(())
    }
    pub fn artwork<'a>(
        &mut self,
        catalog: &Catalog,
        epoch: u64,
        ready: impl Iterator<Item = &'a ArtworkReady>,
    ) -> io::Result<()> {
        let cover = self
            .published
            .as_ref()
            .and_then(|published| {
                catalog
                    .row_index(published.song)
                    .map(|index| catalog.library.row(index))
            })
            .and_then(|row| {
                Some((
                    epoch,
                    Key {
                        id: row.cover_art_id,
                        path: Query::new(row.cover_art_path?)?,
                        max_pixels: 256,
                    },
                ))
            });
        if self.cover != cover {
            self.native.artwork(None)?;
            self.cover = cover;
            self.cover_ready = false;
            self.cover_revision = 0;
        }
        let Some((epoch, key)) = self.cover else {
            return Ok(());
        };
        for image in ready {
            if image.epoch == epoch
                && image.key.id == key.id
                && image.key.path == key.path
                && image.revision > self.cover_revision
            {
                self.native.artwork(image.thumbnail.as_ref())?;
                self.cover_revision = image.revision;
                self.cover_ready = true;
            }
        }
        Ok(())
    }
    pub fn request_artwork(&self, request: &mut Request) {
        if !self.cover_ready
            && let Some((epoch, key)) = self.cover
            && epoch == request.epoch()
        {
            let previous = *request;
            request.clear();
            request.insert(key.id, key.path.as_str(), key.max_pixels);
            for other in previous.keys() {
                request.insert(other.id, other.path.as_str(), other.max_pixels);
            }
        }
    }
}
fn finite_seconds(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
fn command(event: lw_apple_sys::media::Command) -> io::Result<Command> {
    Ok(match event.kind {
        1 => Command::Play,
        2 => Command::Pause,
        3 => Command::Toggle,
        4 => Command::Stop,
        5 => Command::Next,
        6 => Command::Previous,
        7 if event.seconds.is_finite() && event.seconds >= 0.0 => Command::Seek(event.seconds),
        _ => return Err(io::ErrorKind::InvalidInput.into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_clock_extrapolation_handles_ticks_seek_pause_and_track_changes() {
        let at = Instant::now();
        let mut old = Published {
            song: 7,
            generation: 8,
            title: Arc::from("Track"),
            artist: Arc::from("Artist"),
            album: Arc::from("Album"),
            playback_state: 1,
            has_queue: true,
            position: 2.0,
            duration: 30.0,
            at,
        };
        let mut value = Metadata {
            song: 7,
            generation: 8,
            title: "Track",
            artist: "Artist",
            album: "Album",
            playback_state: 1,
            has_queue: true,
            position: 2.0,
            duration: 30.0,
        };
        mem_sys::local_reset();
        for tick in 0..=1000 {
            value.position = (2.0 + tick as f64 * 0.05).min(30.0);
            assert!(old.unchanged(&value, at + std::time::Duration::from_millis(tick * 50)));
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        value.position = 4.0;
        assert!(!old.unchanged(&value, at));
        value.position = 2.0;
        value.playback_state = 2;
        assert!(!old.unchanged(&value, at));
        old.playback_state = 2;
        assert!(old.unchanged(&value, at + std::time::Duration::from_secs(10)));
        value.position += 0.001;
        assert!(!old.unchanged(&value, at));
        value.position = 2.0;
        value.generation += 1;
        assert!(!old.unchanged(&value, at));
        value.generation -= 1;
        value.title = "Edited title";
        assert!(!old.unchanged(&value, at));
    }
    #[test]
    fn remote_commands_map_to_the_serialized_controller_without_allocating() {
        mem_sys::local_reset();
        for _ in 0..100 {
            for kind in 1..=7 {
                let result = command(lw_apple_sys::media::Command { kind, seconds: 3.5 }).unwrap();
                assert!(matches!(
                    (kind, result),
                    (1, Command::Play)
                        | (2, Command::Pause)
                        | (3, Command::Toggle)
                        | (4, Command::Stop)
                        | (5, Command::Next)
                        | (6, Command::Previous)
                        | (7, Command::Seek(3.5))
                ));
            }
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        for (kind, seconds) in [
            (0, 0.0),
            (8, 0.0),
            (7, -1.0),
            (7, f64::NAN),
            (7, f64::INFINITY),
        ] {
            assert!(command(lw_apple_sys::media::Command { kind, seconds }).is_err());
        }
        assert_eq!(finite_seconds(f64::INFINITY), 0.0);
        assert_eq!(finite_seconds(-2.0), 0.0);
    }
}
