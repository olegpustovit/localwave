//! Foundation JSON boundary for Swift's synthesized associated-enum Codable layout.
use crate::bookmarks::failure;
use std::ffi::{c_char, c_void};
use std::{io, ptr};

#[repr(C)]
#[derive(Default)]
struct Bytes {
    data: *const u8,
    len: usize,
}
#[repr(C)]
#[derive(Default)]
struct Fields {
    kind: i32,
    has_path: i32,
    id: i64,
    path: i64,
    name: Bytes,
    artist: Bytes,
}
unsafe extern "C" {
    fn lw_context_decode(
        data: *const u8,
        len: usize,
        fields: *mut Fields,
        error: *mut *mut c_char,
    ) -> *mut c_void;
    fn lw_context_encode(fields: *const Fields, data: *mut Bytes, error: *mut *mut c_char) -> i32;
    fn lw_preferences_release(raw: *mut c_void);
    fn lw_free_string(raw: *mut c_char);
}
struct Decoded(*mut c_void);
impl Drop for Decoded {
    fn drop(&mut self) {
        unsafe { lw_preferences_release(self.0) };
    }
}

pub struct ContextValues {
    pub kind: u32,
    pub name: String,
    pub artist: String,
    pub id: i64,
    pub path_id: Option<i64>,
}

pub fn decode(data: &[u8]) -> io::Result<ContextValues> {
    if data.len() > 64 * 1024 * 1024 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut fields = Fields::default();
    let mut error = ptr::null_mut();
    let decoded =
        Decoded(unsafe { lw_context_decode(data.as_ptr(), data.len(), &mut fields, &mut error) });
    if decoded.0.is_null() {
        return Err(failure(error));
    }
    let text = |bytes: &Bytes| -> io::Result<String> {
        if bytes.len == 0 {
            return Ok(String::new());
        }
        if bytes.data.is_null() || bytes.len > 64 * 1024 * 1024 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        std::str::from_utf8(unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) })
            .map(str::to_owned)
            .map_err(io::Error::other)
    };
    Ok(ContextValues {
        kind: fields.kind as u32,
        name: text(&fields.name)?,
        artist: text(&fields.artist)?,
        id: fields.id,
        path_id: (fields.has_path != 0).then_some(fields.path),
    })
}

pub fn encode(
    kind: u32,
    name: &str,
    artist: &str,
    id: i64,
    path_id: Option<i64>,
) -> io::Result<Vec<u8>> {
    if kind > 6 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let fields = Fields {
        kind: kind as i32,
        has_path: path_id.is_some() as i32,
        id,
        path: path_id.unwrap_or(0),
        name: Bytes {
            data: name.as_ptr(),
            len: name.len(),
        },
        artist: Bytes {
            data: artist.as_ptr(),
            len: artist.len(),
        },
    };
    let mut bytes = Bytes::default();
    let mut error = ptr::null_mut();
    if unsafe { lw_context_encode(&fields, &mut bytes, &mut error) } == 0 {
        return Err(failure(error));
    }
    let result = unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) }.to_vec();
    unsafe { lw_free_string(bytes.data.cast_mut().cast()) };
    Ok(result)
}

/// # Safety
/// `data` is readable for `len` bytes and `output` writable for `capacity` bytes.
#[cfg(feature = "interop-test")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lw_context_interop_fixture(
    data: *const u8,
    len: usize,
    output: *mut u8,
    capacity: usize,
) -> isize {
    if data.is_null() || output.is_null() || len > 64 * 1024 * 1024 {
        return -1;
    }
    let Ok(context) = decode(unsafe { std::slice::from_raw_parts(data, len) }) else {
        return -2;
    };
    let Ok(encoded) = encode(
        context.kind,
        &context.name,
        &context.artist,
        context.id,
        context.path_id,
    ) else {
        return -3;
    };
    if encoded.len() > capacity {
        return -4;
    }
    unsafe { ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len()) };
    encoded.len() as isize
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_json_preserves_cases_strings_and_integer_bounds_and_rejects_invalid_payloads() {
        for kind in 0..=6 {
            let bytes = encode(kind, "名前\0\"\\\n🦊", "Björk", i64::MAX, Some(i64::MIN)).unwrap();
            let value = decode(&bytes).unwrap();
            assert_eq!(value.kind, kind);
            if kind != 0 {
                assert_eq!(value.name, "名前\0\"\\\n🦊");
            }
            if kind == 1 {
                assert_eq!(value.artist, "Björk");
            }
            if matches!(kind, 3 | 4) {
                assert_eq!(value.id, i64::MAX);
            }
            if kind == 4 {
                assert_eq!(value.path_id, Some(i64::MIN));
            }
        }
        for json in [
            r#"{"folder":{"sourceID":1,"name":"Root"}}"#,
            r#"{"folder":{"sourceID":1,"pathID":null,"name":"Root"}}"#,
        ] {
            assert_eq!(decode(json.as_bytes()).unwrap().path_id, None);
        }
        for invalid in [
            "",
            "[]",
            "{}",
            r#"{"unknown":{}}"#,
            r#"{"library":{},"artist":{"_0":"a"}}"#,
            r#"{"artist":{}}"#,
            r#"{"album":{"name":"a","artist":null}}"#,
            r#"{"playlist":{"id":true,"name":"a"}}"#,
            r#"{"playlist":{"id":9223372036854775808,"name":"a"}}"#,
            r#"{"playlist":{"id":9223372036854775807.0,"name":"a"}}"#,
            r#"{"folder":{"sourceID":1,"pathID":1.5,"name":"a"}}"#,
        ] {
            assert!(decode(invalid.as_bytes()).is_err(), "{invalid}");
        }
        assert!(encode(7, "", "", 0, None).is_err());
    }
}
