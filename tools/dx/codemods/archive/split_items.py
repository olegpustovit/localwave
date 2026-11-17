# DX6.2: one item per line in pxui and pxui_types public sources. Whitespace-only change.
import re,sys,glob
root=sys.argv[1]
allow=set()
for row in open(root+'/tools/gate/shape-allowlist.tsv'):
    c=row.rstrip('\n').split('\t')
    if len(c)==4: allow.add(' '.join(c[2].split()))
def split_top(s,sep):
    out=[];d=0;cur='';i=0;instr=False
    while i<len(s):
        ch=s[i]
        if ch=='"' : instr=not instr
        if not instr:
            if ch in '([{<': d+=1
            elif ch in ')]}>':
                if not (ch=='>' and i>0 and s[i-1]=='-'): d-=1
            if ch==sep and d==0:
                out.append(cur+ (sep if sep==';' else ',')); cur=''; i+=1
                while i<len(s) and s[i]==' ': i+=1
                continue
        cur+=ch; i+=1
    if cur.strip(): out.append(cur)
    return [o.strip() for o in out if o.strip()]
FIELD=re.compile(r'\bpub(\(crate\))? [a-z_][a-z_0-9]*\s*:')
def items_count(body):
    parts=split_top(body,',')
    return parts
changed=0
def process(path):
    global changed
    src=open(path).read().split('\n')
    out=[]
    for l in src:
        s=l.strip(); ind=l[:len(l)-len(l.lstrip())]
        if s.startswith('//') or ' '.join(s.split()) in allow or '//' in s:
            out.append(l); continue
        m=re.match(r'^((?:pub(?:\(crate\))? )?(?:struct|enum) [^{]*\{) (.*) \}$',s)
        if m and 'fn ' not in s:
            head,body=m.group(1),m.group(2)
            parts=split_top(body,',')
            if len(parts)>1:
                out.append(ind+head)
                for p in parts: out.append(ind+'    '+(p if p.endswith(',') else p+','))
                # last item: original had no trailing comma; adding one changes bytes only by ','
                out.append(ind+'}'); changed+=1; continue
        m=re.match(r'^(impl [^{]*\{) (.*) \}$',s)
        if m and s.count('pub const ')>1:
            out.append(ind+m.group(1))
            for p in split_top(m.group(2),';'): out.append(ind+'    '+p)
            out.append(ind+'}'); changed+=1; continue
        if s.count('pub const ')>1 and s.endswith(';'):
            for p in split_top(s,';'): out.append(ind+p)
            changed+=1; continue
        if len(FIELD.findall(s))>1 and 'fn ' not in s and '{' not in s and '}' not in s:
            for p in split_top(s,','): out.append(ind+p)
            changed+=1; continue
        if re.match(r'^[A-Z][A-Za-z0-9]*( = [^,]+)?(\([^)]*\))?, *[A-Z][A-Za-z0-9]*',s) and '{' not in s and '}' not in s:
            for p in split_top(s,','): out.append(ind+p)
            changed+=1; continue
        out.append(l)
    open(path,'w').write('\n'.join(out))
for f in sorted(glob.glob(root+'/crates/public/pxui/src/*.rs')+glob.glob(root+'/crates/public/pxui_types/src/*.rs')):
    process(f)
print('split lines',changed)
