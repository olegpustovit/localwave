//! The phone page "…" menu (Swift's albumMenuGroups, automaticMenuGroups and listMenuGroups): whole-page
//! actions on the displayed collection. Labels are static; the controller applies `Intent::Page`.
use super::*;
use crate::PageAction;
use lw_core::context::PlaybackContext;

pub(in crate::shell) fn menu(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    at: (f32, f32),
    p: Palette,
    intent: &mut Option<Intent>,
) {
    // A folder page only gets the deck's list choices; its songs are the marked rows.
    let folder = state.route == Route::Files;
    let collection = state.collection(catalog);
    if collection.is_none() && !(folder && state.page_lists) {
        *intent = Some(Intent::ClosePageMenu);
        return;
    }
    let context = collection.map(|collection| &*collection.context);
    let (list, album) = match context {
        Some(PlaybackContext::Playlist { id, .. }) => (Some(*id), false),
        Some(PlaybackContext::Album { .. }) => (None, true),
        _ => (None, false),
    };
    let artist = match context {
        Some(PlaybackContext::Album { artist, .. }) => catalog.library.artists.iter().position(|a| a.title == *artist),
        _ => None,
    };
    let filled = collection.is_some_and(|collection| !collection.ids.is_empty());
    let kept = filled && collection.is_some_and(|collection| collection.rows.iter().all(|&row| catalog.row(row).pinned));
    let name = list.map_or(crate::browse::Query::default(), |_| {
        crate::browse::Query::new(collection.map_or("", |collection| &collection.title)).unwrap_or_default()
    });
    let mut items: [Option<(&'static str, bool, Intent, bool)>; 11] = [None; 11];
    let mut count = 0;
    for entry in [
        Some(("Play", filled, Intent::Page(PageAction::Play), false)),
        Some(("Shuffle", filled, Intent::Page(PageAction::Shuffle), false)),
        album.then_some(("Play next", filled, Intent::Page(PageAction::PlayNext), false)),
        Some(("Add to queue", filled, Intent::Page(PageAction::AddToQueue), false)),
        album.then_some(("Add to list", filled, Intent::PageLists, false)),
        artist.map(|index| ("Go to artist", true, Intent::Navigate(Route::Artist(index)), false)),
        list.map(|list| ("Rename…", true, Intent::RenameListDialog(list, name), false)),
        list.map(|_| ("Duplicate", true, Intent::Page(PageAction::Duplicate), false)),
        album.then_some(("Show in folder", filled, Intent::Page(PageAction::ShowInFolder), false)),
        Some((
            if kept { "Stop keeping offline" } else { "Keep offline" },
            filled,
            Intent::Page(PageAction::KeepOffline),
            false,
        )),
        list.map(|list| ("Delete list", true, Intent::ConfirmDeleteList(list), true)),
    ] {
        if entry.is_some() {
            items[count] = entry;
            count += 1;
        }
    }
    let lists = state.page_lists;
    let others = catalog.collections.iter().filter(|c| matches!(&*c.context, PlaybackContext::Playlist { .. })).count();
    // "New list…" leads the choices, as in Swift's addToListGroups.
    let rows = if lists { others.max(1) + 1 } else { count };
    let height = rows as f32 * 36.0;
    let (w, h) = ui.view_size();
    let at = (
        (at.0 - 200.0).clamp(0.0, (w - 200.0).max(0.0)),
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
        if lists { "Add to list" } else { "Page actions" },
        &mut |ui| {
            if lists {
                let item = ui.node("new-list", super::super::songs::item_node(true, p));
                ui.text(&item, "New list…");
                if ui.signal_state(&item).clicked {
                    *intent = Some(Intent::NewPageList);
                    accepted = true;
                }
                if others == 0 {
                    let item = ui.node("none", super::super::songs::item_node(false, p));
                    ui.text(&item, "No other lists");
                }
                for c in &catalog.collections {
                    let PlaybackContext::Playlist { id, .. } = &*c.context else { continue };
                    let item = ui.node_keyed(pxui::ui::int_key(*id, 7), super::super::songs::item_node(true, p));
                    ui.text(&item, &c.title);
                    if ui.signal_state(&item).clicked {
                        *intent = Some(Intent::Page(PageAction::AddToList(*id)));
                        accepted = true;
                    }
                }
                return;
            }
            for (label, enabled, action, danger) in items[..count].iter().flatten().copied() {
                let node = ui.node(label, super::super::songs::item_node(enabled, p));
                if danger {
                    ui.set_look(&node, Look { text: p.danger, font: Font { size: 13, ..Font::BODY }, ..Look::NONE }, State::default());
                }
                ui.text(&node, label);
                if enabled && ui.signal_state(&node).clicked {
                    *intent = Some(action);
                    accepted = action != Intent::PageLists;
                }
            }
        },
    );
    if accepted {
        ui.dismiss_popup();
    }
    if !open && intent.is_none() {
        *intent = Some(Intent::ClosePageMenu);
    }
}
