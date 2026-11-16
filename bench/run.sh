#!/bin/sh
# Builds the benchmark with g++ and clang++ and runs it on one CPU.
#
#   bench/run.sh [cpu]     cpu: which CPU to pin to (default: the last one).
#                          Pick one that is idle, e.g. with `mpstat -P ALL 1 1`.
set -e
cd "$(dirname "$0")"
mkdir -p build
CPU=${1:-$(($(nproc --all) - 1))}

# main.cc: (25 rounds + 1 warm-up) x 20,000,000 calls
TOTAL_CALLS=520000000

for CXX in g++ clang++; do
  $CXX -std=c++23 -O2 -o build/entry_cost main.cc exports.cc

  echo "== $($CXX --version | head -1), -O2, CPU $CPU"
  taskset -c "$CPU" build/entry_cost

  # The same again, one variant per process, counted by the CPU instead of timed.
  # Needs perf; skipped without it.
  if command -v perf >/dev/null; then
    for VARIANT in plain unwind setjmp; do
      perf stat -x, -e cycles:u,instructions:u taskset -c "$CPU" build/entry_cost $VARIANT 2>&1 >/dev/null |
        awk -F, -v v=$VARIANT -v n=$TOTAL_CALLS '{ printf "%-8s %8.1f %s per call\n", v, $1 / n, $3 }'
    done
  fi
done
