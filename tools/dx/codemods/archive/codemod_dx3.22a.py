import re,sys
# insert the row-id argument: selection_list(key, count, ids, selected, ..) and table_view(key, titles, widths, rows, ids, selected, ..)
def fix(path, none):
    s=open(path).read(); o=s
    s=re.sub(r'(\.selection_list\("[^"]*", [A-Za-z_0-9]+, )(&mut )', lambda m:m.group(1)+none+m.group(2), s)
    s=re.sub(r'(\.table_view\("[^"]*", &?[^;]*?, (?:&mut [^,]*?|widths|\[\]|&mut \[\]), (?:[A-Za-z_0-9 ]+?), )((?:&mut (?:None|selected|table_sel|cell|s\.\w+))|selected), (&)', lambda m:m.group(1)+none+m.group(2)+', '+m.group(3), s)
    if s!=o: open(path,'w').write(s)
for p in sys.argv[2:]: fix(p, sys.argv[1])
