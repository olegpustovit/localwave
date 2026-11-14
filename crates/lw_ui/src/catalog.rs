//! Owned once by the library worker; frame construction only borrows these labels and orders.
use lw_core::context::PlaybackContext;
use lw_core::{FileAvailability, FileState, Song};
mod song_table;
pub use song_table::{Column as SongColumn, SongTable};
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
    pub format_key: &'a str,
    pub track_number: Option<i64>,
    pub track_label: &'a str,
    pub availability: Option<FileAvailability>,
    pub pinned: bool,
    pub actions_label: &'a str,
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
    labels: [Range<usize>; 9],
    favorite: bool,
    track_number: Option<i64>,
    availability: Option<FileAvailability>,
    pinned: bool,
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
    pub row_keys: Vec<i64>,
    pub ids: Arc<[i64]>,
    pub context: Arc<PlaybackContext>,
    /// A representative row with artwork, prepared once with the collection.
    pub artwork_row: Option<usize>,
    pub song_table: Option<Arc<SongTable>>,
}

pub struct ArtistRow {
    pub albums: String,
    pub minutes: String,
    pub widths: [f32; 2],
}
pub struct ArtistTable {
    pub rows: Vec<ArtistRow>,
    pub preferred: [f32; 4],
    pub headings: [f32; 4],
}

#[derive(Clone, Default)]
pub struct Library {
    labels: String,
    covers: Vec<Range<usize>>,
    rows: Vec<StoredRow>,
    by_id: HashMap<i64, usize>,
    pub all: Option<Collection>,
    pub albums: Vec<Collection>,
    pub album_orders: [Vec<usize>; 3],
    pub size_label: String,
    pub library_subtitle: String,
    pub artists: Vec<Collection>,
    pub collections: Vec<Collection>,
    pub album_count: String,
    pub artist_count: String,
    pub artist_table: Option<Arc<ArtistTable>>,
    song_metrics: Option<Arc<song_table::Metrics>>,
    pub song_count: String,
}

