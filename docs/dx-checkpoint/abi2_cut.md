ABI 2.0 cut list (collected; the OS audit adds more). Dropped at the freeze:
- Legacy/superseded C exports: pxui_label, pxui_ui_label (-> pxui_ui_label_node); pxui_ui_button_response (-> pxui_ui_button_content);
  pxui_ui_tooltip; pxui_ui_inspector_section_view; pxui_ui_inspector_toggle_value; pxui_ui_animate_keys_repeat (one animate_keys);
  pxui_ui_context_menu's has_selected/selected/lead_from fields (checked supersedes); pxui_text_field_with_options; pxui_col; pxui_scope.
- Old-size record reads (button 96/104, icon button 96, toggle/segmented/menu-extra pre-growth sizes, FrameIn 152…) — 2.0 records
  have one size each.
- Event: size-prefixed record with a per-event surface id.
- Colour token in-band sentinel: decide at 2.0 whether a dedicated look record field replaces it.
- Manifests re-baselined at 2.0; fixtures/abi regenerated; docs/abi.md history collapsed to a 2.0 baseline row.
- Menu per-row data: PxuiMenuExtra has parallel columns (ids, has_id, selected, checked, danger…); at 2.0 decide one per-row record.
- Feature discovery at 2.0: PXUI_FEATURE_* registry (DX7.4) becomes the 2.0 baseline's feature list; version-named features not used.
- Text input C exports superseded by batch E (pxui_text_field, pxui_text_field_with_options, pxui_ui_search_field, pxui_ui_line_field, pxui_ui_value_field, pxui_ui_text_area*): drop at 2.0.
- Accepted exception (DX5.4): C worker/shaper handles are not validated (pointer must come from create and not be destroyed); misuse only affects the calling app's process. Revisit at 2.0 if a handle table is wanted.
- Batch F1 legacy exports: pxui_ui_checkbox, _radio, _tabs, _dropdown, _slider, and the label-keyed progress/spinner/badge/popup/modal/anchored/section_list/tree/table_view/scroll_view/toasts/picker exports superseded by keyed size-prefixed descs: drop at 2.0.
- Segmented legacy: pxui_ui_segmented, pxui_ui_segmented_plain, pxui_ui_segmented_response, choice kind 2: drop at 2.0 (superseded by pxui_ui_segmented_result).
- 2.0 cut C1.4: the Rays extension set (pxui_core kit_ext.rs inspector/picker/line_field/value_field + their body kinds and exports) leaves the library (Rays crate takes it).

Validation tightened at 2.0
- PxuiPlacement.align (in PxuiNode.anchor) and PxuiAnchorDesc.align (pxui_ui_anchored*): 3/4 (PXUI_ALIGN_STRETCH/_BASELINE) accepted for 1.x as START (DX1.12C, Rust type `layout::Along` has Start/Center/End only); refuse at 2.0 and drop the legacy_anchor_align copy in pxui_abi/node.rs.
- pxui_paint_image (void, drops invalid input silently): superseded by a checked export (batch B) — drop at 2.0.
- DX1.12G body vocabulary (C source-level renames, layout/numbers unchanged; 1.x keeps the old C names, Rust and theme-file keys renamed in 1.x): PxuiBodies fields and BodyKind names context_menu->menu_row, list_view->selection_list, table_view->table_row, table->table_cell, tab->tabs, segment->segmented, toast->toasts, tip->tooltip; C records PxuiTabConfig->PxuiTabsConfig, PxuiSegmentConfig->PxuiSegmentedConfig, PxuiToastConfig->PxuiToastsConfig, PxuiTipConfig->PxuiTooltipConfig (+ matching _Static_asserts in pxui.h).
