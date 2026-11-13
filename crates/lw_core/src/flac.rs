use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Metadata {
    pub comments: BTreeMap<String, String>,
    pub artwork: Option<Vec<u8>>,
}

impl Metadata {
    /// Read bounded metadata blocks, leaving audio undecoded. Retain valid tags on truncation.
    pub fn read(reader: &mut (impl Read + Seek)) -> io::Result<Option<Self>> {
        let mut magic = [0; 4];
        if !read_exact(reader, &mut magic)? || &magic != b"fLaC" {
            return Ok(None);
        }
        let mut metadata = Self::default();
        let mut total = 0usize;
        let mut header = [0; 4];
        while read_exact(reader, &mut header)? {
            let length = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;
            total += length + 4;
            if total > 64 * 1024 * 1024 {
                break;
            }
            match header[0] & 0x7f {
                kind @ (4 | 6) => {
                    let mut bytes = vec![0; length];
                    if !read_exact(reader, &mut bytes)? {
                        break;
                    }
                    if kind == 4 {
                        metadata.read_comments(&bytes);
                    } else {
                        metadata.read_picture(&bytes);
                    }
                }
                _ => {
                    reader.seek(SeekFrom::Current(length as i64))?;
                }
            }
            if header[0] & 0x80 != 0 {
                break;
            }
        }
        Ok(Some(metadata))
    }

    fn read_comments(&mut self, bytes: &[u8]) {
        let mut cursor = Cursor {
            bytes,
            little: true,
        };
        let Some(length) = cursor.integer() else {
            return;
        };
        if cursor.take(length).is_none() {
            return;
        }
        let Some(count) = cursor.integer() else {
            return;
        };
        if count > bytes.len() / 4 {
            return;
        }
        for _ in 0..count {
            let Some(length) = cursor.integer() else {
                return;
            };
            let Some(bytes) = cursor.take(length) else {
                return;
            };
            let Ok(text) = std::str::from_utf8(bytes) else {
                return;
            };
            let Some((key, value)) = text.split_once('=') else {
                return;
            };
            self.comments
                .entry(key.to_uppercase())
                .or_insert_with(|| value.into());
        }
    }

    fn read_picture(&mut self, bytes: &[u8]) {
        let mut cursor = Cursor {
            bytes,
            little: false,
        };
        let Some(kind) = cursor.integer() else {
            return;
        };
        let Some(length) = cursor.integer() else {
            return;
        };
        let Some(mime) = cursor.take(length) else {
            return;
        };
        if mime == b"-->" {
            return;
        }
        let Some(length) = cursor.integer() else {
            return;
        };
        if cursor.take(length).is_none() || cursor.take(16).is_none() {
            return;
        }
        let Some(length) = cursor.integer() else {
            return;
        };
        let Some(image) = cursor.take(length) else {
            return;
        };
        if self.artwork.is_none() || kind == 3 {
            self.artwork = Some(image.to_vec());
        }
    }
}

fn read_exact(reader: &mut impl Read, bytes: &mut [u8]) -> io::Result<bool> {
    match reader.read_exact(bytes) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(e),
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    little: bool,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let result = self.bytes.get(..count)?;
        self.bytes = &self.bytes[count..];
        Some(result)
    }
    fn integer(&mut self) -> Option<usize> {
        let bytes = self.take(4)?.try_into().ok()?;
        Some(if self.little {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        } as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn native_fixture_tags_and_every_truncated_prefix() {
        let bytes = include_bytes!("../../../Tests/LocalWaveDataTests/Fixtures/tagged.flac");
        let metadata = Metadata::read(&mut Cursor::new(bytes)).unwrap().unwrap();
        assert_eq!(metadata.comments["TITLE"], "FLAC Fixture");
        assert_eq!(metadata.comments["ALBUMARTIST"], "Test Ensemble");
        for prefix in 0..bytes.len() {
            Metadata::read(&mut Cursor::new(&bytes[..prefix])).unwrap();
        }
        assert!(
            Metadata::read(&mut Cursor::new(b"not FLAC"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn malformed_length_and_comments_do_not_read_outside_block() {
        let mut bytes = b"fLaC".to_vec();
        bytes.extend([0x84, 0, 0, 12]);
        bytes.extend(0u32.to_le_bytes());
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(u32::MAX.to_le_bytes());
        assert!(
            Metadata::read(&mut Cursor::new(&bytes))
                .unwrap()
                .unwrap()
                .comments
                .is_empty()
        );
        let mut metadata = Metadata::default();
        let mut comments = Vec::new();
        comments.extend(0u32.to_le_bytes());
        comments.extend(3u32.to_le_bytes());
        for text in [
            "TITLE=Hello=World",
            "artist=Björk",
            "TITLE=Ignored duplicate",
        ] {
            comments.extend((text.len() as u32).to_le_bytes());
            comments.extend(text.as_bytes());
        }
        metadata.read_comments(&comments);
        assert_eq!(metadata.comments["TITLE"], "Hello=World");
        assert_eq!(metadata.comments["ARTIST"], "Björk");
        let mut picture = Vec::new();
        for word in [3u32, 9] {
            picture.extend(word.to_be_bytes());
        }
        picture.extend(b"image/png");
        picture.extend(0u32.to_be_bytes());
        picture.extend([0; 16]);
        picture.extend(4u32.to_be_bytes());
        picture.extend([0x89, 0x50, 0x4e, 0x47]);
        metadata.read_picture(&picture);
        assert_eq!(
            metadata.artwork.as_deref(),
            Some(&[0x89, 0x50, 0x4e, 0x47][..])
        );
    }
}
