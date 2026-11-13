use super::LibraryWorker;
use lw_core::context::PlaybackContext;
use lw_ui::catalog::Catalog;
use std::collections::HashMap;
use std::io;

impl LibraryWorker {
    pub fn catalog(&self) -> io::Result<Catalog> {
        let count = self.repository.song_count("").map_err(io::Error::other)?;
        let songs = self
            .repository
            .search_songs("", count, 0)
            .map_err(io::Error::other)?;
        let mut catalog = Catalog::new(songs);
        let by_id: HashMap<_, _> = (0..catalog.len())
            .map(|index| (catalog.row(index).id, index))
            .collect();
        for list in self.repository.playlists().map_err(io::Error::other)? {
            let rows = self
                .repository
                .playlist_songs(list.id)
                .map_err(io::Error::other)?
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
    }
}
