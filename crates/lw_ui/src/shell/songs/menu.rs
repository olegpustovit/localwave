use super::*;

pub(in crate::shell) fn popup(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    target: (usize, i64, u64),
    at: (f32, f32),
    sharing: bool,
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let click = at;
    let present = state.collection(catalog).is_some_and(|collection| {
        collection.key == target.2 && collection.ids.get(target.0) == Some(&target.1)
    });
    let row = catalog.row_index(target.1).map(|index| catalog.row(index));
    let favorite = row.is_some_and(|row| row.favorite);
    let list = state
        .collection(catalog)
        .and_then(|collection| match &*collection.context {
            lw_core::context::PlaybackContext::Playlist { id, .. } => Some(*id),
            _ => None,
        });
    let routes = catalog.byline_routes(target.1);
    // Swift's LWTrackMenu order.
    let mut items: [Option<(&'static str, bool, Intent)>; 13] = [None; 13];
    let mut count = 0;
    for entry in [
        Some(("Play", true, Intent::PlayRow(target.0))),
        Some(("Play next", true, Intent::PlayNext(target.1))),
        Some(("Add to queue", true, Intent::AppendSong(target.1))),
        Some(("Add to list", true, Intent::SongMenuLists)),
        routes.map(|routes| ("Go to album", true, Intent::Navigate(routes[1]))),
        routes.map(|routes| ("Go to artist", true, Intent::Navigate(routes[0]))),
        cfg!(not(target_os = "ios")).then_some((
            "Show in Finder",
            row.is_some_and(|row| row.source_id.is_some()),
            Intent::RevealSong(target.1),
        )),
        Some((
            if favorite { "Unfavorite" } else { "Favorite" },
            true,
            Intent::SetFavorite {
                song: target.1,
                on: !favorite,
            },
        )),
        Some((
            if row.is_some_and(|row| row.pinned) {
                "Stop keeping offline"
            } else {
                "Keep offline"
            },
            true,
            Intent::KeepOffline(target.1),
        )),
        row.filter(|row| row.state == lw_core::FileState::Failed)
            .map(|_| ("Retry download", true, Intent::RetryDownload(target.1))),
        Some((
            "Share…",
            row.is_some_and(|row| row.source_id.is_some()) && !sharing,
            Intent::ShareSong(target.1, click),
        )),
        list.map(|list| {
            (
                "Remove from list",
                true,
                Intent::RemoveFromList {
                    song: target.1,
                    list,
                },
            )
        }),
    ] {
        if entry.is_some() {
            items[count] = entry;
            count += 1;
        }
    }
    let lists = state.song_lists;
    let others = catalog
        .collections
        .iter()
        .filter(|collection| match &*collection.context {
            lw_core::context::PlaybackContext::Playlist { id, .. } => Some(*id) != list,
            _ => false,
        })
        .count();
    let rows = if lists { others.max(1) + 1 } else { count };
    let height = rows as f32 * 36.0;
    let (w, h) = ui.view_size();
    let at = (
        at.0.clamp(0.0, (w - 200.0).max(0.0)),
        at.1.clamp(0.0, (h - height.min(8.0 * 36.0)).max(0.0)),
    );
    let mut accepted = false;
    let open = ui.popup(
        Some(p.line),
        lists.then_some(8.0 * 36.0),
        false,
        false,
        at,
        200.0,
        height,
        if lists { "Add to list" } else { "Song actions" },
        &mut |ui| {
            if lists {
                {
                    let item = ui.node("new-list", item_node(present, p));
                    ui.text(&item, "New list…");
                    if present && ui.signal_state(&item).clicked {
                        *intent = Some(Intent::NewList(Some(target.1)));
                        accepted = true;
                    }
                    if others == 0 {
                        let item = ui.node("none", item_node(false, p));
                        ui.text(&item, "No other lists");
                    }
                    for collection in &catalog.collections {
                        let lw_core::context::PlaybackContext::Playlist { id, .. } =
                            &*collection.context
                        else {
                            continue;
                        };
                        if Some(*id) == list {
                            continue;
                        }
                        let item = ui.node_keyed(pxui::ui::int_key(*id, 7), item_node(present, p));
                        ui.text(&item, &collection.title);
                        if present && ui.signal_state(&item).clicked {
                            *intent = Some(Intent::AddToList {
                                song: target.1,
                                list: *id,
                            });
                            accepted = true;
                        }
                    }
                }
                return;
            }
            for (label, enabled, action) in items[..count].iter().flatten().copied() {
                let enabled = enabled && present;
                let item = ui.node(label, item_node(enabled, p));
                ui.text(&item, label);
                if enabled && ui.signal_state(&item).clicked {
                    *intent = Some(action);
                    // Opening the list panel keeps this popup.
                    accepted = action != Intent::SongMenuLists;
                }
            }
        },
    );
    if accepted {
        ui.dismiss_popup();
    }
    if !open && intent.is_none() {
        *intent = Some(Intent::CloseSongMenu);
    }
}

fn item_node(enabled: bool, p: Palette) -> Node {
    Node {
        flags: if enabled { CLICKABLE | TAB_STOP } else { 0 },
        role: Role::MenuItem,
        state: State {
            disabled: !enabled,
            ..State::default()
        },
        w: Size::Grow,
        h: Size::Px(36.0),
        padding: Edges {
            l: 12.0,
            r: 12.0,
            ..Edges::ZERO
        },
        look: Look {
            text: p.ink,
            font: Font {
                size: 13,
                ..Font::BODY
            },
            ..Look::NONE
        },
        ..Node::DEFAULT
    }
}