impl Library {
    pub fn new(songs: Vec<Song>) -> Self {
        let mut albums = BTreeMap::<(String, String), Vec<usize>>::new();
        let mut artists = BTreeMap::<String, Vec<usize>>::new();
        let megabytes = songs
            .iter()
            .filter(|song| song.id.is_some())
            .map(|song| i128::from(song.file_size.unwrap_or(0).max(0)))
            .sum::<i128>() as f64
            / 1_048_576.0;
        let size_label = if megabytes >= 1024.0 {
            format!("{:.1} GB", megabytes / 1024.0)
        } else {
            format!("{megabytes:.1} MB")
        };
        let mut out = Self {
            size_label,
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
        let mut added = Vec::with_capacity(songs.len());
        let mut album_added = Vec::new();
        let mut covers = HashMap::new();
        for mut song in songs {
            let Some(id) = song.id else { continue };
            let index = out.rows.len();
            added.push(if song.created_at.is_finite() {
                song.created_at
            } else {
                f64::NEG_INFINITY
            });
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
            let seconds = seconds_value
                .filter(|value| *value < i64::MAX as f64)
                .map(|value| value as u64);
            let has_format = song.file_extension.is_some();
            let format_key = song.file_extension.unwrap_or_default();
            let format = format_key.to_lowercase();
            let track = song
                .track_number
                .map_or_else(String::new, |number| number.to_string());
            let actions_label = format!("Actions for {}", song.title);
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
                &format_key,
                &track,
                &actions_label,
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
                track_number: song.track_number,
                availability: song.source_availability,
                pinned: false,
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
            album_added.push(
                rows.iter()
                    .map(|index| added[*index])
                    .max_by(f64::total_cmp)
                    .unwrap_or(f64::NEG_INFINITY),
            );
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
        out.album_orders = std::array::from_fn(|_| (0..out.albums.len()).collect());
        let artist_keys: Vec<_> = out
            .albums
            .iter()
            .map(|album| match album.context.as_ref() {
                PlaybackContext::Album { artist, .. } => {
                    artist.strip_prefix("The ").unwrap_or(artist)
                }
                _ => unreachable!("album context"),
            })
            .collect();
        out.album_orders[0].sort_by(|a, b| {
            (artist_keys[*a], out.albums[*a].title.as_str())
                .cmp(&(artist_keys[*b], out.albums[*b].title.as_str()))
        });
        out.album_orders[1].sort_by_key(|index| {
            std::cmp::Reverse(
                out.albums[*index]
                    .rows
                    .first()
                    .and_then(|index| out.rows[*index].release_year)
                    .unwrap_or(0),
            )
        });
        out.album_orders[2].sort_by(|a, b| album_added[*b].total_cmp(&album_added[*a]));
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
        out.set_source_count(0);
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
    /// Worker-prepared labels include empty sources as in the Swift library model.
    pub fn set_source_count(&mut self, count: usize) {
        self.library_subtitle = format!(
            "{} songs across {count} sources · {}",
            self.song_count, self.size_label
        );
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub(crate) fn song_body_width(
        &self,
        index: usize,
        column: SongColumn,
        number: usize,
        numbered: bool,
    ) -> f32 {
        self.song_metrics
            .as_ref()
            .expect("prepared song table")
            .body_width(index, column, number, numbered)
    }
    pub(crate) fn song_heading_width(&self, column: SongColumn, ascending: Option<bool>) -> f32 {
        self.song_metrics
            .as_ref()
            .expect("prepared song table")
            .heading_width(column, ascending)
    }
    pub fn row(&self, index: usize) -> Row<'_> {
        let row = &self.rows[index];
        let [
            title,
            artist,
            album,
            format,
            duration,
            album_artist,
            format_key,
            track_label,
            actions_label,
        ] = row
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
            format_key,
            track_number: row.track_number,
            track_label,
            actions_label,
            availability: row.availability,
            pinned: row.pinned,
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
        let mut occurrences = HashMap::new();
        let row_keys = rows
            .iter()
            .map(|index| {
                let id = self.rows[*index].id;
                let occurrence = occurrences.entry(id).or_insert(0i64);
                let key = pxui::ui::int_key(id, *occurrence);
                *occurrence += 1;
                key
            })
            .collect();
        Collection {
            key: key.finish(),
            row_keys,
            title: title.into(),
            subtitle: subtitle.into(),
            count: rows.len().to_string(),
            ids: rows.iter().map(|index| self.rows[*index].id).collect(),
            artwork_row: rows
                .iter()
                .copied()
                .find(|index| self.rows[*index].cover_art != 0),
            song_table: self
                .song_metrics
                .as_ref()
                .map(|metrics| Arc::new(SongTable::prepare(self, &rows, metrics))),
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
    pub source_identity: Option<i64>,
    pub queue: Option<Arc<crate::queue::Snapshot>>,
}
#[derive(Clone)]
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
            source_identity: None,
            queue: None,
        }
    }
    /// A consistent preferences snapshot, applied before preparing table projections.
    pub fn prepare_pins(&mut self, pinned: &mut dyn FnMut(i64) -> bool) {
        let library = Arc::make_mut(&mut self.library);
        for row in &mut library.rows {
            row.pinned = pinned(row.id);
        }
        library.song_metrics = None;
        library.artist_table = None;
        for collection in library
            .all
            .iter_mut()
            .chain(library.albums.iter_mut())
            .chain(library.artists.iter_mut())
            .chain(library.collections.iter_mut())
        {
            collection.song_table = None;
        }
        if let Some(view) = &mut self.view {
            Arc::make_mut(view).collection.song_table = None;
        }
    }
    /// Called on the catalog worker: frames only select and scale prepared table measurements.
    pub fn prepare_tables(&mut self) -> Result<(), String> {
        let mut text = pxui::Text::new(&[crate::FONT, crate::MONO])
            .ok_or_else(|| "Unable to create library text metrics".to_owned())?;
        text.enable_system_fonts()
            .map_err(|code| format!("Library font fallback failed ({code})"))?;
        let mut width = |value: &str, mono: bool, size: f32, weight: u16| {
            let style = pxui::TextStyle {
                font: u16::from(mono),
                features: if !mono && size == 13.0 {
                    pxui::TextStyle::FEATURE_TABULAR_NUMBERS
                } else {
                    0
                },
                size_points: size,
                weight,
                ..Default::default()
            };
            let mut key = DefaultHasher::new();
            (value, mono, size.to_bits(), weight).hash(&mut key);
            text.width(key.finish(), value, &style, 0.0)
                .map_err(|code| format!("Library text measurement failed ({code})"))
        };
        let mut preferred = [0.0f32; 4];
        for (index, title) in ["Name", "Albums", "Songs", "Time"].into_iter().enumerate() {
            preferred[index] = width(title, false, 12.0, 500)?
                + if index == 0 {
                    40.0
                } else if index == 3 {
                    28.0
                } else {
                    16.0
                };
        }
        let headings = std::array::from_fn(|index| {
            preferred[index]
                - if index == 0 {
                    40.0
                } else if index == 3 {
                    28.0
                } else {
                    16.0
                }
        });
        let mut rows = Vec::with_capacity(self.artists.len());
        for artist in &self.artists {
            let mut seen = std::collections::HashSet::new();
            let mut albums = String::new();
            let mut seconds = 0.0;
            for index in &artist.rows {
                let row = self.row(*index);
                if seen.insert(row.album) {
                    if seen.len() > 1 {
                        albums.push_str(", ");
                    }
                    albums.push_str(row.album);
                }
                seconds += row.seconds.unwrap_or(0.0);
            }
            let minutes = if seconds.is_finite() && seconds / 60.0 < i64::MAX as f64 {
                format!("{:.0} min", (seconds / 60.0).round())
            } else {
                "—".to_owned()
            };
            preferred[0] = preferred[0].max(
                (width(
                    if artist.title.is_empty() {
                        "Unknown artist"
                    } else {
                        &artist.title
                    },
                    false,
                    14.0,
                    400,
                )? + 80.0)
                    .min(240.0),
            );
            preferred[1] = preferred[1].max((width(&albums, false, 14.0, 400)? + 16.0).min(240.0));
            let widths = [
                width(&artist.count, true, 12.0, 400)?,
                width(&minutes, true, 12.0, 400)?,
            ];
            preferred[2] = preferred[2].max(widths[0] + 16.0);
            preferred[3] = preferred[3].max(widths[1] + 28.0);
            rows.push(ArtistRow {
                albums,
                minutes,
                widths,
            });
        }
        let songs = Arc::new(song_table::Metrics::prepare(self, &mut width)?);
        let tables = self
            .all
            .iter()
            .chain(self.albums.iter())
            .chain(self.artists.iter())
            .chain(self.collections.iter())
            .map(|collection| Arc::new(SongTable::prepare(self, &collection.rows, &songs)))
            .collect::<Vec<_>>();
        let library = Arc::make_mut(&mut self.library);
        library.artist_table = Some(Arc::new(ArtistTable {
            rows,
            preferred,
            headings,
        }));
        library.song_metrics = Some(songs.clone());
        for (collection, table) in library
            .all
            .iter_mut()
            .chain(library.albums.iter_mut())
            .chain(library.artists.iter_mut())
            .chain(library.collections.iter_mut())
            .zip(tables)
        {
            collection.song_table = Some(table);
        }
        if let Some(view) = &mut self.view {
            let table = Arc::new(SongTable::prepare(library, &view.collection.rows, &songs));
            Arc::make_mut(view).collection.song_table = Some(table);
        }
        Ok(())
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
    fn album_orders_match_swift_artist_first_track_year_and_latest_added() {
        let mut library = super::Library::new(vec![
            Song {
                id: Some(1),
                artist: "Alice".into(),
                album: "B".into(),
                release_year: Some(2019),
                track_number: Some(1),
                created_at: 100.0,
                file_size: Some(1_073_741_824),
                ..Default::default()
            },
            Song {
                id: Some(2),
                artist: "The Bee".into(),
                album: "A".into(),
                release_year: Some(2022),
                created_at: 200.0,
                ..Default::default()
            },
            Song {
                id: Some(3),
                artist: "Cedar".into(),
                album: "C".into(),
                created_at: 300.0,
                file_size: Some(-1),
                ..Default::default()
            },
            Song {
                id: Some(4),
                artist: "Alice".into(),
                album: "B".into(),
                release_year: Some(2030),
                track_number: Some(2),
                created_at: 400.0,
                ..Default::default()
            },
        ]);
        let names = |order: usize| {
            library.album_orders[order]
                .iter()
                .map(|index| library.albums[*index].title.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(0), ["B", "A", "C"]);
        assert_eq!(names(1), ["A", "B", "C"]);
        assert_eq!(names(2), ["B", "C", "A"]);
        library.set_source_count(3);
        assert_eq!(library.size_label, "1.0 GB");
        assert_eq!(
            library.library_subtitle,
            "4 songs across 3 sources · 1.0 GB"
        );
        let empty = super::Library::new(Vec::new());
        assert_eq!(empty.library_subtitle, "0 songs across 0 sources · 0.0 MB");
        assert!(empty.album_orders.iter().all(Vec::is_empty));
    }
    #[test]
    fn table_font_widths_match_coretext_reference_faces_and_weights() {
        let mut text = pxui::Text::new(&[crate::FONT, crate::MONO]).unwrap();
        // CTLineGetTypographicBounds with the bundled named faces, captured on macOS.
        for (index, (label, font, size, weight, expected)) in [
            ("Name", 0, 12.0, 500, 32.424),
            ("Albums", 0, 12.0, 500, 40.98),
            ("Songs", 0, 12.0, 500, 34.224),
            ("Time", 0, 12.0, 500, 27.06),
            ("Björk", 0, 14.0, 400, 32.074),
            ("The National", 0, 14.0, 400, 80.192),
            ("High Violet, Trouble Will Find Me", 0, 14.0, 400, 205.59),
            ("0 min", 1, 12.0, 400, 35.99945),
            ("92", 1, 12.0, 400, 14.39978),
        ]
        .into_iter()
        .enumerate()
        {
            let width = text
                .width(
                    index as u64,
                    label,
                    &pxui::TextStyle {
                        font,
                        size_points: size,
                        weight,
                        ..Default::default()
                    },
                    0.0,
                )
                .unwrap();
            assert!(
                (width - expected).abs() < 0.01,
                "{label} / weight {weight}: pxui {width}, CoreText {expected}"
            );
        }
    }

    #[test]
    fn artist_table_prepares_distinct_album_names_rounded_minutes_and_shared_metrics() {
        let mut catalog = Catalog::new(vec![
            Song {
                id: Some(1),
                artist: "The National".into(),
                album: "High Violet".into(),
                duration: Some(29.0),
                ..Default::default()
            },
            Song {
                id: Some(2),
                artist: "The National".into(),
                album: "High Violet".into(),
                duration: Some(31.0),
                ..Default::default()
            },
            Song {
                id: Some(3),
                artist: "The National".into(),
                album: "Trouble Will Find Me".into(),
                duration: Some(30.0),
                ..Default::default()
            },
            Song {
                id: Some(4),
                artist: "Björk".into(),
                album: "Vespertine".into(),
                duration: None,
                ..Default::default()
            },
            Song {
                id: Some(5),
                artist: "Blank album".into(),
                album: "".into(),
                duration: Some(f64::MAX),
                ..Default::default()
            },
            Song {
                id: Some(6),
                artist: "Blank album".into(),
                album: "Later".into(),
                duration: Some(f64::MAX),
                ..Default::default()
            },
        ]);
        let unchanged = catalog.library.clone();
        catalog.prepare_tables().unwrap();
        assert!(unchanged.artist_table.is_none());
        let table = catalog.artist_table.as_ref().unwrap();
        assert_eq!(table.rows.len(), catalog.artists.len());
        let national = catalog
            .artists
            .iter()
            .position(|artist| artist.title == "The National")
            .unwrap();
        assert_eq!(
            table.rows[national].albums,
            "High Violet, Trouble Will Find Me"
        );
        assert_eq!(table.rows[national].minutes, "2 min");
        let bjork = catalog
            .artists
            .iter()
            .position(|artist| artist.title == "Björk")
            .unwrap();
        assert_eq!(table.rows[bjork].minutes, "0 min");
        let blank = catalog
            .artists
            .iter()
            .position(|artist| artist.title == "Blank album")
            .unwrap();
        assert_eq!(table.rows[blank].albums, ", Later");
        assert_eq!(table.rows[blank].minutes, "—");
        assert!(
            table
                .preferred
                .iter()
                .all(|width| width.is_finite() && *width > 0.0)
        );
        assert!(table.preferred[0] <= 240.0 && table.preferred[1] <= 240.0);
        let clone = catalog.clone();
        assert!(Arc::ptr_eq(clone.artist_table.as_ref().unwrap(), table));
    }

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
    #[test]
    fn song_visual_keys_follow_identity_and_distinguish_repeated_occurrences() {
        let catalog = Catalog::new(vec![
            Song {
                id: Some(1),
                title: "First".into(),
                ..Default::default()
            },
            Song {
                id: Some(2),
                title: "Second".into(),
                ..Default::default()
            },
        ]);
        let original = catalog.all.as_ref().unwrap();
        let reordered = catalog.library.make_collection(
            "Reordered",
            "",
            vec![1, 0, 0],
            PlaybackContext::Library,
        );
        assert_eq!(reordered.row_keys[0], original.row_keys[1]);
        assert_eq!(reordered.row_keys[1], original.row_keys[0]);
        assert_ne!(reordered.row_keys[1], reordered.row_keys[2]);
        assert!(reordered.row_keys.iter().all(|key| !matches!(*key, 0 | 1)));
    }
}
