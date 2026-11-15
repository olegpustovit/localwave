//! Worker-owned native reconciliation and coalesced immutable publications.
use super::*;

impl Worker {
    pub(super) fn acknowledge(&mut self, ack: Ack) -> io::Result<()> {
        let rejected = matches!(&ack.result, Ok(false));
        let Some(pending) = self.pending.take() else {
            return Err(io::Error::other(
                "Unexpected native playback acknowledgement",
            ));
        };
        match ack.result {
            Ok(true) => match pending {
                Pending::Install { ticket, replace } => {
                    self.session.installed(ticket);
                    if replace {
                        self.intent = false;
                    }
                }
                Pending::Intent(intent) => self.intent = intent,
                Pending::Stop | Pending::Reset => {
                    self.intent = false;
                    if matches!(pending, Pending::Reset) {
                        self.reset = false;
                    }
                }
                Pending::Volume | Pending::Trim | Pending::Advance(_) | Pending::Seek => {}
            },
            Ok(false) => {
                if let Pending::Install { ticket, .. } = pending {
                    self.session.retry_request(ticket);
                }
            }
            Err(error) => self.session_error(error),
        }
        self.observe(ack.snapshot);
        if rejected && let Pending::Advance(ticket) = pending {
            let autoplay = self.session.transport().wants_playback();
            if self.commit_selection(ticket) && autoplay {
                let action = self.session.play();
                self.action(action);
            }
        }
        Ok(())
    }
    pub(super) fn observe(&mut self, snapshot: Snapshot) {
        let previous = self.native;
        self.native = snapshot;
        if snapshot.token != 0 {
            if let Some(ticket) = self
                .leases
                .iter()
                .flatten()
                .find(|ticket| {
                    ticket.token == snapshot.token && ticket.generation == snapshot.generation
                })
                .copied()
            {
                self.session.reconcile_native(ticket);
            }
            self.session.update(Reading {
                generation: snapshot.generation,
                token: snapshot.token,
                position: snapshot.position,
                duration: snapshot.duration,
                ready: snapshot.status == 1,
                playing: snapshot.playing(),
                waiting: snapshot.time_control == 1,
            });
        } else if previous.token != 0
            && previous.generation == self.session.transport().generation()
        {
            self.empty_from = Some(previous.generation);
        }
    }
    pub(super) fn event(&mut self, event: Event) {
        match event.kind {
            Kind::EngineFailed => self.session_error(io::Error::other(event.message())),
            Kind::Failed => {
                if self.leases.iter().flatten().any(|ticket| {
                    ticket.token == event.token()
                        && ticket.generation == event.generation()
                        && ticket.entry.id == event.occurrence()
                }) && let Err(error) = self.library.record_playback_failure(event.token())
                {
                    eprintln!("LocalWave native file failure state: {error}");
                }
                let ticket = self.session.planned().into_iter().flatten().find(|ticket| {
                    ticket.token == event.token() && ticket.generation == event.generation()
                });
                if let Some(ticket) = ticket {
                    let action = self.session.file_failed(ticket);
                    self.action(action);
                } else if let Some(entry) = self
                    .session
                    .queue()
                    .entries()
                    .iter()
                    .find(|entry| entry.id == event.occurrence())
                    .copied()
                {
                    self.session.retired_failure(event.generation(), entry);
                }
                if self.session.notice().is_some() || self.session.error().is_some() {
                    self.message = Arc::from(event.message());
                }
            }
            Kind::Finished => {
                self.session
                    .finished(event.generation(), event.token(), event.duration())
            }
            _ => {}
        }
    }
    pub(super) fn action(&mut self, action: Action) {
        if let Action::Advance { generation, token } = action {
            self.advance = Some((generation, token));
        } else if matches!(action, Action::Load(_) | Action::Stop | Action::Ended) {
            self.advance = None;
        }
    }
    pub(super) fn pending_selection(&self) -> Option<Ticket> {
        self.advance
            .and_then(|(generation, token)| {
                self.session
                    .planned()
                    .into_iter()
                    .flatten()
                    .find(|ticket| ticket.generation == generation && ticket.token == token)
            })
            .or(match self.pending {
                Some(Pending::Advance(ticket)) => Some(ticket),
                _ => None,
            })
    }
    pub(super) fn commit_selection(&mut self, ticket: Ticket) -> bool {
        if ticket.generation != self.session.transport().generation()
            || matches!(
                self.session.transport().phase(),
                Phase::Stopped | Phase::Ended | Phase::Failed
            )
            || self.native.token == ticket.token
            || !self.native.queued.contains(&ticket.token)
            || !self.session.queue().entries().contains(&ticket.entry)
        {
            return false;
        }
        self.advance = None;
        self.session.select(ticket.entry.id, false);
        true
    }
    pub(super) fn dirty_catalog(&mut self) {
        self.catalog_dirty = true;
        self.shared.inbox.lock().unwrap().catalog_pending = true;
    }
    pub(super) fn publish_catalog(&self) {
        self.shared.publish_catalog(&self.catalog);
    }
    pub(super) fn publish(&mut self) {
        let cursor = self.session.queue().current_index();
        let can_undo = self.undo.is_some();
        if self.queue_dirty
            || self.queue.as_ref().is_none_or(|queue| {
                queue.current != cursor
                    || queue.can_undo != can_undo
                    || !Arc::ptr_eq(&queue.context, &self.context)
            })
        {
            let entries = if self.queue_dirty {
                Arc::from(self.session.queue().entries())
            } else {
                self.queue.as_ref().unwrap().entries.clone()
            };
            let revision = self
                .queue
                .as_ref()
                .map_or(1, |queue| queue.revision.wrapping_add(1));
            let queue = Arc::new(lw_ui::queue::Snapshot::new(
                revision,
                entries,
                cursor,
                self.context.clone(),
                can_undo,
            ));
            self.queue = Some(queue.clone());
            self.catalog.queue = Some(queue);
            self.queue_dirty = false;
            self.publish_catalog();
        }
        let current = self.session.queue().current();
        let id = current.map(|entry| entry.song_id);
        if id != self.metadata_id {
            self.waveform_key = None;
            self.waveform = None;
            self.metadata_id = id;
            let fetched = match id
                .map(|id| self.library.repository.get_song(id))
                .transpose()
            {
                Ok(song) => song.flatten(),
                Err(error) => {
                    self.message = Arc::from(error.to_string());
                    None
                }
            };
            // A removed library row can still own the current physical cached item.
            let song = fetched.as_ref().or_else(|| {
                current
                    .filter(|entry| entry.id == self.native.occurrence && self.native.token != 0)
                    .and_then(|_| self.library.playback_song(self.native.token))
                    .filter(|song| song.id == id)
            });
            if let Some(song) = song {
                self.title = Arc::from(song.title.as_str());
                self.artist = Arc::from(song.artist.as_str());
                self.album = Arc::from(song.album.as_str());
            } else {
                self.title = self.empty.clone();
                self.artist = self.empty.clone();
                self.album = self.empty.clone();
            }
        }
        let transport = self.session.transport();
        let status = Status {
            settings: self.settings,
            cache_usage: self.cache_usage.clone(),
            settings_message: self.settings_message.clone(),
            storage_busy: self.storage_pending,
            waveform: self.waveform,
            context: self.context.clone(),
            revision: self.status.revision,
            stable_revision: self.status.stable_revision,
            initialized: true,
            processed_command: self.revision,
            generation: transport.generation(),
            current,
            queue_len: self.session.queue().entries().len(),
            current_index: self.session.queue().current_index(),
            phase: transport.phase(),
            wants_playback: transport.wants_playback(),
            fault: transport.fault(),
            repeat: self.session.repeat(),
            shuffled: self.session.queue().shuffled(),
            position: transport.position(),
            duration: transport.duration(),
            volume: self.saved.volume.unwrap_or(1.0),
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            message: self.message.clone(),
        };
        if self.status.update(status) {
            let mut inbox = self.shared.inbox.lock().unwrap();
            inbox.catalog_pending = self.catalog_dirty || self.browse_dirty;
            inbox.status = self.status.clone();
            drop(inbox);
            (self.shared.wake)();
        }
    }
}
