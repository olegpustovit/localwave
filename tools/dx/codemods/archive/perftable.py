# perftable.py LOG HEADLINE -> replaces the "## Latest performance" table rows in docs/rust-port.md
import re,sys
log,head=sys.argv[1],sys.argv[2]
rows=[];cur=None
kv=lambda l: dict(re.findall(r'(\w+)=([\w.\-]+)',l))
for l in open(log):
    if l.startswith('LocalWave performance:'):
        cur=kv(l); cur['b']=cur['f']=cur['w']={}; rows.append(cur)
    elif cur is not None and l.startswith('LocalWave build timing'): cur['b']=kv(l)
    elif cur is not None and l.startswith('LocalWave frame timing'): cur['f']=kv(l)
    elif cur is not None and l.startswith('LocalWave work timing'): cur['w']=kv(l)
    elif l.startswith('LocalWave measured:'):
        m=kv(l)
        for r in rows:
            if r['workload']==m['workload']: r['gate']='measured'
smoke=re.search(r'idle_ms=([\d.]+)',open(log).read())
out=[]
for r in rows:
    b,f,w=r['b'],r['f'],r['w']
    g=lambda d,k: d.get(k,'-')
    out.append(f"| {r['workload']} | {g(b,'cpu_us_p50')}/{g(b,'cpu_us_p99')}/{g(b,'cpu_us_max')} | {g(b,'wall_us_p50')}/{g(b,'wall_us_p95')}/{g(b,'wall_us_p99')}/{g(b,'wall_us_max')} | {g(f,'cpu_us_p50')}/{g(f,'cpu_us_p99')}/{g(f,'cpu_us_max')} | {g(w,'wall_us_p50')}/{g(w,'wall_us_p95')}/{g(w,'wall_us_p99')}/{g(w,'wall_us_max')} | {r.get('max_warm_rust_allocations','-')} | {r.get('gate','pass')} |")
p='docs/rust-port.md'; s=open(p).read()
i=s.index('## Latest performance'); j=s.index('| Workload | Build CPU |',i)
k=s.index('\n\n',j)
hdr=s[j:s.index('\n',s.index('\n',j)+1)+1]
s=s[:i]+'## Latest performance\n\n'+head+(f' Settled idle smoke passes ({smoke.group(1)} ms).' if smoke else '')+'\n\n'+s[s.index('Gate (owner decision',i):j]+hdr+'\n'.join(out)+s[k:]
open(p,'w').write(s)
print(len(out),'rows')
