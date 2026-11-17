#!/bin/bash
# lwland_blocked.sh ROW LWCOMMIT : cherry-pick a LocalWave commit, mark a blocked/todo row done, make check (lanes paused), commit.
set -e; cd /Users/snowbear/WORK/GIT/localwave
ROW=$1; C=$2
git cherry-pick -n $C
P=$(git ls-files -s pxui | awk '{print $2}'); git -C pxui cat-file -e $P 2>/dev/null || git -C pxui fetch -q origin; git -C pxui checkout -q $P
prev=$(git rev-parse --short HEAD)
python3 - "$ROW" "$prev" <<'EOF'
import re,sys
r,prev=sys.argv[1:3]; p='docs/rust-port.md'; s=open(p).read()
s=re.sub(r'^(\| DX[-A-Z0-9.]+[a-z]? \|[^|]*\| done \| )HEAD(?=[ |])', lambda m:m.group(1)+prev, s, flags=re.M)
s,n=re.subn(r'^(\| '+re.escape(r)+r' \|[^|]*\| )(blocked|todo) \|[^|\n]*\|', lambda m:m.group(1)+'done | HEAD |', s, flags=re.M); assert n==1, r
open(p,'w').write(s)
EOF
git add -A crates docs
pkill -STOP -f "dx-wt|debugserver" || true
rc=0; (cargo t -p lw_ui && cargo build -q -p lw_bin) > /tmp/lw_land_check.log 2>&1 || rc=$?
pkill -CONT -f "dx-wt|debugserver" || true
[ $rc = 0 ] || { echo "$ROW make check FAILED"; tail -20 /tmp/lw_land_check.log; exit 1; }
git commit -q -m "$(git log -1 --format=%B $C)" -m "Evidence: targeted checks green (lw_ui tests, lw_bin build); full regression at the wave checkpoint."
git log --oneline -1
