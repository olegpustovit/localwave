//! Immutable queue projection prepared on the worker and retired with its catalog.
use lw_core::{context::PlaybackContext, queue::Entry};
use std::sync::Arc;

pub struct Snapshot {
    pub revision: u64,
    pub entries: Arc<[Entry]>,
    pub current: Option<usize>,
    pub context: Arc<PlaybackContext>,
    pub heading: String,
    pub count: String,
    pub total: String,
    pub can_undo: bool,
}
impl Snapshot {
    pub fn new(
        revision: u64,
        entries: Arc<[Entry]>,
        current: Option<usize>,
        context: Arc<PlaybackContext>,
        can_undo: bool,
    ) -> Self {
        let start = current
            .and_then(|index| index.checked_add(1))
            .unwrap_or(entries.len())
            .min(entries.len());
        Self {
            revision,
            heading: format!("Up next · {}", context.title()),
            count: (entries.len() - start).to_string(),
            total: entries.len().to_string(),
            entries,
            current,
            context,
            can_undo,
        }
    }
    pub fn upcoming(&self) -> &[Entry] {
        let start = self
            .current
            .and_then(|index| index.checked_add(1))
            .unwrap_or(self.entries.len())
            .min(self.entries.len());
        &self.entries[start..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_updates_share_entries_and_keep_duplicate_occurrences() {
        let entries: Arc<[Entry]> = Arc::from([
            Entry { id: 1, song_id: 7 },
            Entry { id: 2, song_id: 7 },
            Entry { id: 3, song_id: 8 },
        ]);
        let context = Arc::new(PlaybackContext::Library);
        let old = Snapshot::new(1, entries.clone(), Some(0), context.clone(), false);
        let new = Snapshot::new(2, entries.clone(), Some(1), context.clone(), true);
        assert!(Arc::ptr_eq(&old.entries, &new.entries));
        assert_eq!((old.count.as_str(), new.count.as_str()), ("2", "1"));
        assert_eq!(old.upcoming()[0].id, 2);
        assert_eq!(new.upcoming()[0].id, 3);
        assert!(
            Snapshot::new(3, entries.clone(), None, context.clone(), false)
                .upcoming()
                .is_empty()
        );
        assert!(
            Snapshot::new(4, entries, Some(usize::MAX), context, false)
                .upcoming()
                .is_empty()
        );
        mem_sys::local_reset();
        for _ in 0..1000 {
            assert_eq!(new.upcoming()[0].song_id, 8);
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
}
