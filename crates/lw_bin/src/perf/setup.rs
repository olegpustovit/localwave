//! Isolated fixture I/O stays on the library worker; startup waits borrow the framework Window.
use super::{Workload, drive};
use crate::app::App;
use lw_bin::{
    controller::{Command, Controller},
    library::LibraryWorker,
};
use pxui_sdl::Window;
use std::path::Path;

pub(crate) const LIST_NAME: &str = "Qualification mix";

pub(crate) fn library(base: &Path, workload: Option<Workload>, table_styles: lw_ui::catalog::TableStyles) -> std::io::Result<LibraryWorker> {
    let documents = base.join("Documents");
    std::fs::create_dir_all(&documents)?;
    let suite = format!(
        "com.olegpustovit.localwave.{}",
        base.file_name().unwrap().to_string_lossy()
    );
    let mut library = LibraryWorker::open(
        &base.join("library.sqlite"),
        lw_apple_sys::preferences::Preferences::open(Some(&suite))?,
        lw_apple_sys::preferences::AppPaths {
            documents,
            caches: base.join("Caches"),
            temporary: base.join("tmp"),
            test_database: None,
        },
        table_styles,
    )?;
    if workload.is_some() {
        if matches!(workload, Some(Workload::Settings)) {
            library.save_settings(&lw_core::preferences::Settings {
                appearance: lw_core::preferences::Appearance::Light,
                ..Default::default()
            })?;
        }
        seed_catalog(&base.join("library.sqlite"))?;
        if matches!(workload, Some(Workload::Artists)) {
            seed_artists(&base.join("library.sqlite"))?;
        }
        if matches!(workload, Some(Workload::Artwork | Workload::AlbumDetail)) {
            seed_artwork(&base.join("library.sqlite"), &base.join("Documents"))?;
        }
        if matches!(
            workload,
            Some(Workload::ListDialogs | Workload::SongDrag | Workload::FirstMenuOpen)
        ) {
            // A sidebar list row for the menu and drop target, in the isolated fixture only.
            library
                .repository
                .create_playlist(LIST_NAME, 1.0)
                .map_err(std::io::Error::other)?;
        }
        seed_waveform(&base.join("Caches"))?;
        if matches!(workload, Some(Workload::Files)) {
            seed_files(&mut library)?;
        }
        library.save_player_state(&lw_core::preferences::PlayerState {
            song_ids: if workload.is_some_and(Workload::uses_queue) {
                (1..=10_000).collect()
            } else {
                vec![1]
            },
            current_index: 0,
            volume: Some(0.0),
            context_data: None,
        })?;
    }
    Ok(library)
}

pub(crate) fn wait_ready(
    window: &mut Window,
    player: &mut Controller,
    smoke: bool,
    workload: Option<Workload>,
) -> Result<Option<i64>, String> {
    if cfg!(target_os = "macos") && (smoke || workload.is_some()) {
        let deadline = window.time() + 5.0;
        loop {
            player.poll().map_err(|error| error.to_string())?;
            if player.status().initialized {
                if !player.status().message.is_empty() {
                    return Err(player.status().message.to_string());
                }
                // Queue and status travel through separate bounded handoffs.
                if (!workload.is_some_and(Workload::uses_queue) || player.catalog().queue.is_some())
                    && (workload.is_none() || player.status().waveform.is_some())
                {
                    break;
                }
            }
            if window.time() >= deadline {
                return Err("Library initialization timed out during qualification".into());
            }
            window.wait(Some(window.time() + 0.05));
        }
    }
    if workload.is_some() && player.catalog().len() != 10_000 {
        return Err("Performance fixture must contain 10,000 songs".into());
    }
    if workload.is_some_and(Workload::uses_queue)
        && player
            .catalog()
            .queue
            .as_ref()
            .map(|queue| queue.upcoming().len())
            != Some(9_999)
    {
        return Err("Queue performance fixture must contain 9,999 upcoming occurrences".into());
    }
    let indicator_song = if matches!(workload, Some(Workload::SongIndicators)) {
        Some(
            player
                .catalog()
                .row(
                    *player
                        .catalog()
                        .all
                        .as_ref()
                        .unwrap()
                        .rows
                        .get(15)
                        .ok_or("Song indicator fixture needs sixteen rows")?,
                )
                .id,
        )
    } else {
        None
    };
    Ok(indicator_song)
}

