import sys,collections
def load(p):
    d={}
    for l in open(p):
        t,n,c,g,f,a=l.rstrip('\n').split('\t')
        d[(t,int(n))]=(c,g,f,a)
    return d
A=load(sys.argv[1]);B=load(sys.argv[2])
tests=sorted({k[0] for k in A}|{k[0] for k in B})
cols=['count','geom','full','a11y']
for t in tests:
    ka=sorted(k for k in A if k[0]==t); kb=sorted(k for k in B if k[0]==t)
    if len(ka)!=len(kb): print(f"{t}: frames {len(ka)} vs {len(kb)}")
    diffs=collections.Counter(); first={}
    for k in set(ka)&set(kb):
        for i,c in enumerate(cols):
            if A[k][i]!=B[k][i]:
                diffs[c]+=1; first.setdefault(c,k[1]); first[c]=min(first[c],k[1])
    if diffs: print(t, dict(diffs), 'first', first, 'of', len(ka))
