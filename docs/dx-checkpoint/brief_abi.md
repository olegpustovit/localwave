ABI additions (this row may add exports): follow docs/abi.md "Policy" → "Adding a call" exactly: the pxui_abi export, the
wrapper method in pxui, the include/pxui.h declaration, appended rows in abi/pxui.v1.txt (and abi/layout.v1.txt via
PXUI_LAYOUT_WRITE=1 cargo t -p pxui_types abi_layout_rows for new records), a docs/abi.md table row, and the ABI_VERSION /
`minor` bump. Use the NEXT minor after the current one in abi/pxui.v1.txt as a provisional number; the coordinator
renumbers at landing if another row landed first. This overrides the brief's "never edit an ABI manifest row" for this row
only. Append only — never change an existing manifest row. Records grow only at the end behind their size prefix.
