//! Owned once by the library worker; frame construction only borrows these labels and orders.
use lw_core::context::PlaybackContext;
use lw_core::{FileState, Song};
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Route {
    #[default]
    Songs,
    Albums,
    Artists,
    Album(usize),
    Artist(usize),
    Collection(usize),
}

#[derive(Clone, Copy)]
pub struct Row<'a> {
    pub id: i64,
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub format: &'a str,
    pub duration: &'a str,
    pub favorite: bool,
    pub state: FileState,
}

struct StoredRow {
    id: i64,
    labels: [Range<usize>; 5],
    favorite: bool,
    state: FileState,
}

pub struct Collection {
    pub key: u64,
    pub title: String,
    pub subtitle: String,
    pub count: String,
    pub rows: Vec<usize>,
    pub ids: Arc<[i64]>,
    pub context: Arc<PlaybackContext>,
}

#[derive(Default)]
pub struct Catalog {
    labels: String,
    rows: Vec<StoredRow>,
    pub all: Option<Collection>,
    pub albums: Vec<Collection>,
    pub artists: Vec<Collection>,
    pub collections: Vec<Collection>,
    pub album_count: String,
    pub artist_count: String,
    pub song_count: String,
}

impl Catalog {
    pub fn new(songs: Vec<Song>) -> Self {
        let mut albums = BTreeMap::<(String, String), Vec<usize>>::new();
        let mut artists = BTreeMap::<String, Vec<usize>>::new();
        let mut out = Self {
            labels: String::with_capacity(
                songs
                    .iter()
                    .map(|song| {
                        song.title.len()
                            + song.artist.len()
                            + song.album.len()
                            + song.file_extension.as_ref().map_or(0, String::len)
                            + 32
                    })
                    .sum(),
            ),
            ..Self::default()
        };
        out.rows.reserve_exact(songs.len());
        let mut tracks = Vec::with_capacity(songs.len());
        for song in songs {
            let Some(id) = song.id else { continue };
            let index = out.rows.len();
            let artist = if song.album_artist.is_empty() {
                &song.artist
            } else {
                &song.album_artist
            };
            albums
                .entry((artist.clone(), song.album.clone()))
                .or_default()
                .push(index);
            artists.entry(song.artist.clone()).or_default().push(index);
            let seconds = song
                .duration
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value as u64);
            let format = song.file_extension.unwrap_or_default().to_ascii_uppercase();
            let duration = seconds.map_or_else(
                || "—".into(),
                |seconds| format!("{}:{:02}", seconds / 60, seconds % 60),
            );
            let labels = [&song.title, &song.artist, &song.album, &format, &duration].map(|text| {
                let start = out.labels.len();
                out.labels.push_str(text);
                start..out.labels.len()
            });
            tracks.push((
                song.disc_number.unwrap_or(1),
                song.track_number.unwrap_or(i64::MAX),
            ));
            out.rows.push(StoredRow {
                id,
                labels,
                favorite: song.is_favorite != 0,
                state: song.file_state,
            });
        }
        for ((artist, name), mut rows) in albums {
            rows.sort_unstable_by(|a, b| {
                tracks[*a]
                    .cmp(&tracks[*b])
                    .then_with(|| out.row(*a).title.cmp(out.row(*b).title))
            });
            out.albums.push(out.make_collection(
                &name,
                &artist,
                rows,
                PlaybackContext::Album {
                    name: name.clone(),
                    artist: artist.clone(),
                },
            ));
        }
        for (name, rows) in artists {
            out.artists.push(out.make_collection(
                &name,
                "Artist",
                rows,
                PlaybackContext::Artist(name.clone()),
            ));
        }
        out.artists.sort_by_cached_key(|artist| {
            artist
                .title
                .strip_prefix("The ")
                .or_else(|| artist.title.strip_prefix("the "))
                .unwrap_or(&artist.title)
                .chars()
                .flat_map(char::to_lowercase)
                .collect::<String>()
        });
        out.song_count = out.rows.len().to_string();
        out.album_count = out.albums.len().to_string();
        out.artist_count = out.artists.len().to_string();
        out.all = Some(out.make_collection(
            "Songs",
            "Your music library",
            (0..out.rows.len()).collect(),
            PlaybackContext::Library,
        ));
        let favorites = out
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| row.favorite.then_some(index))
            .collect();
        out.add_collection(
            "Favorites",
            favorites,
            PlaybackContext::Automatic("Favorites".into()),
        );
        out.add_collection(
            "Recently added",
            (0..out.rows.len().min(24)).collect(),
            PlaybackContext::Automatic("Recently added".into()),
        );
        out
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn row(&self, index: usize) -> Row<'_> {
        let row = &self.rows[index];
        let [title, artist, album, format, duration] = row
            .labels
            .each_ref()
            .map(|range| &self.labels[range.clone()]);
        Row {
            id: row.id,
            title,
            artist,
            album,
            format,
            duration,
            favorite: row.favorite,
            state: row.state,
        }
    }
    pub fn add_collection(&mut self, name: &str, rows: Vec<usize>, context: PlaybackContext) {
        let collection = self.make_collection(name, "Collection", rows, context);
        self.collections.push(collection);
    }
    fn make_collection(
        &self,
        title: &str,
        subtitle: &str,
        rows: Vec<usize>,
        context: PlaybackContext,
    ) -> Collection {
        let mut key = DefaultHasher::new();
        match &context {
            PlaybackContext::Playlist { id, .. } => (true, id).hash(&mut key),
            context => (false, context).hash(&mut key),
        }
        Collection {
            key: key.finish(),
            title: title.into(),
            subtitle: subtitle.into(),
            count: rows.len().to_string(),
            ids: rows.iter().map(|index| self.rows[*index].id).collect(),
            rows,
            context: Arc::new(context),
        }
    }
    pub fn collection(&self, route: Route) -> Option<&Collection> {
        match route {
            Route::Songs => self.all.as_ref(),
            Route::Album(index) => self.albums.get(index),
            Route::Artist(index) => self.artists.get(index),
            Route::Collection(index) => self.collections.get(index),
            Route::Albums | Route::Artists => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_prepares_real_labels_groups_and_queue_orders_once() {
        let songs = (0..10000)
            .map(|index| Song {
                id: Some(index),
                title: format!("Track {index}"),
                artist: format!("Artist {}", index % 10),
                album: format!("Album {}", index % 100),
                duration: Some(90.0),
                is_favorite: index % 2,
                ..Song::default()
            })
            .collect();
        let catalog = Catalog::new(songs);
        assert_eq!(catalog.rows.len(), 10000);
        assert_eq!(catalog.albums.len(), 100);
        assert_eq!(catalog.artists.len(), 10);
        assert_eq!(catalog.row(0).duration, "1:30");
        assert_eq!(catalog.collections[0].ids.len(), 5000);
        assert_eq!(catalog.collections[1].ids.len(), 24);
        for group in catalog
            .albums
            .iter()
            .chain(&catalog.artists)
            .chain(&catalog.collections)
        {
            assert_eq!(group.ids.len(), group.rows.len());
            assert!(
                group
                    .rows
                    .iter()
                    .zip(group.ids.iter())
                    .all(|(index, id)| catalog.rows[*index].id == *id)
            );
        }
        mem_sys::local_reset();
        for _ in 0..100 {
            assert_eq!(catalog.collection(Route::Songs).unwrap().ids.len(), 10000);
            assert_eq!(
                catalog.collection(Route::Album(0)).unwrap().context.title(),
                catalog.albums[0].title
            );
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
}
