use super::*;

impl App {
    pub(super) fn intent(
        &mut self,
        intent: lw_ui::Intent,
        window: &mut Window,
        now: f64,
        seek_duration: f64,
    ) -> Result<Option<Schedule>, String> {
        let Self {
            player,
            files,
            shell_state,
            ui,
            artwork,
            command_error,
            error_revision,
            repaint,
            folder_picker,
            audio_picker,
            graphics_slot,
            ..
        } = self;
        let graphics = graphics_slot
            .as_mut()
            .ok_or("App graphics have already closed")?;
        let intent = if let lw_ui::Intent::ActivateCommand(key) = intent {
            let Some(intent) = shell_state.activate_command(
                player.catalog(),
                key,
                player.status().wants_playback,
                ui.tokens().foreground.r > 128,
            ) else {
                return Ok(Some(Schedule::NextFrame));
            };
            intent
        } else {
            intent
        };
        if let lw_ui::Intent::Files(request) = intent {
            let result = files.request(request);
            if result.is_ok() {
                // A sidebar source row browses from any page, as Swift's Source route does.
                if matches!(request, lw_ui::files::Request::Browse(_)) && !shell_state.files_visible() {
                    shell_state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Files));
                }
                shell_state.apply(lw_ui::Intent::CloseSourceMenu);
                if matches!(request, lw_ui::files::Request::Remove(_)) {
                    shell_state.apply(lw_ui::Intent::CloseRemoveSource);
                }
            }
            (*command_error) = result
                .err()
                .map(|_| "Folder work is busy or unavailable. Try again.");
            (*error_revision) += 1;
            (*repaint) = Some(now);
            return Ok(Some(Schedule::NextFrame));
        }
        if matches!(
            intent,
            lw_ui::Intent::ChooseFolder
                | lw_ui::Intent::ChooseAudio
                | lw_ui::Intent::ReconnectSource(_)
        ) {
            let audio = intent == lw_ui::Intent::ChooseAudio;
            let purpose = match intent {
                lw_ui::Intent::ReconnectSource(target) => {
                    Some(events::FolderPurpose::Reconnect(target))
                }
                _ => files.snapshot().account.map(events::FolderPurpose::Add),
            };
            shell_state.apply(lw_ui::Intent::CloseSourceMenu);
            if (*folder_picker).is_none()
                && (*audio_picker).is_none()
                && let Some(purpose) = purpose
            {
                let filters = if audio {
                    vec![pxui_sdl::DialogFilter {
                        name: "Audio files".into(),
                        pattern: "mp3;wav;m4a;flac;aac;aiff;aif".into(),
                    }]
                } else {
                    Vec::new()
                };
                let kind = if audio {
                    pxui_sdl::DialogKind::OpenFiles
                } else {
                    pxui_sdl::DialogKind::OpenFolder
                };
                match window.show_dialog(&filters, None, kind) {
                    Ok(id) => {
                        if audio {
                            if let events::FolderPurpose::Add(user) = purpose {
                                (*audio_picker) = Some((id, user));
                            }
                        } else {
                            (*folder_picker) = Some((id, purpose));
                        }
                        (*command_error) = None;
                    }
                    Err(_) => {
                        (*command_error) = Some("Could not open the music picker.");
                    }
                }
                (*error_revision) += 1;
                (*repaint) = Some(now);
            }
            return Ok(Some(Schedule::NextFrame));
        }
        if intent == lw_ui::Intent::Navigate(lw_ui::catalog::Route::Files) {
            (*command_error) = files
                .request(lw_ui::files::Request::Roots)
                .err()
                .map(|_| "Folders are unavailable. Try again.");
            (*error_revision) += 1;
        }
        if matches!(
            intent,
            lw_ui::Intent::Navigate(_)
                | lw_ui::Intent::TogglePanel
                | lw_ui::Intent::TogglePlayer
                | lw_ui::Intent::CloseQueue
        ) {
            graphics.images().cancel_requests();
            artwork.cancel().map_err(|error| error.to_string())?;
        }
        let mut browse = None;
        if matches!(
            intent,
            lw_ui::Intent::PrivacyPolicy | lw_ui::Intent::Support
        ) {
            let link = if matches!(intent, lw_ui::Intent::PrivacyPolicy) {
                lw_apple_sys::window::Link::PrivacyPolicy
            } else {
                lw_apple_sys::window::Link::Support
            };
            (*command_error) = lw_apple_sys::window::open_link(link)
                .err()
                .map(|_| "Could not open this link.");
            (*error_revision) += 1;
            (*repaint) = Some(now);
            return Ok(Some(Schedule::NextFrame));
        }
        let command = if shell_state.apply(intent) {
            if matches!(
                intent,
                lw_ui::Intent::Navigate(_)
                    | lw_ui::Intent::Search(_)
                    | lw_ui::Intent::ShowCommands
                    | lw_ui::Intent::CloseCommands
                    | lw_ui::Intent::CommandQuery(_)
                    | lw_ui::Intent::Sort(_)
            ) {
                browse = shell_state.request(player.catalog());
                None
            } else if matches!(intent, lw_ui::Intent::ShowSettings) {
                Some(Command::ReadStorage)
            } else {
                None
            }
        } else {
            Some(match intent {
                lw_ui::Intent::Play => Command::Play,
                lw_ui::Intent::Pause => Command::Pause,
                lw_ui::Intent::Stop => Command::Stop,
                lw_ui::Intent::Next => Command::Next,
                lw_ui::Intent::Previous => Command::Previous,
                lw_ui::Intent::PlayRow(index) => {
                    shell_state.apply(lw_ui::Intent::CloseSongMenu);
                    let Some(collection) = shell_state
                        .collection(player.catalog())
                        .filter(|collection| index < collection.ids.len())
                    else {
                        return Ok(Some(Schedule::NextFrame));
                    };
                    Command::Configure {
                        songs: collection.ids.clone(),
                        start: index,
                        autoplay: true,
                        context: Some(collection.context.clone()),
                    }
                }
                lw_ui::Intent::PlaySelection => {
                    let Some(collection) = shell_state.collection(player.catalog()) else {
                        return Ok(Some(Schedule::NextFrame));
                    };
                    let songs: std::sync::Arc<[i64]> =
                        shell_state.selected_song_ids(player.catalog()).collect();
                    if songs.is_empty() {
                        return Ok(Some(Schedule::NextFrame));
                    }
                    let command = Command::Configure {
                        songs,
                        start: 0,
                        autoplay: true,
                        context: Some(collection.context.clone()),
                    };
                    // Swift leaves selection mode after playing.
                    shell_state.apply(lw_ui::Intent::Select(lw_ui::Selection::Clear));
                    command
                }
                lw_ui::Intent::PlayCommand(key) => {
                    let Some(results) = shell_state.command_results(player.catalog()) else {
                        return Ok(Some(Schedule::NextFrame));
                    };
                    let Some(lw_ui::commands::Action::Play(index)) = results
                        .items
                        .iter()
                        .find(|item| item.key == key)
                        .map(|item| item.action)
                    else {
                        return Ok(Some(Schedule::NextFrame));
                    };
                    if index >= results.songs.len() {
                        return Ok(Some(Schedule::NextFrame));
                    }
                    let command = Command::Configure {
                        songs: results.songs.clone(),
                        start: index,
                        autoplay: true,
                        context: Some(results.context.clone()),
                    };
                    shell_state.apply(lw_ui::Intent::CloseCommands);
                    command
                }
                lw_ui::Intent::TogglePins => {
                    let Some(group) = shell_state.collection(player.catalog()) else {
                        return Ok(Some(Schedule::NextFrame));
                    };
                    let ids = if shell_state.route == lw_ui::catalog::Route::Find {
                        let Some(summary) = player
                            .catalog()
                            .view
                            .as_ref()
                            .and_then(|view| view.find.as_ref())
                        else {
                            return Ok(Some(Schedule::NextFrame));
                        };
                        &summary.eligible
                    } else {
                        &group.ids
                    };
                    if ids.is_empty() {
                        return Ok(Some(Schedule::NextFrame));
                    }
                    Command::TogglePins(ids.clone())
                }
                lw_ui::Intent::SetFavorite { song, on } => {
                    shell_state.apply(lw_ui::Intent::CloseSongMenu);
                    Command::SetFavorite { song, on }
                }
                lw_ui::Intent::PlayNext(id) => {
                    shell_state.apply(lw_ui::Intent::CloseSongMenu);
                    Command::InsertNext(targets(shell_state, player.catalog(), id))
                }
                lw_ui::Intent::AddToList { song, list } => {
                    shell_state.apply(lw_ui::Intent::CloseSongMenu);
                    Command::AddToList(list, targets(shell_state, player.catalog(), song))
                }
                lw_ui::Intent::RemoveFromList { song, list } => {
                    shell_state.apply(lw_ui::Intent::CloseSongMenu);
                    Command::RemoveFromList(list, targets(shell_state, player.catalog(), song))
                }
                lw_ui::Intent::KeepOffline(id) => {
                    shell_state.apply(lw_ui::Intent::CloseSongMenu);
                    Command::TogglePins(targets(shell_state, player.catalog(), id))
                }
                lw_ui::Intent::RetryDownload(id) => {
                    shell_state.apply(lw_ui::Intent::CloseSongMenu);
                    Command::RetryDownload(id)
                }
                lw_ui::Intent::SaveMetadata(edit) => Command::EditMetadata(Box::new(edit)),
                lw_ui::Intent::DeleteSong(id) => Command::DeleteSong(id),
                lw_ui::Intent::RevealSong(id) => {
                    shell_state.apply(lw_ui::Intent::CloseSongMenu);
                    Command::Reveal(id)
                }
                lw_ui::Intent::ShareSong(id, at) => {
                    shell_state.apply(lw_ui::Intent::CloseSongMenu);
                    Command::Share(targets(shell_state, player.catalog(), id), at)
                }
                lw_ui::Intent::AppendSong(id) => {
                    shell_state.apply(lw_ui::Intent::CloseSongMenu);
                    Command::Append(targets(shell_state, player.catalog(), id))
                }
                lw_ui::Intent::ShuffleAll => {
                    let Some(collection) = player
                        .catalog()
                        .all
                        .as_ref()
                        .filter(|collection| !collection.ids.is_empty())
                    else {
                        return Ok(Some(Schedule::NextFrame));
                    };
                    Command::ShuffleAll {
                        songs: collection.ids.clone(),
                        seed: now.to_bits(),
                        context: None,
                    }
                }
                lw_ui::Intent::ShuffleCollection => {
                    let Some(collection) = shell_state
                        .collection(player.catalog())
                        .filter(|collection| !collection.ids.is_empty())
                    else {
                        return Ok(Some(Schedule::NextFrame));
                    };
                    Command::ShuffleAll {
                        songs: collection.ids.clone(),
                        seed: now.to_bits(),
                        context: Some(collection.context.clone()),
                    }
                }
                lw_ui::Intent::Shuffle => Command::Shuffle {
                    enabled: !player.status().shuffled,
                    seed: now.to_bits(),
                },
                lw_ui::Intent::Repeat => Command::Repeat(match player.status().repeat {
                    lw_core::queue::RepeatMode::None => lw_core::queue::RepeatMode::All,
                    lw_core::queue::RepeatMode::All => lw_core::queue::RepeatMode::One,
                    lw_core::queue::RepeatMode::One => lw_core::queue::RepeatMode::None,
                }),
                lw_ui::Intent::Volume(value) => Command::Volume(value),
                lw_ui::Intent::RefreshLibrary => Command::RefreshCatalog,
                lw_ui::Intent::Setting(change) => Command::Setting(change),
                lw_ui::Intent::PlayQueue(occurrence) => Command::Select {
                    occurrence,
                    autoplay: true,
                },
                lw_ui::Intent::RemoveQueue(occurrence) => Command::Remove(occurrence),
                lw_ui::Intent::MoveQueue { occurrence, before } => {
                    Command::MoveUpcoming { occurrence, before }
                }
                lw_ui::Intent::ClearQueue => Command::ClearUpcoming,
                lw_ui::Intent::UndoQueue => Command::UndoClear,
                lw_ui::Intent::SaveQueue(name) => Command::SaveQueue(name),
                lw_ui::Intent::CreateList { name, song } => Command::CreateList(
                    name,
                    song.map_or_else(
                        || std::sync::Arc::from([]),
                        |song| targets(shell_state, player.catalog(), song),
                    ),
                ),
                lw_ui::Intent::RenameList { list, name } => Command::RenameList(list, name),
                lw_ui::Intent::DeleteList(list) => Command::DeleteList(list),
                lw_ui::Intent::MoveListRow { from, to } => {
                    let Some((list, ids)) =
                        shell_state
                            .collection(player.catalog())
                            .and_then(|collection| match &*collection.context {
                                lw_core::context::PlaybackContext::Playlist { id, .. } => {
                                    Some((*id, &collection.ids))
                                }
                                _ => None,
                            })
                    else {
                        return Ok(Some(Schedule::NextFrame));
                    };
                    if from >= ids.len() || to >= ids.len() {
                        return Ok(Some(Schedule::NextFrame));
                    }
                    let mut order = ids.to_vec();
                    let moved = order.remove(from);
                    order.insert(to, moved);
                    Command::ReorderList(list, order.into())
                }
                lw_ui::Intent::SeekFraction(fraction) => Command::Seek(fraction * seek_duration),
                lw_ui::Intent::Navigate(_)
                | lw_ui::Intent::SongMenu(_, _, _, _)
                | lw_ui::Intent::CloseSongMenu
                | lw_ui::Intent::SongMenuLists
                | lw_ui::Intent::SelectRow(_)
                | lw_ui::Intent::Select(_)
                | lw_ui::Intent::Search(_)
                | lw_ui::Intent::SearchTooLong
                | lw_ui::Intent::ShowCommands
                | lw_ui::Intent::CloseCommands
                | lw_ui::Intent::CommandQuery(_)
                | lw_ui::Intent::CommandTooLong
                | lw_ui::Intent::CommandSelect(_)
                | lw_ui::Intent::ShowShortcuts
                | lw_ui::Intent::CloseShortcuts
                | lw_ui::Intent::Sort(_)
                | lw_ui::Intent::SortAlbums(_)
                | lw_ui::Intent::TogglePanel
                | lw_ui::Intent::TogglePlayer
                | lw_ui::Intent::ShowQueue
                | lw_ui::Intent::CloseQueue
                | lw_ui::Intent::NamePlaylist
                | lw_ui::Intent::NewList(_)
                | lw_ui::Intent::RenameListDialog(..)
                | lw_ui::Intent::ListMenu(..)
                | lw_ui::Intent::CloseListMenu
                | lw_ui::Intent::ConfirmDeleteList(_)
                | lw_ui::Intent::CloseDeleteList
                | lw_ui::Intent::EditPlaylistName(_)
                | lw_ui::Intent::PlaylistNameTooLong
                | lw_ui::Intent::CancelPlaylistName => {
                    unreachable!("handled by ShellState")
                }
                lw_ui::Intent::ShowSettings
                | lw_ui::Intent::CloseSettings
                | lw_ui::Intent::ShowPrivacy
                | lw_ui::Intent::ClosePrivacy
                | lw_ui::Intent::SettingsMenu(_, _)
                | lw_ui::Intent::CloseSettingsMenu => {
                    unreachable!("handled by ShellState")
                }
                lw_ui::Intent::PrivacyPolicy | lw_ui::Intent::Support => {
                    unreachable!("handled above")
                }
                lw_ui::Intent::ActivateCommand(_) => unreachable!("resolved above"),
                lw_ui::Intent::Files(_)
                | lw_ui::Intent::ChooseFolder
                | lw_ui::Intent::ChooseAudio
                | lw_ui::Intent::ReconnectSource(_) => {
                    unreachable!("handled above")
                }
                lw_ui::Intent::SourceMenu(_, _)
                | lw_ui::Intent::CloseSourceMenu
                | lw_ui::Intent::ConfirmRemoveSource(_)
                | lw_ui::Intent::CloseRemoveSource
                | lw_ui::Intent::OpenMetadata(_)
                | lw_ui::Intent::MetadataField(..)
                | lw_ui::Intent::MetadataTooLong
                | lw_ui::Intent::CloseMetadata
                | lw_ui::Intent::ConfirmDeleteSong(_)
                | lw_ui::Intent::CloseDeleteSong => {
                    unreachable!("handled by ShellState")
                }
            })
        };
        let result = if let Some(request) = browse {
            player.browse(request)
        } else {
            command.map_or(Ok(0), |command| player.command(command))
        };
        (*command_error) = match result {
            Ok(_) => {
                match intent {
                    lw_ui::Intent::SaveQueue(_)
                    | lw_ui::Intent::CreateList { .. }
                    | lw_ui::Intent::RenameList { .. } => {
                        shell_state.apply(lw_ui::Intent::CancelPlaylistName);
                    }
                    lw_ui::Intent::SaveMetadata(_) => {
                        shell_state.apply(lw_ui::Intent::CloseMetadata);
                    }
                    lw_ui::Intent::DeleteSong(_) => {
                        shell_state.apply(lw_ui::Intent::CloseDeleteSong);
                    }
                    lw_ui::Intent::DeleteList(_) => {
                        shell_state.apply(lw_ui::Intent::CloseDeleteList);
                        // Routes address lists by position, which the deletion shifts.
                        if matches!(shell_state.route, lw_ui::catalog::Route::Collection(_)) {
                            shell_state
                                .apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Songs));
                        }
                    }
                    _ => {}
                }
                None
            }
            Err(error) => Some(match error.kind() {
                std::io::ErrorKind::WouldBlock => "Playback is busy. Try again.",
                std::io::ErrorKind::InvalidInput => "This action is unavailable.",
                std::io::ErrorKind::BrokenPipe => "Playback is unavailable.",
                _ => return Err(error.to_string()),
            }),
        };
        (*error_revision) += 1;
        (*repaint) = Some(now);
        Ok(None)
    }
}