pub(crate) fn prepare(
    app: &mut App,
    workload: Workload,
    window: &mut Window,
) -> Result<(), String> {
    let smoke = app.smoke;
    let workload = Some(workload);
    let App {
        player,
        files,
        shell_state,
        ..
    } = app;
    if matches!(workload, Some(Workload::Settings)) {
        shell_state.apply(lw_ui::Intent::ShowSettings);
        player
            .command(Command::ReadStorage)
            .map_err(|error| error.to_string())?;
    }
    if matches!(workload, Some(Workload::Files)) {
        use lw_ui::files::{Location, Request};
        shell_state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Files));
        files
            .request(Request::Roots)
            .map_err(|error| error.to_string())?;
        let deadline = window.time() + 5.0;
        let mut browsing = false;
        loop {
            files.poll();
            let snapshot = files.snapshot();
            if !browsing
                && let Some(user) = snapshot.account
                && let Some(source) = snapshot.sources.first()
            {
                files
                    .request(Request::Browse(Location {
                        user,
                        source: source.id,
                        parent: None,
                    }))
                    .map_err(|error| error.to_string())?;
                browsing = true;
            }
            if files
                .snapshot()
                .directory
                .as_ref()
                .is_some_and(|directory| directory.entries.len() == 10_000)
            {
                break;
            }
            if window.time() >= deadline {
                return Err(format!(
                    "Files fixture preparation failed: {}",
                    files.snapshot().message
                ));
            }
            window.wait(Some(window.time() + 0.05));
        }
    }
    if matches!(workload, Some(Workload::Artwork)) {
        shell_state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Albums));
    }
    if let Some(route @ (Workload::AlbumDetail | Workload::ArtistDetail)) = workload {
        use lw_ui::catalog::Route;
        shell_state.apply(lw_ui::Intent::Navigate(if route == Workload::AlbumDetail {
            Route::Album(0)
        } else {
            Route::Artist(0)
        }));
        if shell_state
            .collection(player.catalog())
            .is_none_or(|group| group.rows.len() < 50)
        {
            return Err("Detail performance fixture needs a scoped table of 50+ songs".into());
        }
    }
    if matches!(workload, Some(Workload::SearchTyping)) {
        shell_state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Find));
    }
    if matches!(workload, Some(Workload::MetadataEditor | Workload::TextTyping)) {
        use lw_ui::browse::Query;
        let field = |text| Query::new(text).unwrap_or_default();
        shell_state.apply(lw_ui::Intent::OpenMetadata(lw_ui::MetadataEdit {
            song: 1,
            title: field(drive::TITLE_TEXT),
            artist: field("Artist 1"),
            album: field("Album 1"),
            album_artist: field("Artist 1"),
            year: field("2019"),
            track: field("1"),
        }));
    }
    if matches!(workload, Some(Workload::Artists)) {
        if player.catalog().artists.len() != 10_000
            || player
                .catalog()
                .artist_table
                .as_ref()
                .is_none_or(|table| table.rows.len() != 10_000)
        {
            return Err("Artist performance fixture must contain 10,000 prepared artists".into());
        }
        shell_state.apply(lw_ui::Intent::Navigate(lw_ui::catalog::Route::Artists));
    }
    if matches!(workload, Some(Workload::Find)) {
        use lw_ui::{
            Intent,
            browse::{Query, Sort},
            catalog::Route,
        };
        shell_state.apply(Intent::Navigate(Route::Find));
        shell_state.apply(Intent::Search(
            Query::new("Track m4a year:2019 unplayed").unwrap(),
        ));
        shell_state.apply(Intent::Sort(Sort::Title));
        shell_state.apply(Intent::Sort(Sort::Title));
        let request = shell_state.request(player.catalog()).unwrap();
        player.browse(request).map_err(|error| error.to_string())?;
        let deadline = window.time() + 5.0;
        loop {
            player
                .poll_with_catalog(&mut |previous, next| shell_state.refresh(previous, next))
                .map_err(|error| error.to_string())?;
            if player
                .catalog()
                .view
                .as_ref()
                .is_some_and(|view| view.request == request)
            {
                if shell_state
                    .collection(player.catalog())
                    .map(|group| group.rows.len())
                    != Some(10_000)
                {
                    return Err("Find performance fixture must match 10,000 songs".into());
                }
                break;
            }
            if window.time() >= deadline {
                return Err("Find preparation timed out during qualification".into());
            }
            window.wait(Some(window.time() + 0.05));
        }
    }
    player
        .enable_media_controls(!smoke && workload.is_none())
        .map_err(|error| error.to_string())?;
    if matches!(workload, Some(Workload::QueueSheet)) {
        shell_state.apply(lw_ui::Intent::ShowQueue);
        shell_state.apply(lw_ui::Intent::NamePlaylist);
        shell_state.apply(lw_ui::Intent::EditPlaylistName(
            lw_ui::browse::Query::new("Qualification queue").unwrap(),
        ));
    }
    Ok(())
}

