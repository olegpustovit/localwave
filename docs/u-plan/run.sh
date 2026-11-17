#!/bin/bash
# run.sh <rounds> <label=app>... -- <workloads...>; appends "label workload round build_p50 build_p99 frame_p99 allocs load" to stdout
S=$(dirname "$0")
R=$1; shift
apps=(); while [ "$1" != "--" ]; do apps+=("$1"); shift; done; shift
for r in $(seq 1 $R); do
  for a in "${apps[@]}"; do
    l=${a%%=*}; app=${a#*=}
    for w in "$@"; do
      until [ "$(sysctl -n vm.loadavg | awk '{print ($2<4)}')" = 1 ]; do sleep 5; done
      load=$(sysctl -n vm.loadavg | awk "{print \$2}")
      out=$("$app/Contents/MacOS/localwave" --performance-check --performance-$w 2>&1)
      b=$(echo "$out" | grep 'build timing' | sed -E 's/.*cpu_us_p50=([0-9]+).*cpu_us_p99=([0-9]+).*/\1 \2/')
      f=$(echo "$out" | grep 'frame timing' | sed -E 's/.*cpu_us_p99=([0-9]+).*/\1/')
      al=$(echo "$out" | grep 'LocalWave performance:' | sed -E 's/.*max_warm_rust_allocations=([0-9]+).*/\1/')
      echo "$l $w $r $b $f $al $load"
    done
  done
done
