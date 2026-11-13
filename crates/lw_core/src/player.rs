//! Queue occurrences, preparation tickets and bounded file-failure traversal; no framework calls.
use crate::playback::{Fault, Phase, Play, Reading, Transport};
use crate::queue::{Entry, Queue, RepeatMode};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub generation: u64,
    pub token: u64,
    pub entry: Entry,
}
#[derive(Clone, Copy, Debug, Default)]
struct Slot {
    ticket: Option<Ticket>,
    requested: bool,
    installed: bool,
    failed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    /// A current item needs installation; `requests` supplies any preparation not already in flight.
    Load(Ticket),
    Advance {
        generation: u64,
        token: u64,
    },
    Stop,
    Ended,
}
#[derive(Default)]
pub struct Session {
    queue: Queue,
    transport: Transport,
    repeat: RepeatMode,
    slots: [Slot; 3],
    next_token: u64,
    visited: HashSet<u64>,
    notice: Option<Entry>,
    error: Option<Entry>,
    counted: u64,
}
impl Session {
    fn active(&self) -> bool {
        !matches!(
            self.transport.phase(),
            Phase::Stopped | Phase::Ended | Phase::Failed
        )
    }
    pub fn queue(&self) -> &Queue {
        &self.queue
    }
    pub fn transport(&self) -> &Transport {
        &self.transport
    }
    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }
    pub fn notice(&self) -> Option<Entry> {
        self.notice
    }
    pub fn error(&self) -> Option<Entry> {
        self.error
    }
    pub fn current_ticket(&self) -> Option<Ticket> {
        self.slots[0].ticket
    }
    pub fn planned(&self) -> [Option<Ticket>; 3] {
        self.slots.map(|slot| slot.ticket)
    }
    pub fn configure(&mut self, songs: &[i64], start: usize) {
        self.stop();
        self.queue.configure(songs, start);
        self.visited.clear();
        self.visited.reserve(songs.len());
        self.error = None;
        self.notice = None;
    }
    fn ticket(&mut self, entry: Entry) -> Ticket {
        self.next_token = self
            .next_token
            .checked_add(1)
            .expect("Native player identity exhausted");
        Ticket {
            generation: self.transport.generation(),
            token: self.next_token,
            entry,
        }
    }
    fn begin(&mut self, entry: Entry, autoplay: bool) -> Action {
        self.queue.select(entry.id);
        self.transport.begin(autoplay);
        self.slots = [Slot::default(); 3];
        self.error = None;
        let ticket = self.ticket(entry);
        self.slots[0].ticket = Some(ticket);
        Action::Load(ticket)
    }
    pub fn select(&mut self, occurrence: u64, autoplay: bool) -> Action {
        let Some(entry) = self
            .queue
            .entries()
            .iter()
            .find(|entry| entry.id == occurrence)
            .copied()
        else {
            return Action::None;
        };
        self.visited.clear();
        self.notice = None;
        self.error = None;
        if autoplay
            && self.slots[1].installed
            && !self.slots[1].failed
            && self.slots[1]
                .ticket
                .is_some_and(|ticket| ticket.entry == entry)
        {
            self.transport.play();
            let next = self.slots[1]
                .ticket
                .expect("Installed slot has an identity");
            Action::Advance {
                generation: next.generation,
                token: next.token,
            }
        } else {
            self.begin(entry, autoplay)
        }
    }
    pub fn play(&mut self) -> Action {
        if self.queue.current().is_none() {
            return Action::None;
        }
        match self.transport.play() {
            Play::Load => self.queue.current().map_or(Action::None, |entry| {
                self.visited.clear();
                self.notice = None;
                self.begin(entry, true)
            }),
            _ => Action::None,
        }
    }
    pub fn pause(&mut self) {
        self.transport.pause();
        if self.transport.phase() == Phase::Stopped {
            self.slots = [Slot::default(); 3];
        }
    }
    pub fn stop(&mut self) {
        self.transport.stop();
        self.slots = [Slot::default(); 3];
        self.visited.clear();
        self.counted = 0;
    }
    pub fn next_song(&mut self) -> Action {
        match self.queue.upcoming(self.repeat)[0] {
            Some(entry) => self.select(entry.id, true),
            None => {
                self.stop();
                Action::Stop
            }
        }
    }
    pub fn previous(&mut self) -> Action {
        self.queue
            .previous()
            .map_or(Action::None, |entry| self.select(entry.id, true))
    }
    pub fn expected(&self, ticket: Ticket) -> bool {
        self.active()
            && self
                .slots
                .iter()
                .any(|slot| slot.ticket == Some(ticket) && !slot.failed)
    }
    pub fn requests(&mut self) -> [Option<Ticket>; 3] {
        if matches!(
            self.transport.phase(),
            Phase::Stopped | Phase::Ended | Phase::Failed
        ) {
            return [None; 3];
        }
        std::array::from_fn(|index| {
            let slot = &mut self.slots[index];
            if slot.requested || slot.failed || slot.installed {
                None
            } else {
                slot.requested = slot.ticket.is_some();
                slot.ticket
            }
        })
    }
    pub fn retry_request(&mut self, ticket: Ticket) {
        for slot in &mut self.slots {
            if slot.ticket == Some(ticket) && !slot.installed && !slot.failed {
                slot.requested = false;
            }
        }
    }
    pub fn installable(&self, ticket: Ticket) -> bool {
        if !self.active() {
            return false;
        }
        let Some(index) = self
            .slots
            .iter()
            .position(|slot| slot.ticket == Some(ticket) && !slot.failed && !slot.installed)
        else {
            return false;
        };
        index == 0
            || self.slots[..index]
                .iter()
                .all(|slot| slot.installed || slot.failed)
    }
    pub fn installed(&mut self, ticket: Ticket) -> bool {
        if !self.installable(ticket) {
            return false;
        }
        let index = self
            .slots
            .iter()
            .position(|slot| slot.ticket == Some(ticket))
            .expect("Validated playback ticket");
        self.slots[index].installed = true;
        self.slots[index].requested = true;
        if index == 0 {
            self.transport.adopt(ticket.generation, ticket.token);
            self.rebuild_upcoming();
        }
        true
    }
    fn rebuild_upcoming(&mut self) {
        if !self.slots[0].installed
            || matches!(
                self.transport.phase(),
                Phase::Stopped | Phase::Ended | Phase::Failed
            )
        {
            return;
        }
        let desired = self.queue.upcoming(self.repeat);
        let mut preserve = true;
        for (index, entry) in desired.into_iter().enumerate() {
            preserve &= self.slots[index + 1].ticket.map(|ticket| ticket.entry) == entry;
            if !preserve {
                self.slots[index + 1] = Slot {
                    ticket: entry.map(|entry| self.ticket(entry)),
                    ..Slot::default()
                };
            }
        }
    }
    /// Reconcile the actual native current item before delivering its queued failure events.
    pub fn adopt(&mut self, generation: u64, token: u64) -> bool {
        if !self.active() {
            return false;
        }
        let Some(index) = self.slots.iter().position(|slot| {
            slot.installed
                && !slot.failed
                && slot
                    .ticket
                    .is_some_and(|ticket| ticket.generation == generation && ticket.token == token)
        }) else {
            return false;
        };
        if index == 0 {
            return true;
        }
        let ticket = self.slots[index]
            .ticket
            .expect("Installed slot has an identity");
        if !self.transport.adopt(generation, token) {
            return false;
        }
        self.queue.select(ticket.entry.id);
        for offset in 0..3 {
            self.slots[offset] = if index + offset < 3 {
                self.slots[index + offset]
            } else {
                Slot::default()
            };
        }
        self.rebuild_upcoming();
        true
    }
    pub fn update(&mut self, reading: Reading) -> Option<Entry> {
        if !self.transport.update(reading)
            || self.transport.phase() != Phase::Playing
            || self.counted == reading.token
        {
            return None;
        }
        self.counted = reading.token;
        self.visited.clear();
        self.error = None;
        self.queue.current()
    }
    /// The caller must supply a leased ticket observed as the actual native current item.
    pub fn reconcile_native(&mut self, ticket: Ticket) -> bool {
        if self.adopt(ticket.generation, ticket.token) {
            return true;
        }
        if !self.active()
            || ticket.generation != self.transport.generation()
            || !self.queue.entries().contains(&ticket.entry)
            || !self.transport.adopt(ticket.generation, ticket.token)
        {
            return false;
        }
        self.queue.select(ticket.entry.id);
        self.slots = [
            Slot {
                ticket: Some(ticket),
                requested: true,
                installed: true,
                failed: false,
            },
            Slot::default(),
            Slot::default(),
        ];
        self.rebuild_upcoming();
        true
    }
    pub fn file_failed(&mut self, ticket: Ticket) -> Action {
        if !self.active()
            || ticket.generation != self.transport.generation()
            || !self.queue.entries().contains(&ticket.entry)
        {
            return Action::None;
        }
        let Some(index) = self
            .slots
            .iter()
            .position(|slot| slot.ticket == Some(ticket) && !slot.failed)
        else {
            return Action::None;
        };
        self.slots[index].failed = true;
        if index != 0 {
            self.notice = Some(ticket.entry);
            return Action::None;
        }
        let autoplay = self.transport.wants_playback();
        self.visited.insert(ticket.entry.id);
        if autoplay && let Some(next) = self.following_unvisited(ticket.entry) {
            self.notice = Some(ticket.entry);
            return self.begin(next, true);
        }
        self.error = Some(ticket.entry);
        self.transport
            .fail(ticket.generation, self.transport.token(), Fault::File);
        Action::Stop
    }
    pub fn retired_failure(&mut self, generation: u64, entry: Entry) {
        if self.active()
            && generation == self.transport.generation()
            && self.queue.entries().contains(&entry)
        {
            self.notice = Some(entry);
        }
    }
    fn following_unvisited(&self, entry: Entry) -> Option<Entry> {
        let index = self
            .queue
            .entries()
            .iter()
            .position(|candidate| *candidate == entry)?;
        let following = index + 1..self.queue.entries().len();
        let wrapped = if self.repeat == RepeatMode::None {
            0..0
        } else {
            0..index
        };
        following
            .chain(wrapped)
            .map(|index| self.queue.entries()[index])
            .find(|entry| !self.visited.contains(&entry.id))
    }
    pub fn session_failed(&mut self, generation: u64) -> bool {
        if !self
            .transport
            .fail(generation, self.transport.token(), Fault::Session)
        {
            return false;
        }
        self.error = self.queue.current();
        true
    }
    pub fn native_empty(&mut self) -> Action {
        if matches!(
            self.transport.phase(),
            Phase::Stopped | Phase::Ended | Phase::Failed
        ) {
            return Action::None;
        }
        if let Some(index) =
            (1..3).find(|index| self.slots[*index].ticket.is_some() && !self.slots[*index].failed)
        {
            let ticket = self.slots[index].ticket.expect("Existing upcoming item");
            self.queue.select(ticket.entry.id);
            self.transport.await_next();
            for offset in 0..3 {
                self.slots[offset] = if index + offset < 3 {
                    self.slots[index + offset]
                } else {
                    Slot::default()
                };
            }
            self.slots[0].installed = false;
            return Action::Load(ticket);
        }
        let last_bad = self.slots[1..]
            .iter()
            .filter(|slot| slot.failed)
            .filter_map(|slot| slot.ticket)
            .next_back();
        if let Some(last_bad) = last_bad {
            for slot in &self.slots[1..] {
                if slot.failed
                    && let Some(ticket) = slot.ticket
                {
                    self.visited.insert(ticket.entry.id);
                }
            }
            self.notice = Some(last_bad.entry);
            if let Some(next) = self.following_unvisited(last_bad.entry) {
                return self.begin(next, true);
            }
            self.begin(last_bad.entry, false);
            self.error = Some(last_bad.entry);
            self.slots[0].failed = true;
            self.transport
                .fail(self.transport.generation(), 0, Fault::File);
            return Action::Stop;
        }
        self.transport.finish(
            self.transport.generation(),
            self.transport.token(),
            self.transport.duration(),
        );
        Action::Ended
    }
    pub fn finished(&mut self, generation: u64, token: u64, duration: f64) {
        if self.transport.phase() == Phase::Ended {
            self.transport.finish(generation, token, duration);
        } else {
            self.transport.update(Reading {
                generation,
                token,
                duration,
                position: duration,
                ready: true,
                playing: false,
                waiting: false,
            });
        }
    }
    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
        self.rebuild_upcoming();
    }
    pub fn set_shuffle(&mut self, enabled: bool, seed: u64) {
        self.queue.set_shuffle(enabled, seed);
        self.rebuild_upcoming();
    }
    pub fn reorder(&mut self, indices: &[usize], destination: usize) -> bool {
        let changed = self.queue.reorder(indices, destination);
        if changed {
            self.rebuild_upcoming();
        }
        changed
    }
    pub fn remove(&mut self, occurrence: u64) -> bool {
        let changed = self.queue.remove(occurrence);
        if changed {
            self.rebuild_upcoming();
        }
        changed
    }
    pub fn append(&mut self, song_id: i64) -> Entry {
        let entry = self.queue.append(song_id);
        self.visited.reserve(
            self.queue
                .entries()
                .len()
                .saturating_sub(self.visited.len()),
        );
        self.rebuild_upcoming();
        entry
    }
    pub fn clear_upcoming(&mut self) {
        self.queue.clear_upcoming();
        self.rebuild_upcoming();
    }
    pub fn restore(&mut self, entries: &[Entry]) -> bool {
        let changed = self.queue.restore(entries);
        if changed {
            self.rebuild_upcoming();
        }
        changed
    }
    pub fn dismiss_notice(&mut self) {
        self.notice = None;
    }
    pub fn invalidate_upcoming(&mut self) {
        self.slots[1..].fill(Slot::default());
        self.rebuild_upcoming();
    }
    pub fn interruption_began(&mut self) {
        self.transport.interruption_began();
        if self.transport.phase() == Phase::Stopped {
            self.slots = [Slot::default(); 3];
        }
    }
    pub fn interruption_ended(&mut self, allowed: bool) -> Action {
        match self.transport.interruption_ended(allowed) {
            Some(Play::Load) => self
                .queue
                .current()
                .map_or(Action::None, |entry| self.begin(entry, true)),
            _ => Action::None,
        }
    }
    pub fn output_unavailable(&mut self) {
        self.transport.output_unavailable();
        if self.transport.phase() == Phase::Stopped {
            self.slots = [Slot::default(); 3];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn load(action: Action) -> Ticket {
        let Action::Load(ticket) = action else {
            panic!("Expected a load, got {action:?}");
        };
        ticket
    }
    fn start(session: &mut Session, ids: &[i64]) -> Ticket {
        session.configure(ids, 0);
        load(session.select(session.queue().current().unwrap().id, true))
    }
    fn playing(session: &mut Session, ticket: Ticket) {
        session.update(Reading {
            generation: ticket.generation,
            token: ticket.token,
            position: 1.0,
            duration: 12.5,
            ready: true,
            playing: true,
            waiting: false,
        });
    }
    #[test]
    fn duplicate_failures_visit_each_occurrence_once_and_a_good_file_resets_the_traversal() {
        let mut player = Session::default();
        let first = start(&mut player, &[7, 7, 8]);
        let second = load(player.file_failed(first));
        assert_eq!(first.entry.song_id, second.entry.song_id);
        assert_ne!(first.entry.id, second.entry.id);
        let good = load(player.file_failed(second));
        assert_eq!(good.entry.song_id, 8);
        assert_eq!(player.notice(), Some(second.entry));
        assert!(player.installed(good));
        playing(&mut player, good);
        assert_eq!(player.transport().phase(), Phase::Playing);
        assert_eq!(player.error(), None);
        assert_eq!(player.file_failed(first), Action::None);
        for repeat in [RepeatMode::All, RepeatMode::One] {
            let first = start(&mut player, &[7, 7]);
            player.set_repeat(repeat);
            let second = load(player.file_failed(first));
            assert_eq!(player.file_failed(second), Action::Stop);
            assert_eq!(player.queue().current_index(), Some(1));
            assert_eq!(player.error(), Some(second.entry));
            assert_eq!(player.requests(), [None; 3]);
            assert_eq!(player.transport().phase(), Phase::Failed);
        }
    }
    #[test]
    fn prequeue_failures_preserve_playing_audio_and_final_failures_keep_a_terminal_error() {
        let mut player = Session::default();
        let first = start(&mut player, &[1, 2, 3]);
        player.installed(first);
        playing(&mut player, first);
        let [_, Some(bad), Some(good)] = player.planned() else {
            panic!("Missing prequeue");
        };
        assert!(!player.installable(good));
        assert_eq!(player.file_failed(bad), Action::None);
        assert!(player.installable(good));
        player.installed(good);
        assert!(player.adopt(good.generation, good.token));
        playing(&mut player, good);
        assert_eq!(player.queue().current_index(), Some(2));
        assert_eq!(player.error(), None);
        assert_eq!(player.notice(), Some(bad.entry));
        assert_eq!(player.native_empty(), Action::Ended);
        assert_eq!(player.transport().position(), 12.5);
        let replay = load(player.play());
        assert_ne!(replay.token, good.token);
        assert_eq!(player.transport().position(), 0.0);
        let first = start(&mut player, &[1, 2, 3]);
        player.installed(first);
        playing(&mut player, first);
        let [_, Some(bad1), Some(bad2)] = player.planned() else {
            panic!("Missing prequeue");
        };
        player.file_failed(bad1);
        player.file_failed(bad2);
        assert_eq!(player.native_empty(), Action::Stop);
        assert_eq!(player.queue().current_index(), Some(2));
        assert_eq!(player.error(), Some(bad2.entry));
        assert_eq!(player.transport().phase(), Phase::Failed);
        assert_eq!(player.native_empty(), Action::None);
    }
    #[test]
    fn exhausted_prequeue_continues_after_bad_files_and_promotes_inflight_preparation_without_restarting_it()
     {
        let mut player = Session::default();
        let first = start(&mut player, &[1, 2, 3, 4]);
        player.installed(first);
        let [_, Some(bad1), Some(bad2)] = player.planned() else {
            panic!("Missing prequeue");
        };
        player.file_failed(bad1);
        player.file_failed(bad2);
        let fourth = load(player.native_empty());
        assert_eq!(fourth.entry.song_id, 4);
        assert_eq!(player.notice(), Some(bad2.entry));
        assert!(player.installed(fourth));
        playing(&mut player, fourth);
        assert_eq!(player.error(), None);
        let first = start(&mut player, &[1, 2, 3]);
        player.installed(first);
        player.requests();
        let second = player.planned()[1].unwrap();
        assert_eq!(player.native_empty(), Action::Load(second));
        assert!(player.expected(second));
        assert!(player.installable(second));
        assert_eq!(player.requests()[0], None);
        assert!(player.installed(second));
    }
    #[test]
    fn queue_edits_reject_stale_tickets_and_keep_the_current_native_identity() {
        let mut player = Session::default();
        let first = start(&mut player, &[7, 7, 8, 9]);
        player.installed(first);
        let old = player.planned()[1].unwrap();
        let snapshot = player.queue().entries().to_vec();
        assert!(player.reorder(&[2], 1));
        assert_eq!(player.current_ticket(), Some(first));
        assert!(!player.expected(old));
        player.set_shuffle(true, 42);
        assert_eq!(player.current_ticket(), Some(first));
        player.set_shuffle(false, 0);
        assert_eq!(player.current_ticket(), Some(first));
        player.clear_upcoming();
        assert_eq!(player.planned(), [Some(first), None, None]);
        assert!(player.restore(&snapshot));
        let future = player.planned()[1].unwrap();
        player.invalidate_upcoming();
        assert!(!player.expected(future));
        let future = player.planned()[1].unwrap();
        player.installed(future);
        assert_eq!(
            player.next_song(),
            Action::Advance {
                generation: future.generation,
                token: future.token
            }
        );
        assert!(player.adopt(future.generation, future.token));
        assert_eq!(player.queue().current_index(), Some(1));
        assert!(!player.remove(future.entry.id));
        assert!(player.remove(first.entry.id));
        assert_eq!(player.queue().current_index(), Some(0));
        player.configure(&[10], 0);
        assert!(!player.expected(future));
        assert!(!player.adopt(future.generation, future.token));
    }
    #[test]
    fn pause_and_session_failure_do_not_skip_files_or_resume_cancelled_loads() {
        let mut player = Session::default();
        let old = start(&mut player, &[1, 2]);
        player.pause();
        assert!(!player.expected(old));
        assert!(!player.installed(old));
        assert_eq!(player.file_failed(old), Action::None);
        let current = load(player.play());
        player.installed(current);
        assert!(player.session_failed(current.generation));
        assert!(!player.expected(current));
        assert!(!player.adopt(current.generation, current.token));
        assert_eq!(player.file_failed(current), Action::None);
        assert_eq!(player.queue().current_index(), Some(0));
        assert_eq!(player.transport().fault(), Some(Fault::Session));
        assert_eq!(player.notice(), None);
        let retry = load(player.play());
        player.installed(retry);
        playing(&mut player, retry);
        player.interruption_began();
        player.output_unavailable();
        assert_eq!(player.interruption_ended(true), Action::None);
        assert!(!player.transport().wants_playback());
        let old = load(player.select(player.queue().current().unwrap().id, true));
        player.interruption_began();
        assert!(!player.expected(old));
        let resumed = load(player.interruption_ended(true));
        assert_ne!(resumed.generation, old.generation);
    }
    #[test]
    fn native_advancement_after_reorder_keeps_the_leased_current_and_late_completion_duration() {
        let mut player = Session::default();
        let first = start(&mut player, &[1, 2, 3]);
        player.installed(first);
        let next = player.requests()[1].unwrap();
        player.installed(next);
        assert!(player.reorder(&[2], 1));
        assert!(!player.expected(next));
        assert!(player.reconcile_native(next));
        assert_eq!(player.queue().current_index(), Some(2));
        assert_eq!(player.transport().token(), next.token);
        assert_eq!(player.native_empty(), Action::Ended);
        player.finished(next.generation, next.token, 2.5);
        assert_eq!(player.transport().position(), 2.5);
        player.finished(first.generation, first.token, 10.0);
        assert_eq!(player.transport().position(), 2.5);
        player.stop();
        assert!(!player.reconcile_native(next));
        player.finished(next.generation, next.token, 3.0);
        assert_eq!(player.transport().position(), 0.0);
        let first = start(&mut player, &[1, 2]);
        player.installed(first);
        let next = player.requests()[1].unwrap();
        player.installed(next);
        player.remove(next.entry.id);
        assert!(!player.reconcile_native(next));
    }
    #[test]
    fn ten_thousand_natural_advances_and_prequeue_refills_allocate_nothing() {
        let mut player = Session::default();
        let first = start(&mut player, &(1..=10000).collect::<Vec<_>>());
        player.set_repeat(RepeatMode::All);
        player.installed(first);
        for ticket in player.requests().into_iter().flatten() {
            player.installed(ticket);
        }
        mem_sys::local_reset();
        for _ in 0..10000 {
            let next = player.planned()[1].unwrap();
            assert!(player.adopt(next.generation, next.token));
            for ticket in player.requests().into_iter().flatten() {
                player.installed(ticket);
            }
            playing(&mut player, next);
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        assert_eq!(player.queue().current_index(), Some(0));
        assert_ne!(
            player.planned()[0].unwrap().token,
            player.planned()[1].unwrap().token
        );
    }
}
