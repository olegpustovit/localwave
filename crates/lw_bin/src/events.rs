//! Native dialogs use exact one-shot request IDs and the account captured at opening. Take
//! Window::take_dialog_selection before the next pump and transfer original FileSelections to
//! FilesWorker. Match every path/count before submission; invalid grants create no private files.
//! Never reconstruct iOS provider access from portable event paths. Bookmark/grant mechanics belong
//! to pxui/file_access_sys; source/account validation and import policy belong here.
//!
//! Consume completed host drops. Capture the known account on first hover/file notification;
//! if Files has never opened, its worker resolves identity instead of guessing. Focus loss only
//! dismisses hover. Validate the whole bounded set before one worker submission; empty/cancel
//! cycles allocate nothing. Native batching and path ownership belong to pxui_sdl.
//!
use input::Event as HostEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderPurpose {
    Add(i64),
    Reconnect(lw_ui::files::Target),
}

type AudioSelection = (i64, Vec<std::path::PathBuf>);

#[derive(Debug, PartialEq, Eq)]
pub struct DroppedFiles {
    pub user: Option<i64>,
    pub paths: Vec<std::path::PathBuf>,
}

#[derive(Default)]
pub struct FileDrop {
    active: bool,
    hovered: bool,
    user: Option<i64>,
}
impl FileDrop {
    pub fn hovered(&self) -> bool {
        self.hovered
    }

    pub fn event(
        &mut self,
        event: &mut HostEvent,
        user: Option<i64>,
    ) -> Option<Result<DroppedFiles, &'static str>> {
        if matches!(
            event,
            HostEvent::FileDragMoved(_) | HostEvent::FilesDropped { .. }
        ) && !self.active
        {
            self.active = true;
            self.user = user;
        }
        match event {
            HostEvent::FileDragMoved(_) => self.hovered = true,
            HostEvent::FilesDropped { paths, .. } => {
                let user = self.user.take();
                self.active = false;
                self.hovered = false;
                return match std::mem::replace(paths, Ok(Vec::new())) {
                    Ok(paths) if paths.is_empty() => None,
                    Ok(paths)
                        if paths.len() <= 1_000_000
                            && paths
                                .iter()
                                .fold(0usize, |bytes, path| bytes.saturating_add(path.len()))
                                <= 64 * 1024 * 1024
                            && paths.iter().all(|path| {
                                !path.contains('\0') && std::path::Path::new(path).is_absolute()
                            }) =>
                    {
                        Some(Ok(DroppedFiles {
                            user,
                            paths: paths.into_iter().map(std::path::PathBuf::from).collect(),
                        }))
                    }
                    _ => Some(Err(
                        "Choose supported audio files or original music folders. The drop was invalid or too large.",
                    )),
                };
            }
            HostEvent::FileDragEnded => {
                self.active = false;
                self.hovered = false;
                self.user = None;
            }
            // Finder can change keyboard focus without cancelling its drop delivery.
            HostEvent::FocusLost => self.hovered = false,
            _ => {}
        }
        None
    }
}

pub fn audio_selection(
    pending: &mut Option<(i64, i64)>,
    events: &mut [HostEvent],
) -> Option<Result<Option<AudioSelection>, &'static str>> {
    let (picker, user) = (*pending)?;
    for event in events {
        if let HostEvent::FileDialog { id, result } = event
            && *id == picker
        {
            *pending = None;
            return Some(match result {
                Ok(paths) if paths.is_empty() => Ok(None),
                Ok(paths)
                    if paths.len() <= 1_000_000
                        && paths.iter().map(String::len).sum::<usize>() <= 64 * 1024 * 1024
                        && paths
                            .iter()
                            .all(|path| std::path::Path::new(path).is_absolute()) =>
                {
                    Ok(Some((
                        user,
                        std::mem::take(paths)
                            .into_iter()
                            .map(std::path::PathBuf::from)
                            .collect(),
                    )))
                }
                _ => Err("Could not choose these audio files. Try again."),
            });
        }
    }
    None
}

