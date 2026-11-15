use super::*;

impl App {
    pub(super) fn input(&mut self, window: &mut Window) -> Result<(), String> {
        let Self {
            events,
            file_drop,
            files,
            command_error,
            error_revision,
            shell_state,
            folder_picker,
            audio_picker,
            ..
        } = self;
        for event in &mut *events {
            if let Some(drop) = file_drop.event(event, files.snapshot().account) {
                (*command_error) = match drop {
                    Err(message) => Some(message),
                    Ok(drop) => match files.drop_files(drop.user, drop.paths) {
                        Ok(()) => {
                            shell_state.apply(lw_ui::Intent::CloseQueue);
                            shell_state
                                .apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Files));
                            None
                        }
                        Err(_) => Some(
                            "Folder work is busy or unavailable. Wait or cancel it, then drop the files again.",
                        ),
                    },
                };
                (*error_revision) += 1;
            }
        }
        let folder_id = (*folder_picker).map(|(id, _)| id);
        if let Some(selection) = events::folder_selection(&mut (*folder_picker), &mut *events) {
            (*command_error) = match selection {
                Ok(None) => None,
                Ok(Some((purpose, path))) => window
                    .take_dialog_selection(folder_id.unwrap())
                    .map_err(std::io::Error::other)
                    .and_then(|mut selected| {
                        if selected.len() != 1 || selected[0].path() != path {
                            return Err(std::io::ErrorKind::InvalidInput.into());
                        }
                        let file = selected.pop().unwrap();
                        match purpose {
                            events::FolderPurpose::Add(user) => {
                                files.add_folder_selected(user, file)
                            }
                            events::FolderPurpose::Reconnect(target) => {
                                files.reconnect_folder_selected(target, file)
                            }
                        }
                    })
                    .err()
                    .map(|_| "Could not update this folder. Try again."),
                Err(message) => Some(message),
            };
            (*error_revision) += 1;
        }
        let audio_id = (*audio_picker).map(|(id, _)| id);
        if let Some(selection) = events::audio_selection(&mut (*audio_picker), &mut *events) {
            (*command_error) = match selection {
                Ok(None) => None,
                Ok(Some((user, paths))) => window
                    .take_dialog_selection(audio_id.unwrap())
                    .map_err(std::io::Error::other)
                    .and_then(|grants| files.add_audio_granted(user, paths, grants))
                    .err()
                    .map(
                        |_| "Could not add these audio files. Folder work is busy or unavailable.",
                    ),
                Err(message) => Some(message),
            };
            (*error_revision) += 1;
        }
        Ok(())
    }
}
