import re,sys
MAP=dict([("context_menu","menu_row"),("list_view","selection_list"),("table_view","table_row"),("table","table_cell"),("tab","tabs"),("segment","segmented"),("toast","toasts"),("tip","tooltip")])
for p in sys.argv[1:]:
    s=open(p).read();o=s
    s=re.sub(r"body_of!\((\w+), (\w+)\)",lambda m:f"body_of!({m.group(1)}, {MAP.get(m.group(2),m.group(2))})",s)
    s=re.sub(r"(in_body!\((?:flat; )?\w+, [^,]+, [^,]+, )(\w+)([,)])",lambda m:m.group(1)+MAP.get(m.group(2),m.group(2))+m.group(3),s)
    if s!=o: open(p,"w").write(s);print(p)
