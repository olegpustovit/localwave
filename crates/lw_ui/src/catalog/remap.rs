//! Prepared on the worker against the displayed page, including repeated-song occurrences.
use super::{Catalog, Collection, Library, Route, View};
use crate::browse::{Query, Scope};
use lw_core::context::PlaybackContext;
use std::collections::HashMap;
use std::sync::{Arc, Weak};

enum Selection {
    Clear,
    Same(usize),
    Changed(Box<[Option<usize>]>),
}
struct Mapping {
    route: Route,
    selection: Selection,
}
impl Mapping {
    fn resolve(&self, selected: Option<usize>) -> (Route, Option<usize>) {
        #[cfg(test)]
        VISITS.with(|visits| visits.set(visits.get() + 1));
        (
            self.route,
            selected.and_then(|index| match &self.selection {
                Selection::Clear => None,
                Selection::Same(len) => (index < *len).then_some(index),
                Selection::Changed(rows) => {
                    #[cfg(test)]
                    VISITS.with(|visits| visits.set(visits.get() + 1));
                    rows.get(index).copied().flatten()
                }
            }),
        )
    }
}

#[cfg(test)]
thread_local! { static VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

fn identity(context: &PlaybackContext) -> (Option<i64>, Option<&PlaybackContext>) {
    match context {
        PlaybackContext::Playlist { id, .. } => (Some(*id), None),
        context => (None, Some(context)),
    }
}

pub struct Remap {
    from_library: Weak<Library>,
    from_view: Option<Weak<View>>,
    to_library: Weak<Library>,
    to_view: Option<Weak<View>>,
    songs: Mapping,
    albums: Box<[Mapping]>,
    artists: Box<[Mapping]>,
    collections: Box<[Mapping]>,
    find: Mapping,
    find_query: Option<Query>,
}
impl Remap {
    pub(super) fn new(previous: &Catalog, next: &Catalog) -> Self {
        let mut routes = HashMap::new();
        for (route, group) in next
            .library
            .all
            .iter()
            .map(|group| (Route::Songs, group))
            .chain(
                next.albums
                    .iter()
                    .enumerate()
                    .map(|(index, group)| (Route::Album(index), group)),
            )
            .chain(
                next.artists
                    .iter()
                    .enumerate()
                    .map(|(index, group)| (Route::Artist(index), group)),
            )
            .chain(
                next.collections
                    .iter()
                    .enumerate()
                    .map(|(index, group)| (Route::Collection(index), group)),
            )
        {
            routes
                .entry((std::mem::discriminant(&route), identity(&group.context)))
                .or_insert(route);
        }
        let mut counts = HashMap::<i64, usize>::new();
        let mut positions = HashMap::new();
        let mut mapping = |old: Option<&Collection>, new: Option<&Collection>, route| {
            let selection = match old
                .zip(new)
                .filter(|(old, new)| identity(&old.context) == identity(&new.context))
            {
                None => Selection::Clear,
                Some((old, new)) if old.ids == new.ids => Selection::Same(old.ids.len()),
                Some((old, new)) => {
                    counts.clear();
                    positions.clear();
                    for (index, id) in new.ids.iter().copied().enumerate() {
                        let ordinal = counts.entry(id).or_default();
                        positions.insert((id, *ordinal), index);
                        *ordinal += 1;
                    }
                    counts.clear();
                    Selection::Changed(
                        old.ids
                            .iter()
                            .map(|id| {
                                let ordinal = counts.entry(*id).or_default();
                                let index = positions.get(&(*id, *ordinal)).copied();
                                *ordinal += 1;
                                index
                            })
                            .collect(),
                    )
                }
            };
            Mapping { route, selection }
        };
        let mut group_mapping = |route, fallback| {
            let old = previous.collection(route);
            let route = old
                .and_then(|old| {
                    routes.get(&(std::mem::discriminant(&route), identity(&old.context)))
                })
                .copied()
                .unwrap_or(fallback);
            mapping(old, next.collection(route), route)
        };
        let songs = group_mapping(Route::Songs, Route::Songs);
        let albums = (0..previous.albums.len())
            .map(|index| group_mapping(Route::Album(index), Route::Albums))
            .collect();
        let artists = (0..previous.artists.len())
            .map(|index| group_mapping(Route::Artist(index), Route::Artists))
            .collect();
        let collections = (0..previous.collections.len())
            .map(|index| group_mapping(Route::Collection(index), Route::Songs))
            .collect();
        let old_find = previous
            .view
            .as_ref()
            .filter(|view| view.request.scope == Scope::Find);
        let new_find = next
            .view
            .as_ref()
            .filter(|view| view.request.scope == Scope::Find);
        let find_query = old_find
            .zip(new_find)
            .filter(|(old, new)| old.request.query == new.request.query)
            .map(|(old, _)| old.request.query);
        let find = mapping(
            old_find.map(|view| &view.collection),
            new_find.map(|view| &view.collection),
            Route::Find,
        );
        Self {
            from_library: Arc::downgrade(&previous.library),
            from_view: previous.view.as_ref().map(Arc::downgrade),
            to_library: Arc::downgrade(&next.library),
            to_view: next.view.as_ref().map(Arc::downgrade),
            songs,
            albums,
            artists,
            collections,
            find,
            find_query,
        }
    }
    pub fn matches(&self, previous: &Catalog, next: &Catalog) -> bool {
        let view = |weak: &Option<Weak<View>>, strong: &Option<Arc<View>>| match (weak, strong) {
            (None, None) => true,
            (Some(weak), Some(strong)) => weak.as_ptr() == Arc::as_ptr(strong),
            _ => false,
        };
        self.from_library.as_ptr() == Arc::as_ptr(&previous.library)
            && self.to_library.as_ptr() == Arc::as_ptr(&next.library)
            && view(&self.from_view, &previous.view)
            && view(&self.to_view, &next.view)
    }
    pub(crate) fn resolve(
        &self,
        route: Route,
        selected: Option<usize>,
        query: Query,
    ) -> (Route, Option<usize>) {
        #[cfg(test)]
        VISITS.with(|visits| visits.set(visits.get() + 1));
        let mapping = match route {
            Route::Songs => Some(&self.songs),
            Route::Album(index) => self.albums.get(index),
            Route::Artist(index) => self.artists.get(index),
            Route::Collection(index) => self.collections.get(index),
            Route::Find if self.find_query == Some(query) => Some(&self.find),
            _ => None,
        };
        mapping.map_or_else(
            || {
                (
                    match route {
                        Route::Album(_) => Route::Albums,
                        Route::Artist(_) => Route::Artists,
                        Route::Collection(_) => Route::Songs,
                        route => route,
                    },
                    None,
                )
            },
            |mapping| mapping.resolve(selected),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Intent, ShellState};
    use lw_core::Song;

    #[test]
    fn ten_thousand_song_refresh_has_constant_probes_zero_allocations_and_prepared_byline_routes() {
        let song = |id: i64| Song {
            id: Some(id),
            title: format!("Track {id:05}"),
            artist: format!("B{id:05}"),
            album: format!("Album {id:05}"),
            ..Default::default()
        };
        let mut previous = Catalog::new((0..10_000).map(song).collect());
        previous.add_collection(
            "Old name",
            vec![9_999, 9_999],
            PlaybackContext::Playlist {
                id: 7,
                name: "Old name".into(),
            },
        );
        let mut songs = (0..10_000).rev().map(song).collect::<Vec<_>>();
        songs.push(Song {
            id: Some(10_000),
            artist: "A".into(),
            album: "Inserted".into(),
            ..Default::default()
        });
        let mut next = Catalog::new(songs);
        next.add_collection(
            "Renamed",
            vec![1, 0, 0],
            PlaybackContext::Playlist {
                id: 7,
                name: "Renamed".into(),
            },
        );
        next.prepare_remap(&previous);
        let mut state = ShellState::default();
        for (route, selected, expected_route, expected_selected) in [
            (Route::Songs, 9_999, Route::Songs, 0),
            (Route::Album(9_999), 0, Route::Album(10_000), 0),
            (Route::Artist(9_999), 0, Route::Artist(10_000), 0),
            (Route::Collection(3), 1, Route::Collection(3), 2),
        ] {
            state.apply(Intent::Navigate(route));
            state.selected = Some(selected);
            VISITS.with(|visits| visits.set(0));
            mem_sys::local_reset();
            state.refresh(&previous, &next);
            assert_eq!(
                (state.route, state.selected),
                (expected_route, Some(expected_selected))
            );
            assert!(VISITS.with(|visits| visits.get()) <= 3);
            assert_eq!(
                next.byline_routes(9_999),
                Some([Route::Artist(10_000), Route::Album(10_000)])
            );
            assert_eq!(mem_sys::local_alloc_count(), 0);
        }
        assert!(
            !next
                .remap
                .as_ref()
                .unwrap()
                .matches(&Catalog::default(), &next)
        );
    }

    #[test]
    fn find_remaps_require_the_displayed_query_and_duplicate_occurrence() {
        let mut previous = Catalog::new(
            (0..3)
                .map(|id| Song {
                    id: Some(id),
                    ..Default::default()
                })
                .collect(),
        );
        let request = crate::browse::Request {
            scope: Scope::Find,
            query: Query::new("a").unwrap(),
            commands: None,
            sort: crate::browse::Sort::Original,
            ascending: true,
        };
        previous.project(request, vec![0, 1, 0]);
        let mut next = previous.clone();
        next.project(request, vec![2, 0, 0, 1]);
        next.prepare_remap(&previous);
        let mut state = ShellState::default();
        state.apply(Intent::Navigate(Route::Find));
        state.apply(Intent::Search(request.query));
        state.selected = Some(2);
        state.refresh(&previous, &next);
        assert_eq!(state.selected, Some(2));
        state.apply(Intent::Search(Query::new("b").unwrap()));
        state.selected = Some(2);
        state.refresh(&previous, &next);
        assert_eq!(
            state.selected, None,
            "stale results cannot lend their selection to a new query"
        );
        let mut different = next.clone();
        different.project(
            crate::browse::Request {
                query: state.query,
                ..request
            },
            vec![0, 0],
        );
        different.prepare_remap(&previous);
        state.selected = Some(1);
        state.refresh(&previous, &different);
        assert_eq!(
            state.selected, None,
            "a reply for the new query starts without an old-query selection"
        );
    }
}