/// A menu action on a song that is part of a multi-row selection applies to the whole selection.
fn targets(
    state: &lw_ui::ShellState,
    catalog: &lw_ui::catalog::Catalog,
    song: i64,
) -> std::sync::Arc<[i64]> {
    let selected = || state.selected_song_ids(catalog);
    if selected().nth(1).is_some() && selected().any(|id| id == song) {
        selected().collect()
    } else {
        std::sync::Arc::from([song])
    }
}

#[cfg(test)]
mod tests {
    use lw_ui::{Intent, Selection, ShellState, catalog::Catalog};

    #[test]
    fn menu_targets_follow_a_multi_row_selection_that_contains_the_song() {
        let mut catalog = Catalog::new(
            (1..=4)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Song {id}"),
                    ..Default::default()
                })
                .collect(),
        );
        catalog.prepare_tables().unwrap();
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(lw_ui::catalog::Route::Songs));
        assert_eq!(&*super::targets(&state, &catalog, 2), &[2]);
        state.apply(Intent::Select(Selection::Toggle(0)));
        assert_eq!(
            &*super::targets(&state, &catalog, 2),
            &[2],
            "one row is not a group"
        );
        state.apply(Intent::Select(Selection::Toggle(2)));
        let group = super::targets(
            &state,
            &catalog,
            state.selected_song_ids(&catalog).next().unwrap(),
        );
        assert_eq!(group.len(), 2);
        assert_eq!(
            &*super::targets(&state, &catalog, 99),
            &[99],
            "outside the selection"
        );
    }
}
