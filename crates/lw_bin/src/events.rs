use input::Event as HostEvent;
use pxui::{Event, Str};

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
    paths: Vec<std::path::PathBuf>,
    bytes: usize,
    invalid: bool,
}
impl FileDrop {
    pub fn hovered(&self) -> bool {
        self.hovered
    }

    /// SDL completes a set after its file events, which may span more than one pump.
    pub fn event(
        &mut self,
        event: &mut HostEvent,
        user: Option<i64>,
    ) -> Option<Result<DroppedFiles, &'static str>> {
        if matches!(
            event,
            HostEvent::FileDragMoved(_) | HostEvent::FileDropped(_)
        ) && !self.active
        {
            self.active = true;
            self.user = user;
        }
        match event {
            HostEvent::FileDragMoved(_) => self.hovered = true,
            HostEvent::FileDropped(path) => {
                self.bytes = self.bytes.saturating_add(path.len());
                if self.invalid
                    || self.paths.len() == 1_000_000
                    || self.bytes > 64 * 1024 * 1024
                    || path.contains('\0')
                    || !std::path::Path::new(path).is_absolute()
                {
                    self.invalid = true;
                } else {
                    self.paths
                        .push(std::path::PathBuf::from(std::mem::take(path)));
                }
            }
            HostEvent::FileDragEnded => {
                let outcome = if self.invalid {
                    Some(Err(
                        "Choose supported audio files or original music folders. The drop was invalid or too large.",
                    ))
                } else if self.paths.is_empty() {
                    None
                } else {
                    Some(Ok(DroppedFiles {
                        user: self.user,
                        paths: std::mem::take(&mut self.paths),
                    }))
                };
                self.active = false;
                self.hovered = false;
                self.user = None;
                self.bytes = 0;
                self.invalid = false;
                self.paths.clear();
                return outcome;
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

pub fn encode(events: &[HostEvent], scratch: &mut Vec<String>, wire: &mut Vec<Event>) {
    wire.clear();
    if scratch.len() < events.len() {
        scratch.resize_with(events.len(), String::new);
    }
    for (event, text) in events.iter().zip(scratch.iter_mut()) {
        text.clear();
        let raw = if let HostEvent::FileDialog {
            id,
            result: Ok(paths),
        } = event
        {
            for (index, path) in paths.iter().enumerate() {
                if index > 0 {
                    text.push('\0');
                }
                text.push_str(path);
            }
            input::wire::Raw {
                kind: input::wire::FILE_DIALOG,
                code: 0,
                x: 0.0,
                y: 0.0,
                value: *id as f64,
                text: Some(text),
            }
        } else {
            event.raw(text)
        };
        let text = raw.text.unwrap_or("");
        wire.push(Event {
            kind: raw.kind,
            code: raw.code,
            x: raw.x,
            y: raw.y,
            value: raw.value,
            text: Str {
                ptr: text.as_ptr(),
                len: text.len(),
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_drops_accumulate_across_pumps_capture_owner_and_reject_whole_invalid_batches() {
        let mut drop = FileDrop::default();
        assert!(
            drop.event(&mut HostEvent::FileDragMoved((10.0, 20.0)), Some(42))
                .is_none()
        );
        assert!(drop.hovered());
        assert!(
            drop.event(
                &mut HostEvent::FileDropped("/Music/名前.flac".into()),
                Some(43)
            )
            .is_none()
        );
        assert!(drop.event(&mut HostEvent::FocusLost, Some(43)).is_none());
        assert!(!drop.hovered());
        assert!(
            drop.event(
                &mut HostEvent::FileDropped("/Music/Folder".into()),
                Some(43)
            )
            .is_none()
        );
        assert_eq!(
            drop.event(&mut HostEvent::FileDragEnded, Some(43)),
            Some(Ok(DroppedFiles {
                user: Some(42),
                paths: vec!["/Music/名前.flac".into(), "/Music/Folder".into()],
            }))
        );
        assert!(drop.event(&mut HostEvent::FileDragEnded, None).is_none());
        for invalid in ["relative.flac", "/invalid\0file.m4a"] {
            drop.event(&mut HostEvent::FileDropped("/valid.flac".into()), None);
            drop.event(&mut HostEvent::FileDropped(invalid.into()), Some(99));
            assert!(
                drop.event(&mut HostEvent::FileDragEnded, Some(99))
                    .unwrap()
                    .is_err()
            );
            assert!(!drop.hovered());
        }
        drop.event(&mut HostEvent::FileDropped("/first.flac".into()), None);
        assert_eq!(
            drop.event(&mut HostEvent::FileDragEnded, Some(99))
                .unwrap()
                .unwrap()
                .user,
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
        let mut scratch = Vec::new();
        let mut wire = Vec::new();
        encode(&events, &mut scratch, &mut wire);
        assert_eq!(scratch[2], "/Music/Björk.flac\0/Music/日本語.flac");
        let capacity = scratch[2].capacity();
        mem_sys::local_reset();
        for count in [0, 1, 3, 2, 0, 3] {
            encode(&events[..count], &mut scratch, &mut wire);
            assert_eq!(wire.len(), count);
        }
        let allocations = mem_sys::local_alloc_count();
        assert_eq!(allocations, 0);
        assert_eq!(scratch[2].capacity(), capacity);
        assert_eq!(wire[2].kind, input::wire::FILE_DIALOG);
        assert_eq!(wire[2].value, 7.0);
        assert_eq!(wire[2].text.ptr, scratch[2].as_ptr());
        assert_eq!(wire[2].text.len, scratch[2].len());
        let empty = [HostEvent::FileDialog {
            id: 8,
            result: Ok(Vec::new()),
        }];
        encode(&empty, &mut scratch, &mut wire);
        assert!(scratch[0].is_empty());
        assert_eq!(wire[0].text.len, 0);
    }
}
