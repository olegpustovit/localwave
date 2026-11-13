use input::Event as HostEvent;
use pxui::{Event, Str};

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
