use input::Event as HostEvent;
use pxui::{Event, Str};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderPurpose {
    Add(i64),
    Reconnect(lw_ui::files::Target),
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
