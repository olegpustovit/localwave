//! Fixed-capacity browse commands; requests never own library storage or allocate in a frame.
pub const MAX_QUERY_BYTES: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Query {
    bytes: [u8; MAX_QUERY_BYTES],
    len: u16,
}
impl Default for Query {
    fn default() -> Self {
        Self {
            bytes: [0; MAX_QUERY_BYTES],
            len: 0,
        }
    }
}
impl Query {
    pub fn new(text: &str) -> Option<Self> {
        if text.len() > MAX_QUERY_BYTES {
            return None;
        }
        let mut query = Self::default();
        query.bytes[..text.len()].copy_from_slice(text.as_bytes());
        query.len = text.len() as u16;
        Some(query)
    }
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)])
            .expect("query was copied from UTF-8")
    }
    pub fn has_token(&self, token: &str) -> bool {
        self.as_str()
            .split_whitespace()
            .any(|word| word.eq_ignore_ascii_case(token))
    }
    pub fn toggle_token(&self, token: &str) -> Option<Self> {
        let remove = self.has_token(token);
        let mut out = Self::default();
        for word in self.as_str().split_whitespace() {
            if !(remove && word.eq_ignore_ascii_case(token)) && !out.append(word) {
                return None;
            }
        }
        if !remove && !out.append(token) {
            return None;
        }
        Some(out)
    }
    fn append(&mut self, word: &str) -> bool {
        let start = usize::from(self.len);
        let space = usize::from(start != 0);
        let Some(end) = start
            .checked_add(space)
            .and_then(|len| len.checked_add(word.len()))
            .filter(|len| *len <= MAX_QUERY_BYTES)
        else {
            return false;
        };
        if space != 0 {
            self.bytes[start] = b' ';
        }
        self.bytes[start + space..end].copy_from_slice(word.as_bytes());
        self.len = end as u16;
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Collection(u64),
    Find,
    Overview,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Original,
    Title,
    Artist,
    Album,
    Format,
    Duration,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(usize)]
pub enum AlbumSort {
    #[default]
    Artist,
    Year,
    Added,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub scope: Scope,
    pub query: Query,
    pub sort: Sort,
    pub ascending: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_packets_preserve_utf8_and_refuse_oversized_edits_without_allocating() {
        let maximum = "🎵".repeat(MAX_QUERY_BYTES / 4);
        let too_long = format!("{maximum}x");
        mem_sys::local_reset();
        assert_eq!(Query::new(&maximum).unwrap().as_str(), maximum);
        assert!(Query::new(&too_long).is_none());
        assert_eq!(Query::default().as_str(), "");
        let original = Query::new("CAFÉ\tFLAC flac   live").unwrap();
        assert!(original.has_token("flac"));
        assert_eq!(original.toggle_token("flac").unwrap().as_str(), "CAFÉ live");
        assert_eq!(
            original.toggle_token("cloud").unwrap().as_str(),
            "CAFÉ FLAC flac live cloud"
        );
        assert!(Query::new(&maximum).unwrap().toggle_token("flac").is_none());
        assert_eq!(original.as_str(), "CAFÉ\tFLAC flac   live");
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
}
