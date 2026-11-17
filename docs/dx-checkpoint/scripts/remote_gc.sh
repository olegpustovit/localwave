#!/bin/bash
# remote_gc.sh [LANE...] : delete gigacomp copies + targets for the named lanes and for any lane whose local worktree is gone.
H=snowbear@gigacomp
for l in "$@"; do ssh -q -o ConnectTimeout=10 $H "rm -rf ~/pxui-dx/dx-wt_$l ~/pxui-dx/target-dx-wt_$l"; done
for d in $(ssh -q -o ConnectTimeout=10 $H 'ls ~/pxui-dx 2>/dev/null' | grep -v '^target-'); do
  [ -d "/Users/snowbear/WORK/GIT/$(echo $d | sed 's#_#/#')" ] || ssh -q $H "rm -rf ~/pxui-dx/$d ~/pxui-dx/target-$d"
done
ssh -q -o ConnectTimeout=10 $H 'du -sh ~/pxui-dx 2>/dev/null; df -h / | tail -1'
