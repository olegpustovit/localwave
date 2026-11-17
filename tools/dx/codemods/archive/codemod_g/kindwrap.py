import re,sys
cam=lambda k:"".join(x.capitalize() for x in k.split("_"))
for p in sys.argv[1:]:
    s=open(p).read();o=s
    regs={}
    for m in re.finditer(r"register_(\w+)_body\(\s*[^,()]+,\s*(\w+)\s*\)",s): regs.setdefault(m.group(2),set()).add(m.group(1))
    def f(m):
        name=m.group(2)
        ks=regs.get(name)
        if ks and len(ks)==1: return m.group(1)+cam(next(iter(ks)))+"Config"
        return m.group(0)
    s=re.sub(r"(fn (\w+)\([^)]*?c: &(?:crate::bodies::|pxui::bodies::)?)\w+Config",f,s)
    if s!=o: open(p,"w").write(s);print(p)
