//! Swift-compatible Find tokens. Parsing and matching belong on the library worker.
use crate::FileAvailability;

enum Word {
    Text(String),
    Favorite,
    Unplayed,
    Format(&'static str),
    Year(i64),
    Availability(FileAvailability),
}

pub struct SongQuery {
    words: Vec<Word>,
    text: String,
    has_filters: bool,
}
impl SongQuery {
    pub fn new(query: &str) -> Self {
        let mut out = Self {
            words: Vec::new(),
            text: String::new(),
            has_filters: false,
        };
        for word in query.split_whitespace() {
            let word: String = word.chars().flat_map(char::to_lowercase).collect();
            let predicate = match word.as_str() {
                "cloud" => Word::Availability(FileAvailability::InCloud),
                "offline" => Word::Availability(FileAvailability::OnDevice),
                "fav" | "favorites" => Word::Favorite,
                "unplayed" => Word::Unplayed,
                "flac" => Word::Format("flac"),
                "mp3" => Word::Format("mp3"),
                "m4a" => Word::Format("m4a"),
                "wav" => Word::Format("wav"),
                "aiff" => Word::Format("aiff"),
                _ => {
                    // Swift excludes every year: word from FTS, including an invalid year.
                    if let Some(year) = word.strip_prefix("year:") {
                        out.has_filters = true;
                        match year.parse() {
                            Ok(year) => Word::Year(year),
                            Err(_) => Word::Text(word),
                        }
                    } else {
                        if !out.text.is_empty() {
                            out.text.push(' ');
                        }
                        out.text.push_str(&word);
                        Word::Text(word)
                    }
                }
            };
            out.has_filters |= !matches!(predicate, Word::Text(_));
            out.words.push(predicate);
        }
        out
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn has_filters(&self) -> bool {
        self.has_filters
    }
    pub fn needs_availability(&self) -> bool {
        self.words
            .iter()
            .any(|word| matches!(word, Word::Availability(_)))
    }
    pub fn needs_history(&self) -> bool {
        self.words.iter().any(|word| matches!(word, Word::Unplayed))
    }
    pub fn needs_format(&self) -> bool {
        self.words
            .iter()
            .any(|word| matches!(word, Word::Format(_)))
    }
    pub fn text_words(&self) -> impl Iterator<Item = &str> {
        self.words.iter().filter_map(|word| match word {
            Word::Text(word) => Some(word.as_str()),
            _ => None,
        })
    }
    /// Text matching uses the platform's localized containment, supplied by the worker.
    pub fn matches_metadata(
        &self,
        year: Option<i64>,
        favorite: bool,
        plays: i64,
        contains: &mut dyn FnMut(&str) -> bool,
    ) -> bool {
        self.words.iter().all(|word| match word {
            Word::Text(word) => contains(word),
            Word::Favorite => favorite,
            Word::Unplayed => plays == 0,
            Word::Year(expected) => year == Some(*expected),
            _ => true,
        })
    }
    pub fn matches_format(&self, format: &str) -> bool {
        self.words.iter().all(|word| match word {
            Word::Format(expected) => format.eq_ignore_ascii_case(expected),
            _ => true,
        })
    }
    pub fn matches_availability(&self, availability: FileAvailability) -> bool {
        self.words.iter().all(|word| match word {
            Word::Availability(expected) => availability == *expected,
            _ => true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn find_tokens_preserve_swift_text_year_and_conjunctive_filter_semantics() {
        let query = SongQuery::new("  CAFÉ\tFLAC favorites YEAR:+2019\nunplayed offline  ");
        assert_eq!(query.text(), "café");
        assert!(query.has_filters() && query.needs_availability() && query.needs_format());
        assert_eq!(query.text_words().collect::<Vec<_>>(), ["café"]);
        assert!(query.matches_metadata(Some(2019), true, 0, &mut |word| word == "café"));
        assert!(!query.matches_metadata(Some(2020), true, 0, &mut |_| true));
        assert!(!query.matches_metadata(Some(2019), false, 0, &mut |_| true));
        assert!(!query.matches_metadata(Some(2019), true, 1, &mut |_| true));
        assert!(!query.matches_metadata(Some(2019), true, 0, &mut |_| false));
        assert!(
            query.matches_format("FLAC") && query.matches_availability(FileAvailability::OnDevice)
        );
        assert!(!query.matches_availability(FileAvailability::Unknown));
        assert!(!query.matches_format("mp3"));
        assert!(!SongQuery::new("cloud offline").matches_availability(FileAvailability::InCloud));
        let cloud = SongQuery::new("cloud flac");
        assert!(
            cloud.matches_availability(FileAvailability::InCloud) && cloud.matches_format("flac")
        );
        assert!(!cloud.matches_availability(FileAvailability::Unknown));
        assert!(!SongQuery::new("flac mp3").matches_format("flac"));
        for format in ["flac", "mp3", "m4a", "wav", "aiff"] {
            let query = SongQuery::new(format);
            assert_eq!(query.text(), "");
            assert!(query.matches_format(format));
        }
        let invalid = SongQuery::new("year:no year:999999999999999999999999 title");
        assert_eq!(invalid.text(), "title");
        assert!(invalid.has_filters());
        assert_eq!(invalid.text_words().count(), 3);
        assert!(!invalid.matches_metadata(None, false, 0, &mut |word| word == "title"));
        let empty = SongQuery::new(" \n\t ");
        assert!(!empty.has_filters());
        assert!(empty.matches_metadata(None, false, 0, &mut |_| false));
        mem_sys::local_reset();
        for _ in 0..1000 {
            assert!(query.matches_metadata(Some(2019), true, 0, &mut |_| true));
            assert!(
                query.matches_format("flac")
                    && query.matches_availability(FileAvailability::OnDevice)
            );
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
}
