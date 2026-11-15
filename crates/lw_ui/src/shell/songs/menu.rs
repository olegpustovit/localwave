use super::*;

pub(in crate::shell) fn popup(
    ui: &mut Ui,
    catalog: &Catalog,
    state: &ShellState,
    target: (usize, i64, u64),
    at: (f32, f32),
    p: Palette,
    intent: &mut Option<Intent>,
) {
    let present = state.collection(catalog).is_some_and(|collection| {
        collection.key == target.2 && collection.ids.get(target.0) == Some(&target.1)
    });
    let favorite = catalog
        .row_index(target.1)
        .is_some_and(|index| catalog.row(index).favorite);
    let (w, h) = ui.view_size();
    let at = (
        at.0.clamp(0.0, (w - 200.0).max(0.0)),
        at.1.clamp(0.0, (h - 108.0).max(0.0)),
    );
    let mut accepted = false;
    let open = ui.popup(
        Some(p.line),
        None,
        false,
        false,
        at,
        200.0,
        108.0,
        "Song actions",
        &mut |ui| {
            for (label, action) in [
                ("Play", Intent::PlayRow(target.0)),
                ("Add to queue", Intent::AppendSong(target.1)),
                (
                    if favorite { "Unfavorite" } else { "Favorite" },
                    Intent::SetFavorite {
                        song: target.1,
                        on: !favorite,
                    },
                ),
            ] {
                let item = ui.node(
                    label,
                    Node {
                        flags: if present { CLICKABLE | TAB_STOP } else { 0 },
                        role: Role::MenuItem,
                        state: State {
                            disabled: !present,
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
                    },
                );
                ui.text(&item, label);
                if present && ui.signal_state(&item).clicked {
                    *intent = Some(action);
                    accepted = true;
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
