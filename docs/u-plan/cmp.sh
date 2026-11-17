#!/bin/bash
# cmp.sh <rounds> <label=app>...: resize + theme-switch alternating; prints per-label mean of build p50 per workload
S=$(dirname "$0"); R=$1; shift
out=$("$S/run.sh" $R "$@" -- resize theme-switch)
echo "$out" > "$S/last_cmp.txt"
echo "$out" | awk '{k=$1" "$2; s[k]+=$4; p[k]=p[k]" "$4"/"$5; n[k]++} END {for (k in s) printf "%s mean_p50=%.0f  p50/p99:%s\n", k, s[k]/n[k], p[k]}' | sort
