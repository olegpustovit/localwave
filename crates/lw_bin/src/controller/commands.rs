//! Dispatch retains the common fencing, queue persistence and audio-notice acknowledgement.
use super::*;

impl Worker {
    pub(super) fn command(&mut self, mut envelope: Envelope) -> io::Result<()> {
        if let Command::ShuffleAll {
            songs,
            seed,
            context,
        } = &envelope.command
        {
            let mut songs = songs.clone();
            lw_core::queue::shuffle_songs(Arc::make_mut(&mut songs), *seed);
            envelope.command = Command::Configure {
                songs,
                start: 0,
                autoplay: true,
                context: context.clone(),
            };
        }
        if matches!(envelope.command, Command::Toggle) {
            envelope.command = if self.session.transport().wants_playback() {
                Command::Pause
            } else {
                Command::Play
            };
        }
        self.revision = self.revision.max(envelope.revision);
        self.preparation = self.preparation.max(envelope.preparation);
        let session_pause =
            matches!(&envelope.command, Command::AudioSession(event) if event.pauses());
        if (session_pause
            || matches!(
                envelope.command,
                Command::Next | Command::Previous | Command::Pause | Command::Stop
            ))
            && let Some(ticket) = self.pending_selection()
        {
            self.commit_selection(ticket);
        }
        let adding = match &envelope.command {
            Command::Append(_) => 1,
            Command::InsertNext(songs) => songs.len(),
            _ => 0,
        };
        if adding > 0 && self.session.queue().entries().len() + adding > 1_000_000 {
            self.message = Arc::from("The queue is full");
            return Ok(());
        }
        let keep_undo = matches!(
            envelope.command,
            Command::ClearUpcoming | Command::UndoClear
        );
        if envelope.command.interrupts() {
            self.intent = false;
        }
        let mut structural = false;
        let action = match envelope.command {
            command @ (Command::Select { .. }
            | Command::Play
            | Command::Toggle
            | Command::AudioSession(_)
            | Command::Pause
            | Command::Stop
            | Command::Next
            | Command::Previous
            | Command::Seek(_)
            | Command::Repeat(_)) => self.playback_command(command)?,
            command @ (Command::Configure { .. }
            | Command::ShuffleAll { .. }
            | Command::Shuffle { .. }
            | Command::Reorder { .. }
            | Command::MoveUpcoming { .. }
            | Command::MoveEntry { .. }
            | Command::Remove(_)
            | Command::Append(_)
            | Command::InsertNext(_)
            | Command::ClearUpcoming
            | Command::UndoClear) => self.queue_command(command, &mut structural)?,
            command @ (Command::Volume(_) | Command::ReadStorage | Command::Setting(_)) => {
                self.settings_command(command)?
            }
            command @ (Command::SaveQueue(_)
            | Command::DismissNotice
            | Command::RefreshCatalog
            | Command::TogglePins(_)
            | Command::SetFavorite { .. }
            | Command::AddToList(..)
            | Command::RemoveFromList(..)
            | Command::RetryDownload(_)
            | Command::Reveal(_)) => self.library_command(command)?,
        };
        self.action(action);
        if structural && !keep_undo {
            self.undo = None;
        }
        if structural {
            self.queue_dirty = true;
            self.saved.song_ids.clear();
            self.saved.song_ids.extend(
                self.session
                    .queue()
                    .entries()
                    .iter()
                    .map(|entry| entry.song_id),
            );
            self.saved.current_index = self.session.queue().current_index().unwrap_or(0) as i64;
            self.library.save_player_state(&self.saved)?;
        }
        if let Some(notice) = envelope.audio_notice {
            notice.mark_processed();
        }
        Ok(())
    }
    fn playback_command(&mut self, command: Command) -> io::Result<Action> {
        Ok(match command {
            Command::Select {
                occurrence,
                autoplay,
            } => self.session.select(occurrence, autoplay),
            Command::Play => {
                self.message = self.empty.clone();
                self.session.play()
            }
            Command::Toggle => {
                unreachable!("Toggle resolves before committing a pending selection")
            }
            Command::AudioSession(event) => match event {
                AudioEvent::Began => {
                    self.session.interruption_began();
                    Action::None
                }
                AudioEvent::Ended { resume } => self.session.interruption_ended(resume),
                AudioEvent::OutputUnavailable => {
                    self.session.output_unavailable();
                    Action::None
                }
                AudioEvent::Reset => {
                    self.session_error(io::Error::other(
                        "The audio service restarted. Play to try again.",
                    ));
                    Action::None
                }
                AudioEvent::Overflow => {
                    self.session_error(io::Error::other(
                        "Audio-session notifications overflowed; playback was paused",
                    ));
                    Action::None
                }
            },
            Command::Pause => {
                self.session.pause();
                Action::None
            }
            Command::Stop => {
                self.session.stop();
                self.advance = None;
                self.seek = None;
                Action::Stop
            }
            Command::Next => self.session.next_song(),
            Command::Previous => self.session.previous(),
            Command::Seek(seconds) => {
                self.seek = Some(seconds.max(0.0));
                Action::None
            }
            Command::Repeat(mode) => {
                self.session.set_repeat(mode);
                Action::None
            }
            _ => unreachable!("command family dispatched above"),
        })
    }
    fn queue_command(&mut self, command: Command, structural: &mut bool) -> io::Result<Action> {
        Ok(match command {
            Command::Configure {
                songs,
                start,
                autoplay,
                context,
            } => {
                let context_data = context
                    .as_ref()
                    .map(|value| self.library.playback_context_data(value))
                    .transpose()?;
                self.session.configure(&songs, start);
                self.context = context.unwrap_or_else(|| Arc::new(PlaybackContext::Library));
                self.saved.context_data = context_data;
                self.undo = None;
                self.advance = None;
                self.seek = None;
                self.message = self.empty.clone();
                *structural = true;
                self.session
                    .queue()
                    .current()
                    .map_or(Action::None, |entry| {
                        self.session.select(entry.id, autoplay)
                    })
            }
            Command::ShuffleAll { .. } => unreachable!("translated to Configure on the worker"),
            Command::Shuffle { enabled, seed } => {
                self.session.set_shuffle(enabled, seed);
                *structural = true;
                Action::None
            }
            Command::Reorder {
                indices,
                destination,
            } => {
                *structural = self.session.reorder(&indices, destination);
                Action::None
            }
            Command::MoveUpcoming { occurrence, before } => {
                *structural = self.session.move_upcoming(occurrence, before);
                Action::None
            }
            Command::MoveEntry { occurrence, before } => {
                *structural = self.session.move_entry(occurrence, before);
                Action::None
            }
            Command::Remove(occurrence) => {
                *structural = self.session.remove(occurrence);
                Action::None
            }
            Command::Append(song) => {
                self.session.append(song);
                *structural = true;
                Action::None
            }
            Command::InsertNext(songs) => {
                let count = self.session.queue().entries().len();
                let after = self.session.queue().current_index().map(|index| index + 1);
                for song in songs.iter() {
                    self.session.append(*song);
                }
                // An empty queue makes the first append current; nothing needs to move.
                if let (Some(after), true) = (after, count > 0) {
                    let moved: Vec<usize> = (count..count + songs.len()).collect();
                    self.session.reorder(&moved, after.min(count));
                }
                *structural = true;
                Action::None
            }
            Command::ClearUpcoming => {
                if self
                    .session
                    .queue()
                    .current_index()
                    .is_some_and(|index| index + 1 < self.session.queue().entries().len())
                {
                    self.undo = Some(self.session.queue().entries().to_vec());
                    self.session.clear_upcoming();
                    *structural = true;
                }
                Action::None
            }
            Command::UndoClear => {
                if let Some(entries) = self.undo.take() {
                    *structural = self.session.restore(&entries);
                }
                Action::None
            }
            _ => unreachable!("command family dispatched above"),
        })
    }
    fn settings_command(&mut self, command: Command) -> io::Result<Action> {
        Ok(match command {
            Command::Volume(volume) => {
                if self.saved.volume != Some(volume) {
                    self.library.save_player_volume(volume)?;
                    self.saved.volume = Some(volume);
                }
                Action::None
            }
            Command::ReadStorage => {
                self.storage_pending = true;
                Action::None
            }
            Command::Setting(change) => {
                let next = self.settings.applying(change)?;
                if next != self.settings {
                    match self.library.save_settings(&next) {
                        Ok(()) => {
                            self.storage_pending |= next.cache_gb != self.settings.cache_gb;
                            if next.download_ahead != self.settings.download_ahead {
                                self.deferred = [None; 3];
                            }
                            self.settings = next;
                            self.settings_message = self.empty.clone();
                        }
                        Err(error) => {
                            eprintln!("LocalWave settings: {error}");
                            self.settings_message =
                                Arc::from("Could not save this setting. Try again.");
                        }
                    }
                }
                Action::None
            }
            _ => unreachable!("command family dispatched above"),
        })
    }
    fn library_command(&mut self, command: Command) -> io::Result<Action> {
        Ok(match command {
            Command::SaveQueue(name) => {
                if self.saved.song_ids.is_empty() {
                    self.message =
                        Arc::from("The queue is empty. Add tracks before saving a playlist.");
                } else {
                    match self.library.repository.create_playlist_with_songs(
                        name.as_str().trim(),
                        &self.saved.song_ids,
                        now(),
                    ) {
                        Ok(_) => {
                            self.dirty_catalog();
                            self.metadata_id = None;
                            self.message = Arc::from("Playlist saved.");
                        }
                        Err(error) => {
                            eprintln!("LocalWave save queue: {error}");
                            self.message = Arc::from("Could not save the playlist. Try again.");
                        }
                    }
                }
                Action::None
            }
            Command::DismissNotice => {
                self.session.dismiss_notice();
                self.message = self.empty.clone();
                Action::None
            }
            Command::RefreshCatalog => {
                self.dirty_catalog();
                self.copies_pending = true;
                Action::None
            }
            Command::TogglePins(songs) => {
                match self.library.toggle_pins(&songs) {
                    Ok(pinned) => {
                        self.copies_pending |= pinned;
                        self.dirty_catalog();
                        self.message = self.empty.clone();
                    }
                    Err(error) => {
                        eprintln!("LocalWave keep offline: {error}");
                        self.message = Arc::from(
                            "Could not change kept songs. Refresh or reconnect the source and try again.",
                        );
                    }
                }
                Action::None
            }
            Command::SetFavorite { song, on } => {
                match self.library.set_favorite(song, on) {
                    Ok(()) => {
                        self.dirty_catalog();
                        self.message = self.empty.clone();
                    }
                    Err(error) => {
                        eprintln!("LocalWave favorite: {error}");
                        self.message =
                            Arc::from("Could not change Favorites. Refresh and try again.");
                    }
                }
                Action::None
            }
            Command::AddToList(list, songs) => {
                let result = self.library.add_to_list(list, &songs);
                self.list_result(result, "add to the list", "Added to the list.")
            }
            Command::RemoveFromList(list, songs) => {
                let result = self.library.remove_from_list(list, &songs);
                self.list_result(result, "remove from the list", "")
            }
            Command::RetryDownload(song) => {
                match self.library.retry_download(song) {
                    Ok(()) => {
                        self.copies_pending = true;
                        self.dirty_catalog();
                        self.message = self.empty.clone();
                    }
                    Err(error) => {
                        eprintln!("LocalWave retry download: {error}");
                        self.message =
                            Arc::from("Could not retry this download. Refresh and try again.");
                    }
                }
                Action::None
            }
            Command::Reveal(song) => {
                if let Err(error) = self.library.reveal_song(song, now()) {
                    eprintln!("LocalWave reveal: {error}");
                    self.message = Arc::from(
                        "Could not show this file. Reconnect its source folder and try again.",
                    );
                }
                Action::None
            }
            _ => unreachable!("command family dispatched above"),
        })
    }
    fn list_result(&mut self, result: io::Result<()>, what: &str, done: &str) -> Action {
        match result {
            Ok(()) => {
                self.dirty_catalog();
                self.message = if done.is_empty() {
                    self.empty.clone()
                } else {
                    Arc::from(done)
                };
            }
            Err(error) => {
                eprintln!("LocalWave {what}: {error}");
                self.message = Arc::from(format!("Could not {what}. Refresh and try again."));
            }
        }
        Action::None
    }
}
