//! Isolated fixture I/O stays on the library worker; startup waits borrow the framework Window.
use super::Workload;
use crate::{App, smoke};
use lw_bin::{
    controller::{Command, Controller},
    library::LibraryWorker,
};
use pxui_sdl::Window;
use std::path::Path;

pub(crate) fn library(base: &Path, workload: Option<Workload>) -> std::io::Result<LibraryWorker> {
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
    )?;
    if workload.is_some() {
        if matches!(workload, Some(Workload::Settings)) {
            library.save_settings(&lw_core::preferences::Settings {
                appearance: lw_core::preferences::Appearance::Light,
                ..Default::default()
            })?;
        }
        smoke::seed_catalog(&base.join("library.sqlite"))?;
        if matches!(workload, Some(Workload::Artists)) {
            smoke::seed_artists(&base.join("library.sqlite"))?;
        }
        if matches!(workload, Some(Workload::Artwork)) {
            smoke::seed_artwork(&base.join("library.sqlite"), &base.join("Documents"))?;
        }
        smoke::seed_waveform(&base.join("Caches"))?;
        if matches!(workload, Some(Workload::Files)) {
            smoke::seed_files(&mut library)?;
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