pub fn folder_selection(
    pending: &mut Option<(i64, FolderPurpose)>,
    events: &mut [HostEvent],
) -> Option<Result<Option<(FolderPurpose, std::path::PathBuf)>, &'static str>> {
    let (picker, purpose) = (*pending)?;
    for event in events {
        if let HostEvent::FileDialog { id, result } = event
            && *id == picker
        {
            *pending = None;
            return Some(match result {
                Ok(paths) if paths.is_empty() => Ok(None),
                Ok(paths) if paths.len() == 1 => {
                    let path = std::path::PathBuf::from(paths.pop().unwrap());
                    if path.is_absolute() {
                        Ok(Some((purpose, path)))
                    } else {
                        Err("The selected folder has an invalid path.")
                    }
                }
                _ => Err("Could not choose this folder. Try again."),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_drops_capture_owner_validate_whole_batches_and_reuse_empty_cancel() {
        let mut drop = FileDrop::default();
        let completed = |paths| HostEvent::FilesDropped {
            position: (10.0, 20.0),
            paths,
        };
        assert!(
            drop.event(&mut HostEvent::FileDragMoved((10.0, 20.0)), Some(42))
                .is_none()
        );
        assert!(drop.hovered());
        assert!(drop.event(&mut HostEvent::FocusLost, Some(43)).is_none());
        assert!(!drop.hovered());
        let mut batch = completed(Ok(vec!["/Music/名前.flac".into(), "/Music/Folder".into()]));
        assert_eq!(
            drop.event(&mut batch, Some(43)),
            Some(Ok(DroppedFiles {
                user: Some(42),
                paths: vec!["/Music/名前.flac".into(), "/Music/Folder".into()]
            }))
        );
        assert!(
            drop.event(&mut batch, Some(43)).is_none(),
            "completed payload is consumed once"
        );
        for paths in [
            Ok(vec!["/valid.flac".into(), "relative.flac".into()]),
            Ok(vec!["/valid.flac".into(), "/invalid\0file.m4a".into()]),
            Err("transport limit exceeded".into()),
            Ok(vec![String::new(); 1_000_001]),
            Ok(vec![
                String::from_utf8(vec![b'/'; 64 * 1024 * 1024 + 1]).unwrap(),
            ]),
        ] {
            let mut batch = completed(paths);
            assert!(drop.event(&mut batch, Some(99)).unwrap().is_err());
            assert_eq!(
                batch,
                completed(Ok(Vec::new())),
                "invalid payload cannot reach frame encoding or a worker"
            );
            assert!(!drop.hovered());
        }
        drop.event(&mut HostEvent::FileDragMoved((1.0, 2.0)), None);
        let mut batch = completed(Ok(vec!["/first.flac".into()]));
        assert_eq!(
            drop.event(&mut batch, Some(99)).unwrap().unwrap().user,
            None
        );
        mem_sys::local_reset();
        for _ in 0..100 {
            assert!(
                drop.event(&mut HostEvent::FileDragMoved((12.0, 24.0)), Some(42))
                    .is_none()
            );
            assert!(
                drop.event(&mut HostEvent::FileDragEnded, Some(42))
                    .is_none()
            );
            assert!(
                drop.event(&mut HostEvent::PointerMoved((12.0, 24.0)), Some(42))
                    .is_none()
            );
            assert!(drop.event(&mut completed(Ok(Vec::new())), None).is_none());
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }

    #[test]
    fn audio_dialogs_preserve_order_account_and_one_shot_validation() {
        let mut pending = Some((17, 42));
        let mut unrelated = [HostEvent::FileDialog {
            id: 18,
            result: Ok(vec!["/other.flac".into()]),
        }];
        assert!(audio_selection(&mut pending, &mut unrelated).is_none());
        assert_eq!(pending, Some((17, 42)));
        let mut chosen = [HostEvent::FileDialog {
            id: 17,
            result: Ok(vec!["/Music/名前.flac".into(), "/Music/Björk.m4a".into()]),
        }];
        assert_eq!(
            audio_selection(&mut pending, &mut chosen),
            Some(Ok(Some((
                42,
                vec!["/Music/名前.flac".into(), "/Music/Björk.m4a".into()]
            ))))
        );
        assert!(audio_selection(&mut pending, &mut chosen).is_none());
        for result in [
            Ok(Vec::new()),
            Err("failure".into()),
            Ok(vec!["/valid.flac".into(), "relative.m4a".into()]),
        ] {
            pending = Some((19, 43));
            let cancelled = result.as_ref().is_ok_and(Vec::is_empty);
            let outcome = audio_selection(
                &mut pending,
                &mut [HostEvent::FileDialog { id: 19, result }],
            )
            .unwrap();
            assert!(if cancelled {
                outcome == Ok(None)
            } else {
                outcome.is_err()
            });
            assert!(pending.is_none());
        }
        mem_sys::local_reset();
        for _ in 0..100 {
            assert!(audio_selection(&mut pending, &mut unrelated).is_none());
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }

    #[test]
    fn folder_dialogs_match_once_preserve_owner_and_distinguish_cancel_from_failure() {
        let mut pending = Some((7, FolderPurpose::Add(42)));
        let mut unrelated = [HostEvent::FileDialog {
            id: 8,
            result: Ok(vec!["/Music/Other".into()]),
        }];
        assert!(folder_selection(&mut pending, &mut unrelated).is_none());
        assert_eq!(pending, Some((7, FolderPurpose::Add(42))));
        let mut chosen = [HostEvent::FileDialog {
            id: 7,
            result: Ok(vec!["/Music/名前".into()]),
        }];
        assert_eq!(
            folder_selection(&mut pending, &mut chosen),
            Some(Ok(Some((FolderPurpose::Add(42), "/Music/名前".into()))))
        );
        assert!(pending.is_none());
        assert!(folder_selection(&mut pending, &mut chosen).is_none());
        for result in [
            Ok(Vec::new()),
            Err("injected failure".into()),
            Ok(vec!["relative".into()]),
            Ok(vec!["/one".into(), "/two".into()]),
        ] {
            pending = Some((9, FolderPurpose::Add(43)));
            let cancelled = result.as_ref().is_ok_and(Vec::is_empty);
            let outcome =
                folder_selection(&mut pending, &mut [HostEvent::FileDialog { id: 9, result }])
                    .unwrap();
            if cancelled {
                assert_eq!(outcome, Ok(None));
            } else {
                assert!(outcome.is_err());
            }
            assert!(pending.is_none());
        }
        mem_sys::local_reset();
        for _ in 0..100 {
            assert!(folder_selection(&mut pending, &mut unrelated).is_none());
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        let target = lw_ui::files::Target {
            user: 43,
            source: 12,
            root: 99,
        };
        pending = Some((10, FolderPurpose::Reconnect(target)));
        assert_eq!(
            folder_selection(
                &mut pending,
                &mut [HostEvent::FileDialog {
                    id: 10,
                    result: Ok(vec!["/Music/Reconnected".into()])
                }]
            ),
            Some(Ok(Some((
                FolderPurpose::Reconnect(target),
                "/Music/Reconnected".into()
            ))))
        );
    }

    #[test]
    fn changing_event_counts_retain_dialog_buffers_without_allocating() {
        let events = [
            HostEvent::PointerMoved((10.0, 20.0)),
            HostEvent::KeyPressed(input::Key::Space),
            HostEvent::FileDialog {
                id: 7,
                result: Ok(vec![
                    "/Music/Björk.flac".into(),
                    "/Music/日本語.flac".into(),
                ]),
            },
        ];
        let mut encoded = pxui_host::Events::default();
        encoded.encode(&events);
        mem_sys::local_reset();
        for count in [0, 1, 3, 2, 0, 3] {
            let wire = encoded.encode(&events[..count]);
            assert_eq!(wire.len(), count);
        }
        let allocations = mem_sys::local_alloc_count();
        assert_eq!(allocations, 0);
        let wire = encoded.as_slice();
        assert_eq!(wire[2].kind, input::wire::FILE_DIALOG);
        assert_eq!(wire[2].value, 7.0);
        assert!(!wire[2].text.ptr.is_null());
        assert_eq!(
            wire[2].text.len,
            "/Music/Björk.flac\0/Music/日本語.flac".len()
        );
        let empty = [HostEvent::FileDialog {
            id: 8,
            result: Ok(Vec::new()),
        }];
        let wire = encoded.encode(&empty);
        assert_eq!(wire[0].text.len, 0);
    }
}
