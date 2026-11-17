p='crates/public/pxui/src/bodies.rs'
s=open(p).read()
a=s.index("pub use pxui_types::ffi::{PxuiToggleConfig")
b=s.index("library!(sys {")
cfgs=[("ToggleConfig","PxuiToggleConfig"),("SliderConfig","PxuiSliderConfig"),("ProgressConfig","PxuiProgressConfig"),("ButtonConfig","PxuiButtonConfig"),("IconButtonConfig","PxuiIconButtonConfig"),("CheckConfig","PxuiCheckConfig"),("SpinnerConfig","PxuiSpinnerConfig"),("BadgeConfig","PxuiBadgeConfig"),("LabelConfig","PxuiLabelConfig"),("FooterConfig","PxuiFooterConfig"),("GripConfig","PxuiGripConfig"),("PanelConfig","PxuiPanelConfig"),("DialogConfig","PxuiDialogConfig"),("SectionConfig","PxuiSectionConfig"),("InspectorRowConfig","PxuiInspectorRowConfig"),("SplitConfig","PxuiSplitConfig"),("ScrollConfig","PxuiScrollConfig"),("ToastsConfig","PxuiToastConfig"),("TooltipConfig","PxuiTipConfig"),("TabsConfig","PxuiTabConfig"),("SegmentedConfig","PxuiSegmentConfig"),("DropdownConfig","PxuiDropdownConfig"),("MenuItemConfig","PxuiMenuItemConfig"),("MenuRowConfig","PxuiMenuRowConfig"),("RowConfig","PxuiRowConfig"),("TreeRowConfig","PxuiTreeRowConfig"),("TableHeadConfig","PxuiTableHeadConfig"),("TableCellConfig","PxuiTableCellConfig"),("PickerRowConfig","PxuiPickerRowConfig"),("SearchFieldConfig","PxuiSearchFieldConfig"),("TextFieldConfig","PxuiTextFieldConfig"),("LineFieldConfig","PxuiLineFieldConfig"),("ValueFieldConfig","PxuiValueFieldConfig"),("PickerFieldConfig","PxuiPickerFieldConfig")]
use_line="use pxui_types::ffi::{"+", ".join(r for _,r in cfgs)+"};\n"
decl="configs! {\n"+"".join(f"    {w} {r},\n" for w,r in cfgs)+"}\n\n"
s=s[:a]+use_line+"\n@@CONFIGS@@"+s[b:]
new_api=open('tools/dx/codemods/archive/codemod_g/api.rs.txt').read()
s=s.replace("@@CONFIGS@@",new_api+decl)
c=s.index("/// The text of a config field, borrowed")
d=s.index("/// What every registered body's C entry does")
s=s[:c]+s[d:]
s=s.replace("body: fn(&mut Ui, &$config)) -> Result<BodyId, String> {","body: fn(&mut Ui, &$wrapper)) -> Result<BodyId, String> {")
s=s.replace("core::mem::transmute::<*mut c_void, fn(&mut Ui, &$config)>(user)","core::mem::transmute::<*mut c_void, fn(&mut Ui, &$wrapper)>(user)")
s=s.replace("unsafe { &*config.cast::<$config>() }","unsafe { &*config.cast::<$wrapper>() }")
open(p,'w').write(s)
