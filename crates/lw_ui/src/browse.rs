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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Collection(u64),
    Find,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Original,
    Title,
    Artist,
    Album,
    Duration,
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
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
}
