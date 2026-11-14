//! Owned once by the library worker; frame construction only borrows these labels and orders.
use lw_core::context::PlaybackContext;
use lw_core::{FileState, Song};
use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::{Deref, Range};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Route {
    #[default]
    Songs,
    Albums,
    Artists,
    Find,
    Files,
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
    pub album_artist: &'a str,
    pub format: &'a str,
    pub duration: &'a str,
    pub seconds: Option<f64>,
    pub release_year: Option<i64>,
    pub has_format: bool,
    pub cover_art_path: Option<&'a str>,
    pub cover_art_id: usize,
    pub favorite: bool,
    pub state: FileState,
}

#[derive(Clone)]
struct StoredRow {
    id: i64,
    labels: [Range<usize>; 6],
    favorite: bool,
    state: FileState,
    seconds: Option<f64>,
    release_year: Option<i64>,
    has_format: bool,
    cover_art: usize,
}

#[derive(Clone)]
pub struct Collection {
    pub key: u64,
    pub title: String,
    pub subtitle: String,
    pub count: String,
    pub rows: Vec<usize>,
    pub ids: Arc<[i64]>,
    pub context: Arc<PlaybackContext>,
    /// A representative row with artwork, prepared once with the collection.
    pub artwork_row: Option<usize>,
}

#[derive(Clone, Default)]
pub struct Library {
    labels: String,
    covers: Vec<Range<usize>>,
    rows: Vec<StoredRow>,
    by_id: HashMap<i64, usize>,
    pub all: Option<Collection>,
    pub albums: Vec<Collection>,
    pub artists: Vec<Collection>,
    pub collections: Vec<Collection>,
    pub album_count: String,
    pub artist_count: String,
    pub song_count: String,
}

