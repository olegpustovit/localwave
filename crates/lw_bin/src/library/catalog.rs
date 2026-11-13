use super::LibraryWorker;
use lw_core::context::PlaybackContext;
use lw_ui::catalog::Catalog;
use std::collections::HashMap;
use std::io;

impl LibraryWorker {
    pub fn catalog(&self) -> io::Result<Catalog> {
        self.repository
            .read_snapshot(|repository| {
                let count = repository.song_count("")?;
                let songs = repository.search_songs("", count, 0)?;
                let mut catalog = Catalog::new(songs);
                let by_id: HashMap<_, _> = (0..catalog.len())
                    .map(|index| (catalog.row(index).id, index))
                    .collect();
                for list in repository.playlists()? {
                    let rows = repository
                        .playlist_songs(list.id)?
                        .into_iter()
                        .filter_map(|song| by_id.get(&song.id?).copied())
                        .collect();
                    catalog.add_collection(
                        &list.name,
                        rows,
                        PlaybackContext::Playlist {
                            id: list.id,
                            name: list.name.clone(),
                        },
                    );
                }
                Ok(catalog)
            })
            .map_err(io::Error::other)
    }
}