pub(crate) fn configure_ui(ui: &mut pxui::Ui, workload: Option<Workload>) {
    if matches!(
        workload,
        Some(
            Workload::LongScroll
                | Workload::QueueDrag
                | Workload::Artists
                | Workload::SongIndicators
                | Workload::AlbumDetail
                | Workload::ArtistDetail
                | Workload::ListDialogs
                | Workload::FirstMenuOpen
                | Workload::MetadataEditor
                | Workload::SongDrag
                | Workload::Settings
        )
    ) {
        ui.set_a11y(true);
    }
}

/// A valid persisted waveform with no audio copy exercises the same cache delivery as eviction.
pub fn seed_waveform(caches: &std::path::Path) -> std::io::Result<()> {
    let directory = caches.join("LocalWave");
    std::fs::create_dir_all(&directory)?;
    let database = directory.join("waveforms.sqlite");
    let peaks = std::array::from_fn(|index| (index % 64) as f32 / 63.0);
    lw_core::waveform::Repository::open(
        database
            .to_str()
            .ok_or_else(|| std::io::Error::other("Fixture path is not UTF-8"))?,
    )
    .map_err(std::io::Error::other)?
    .put(1, "qualification-cached-waveform", &peaks)
    .map_err(std::io::Error::other)
}

/// Populate only the isolated qualification database, after its normal migration.
pub fn seed_catalog(path: &std::path::Path) -> std::io::Result<()> {
    let path = path
        .to_str()
        .ok_or_else(|| std::io::Error::other("Fixture path is not UTF-8"))?;
    let db = lw_sqlite::Connection::open(path).map_err(std::io::Error::other)?;
    db.execute_batch("BEGIN IMMEDIATE; WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM seq WHERE n<10000) INSERT INTO songs (songKey,artist,title,album,albumArtist,pathHash,createdAt,duration,fileExtension,isFavorite,releaseYear) SELECT n,'Artist '||(n%10),printf('Track %05d',n),'Album '||(n%100),'Artist '||(n%10),n,n,240,'m4a',n%2,2019 FROM seq; INSERT INTO songs_fts(songId,artist,title,album,albumArtist) SELECT id,artist,title,album,albumArtist FROM songs; COMMIT").map_err(std::io::Error::other)
}

/// A separate artist workload keeps the ten existing fixture distributions unchanged.
pub fn seed_artists(path: &std::path::Path) -> std::io::Result<()> {
    let path = path
        .to_str()
        .ok_or_else(|| std::io::Error::other("Fixture path is not UTF-8"))?;
    let db = lw_sqlite::Connection::open(path).map_err(std::io::Error::other)?;
    db.execute_batch("BEGIN IMMEDIATE; UPDATE songs SET artist=printf('Artist %05d',id),albumArtist=printf('Artist %05d',id); DELETE FROM songs_fts; INSERT INTO songs_fts(songId,artist,title,album,albumArtist) SELECT id,artist,title,album,albumArtist FROM songs; COMMIT").map_err(std::io::Error::other)
}

/// Indexed metadata only: Files performance must never enumerate a user's real folders.
pub fn seed_files(library: &mut lw_bin::library::LibraryWorker) -> std::io::Result<()> {
    let account = library.current_sources()?;
    let source = library
        .repository
        .create_source(&lw_core::Source {
            user_id: account.user.id,
            dir_path: "/qualification/original-music".into(),
            path_id: 1_000_000,
            is_current: true,
            total_paths: Some(10_000),
            created_at: 1.0,
            ..Default::default()
        })
        .map_err(std::io::Error::other)?;
    let paths: Vec<_> = (1..=10_000)
        .map(|id| lw_core::SourcePath {
            source_id: source.id.unwrap(),
            path_id: id,
            parent_path_id: Some(source.path_id),
            name: format!("Track {id:05}.flac"),
            relative_path: format!("Track {id:05}.flac"),
            file_size: Some(2_000_000),
            created_at: 1.0,
            ..Default::default()
        })
        .collect();
    library
        .repository
        .upsert_source_paths(&paths)
        .map_err(std::io::Error::other)
}

pub fn seed_artwork(
    database: &std::path::Path,
    documents: &std::path::Path,
) -> std::io::Result<()> {
    let directory = documents.join("CoverArt");
    std::fs::create_dir_all(&directory)?;
    for index in 0..100 {
        std::fs::write(
            directory.join(format!("benchmark-{index}.png")),
            include_bytes!("../../../../Tests/LocalWaveDataTests/Fixtures/artwork-large.png"),
        )?;
    }
    let database = database
        .to_str()
        .ok_or_else(|| std::io::Error::other("Fixture path is not UTF-8"))?;
    lw_sqlite::Connection::open(database)
        .map_err(std::io::Error::other)?
        .execute_batch("UPDATE songs SET coverArtPath='CoverArt/benchmark-'||(songKey%100)||'.png'")
        .map_err(std::io::Error::other)
}
