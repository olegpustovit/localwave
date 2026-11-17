# button_content(.., |ui| ..) -> |ui, _| for DX3.12
import glob,re
n=0
for p in glob.glob('crates/lw_ui/src/**/*.rs', recursive=True):
    s=open(p).read(); out=[]; i=0
    while True:
        j=s.find('button_content(', i)
        if j<0: out.append(s[i:]); break
        k=s.find('|ui|', j)
        nxt=s.find('button_content(', j+1)
        if k<0 or (nxt!=-1 and k>nxt): out.append(s[i:j+1]); i=j+1; continue
        out.append(s[i:k]); out.append('|ui, _|'); i=k+4; n+=1
    t=''.join(out)
    if t!=s: open(p,'w').write(t)
print('closures updated:', n)
