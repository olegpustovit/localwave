#!/usr/bin/env python3
"""DX1.12G codemod 2: body accessors become BodyConfig methods; Pxui*Config in Rust bodies -> wrappers. usage: callers.py <files...>"""
import re, sys
W={"Toggle":"ToggleConfig","Slider":"SliderConfig","Progress":"ProgressConfig","Button":"ButtonConfig","IconButton":"IconButtonConfig","Check":"CheckConfig","Spinner":"SpinnerConfig","Badge":"BadgeConfig","Label":"LabelConfig","Footer":"FooterConfig","Grip":"GripConfig","Panel":"PanelConfig","Dialog":"DialogConfig","Section":"SectionConfig","InspectorRow":"InspectorRowConfig","Split":"SplitConfig","Scroll":"ScrollConfig","Toast":"ToastsConfig","Tip":"TooltipConfig","Tab":"TabsConfig","Segment":"SegmentedConfig","Dropdown":"DropdownConfig","MenuItem":"MenuItemConfig","MenuRow":"MenuRowConfig","Row":"RowConfig","TreeRow":"TreeRowConfig","TableHead":"TableHeadConfig","TableCell":"TableCellConfig","PickerRow":"PickerRowConfig","SearchField":"SearchFieldConfig","TextField":"TextFieldConfig","LineField":"LineFieldConfig","ValueField":"ValueFieldConfig","PickerField":"PickerFieldConfig"}
FN="text_of|text_at|hint_at|states_of|curves_of"
for p in sys.argv[1:]:
    s=open(p).read(); o=s
    uses_bodies = re.search(rf"bodies::(?:\{{[^}}]*\b(?:{FN})\b[^}}]*\}}|(?:{FN})\b)", s) is not None
    # wrapper names where a Rust body signature takes &Pxui*Config (via bodies path)
    def ty(m):
        return m.group(1)+W[m.group(2)]
    s=re.sub(r"((?:crate::|pxui::)?bodies::)Pxui(\w+?)Config\b", lambda m: m.group(1)+W[m.group(2)] if m.group(2) in W else m.group(0), s)
    s=re.sub(r"(use (?:crate|pxui)::bodies::\{)([^}]*)\}", lambda m: m.group(1)+re.sub(r"\bPxui(\w+?)Config\b", lambda k: W.get(k.group(1),k.group(0)), m.group(2))+"}", s)
    if uses_bodies:
        # qualified calls
        s=re.sub(rf"(?:crate::|pxui::)?bodies::({FN})\(&(\w+)\.", lambda m: f"{m.group(2)}.{m.group(1)}(&{m.group(2)}.", s)
        # bare calls after `use crate::bodies::{text_of,...}`
        s=re.sub(rf"(?<![\w.:])({FN})\(&(c|config)\.", lambda m: f"{m.group(2)}.{m.group(1)}(&{m.group(2)}.", s)
        s=re.sub(r"(use (?:crate|pxui)::bodies::\{)([^}]*)\}", lambda m: m.group(1)+", ".join(x for x in [y.strip() for y in m.group(2).split(",")] if x not in FN.split("|")+[""] )+"}" , s)
        s=s.replace("use crate::bodies::{}", "use crate::bodies::BodyConfig")
    if s!=o:
        if uses_bodies and "BodyConfig" not in s:
            # add trait import at first `use` line inside module or file top
            s=re.sub(r"(\n\s*)(use super::\*;)", r"\1\2\1use crate::bodies::BodyConfig;", s, count=1) if "use super::*;" in s else s
        open(p,"w").write(s); print(p)
# pass 2 (run with BARE=1): bare Pxui*Config names in files that import wrappers
import os
if os.environ.get("BARE"):
    for p in sys.argv[1:]:
        s=open(p).read(); o=s
        s=re.sub(r"\bPxui(\w+?)Config\b", lambda k: W.get(k.group(1),k.group(0)), s)
        if s!=o: open(p,"w").write(s); print("bare",p)
