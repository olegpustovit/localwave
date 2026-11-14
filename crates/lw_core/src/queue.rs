//! Queue identity belongs to an occurrence, including repeated copies of the same song.
use std::collections::HashSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: u64,
    pub song_id: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RepeatMode {
    #[default]
    None,
    All,
    One,
}

#[derive(Default)]
pub struct Queue {
    entries: Vec<Entry>,
    original: Vec<Entry>,
    current: Option<usize>,
    shuffled: bool,
    next_id: u64,
}

impl Queue {
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn current_index(&self) -> Option<usize> {
        self.current
    }
    pub fn current(&self) -> Option<Entry> {
        self.current.map(|index| self.entries[index])
    }
    pub fn shuffled(&self) -> bool {
        self.shuffled
    }

    pub fn configure(&mut self, song_ids: &[i64], start: usize) {
        self.entries.clear();
        self.original.clear();
        self.current = None;
        self.shuffled = false;
        for id in song_ids {
            self.append(*id);
        }
        if !self.entries.is_empty() {
            self.current = Some(if start < self.entries.len() { start } else { 0 });
        }
    }

    pub fn append(&mut self, song_id: i64) -> Entry {
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("Queue occurrence identity exhausted");
        let entry = Entry {
            id: self.next_id,
            song_id,
        };
        self.entries.push(entry);
        if self.shuffled {
            self.original.push(entry);
        }
        self.current.get_or_insert(0);
        entry
    }

    pub fn select(&mut self, id: u64) -> bool {
        if let Some(index) = self.entries.iter().position(|entry| entry.id == id) {
            self.current = Some(index);
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: u64) -> bool {
        if self.current().is_some_and(|entry| entry.id == id) {
            return false;
        }
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return false;
        };
        self.entries.remove(index);
        self.original.retain(|entry| entry.id != id);
        if let Some(current) = self.current.as_mut()
            && index < *current
        {
            *current -= 1;
        }
        true
    }

    /// Destination uses the insertion boundary before moving, as Swift's move(fromOffsets:toOffset:).
    pub fn reorder(&mut self, indices: &[usize], destination: usize) -> bool {
        if indices.is_empty()
            || destination > self.entries.len()
            || indices.iter().any(|index| *index >= self.entries.len())
            || indices.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return false;
        }
        let current = self.current();
        let moved: Vec<_> = indices.iter().map(|index| self.entries[*index]).collect();
        let mut index = 0;
        self.entries.retain(|_| {
            let keep = indices.binary_search(&index).is_err();
            index += 1;
            keep
        });
        let insertion = destination - indices.iter().filter(|index| **index < destination).count();
        self.entries.splice(insertion..insertion, moved);
        self.current =
            current.and_then(|current| self.entries.iter().position(|entry| *entry == current));
        true
    }

    pub fn clear_upcoming(&mut self) {
        self.entries
            .truncate(self.current.map_or(0, |index| index + 1));
        if self.shuffled {
            let retained: HashSet<_> = self.entries.iter().map(|entry| entry.id).collect();
            self.original.retain(|entry| retained.contains(&entry.id));
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.original.clear();
        self.current = None;
    }

    /// Preserve exact occurrences for Clear/Undo. Reject stale snapshots that would change the playing entry.
    pub fn restore(&mut self, entries: &[Entry]) -> bool {
        let mut seen = HashSet::with_capacity(entries.len());
        if entries
            .iter()
            .any(|entry| entry.id == 0 || entry.id > self.next_id || !seen.insert(entry.id))
        {
            return false;
        }
        let current = self.current();
        if current.is_some_and(|current| !entries.contains(&current)) {
            return false;
        }
        self.entries.clear();
        self.entries.extend_from_slice(entries);
        self.current = current
            .and_then(|current| self.entries.iter().position(|entry| *entry == current))
            .or_else(|| (!entries.is_empty()).then_some(0));
        if self.shuffled {
            self.original.clone_from(&self.entries);
        }
        true
    }

    pub fn advance(&mut self, repeat: RepeatMode) -> Option<Entry> {
        let index = self.current?;
        self.current = Some(match repeat {
            RepeatMode::One => index,
            RepeatMode::All => (index + 1) % self.entries.len(),
            RepeatMode::None if index + 1 < self.entries.len() => index + 1,
            RepeatMode::None => return None,
        });
        self.current()
    }

    pub fn previous(&mut self) -> Option<Entry> {
        let index = self.current?;
        self.current = Some((index + self.entries.len() - 1) % self.entries.len());
        self.current()
    }

    pub fn upcoming(&self, repeat: RepeatMode) -> [Option<Entry>; 2] {
        let Some(index) = self.current else {
            return [None; 2];
        };
        match repeat {
            RepeatMode::One => [self.current(); 2],
            RepeatMode::All => [
                Some(self.entries[(index + 1) % self.entries.len()]),
                Some(self.entries[(index + 2) % self.entries.len()]),
            ],
            RepeatMode::None => [
                self.entries.get(index + 1).copied(),
                self.entries.get(index + 2).copied(),
            ],
        }
    }

    /// Seeded by the host once per shuffle; deterministic here so playback tests need no OS randomness.
    pub fn set_shuffle(&mut self, enabled: bool, mut seed: u64) {
        if enabled == self.shuffled {
            return;
        }
        let current = self.current();
        if enabled {
            self.original.clone_from(&self.entries);
            if let Some(index) = self.current {
                self.entries.swap(0, index);
            }
            for index in (2..self.entries.len()).rev() {
                self.entries
                    .swap(index, 1 + random_below(&mut seed, index as u64) as usize);
            }
        } else {
            self.entries.clone_from(&self.original);
            self.original.clear();
        }
        self.current =
            current.and_then(|current| self.entries.iter().position(|entry| *entry == current));
        self.shuffled = enabled;
    }
}