impl Library {
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
                            + song.album_artist.len()
                            + song.file_extension.as_ref().map_or(0, String::len)
                            + 32
                    })
                    .sum(),
            ),
            ..Self::default()
        };
        out.rows.reserve_exact(songs.len());
        let mut tracks = Vec::with_capacity(songs.len());
        let mut covers = HashMap::new();
        for mut song in songs {
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
            let seconds_value = song
                .duration
                .filter(|value| value.is_finite() && *value >= 0.0);
            let seconds = seconds_value.map(|value| value as u64);
            let has_format = song.file_extension.is_some();
            let format = song.file_extension.unwrap_or_default().to_ascii_uppercase();
            let duration = seconds.map_or_else(
                || "—".into(),
                |seconds| format!("{}:{:02}", seconds / 60, seconds % 60),
            );
            let labels = [
                &song.title,
                &song.artist,
                &song.album,
                &format,
                &duration,
                &song.album_artist,
            ]
            .map(|text| {
                let start = out.labels.len();
                out.labels.push_str(text);
                start..out.labels.len()
            });
            let cover_art = song
                .cover_art_path
                .take()
                .filter(|path| !path.is_empty())
                .map_or(0, |path| {
                    *covers.entry(path).or_insert_with_key(|path| {
                        let start = out.labels.len();
                        out.labels.push_str(path);
                        out.covers.push(start..out.labels.len());
                        out.covers.len()
                    })
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
                seconds: seconds_value,
                release_year: song.release_year,
                has_format,
                cover_art,
            });
        }
        out.by_id = out
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| (row.id, index))
            .collect();
        for ((artist, name), mut rows) in albums {
            rows.sort_unstable_by(|a, b| {
                tracks[*a]
                    .cmp(&tracks[*b])
                    .then_with(|| out.row(*a).title.cmp(out.row(*b).title))
            });
            let caption = rows
                .first()
                .and_then(|index| out.rows[*index].release_year)
                .map_or_else(|| artist.clone(), |year| format!("{artist} · {year}"));
            out.albums.push(out.make_collection(
                &name,
                &caption,
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
        let [title, artist, album, format, duration, album_artist] = row
            .labels
            .each_ref()
            .map(|range| &self.labels[range.clone()]);
        Row {
            id: row.id,
            title,
            artist,
            album,
            album_artist,
            format,
            duration,
            favorite: row.favorite,
            state: row.state,
            seconds: row.seconds,
            release_year: row.release_year,
            has_format: row.has_format,
            cover_art_id: row.cover_art,
            cover_art_path: row
                .cover_art
                .checked_sub(1)
                .map(|index| &self.labels[self.covers[index].clone()]),
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
            artwork_row: rows
                .iter()
                .copied()
                .find(|index| self.rows[*index].cover_art != 0),
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
            Route::Albums | Route::Artists | Route::Find | Route::Files => None,
        }
    }
}

// Each prepared view shares the library's arena and collection data.
#[derive(Clone, Default)]
pub struct Catalog {
    pub library: Arc<Library>,
    pub view: Option<Arc<View>>,
    pub source_version: Option<i64>,
    pub queue: Option<Arc<crate::queue::Snapshot>>,
}
pub struct View {
    pub request: crate::browse::Request,
    pub collection: Collection,
}
impl Deref for Catalog {
    type Target = Library;
    fn deref(&self) -> &Library {
        &self.library
    }
}
impl Catalog {
    pub fn new(songs: Vec<Song>) -> Self {
        Self {
            library: Arc::new(Library::new(songs)),
            view: None,
            source_version: None,
            queue: None,
        }
    }
    pub fn add_collection(&mut self, name: &str, rows: Vec<usize>, context: PlaybackContext) {
        Arc::make_mut(&mut self.library).add_collection(name, rows, context);
    }
    pub fn row_index(&self, id: i64) -> Option<usize> {
        self.library.by_id.get(&id).copied()
    }
    pub fn base_collection(&self, key: u64) -> Option<&Collection> {
        self.all
            .iter()
            .chain(self.albums.iter())
            .chain(self.artists.iter())
            .chain(self.collections.iter())
            .find(|group| group.key == key)
    }
    pub fn scope(&self, route: Route) -> Option<crate::browse::Scope> {
        if route == Route::Find {
            Some(crate::browse::Scope::Find)
        } else if matches!(route, Route::Albums | Route::Artists) {
            Some(crate::browse::Scope::Overview)
        } else {
            self.library
                .collection(route)
                .map(|group| crate::browse::Scope::Collection(group.key))
        }
    }
    pub fn collection(&self, route: Route) -> Option<&Collection> {
        let scope = self.scope(route)?;
        if let Some(view) = self
            .view
            .as_ref()
            .filter(|view| view.request.scope == scope)
        {
            return Some(&view.collection);
        }
        self.library.collection(route)
    }
    pub fn project(&mut self, request: crate::browse::Request, rows: Vec<usize>) {
        let collection = match request.scope {
            crate::browse::Scope::Overview => {
                self.view = None;
                return;
            }
            crate::browse::Scope::Find => self.library.make_collection(
                "Songs",
                "Find results",
                rows,
                PlaybackContext::Selection("Find results".into()),
            ),
            crate::browse::Scope::Collection(key) => {
                let Some(base) = self.base_collection(key) else {
                    self.view = None;
                    return;
                };
                self.library.make_collection(
                    &base.title,
                    &base.subtitle,
                    rows,
                    (*base.context).clone(),
                )
            }
        };
        self.view = Some(Arc::new(View {
            request,
            collection,
        }));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cover_paths_share_storage_and_collections_keep_a_representative_row() {
        let catalog = super::Catalog::new(vec![
            lw_core::Song {
                id: Some(1),
                title: "A".into(),
                album: "Album".into(),
                track_number: Some(1),
                ..Default::default()
            },
            lw_core::Song {
                id: Some(2),
                title: "B".into(),
                album: "Album".into(),
                track_number: Some(2),
                cover_art_path: Some("CoverArt/shared.png".into()),
                ..Default::default()
            },
            lw_core::Song {
                id: Some(3),
                title: "C".into(),
                album: "Other".into(),
                cover_art_path: Some("CoverArt/shared.png".into()),
                ..Default::default()
            },
            lw_core::Song {
                id: Some(4),
                title: "D".into(),
                album: "Empty".into(),
                cover_art_path: Some(String::new()),
                ..Default::default()
            },
        ]);
        assert_eq!(catalog.library.covers.len(), 1);
        assert!(catalog.row(0).cover_art_path.is_none());
        assert!(catalog.row(3).cover_art_path.is_none());
        assert_eq!(
            catalog
                .albums
                .iter()
                .find(|album| album.title == "Album")
                .unwrap()
                .artwork_row,
            Some(1)
        );
        let first = catalog.row(1).cover_art_path.unwrap();
        let second = catalog.row(2).cover_art_path.unwrap();
        assert_eq!(first, "CoverArt/shared.png");
        assert_eq!(first.as_ptr(), second.as_ptr());
        mem_sys::local_reset();
        for _ in 0..1000 {
            assert_eq!(
                catalog.row(2).cover_art_path.unwrap().as_ptr(),
                first.as_ptr()
            );
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
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