/// Shuffle a fresh playback selection, including its first song; called on the controller worker.
pub fn shuffle_songs(songs: &mut [i64], mut seed: u64) {
    for index in (1..songs.len()).rev() {
        songs.swap(index, random_below(&mut seed, index as u64 + 1) as usize);
    }
}

fn random_below(seed: &mut u64, bound: u64) -> u64 {
    let threshold = bound.wrapping_neg() % bound;
    loop {
        *seed = seed.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = *seed;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^= value >> 31;
        if value >= threshold {
            return value % bound;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_selection_shuffle_includes_first_song_preserves_duplicates_and_is_seeded() {
        let original = [1, 1, 2, 3, 4, 5, 6, 7];
        let mut changed_first = false;
        for seed in 0..64 {
            let mut a = original;
            let mut b = original;
            shuffle_songs(&mut a, seed);
            shuffle_songs(&mut b, seed);
            assert_eq!(a, b);
            changed_first |= a[0] != original[0];
            a.sort_unstable();
            assert_eq!(a, original);
        }
        assert!(
            changed_first,
            "fresh shuffle must not preserve the first song"
        );
        shuffle_songs(&mut [], 0);
        let mut one = [9];
        shuffle_songs(&mut one, u64::MAX);
        assert_eq!(one, [9]);
    }

    #[test]
    fn ten_thousand_song_advance_and_prequeue_allocate_nothing() {
        let mut queue = Queue::default();
        queue.configure(&(1..=10000).collect::<Vec<_>>(), 0);
        mem_sys::local_reset();
        for _ in 0..10000 {
            std::hint::black_box(queue.advance(RepeatMode::All));
            std::hint::black_box(queue.upcoming(RepeatMode::All));
        }
        let allocations = mem_sys::local_alloc_count();
        assert_eq!(allocations, 0);
        assert_eq!(queue.current_index(), Some(0));
    }

    #[test]
    fn duplicate_occurrences_survive_reorder_shuffle_remove_clear_and_undo() {
        let mut queue = Queue::default();
        queue.configure(&[7, 7, 8, 9, 10], 1);
        let playing = queue.current().unwrap();
        let original = queue.entries().to_vec();
        assert_ne!(original[0].id, playing.id);
        assert!(queue.reorder(&[0, 2], 5));
        assert_eq!(queue.current(), Some(playing));
        assert_eq!(
            queue
                .entries()
                .iter()
                .map(|entry| entry.song_id)
                .collect::<Vec<_>>(),
            [7, 9, 10, 7, 8]
        );
        assert!(!queue.remove(playing.id));
        let before_shuffle = queue.entries().to_vec();
        queue.set_shuffle(true, 42);
        assert_eq!(queue.current_index(), Some(0));
        assert_eq!(queue.current(), Some(playing));
        let removed = original[0];
        assert!(queue.remove(removed.id));
        let appended = queue.append(7);
        queue.set_shuffle(false, 0);
        let mut expected: Vec<_> = before_shuffle
            .into_iter()
            .filter(|entry| *entry != removed)
            .collect();
        expected.push(appended);
        assert_eq!(queue.entries(), expected);
        let snapshot = queue.entries().to_vec();
        queue.clear_upcoming();
        assert_eq!(queue.entries(), [playing]);
        assert!(queue.restore(&snapshot));
        assert_eq!(queue.entries(), snapshot);
        assert_eq!(queue.current(), Some(playing));
        assert!(!queue.restore(&[playing, playing]));
        assert!(!queue.reorder(&[2, 1], 0));
        assert!(!queue.reorder(&[99], 0));
        queue.clear();
        assert_eq!(queue.current(), None);
        queue.configure(&[7], 99);
        assert!(queue.current().unwrap().id > appended.id);
    }

    #[test]
    fn repeat_and_two_item_prequeue_match_player_semantics() {
        let mut queue = Queue::default();
        assert_eq!(queue.upcoming(RepeatMode::All), [None; 2]);
        assert_eq!(queue.advance(RepeatMode::All), None);
        queue.configure(&[1, 2, 3], 0);
        let entries = queue.entries().to_vec();
        assert_eq!(
            queue.upcoming(RepeatMode::None),
            [Some(entries[1]), Some(entries[2])]
        );
        assert_eq!(queue.previous(), Some(entries[2]));
        assert_eq!(queue.upcoming(RepeatMode::None), [None; 2]);
        assert_eq!(queue.advance(RepeatMode::None), None);
        assert_eq!(queue.current(), Some(entries[2]));
        assert_eq!(queue.advance(RepeatMode::All), Some(entries[0]));
        assert_eq!(queue.advance(RepeatMode::One), Some(entries[0]));
        assert_eq!(queue.upcoming(RepeatMode::One), [Some(entries[0]); 2]);
        queue.configure(&[1], 0);
        assert_eq!(queue.upcoming(RepeatMode::All), [queue.current(); 2]);
    }
}
